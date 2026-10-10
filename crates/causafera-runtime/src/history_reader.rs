use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex};

use causafera_core::{
    CausalHistoryReader, CausalTarget, DomainEvidenceRecord, EvidenceSchemaId, HistoryAnchor,
    HistoryAnchorFields, HistoryDurability, HistoryError, HistoryFrontier, HistoryPage,
    HistoryPageItem, HistoryQuery, HistoryRecord, OwnedCausalRecord,
};
use causafera_types::{EventId, TraceId};

use crate::{CURRENT_DIGEST_SCHEMA_VERSION, CanonicalDigest};

#[derive(Debug)]
pub(crate) struct RuntimeHistoryReader {
    inner: Mutex<HistoryIndex>,
}

#[derive(Debug)]
struct HistoryIndex {
    cold: Arc<dyn CausalHistoryReader>,
    cold_records: usize,
    cold_causal_records: usize,
    namespace: [u8; 16],
    logical_generation: u64,
    durable_generation: u64,
    checkpoint_digest: [u8; 32],
    frontier: HistoryFrontier,
    logical_digest: [u8; 32],
    storage_root: [u8; 32],
    evidence_manifest_digest: [u8; 32],
    durability: HistoryDurability,
    overlay: Vec<HistoryRecord>,
    overlay_events: BTreeMap<TraceId, usize>,
    overlay_event_ids: BTreeMap<EventId, usize>,
    overlay_evidence: BTreeMap<(EvidenceSchemaId, Vec<u8>), usize>,
}

impl RuntimeHistoryReader {
    pub(crate) fn new(
        cold: Arc<dyn CausalHistoryReader>,
        cold_records: usize,
        cold_causal_records: usize,
    ) -> Result<Self, HistoryError> {
        let anchor = cold.anchor()?;
        Ok(Self {
            inner: Mutex::new(HistoryIndex {
                cold,
                cold_records,
                cold_causal_records,
                namespace: *anchor.namespace(),
                logical_generation: anchor.logical_generation(),
                durable_generation: anchor.durable_generation(),
                checkpoint_digest: *anchor.checkpoint_digest(),
                frontier: anchor.frontier(),
                logical_digest: *anchor.logical_digest(),
                storage_root: *anchor.storage_root(),
                evidence_manifest_digest: *anchor.evidence_manifest_digest(),
                durability: anchor.durability(),
                overlay: Vec::new(),
                overlay_events: BTreeMap::new(),
                overlay_event_ids: BTreeMap::new(),
                overlay_evidence: BTreeMap::new(),
            }),
        })
    }

    pub(crate) fn validate_append(
        &self,
        generation: u64,
        records: &[HistoryRecord],
    ) -> Result<(), HistoryError> {
        let index = self
            .inner
            .lock()
            .map_err(|_| HistoryError::ReaderOperationUnavailable)?;
        let expected = index
            .logical_generation
            .checked_add(1)
            .ok_or(HistoryError::ArithmeticOverflow)?;
        if generation != expected {
            return Err(HistoryError::NonCanonicalOrder { index: 0 });
        }
        validate_records(&index, records)
    }

    pub(crate) fn append(
        &self,
        generation: u64,
        records: &[HistoryRecord],
        logical_digest: [u8; 32],
        storage_root: [u8; 32],
        evidence_manifest_digest: [u8; 32],
    ) -> Result<(), HistoryError> {
        let mut index = self
            .inner
            .lock()
            .map_err(|_| HistoryError::ReaderOperationUnavailable)?;
        let expected = index
            .logical_generation
            .checked_add(1)
            .ok_or(HistoryError::ArithmeticOverflow)?;
        if generation != expected {
            return Err(HistoryError::NonCanonicalOrder { index: 0 });
        }
        validate_records(&index, records)?;
        for record in records {
            if let HistoryRecord::Causal(causal) = record {
                index.frontier = HistoryFrontier::new(
                    generation,
                    causal.event_id(),
                    causal.trace_id(),
                    causal.time(),
                );
            }
        }
        let overlay_start = index.overlay.len();
        for (offset, record) in records.iter().enumerate() {
            let position = overlay_start
                .checked_add(offset)
                .ok_or(HistoryError::ArithmeticOverflow)?;
            match record {
                HistoryRecord::Causal(record) => {
                    index.overlay_events.insert(record.trace_id(), position);
                    index.overlay_event_ids.insert(record.event_id(), position);
                }
                HistoryRecord::Evidence(record) => {
                    index.overlay_evidence.insert(
                        (record.schema_id(), record.canonical_key().to_vec()),
                        position,
                    );
                }
            }
        }
        index.overlay.extend_from_slice(records);
        index.logical_generation = generation;
        index.logical_digest = logical_digest;
        index.storage_root = storage_root;
        index.evidence_manifest_digest = evidence_manifest_digest;
        Ok(())
    }

