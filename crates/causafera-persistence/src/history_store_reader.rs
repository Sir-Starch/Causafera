//! Catalog-backed history reads.
//!
//! The reader deliberately keeps only its anchor and the bounded logical
//! overlay. Durable records remain in CFHF files and are located through the
//! SQLite indexes; each operation decodes only the indexed frame or bounded
//! page it needs. Complete-history materialization belongs to the explicit
//! cold/recovery APIs on `HistoryStore`, never to this ordinary reader.

use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

#[cfg(test)]
use std::sync::atomic::{AtomicUsize, Ordering};

use causafera_core::{
    CausalTarget, DomainEvidenceRecord, EvidenceDecoderRegistry, EvidenceSchemaId, HistoryAnchor,
    HistoryAnchorFields, HistoryDurability, HistoryError, HistoryFrontier, HistoryPage,
    HistoryPageItem, HistoryQuery, HistoryReader, HistoryRecord, OwnedCausalRecord,
};
use causafera_types::{EventId, TraceId};
use rusqlite::{Connection, OpenFlags, OptionalExtension, params};

use crate::{
    CatalogCheckpoint, DecodedHistoryFrame, DurabilityMode, PersistenceError, decode_history_frame,
    history_store::{
        MAX_STORED_FRAME_BYTES, from_i64, key, read_exact_range, resolve_segment_path,
    },
};

const GROUP_MAX_TICKS: u16 = 32;
const GROUP_MAX_MILLIS: u32 = 250;
const CAUSAL_OVERLAY_QUERY_IDS: usize = 900;
const MAX_CACHED_FRAMES: usize = 8;

#[cfg(test)]
static CAUSAL_OVERLAY_EVENT_INDEX_BATCHES: AtomicUsize = AtomicUsize::new(0);

#[derive(Clone, Copy)]
struct IndexedCausalLocation {
    trace_id: TraceId,
    event_id: EventId,
    time: u64,
    frame_ordinal: u64,
    record_ordinal: u64,
}

#[derive(Clone)]
pub struct HistoryStoreReader {
    source: Arc<ReaderSource>,
    anchor: HistoryAnchor,
    overlay: Vec<HistoryRecord>,
}

struct ReaderSource {
    root: PathBuf,
    registry: Arc<EvidenceDecoderRegistry>,
    connection: Mutex<Connection>,
    frame_cache: Mutex<BTreeMap<u64, Arc<DecodedHistoryFrame>>>,
    /// Frames whose derived index rows this reader has already checked against
    /// their canonical records. Frames are immutable, so one check per reader is
    /// enough; without it every page re-validates its whole frame.
    validated_frames: Mutex<BTreeSet<u64>>,
    namespace: [u8; 32],
    cold_generation: u64,
}

impl fmt::Debug for HistoryStoreReader {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("HistoryStoreReader")
            .field("logical_generation", &self.anchor.logical_generation())
            .field("durable_generation", &self.anchor.durable_generation())
            .field("cold_generation", &self.source.cold_generation)
            .field("bounded_overlay_records", &self.overlay.len())
            .finish()
    }
}

impl HistoryStoreReader {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new_indexed(
        root: &Path,
        registry: Arc<EvidenceDecoderRegistry>,
        namespace: [u8; 32],
        mode: DurabilityMode,
        logical_generation: u64,
        durable_generation: u64,
        cold_generation: u64,
        frontier: HistoryFrontier,
        checkpoint: Option<CatalogCheckpoint>,
        logical_digest: [u8; 32],
        evidence_manifest_digest: [u8; 32],
        overlay: Vec<HistoryRecord>,
    ) -> Result<Self, PersistenceError> {
        let connection = open_reader_connection(root)?;
        connection
            .busy_timeout(std::time::Duration::from_millis(0))
            .map_err(|error| PersistenceError::HistoryCatalog {
                operation: "configure catalog reader timeout",
                code: None,
                message: error.to_string(),
            })?;
        validate_catalog_indexes(
            &connection,
            root,
            registry.as_ref(),
            namespace,
            cold_generation,
        )?;
        let checkpoint_digest = checkpoint.as_ref().map_or([0; 32], |value| value.digest);
        let anchor = HistoryAnchor::new(HistoryAnchorFields {
            namespace: namespace[..16].try_into().map_err(|_| {
                PersistenceError::HistoryCatalogInvariant {
                    invariant: "namespace width",
                }
            })?,
            logical_generation,
            durable_generation,
            checkpoint_digest,
            frontier,
            logical_digest,
            storage_root: logical_digest,
            physical_digest_schema: crate::CURRENT_DIGEST_SCHEMA_VERSION,
            history_digest_schema: crate::CURRENT_DIGEST_SCHEMA_VERSION,
            evidence_manifest_digest,
            durability: match mode {
                DurabilityMode::GroupCommit => HistoryDurability::GroupCommit {
                    max_ticks: GROUP_MAX_TICKS,
                    max_wall_time_ms: GROUP_MAX_MILLIS,
                },
                DurabilityMode::EveryTick => HistoryDurability::EveryTick,
            },
        })?;
        Ok(Self {
            source: Arc::new(ReaderSource {
                root: root.to_path_buf(),
                registry,
                connection: Mutex::new(connection),
                frame_cache: Mutex::new(BTreeMap::new()),
                validated_frames: Mutex::new(BTreeSet::new()),
                namespace,
                cold_generation,
            }),
            anchor,
            overlay,
        })
    }

    pub fn append_durable(
        &mut self,
        _records: &[HistoryRecord],
        anchor: HistoryAnchor,
    ) -> Result<(), PersistenceError> {
        let connection = open_reader_connection(&self.source.root)?;
        connection
            .busy_timeout(std::time::Duration::from_millis(0))
            .map_err(|error| PersistenceError::HistoryCatalog {
                operation: "configure catalog reader timeout",
                code: None,
                message: error.to_string(),
            })?;
        self.source = Arc::new(ReaderSource {
            root: self.source.root.clone(),
            registry: Arc::clone(&self.source.registry),
            connection: Mutex::new(connection),
            frame_cache: Mutex::new(BTreeMap::new()),
            validated_frames: Mutex::new(BTreeSet::new()),
            namespace: self.source.namespace,
            cold_generation: anchor.durable_generation(),
        });
        self.overlay.clear();
        self.anchor = anchor;
        Ok(())
    }

