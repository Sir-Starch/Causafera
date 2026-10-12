use std::collections::BTreeSet;

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
    pub const fn page(offset: u64) -> Result<Self, HistoryError> {
        Self::new(offset, MAX_QUERY_PAGE_RECORDS, MAX_QUERY_PAGE_BYTES)
    }

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HistoryQueryStatus {
    Complete,
    Partial,
    Unknown,
    Missing,
    IntegrityError,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HistoryQueryOutcome<T> {
    Complete(T),
    Partial(T),
    Unknown,
    Missing,
    IntegrityError(HistoryError),
}

impl<T> HistoryQueryOutcome<T> {
    pub const fn status(&self) -> HistoryQueryStatus {
        match self {
            Self::Complete(_) => HistoryQueryStatus::Complete,
            Self::Partial(_) => HistoryQueryStatus::Partial,
            Self::Unknown => HistoryQueryStatus::Unknown,
            Self::Missing => HistoryQueryStatus::Missing,
            Self::IntegrityError(_) => HistoryQueryStatus::IntegrityError,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HistoryWalkBudget {
    max_depth: usize,
    max_nodes: usize,
}

impl HistoryWalkBudget {
    pub const MAX_DEPTH: usize = 10_000;
    pub const MAX_NODES: usize = 10_000;

    pub const fn new(max_depth: usize, max_nodes: usize) -> Result<Self, HistoryError> {
        if max_depth > Self::MAX_DEPTH {
            return Err(HistoryError::QueryDepthLimitExceeded {
                requested: max_depth,
                max: Self::MAX_DEPTH,
            });
        }
        if max_nodes > Self::MAX_NODES {
            return Err(HistoryError::QueryNodeLimitExceeded {
                requested: max_nodes,
                max: Self::MAX_NODES,
            });
        }
        Ok(Self {
            max_depth,
            max_nodes,
        })
    }

    pub const fn max_depth(self) -> usize {
        self.max_depth
    }
    pub const fn max_nodes(self) -> usize {
        self.max_nodes
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HistoryAncestorCursor {
    anchor: HistoryAnchor,
    root: TraceId,
    frontier: Vec<(TraceId, usize, bool)>,
    queued: BTreeSet<TraceId>,
    visited: BTreeSet<TraceId>,
    emitted: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HistoryAncestorPage {
    page: HistoryPage<TraceId>,
    next_cursor: Option<HistoryAncestorCursor>,
    status: HistoryQueryStatus,
}

impl HistoryAncestorPage {
    pub fn page(&self) -> &HistoryPage<TraceId> {
        &self.page
    }
    pub fn into_page(self) -> HistoryPage<TraceId> {
        self.page
    }
    pub fn next_cursor(&self) -> Option<&HistoryAncestorCursor> {
        self.next_cursor.as_ref()
    }
    pub fn into_next_cursor(self) -> Option<HistoryAncestorCursor> {
        self.next_cursor
    }
    pub const fn status(&self) -> HistoryQueryStatus {
        self.status
    }
}

impl<T: HistoryPageItem> HistoryPage<T> {
    pub fn new(
        records: Vec<T>,
        next_offset: Option<u64>,
        query: HistoryQuery,
    ) -> Result<Self, HistoryError> {
        if let Some(next_offset) = next_offset
            && next_offset <= query.offset
        {
            return Err(HistoryError::QueryPageDoesNotAdvance {
                offset: query.offset,
                next_offset,
            });
        }
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
    pub const fn status(&self) -> HistoryQueryStatus {
        if self.next_offset.is_some() {
            HistoryQueryStatus::Partial
        } else {
            HistoryQueryStatus::Complete
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HistoryCheckpoint {
    pub time: SimulationTime,
    pub frontier: HistoryFrontier,
    pub digest: [u8; 32],
}

pub trait HistoryReader: std::fmt::Debug + Send + Sync {
    /// Capture a reader whose logical generation and bounded overlay cannot move.
    fn pin_snapshot(&self) -> Result<std::sync::Arc<dyn HistoryReader>, HistoryError> {
        Err(HistoryError::ReaderOperationUnavailable)
    }
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
    fn child_count(&self, trace_id: TraceId) -> Result<u64, HistoryError> {
        let mut offset = 0_u64;
        loop {
            let page = self.children(trace_id, HistoryQuery::page(offset)?)?;
            offset = offset
                .checked_add(
                    u64::try_from(page.records().len())
                        .map_err(|_| HistoryError::ArithmeticOverflow)?,
                )
                .ok_or(HistoryError::ArithmeticOverflow)?;
            if page.next_offset().is_none() {
                return Ok(offset);
            }
        }
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
    fn evidence_count(
        &self,
        schema_id: EvidenceSchemaId,
        canonical_key: &[u8],
    ) -> Result<u64, HistoryError> {
        let mut offset = 0_u64;
        loop {
            let page = self.evidence(schema_id, canonical_key, HistoryQuery::page(offset)?)?;
            offset = offset
                .checked_add(
                    u64::try_from(page.records().len())
                        .map_err(|_| HistoryError::ArithmeticOverflow)?,
                )
                .ok_or(HistoryError::ArithmeticOverflow)?;
            if page.next_offset().is_none() {
                return Ok(offset);
            }
        }
    }
    fn evidence_by_anchor(
        &self,
        _schema_id: EvidenceSchemaId,
        _anchor: TraceId,
        _query: HistoryQuery,
    ) -> Result<HistoryPage<DomainEvidenceRecord>, HistoryError> {
        Err(HistoryError::ReaderOperationUnavailable)
    }
    fn evidence_by_anchor_count(
        &self,
        schema_id: EvidenceSchemaId,
        anchor: TraceId,
    ) -> Result<u64, HistoryError> {
        let mut offset = 0_u64;
        loop {
            let page = self.evidence_by_anchor(schema_id, anchor, HistoryQuery::page(offset)?)?;
            offset = offset
                .checked_add(
                    u64::try_from(page.records().len())
                        .map_err(|_| HistoryError::ArithmeticOverflow)?,
                )
                .ok_or(HistoryError::ArithmeticOverflow)?;
            if page.next_offset().is_none() {
                return Ok(offset);
            }
        }
    }
    fn evidence_by_key_prefix(
        &self,
        _schema_id: EvidenceSchemaId,
        _canonical_key_prefix: &[u8],
        _query: HistoryQuery,
    ) -> Result<HistoryPage<DomainEvidenceRecord>, HistoryError> {
        Err(HistoryError::ReaderOperationUnavailable)
    }
    fn evidence_by_key_prefix_count(
        &self,
        schema_id: EvidenceSchemaId,
        canonical_key_prefix: &[u8],
    ) -> Result<u64, HistoryError> {
        let mut offset = 0_u64;
        loop {
            let page = self.evidence_by_key_prefix(
                schema_id,
                canonical_key_prefix,
                HistoryQuery::page(offset)?,
            )?;
            offset = offset
                .checked_add(
                    u64::try_from(page.records().len())
                        .map_err(|_| HistoryError::ArithmeticOverflow)?,
                )
                .ok_or(HistoryError::ArithmeticOverflow)?;
            if page.next_offset().is_none() {
                return Ok(offset);
            }
        }
    }
    fn walk_ancestors(
        &self,
        _trace_id: TraceId,
        _max_depth: usize,
        _query: HistoryQuery,
    ) -> Result<HistoryPage<TraceId>, HistoryError> {
        Err(HistoryError::ReaderOperationUnavailable)
    }
    fn walk_ancestors_bounded(
        &self,
        trace_id: TraceId,
        budget: HistoryWalkBudget,
        query: HistoryQuery,
    ) -> Result<HistoryQueryOutcome<HistoryPage<TraceId>>, HistoryError> {
        let Some(_) = self.event(trace_id)? else {
            return Ok(HistoryQueryOutcome::Missing);
        };
        let mut frontier = vec![(trace_id, 0_usize)];
        let mut visited = BTreeSet::new();
        let mut exhausted = false;
        while let Some((current, depth)) = frontier.pop() {
            let mut parents = Vec::new();
            let mut offset = 0_u64;
            loop {
                let parent_page = match self.parents(
                    current,
                    HistoryQuery::new(offset, MAX_QUERY_PAGE_RECORDS, MAX_QUERY_PAGE_BYTES)?,
                ) {
                    Ok(page) => page,
                    Err(HistoryError::ReaderOperationUnavailable) => {
                        return Ok(HistoryQueryOutcome::Unknown);
                    }
                    Err(error) => return Ok(HistoryQueryOutcome::IntegrityError(error)),
                };
                parents.extend_from_slice(parent_page.records());
                let Some(next_offset) = parent_page.next_offset() else {
                    break;
                };
                offset = next_offset;
            }
            if depth >= budget.max_depth() {
                exhausted |= !parents.is_empty();
                continue;
            }
            for parent in parents.iter().rev() {
                if visited.contains(parent) {
                    continue;
                }
                if visited.len() == budget.max_nodes() {
                    exhausted = true;
                    break;
                }
                visited.insert(*parent);
                frontier.push((*parent, depth + 1));
            }
            if visited.len() == budget.max_nodes() && !frontier.is_empty() {
                exhausted = true;
                break;
            }
        }
        let total = visited.len();
        let values = visited.into_iter().collect::<Vec<_>>();
        let start =
            usize::try_from(query.offset()).map_err(|_| HistoryError::ArithmeticOverflow)?;
        let mut bytes = 0_usize;
        let mut page_values = Vec::new();
        for value in values.iter().skip(start) {
            if page_values.len() == query.max_records() {
                break;
            }
            let next = bytes
                .checked_add(value.decoded_size())
                .ok_or(HistoryError::ArithmeticOverflow)?;
            if next > query.max_bytes() {
                break;
            }
            bytes = next;
            page_values.push(*value);
        }
        let consumed = query
            .offset()
            .checked_add(page_values.len() as u64)
            .ok_or(HistoryError::ArithmeticOverflow)?;
        let page_has_more =
            usize::try_from(consumed).map_err(|_| HistoryError::ArithmeticOverflow)? < total;
        let page = HistoryPage::new(page_values, page_has_more.then_some(consumed), query)?;
        Ok(if exhausted || page_has_more {
            HistoryQueryOutcome::Partial(page)
        } else {
            HistoryQueryOutcome::Complete(page)
        })
    }
    fn checkpoint_at(
        &self,
        _time: SimulationTime,
    ) -> Result<Option<HistoryCheckpoint>, HistoryError> {
        Err(HistoryError::ReaderOperationUnavailable)
    }

    fn ancestor_cursor(
        &self,
        trace_id: TraceId,
    ) -> Result<Option<HistoryAncestorCursor>, HistoryError> {
        let anchor = self.anchor()?;
        if self.event(trace_id)?.is_none() {
            return Ok(None);
        }
        Ok(Some(HistoryAncestorCursor {
            anchor,
            root: trace_id,
            frontier: vec![(trace_id, 0, false)],
            queued: BTreeSet::from([trace_id]),
            visited: BTreeSet::from([trace_id]),
            emitted: 0,
        }))
    }

    fn walk_ancestor_page(
        &self,
        mut cursor: HistoryAncestorCursor,
        budget: HistoryWalkBudget,
        query: HistoryQuery,
    ) -> Result<HistoryAncestorPage, HistoryError> {
        if query.offset() != 0 {
            return Err(HistoryError::QueryCursorOffsetMustBeZero);
        }
        if query.max_records() == 0 {
            return Err(HistoryError::QueryRecordLimitExceeded {
                requested: 1,
                max: 0,
            });
        }
        if query.max_bytes() < 8 {
            return Err(HistoryError::QueryByteLimitExceeded {
                requested: 8,
                max: query.max_bytes(),
            });
        }
        if self.anchor()? != cursor.anchor {
            return Err(HistoryError::StaleQueryCursor);
        }
        let mut records = Vec::new();
        let mut bytes = 0_usize;
        let mut expanded = 0_usize;
        while let Some((current, depth, emitted)) = cursor.frontier.pop() {
            cursor.queued.remove(&current);
            if current != cursor.root && !emitted {
                let next_bytes = bytes
                    .checked_add(current.decoded_size())
                    .ok_or(HistoryError::ArithmeticOverflow)?;
                if records.len() == query.max_records() || next_bytes > query.max_bytes() {
                    cursor.frontier.push((current, depth, false));
                    cursor.queued.insert(current);
                    break;
                }
                bytes = next_bytes;
                records.push(current);
                cursor.emitted = cursor
                    .emitted
                    .checked_add(1)
                    .ok_or(HistoryError::ArithmeticOverflow)?;
            }
            if expanded == budget.max_nodes() {
                cursor
                    .frontier
                    .push((current, depth, current != cursor.root));
                cursor.queued.insert(current);
                break;
            }
            expanded += 1;
            let mut parent_offset = 0_u64;
            let mut depth_limited = false;
            loop {
                let parents = self.parents(current, HistoryQuery::page(parent_offset)?)?;
                if depth >= budget.max_depth() {
                    if !parents.records().is_empty() || parents.next_offset().is_some() {
                        cursor
                            .frontier
                            .push((current, depth, current != cursor.root));
                        cursor.queued.insert(current);
                        depth_limited = true;
                    }
                    break;
                }
                for parent in parents.records() {
                    if cursor.visited.contains(parent) {
                        continue;
                    }
                    if cursor.visited.len() == budget.max_nodes() {
                        cursor
                            .frontier
                            .push((current, depth, current != cursor.root));
                        cursor.queued.insert(current);
                        depth_limited = true;
                        break;
                    }
                    cursor.visited.insert(*parent);
                    cursor.queued.insert(*parent);
                    cursor.frontier.push((*parent, depth + 1, false));
                }
                if depth_limited || parents.next_offset().is_none() {
                    break;
                }
                parent_offset = parents.next_offset().expect("checked page continuation");
            }
            if depth_limited || records.len() == query.max_records() {
                break;
            }
        }
        let has_more = !cursor.frontier.is_empty();
        let next_offset = has_more.then_some(cursor.emitted);
        let page = HistoryPage::new(records, next_offset, query)?;
        Ok(HistoryAncestorPage {
            page,
            next_cursor: has_more.then_some(cursor),
            status: if has_more {
                HistoryQueryStatus::Partial
            } else {
                HistoryQueryStatus::Complete
            },
        })
    }
}

pub trait TransactionHistoryReader: HistoryReader {
    fn staged_event(&self, trace_id: TraceId) -> Result<Option<OwnedCausalRecord>, HistoryError>;
}

pub use HistoryReader as CausalHistoryReader;
pub use TransactionHistoryReader as CausalHistoryTxnReader;