    pub(crate) fn install_durable(
        &self,
        cold: Arc<dyn CausalHistoryReader>,
        cold_records: usize,
        cold_causal_records: usize,
        anchor: HistoryAnchor,
    ) -> Result<(), HistoryError> {
        let mut index = self
            .inner
            .lock()
            .map_err(|_| HistoryError::ReaderOperationUnavailable)?;
        index.cold = cold;
        index.cold_records = cold_records;
        index.cold_causal_records = cold_causal_records;
        index.namespace = *anchor.namespace();
        index.logical_generation = anchor.logical_generation();
        index.durable_generation = anchor.durable_generation();
        index.checkpoint_digest = *anchor.checkpoint_digest();
        index.frontier = anchor.frontier();
        index.logical_digest = *anchor.logical_digest();
        index.storage_root = *anchor.storage_root();
        index.evidence_manifest_digest = *anchor.evidence_manifest_digest();
        index.durability = anchor.durability();
        index.overlay.clear();
        index.overlay_events.clear();
        index.overlay_event_ids.clear();
        index.overlay_evidence.clear();
        Ok(())
    }

    /// Replace the digests the anchors carry, once a replay has reached the
    /// history's durable head and they are known exactly.
    pub(crate) fn set_digests(
        &self,
        logical_digest: [u8; 32],
        storage_root: [u8; 32],
        evidence_manifest_digest: [u8; 32],
    ) -> Result<(), HistoryError> {
        let mut index = self
            .inner
            .lock()
            .map_err(|_| HistoryError::ReaderOperationUnavailable)?;
        index.logical_digest = logical_digest;
        index.storage_root = storage_root;
        index.evidence_manifest_digest = evidence_manifest_digest;
        Ok(())
    }

    /// Record that a checkpoint with this digest is now the latest one, so the
    /// anchors this reader hands out point at it.
    pub(crate) fn set_checkpoint_digest(&self, digest: [u8; 32]) -> Result<(), HistoryError> {
        self.inner
            .lock()
            .map_err(|_| HistoryError::ReaderOperationUnavailable)?
            .checkpoint_digest = digest;
        Ok(())
    }

    pub(crate) fn rollback(
        &self,
        previous_generation: u64,
        records: &[HistoryRecord],
        _previous_digest: CanonicalDigest,
        previous_digest_bytes: [u8; 32],
        previous_frontier: HistoryFrontier,
    ) -> Result<(), HistoryError> {
        let mut index = self
            .inner
            .lock()
            .map_err(|_| HistoryError::ReaderOperationUnavailable)?;
        if index.overlay.len() < records.len()
            || index.overlay[index.overlay.len() - records.len()..] != *records
        {
            return Err(HistoryError::ReaderOperationUnavailable);
        }
        let new_len = index.overlay.len() - records.len();
        let removed = index.overlay[new_len..]
            .iter()
            .map(|record| match record {
                HistoryRecord::Causal(record) => {
                    (Some((record.trace_id(), record.event_id())), None)
                }
                HistoryRecord::Evidence(record) => (
                    None,
                    Some((record.schema_id(), record.canonical_key().to_vec())),
                ),
            })
            .collect::<Vec<_>>();
        for (causal, evidence) in removed {
            if let Some((trace_id, event_id)) = causal {
                index.overlay_events.remove(&trace_id);
                index.overlay_event_ids.remove(&event_id);
            }
            if let Some(key) = evidence {
                index.overlay_evidence.remove(&key);
            }
        }
        index.overlay.truncate(new_len);
        index.logical_generation = previous_generation;
        index.logical_digest = previous_digest_bytes;
        index.frontier = previous_frontier;
        Ok(())
    }