    pub fn all_records_for_oracle(&self) -> Result<Vec<OwnedCausalRecord>, HistoryError> {
        self.with_connection(|connection, source| {
            let mut statement = connection
                .prepare(
                    "SELECT f.frame_ordinal,f.segment_sequence,f.start_offset,f.end_offset,
                            f.frame_digest,s.relative_path,s.sealed,s.identity
                     FROM frames f JOIN segments s ON s.sequence=f.segment_sequence
                     WHERE f.generation<=?1 ORDER BY f.frame_ordinal",
                )
                .map_err(|error| catalog_error("prepare cold history oracle", error))?;
            let rows = statement
                .query_map([to_i64(source.cold_generation)?], |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, i64>(1)?,
                        row.get::<_, i64>(2)?,
                        row.get::<_, i64>(3)?,
                        row.get::<_, Vec<u8>>(4)?,
                        row.get::<_, String>(5)?,
                        row.get::<_, i64>(6)?,
                        row.get::<_, Option<Vec<u8>>>(7)?,
                    ))
                })
                .map_err(|error| catalog_error("query cold history oracle", error))?;
            let mut records = Vec::new();
            for row in rows {
                let (frame, segment, start, end, digest, relative, sealed, identity) =
                    row.map_err(|error| catalog_error("read cold history oracle", error))?;
                let identity = identity
                    .map(|bytes| {
                        bytes
                            .try_into()
                            .map_err(|_| PersistenceError::HistoryCatalogInvariant {
                                invariant: "segment identity width",
                            })
                    })
                    .transpose()?;
                let path = resolve_segment_path(
                    &source.root,
                    from_i64(segment)?,
                    sealed != 0,
                    identity,
                    Path::new(&relative),
                    true,
                )?;
                let bytes = read_exact_range(
                    &path,
                    from_i64(start)?..from_i64(end)?,
                    MAX_STORED_FRAME_BYTES,
                )?;
                let decoded = decode_history_frame(&bytes, source.registry.as_ref())?;
                if decoded.header().fields.namespace != source.namespace
                    || decoded.header().frame_digest.as_slice() != digest.as_slice()
                {
                    return Err(PersistenceError::HistoryCatalogInvariant {
                        invariant: "catalog frame does not match canonical frame",
                    });
                }
                let _ = frame;
                records.extend(decoded.records().iter().filter_map(|record| match record {
                    HistoryRecord::Causal(event) => Some(event.clone()),
                    HistoryRecord::Evidence(_) => None,
                }));
            }
            Ok(records)
        })
    }

    fn with_connection<T>(
        &self,
        operation: impl FnOnce(&Connection, &ReaderSource) -> Result<T, PersistenceError>,
    ) -> Result<T, HistoryError> {
        let connection = self
            .source
            .connection
            .lock()
            .map_err(|_| HistoryError::ReaderOperationUnavailable)?;
        operation(&connection, &self.source).map_err(|_| HistoryError::ReaderOperationUnavailable)
    }

    fn overlay_event(&self, trace_id: TraceId) -> Option<OwnedCausalRecord> {
        self.overlay.iter().rev().find_map(|record| match record {
            HistoryRecord::Causal(record) if record.trace_id() == trace_id => Some(record.clone()),
            HistoryRecord::Causal(_) => None,
            HistoryRecord::Evidence(_) => None,
        })
    }

    fn overlay_event_by_id(&self, event_id: EventId) -> Option<OwnedCausalRecord> {
        self.overlay.iter().rev().find_map(|record| match record {
            HistoryRecord::Causal(record) if record.event_id() == event_id => Some(record.clone()),
            HistoryRecord::Causal(_) => None,
            HistoryRecord::Evidence(_) => None,
        })
    }

    fn page<T: HistoryPageItem + Clone>(
        values: &[T],
        query: HistoryQuery,
    ) -> Result<HistoryPage<T>, HistoryError> {
        let start =
            usize::try_from(query.offset()).map_err(|_| HistoryError::ArithmeticOverflow)?;
        let mut selected = Vec::new();
        let mut bytes = 0_usize;
        for value in values.iter().skip(start) {
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
        let consumed = query
            .offset()
            .checked_add(
                u64::try_from(selected.len()).map_err(|_| HistoryError::ArithmeticOverflow)?,
            )
            .ok_or(HistoryError::ArithmeticOverflow)?;
        let end = u64::try_from(values.len())
            .map_err(|_| HistoryError::ArithmeticOverflow)?
            .checked_add(query.offset())
            .ok_or(HistoryError::ArithmeticOverflow)?;
        HistoryPage::new(selected, (consumed < end).then_some(consumed), query)
    }

    fn page_records_from_catalog(
        &self,
        query: HistoryQuery,
        indexed: bool,
    ) -> Result<HistoryPage<HistoryRecord>, HistoryError> {
        let offset = query.offset();
        let max_records = query.max_records();
        let max_bytes = query.max_bytes();
        let cold_generation = self.source.cold_generation;
        let mut values = Vec::new();
        let mut value_bytes = 0_usize;
        let mut next_offset = None;
        let mut position = 0_u64;
        self.with_connection(|connection, source| {
            let mut statement = connection
                .prepare(
                    "SELECT frame_ordinal,record_count FROM frames
                     WHERE generation<=?1 ORDER BY frame_ordinal",
                )
                .map_err(|error| catalog_error("prepare catalog record page", error))?;
            let rows = statement
                .query_map([to_i64(cold_generation)?], |row| {
                    Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?))
                })
                .map_err(|error| catalog_error("query catalog record page", error))?;
            for row in rows {
                let (frame, record_count) =
                    row.map_err(|error| catalog_error("read catalog record page", error))?;
                let frame = from_i64(frame)?;
                let record_count = from_i64(record_count)?;
                let frame_end = position
                    .checked_add(record_count)
                    .ok_or(PersistenceError::HistoryArithmeticOverflow)?;
                // A frame wholly before the page is skipped on the catalog's
                // word, so a page costs the frames it touches, not the history
                // before it.
                if frame_end <= offset {
                    position = frame_end;
                    continue;
                }
                let decoded =
                    indexed_frame(connection, source, frame)?.ok_or_else(index_mismatch)?;
                if u64::try_from(decoded.records().len())? != record_count {
                    return Err(index_mismatch());
                }
                if indexed {
                    let already_validated = source
                        .validated_frames
                        .lock()
                        .map_err(|_| PersistenceError::HistoryCatalogInvariant {
                            invariant: "history validated frame lock",
                        })?
                        .contains(&frame);
                    if !already_validated {
                        validate_indexed_frame(
                            connection,
                            cold_generation,
                            frame,
                            decoded.records(),
                        )?;
                        source
                            .validated_frames
                            .lock()
                            .map_err(|_| PersistenceError::HistoryCatalogInvariant {
                                invariant: "history validated frame lock",
                            })?
                            .insert(frame);
                    }
                }
                for record in decoded.records() {
                    if position < offset {
                        position = position
                            .checked_add(1)
                            .ok_or(PersistenceError::HistoryArithmeticOverflow)?;
                        continue;
                    }
                    if values.len() == max_records {
                        next_offset = Some(position);
                        return Ok(());
                    }
                    let next_bytes = value_bytes
                        .checked_add(record.decoded_size())
                        .ok_or(PersistenceError::HistoryArithmeticOverflow)?;
                    if next_bytes > max_bytes {
                        next_offset = Some(position);
                        return Ok(());
                    }
                    value_bytes = next_bytes;
                    values.push(record.clone());
                    position = position
                        .checked_add(1)
                        .ok_or(PersistenceError::HistoryArithmeticOverflow)?;
                }
            }
            Ok(())
        })?;
        if next_offset.is_none() {
            return self.page_overlay_records(values, position, query);
        }
        HistoryPage::new(values, next_offset, query)
    }

    fn page_overlay_records(
        &self,
        mut values: Vec<HistoryRecord>,
        cold_count: u64,
        query: HistoryQuery,
    ) -> Result<HistoryPage<HistoryRecord>, HistoryError> {
        let overlay_offset = query.offset().saturating_sub(cold_count);
        let mut index =
            usize::try_from(overlay_offset).map_err(|_| HistoryError::ArithmeticOverflow)?;
        while index < self.overlay.len() && values.len() < query.max_records() {
            let record = &self.overlay[index];
            let bytes = values
                .iter()
                .try_fold(0_usize, |total, value| {
                    total.checked_add(value.decoded_size())
                })
                .and_then(|total| total.checked_add(record.decoded_size()))
                .ok_or(HistoryError::ArithmeticOverflow)?;
            if bytes > query.max_bytes() {
                break;
            }
            values.push(record.clone());
            index += 1;
        }
        let consumed = query
            .offset()
            .checked_add(u64::try_from(values.len()).map_err(|_| HistoryError::ArithmeticOverflow)?)
            .ok_or(HistoryError::ArithmeticOverflow)?;
        HistoryPage::new(
            values,
            (index < self.overlay.len()).then_some(consumed),
            query,
        )
    }
}

