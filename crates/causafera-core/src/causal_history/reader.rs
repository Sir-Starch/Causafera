use causafera_types::{EventId, SimulationTime, TraceId};

use crate::CausalTarget;

use super::{
    DomainEvidenceRecord, EvidenceSchemaId, HistoryAnchor, HistoryError, HistoryFrontier,
    HistoryRecord, MAX_QUERY_PAGE_BYTES, MAX_QUERY_PAGE_RECORDS, OwnedCausalRecord,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HistoryQuery {
    offset: u64,
    max_records: usize,
    max_bytes: usize,
}

impl HistoryQuery {
    pub const fn new(
        offset: u64,
        max_records: usize,
        max_bytes: usize,
    ) -> Result<Self, HistoryError> {
        if max_records > MAX_QUERY_PAGE_RECORDS {
            return Err(HistoryError::QueryRecordLimitExceeded {
                requested: max_records,
                max: MAX_QUERY_PAGE_RECORDS,
            });
        }
        if max_bytes > MAX_QUERY_PAGE_BYTES {
            return Err(HistoryError::QueryByteLimitExceeded {
                requested: max_bytes,
                max: MAX_QUERY_PAGE_BYTES,
            });
        }
        if offset.checked_add(max_records as u64).is_none() {
            return Err(HistoryError::ArithmeticOverflow);
        }
        Ok(Self {
            offset,
            max_records,
            max_bytes,
        })
    }
    pub const fn offset(self) -> u64 {
        self.offset
    }
    pub const fn max_records(self) -> usize {
        self.max_records
    }
    pub const fn max_bytes(self) -> usize {
        self.max_bytes
    }
}

pub trait HistoryPageItem {
    fn decoded_size(&self) -> usize;
}

impl HistoryPageItem for TraceId {
    fn decoded_size(&self) -> usize {
        8
    }
}
impl HistoryPageItem for OwnedCausalRecord {
    fn decoded_size(&self) -> usize {
        self.canonical_field_bytes()
            .map_or(usize::MAX, |bytes| bytes.len())
    }
}
impl HistoryPageItem for DomainEvidenceRecord {
    fn decoded_size(&self) -> usize {
        8 * 3
            + 4
            + 1
            + 8
            + self.causal_anchors().len() * 8
            + self.canonical_key().len()
            + self.canonical_payload().len()
            + 32
    }
}
impl HistoryPageItem for HistoryRecord {
    fn decoded_size(&self) -> usize {
        match self {
            HistoryRecord::Causal(record) => record.decoded_size(),
            HistoryRecord::Evidence(record) => record.decoded_size(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HistoryPage<T> {
    records: Vec<T>,
    next_offset: Option<u64>,
    decoded_bytes: usize,
}

impl<T: HistoryPageItem> HistoryPage<T> {
    pub fn new(
        records: Vec<T>,
        next_offset: Option<u64>,
        query: HistoryQuery,
    ) -> Result<Self, HistoryError> {
        if records.len() > query.max_records {
            return Err(HistoryError::QueryRecordLimitExceeded {
                requested: records.len(),
                max: query.max_records,
            });
        }
        let decoded_bytes = records.iter().try_fold(0_usize, |total, record| {
            total
                .checked_add(record.decoded_size())
                .ok_or(HistoryError::ArithmeticOverflow)
        })?;
        if decoded_bytes > query.max_bytes {
            return Err(HistoryError::QueryByteLimitExceeded {
                requested: decoded_bytes,
                max: query.max_bytes,
            });
        }
        Ok(Self {
            records,
            next_offset,
            decoded_bytes,
        })
    }
    pub fn records(&self) -> &[T] {
        &self.records
    }
    pub const fn next_offset(&self) -> Option<u64> {
        self.next_offset
    }
    pub const fn decoded_bytes(&self) -> usize {
        self.decoded_bytes
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HistoryCheckpoint {
    pub time: SimulationTime,
    pub frontier: HistoryFrontier,
    pub digest: [u8; 32],
}

pub trait HistoryReader: std::fmt::Debug + Send + Sync {
    fn anchor(&self) -> Result<HistoryAnchor, HistoryError>;
    fn event(&self, trace_id: TraceId) -> Result<Option<OwnedCausalRecord>, HistoryError>;
    fn validate_causal_overlay(&self, records: &[OwnedCausalRecord]) -> Result<(), HistoryError> {
        for record in records {
            if self.event(record.trace_id())?.as_ref() != Some(record) {
                return Err(HistoryError::ColdRecordMismatch {
                    trace_id: record.trace_id(),
                });
            }
        }
        Ok(())
    }
    fn event_by_id(&self, _event_id: EventId) -> Result<Option<OwnedCausalRecord>, HistoryError> {
        Err(HistoryError::ReaderOperationUnavailable)
    }
    fn records(&self, query: HistoryQuery) -> Result<HistoryPage<HistoryRecord>, HistoryError>;
    fn indexed_range(
        &self,
        query: HistoryQuery,
    ) -> Result<HistoryPage<HistoryRecord>, HistoryError> {
        self.records(query)
    }
    fn all_records_for_oracle(&self) -> Result<Vec<OwnedCausalRecord>, HistoryError> {
        let mut records = Vec::new();
        let mut offset = 0_u64;
        loop {
            let query = HistoryQuery::new(offset, MAX_QUERY_PAGE_RECORDS, MAX_QUERY_PAGE_BYTES)?;
            let page = self.indexed_range(query)?;
            records.extend(page.records().iter().filter_map(|record| match record {
                HistoryRecord::Causal(record) => Some(record.clone()),
                HistoryRecord::Evidence(_) => None,
            }));
            let Some(next) = page.next_offset() else {
                break;
            };
            offset = next;
        }
        Ok(records)
    }
    fn parents(
        &self,
        trace_id: TraceId,
        query: HistoryQuery,
    ) -> Result<HistoryPage<TraceId>, HistoryError> {
        let parents = self
            .event(trace_id)?
            .map_or_else(Vec::new, |event| event.causes().to_vec());
        HistoryPage::new(parents, None, query)
    }
    fn children(
        &self,
        _trace_id: TraceId,
        _query: HistoryQuery,
    ) -> Result<HistoryPage<TraceId>, HistoryError> {
        Err(HistoryError::ReaderOperationUnavailable)
    }
    fn latest_matching_event_before(
        &self,
        _target: CausalTarget,
        _trace_id: TraceId,
    ) -> Result<Option<OwnedCausalRecord>, HistoryError> {
        Err(HistoryError::ReaderOperationUnavailable)
    }
    fn evidence(
        &self,
        _schema_id: EvidenceSchemaId,
        _canonical_key: &[u8],
        _query: HistoryQuery,
    ) -> Result<HistoryPage<DomainEvidenceRecord>, HistoryError> {
        Err(HistoryError::ReaderOperationUnavailable)
    }
    fn walk_ancestors(
        &self,
        _trace_id: TraceId,
        _max_depth: usize,
        _query: HistoryQuery,
    ) -> Result<HistoryPage<TraceId>, HistoryError> {
        Err(HistoryError::ReaderOperationUnavailable)
    }
    fn checkpoint_at(
        &self,
        _time: SimulationTime,
    ) -> Result<Option<HistoryCheckpoint>, HistoryError> {
        Err(HistoryError::ReaderOperationUnavailable)
    }
}

pub trait TransactionHistoryReader: HistoryReader {
    fn staged_event(&self, trace_id: TraceId) -> Result<Option<OwnedCausalRecord>, HistoryError>;
}

pub use HistoryReader as CausalHistoryReader;
pub use TransactionHistoryReader as CausalHistoryTxnReader;