    pub(crate) fn publication_preimage(
        &self,
    ) -> Result<(u64, CanonicalDigest, HistoryFrontier), HistoryError> {
        let index = self
            .inner
            .lock()
            .map_err(|_| HistoryError::ReaderOperationUnavailable)?;
        Ok((
            index.logical_generation,
            digest_bytes(index.logical_digest),
            index.frontier,
        ))
    }

    pub(crate) fn logical_digest_bytes(&self) -> Result<[u8; 32], HistoryError> {
        let index = self
            .inner
            .lock()
            .map_err(|_| HistoryError::ReaderOperationUnavailable)?;
        Ok(index.logical_digest)
    }

    pub(crate) fn all_records_for_oracle(&self) -> Result<Vec<OwnedCausalRecord>, HistoryError> {
        let (cold, overlay) = {
            let index = self
                .inner
                .lock()
                .map_err(|_| HistoryError::ReaderOperationUnavailable)?;
            let overlay = index
                .overlay
                .iter()
                .filter_map(|record| match record {
                    HistoryRecord::Causal(record) => Some(record.clone()),
                    HistoryRecord::Evidence(_) => None,
                })
                .collect::<Vec<_>>();
            (index.cold.clone(), overlay)
        };
        let mut records = cold.all_records_for_oracle()?;
        records.extend(overlay);
        Ok(records)
    }

    #[cfg(test)]
    pub(crate) fn new_for_test(
        namespace: [u8; 16],
        records: Vec<OwnedCausalRecord>,
    ) -> Result<Self, HistoryError> {
        let count = records.len();
        let cold: Arc<dyn CausalHistoryReader> = Arc::new(TestColdReader::new(namespace, records)?);
        Self::new(cold, count, count)
    }
}

impl CausalHistoryReader for RuntimeHistoryReader {
    fn anchor(&self) -> Result<HistoryAnchor, HistoryError> {
        let index = self
            .inner
            .lock()
            .map_err(|_| HistoryError::ReaderOperationUnavailable)?;
        HistoryAnchor::new(HistoryAnchorFields {
            namespace: index.namespace,
            logical_generation: index.logical_generation,
            durable_generation: index.durable_generation,
            checkpoint_digest: index.checkpoint_digest,
            frontier: index.frontier,
            logical_digest: index.logical_digest,
            storage_root: index.storage_root,
            physical_digest_schema: CURRENT_DIGEST_SCHEMA_VERSION.raw(),
            history_digest_schema: CURRENT_DIGEST_SCHEMA_VERSION.raw(),
            evidence_manifest_digest: index.evidence_manifest_digest,
            durability: index.durability,
        })
    }

    fn event(&self, trace_id: TraceId) -> Result<Option<OwnedCausalRecord>, HistoryError> {
        let cold = {
            let index = self
                .inner
                .lock()
                .map_err(|_| HistoryError::ReaderOperationUnavailable)?;
            if let Some(record) = index.event_from_overlay(trace_id) {
                return Ok(Some(record));
            }
            Arc::clone(&index.cold)
        };
        cold.event(trace_id)
    }

    fn event_by_id(&self, event_id: EventId) -> Result<Option<OwnedCausalRecord>, HistoryError> {
        let cold = {
            let index = self
                .inner
                .lock()
                .map_err(|_| HistoryError::ReaderOperationUnavailable)?;
            if let Some(record) = index.event_by_id_from_overlay(event_id) {
                return Ok(Some(record));
            }
            Arc::clone(&index.cold)
        };
        cold.event_by_id(event_id)
    }

    fn records(&self, query: HistoryQuery) -> Result<HistoryPage<HistoryRecord>, HistoryError> {
        self.page_records(query, false)
    }

    fn indexed_range(
        &self,
        query: HistoryQuery,
    ) -> Result<HistoryPage<HistoryRecord>, HistoryError> {
        self.page_records(query, true)
    }

    fn parents(
        &self,
        trace_id: TraceId,
        query: HistoryQuery,
    ) -> Result<HistoryPage<TraceId>, HistoryError> {
        let parents = self
            .event(trace_id)?
            .map_or_else(Vec::new, |event| event.causes().to_vec());
        page_values(&parents, query)
    }