impl HistoryReader for HistoryStoreReader {
    fn all_records_for_oracle(&self) -> Result<Vec<OwnedCausalRecord>, HistoryError> {
        HistoryStoreReader::all_records_for_oracle(self)
    }

    fn validate_causal_overlay(&self, records: &[OwnedCausalRecord]) -> Result<(), HistoryError> {
        if records.is_empty() {
            return Ok(());
        }

        let overlay_by_trace = self
            .overlay
            .iter()
            .filter_map(|record| match record {
                HistoryRecord::Causal(event) => Some((event.trace_id(), event)),
                HistoryRecord::Evidence(_) => None,
            })
            .collect::<BTreeMap<_, _>>();
        let cold_records = records
            .iter()
            .filter(|record| match overlay_by_trace.get(&record.trace_id()) {
                Some(stored) => *stored != *record,
                None => true,
            })
            .collect::<Vec<_>>();

        for record in records {
            if let Some(stored) = overlay_by_trace.get(&record.trace_id())
                && *stored != record
            {
                return Err(HistoryError::ColdRecordMismatch {
                    trace_id: record.trace_id(),
                });
            }
        }
        if cold_records.is_empty() {
            return Ok(());
        }

        let indexed = self.with_connection(|connection, source| {
            let mut locations = BTreeMap::new();
            for chunk in cold_records.chunks(CAUSAL_OVERLAY_QUERY_IDS) {
                #[cfg(test)]
                CAUSAL_OVERLAY_EVENT_INDEX_BATCHES.fetch_add(1, Ordering::SeqCst);
                let placeholders = (2..=chunk.len() + 1)
                    .map(|index| format!("?{index}"))
                    .collect::<Vec<_>>()
                    .join(",");
                let sql = format!(
                    "SELECT trace_id,event_id,frame_ordinal,record_ordinal,time
                     FROM events
                     WHERE generation<=?1 AND trace_id IN ({placeholders})"
                );
                let mut statement = connection
                    .prepare(&sql)
                    .map_err(|error| catalog_error("prepare causal overlay index", error))?;
                statement
                    .raw_bind_parameter(1, to_i64(source.cold_generation)?)
                    .map_err(|error| catalog_error("bind causal overlay generation", error))?;
                for (index, record) in chunk.iter().enumerate() {
                    statement
                        .raw_bind_parameter(index + 2, key(record.trace_id().raw()))
                        .map_err(|error| catalog_error("bind causal overlay trace", error))?;
                }
                let mut rows = statement.raw_query();
                while let Some(row) = rows
                    .next()
                    .map_err(|error| catalog_error("read causal overlay index", error))?
                {
                    let trace_id =
                        TraceId::new(key_from_bytes(row.get(0).map_err(|error| {
                            catalog_error("read causal overlay trace", error)
                        })?)?);
                    let location =
                        IndexedCausalLocation {
                            trace_id,
                            event_id: EventId::new(key_from_bytes(row.get(1).map_err(
                                |error| catalog_error("read causal overlay event", error),
                            )?)?),
                            frame_ordinal: from_i64(row.get(2).map_err(|error| {
                                catalog_error("read causal overlay frame", error)
                            })?)?,
                            record_ordinal: from_i64(row.get(3).map_err(|error| {
                                catalog_error("read causal overlay ordinal", error)
                            })?)?,
                            time: key_from_bytes(row.get(4).map_err(|error| {
                                catalog_error("read causal overlay time", error)
                            })?)?,
                        };
                    if locations.insert(trace_id, location).is_some() {
                        return Err(index_mismatch());
                    }
                }
            }

            let mut frames = BTreeMap::<u64, Arc<DecodedHistoryFrame>>::new();
            let mut stored = BTreeMap::new();
            for record in &cold_records {
                let Some(location) = locations.get(&record.trace_id()) else {
                    continue;
                };
                let frame = match frames.entry(location.frame_ordinal) {
                    std::collections::btree_map::Entry::Occupied(entry) => entry.into_mut(),
                    std::collections::btree_map::Entry::Vacant(entry) => {
                        let decoded = indexed_frame(connection, source, location.frame_ordinal)?
                            .ok_or_else(index_mismatch)?;
                        entry.insert(decoded)
                    }
                };
                let Some(HistoryRecord::Causal(event)) = frame.records().get(
                    usize::try_from(location.record_ordinal)
                        .map_err(|_| PersistenceError::HistoryArithmeticOverflow)?,
                ) else {
                    return Err(index_mismatch());
                };
                if event.trace_id() != location.trace_id
                    || event.event_id() != location.event_id
                    || event.time().raw() != location.time
                {
                    return Err(index_mismatch());
                }
                stored.insert(location.trace_id, event.clone());
            }
            Ok(stored)
        })?;

        for record in cold_records {
            if indexed.get(&record.trace_id()) != Some(record) {
                return Err(HistoryError::ColdRecordMismatch {
                    trace_id: record.trace_id(),
                });
            }
        }
        Ok(())
    }

