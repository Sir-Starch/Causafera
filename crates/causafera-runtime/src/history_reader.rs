use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex};

use causafera_core::{
    CausalHistoryReader, CausalTarget, DomainEvidenceRecord, EvidenceSchemaId, HistoryAnchor,
    HistoryAnchorFields, HistoryCheckpoint, HistoryDurability, HistoryError, HistoryFrontier,
    HistoryPage, HistoryPageItem, HistoryQuery, HistoryQueryOutcome, HistoryRecord,
    HistoryWalkBudget, OwnedCausalRecord,
};
use causafera_types::{EventId, TraceId};

use crate::{CURRENT_DIGEST_SCHEMA_VERSION, CanonicalDigest};

#[derive(Debug)]
pub(crate) struct RuntimeHistoryReader {
    inner: Mutex<HistoryIndex>,
}

#[derive(Clone, Debug)]
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
    overlay_evidence: BTreeMap<(EvidenceSchemaId, Vec<u8>), Vec<usize>>,
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
                    index
                        .overlay_evidence
                        .entry((record.schema_id(), record.canonical_key().to_vec()))
                        .or_default()
                        .push(position);
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
            .enumerate()
            .map(|(offset, record)| match record {
                HistoryRecord::Causal(record) => (
                    Some((record.trace_id(), record.event_id())),
                    None,
                    new_len + offset,
                ),
                HistoryRecord::Evidence(record) => (
                    None,
                    Some((record.schema_id(), record.canonical_key().to_vec())),
                    new_len + offset,
                ),
            })
            .collect::<Vec<_>>();
        for (causal, evidence, position) in removed {
            if let Some((trace_id, event_id)) = causal {
                index.overlay_events.remove(&trace_id);
                index.overlay_event_ids.remove(&event_id);
            }
            if let Some(key) = evidence
                && let Some(positions) = index.overlay_evidence.get_mut(&key)
            {
                positions.retain(|candidate| *candidate != position);
                let remove_key = positions.is_empty();
                if remove_key {
                    index.overlay_evidence.remove(&key);
                }
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
    fn pin_snapshot(&self) -> Result<Arc<dyn CausalHistoryReader>, HistoryError> {
        let mut snapshot = self
            .inner
            .lock()
            .map_err(|_| HistoryError::ReaderOperationUnavailable)?
            .clone();
        snapshot.cold = snapshot.cold.pin_snapshot()?;
        Ok(Arc::new(Self {
            inner: Mutex::new(snapshot),
        }))
    }

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
        let cold_count = cold.child_count(trace_id)?;
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
        if query.offset() < cold_count {
            page_values_at(&values, query, query.offset())
        } else {
            page_values_at(&values, query, cold_count)
        }
    }

    fn child_count(&self, trace_id: TraceId) -> Result<u64, HistoryError> {
        let (cold, overlay) = {
            let index = self
                .inner
                .lock()
                .map_err(|_| HistoryError::ReaderOperationUnavailable)?;
            let overlay = index
                .overlay
                .iter()
                .filter(|record| matches!(record, HistoryRecord::Causal(event) if event.causes().contains(&trace_id)))
                .count();
            (Arc::clone(&index.cold), overlay)
        };
        cold.child_count(trace_id)?
            .checked_add(u64::try_from(overlay).map_err(|_| HistoryError::ArithmeticOverflow)?)
            .ok_or(HistoryError::ArithmeticOverflow)
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
        let cold_count = cold.evidence_count(schema_id, canonical_key)?;
        let page = cold.evidence(schema_id, canonical_key, query)?;
        if page.next_offset().is_some() {
            return Ok(page);
        }
        let mut values = page.records().to_vec();
        let index = self
            .inner
            .lock()
            .map_err(|_| HistoryError::ReaderOperationUnavailable)?;
        if let Some(positions) = index
            .overlay_evidence
            .get(&(schema_id, canonical_key.to_vec()))
        {
            values.extend(positions.iter().filter_map(
                |position| match index.overlay.get(*position) {
                    Some(HistoryRecord::Evidence(record)) => Some(record.clone()),
                    _ => None,
                },
            ));
        }
        if query.offset() < cold_count {
            page_values_at(&values, query, query.offset())
        } else {
            page_values_at(&values, query, cold_count)
        }
    }

    fn evidence_count(
        &self,
        schema_id: EvidenceSchemaId,
        canonical_key: &[u8],
    ) -> Result<u64, HistoryError> {
        let (cold, overlay) = {
            let index = self
                .inner
                .lock()
                .map_err(|_| HistoryError::ReaderOperationUnavailable)?;
            let overlay = index
                .overlay_evidence
                .get(&(schema_id, canonical_key.to_vec()))
                .map_or(0, Vec::len);
            (Arc::clone(&index.cold), overlay)
        };
        cold.evidence_count(schema_id, canonical_key)?
            .checked_add(u64::try_from(overlay).map_err(|_| HistoryError::ArithmeticOverflow)?)
            .ok_or(HistoryError::ArithmeticOverflow)
    }

    fn evidence_by_anchor(
        &self,
        schema_id: EvidenceSchemaId,
        anchor: TraceId,
        query: HistoryQuery,
    ) -> Result<HistoryPage<DomainEvidenceRecord>, HistoryError> {
        let cold = {
            let index = self
                .inner
                .lock()
                .map_err(|_| HistoryError::ReaderOperationUnavailable)?;
            Arc::clone(&index.cold)
        };
        let cold_count = cold.evidence_by_anchor_count(schema_id, anchor)?;
        let page = cold.evidence_by_anchor(schema_id, anchor, query)?;
        if page.next_offset().is_some() {
            return Ok(page);
        }
        let mut values = page.records().to_vec();
        let index = self
            .inner
            .lock()
            .map_err(|_| HistoryError::ReaderOperationUnavailable)?;
        values.extend(index.overlay.iter().filter_map(|record| match record {
            HistoryRecord::Evidence(record)
                if record.schema_id() == schema_id && record.causal_anchors().contains(&anchor) =>
            {
                Some(record.clone())
            }
            _ => None,
        }));
        let base_offset = if query.offset() < cold_count {
            query.offset()
        } else {
            cold_count
        };
        page_values_at(&values, query, base_offset)
    }

    fn evidence_by_anchor_count(
        &self,
        schema_id: EvidenceSchemaId,
        anchor: TraceId,
    ) -> Result<u64, HistoryError> {
        let (cold, overlay) = {
            let index = self
                .inner
                .lock()
                .map_err(|_| HistoryError::ReaderOperationUnavailable)?;
            let overlay = index
                .overlay
                .iter()
                .filter(|record| {
                    matches!(record, HistoryRecord::Evidence(evidence) if evidence.schema_id() == schema_id && evidence.causal_anchors().contains(&anchor))
                })
                .count();
            (Arc::clone(&index.cold), overlay)
        };
        cold.evidence_by_anchor_count(schema_id, anchor)?
            .checked_add(u64::try_from(overlay).map_err(|_| HistoryError::ArithmeticOverflow)?)
            .ok_or(HistoryError::ArithmeticOverflow)
    }

    fn evidence_by_key_prefix(
        &self,
        schema_id: EvidenceSchemaId,
        canonical_key_prefix: &[u8],
        query: HistoryQuery,
    ) -> Result<HistoryPage<DomainEvidenceRecord>, HistoryError> {
        let cold = {
            let index = self
                .inner
                .lock()
                .map_err(|_| HistoryError::ReaderOperationUnavailable)?;
            Arc::clone(&index.cold)
        };
        let cold_count = cold.evidence_by_key_prefix_count(schema_id, canonical_key_prefix)?;
        let page = cold.evidence_by_key_prefix(schema_id, canonical_key_prefix, query)?;
        if page.next_offset().is_some() {
            return Ok(page);
        }
        let mut values = page.records().to_vec();
        let index = self
            .inner
            .lock()
            .map_err(|_| HistoryError::ReaderOperationUnavailable)?;
        values.extend(index.overlay.iter().filter_map(|record| match record {
            HistoryRecord::Evidence(record)
                if record.schema_id() == schema_id
                    && record.canonical_key().starts_with(canonical_key_prefix) =>
            {
                Some(record.clone())
            }
            _ => None,
        }));
        let base_offset = if query.offset() < cold_count {
            query.offset()
        } else {
            cold_count
        };
        page_values_at(&values, query, base_offset)
    }

    fn evidence_by_key_prefix_count(
        &self,
        schema_id: EvidenceSchemaId,
        canonical_key_prefix: &[u8],
    ) -> Result<u64, HistoryError> {
        let (cold, overlay) = {
            let index = self
                .inner
                .lock()
                .map_err(|_| HistoryError::ReaderOperationUnavailable)?;
            let overlay = index
                .overlay
                .iter()
                .filter(|record| {
                    matches!(record, HistoryRecord::Evidence(evidence) if evidence.schema_id() == schema_id && evidence.canonical_key().starts_with(canonical_key_prefix))
                })
                .count();
            (Arc::clone(&index.cold), overlay)
        };
        cold.evidence_by_key_prefix_count(schema_id, canonical_key_prefix)?
            .checked_add(u64::try_from(overlay).map_err(|_| HistoryError::ArithmeticOverflow)?)
            .ok_or(HistoryError::ArithmeticOverflow)
    }

    fn walk_ancestors(
        &self,
        trace_id: TraceId,
        max_depth: usize,
        query: HistoryQuery,
    ) -> Result<HistoryPage<TraceId>, HistoryError> {
        match self.walk_ancestors_bounded(
            trace_id,
            HistoryWalkBudget::new(max_depth, HistoryWalkBudget::MAX_NODES)?,
            query,
        )? {
            HistoryQueryOutcome::Complete(page) | HistoryQueryOutcome::Partial(page) => Ok(page),
            HistoryQueryOutcome::Missing => HistoryPage::new(Vec::new(), None, query),
            HistoryQueryOutcome::Unknown => Err(HistoryError::ReaderOperationUnavailable),
            HistoryQueryOutcome::IntegrityError(error) => Err(error),
        }
    }

    fn checkpoint_at(
        &self,
        time: causafera_types::SimulationTime,
    ) -> Result<Option<HistoryCheckpoint>, HistoryError> {
        let cold = {
            let index = self
                .inner
                .lock()
                .map_err(|_| HistoryError::ReaderOperationUnavailable)?;
            index.cold.pin_snapshot()?
        };
        cold.checkpoint_at(time)
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

#[cfg(test)]
mod pagination_regression_tests {
    use super::*;
    use causafera_core::{
        CausalEffect, CausalTarget, HistoryQueryStatus, HistoryReader, Phase, StateFingerprint,
    };
    use causafera_types::{EventKindId, SimulationTime, StateObjectKindId, StatePropertyId};

    #[test]
    fn pin_snapshot_keeps_generation_and_overlay_after_live_append() {
        let effect = CausalEffect::new(
            CausalTarget::new(StateObjectKindId::new(8), 9, StatePropertyId::new(10)),
            StateFingerprint::new([1; 32]),
            StateFingerprint::new([2; 32]),
        )
        .expect("effect is valid");
        let root = OwnedCausalRecord::new(
            0,
            EventId::new(0),
            TraceId::new(0),
            SimulationTime::new(0),
            Phase::Physics,
            EventKindId::new(6),
            Vec::new(),
            vec![effect],
        )
        .expect("root is valid");
        let next = OwnedCausalRecord::new(
            1,
            EventId::new(1),
            TraceId::new(1),
            SimulationTime::new(1),
            Phase::Physics,
            EventKindId::new(6),
            vec![TraceId::new(0)],
            vec![effect],
        )
        .expect("next is valid");
        let reader = RuntimeHistoryReader::new_for_test([0; 16], vec![root]).expect("reader");
        let pinned = reader.pin_snapshot().expect("pinned reader");

        reader
            .append(
                1,
                &[HistoryRecord::Causal(next.clone())],
                [3; 32],
                [4; 32],
                [5; 32],
            )
            .expect("append");

        assert_eq!(
            pinned.anchor().expect("pinned anchor").logical_generation(),
            0
        );
        assert_eq!(pinned.event(next.trace_id()).expect("pinned event"), None);
        assert_eq!(
            reader.event(next.trace_id()).expect("live event"),
            Some(next)
        );
    }

    #[test]
    fn ancestor_query_returns_partial_at_its_node_budget() {
        let effect = CausalEffect::new(
            CausalTarget::new(StateObjectKindId::new(8), 9, StatePropertyId::new(10)),
            StateFingerprint::new([1; 32]),
            StateFingerprint::new([2; 32]),
        )
        .expect("effect is valid");
        let records = (0..5)
            .map(|index| {
                OwnedCausalRecord::new(
                    index,
                    EventId::new(index),
                    TraceId::new(index),
                    SimulationTime::new(index),
                    Phase::Physics,
                    EventKindId::new(6),
                    (index > 0)
                        .then(|| TraceId::new(index - 1))
                        .into_iter()
                        .collect(),
                    vec![effect],
                )
                .expect("chain record is valid")
            })
            .collect();
        let reader = RuntimeHistoryReader::new_for_test([0; 16], records).expect("reader");
        let outcome = reader
            .walk_ancestors_bounded(
                TraceId::new(4),
                HistoryWalkBudget::new(10, 2).expect("budget"),
                HistoryQuery::new(0, 256, 4 * 1024 * 1024).expect("page"),
            )
            .expect("bounded query");

        let HistoryQueryOutcome::Partial(page) = outcome else {
            panic!("node budget must produce a partial outcome");
        };
        assert_eq!(page.records(), &[TraceId::new(2), TraceId::new(3)]);
    }

    #[test]
    fn ancestor_query_reports_missing_unknown_and_integrity_failure_distinctly() {
        let budget = HistoryWalkBudget::new(10, 10).expect("budget");
        let query = HistoryQuery::page(0).expect("page");
        let missing = RuntimeHistoryReader::new_for_test([0; 16], Vec::new())
            .expect("empty reader")
            .walk_ancestors_bounded(TraceId::new(99), budget, query)
            .expect("missing query");
        assert_eq!(missing.status(), HistoryQueryStatus::Missing);

        let effect = CausalEffect::new(
            CausalTarget::new(StateObjectKindId::new(8), 9, StatePropertyId::new(10)),
            StateFingerprint::new([1; 32]),
            StateFingerprint::new([2; 32]),
        )
        .expect("effect is valid");
        let root = OwnedCausalRecord::new(
            0,
            EventId::new(0),
            TraceId::new(0),
            SimulationTime::new(0),
            Phase::Physics,
            EventKindId::new(6),
            Vec::new(),
            vec![effect],
        )
        .expect("root is valid");
        let unavailable_reader =
            ParentFailureReader::new(root.clone(), HistoryError::ReaderOperationUnavailable);
        let unavailable = unavailable_reader
            .walk_ancestors_bounded(TraceId::new(0), budget, query)
            .expect("unavailable ancestry query");
        assert_eq!(unavailable.status(), HistoryQueryStatus::Unknown);

        let integrity_reader = ParentFailureReader::new(
            root,
            HistoryError::IntegrityFailure {
                details: "injected parent-index corruption".to_owned(),
            },
        );
        let integrity = integrity_reader
            .walk_ancestors_bounded(TraceId::new(0), budget, query)
            .expect("integrity outcome is explicit");
        assert_eq!(integrity.status(), HistoryQueryStatus::IntegrityError);
        assert!(matches!(
            integrity,
            HistoryQueryOutcome::IntegrityError(HistoryError::IntegrityFailure { .. })
        ));
    }

    #[test]
    fn ancestor_cursor_advances_across_pages_without_repeating_ancestors() {
        let effect = CausalEffect::new(
            CausalTarget::new(StateObjectKindId::new(8), 9, StatePropertyId::new(10)),
            StateFingerprint::new([1; 32]),
            StateFingerprint::new([2; 32]),
        )
        .expect("effect is valid");
        let records = (0..3)
            .map(|index| {
                OwnedCausalRecord::new(
                    index,
                    EventId::new(index),
                    TraceId::new(index),
                    SimulationTime::new(index),
                    Phase::Physics,
                    EventKindId::new(6),
                    (index > 0)
                        .then(|| TraceId::new(index - 1))
                        .into_iter()
                        .collect(),
                    vec![effect],
                )
                .expect("chain record is valid")
            })
            .collect();
        let reader = RuntimeHistoryReader::new_for_test([0; 16], records).expect("reader");
        let cursor = reader
            .ancestor_cursor(TraceId::new(2))
            .expect("cursor creation")
            .expect("root event exists");
        let budget = HistoryWalkBudget::new(10, 10).expect("budget");
        let query = HistoryQuery::new(0, 1, 8).expect("one-record page");

        let first = reader
            .walk_ancestor_page(cursor, budget, query)
            .expect("first ancestor page");
        assert_eq!(first.page().records(), &[TraceId::new(1)]);
        assert_eq!(first.status(), HistoryQueryStatus::Partial);
        let second = reader
            .walk_ancestor_page(
                first.into_next_cursor().expect("continuation"),
                budget,
                query,
            )
            .expect("second ancestor page");
        assert_eq!(second.page().records(), &[TraceId::new(0)]);
        assert_eq!(second.status(), HistoryQueryStatus::Complete);
        assert!(second.next_cursor().is_none());
    }

    #[test]
    fn evidence_pagination_uses_absolute_offset_across_cold_and_overlay_records() {
        use causafera_core::{Criticality, EvidenceSchemaVersion, StateFingerprint};
        use causafera_types::SimulationTime;

        let schema_id = EvidenceSchemaId::new(71);
        let key = b"shared-test-key";
        let evidence = |sequence, payload: &[u8]| {
            DomainEvidenceRecord::new(
                sequence,
                schema_id,
                EvidenceSchemaVersion::new(1, 0),
                Criticality::ExplanationOnly,
                SimulationTime::new(sequence),
                vec![TraceId::new(17)],
                key.to_vec(),
                payload.to_vec(),
                StateFingerprint::new([sequence as u8; 32]),
            )
            .expect("evidence is valid")
        };
        let cold = Arc::new(TestColdReader::with_evidence(
            [0; 16],
            Vec::new(),
            vec![evidence(0, b"cold-0"), evidence(1, b"cold-1")],
        ));
        let reader = RuntimeHistoryReader::new(cold, 2, 0).expect("reader");
        let overlay = evidence(2, b"overlay-2");
        {
            let mut index = reader.inner.lock().expect("reader lock");
            index.overlay.push(HistoryRecord::Evidence(overlay.clone()));
            index
                .overlay_evidence
                .insert((schema_id, key.to_vec()), vec![0]);
        }

        let query = HistoryQuery::new(1, 2, 4096).expect("query");
        let page = reader.evidence(schema_id, key, query).expect("page");
        assert_eq!(page.records(), &[evidence(1, b"cold-1"), overlay]);
        assert_eq!(page.next_offset(), None);

        let query = HistoryQuery::new(2, 1, 4096).expect("query");
        let page = reader.evidence(schema_id, key, query).expect("page");
        assert_eq!(page.records(), &[evidence(2, b"overlay-2")]);
        assert_eq!(page.next_offset(), None);

        let page = reader
            .evidence_by_anchor(
                schema_id,
                TraceId::new(17),
                HistoryQuery::new(1, 2, 4096).expect("anchor query"),
            )
            .expect("anchored page");
        assert_eq!(
            page.records(),
            &[evidence(1, b"cold-1"), evidence(2, b"overlay-2")]
        );
        assert_eq!(page.next_offset(), None);

        let page = reader
            .evidence_by_key_prefix(
                schema_id,
                b"shared-",
                HistoryQuery::new(1, 2, 4096).expect("prefix query"),
            )
            .expect("prefix page");
        assert_eq!(
            page.records(),
            &[evidence(1, b"cold-1"), evidence(2, b"overlay-2")]
        );
        assert_eq!(page.next_offset(), None);
    }

    #[derive(Debug)]
    struct ParentFailureReader {
        inner: TestColdReader,
        error: HistoryError,
    }

    impl ParentFailureReader {
        fn new(root: OwnedCausalRecord, error: HistoryError) -> Self {
            Self {
                inner: TestColdReader::new([0; 16], vec![root]).expect("cold reader"),
                error,
            }
        }
    }

    impl CausalHistoryReader for ParentFailureReader {
        fn anchor(&self) -> Result<HistoryAnchor, HistoryError> {
            self.inner.anchor()
        }

        fn event(&self, trace_id: TraceId) -> Result<Option<OwnedCausalRecord>, HistoryError> {
            self.inner.event(trace_id)
        }

        fn records(&self, query: HistoryQuery) -> Result<HistoryPage<HistoryRecord>, HistoryError> {
            self.inner.records(query)
        }

        fn parents(
            &self,
            _trace_id: TraceId,
            _query: HistoryQuery,
        ) -> Result<HistoryPage<TraceId>, HistoryError> {
            Err(self.error.clone())
        }
    }
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
    evidence: Vec<DomainEvidenceRecord>,
}

#[cfg(test)]
impl TestColdReader {
    fn new(namespace: [u8; 16], records: Vec<OwnedCausalRecord>) -> Result<Self, HistoryError> {
        Ok(Self {
            namespace,
            records: records.into_iter().map(HistoryRecord::Causal).collect(),
            evidence: Vec::new(),
        })
    }

    fn with_evidence(
        namespace: [u8; 16],
        records: Vec<OwnedCausalRecord>,
        evidence: Vec<DomainEvidenceRecord>,
    ) -> Self {
        Self {
            namespace,
            records: records.into_iter().map(HistoryRecord::Causal).collect(),
            evidence,
        }
    }
}

#[cfg(test)]
impl CausalHistoryReader for TestColdReader {
    fn pin_snapshot(&self) -> Result<Arc<dyn CausalHistoryReader>, HistoryError> {
        Ok(Arc::new(Self {
            namespace: self.namespace,
            records: self.records.clone(),
            evidence: self.evidence.clone(),
        }))
    }

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
        schema_id: EvidenceSchemaId,
        canonical_key: &[u8],
        query: HistoryQuery,
    ) -> Result<HistoryPage<DomainEvidenceRecord>, HistoryError> {
        let matching = self
            .evidence
            .iter()
            .filter(|record| {
                record.schema_id() == schema_id && record.canonical_key() == canonical_key
            })
            .cloned()
            .collect::<Vec<_>>();
        page_values(&matching, query)
    }

    fn evidence_by_anchor(
        &self,
        schema_id: EvidenceSchemaId,
        anchor: TraceId,
        query: HistoryQuery,
    ) -> Result<HistoryPage<DomainEvidenceRecord>, HistoryError> {
        let matching = self
            .evidence
            .iter()
            .filter(|record| {
                record.schema_id() == schema_id && record.causal_anchors().contains(&anchor)
            })
            .cloned()
            .collect::<Vec<_>>();
        page_values(&matching, query)
    }

    fn evidence_by_anchor_count(
        &self,
        schema_id: EvidenceSchemaId,
        anchor: TraceId,
    ) -> Result<u64, HistoryError> {
        Ok(self
            .evidence
            .iter()
            .filter(|record| {
                record.schema_id() == schema_id && record.causal_anchors().contains(&anchor)
            })
            .count() as u64)
    }

    fn evidence_by_key_prefix(
        &self,
        schema_id: EvidenceSchemaId,
        canonical_key_prefix: &[u8],
        query: HistoryQuery,
    ) -> Result<HistoryPage<DomainEvidenceRecord>, HistoryError> {
        let matching = self
            .evidence
            .iter()
            .filter(|record| {
                record.schema_id() == schema_id
                    && record.canonical_key().starts_with(canonical_key_prefix)
            })
            .cloned()
            .collect::<Vec<_>>();
        page_values(&matching, query)
    }

    fn evidence_by_key_prefix_count(
        &self,
        schema_id: EvidenceSchemaId,
        canonical_key_prefix: &[u8],
    ) -> Result<u64, HistoryError> {
        Ok(self
            .evidence
            .iter()
            .filter(|record| {
                record.schema_id() == schema_id
                    && record.canonical_key().starts_with(canonical_key_prefix)
            })
            .count() as u64)
    }

    fn evidence_count(
        &self,
        schema_id: EvidenceSchemaId,
        canonical_key: &[u8],
    ) -> Result<u64, HistoryError> {
        Ok(self
            .evidence
            .iter()
            .filter(|record| {
                record.schema_id() == schema_id && record.canonical_key() == canonical_key
            })
            .count() as u64)
    }
}