    fn children(
        &self,
        trace_id: TraceId,
        query: HistoryQuery,
    ) -> Result<HistoryPage<TraceId>, HistoryError> {
        let cold = {
            let index = self
                .inner
                .lock()
                .map_err(|_| HistoryError::ReaderOperationUnavailable)?;
            Arc::clone(&index.cold)
        };
        let page = cold.children(trace_id, query)?;
        if page.next_offset().is_some() {
            return Ok(page);
        }
        let mut values = page.records().to_vec();
        let index = self
            .inner
            .lock()
            .map_err(|_| HistoryError::ReaderOperationUnavailable)?;
        values.extend(index.overlay.iter().filter_map(|record| match record {
            HistoryRecord::Causal(record) if record.causes().contains(&trace_id) => {
                Some(record.trace_id())
            }
            _ => None,
        }));
        page_values_at(&values, query, query.offset())
    }

    fn latest_matching_event_before(
        &self,
        target: CausalTarget,
        trace_id: TraceId,
    ) -> Result<Option<OwnedCausalRecord>, HistoryError> {
        let cold = {
            let index = self
                .inner
                .lock()
                .map_err(|_| HistoryError::ReaderOperationUnavailable)?;
            Arc::clone(&index.cold)
        };
        let candidate = cold.latest_matching_event_before(target, trace_id)?;
        let index = self
            .inner
            .lock()
            .map_err(|_| HistoryError::ReaderOperationUnavailable)?;
        Ok(index
            .overlay
            .iter()
            .rev()
            .filter_map(|record| match record {
                HistoryRecord::Causal(record)
                    if record.trace_id() < trace_id
                        && record
                            .effects()
                            .iter()
                            .any(|effect| effect.target() == target) =>
                {
                    Some(record.clone())
                }
                _ => None,
            })
            .next()
            .or(candidate))
    }

    fn evidence(
        &self,
        schema_id: EvidenceSchemaId,
        canonical_key: &[u8],
        query: HistoryQuery,
    ) -> Result<HistoryPage<DomainEvidenceRecord>, HistoryError> {
        let cold = {
            let index = self
                .inner
                .lock()
                .map_err(|_| HistoryError::ReaderOperationUnavailable)?;
            Arc::clone(&index.cold)
        };
        let page = cold.evidence(schema_id, canonical_key, query)?;
        if page.next_offset().is_some() {
            return Ok(page);
        }
        let mut values = page.records().to_vec();
        let index = self
            .inner
            .lock()
            .map_err(|_| HistoryError::ReaderOperationUnavailable)?;
        if let Some(position) = index
            .overlay_evidence
            .get(&(schema_id, canonical_key.to_vec()))
            && let Some(HistoryRecord::Evidence(record)) = index.overlay.get(*position)
        {
            values.push(record.clone());
        }
        page_values_at(&values, query, query.offset())
    }
}

impl RuntimeHistoryReader {
    fn page_records(
        &self,
        query: HistoryQuery,
        indexed: bool,
    ) -> Result<HistoryPage<HistoryRecord>, HistoryError> {
        let (cold, cold_records, overlay) = {
            let index = self
                .inner
                .lock()
                .map_err(|_| HistoryError::ReaderOperationUnavailable)?;
            (
                Arc::clone(&index.cold),
                index.cold_records,
                index.overlay.clone(),
            )
        };
        let offset =
            usize::try_from(query.offset()).map_err(|_| HistoryError::ArithmeticOverflow)?;
        if offset < cold_records {
            let page = if indexed {
                cold.indexed_range(query)?
            } else {
                cold.records(query)?
            };
            if page.next_offset().is_some() || overlay.is_empty() {
                return Ok(page);
            }
            let base_offset = query.offset();
            let mut values = page.records().to_vec();
            values.extend(overlay);
            return page_values_at(&values, query, base_offset);
        }
        page_values_at(&overlay, query, cold_records as u64)
    }
}