    fn anchor(&self) -> Result<HistoryAnchor, HistoryError> {
        Ok(self.anchor.clone())
    }

    fn event(&self, trace_id: TraceId) -> Result<Option<OwnedCausalRecord>, HistoryError> {
        if let Some(record) = self.overlay_event(trace_id) {
            return Ok(Some(record));
        }
        self.with_connection(|connection, source| {
            let row = connection
                .query_row(
                    "SELECT event_id,frame_ordinal,record_ordinal,time FROM events
                     WHERE trace_id=?1 AND generation<=?2",
                    params![key(trace_id.raw()), to_i64(source.cold_generation)?],
                    |row| {
                        Ok((
                            row.get::<_, Vec<u8>>(0)?,
                            row.get::<_, i64>(1)?,
                            row.get::<_, i64>(2)?,
                            row.get::<_, Vec<u8>>(3)?,
                        ))
                    },
                )
                .optional()
                .map_err(|error| catalog_error("read event index", error))?;
            let Some((event_id, frame, ordinal, time)) = row else {
                return Ok(None);
            };
            let event_id = key_from_bytes(event_id)?;
            let time = key_from_bytes(time)?;
            let record = record_at(connection, source, from_i64(frame)?, from_i64(ordinal)?)?;
            let Some(HistoryRecord::Causal(event)) = record else {
                return Err(index_mismatch());
            };
            if event.trace_id() != trace_id
                || event.event_id().raw() != event_id
                || event.time().raw() != time
            {
                return Err(index_mismatch());
            }
            Ok(Some(event))
        })
    }

    fn event_by_id(&self, event_id: EventId) -> Result<Option<OwnedCausalRecord>, HistoryError> {
        if let Some(record) = self.overlay_event_by_id(event_id) {
            return Ok(Some(record));
        }
        self.with_connection(|connection, source| {
            let row = connection
                .query_row(
                    "SELECT trace_id,frame_ordinal,record_ordinal FROM events
                     WHERE event_id=?1 AND generation<=?2",
                    params![key(event_id.raw()), to_i64(source.cold_generation)?],
                    |row| {
                        Ok((
                            row.get::<_, Vec<u8>>(0)?,
                            row.get::<_, i64>(1)?,
                            row.get::<_, i64>(2)?,
                        ))
                    },
                )
                .optional()
                .map_err(|error| catalog_error("read event-id index", error))?;
            let Some((trace_id, frame, ordinal)) = row else {
                return Ok(None);
            };
            let trace_id = TraceId::new(key_from_bytes(trace_id)?);
            let record = record_at(connection, source, from_i64(frame)?, from_i64(ordinal)?)?;
            let Some(HistoryRecord::Causal(event)) = record else {
                return Err(index_mismatch());
            };
            if event.event_id() != event_id || event.trace_id() != trace_id {
                return Err(index_mismatch());
            }
            Ok(Some(event))
        })
    }

    fn records(&self, query: HistoryQuery) -> Result<HistoryPage<HistoryRecord>, HistoryError> {
        self.page_records_from_catalog(query, false)
    }

    fn indexed_range(
        &self,
        query: HistoryQuery,
    ) -> Result<HistoryPage<HistoryRecord>, HistoryError> {
        self.page_records_from_catalog(query, true)
    }

    fn parents(
        &self,
        trace_id: TraceId,
        query: HistoryQuery,
    ) -> Result<HistoryPage<TraceId>, HistoryError> {
        let parents = self
            .event(trace_id)?
            .map_or_else(Vec::new, |event| event.causes().to_vec());
        Self::page(&parents, query)
    }

    fn children(
        &self,
        trace_id: TraceId,
        query: HistoryQuery,
    ) -> Result<HistoryPage<TraceId>, HistoryError> {
        let mut values = Vec::new();
        self.with_connection(|connection, source| {
            let limit = u64::try_from(query.max_records())?
                .checked_add(1)
                .ok_or(PersistenceError::HistoryArithmeticOverflow)?;
            let mut statement = connection
                .prepare(
                    "SELECT child_trace FROM cause_edges WHERE parent_trace=?1 AND generation<=?2
                     ORDER BY child_trace,ordinal LIMIT ?3 OFFSET ?4",
                )
                .map_err(|error| catalog_error("prepare child index", error))?;
            let rows = statement
                .query_map(
                    params![
                        key(trace_id.raw()),
                        to_i64(source.cold_generation)?,
                        to_i64(limit)?,
                        to_i64(query.offset())?
                    ],
                    |row| row.get::<_, Vec<u8>>(0),
                )
                .map_err(|error| catalog_error("query child index", error))?;
            for row in rows {
                values.push(TraceId::new(key_from_bytes(
                    row.map_err(|error| catalog_error("read child index", error))?,
                )?));
            }
            Ok(())
        })?;
        let has_more = values.len() > query.max_records();
        if has_more {
            values.pop();
        }
        HistoryPage::new(
            values,
            has_more
                .then(|| query.offset() + u64::try_from(query.max_records()).unwrap_or(u64::MAX)),
            query,
        )
    }

    fn latest_matching_event_before(
        &self,
        target: CausalTarget,
        trace_id: TraceId,
    ) -> Result<Option<OwnedCausalRecord>, HistoryError> {
        if let Some(record) = self.overlay.iter().rev().find_map(|record| match record {
            HistoryRecord::Causal(event)
                if event.trace_id() < trace_id
                    && event
                        .effects()
                        .iter()
                        .any(|effect| effect.target() == target) =>
            {
                Some(event.clone())
            }
            HistoryRecord::Causal(_) => None,
            HistoryRecord::Evidence(_) => None,
        }) {
            return Ok(Some(record));
        }
        self.with_connection(|connection, source| {
            let row = connection
                .query_row(
                    "SELECT e.frame_ordinal,e.record_ordinal FROM effect_edges x
                     JOIN events e ON e.trace_id=x.trace_id
                     WHERE x.object_kind=?1 AND x.object_id=?2 AND x.property_id=?3
                       AND x.trace_id<?4 AND x.generation<=?5
                     ORDER BY x.trace_id DESC LIMIT 1",
                    params![
                        key(target.object_kind().raw()),
                        key(target.object_id()),
                        key(target.property().raw()),
                        key(trace_id.raw()),
                        to_i64(source.cold_generation)?
                    ],
                    |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
                )
                .optional()
                .map_err(|error| catalog_error("read latest effect index", error))?;
            let Some((frame, ordinal)) = row else {
                return Ok(None);
            };
            let record = record_at(connection, source, from_i64(frame)?, from_i64(ordinal)?)?;
            match record {
                Some(HistoryRecord::Causal(event)) => Ok(Some(event)),
                _ => Err(index_mismatch()),
            }
        })
    }

    fn evidence(
        &self,
        schema_id: EvidenceSchemaId,
        canonical_key: &[u8],
        query: HistoryQuery,
    ) -> Result<HistoryPage<DomainEvidenceRecord>, HistoryError> {
        let mut values = Vec::new();
        let mut has_more = false;
        self.with_connection(|connection, source| {
            let limit = u64::try_from(query.max_records())?
                .checked_add(1)
                .ok_or(PersistenceError::HistoryArithmeticOverflow)?;
            let mut statement = connection
                .prepare(
                    "SELECT frame_ordinal,record_ordinal,payload_digest FROM evidence
                     WHERE schema_id=?1 AND canonical_key=?2 AND generation<=?3
                     ORDER BY frame_ordinal,record_ordinal LIMIT ?4 OFFSET ?5",
                )
                .map_err(|error| catalog_error("prepare evidence index", error))?;
            let rows = statement
                .query_map(
                    params![
                        key(schema_id.raw()),
                        canonical_key,
                        to_i64(source.cold_generation)?,
                        to_i64(limit)?,
                        to_i64(query.offset())?
                    ],
                    |row| {
                        Ok((
                            row.get::<_, i64>(0)?,
                            row.get::<_, i64>(1)?,
                            row.get::<_, Vec<u8>>(2)?,
                        ))
                    },
                )
                .map_err(|error| catalog_error("query evidence index", error))?;
            for row in rows {
                let (frame, ordinal, digest) =
                    row.map_err(|error| catalog_error("read evidence index", error))?;
                let record = record_at(connection, source, from_i64(frame)?, from_i64(ordinal)?)?;
                let Some(HistoryRecord::Evidence(evidence)) = record else {
                    return Err(index_mismatch());
                };
                if evidence.schema_id() != schema_id
                    || evidence.canonical_key() != canonical_key
                    || evidence.payload_digest().bytes().as_slice() != digest.as_slice()
                {
                    return Err(index_mismatch());
                }
                if values.len() == query.max_records() {
                    has_more = true;
                    break;
                }
                values.push(evidence);
            }
            Ok(())
        })?;
        HistoryPage::new(
            values,
            has_more
                .then(|| query.offset() + u64::try_from(query.max_records()).unwrap_or(u64::MAX)),
            query,
        )
    }

    fn walk_ancestors(
        &self,
        trace_id: TraceId,
        max_depth: usize,
        query: HistoryQuery,
    ) -> Result<HistoryPage<TraceId>, HistoryError> {
        let mut frontier = vec![(trace_id, 0_usize)];
        let mut visited = BTreeSet::new();
        while let Some((current, depth)) = frontier.pop() {
            if depth >= max_depth {
                continue;
            }
            let parents = self.parents(current, HistoryQuery::new(0, 64, 64 * 1024)?)?;
            for parent in parents.records().iter().rev() {
                if visited.insert(*parent) {
                    frontier.push((*parent, depth + 1));
                }
            }
        }
        Self::page(&visited.into_iter().collect::<Vec<_>>(), query)
    }
}