fn validate_records(index: &HistoryIndex, records: &[HistoryRecord]) -> Result<(), HistoryError> {
    let mut expected_event = index.frontier.event_id.raw();
    let mut expected_trace = index.frontier.trace_id.raw();
    let has_head = index.cold_records > 0 || !index.overlay.is_empty();
    if has_head {
        expected_event = expected_event
            .checked_add(1)
            .ok_or(HistoryError::ArithmeticOverflow)?;
        expected_trace = expected_trace
            .checked_add(1)
            .ok_or(HistoryError::ArithmeticOverflow)?;
    }
    let first_trace = expected_trace;
    let mut seen_events = BTreeSet::new();
    let mut seen_traces = BTreeSet::new();
    let mut seen_evidence_keys: BTreeSet<_> = index.overlay_evidence.keys().cloned().collect();
    for (position, record) in records.iter().enumerate() {
        match record {
            HistoryRecord::Causal(record) => {
                let is_next = record.event_id().raw() == expected_event
                    && record.trace_id().raw() == expected_trace;
                if (!is_next && index.cold.event(record.trace_id())?.is_some())
                    || index.event_from_overlay(record.trace_id()).is_some()
                    || seen_traces.contains(&record.trace_id())
                {
                    return Err(HistoryError::DuplicateTraceId(record.trace_id()));
                }
                if (!is_next && index.cold.event_by_id(record.event_id())?.is_some())
                    || index.event_by_id_from_overlay(record.event_id()).is_some()
                    || seen_events.contains(&record.event_id())
                {
                    return Err(HistoryError::DuplicateEventId(record.event_id()));
                }
                if record.event_id().raw() != expected_event
                    || record.trace_id().raw() != expected_trace
                {
                    return Err(HistoryError::NonCanonicalOrder { index: position });
                }
                for parent in record.causes() {
                    if !seen_traces.contains(parent)
                        && parent.raw() >= first_trace
                        && index.cold.event(*parent)?.is_none()
                        && index.event_from_overlay(*parent).is_none()
                    {
                        return Err(HistoryError::UnknownParent {
                            trace_id: record.trace_id(),
                            parent: *parent,
                        });
                    }
                }
                seen_events.insert(record.event_id());
                seen_traces.insert(record.trace_id());
                expected_event = expected_event
                    .checked_add(1)
                    .ok_or(HistoryError::ArithmeticOverflow)?;
                expected_trace = expected_trace
                    .checked_add(1)
                    .ok_or(HistoryError::ArithmeticOverflow)?;
            }
            HistoryRecord::Evidence(record) => {
                let existing = index.cold.evidence(
                    record.schema_id(),
                    record.canonical_key(),
                    HistoryQuery::new(0, 1, 4 * 1024 * 1024)?,
                )?;
                let evidence_key = (record.schema_id(), record.canonical_key().to_vec());
                if existing.records().iter().any(|value| {
                    value.schema_id() == record.schema_id()
                        && value.canonical_key() == record.canonical_key()
                }) || !seen_evidence_keys.insert(evidence_key)
                {
                    return Err(HistoryError::DuplicateRecordKey);
                }
                for anchor in record.causal_anchors() {
                    if !seen_traces.contains(anchor)
                        && index.cold.event(*anchor)?.is_none()
                        && index.event_from_overlay(*anchor).is_none()
                    {
                        return Err(HistoryError::UnknownEvidenceAnchor { anchor: *anchor });
                    }
                }
            }
        }
    }
    Ok(())
}

impl HistoryIndex {
    fn event_from_overlay(&self, trace_id: TraceId) -> Option<OwnedCausalRecord> {
        self.overlay_events
            .get(&trace_id)
            .and_then(|position| self.overlay.get(*position))
            .and_then(|record| match record {
                HistoryRecord::Causal(record) => Some(record.clone()),
                HistoryRecord::Evidence(_) => None,
            })
    }

    fn event_by_id_from_overlay(&self, event_id: EventId) -> Option<OwnedCausalRecord> {
        self.overlay_event_ids
            .get(&event_id)
            .and_then(|position| self.overlay.get(*position))
            .and_then(|record| match record {
                HistoryRecord::Causal(record) => Some(record.clone()),
                HistoryRecord::Evidence(_) => None,
            })
    }
}

fn page_values<T: HistoryPageItem + Clone>(
    values: &[T],
    query: HistoryQuery,
) -> Result<HistoryPage<T>, HistoryError> {
    page_values_at(values, query, 0)
}