pub(crate) fn validate_catalog_indexes(
    connection: &Connection,
    root: &Path,
    registry: &EvidenceDecoderRegistry,
    namespace: [u8; 32],
    generation: u64,
) -> Result<(), PersistenceError> {
    let mut statement = connection
        .prepare(
            "SELECT f.generation,f.frame_ordinal,f.segment_sequence,f.start_offset,
                    f.end_offset,f.frame_digest,f.record_count,s.relative_path,s.sealed,s.identity
             FROM frames f JOIN segments s ON s.sequence=f.segment_sequence
             WHERE f.generation<=?1 ORDER BY f.frame_ordinal",
        )
        .map_err(|error| catalog_error("prepare catalog index validation", error))?;
    let rows = statement
        .query_map([to_i64(generation)?], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, i64>(4)?,
                row.get::<_, Vec<u8>>(5)?,
                row.get::<_, i64>(6)?,
                row.get::<_, String>(7)?,
                row.get::<_, i64>(8)?,
                row.get::<_, Option<Vec<u8>>>(9)?,
            ))
        })
        .map_err(|error| catalog_error("query catalog index validation", error))?;
    let mut expected = (0_u64, 0_u64, 0_u64, 0_u64);
    for row in rows {
        let (
            _row_generation,
            frame,
            segment,
            start,
            end,
            digest,
            record_count,
            relative,
            sealed,
            identity,
        ) = row.map_err(|error| catalog_error("read catalog index validation", error))?;
        if sealed != 0 && sealed != 1 {
            return Err(index_mismatch());
        }
        let segment = from_i64(segment)?;
        let identity = identity
            .map(|bytes| {
                bytes
                    .try_into()
                    .map_err(|_| PersistenceError::HistoryCatalogInvariant {
                        invariant: "segment identity width",
                    })
            })
            .transpose()?;
        let path = resolve_segment_path(
            root,
            segment,
            sealed != 0,
            identity,
            Path::new(&relative),
            true,
        )?;
        let bytes = read_exact_range(
            &path,
            from_i64(start)?..from_i64(end)?,
            MAX_STORED_FRAME_BYTES,
        )?;
        let decoded = decode_history_frame(&bytes, registry)?;
        if decoded.header().fields.namespace != namespace
            || decoded.header().frame_digest.as_slice() != digest.as_slice()
            || decoded.records().len() != usize::try_from(from_i64(record_count)?)?
        {
            return Err(index_mismatch());
        }
        let frame = from_i64(frame)?;
        validate_indexed_frame(connection, generation, frame, decoded.records())?;
        expected.0 = expected
            .0
            .checked_add(u64::try_from(
                decoded
                    .records()
                    .iter()
                    .filter(|record| matches!(record, HistoryRecord::Causal(_)))
                    .count(),
            )?)
            .ok_or(PersistenceError::HistoryArithmeticOverflow)?;
        expected.1 = expected
            .1
            .checked_add(decoded.records().iter().try_fold(0_u64, |total, record| {
                match record {
                    HistoryRecord::Causal(event) => total
                        .checked_add(u64::try_from(event.causes().len())?)
                        .ok_or(PersistenceError::HistoryArithmeticOverflow),
                    HistoryRecord::Evidence(_) => Ok(total),
                }
            })?)
            .ok_or(PersistenceError::HistoryArithmeticOverflow)?;
        expected.2 = expected
            .2
            .checked_add(decoded.records().iter().try_fold(0_u64, |total, record| {
                match record {
                    HistoryRecord::Causal(event) => total
                        .checked_add(u64::try_from(event.effects().len())?)
                        .ok_or(PersistenceError::HistoryArithmeticOverflow),
                    HistoryRecord::Evidence(_) => Ok(total),
                }
            })?)
            .ok_or(PersistenceError::HistoryArithmeticOverflow)?;
        expected.3 = expected
            .3
            .checked_add(u64::try_from(
                decoded
                    .records()
                    .iter()
                    .filter(|record| matches!(record, HistoryRecord::Evidence(_)))
                    .count(),
            )?)
            .ok_or(PersistenceError::HistoryArithmeticOverflow)?;
    }
    let actual = (
        catalog_count(connection, "events", generation)?,
        catalog_count(connection, "cause_edges", generation)?,
        catalog_count(connection, "effect_edges", generation)?,
        catalog_count(connection, "evidence", generation)?,
    );
    if actual != expected {
        return Err(index_mismatch());
    }
    Ok(())
}

fn record_at(
    connection: &Connection,
    source: &ReaderSource,
    frame_ordinal: u64,
    record_ordinal: u64,
) -> Result<Option<HistoryRecord>, PersistenceError> {
    let Some(decoded) = indexed_frame(connection, source, frame_ordinal)? else {
        return Ok(None);
    };
    Ok(decoded
        .records()
        .get(usize::try_from(record_ordinal)?)
        .cloned())
}

fn indexed_frame(
    connection: &Connection,
    source: &ReaderSource,
    frame_ordinal: u64,
) -> Result<Option<Arc<DecodedHistoryFrame>>, PersistenceError> {
    if let Some(frame) = source
        .frame_cache
        .lock()
        .map_err(|_| PersistenceError::HistoryCatalogInvariant {
            invariant: "history frame cache lock",
        })?
        .get(&frame_ordinal)
        .cloned()
    {
        return Ok(Some(frame));
    }
    let row = connection
        .query_row(
            "SELECT f.segment_sequence,f.start_offset,f.end_offset,f.frame_digest,
                    s.relative_path,s.sealed,s.identity
             FROM frames f JOIN segments s ON s.sequence=f.segment_sequence
             WHERE f.frame_ordinal=?1 AND f.generation<=?2",
            params![to_i64(frame_ordinal)?, to_i64(source.cold_generation)?],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, Vec<u8>>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, i64>(5)?,
                    row.get::<_, Option<Vec<u8>>>(6)?,
                ))
            },
        )
        .optional()
        .map_err(|error| catalog_error("read indexed frame", error))?;
    let Some((segment, start, end, digest, relative, sealed, identity)) = row else {
        return Ok(None);
    };
    let identity = identity
        .map(|bytes| {
            bytes
                .try_into()
                .map_err(|_| PersistenceError::HistoryCatalogInvariant {
                    invariant: "segment identity width",
                })
        })
        .transpose()?;
    let segment = from_i64(segment)?;
    let path = resolve_segment_path(
        &source.root,
        segment,
        sealed != 0,
        identity,
        Path::new(&relative),
        true,
    )?;
    let bytes = read_exact_range(
        &path,
        from_i64(start)?..from_i64(end)?,
        MAX_STORED_FRAME_BYTES,
    )?;
    let decoded = Arc::new(decode_history_frame(&bytes, source.registry.as_ref())?);
    if decoded.header().fields.namespace != source.namespace
        || decoded.header().frame_digest.as_slice() != digest.as_slice()
    {
        return Err(index_mismatch());
    }
    let mut cache =
        source
            .frame_cache
            .lock()
            .map_err(|_| PersistenceError::HistoryCatalogInvariant {
                invariant: "history frame cache lock",
            })?;
    cache.insert(frame_ordinal, Arc::clone(&decoded));
    while cache.len() > MAX_CACHED_FRAMES {
        let Some(oldest) = cache.keys().next().copied() else {
            break;
        };
        cache.remove(&oldest);
    }
    Ok(Some(decoded))
}

pub(crate) fn validate_indexed_frame(
    connection: &Connection,
    generation: u64,
    frame: u64,
    records: &[HistoryRecord],
) -> Result<(), PersistenceError> {
    for (ordinal, record) in records.iter().enumerate() {
        let ordinal = to_i64(u64::try_from(ordinal)?)?;
        match record {
            HistoryRecord::Causal(event) => {
                let indexed = connection
                    .query_row(
                        "SELECT trace_id,event_id,time FROM events
                         WHERE frame_ordinal=?1 AND record_ordinal=?2 AND generation<=?3 AND trace_id=?4",
                        params![
                            to_i64(frame)?,
                            ordinal,
                            to_i64(generation)?,
                            key(event.trace_id().raw())
                        ],
                        |row| {
                            Ok((
                                row.get::<_, Vec<u8>>(0)?,
                                row.get::<_, Vec<u8>>(1)?,
                                row.get::<_, Vec<u8>>(2)?,
                            ))
                        },
                    )
                    .optional()
                    .map_err(|error| catalog_error("validate event index", error))?;
                let Some((trace, event_id, time)) = indexed else {
                    return Err(index_mismatch());
                };
                if key(event.trace_id().raw()).as_slice() != trace.as_slice()
                    || key(event.event_id().raw()).as_slice() != event_id.as_slice()
                    || key(event.time().raw()).as_slice() != time.as_slice()
                {
                    return Err(index_mismatch());
                }
                let cause_count: i64 = connection
                    .query_row(
                        "SELECT COUNT(*) FROM cause_edges WHERE child_trace=?1 AND generation<=?2",
                        params![key(event.trace_id().raw()), to_i64(generation)?],
                        |row| row.get(0),
                    )
                    .map_err(|error| catalog_error("validate cause index count", error))?;
                if u64::try_from(cause_count)? != u64::try_from(event.causes().len())? {
                    return Err(index_mismatch());
                }
                let mut causes = connection
                    .prepare(
                        "SELECT parent_trace FROM cause_edges WHERE child_trace=?1 AND generation<=?2
                         ORDER BY ordinal",
                    )
                    .map_err(|error| catalog_error("prepare cause index values", error))?;
                let cause_rows = causes
                    .query_map(
                        params![key(event.trace_id().raw()), to_i64(generation)?],
                        |row| row.get::<_, Vec<u8>>(0),
                    )
                    .map_err(|error| catalog_error("query cause index values", error))?;
                for (expected, row) in event.causes().iter().zip(cause_rows) {
                    let actual = key_from_bytes(
                        row.map_err(|error| catalog_error("read cause index value", error))?,
                    )?;
                    if *expected != TraceId::new(actual) {
                        return Err(index_mismatch());
                    }
                }
                let effect_count: i64 = connection
                    .query_row(
                        "SELECT COUNT(*) FROM effect_edges WHERE trace_id=?1 AND generation<=?2",
                        params![key(event.trace_id().raw()), to_i64(generation)?],
                        |row| row.get(0),
                    )
                    .map_err(|error| catalog_error("validate effect index count", error))?;
                if u64::try_from(effect_count)? != u64::try_from(event.effects().len())? {
                    return Err(index_mismatch());
                }
                let mut effects = connection
                    .prepare(
                        "SELECT ordinal,object_kind,object_id,property_id FROM effect_edges
                         WHERE trace_id=?1 AND generation<=?2 ORDER BY ordinal",
                    )
                    .map_err(|error| catalog_error("prepare effect index values", error))?;
                let effect_rows = effects
                    .query_map(
                        params![key(event.trace_id().raw()), to_i64(generation)?],
                        |row| {
                            Ok((
                                row.get::<_, i64>(0)?,
                                row.get::<_, Vec<u8>>(1)?,
                                row.get::<_, Vec<u8>>(2)?,
                                row.get::<_, Vec<u8>>(3)?,
                            ))
                        },
                    )
                    .map_err(|error| catalog_error("query effect index values", error))?;
                for (expected_ordinal, (expected, row)) in
                    event.effects().iter().zip(effect_rows).enumerate()
                {
                    let (ordinal, object_kind, object_id, property_id) =
                        row.map_err(|error| catalog_error("read effect index value", error))?;
                    let expected_target = expected.target();
                    if u64::try_from(ordinal)? != u64::try_from(expected_ordinal)? {
                        return Err(index_mismatch());
                    }
                    if key_from_bytes(object_kind)? != expected_target.object_kind().raw()
                        || key_from_bytes(object_id)? != expected_target.object_id()
                        || key_from_bytes(property_id)? != expected_target.property().raw()
                    {
                        return Err(index_mismatch());
                    }
                }
            }
            HistoryRecord::Evidence(evidence) => {
                let indexed = connection
                    .query_row(
                        "SELECT schema_id,canonical_key,payload_digest FROM evidence
                         WHERE frame_ordinal=?1 AND record_ordinal=?2 AND generation<=?3",
                        params![to_i64(frame)?, ordinal, to_i64(generation)?],
                        |row| {
                            Ok((
                                row.get::<_, Vec<u8>>(0)?,
                                row.get::<_, Vec<u8>>(1)?,
                                row.get::<_, Vec<u8>>(2)?,
                            ))
                        },
                    )
                    .optional()
                    .map_err(|error| catalog_error("validate evidence index", error))?;
                let Some((schema, key_bytes, digest)) = indexed else {
                    return Err(index_mismatch());
                };
                if key(evidence.schema_id().raw()).as_slice() != schema.as_slice()
                    || evidence.canonical_key() != key_bytes
                    || evidence.payload_digest().bytes().as_slice() != digest.as_slice()
                {
                    return Err(index_mismatch());
                }
            }
        }
    }
    Ok(())
}