fn page_values_at<T: HistoryPageItem + Clone>(
    values: &[T],
    query: HistoryQuery,
    base_offset: u64,
) -> Result<HistoryPage<T>, HistoryError> {
    let offset = usize::try_from(query.offset()).map_err(|_| HistoryError::ArithmeticOverflow)?;
    let base = usize::try_from(base_offset).map_err(|_| HistoryError::ArithmeticOverflow)?;
    let local_offset = offset
        .checked_sub(base)
        .ok_or(HistoryError::ArithmeticOverflow)?;
    let mut selected = Vec::new();
    let mut bytes = 0_usize;
    for value in values.iter().skip(local_offset) {
        if selected.len() == query.max_records() {
            break;
        }
        let next = bytes
            .checked_add(value.decoded_size())
            .ok_or(HistoryError::ArithmeticOverflow)?;
        if next > query.max_bytes() {
            break;
        }
        bytes = next;
        selected.push(value.clone());
    }
    let consumed = offset
        .checked_add(selected.len())
        .ok_or(HistoryError::ArithmeticOverflow)?;
    let end = base
        .checked_add(values.len())
        .ok_or(HistoryError::ArithmeticOverflow)?;
    let next = (consumed < end)
        .then(|| u64::try_from(consumed).map_err(|_| HistoryError::ArithmeticOverflow))
        .transpose()?;
    HistoryPage::new(selected, next, query)
}

fn digest_bytes(bytes: [u8; 32]) -> CanonicalDigest {
    let mut digest = CanonicalDigest::new();
    for chunk in bytes.as_chunks::<8>().0 {
        digest.write(u64::from_le_bytes(*chunk));
    }
    digest
}

#[cfg(test)]
#[derive(Debug)]
struct TestColdReader {
    namespace: [u8; 16],
    records: Vec<HistoryRecord>,
}

#[cfg(test)]
impl TestColdReader {
    fn new(namespace: [u8; 16], records: Vec<OwnedCausalRecord>) -> Result<Self, HistoryError> {
        Ok(Self {
            namespace,
            records: records.into_iter().map(HistoryRecord::Causal).collect(),
        })
    }
}

#[cfg(test)]
impl CausalHistoryReader for TestColdReader {
    fn anchor(&self) -> Result<HistoryAnchor, HistoryError> {
        let frontier = self
            .records
            .iter()
            .filter_map(|record| match record {
                HistoryRecord::Causal(record) => Some(record),
                HistoryRecord::Evidence(_) => None,
            })
            .next_back()
            .map_or(
                HistoryFrontier::new(
                    0,
                    EventId::new(0),
                    TraceId::new(0),
                    causafera_types::SimulationTime::new(0),
                ),
                |record| {
                    HistoryFrontier::new(0, record.event_id(), record.trace_id(), record.time())
                },
            );
        HistoryAnchor::new(HistoryAnchorFields {
            namespace: self.namespace,
            logical_generation: 0,
            durable_generation: 0,
            checkpoint_digest: [0; 32],
            frontier,
            logical_digest: [1; 32],
            storage_root: [2; 32],
            physical_digest_schema: CURRENT_DIGEST_SCHEMA_VERSION.raw(),
            history_digest_schema: CURRENT_DIGEST_SCHEMA_VERSION.raw(),
            evidence_manifest_digest: [3; 32],
            durability: HistoryDurability::GroupCommit {
                max_ticks: 32,
                max_wall_time_ms: 250,
            },
        })
    }

    fn event(&self, trace_id: TraceId) -> Result<Option<OwnedCausalRecord>, HistoryError> {
        Ok(self.records.iter().find_map(|record| match record {
            HistoryRecord::Causal(record) if record.trace_id() == trace_id => Some(record.clone()),
            _ => None,
        }))
    }

    fn event_by_id(&self, event_id: EventId) -> Result<Option<OwnedCausalRecord>, HistoryError> {
        Ok(self.records.iter().find_map(|record| match record {
            HistoryRecord::Causal(record) if record.event_id() == event_id => Some(record.clone()),
            _ => None,
        }))
    }

    fn records(&self, query: HistoryQuery) -> Result<HistoryPage<HistoryRecord>, HistoryError> {
        page_values(&self.records, query)
    }

    fn evidence(
        &self,
        _schema_id: EvidenceSchemaId,
        _canonical_key: &[u8],
        query: HistoryQuery,
    ) -> Result<HistoryPage<DomainEvidenceRecord>, HistoryError> {
        page_values(&[], query)
    }
}