fn key_from_bytes(bytes: Vec<u8>) -> Result<u64, PersistenceError> {
    let key: [u8; 8] = bytes.try_into().map_err(|_| index_mismatch())?;
    Ok(u64::from_be_bytes(key))
}

fn to_i64(value: u64) -> Result<i64, PersistenceError> {
    i64::try_from(value).map_err(|_| PersistenceError::HistoryArithmeticOverflow)
}

fn catalog_count(
    connection: &Connection,
    table: &'static str,
    generation: u64,
) -> Result<u64, PersistenceError> {
    let sql = format!("SELECT COUNT(*) FROM {table} WHERE generation<=?1");
    let value: i64 = connection
        .query_row(&sql, [to_i64(generation)?], |row| row.get(0))
        .map_err(|error| catalog_error("count catalog index rows", error))?;
    u64::try_from(value).map_err(|_| index_mismatch())
}

fn catalog_error(operation: &'static str, error: rusqlite::Error) -> PersistenceError {
    PersistenceError::HistoryCatalog {
        operation,
        code: match &error {
            rusqlite::Error::SqliteFailure(failure, _) => Some(failure.extended_code),
            _ => None,
        },
        message: error.to_string(),
    }
}

fn index_mismatch() -> PersistenceError {
    PersistenceError::HistoryCatalogInvariant {
        invariant: "derived catalog index rows do not match canonical frames",
    }
}

fn open_reader_connection(root: &Path) -> Result<Connection, PersistenceError> {
    Connection::open_with_flags(
        root.join("catalog.sqlite"),
        OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .map_err(|error| catalog_error("open catalog reader", error))
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        sync::atomic::{AtomicU64, Ordering},
    };

    use causafera_core::{
        CausalEffect, CausalTarget, HistoryReader, HistoryRecord, OwnedCausalRecord, Phase,
        StateFingerprint,
    };
    use causafera_types::{
        EventId, EventKindId, SimulationTime, StateObjectKindId, StatePropertyId, TraceId,
    };

    use super::*;
    use crate::{DurabilityMode, HistoryStore};

    static NEXT_TEST_ROOT: AtomicU64 = AtomicU64::new(0);

    fn causal(sequence: u64, causes: Vec<TraceId>) -> HistoryRecord {
        let effect = CausalEffect::new(
            CausalTarget::new(StateObjectKindId::new(7), sequence, StatePropertyId::new(9)),
            StateFingerprint::new([1; 32]),
            StateFingerprint::new([2; 32]),
        )
        .expect("test effect changes state");
        HistoryRecord::Causal(
            OwnedCausalRecord::new(
                sequence,
                EventId::new(sequence),
                TraceId::new(sequence),
                SimulationTime::new(sequence * 10),
                Phase::Physics,
                EventKindId::new(13),
                causes,
                vec![effect],
            )
            .expect("test causal record is valid"),
        )
    }

    #[test]
    fn causal_overlay_validation_uses_one_batched_event_index_lookup() {
        let root = std::env::temp_dir().join(format!(
            "causafera-history-reader-batch-{}-{}",
            std::process::id(),
            NEXT_TEST_ROOT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).expect("create test bundle root");
        let scenario = || {
            let mut store = HistoryStore::create(&root, [0xA7; 32], DurabilityMode::EveryTick)
                .expect("create test store");
            let first = causal(1, Vec::new());
            let second = causal(2, vec![TraceId::new(1)]);
            store.append_tick(vec![first.clone()]).expect("first frame");
            store
                .append_tick(vec![second.clone()])
                .expect("second frame");
            let reader = store.durable_reader().expect("indexed reader");
            let expected = [first, second]
                .into_iter()
                .map(|record| match record {
                    HistoryRecord::Causal(event) => event,
                    HistoryRecord::Evidence(_) => unreachable!("causal test record"),
                })
                .collect::<Vec<_>>();

            CAUSAL_OVERLAY_EVENT_INDEX_BATCHES.store(0, Ordering::SeqCst);
            reader
                .validate_causal_overlay(&expected)
                .expect("exact overlay validates");

            let batches = CAUSAL_OVERLAY_EVENT_INDEX_BATCHES.load(Ordering::SeqCst);
            println!(
                "indexed_overlay catalog_event_index_batches={batches} records={}",
                expected.len()
            );
            assert_eq!(
                batches, 1,
                "overlay validation must batch event index lookup"
            );
            Ok::<(), String>(())
        };
        let result = scenario();
        fs::remove_dir_all(&root).expect("remove test bundle root");
        result.expect("batched overlay validation test");
    }
}
