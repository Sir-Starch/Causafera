use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
    sync::Arc,
};

use causafera_types::{
    EventId, EventKindId, SimulationTime, StateObjectKindId, StatePropertyId, TraceId,
};
use thiserror::Error;

use crate::{
    CausalHistoryReader, DEFAULT_CAUSAL_HOT_BUDGET_BYTES, HistoryError, HistoryPage,
    HistoryPageItem, HistoryQuery, HistoryRecord, MAX_QUERY_PAGE_BYTES, MAX_QUERY_PAGE_RECORDS,
    OwnedCausalRecord, Phase,
};

const HOT_CHILD_CURSOR_FLAG: u64 = 1 << 63;

/// Canonical identity of one property on one authoritative state object.
///
/// Kind and property IDs are opaque schema identities. Human-readable names
/// belong to observer metadata and are not authoritative simulation meaning.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CausalTarget {
    object_kind: StateObjectKindId,
    object_id: u64,
    property: StatePropertyId,
}

impl CausalTarget {
    pub const fn new(
        object_kind: StateObjectKindId,
        object_id: u64,
        property: StatePropertyId,
    ) -> Self {
        Self {
            object_kind,
            object_id,
            property,
        }
    }

    pub const fn object_kind(self) -> StateObjectKindId {
        self.object_kind
    }

    pub const fn object_id(self) -> u64 {
        self.object_id
    }

    pub const fn property(self) -> StatePropertyId {
        self.property
    }
}

/// Caller-supplied fingerprint of a canonical property representation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StateFingerprint([u8; 32]);

impl StateFingerprint {
    pub const fn new(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub const fn bytes(self) -> [u8; 32] {
        self.0
    }
}

/// One objective property transition committed by an event.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CausalEffect {
    target: CausalTarget,
    before: StateFingerprint,
    after: StateFingerprint,
}

impl CausalEffect {
    pub fn new(
        target: CausalTarget,
        before: StateFingerprint,
        after: StateFingerprint,
    ) -> Result<Self, CausalEffectError> {
        if before == after {
            return Err(CausalEffectError::UnchangedState { target });
        }
        Ok(Self {
            target,
            before,
            after,
        })
    }

    pub const fn target(self) -> CausalTarget {
        self.target
    }

    pub const fn before(self) -> StateFingerprint {
        self.before
    }

    pub const fn after(self) -> StateFingerprint {
        self.after
    }
}

#[derive(Clone, Copy, Debug, Error, PartialEq, Eq)]
pub enum CausalEffectError {
    #[error("causal effect for {target:?} does not change canonical state")]
    UnchangedState { target: CausalTarget },
}

/// Stable reduction key for a proposal within one scheduler phase.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EventProposalKey {
    system_id: u64,
    subject_ordinal: u64,
    operation_ordinal: u64,
}

impl EventProposalKey {
    pub const fn new(system_id: u64, subject_ordinal: u64, operation_ordinal: u64) -> Self {
        Self {
            system_id,
            subject_ordinal,
            operation_ordinal,
        }
    }

    pub const fn system_id(self) -> u64 {
        self.system_id
    }

    pub const fn subject_ordinal(self) -> u64 {
        self.subject_ordinal
    }

    pub const fn operation_ordinal(self) -> u64 {
        self.operation_ordinal
    }
}

/// Validated proposed event, ready for deterministic reduction and commit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CausalEventProposal {
    key: EventProposalKey,
    kind: EventKindId,
    causes: Vec<TraceId>,
    effects: Vec<CausalEffect>,
}

impl CausalEventProposal {
    pub fn new(
        key: EventProposalKey,
        kind: EventKindId,
        causes: Vec<TraceId>,
        effects: Vec<CausalEffect>,
    ) -> Result<Self, CausalEventProposalError> {
        validate_strict_order(&causes)
            .map_err(|index| CausalEventProposalError::CausesNotStrictlyOrdered { index })?;
        if effects.is_empty() {
            return Err(CausalEventProposalError::NoEffects);
        }
        for index in 1..effects.len() {
            if effects[index - 1].target() >= effects[index].target() {
                return Err(CausalEventProposalError::EffectsNotStrictlyOrdered { index });
            }
        }
        Ok(Self {
            key,
            kind,
            causes,
            effects,
        })
    }

    pub const fn key(&self) -> EventProposalKey {
        self.key
    }

    pub const fn kind(&self) -> EventKindId {
        self.kind
    }

    pub fn causes(&self) -> &[TraceId] {
        &self.causes
    }

    pub fn effects(&self) -> &[CausalEffect] {
        &self.effects
    }
}

#[derive(Clone, Copy, Debug, Error, PartialEq, Eq)]
pub enum CausalEventProposalError {
    #[error("causal event proposal must contain at least one effect")]
    NoEffects,
    #[error("causes are not strictly ordered at index {index}")]
    CausesNotStrictlyOrdered { index: usize },
    #[error("effects are not strictly ordered by target at index {index}")]
    EffectsNotStrictlyOrdered { index: usize },
    #[error("proposal-key carrier is {length} bytes, which does not fit its 16-bit length field")]
    CarrierKeyTooLong { length: usize },
    /// A persisted proposal key's bytes do not describe a key: wrong version, or a
    /// declared carrier length that disagrees with the payload that follows it.
    #[error("a hydrology proposal key's canonical bytes are malformed")]
    MalformedProposalKey,
}

/// Version byte every [`CausalEventProposalKey`] encoding starts with.
pub const CAUSAL_EVENT_PROPOSAL_KEY_VERSION: u8 = 0x01;

/// Canonical identity of one proposal inside an atomic DAG batch.
///
/// [`EventProposalKey`] identifies a proposal by three producer-side ordinals,
/// which is enough when causes are already-committed traces. A batch whose
/// events cause each other needs an identity a *sibling* can name before either
/// has a trace ID, and one whose byte order is the canonical tie-break for the
/// commit order. So this key is its own encoding:
///
/// ```text
/// 0x01 version
/// substage_ordinal: u8
/// process_kind:     u32 big-endian
/// carrier_length:   u16 big-endian
/// carrier_key_bytes
/// local_ordinal:    u32 big-endian
/// ```
///
/// Big-endian throughout, so the key compares as bytes in the same order it
/// compares as fields. The carrier bytes are opaque here: the domain that
/// builds them decides what they name, and this crate only orders them.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CausalEventProposalKey {
    bytes: Vec<u8>,
}

impl CausalEventProposalKey {
    const PREFIX_LEN: usize = 1 + 1 + 4 + 2;

    pub fn new(
        substage_ordinal: u8,
        process_kind: u32,
        carrier_key: &[u8],
        local_ordinal: u32,
    ) -> Result<Self, CausalEventProposalError> {
        let carrier_length = u16::try_from(carrier_key.len()).map_err(|_| {
            CausalEventProposalError::CarrierKeyTooLong {
                length: carrier_key.len(),
            }
        })?;
        let mut bytes = Vec::with_capacity(Self::PREFIX_LEN + carrier_key.len() + 4);
        bytes.push(CAUSAL_EVENT_PROPOSAL_KEY_VERSION);
        bytes.push(substage_ordinal);
        bytes.extend_from_slice(&process_kind.to_be_bytes());
        bytes.extend_from_slice(&carrier_length.to_be_bytes());
        bytes.extend_from_slice(carrier_key);
        bytes.extend_from_slice(&local_ordinal.to_be_bytes());
        Ok(Self { bytes })
    }

    /// Rebuild a key from its canonical bytes.
    ///
    /// Import needs this: a persisted receipt names the batch-local event that
    /// carried it, and the name has to survive a round trip. The prefix is
    /// re-validated rather than trusted, so a forged key with a wrong version or a
    /// carrier length that disagrees with its payload is rejected here.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, CausalEventProposalError> {
        if bytes.len() < Self::PREFIX_LEN + 4 {
            return Err(CausalEventProposalError::MalformedProposalKey);
        }
        if bytes[0] != CAUSAL_EVENT_PROPOSAL_KEY_VERSION {
            return Err(CausalEventProposalError::MalformedProposalKey);
        }
        let carrier_length = usize::from(u16::from_be_bytes([bytes[6], bytes[7]]));
        if bytes.len() != Self::PREFIX_LEN + carrier_length + 4 {
            return Err(CausalEventProposalError::MalformedProposalKey);
        }
        Ok(Self {
            bytes: bytes.to_vec(),
        })
    }

    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn substage_ordinal(&self) -> u8 {
        self.bytes[1]
    }

    pub fn process_kind(&self) -> u32 {
        u32::from_be_bytes(self.bytes[2..6].try_into().expect("four bytes"))
    }

    pub fn carrier_key(&self) -> &[u8] {
        let length = usize::from(u16::from_be_bytes(
            self.bytes[6..8].try_into().expect("two bytes"),
        ));
        &self.bytes[Self::PREFIX_LEN..Self::PREFIX_LEN + length]
    }

    pub fn local_ordinal(&self) -> u32 {
        let start = self.bytes.len() - 4;
        u32::from_be_bytes(self.bytes[start..].try_into().expect("four bytes"))
    }
}

/// One cause of a DAG proposal: an already-committed trace, or a sibling.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CausalEventDagCause {
    Existing(TraceId),
    Local(CausalEventProposalKey),
}

/// Per-event structural caps a DAG batch is validated against.
///
/// Supplied by the caller rather than fixed here: the limits are a domain's
/// contract about its own event shapes, and baking one domain's numbers into
/// the trace store would make them everyone's.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CausalDagBatchLimits {
    pub max_causes_per_event: usize,
    pub max_effects_per_event: usize,
}

/// A proposed event whose causes may include siblings in the same batch.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CausalEventDagProposal {
    key: CausalEventProposalKey,
    kind: EventKindId,
    causes: Vec<CausalEventDagCause>,
    effects: Vec<CausalEffect>,
}

impl CausalEventDagProposal {
    pub fn new(
        key: CausalEventProposalKey,
        kind: EventKindId,
        causes: Vec<CausalEventDagCause>,
        effects: Vec<CausalEffect>,
    ) -> Result<Self, CausalEventProposalError> {
        // Strictly ordered on the *proposal's* cause form, not on the resolved
        // trace IDs, which do not exist yet. This is what makes duplicate
        // causes impossible after resolution: two distinct locals get two
        // distinct fresh IDs, and an existing trace can never equal a fresh one.
        validate_strict_order(&causes)
            .map_err(|index| CausalEventProposalError::CausesNotStrictlyOrdered { index })?;
        if effects.is_empty() {
            return Err(CausalEventProposalError::NoEffects);
        }
        for index in 1..effects.len() {
            if effects[index - 1].target() >= effects[index].target() {
                return Err(CausalEventProposalError::EffectsNotStrictlyOrdered { index });
            }
        }
        Ok(Self {
            key,
            kind,
            causes,
            effects,
        })
    }

    pub fn key(&self) -> &CausalEventProposalKey {
        &self.key
    }

    pub const fn kind(&self) -> EventKindId {
        self.kind
    }

    pub fn causes(&self) -> &[CausalEventDagCause] {
        &self.causes
    }

    pub fn effects(&self) -> &[CausalEffect] {
        &self.effects
    }
}

/// One DAG event with its identifiers reserved and its causes resolved, held
/// aside until every check in the batch has passed.
struct PreparedDagEvent {
    event_id: EventId,
    trace_id: TraceId,
    kind: EventKindId,
    causes: Vec<TraceId>,
    effects: Vec<CausalEffect>,
}

#[derive(Clone, Copy)]
struct ChildIndexUndo {
    cause: TraceId,
    previous_len: usize,
}

#[derive(Clone)]
struct CausalTraceUndo {
    commit_sequences_len: usize,
    event_ids_len: usize,
    trace_ids_len: usize,
    times_len: usize,
    phases_len: usize,
    kinds_len: usize,
    cause_offsets_len: usize,
    causes_len: usize,
    effect_offsets_len: usize,
    effects_len: usize,
    next_event_id: u64,
    next_trace_id: u64,
    hot_bytes: usize,
    child_appends: Vec<ChildIndexUndo>,
    deferred_eviction: Option<usize>,
}

/// Borrowed view of one committed event in the structure-of-arrays store.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CausalEventRef<'a> {
    pub commit_sequence: u64,
    pub event_id: EventId,
    pub trace_id: TraceId,
    pub time: SimulationTime,
    pub phase: Phase,
    pub kind: EventKindId,
    pub causes: &'a [TraceId],
    pub effects: &'a [CausalEffect],
}

impl CausalEventRef<'_> {
    fn canonical_field_size(&self) -> Result<usize, HistoryError> {
        let effect_bytes = self
            .effects
            .len()
            .checked_mul(3 * 8 + 2 * 32)
            .ok_or(HistoryError::ArithmeticOverflow)?;
        6_usize
            .checked_mul(8)
            .and_then(|size| size.checked_add(self.causes.len().checked_mul(8)?))
            .and_then(|size| size.checked_add(8))
            .and_then(|size| size.checked_add(effect_bytes))
            .ok_or(HistoryError::ArithmeticOverflow)
    }
}

/// Canonical append-only event graph.
///
/// Hot event fields and forward edges are flat. Reverse edges are a cold,
/// deterministic side index used for direct descendant traversal.
pub struct CausalTraceStore {
    commit_sequences: Vec<u64>,
    event_ids: Vec<EventId>,
    trace_ids: Vec<TraceId>,
    times: Vec<SimulationTime>,
    phases: Vec<Phase>,
    kinds: Vec<EventKindId>,
    cause_offsets: Vec<u32>,
    causes: Vec<TraceId>,
    effect_offsets: Vec<u32>,
    effects: Vec<CausalEffect>,
    children: BTreeMap<TraceId, Vec<TraceId>>,
    next_event_id: u64,
    next_trace_id: u64,
    hot_bytes: usize,
    hot_budget: usize,
    cold_reader: Option<Arc<dyn CausalHistoryReader>>,
    transaction_undo: Option<CausalTraceUndo>,
}

impl fmt::Debug for CausalTraceStore {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CausalTraceStore")
            .field("commit_sequences", &self.commit_sequences)
            .field("event_ids", &self.event_ids)
            .field("trace_ids", &self.trace_ids)
            .field("times", &self.times)
            .field("phases", &self.phases)
            .field("kinds", &self.kinds)
            .field("cause_offsets", &self.cause_offsets)
            .field("causes", &self.causes)
            .field("effect_offsets", &self.effect_offsets)
            .field("effects", &self.effects)
            .field("children", &self.children)
            .field("next_event_id", &self.next_event_id)
            .field("next_trace_id", &self.next_trace_id)
            .field("hot_bytes", &self.hot_bytes)
            .field("hot_budget", &self.hot_budget)
            .field("has_cold_reader", &self.cold_reader.is_some())
            .field("transaction_active", &self.transaction_undo.is_some())
            .finish()
    }
}

impl PartialEq for CausalTraceStore {
    fn eq(&self, other: &Self) -> bool {
        self.commit_sequences == other.commit_sequences
            && self.event_ids == other.event_ids
            && self.trace_ids == other.trace_ids
            && self.times == other.times
            && self.phases == other.phases
            && self.kinds == other.kinds
            && self.cause_offsets == other.cause_offsets
            && self.causes == other.causes
            && self.effect_offsets == other.effect_offsets
            && self.effects == other.effects
            && self.children == other.children
            && self.next_event_id == other.next_event_id
            && self.next_trace_id == other.next_trace_id
            && self.hot_bytes == other.hot_bytes
            && self.hot_budget == other.hot_budget
            && self.transaction_undo.is_some() == other.transaction_undo.is_some()
            && match (&self.cold_reader, &other.cold_reader) {
                (Some(left), Some(right)) => Arc::ptr_eq(left, right),
                (None, None) => true,
                (Some(_), None) | (None, Some(_)) => false,
            }
    }
}

impl Eq for CausalTraceStore {}

impl Clone for CausalTraceStore {
    fn clone(&self) -> Self {
        self.clone_hot_cache()
    }
}

impl CausalTraceStore {
    pub fn new() -> Self {
        Self::with_hot_budget(DEFAULT_CAUSAL_HOT_BUDGET_BYTES)
    }

    pub fn with_hot_budget(hot_budget: usize) -> Self {
        Self {
            commit_sequences: Vec::new(),
            event_ids: Vec::new(),
            trace_ids: Vec::new(),
            times: Vec::new(),
            phases: Vec::new(),
            kinds: Vec::new(),
            cause_offsets: vec![0],
            causes: Vec::new(),
            effect_offsets: vec![0],
            effects: Vec::new(),
            children: BTreeMap::new(),
            next_event_id: 0,
            next_trace_id: 0,
            hot_bytes: 0,
            hot_budget,
            cold_reader: None,
            transaction_undo: None,
        }
    }

    pub fn with_cold_reader(
        cold_reader: Arc<dyn CausalHistoryReader>,
        hot_budget: usize,
        hot_records: Vec<OwnedCausalRecord>,
    ) -> Result<Self, HistoryError> {
        let frontier = cold_reader.anchor()?.frontier();
        let has_causal_head = cold_reader.event(frontier.trace_id)?.is_some();
        let next_event_id = if has_causal_head {
            frontier
                .event_id
                .raw()
                .checked_add(1)
                .ok_or(HistoryError::ArithmeticOverflow)?
        } else {
            0
        };
        let next_trace_id = if has_causal_head {
            frontier
                .trace_id
                .raw()
                .checked_add(1)
                .ok_or(HistoryError::ArithmeticOverflow)?
        } else {
            0
        };
        for (index, pair) in hot_records.windows(2).enumerate() {
            if pair[0].event_id() >= pair[1].event_id() || pair[0].trace_id() >= pair[1].trace_id()
            {
                return Err(HistoryError::NonCanonicalOrder { index: index + 1 });
            }
        }
        let mut store = Self::with_hot_budget(hot_budget);
        store.next_event_id = next_event_id;
        store.next_trace_id = next_trace_id;
        store.cold_reader = Some(cold_reader);
        store
            .cold_reader
            .as_ref()
            .ok_or(HistoryError::ReaderOperationUnavailable)?
            .validate_causal_overlay(&hot_records)?;
        for record in hot_records {
            store.push_hot_record(record)?;
        }
        let evict = store.evictable_prefix_proven()?;
        store.evict_hot_prefix(evict)?;
        Ok(store)
    }

    pub fn len(&self) -> usize {
        self.trace_ids.len()
    }

    /// Number of records held by the bounded hot facade.
    pub const fn hot_len(&self) -> usize {
        self.trace_ids.len()
    }

    /// Clone only the bounded hot facade and its cold-reader handle.
    ///
    /// The reader is shared by reference-counted ownership; cold records are
    /// never copied into this value. Callers that need a materialized hot-cache
    /// snapshot may use this explicitly; runtime tick rollback uses the bounded
    /// transaction marker instead.
    pub fn clone_hot_cache(&self) -> Self {
        Self {
            commit_sequences: self.commit_sequences.clone(),
            event_ids: self.event_ids.clone(),
            trace_ids: self.trace_ids.clone(),
            times: self.times.clone(),
            phases: self.phases.clone(),
            kinds: self.kinds.clone(),
            cause_offsets: self.cause_offsets.clone(),
            causes: self.causes.clone(),
            effect_offsets: self.effect_offsets.clone(),
            effects: self.effects.clone(),
            children: self.children.clone(),
            next_event_id: self.next_event_id,
            next_trace_id: self.next_trace_id,
            hot_bytes: self.hot_bytes,
            hot_budget: self.hot_budget,
            cold_reader: self.cold_reader.clone(),
            transaction_undo: self.transaction_undo.clone(),
        }
    }

    pub fn begin_transaction(&mut self) {
        if self.transaction_undo.is_some() {
            return;
        }
        self.transaction_undo = Some(CausalTraceUndo {
            commit_sequences_len: self.commit_sequences.len(),
            event_ids_len: self.event_ids.len(),
            trace_ids_len: self.trace_ids.len(),
            times_len: self.times.len(),
            phases_len: self.phases.len(),
            kinds_len: self.kinds.len(),
            cause_offsets_len: self.cause_offsets.len(),
            causes_len: self.causes.len(),
            effect_offsets_len: self.effect_offsets.len(),
            effects_len: self.effects.len(),
            next_event_id: self.next_event_id,
            next_trace_id: self.next_trace_id,
            hot_bytes: self.hot_bytes,
            child_appends: Vec::new(),
            deferred_eviction: None,
        });
    }

    pub fn end_transaction(&mut self) -> Result<(), HistoryError> {
        let Some(deferred_eviction) = self
            .transaction_undo
            .as_ref()
            .and_then(|undo| undo.deferred_eviction)
        else {
            if self.hot_bytes > self.hot_budget {
                let evict = self.evictable_prefix(0)?;
                self.evict_hot_prefix(evict)?;
            }
            self.transaction_undo = None;
            return Ok(());
        };
        self.evict_hot_prefix(deferred_eviction)?;
        self.transaction_undo = None;
        Ok(())
    }

    pub fn rollback_transaction(&mut self) {
        let Some(undo) = self.transaction_undo.take() else {
            return;
        };
        for child in undo.child_appends.into_iter().rev() {
            if let Some(children) = self.children.get_mut(&child.cause) {
                children.truncate(child.previous_len);
                if children.is_empty() {
                    self.children.remove(&child.cause);
                }
            }
        }
        self.commit_sequences.truncate(undo.commit_sequences_len);
        self.event_ids.truncate(undo.event_ids_len);
        self.trace_ids.truncate(undo.trace_ids_len);
        self.times.truncate(undo.times_len);
        self.phases.truncate(undo.phases_len);
        self.kinds.truncate(undo.kinds_len);
        self.cause_offsets.truncate(undo.cause_offsets_len);
        self.causes.truncate(undo.causes_len);
        self.effect_offsets.truncate(undo.effect_offsets_len);
        self.effects.truncate(undo.effects_len);
        self.next_event_id = undo.next_event_id;
        self.next_trace_id = undo.next_trace_id;
        self.hot_bytes = undo.hot_bytes;
    }

    pub fn is_empty(&self) -> bool {
        self.trace_ids.is_empty()
    }

    pub const fn hot_bytes(&self) -> usize {
        self.hot_bytes
    }

    pub const fn hot_budget(&self) -> usize {
        self.hot_budget
    }

    pub fn cold_reader(&self) -> Option<&dyn CausalHistoryReader> {
        self.cold_reader.as_deref()
    }

    /// Deterministically reduce and commit proposals for one phase.
    ///
    /// Proposal keys define order independently of producer scheduling. Causes
    /// must refer to traces committed before this batch, preventing cycles.
    pub fn commit_batch(
        &mut self,
        time: SimulationTime,
        phase: Phase,
        mut proposals: Vec<CausalEventProposal>,
    ) -> Result<Vec<TraceId>, CausalCommitError> {
        proposals.sort_by_key(CausalEventProposal::key);
        for index in 1..proposals.len() {
            if proposals[index - 1].key() == proposals[index].key() {
                return Err(CausalCommitError::DuplicateProposalKey {
                    key: proposals[index].key(),
                });
            }
        }
        for proposal in &proposals {
            for &cause in proposal.causes() {
                if !self
                    .trace_exists(cause)
                    .map_err(CausalCommitError::HistoryRead)?
                {
                    return Err(CausalCommitError::UnknownCause {
                        key: proposal.key(),
                        cause,
                    });
                }
            }
        }

        let count =
            u64::try_from(proposals.len()).map_err(|_| CausalCommitError::CapacityExceeded)?;
        self.next_event_id
            .checked_add(count)
            .ok_or(CausalCommitError::IdentifierExhausted)?;
        self.next_trace_id
            .checked_add(count)
            .ok_or(CausalCommitError::IdentifierExhausted)?;
        let added_causes = proposals.iter().map(|p| p.causes().len()).sum::<usize>();
        let added_effects = proposals.iter().map(|p| p.effects().len()).sum::<usize>();
        let final_causes = self
            .causes
            .len()
            .checked_add(added_causes)
            .ok_or(CausalCommitError::CapacityExceeded)?;
        let final_effects = self
            .effects
            .len()
            .checked_add(added_effects)
            .ok_or(CausalCommitError::CapacityExceeded)?;
        u32::try_from(final_causes).map_err(|_| CausalCommitError::CapacityExceeded)?;
        u32::try_from(final_effects).map_err(|_| CausalCommitError::CapacityExceeded)?;

        let mut prepared = Vec::with_capacity(proposals.len());
        for (offset, proposal) in proposals.into_iter().enumerate() {
            let offset = u64::try_from(offset).map_err(|_| CausalCommitError::CapacityExceeded)?;
            prepared.push(
                OwnedCausalRecord::new(
                    time.raw(),
                    EventId::new(self.next_event_id + offset),
                    TraceId::new(self.next_trace_id + offset),
                    time,
                    phase,
                    proposal.kind(),
                    proposal.causes().to_vec(),
                    proposal.effects().to_vec(),
                )
                .map_err(CausalCommitError::HistoryRead)?,
            );
        }
        let added_bytes = prepared.iter().try_fold(0_usize, |bytes, record| {
            bytes
                .checked_add(
                    record
                        .canonical_field_size()
                        .map_err(CausalCommitError::HistoryRead)?,
                )
                .ok_or(CausalCommitError::CapacityExceeded)
        })?;
        let evict = if self.transaction_undo.is_some() {
            0
        } else {
            self.evictable_prefix(added_bytes)
                .map_err(CausalCommitError::HistoryRead)?
        };
        let hot_budget = self.hot_budget;
        let mut candidate = std::mem::replace(self, Self::with_hot_budget(hot_budget));
        let result = (|| {
            candidate.evict_hot_prefix(evict)?;
            let mut committed = Vec::with_capacity(prepared.len());
            for record in prepared {
                committed.push(record.trace_id());
                candidate.push_hot_record(record)?;
            }
            candidate.next_event_id += count;
            candidate.next_trace_id += count;
            Ok(committed)
        })();
        *self = candidate;
        result.map_err(CausalCommitError::HistoryRead)
    }

    /// Commit a batch whose events may cause each other, atomically.
    ///
    /// `commit_batch` requires every cause to be already committed, which makes
    /// cycles impossible by construction but also makes same-tick ancestry
    /// unrepresentable: an infiltration that consumed water a forcing event
    /// delivered in the same tick can only cite the forcing event's *previous*
    /// state. That loses exactly the dependency a later reader needs.
    ///
    /// So local causes are allowed, and everything that `commit_batch` got for
    /// free has to be checked instead: unique keys, resolvable external traces,
    /// resolvable local references, the substage ordering rule, per-event caps,
    /// store capacity, and acyclicity. Only after all of that does the store
    /// change at all — every failure path returns before `self` is touched, so
    /// a rejected batch leaves the store byte-identical.
    ///
    /// Commit order is Kahn's algorithm with the ready set ordered by complete
    /// key bytes: the lexicographically least ready key is always taken first.
    /// Producer insertion order therefore has no effect on the trace IDs
    /// assigned, which is what makes the committed DAG replayable.
    ///
    /// Returns each proposal key's committed trace ID.
    pub fn commit_dag_batch(
        &mut self,
        time: SimulationTime,
        phase: Phase,
        proposals: Vec<CausalEventDagProposal>,
        limits: CausalDagBatchLimits,
    ) -> Result<BTreeMap<CausalEventProposalKey, TraceId>, CausalDagCommitError> {
        self.commit_dag_batch_inner(time, phase, proposals, limits, false)
    }

    /// Commit a DAG batch held by the current tick transaction.
    ///
    /// A single authoritative frame may be larger than the hot facade while it
    /// is being handed to the bounded history transaction. The caller must copy
    /// that frame into its transaction-local staging area and trim it with
    /// [`Self::trim_staged_frame_to_hot_budget`] before backend publication.
    /// Ordinary callers keep the strict bounded behavior of [`Self::commit_dag_batch`].
    pub fn commit_dag_batch_for_transaction(
        &mut self,
        time: SimulationTime,
        phase: Phase,
        proposals: Vec<CausalEventDagProposal>,
        limits: CausalDagBatchLimits,
    ) -> Result<BTreeMap<CausalEventProposalKey, TraceId>, CausalDagCommitError> {
        self.commit_dag_batch_inner(time, phase, proposals, limits, true)
    }

    fn commit_dag_batch_inner(
        &mut self,
        time: SimulationTime,
        phase: Phase,
        proposals: Vec<CausalEventDagProposal>,
        limits: CausalDagBatchLimits,
        allow_staged_overflow: bool,
    ) -> Result<BTreeMap<CausalEventProposalKey, TraceId>, CausalDagCommitError> {
        if proposals.is_empty() {
            return Ok(BTreeMap::new());
        }

        // -- unique keys, indexed by key ---------------------------------
        let mut indexed: BTreeMap<CausalEventProposalKey, &CausalEventDagProposal> =
            BTreeMap::new();
        for proposal in &proposals {
            if indexed.insert(proposal.key().clone(), proposal).is_some() {
                return Err(CausalDagCommitError::DuplicateProposalKey {
                    key: proposal.key().clone(),
                });
            }
        }

        // -- per-event caps, external traces, local references ------------
        for proposal in &proposals {
            if proposal.causes().len() > limits.max_causes_per_event {
                return Err(CausalDagCommitError::CauseLimitExceeded {
                    key: proposal.key().clone(),
                    count: proposal.causes().len(),
                    max: limits.max_causes_per_event,
                });
            }
            if proposal.effects().len() > limits.max_effects_per_event {
                return Err(CausalDagCommitError::EffectLimitExceeded {
                    key: proposal.key().clone(),
                    count: proposal.effects().len(),
                    max: limits.max_effects_per_event,
                });
            }
            for cause in proposal.causes() {
                match cause {
                    CausalEventDagCause::Existing(trace) => {
                        if !self
                            .trace_exists(*trace)
                            .map_err(CausalDagCommitError::HistoryRead)?
                        {
                            return Err(CausalDagCommitError::UnknownCause {
                                key: proposal.key().clone(),
                                cause: *trace,
                            });
                        }
                    }
                    CausalEventDagCause::Local(local) => {
                        if !indexed.contains_key(local) {
                            return Err(CausalDagCommitError::UnknownLocalCause {
                                key: proposal.key().clone(),
                                cause: local.clone(),
                            });
                        }
                        // Substages are frozen: one reads the state the
                        // previous produced. A local cause from a *later*
                        // substage would mean a substage read its own future,
                        // which the encoded ordinal exists to make checkable.
                        // Equal ordinals stay legal — within one substage the
                        // shape is an arbitrary DAG — and Kahn's algorithm
                        // below is what rejects a cycle among them.
                        if local.substage_ordinal() > proposal.key().substage_ordinal() {
                            return Err(CausalDagCommitError::SubstageOrderViolated {
                                key: proposal.key().clone(),
                                cause: local.clone(),
                            });
                        }
                    }
                }
            }
        }

        // -- store capacity ------------------------------------------------
        let count =
            u64::try_from(proposals.len()).map_err(|_| CausalDagCommitError::CapacityExceeded)?;
        self.next_event_id
            .checked_add(count)
            .ok_or(CausalDagCommitError::IdentifierExhausted)?;
        self.next_trace_id
            .checked_add(count)
            .ok_or(CausalDagCommitError::IdentifierExhausted)?;
        let added_causes = proposals.iter().map(|p| p.causes().len()).sum::<usize>();
        let added_effects = proposals.iter().map(|p| p.effects().len()).sum::<usize>();
        let final_causes = self
            .causes
            .len()
            .checked_add(added_causes)
            .ok_or(CausalDagCommitError::CapacityExceeded)?;
        let final_effects = self
            .effects
            .len()
            .checked_add(added_effects)
            .ok_or(CausalDagCommitError::CapacityExceeded)?;
        u32::try_from(final_causes).map_err(|_| CausalDagCommitError::CapacityExceeded)?;
        u32::try_from(final_effects).map_err(|_| CausalDagCommitError::CapacityExceeded)?;

        // -- canonical topological order -----------------------------------
        let mut pending: BTreeMap<&CausalEventProposalKey, usize> = BTreeMap::new();
        let mut dependents: BTreeMap<&CausalEventProposalKey, Vec<&CausalEventProposalKey>> =
            BTreeMap::new();
        for proposal in &proposals {
            let local_causes = proposal
                .causes()
                .iter()
                .filter_map(|cause| match cause {
                    CausalEventDagCause::Local(local) => Some(local),
                    CausalEventDagCause::Existing(_) => None,
                })
                .count();
            pending.insert(proposal.key(), local_causes);
            for cause in proposal.causes() {
                if let CausalEventDagCause::Local(local) = cause {
                    dependents
                        .entry(
                            indexed
                                .get_key_value(local)
                                .expect("local references were validated above")
                                .0,
                        )
                        .or_default()
                        .push(proposal.key());
                }
            }
        }

        let mut ready: BTreeSet<&CausalEventProposalKey> = pending
            .iter()
            .filter(|(_, degree)| **degree == 0)
            .map(|(&key, _)| key)
            .collect();
        let mut order: Vec<&CausalEventProposalKey> = Vec::with_capacity(proposals.len());
        while let Some(&next) = ready.iter().next() {
            ready.remove(next);
            order.push(next);
            if let Some(waiting) = dependents.get(next) {
                for dependent in waiting {
                    let degree = pending
                        .get_mut(*dependent)
                        .expect("every proposal is in the pending map");
                    *degree -= 1;
                    if *degree == 0 {
                        ready.insert(dependent);
                    }
                }
            }
        }
        if order.len() != proposals.len() {
            // Everything still pending is in a cycle or downstream of one. The
            // least such key names it deterministically, so the error is the
            // same on every machine that rejects the same batch.
            let unresolved = pending
                .iter()
                .filter(|(_, degree)| **degree > 0)
                .map(|(&key, _)| key.clone())
                .next()
                .expect("an incomplete order leaves at least one pending proposal");
            return Err(CausalDagCommitError::CyclicBatch { key: unresolved });
        }

        // -- reserve identifiers and resolve local causes ------------------
        let mut resolved: BTreeMap<CausalEventProposalKey, TraceId> = BTreeMap::new();
        let mut prepared: Vec<PreparedDagEvent> = Vec::with_capacity(order.len());
        for (offset, key) in order.iter().enumerate() {
            let offset = offset as u64;
            let event_id = EventId::new(self.next_event_id + offset);
            let trace_id = TraceId::new(self.next_trace_id + offset);
            let proposal = indexed[*key];
            let mut causes = Vec::with_capacity(proposal.causes().len());
            for cause in proposal.causes() {
                causes.push(match cause {
                    CausalEventDagCause::Existing(trace) => *trace,
                    CausalEventDagCause::Local(local) => *resolved
                        .get(local)
                        .expect("topological order commits every local cause first"),
                });
            }
            // The proposal's own cause list is strictly ordered on its
            // unresolved form; the resolved IDs interleave differently. Cause
            // sets are unordered, so sorting is the identity here — but the
            // uniqueness check is not, and it is what a corrupted resolution
            // would trip.
            causes.sort_unstable();
            if let Err(index) = validate_strict_order(&causes) {
                return Err(CausalDagCommitError::DuplicateResolvedCause {
                    key: (*key).clone(),
                    cause: causes[index],
                });
            }
            resolved.insert((*key).clone(), trace_id);
            prepared.push(PreparedDagEvent {
                event_id,
                trace_id,
                kind: proposal.kind(),
                causes,
                effects: proposal.effects().to_vec(),
            });
        }

        let prepared = prepared
            .into_iter()
            .map(|event| {
                OwnedCausalRecord::new(
                    time.raw(),
                    event.event_id,
                    event.trace_id,
                    time,
                    phase,
                    event.kind,
                    event.causes,
                    event.effects,
                )
                .map_err(CausalDagCommitError::HistoryRead)
            })
            .collect::<Result<Vec<_>, _>>()?;
        let added_bytes = prepared.iter().try_fold(0_usize, |bytes, record| {
            bytes
                .checked_add(
                    record
                        .canonical_field_size()
                        .map_err(CausalDagCommitError::HistoryRead)?,
                )
                .ok_or(CausalDagCommitError::CapacityExceeded)
        })?;
        let evict = if allow_staged_overflow || self.transaction_undo.is_some() {
            0
        } else {
            self.evictable_prefix(added_bytes)
                .map_err(CausalDagCommitError::HistoryRead)?
        };
        let hot_budget = self.hot_budget;
        let mut candidate = std::mem::replace(self, Self::with_hot_budget(hot_budget));
        let result = (|| {
            candidate.evict_hot_prefix(evict)?;
            for record in prepared {
                candidate.push_hot_record(record)?;
            }
            candidate.next_event_id += count;
            candidate.next_trace_id += count;
            Ok(())
        })();
        *self = candidate;
        result
            .map(|()| resolved)
            .map_err(CausalDagCommitError::HistoryRead)
    }

    /// Trim a staged causal frame after it has been copied into a history
    /// transaction but before that transaction publishes to storage.
    pub fn trim_staged_frame_to_hot_budget(
        &mut self,
        frame: &[HistoryRecord],
    ) -> Result<(), HistoryError> {
        if self.hot_bytes <= self.hot_budget {
            return Ok(());
        }
        let Some(first_staged_trace) = frame.iter().find_map(|record| match record {
            HistoryRecord::Causal(record) => Some(record.trace_id()),
            HistoryRecord::Evidence(_) => None,
        }) else {
            return Err(HistoryError::HotBudgetExceeded {
                required: self.hot_bytes,
                budget: self.hot_budget,
            });
        };
        let Some(staged_start) = self.trace_index(first_staged_trace) else {
            return Err(HistoryError::HotBudgetExceeded {
                required: self.hot_bytes,
                budget: self.hot_budget,
            });
        };
        let mut staged = 0_usize;
        for record in frame {
            let HistoryRecord::Causal(record) = record else {
                continue;
            };
            let index = staged_start
                .checked_add(staged)
                .ok_or(HistoryError::ArithmeticOverflow)?;
            if index >= self.len() {
                return Err(HistoryError::HotBudgetExceeded {
                    required: self.hot_bytes,
                    budget: self.hot_budget,
                });
            }
            let hot = self.event_at(index);
            if hot.commit_sequence != record.commit_sequence()
                || hot.event_id != record.event_id()
                || hot.trace_id != record.trace_id()
                || hot.time != record.time()
                || hot.phase != record.phase()
                || hot.kind != record.kind()
                || hot.causes != record.causes()
                || hot.effects != record.effects()
            {
                return Err(HistoryError::HotBudgetExceeded {
                    required: self.hot_bytes,
                    budget: self.hot_budget,
                });
            }
            staged += 1;
        }
        if staged != self.len().saturating_sub(staged_start) {
            return Err(HistoryError::HotBudgetExceeded {
                required: self.hot_bytes,
                budget: self.hot_budget,
            });
        }
        if staged_start > 0 {
            let Some(reader) = &self.cold_reader else {
                return Err(HistoryError::HotBudgetExceeded {
                    required: self.hot_bytes,
                    budget: self.hot_budget,
                });
            };
            let boundary = self.event_at_owned(staged_start - 1)?;
            if reader.event(boundary.trace_id())?.as_ref() != Some(&boundary) {
                return Err(HistoryError::HotBudgetExceeded {
                    required: self.hot_bytes,
                    budget: self.hot_budget,
                });
            }
        }
        let mut required = self.hot_bytes;
        let mut remove = 0_usize;
        while required > self.hot_budget && remove < self.len() {
            let record = self.event_at(remove);
            required = required
                .checked_sub(record.canonical_field_size()?)
                .ok_or(HistoryError::ArithmeticOverflow)?;
            remove += 1;
        }
        if required > self.hot_budget {
            return Err(HistoryError::HotBudgetExceeded {
                required,
                budget: self.hot_budget,
            });
        }
        if let Some(undo) = self.transaction_undo.as_mut() {
            undo.deferred_eviction = Some(remove);
            Ok(())
        } else {
            self.evict_hot_prefix(remove)
        }
    }

    pub fn event(&self, trace_id: TraceId) -> Result<Option<OwnedCausalRecord>, HistoryError> {
        if let Some(index) = self.trace_index(trace_id) {
            return self.event_at_owned(index).map(Some);
        }
        self.cold_reader
            .as_ref()
            .map_or(Ok(None), |reader| reader.event(trace_id))
    }

    pub fn event_by_id(
        &self,
        event_id: EventId,
    ) -> Result<Option<OwnedCausalRecord>, HistoryError> {
        if let Ok(index) = self.event_ids.binary_search(&event_id) {
            return self.event_at_owned(index).map(Some);
        }
        self.cold_reader
            .as_ref()
            .map_or(Ok(None), |reader| reader.event_by_id(event_id))
    }

    pub fn parents(
        &self,
        trace_id: TraceId,
        query: HistoryQuery,
    ) -> Result<HistoryPage<TraceId>, HistoryError> {
        if let Some(index) = self.trace_index(trace_id) {
            return Self::page(self.event_at(index).causes, query);
        }
        self.cold_reader.as_ref().map_or_else(
            || HistoryPage::new(Vec::new(), None, query),
            |reader| reader.parents(trace_id, query),
        )
    }

    pub fn children(
        &self,
        trace_id: TraceId,
        query: HistoryQuery,
    ) -> Result<HistoryPage<TraceId>, HistoryError> {
        let hot = self.children.get(&trace_id).map_or(&[][..], Vec::as_slice);
        let Some(reader) = &self.cold_reader else {
            return Self::page(hot, query);
        };
        let mut overlay = Vec::with_capacity(hot.len());
        for child in hot {
            if reader.event(*child)?.is_none() {
                overlay.push(*child);
            }
        }
        if query.offset() & HOT_CHILD_CURSOR_FLAG != 0 {
            let overlay_query = HistoryQuery::new(
                query.offset() & !HOT_CHILD_CURSOR_FLAG,
                query.max_records(),
                query.max_bytes(),
            )?;
            let page = Self::page(&overlay, overlay_query)?;
            let next_offset = page
                .next_offset()
                .map(|offset| HOT_CHILD_CURSOR_FLAG | offset);
            return HistoryPage::new(page.records().to_vec(), next_offset, query);
        }
        let cold = reader.children(trace_id, query)?;
        if overlay.is_empty() || cold.next_offset().is_some() {
            return Ok(cold);
        }
        let overlay_query = HistoryQuery::new(
            0,
            query.max_records() - cold.records().len(),
            query.max_bytes() - cold.decoded_bytes(),
        )?;
        let overlay_page = Self::page(&overlay, overlay_query)?;
        let mut children = cold.records().to_vec();
        children.extend_from_slice(overlay_page.records());
        let next_offset = overlay_page
            .next_offset()
            .map(|offset| HOT_CHILD_CURSOR_FLAG | offset);
        HistoryPage::new(children, next_offset, query)
    }

    pub fn indexed_range(
        &self,
        query: HistoryQuery,
    ) -> Result<HistoryPage<HistoryRecord>, HistoryError> {
        if let Some(reader) = &self.cold_reader {
            let frontier = reader.anchor()?.frontier();
            let cold_count = if reader.event(frontier.trace_id)?.is_some() {
                frontier
                    .trace_id
                    .raw()
                    .checked_add(1)
                    .ok_or(HistoryError::ArithmeticOverflow)?
            } else {
                0
            };
            let hot = self
                .iter_hot()
                .filter(|event| event.trace_id.raw() >= cold_count)
                .map(|event| {
                    self.event_at_owned(
                        self.trace_index(event.trace_id)
                            .ok_or(HistoryError::ReaderOperationUnavailable)?,
                    )
                })
                .map(|record| record.map(HistoryRecord::Causal))
                .collect::<Result<Vec<_>, _>>()?;
            if query.offset() >= cold_count {
                let hot_offset = query
                    .offset()
                    .checked_sub(cold_count)
                    .ok_or(HistoryError::ArithmeticOverflow)?;
                let hot_query =
                    HistoryQuery::new(hot_offset, query.max_records(), query.max_bytes())?;
                return Self::page(&hot, hot_query);
            }
            let cold = reader.indexed_range(query)?;
            if cold.next_offset().is_some() || hot.is_empty() {
                return Ok(cold);
            }
            let overlay_query = HistoryQuery::new(
                0,
                query.max_records().saturating_sub(cold.records().len()),
                query.max_bytes().saturating_sub(cold.decoded_bytes()),
            )?;
            let overlay = Self::page(&hot, overlay_query)?;
            let mut records = cold.records().to_vec();
            records.extend_from_slice(overlay.records());
            let overlay_len = u64::try_from(overlay.records().len())
                .map_err(|_| HistoryError::ArithmeticOverflow)?;
            let hot_len = u64::try_from(hot.len()).map_err(|_| HistoryError::ArithmeticOverflow)?;
            let next_offset = overlay
                .next_offset()
                .map(|offset| cold_count + offset)
                .or_else(|| (overlay_len < hot_len).then_some(cold_count + overlay_len));
            return HistoryPage::new(records, next_offset, query);
        }
        let records = (0..self.hot_len())
            .map(|index| self.event_at_owned(index).map(HistoryRecord::Causal))
            .collect::<Result<Vec<_>, _>>()?;
        Self::page(&records, query)
    }

    pub fn latest_matching_event_before(
        &self,
        target: CausalTarget,
        trace_id: TraceId,
    ) -> Result<Option<OwnedCausalRecord>, HistoryError> {
        let hot = (0..self.len())
            .rev()
            .map(|index| self.event_at_owned(index))
            .find_map(|record| match record {
                Ok(record)
                    if record.trace_id() < trace_id
                        && record
                            .effects()
                            .iter()
                            .any(|effect| effect.target() == target) =>
                {
                    Some(Ok(record))
                }
                Ok(_) => None,
                Err(error) => Some(Err(error)),
            })
            .transpose()?;
        let cold = self.cold_reader.as_ref().map_or(Ok(None), |reader| {
            reader.latest_matching_event_before(target, trace_id)
        })?;
        Ok(match (hot, cold) {
            (Some(hot), Some(cold)) if cold.trace_id() > hot.trace_id() => Some(cold),
            (Some(hot), _) => Some(hot),
            (None, cold) => cold,
        })
    }

    pub fn iter_hot(&self) -> impl ExactSizeIterator<Item = CausalEventRef<'_>> + '_ {
        (0..self.len()).map(|index| self.event_at(index))
    }

    fn trace_index(&self, trace_id: TraceId) -> Option<usize> {
        self.trace_ids.binary_search(&trace_id).ok()
    }

    fn event_at(&self, index: usize) -> CausalEventRef<'_> {
        let cause_start = self.cause_offsets[index] as usize;
        let cause_end = self.cause_offsets[index + 1] as usize;
        let effect_start = self.effect_offsets[index] as usize;
        let effect_end = self.effect_offsets[index + 1] as usize;
        CausalEventRef {
            commit_sequence: self.commit_sequences[index],
            event_id: self.event_ids[index],
            trace_id: self.trace_ids[index],
            time: self.times[index],
            phase: self.phases[index],
            kind: self.kinds[index],
            causes: &self.causes[cause_start..cause_end],
            effects: &self.effects[effect_start..effect_end],
        }
    }

    fn event_at_owned(&self, index: usize) -> Result<OwnedCausalRecord, HistoryError> {
        let event = self.event_at(index);
        OwnedCausalRecord::new(
            event.commit_sequence,
            event.event_id,
            event.trace_id,
            event.time,
            event.phase,
            event.kind,
            event.causes.to_vec(),
            event.effects.to_vec(),
        )
    }

    fn trace_exists(&self, trace_id: TraceId) -> Result<bool, HistoryError> {
        if trace_id.raw() < self.next_trace_id || self.trace_index(trace_id).is_some() {
            return Ok(true);
        }
        self.cold_reader.as_ref().map_or(Ok(false), |reader| {
            reader.event(trace_id).map(|event| event.is_some())
        })
    }

    fn push_hot_record(&mut self, record: OwnedCausalRecord) -> Result<(), HistoryError> {
        let record_bytes = record.canonical_field_size()?;
        let next_bytes = self
            .hot_bytes
            .checked_add(record_bytes)
            .ok_or(HistoryError::ArithmeticOverflow)?;
        let cause_offset = self
            .causes
            .len()
            .checked_add(record.causes().len())
            .and_then(|offset| u32::try_from(offset).ok())
            .ok_or(HistoryError::ArithmeticOverflow)?;
        let effect_offset = self
            .effects
            .len()
            .checked_add(record.effects().len())
            .and_then(|offset| u32::try_from(offset).ok())
            .ok_or(HistoryError::ArithmeticOverflow)?;
        self.commit_sequences.push(record.commit_sequence());
        self.event_ids.push(record.event_id());
        self.trace_ids.push(record.trace_id());
        self.times.push(record.time());
        self.phases.push(record.phase());
        self.kinds.push(record.kind());
        self.causes.extend_from_slice(record.causes());
        self.cause_offsets.push(cause_offset);
        self.effects.extend_from_slice(record.effects());
        self.effect_offsets.push(effect_offset);
        for cause in record.causes() {
            if let Some(undo) = self.transaction_undo.as_mut() {
                let previous_len = self.children.get(cause).map_or(0, Vec::len);
                undo.child_appends.push(ChildIndexUndo {
                    cause: *cause,
                    previous_len,
                });
            }
            self.children
                .entry(*cause)
                .or_default()
                .push(record.trace_id());
        }
        self.hot_bytes = next_bytes;
        Ok(())
    }

    fn evictable_prefix(&self, extra_bytes: usize) -> Result<usize, HistoryError> {
        let mut required = self
            .hot_bytes
            .checked_add(extra_bytes)
            .ok_or(HistoryError::ArithmeticOverflow)?;
        let mut count = 0;
        while required > self.hot_budget && count < self.len() {
            let record = self.event_at_owned(count)?;
            let Some(reader) = &self.cold_reader else {
                return Err(HistoryError::HotBudgetExceeded {
                    required,
                    budget: self.hot_budget,
                });
            };
            if reader.event(record.trace_id())?.as_ref() != Some(&record) {
                return Err(HistoryError::HotBudgetExceeded {
                    required,
                    budget: self.hot_budget,
                });
            }
            required = required
                .checked_sub(record.canonical_field_size()?)
                .ok_or(HistoryError::ArithmeticOverflow)?;
            count += 1;
        }
        if required > self.hot_budget {
            return Err(HistoryError::HotBudgetExceeded {
                required,
                budget: self.hot_budget,
            });
        }
        Ok(count)
    }

    /// Eviction count for records whose cold copy the caller has just proven in
    /// one batch (`validate_causal_overlay`), so no per-record cold read repeats.
    fn evictable_prefix_proven(&self) -> Result<usize, HistoryError> {
        let mut required = self.hot_bytes;
        let mut count = 0;
        while required > self.hot_budget && count < self.len() {
            required = required
                .checked_sub(self.event_at_owned(count)?.canonical_field_size()?)
                .ok_or(HistoryError::ArithmeticOverflow)?;
            count += 1;
        }
        if required > self.hot_budget {
            return Err(HistoryError::HotBudgetExceeded {
                required,
                budget: self.hot_budget,
            });
        }
        Ok(count)
    }

    fn evict_hot_prefix(&mut self, count: usize) -> Result<(), HistoryError> {
        if count == 0 {
            return Ok(());
        }
        let removed = (0..count)
            .map(|index| self.event_at_owned(index))
            .collect::<Result<Vec<_>, _>>()?;
        let removed_bytes = removed.iter().try_fold(0_usize, |total, record| {
            total
                .checked_add(record.canonical_field_size()?)
                .ok_or(HistoryError::ArithmeticOverflow)
        })?;
        let remaining_bytes = self
            .hot_bytes
            .checked_sub(removed_bytes)
            .ok_or(HistoryError::ArithmeticOverflow)?;
        let cause_offset = self.cause_offsets[count];
        let effect_offset = self.effect_offsets[count];
        let cause_count = cause_offset as usize;
        let effect_count = effect_offset as usize;
        self.commit_sequences.drain(0..count);
        self.event_ids.drain(0..count);
        self.trace_ids.drain(0..count);
        self.times.drain(0..count);
        self.phases.drain(0..count);
        self.kinds.drain(0..count);
        self.causes.drain(0..cause_count);
        self.effects.drain(0..effect_count);
        self.cause_offsets.drain(0..count);
        self.effect_offsets.drain(0..count);
        for offset in &mut self.cause_offsets {
            *offset -= cause_offset;
        }
        for offset in &mut self.effect_offsets {
            *offset -= effect_offset;
        }
        for record in removed {
            for cause in record.causes() {
                if let Some(children) = self.children.get_mut(cause) {
                    children.retain(|child| *child != record.trace_id());
                }
            }
            if self
                .children
                .get(&record.trace_id())
                .is_some_and(Vec::is_empty)
            {
                self.children.remove(&record.trace_id());
            }
        }
        self.hot_bytes = remaining_bytes;
        Ok(())
    }

    fn page<T: HistoryPageItem + Clone>(
        values: &[T],
        query: HistoryQuery,
    ) -> Result<HistoryPage<T>, HistoryError> {
        let start =
            usize::try_from(query.offset()).map_err(|_| HistoryError::ArithmeticOverflow)?;
        if start >= values.len() {
            return HistoryPage::new(Vec::new(), None, query);
        }
        let mut selected = Vec::new();
        let mut bytes = 0_usize;
        for value in &values[start..] {
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
        let consumed = start
            .checked_add(selected.len())
            .ok_or(HistoryError::ArithmeticOverflow)?;
        let next_offset = (consumed < values.len())
            .then(|| u64::try_from(consumed).map_err(|_| HistoryError::ArithmeticOverflow))
            .transpose()?;
        HistoryPage::new(selected, next_offset, query)
    }
}

impl Default for CausalTraceStore {
    fn default() -> Self {
        Self::new()
    }
}

/// Logical snapshot of the bounded hot records in a `CausalTraceStore`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CausalTraceSnapshot {
    pub next_event_id: u64,
    pub next_trace_id: u64,
    pub events: Vec<CausalEventSnapshot>,
}

/// Logical snapshot of a single causal event.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CausalEventSnapshot {
    pub event_id: EventId,
    pub trace_id: TraceId,
    pub time: SimulationTime,
    pub phase: Phase,
    pub kind: EventKindId,
    pub causes: Vec<TraceId>,
    pub effects: Vec<CausalEffect>,
}

#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum CausalSnapshotError {
    #[error("event count exceeds capacity")]
    EventCountExceeded,
    #[error("cause count exceeds capacity")]
    CauseCountExceeded,
    #[error("effect count exceeds capacity")]
    EffectCountExceeded,
    #[error("trace IDs are not strictly monotonic")]
    NonMonotonicTraceIds,
    #[error("event IDs are not strictly increasing")]
    NonMonotonicEventIds,
    #[error("event times are not non-decreasing")]
    NonMonotonicTimes,
    #[error("event carries no effects")]
    NoEffects,
    #[error("cause refers to unknown trace ID: {0}")]
    UnknownCause(TraceId),
    #[error("causal hot cache requires {required} bytes but its budget is {budget}")]
    HotBudgetExceeded { required: usize, budget: usize },
}

impl CausalTraceStore {
    /// Export only the bounded hot cache, never the reader's cold history.
    pub fn export_snapshot(&self) -> CausalTraceSnapshot {
        let events = self
            .iter_hot()
            .map(|event| CausalEventSnapshot {
                event_id: event.event_id,
                trace_id: event.trace_id,
                time: event.time,
                phase: event.phase,
                kind: event.kind,
                causes: event.causes.to_vec(),
                effects: event.effects.to_vec(),
            })
            .collect();
        CausalTraceSnapshot {
            next_event_id: self.next_event_id,
            next_trace_id: self.next_trace_id,
            events,
        }
    }

    /// Export the logical history through the explicit indexed reader path.
    ///
    /// This is a durable/legacy operation, not a hot-cache iterator. A runtime
    /// tick must use [`Self::export_snapshot`] or a bounded overlay instead.
    pub fn export_snapshot_durable(&self) -> Result<CausalTraceSnapshot, HistoryError> {
        let mut offset = 0_u64;
        let mut events = Vec::new();
        loop {
            let query = HistoryQuery::new(offset, MAX_QUERY_PAGE_RECORDS, MAX_QUERY_PAGE_BYTES)?;
            let page = self
                .cold_reader
                .as_ref()
                .map_or_else(|| self.indexed_range(query), |reader| reader.records(query))?;
            for record in page.records() {
                if let HistoryRecord::Causal(record) = record {
                    events.push(CausalEventSnapshot {
                        event_id: record.event_id(),
                        trace_id: record.trace_id(),
                        time: record.time(),
                        phase: record.phase(),
                        kind: record.kind(),
                        causes: record.causes().to_vec(),
                        effects: record.effects().to_vec(),
                    });
                }
            }
            let Some(next) = page.next_offset() else {
                break;
            };
            if next <= offset {
                return Err(HistoryError::ReaderOperationUnavailable);
            }
            offset = next;
        }
        Ok(CausalTraceSnapshot {
            next_event_id: self.next_event_id,
            next_trace_id: self.next_trace_id,
            events,
        })
    }

    /// Return the exact canonical hot-byte requirement for a logical snapshot.
    pub fn snapshot_required_bytes(
        snapshot: &CausalTraceSnapshot,
    ) -> Result<usize, CausalSnapshotError> {
        if snapshot.events.len() > usize::MAX / 2 {
            return Err(CausalSnapshotError::EventCountExceeded);
        }
        snapshot.events.iter().try_fold(0_usize, |total, event| {
            let cause_bytes = event
                .causes
                .len()
                .checked_mul(8)
                .ok_or(CausalSnapshotError::EventCountExceeded)?;
            let effect_bytes = event
                .effects
                .len()
                .checked_mul(3 * 8 + 2 * 32)
                .ok_or(CausalSnapshotError::EventCountExceeded)?;
            let event_bytes = 6_usize
                .checked_mul(8)
                .and_then(|size| size.checked_add(cause_bytes))
                .and_then(|size| size.checked_add(8))
                .and_then(|size| size.checked_add(effect_bytes))
                .ok_or(CausalSnapshotError::EventCountExceeded)?;
            total
                .checked_add(event_bytes)
                .ok_or(CausalSnapshotError::EventCountExceeded)
        })
    }

    /// Reconstruct a trace store from a logical snapshot.
    ///
    /// Validates monotonic IDs, parent-before-child ordering, and
    /// cause existence. Rebuilds the child index deterministically.
    pub fn import_snapshot(snapshot: CausalTraceSnapshot) -> Result<Self, CausalSnapshotError> {
        Self::import_snapshot_with_hot_budget(snapshot, DEFAULT_CAUSAL_HOT_BUDGET_BYTES)
    }

    /// Reconstruct a trace store with an explicit import budget.
    ///
    /// Runtime legacy import uses a temporary budget equal to the supplied
    /// snapshot's checked requirement, then immediately installs the records
    /// behind a bounded cold-reader facade. The resulting runtime never owns
    /// this temporary full store.
    pub fn import_snapshot_with_hot_budget(
        snapshot: CausalTraceSnapshot,
        hot_budget: usize,
    ) -> Result<Self, CausalSnapshotError> {
        let event_count = snapshot.events.len();
        if event_count > usize::MAX / 2 {
            return Err(CausalSnapshotError::EventCountExceeded);
        }

        let mut store = Self::with_hot_budget(hot_budget);
        store.next_event_id = snapshot.next_event_id;
        store.next_trace_id = snapshot.next_trace_id;

        let mut last_trace_id: Option<TraceId> = None;
        let mut last_event_id: Option<EventId> = None;
        let mut last_time: Option<SimulationTime> = None;
        for event in &snapshot.events {
            // Import accepted two shapes the commit path forbids, and both made
            // a forged trailing event cheap. `CausalEventProposal::new` rejects
            // an empty effect set, so no committed event has ever had one: an
            // effectless event is pure padding that carries no state transition
            // and exists only to change how the store's tail reads.
            if event.effects.is_empty() {
                return Err(CausalSnapshotError::NoEffects);
            }
            // Commits advance a tick at a time, so store order is time order.
            // Without this, a store whose times run 0…0, 5, 1 imports cleanly
            // and any bound phrased as `any(time > limit)` misses the event
            // hiding behind the larger one.
            if let Some(last) = last_time
                && event.time < last
            {
                return Err(CausalSnapshotError::NonMonotonicTimes);
            }
            last_time = Some(event.time);
            if let Some(last) = last_trace_id
                && event.trace_id <= last
            {
                return Err(CausalSnapshotError::NonMonotonicTraceIds);
            }
            last_trace_id = Some(event.trace_id);
            // Event IDs are binary-searched by `event_by_id`, exactly as trace
            // IDs are by `trace_index`, so they need the same ordering guarantee
            // and did not have it.
            if let Some(last) = last_event_id
                && event.event_id <= last
            {
                return Err(CausalSnapshotError::NonMonotonicEventIds);
            }
            last_event_id = Some(event.event_id);

            store.event_ids.push(event.event_id);
            store.commit_sequences.push(event.time.raw());
            store.trace_ids.push(event.trace_id);
            store.times.push(event.time);
            store.phases.push(event.phase);
            store.kinds.push(event.kind);

            let cause_count = event.causes.len();
            if cause_count > u32::MAX as usize {
                return Err(CausalSnapshotError::CauseCountExceeded);
            }
            for &cause in &event.causes {
                if store.trace_index(cause).is_none() {
                    return Err(CausalSnapshotError::UnknownCause(cause));
                }
            }
            store.causes.extend_from_slice(&event.causes);
            store.cause_offsets.push(store.causes.len() as u32);

            let effect_count = event.effects.len();
            if effect_count > u32::MAX as usize {
                return Err(CausalSnapshotError::EffectCountExceeded);
            }
            store.effects.extend_from_slice(&event.effects);
            store.effect_offsets.push(store.effects.len() as u32);
            store.hot_bytes = store
                .hot_bytes
                .checked_add(
                    6_usize
                        .checked_mul(8)
                        .and_then(|size| event.causes.len().checked_mul(8)?.checked_add(size))
                        .and_then(|size| size.checked_add(8))
                        .and_then(|size| {
                            event
                                .effects
                                .len()
                                .checked_mul(3 * 8 + 2 * 32)?
                                .checked_add(size)
                        })
                        .ok_or(CausalSnapshotError::EventCountExceeded)?,
                )
                .ok_or(CausalSnapshotError::EventCountExceeded)?;

            // Rebuild child index.
            for &cause in &event.causes {
                store
                    .children
                    .entry(cause)
                    .or_default()
                    .push(event.trace_id);
            }
        }

        // The counters decide which identifiers the next commit issues. Rolled
        // back below what the store already holds, `commit_batch` re-issues live
        // IDs: the arrays stop being sorted, and both `trace_index` and
        // `event_by_id` binary-search them, so every provenance lookup — receipt
        // validation and Explanation evidence included — can silently resolve to
        // the wrong event. The corrupted store only fails to re-import one save
        // later, which is one save too late.
        if let Some(last) = last_trace_id
            && store.next_trace_id <= last.raw()
        {
            return Err(CausalSnapshotError::NonMonotonicTraceIds);
        }
        if let Some(last) = last_event_id
            && store.next_event_id <= last.raw()
        {
            return Err(CausalSnapshotError::NonMonotonicEventIds);
        }
        if store.hot_bytes > store.hot_budget {
            return Err(CausalSnapshotError::HotBudgetExceeded {
                required: store.hot_bytes,
                budget: store.hot_budget,
            });
        }
        Ok(store)
    }
}

#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum CausalDagCommitError {
    #[error("causal history read failed: {0}")]
    HistoryRead(HistoryError),
    #[error("proposal key appears more than once in a DAG batch")]
    DuplicateProposalKey { key: CausalEventProposalKey },
    #[error("a DAG proposal refers to unknown prior cause {cause}")]
    UnknownCause {
        key: CausalEventProposalKey,
        cause: TraceId,
    },
    #[error("a DAG proposal refers to a local cause that is not in the batch")]
    UnknownLocalCause {
        key: CausalEventProposalKey,
        cause: CausalEventProposalKey,
    },
    #[error("a DAG proposal cites a local cause from a later substage")]
    SubstageOrderViolated {
        key: CausalEventProposalKey,
        cause: CausalEventProposalKey,
    },
    #[error("a DAG batch contains a cycle among its local causes")]
    CyclicBatch { key: CausalEventProposalKey },
    #[error("a DAG proposal carries {count} causes, at most {max} are allowed")]
    CauseLimitExceeded {
        key: CausalEventProposalKey,
        count: usize,
        max: usize,
    },
    #[error("a DAG proposal carries {count} effects, at most {max} are allowed")]
    EffectLimitExceeded {
        key: CausalEventProposalKey,
        count: usize,
        max: usize,
    },
    #[error("a DAG proposal resolved to duplicate cause {cause}")]
    DuplicateResolvedCause {
        key: CausalEventProposalKey,
        cause: TraceId,
    },
    #[error("causal event or trace identifier space is exhausted")]
    IdentifierExhausted,
    #[error("causal store flat offset capacity is exceeded")]
    CapacityExceeded,
}

#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum CausalCommitError {
    #[error("causal history read failed: {0}")]
    HistoryRead(HistoryError),
    #[error("proposal key {key:?} appears more than once in a commit batch")]
    DuplicateProposalKey { key: EventProposalKey },
    #[error("proposal {key:?} refers to unknown prior cause {cause}")]
    UnknownCause {
        key: EventProposalKey,
        cause: TraceId,
    },
    #[error("causal event or trace identifier space is exhausted")]
    IdentifierExhausted,
    #[error("causal store flat offset capacity is exceeded")]
    CapacityExceeded,
}

fn validate_strict_order<T: Ord>(values: &[T]) -> Result<(), usize> {
    for index in 1..values.len() {
        if values[index - 1] >= values[index] {
            return Err(index);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fingerprint(byte: u8) -> StateFingerprint {
        StateFingerprint::new([byte; 32])
    }

    fn effect(object_id: u64, before: u8, after: u8) -> CausalEffect {
        CausalEffect::new(
            CausalTarget::new(
                StateObjectKindId::new(4),
                object_id,
                StatePropertyId::new(9),
            ),
            fingerprint(before),
            fingerprint(after),
        )
        .unwrap()
    }

    fn proposal(key: u64, causes: Vec<TraceId>, object_id: u64) -> CausalEventProposal {
        CausalEventProposal::new(
            EventProposalKey::new(2, key, 0),
            EventKindId::new(7),
            causes,
            vec![effect(object_id, 1, 2)],
        )
        .unwrap()
    }

    #[test]
    fn unchanged_effect_is_rejected() {
        let target = CausalTarget::new(StateObjectKindId::new(1), 2, StatePropertyId::new(3));
        assert_eq!(
            CausalEffect::new(target, fingerprint(5), fingerprint(5)),
            Err(CausalEffectError::UnchangedState { target })
        );
    }

    #[test]
    fn proposals_require_canonical_edges() {
        assert_eq!(
            CausalEventProposal::new(
                EventProposalKey::new(0, 0, 0),
                EventKindId::new(1),
                vec![TraceId::new(2), TraceId::new(1)],
                vec![effect(1, 0, 1)],
            ),
            Err(CausalEventProposalError::CausesNotStrictlyOrdered { index: 1 })
        );
        assert_eq!(
            CausalEventProposal::new(
                EventProposalKey::new(0, 0, 0),
                EventKindId::new(1),
                vec![],
                vec![],
            ),
            Err(CausalEventProposalError::NoEffects)
        );
    }

    #[test]
    fn reduction_order_is_independent_of_input_order() {
        let proposals = vec![proposal(8, vec![], 80), proposal(3, vec![], 30)];
        let mut a = CausalTraceStore::new();
        let mut b = CausalTraceStore::new();
        a.commit_batch(SimulationTime::new(5), Phase::Physics, proposals.clone())
            .unwrap();
        b.commit_batch(
            SimulationTime::new(5),
            Phase::Physics,
            proposals.into_iter().rev().collect(),
        )
        .unwrap();
        assert_eq!(a, b);
        assert_eq!(
            a.event(TraceId::new(0)).unwrap().unwrap().effects()[0]
                .target()
                .object_id(),
            30
        );
    }

    #[test]
    fn graph_retains_forward_and_reverse_edges() {
        let mut store = CausalTraceStore::new();
        let roots = store
            .commit_batch(
                SimulationTime::new(1),
                Phase::Physics,
                vec![proposal(0, vec![], 10), proposal(1, vec![], 11)],
            )
            .unwrap();
        let child = store
            .commit_batch(
                SimulationTime::new(2),
                Phase::Resolution,
                vec![proposal(0, roots.clone(), 12)],
            )
            .unwrap()[0];

        assert_eq!(store.event(child).unwrap().unwrap().causes(), roots);
        let query = HistoryQuery::new(0, 4, 64).unwrap();
        assert_eq!(
            store.children(TraceId::new(0), query).unwrap().records(),
            &[child]
        );
        assert_eq!(
            store.children(TraceId::new(1), query).unwrap().records(),
            &[child]
        );
        assert!(store.children(child, query).unwrap().records().is_empty());
    }

    #[test]
    fn unknown_causes_do_not_partially_commit() {
        let mut store = CausalTraceStore::new();
        let result = store.commit_batch(
            SimulationTime::new(1),
            Phase::Action,
            vec![proposal(0, vec![TraceId::new(44)], 1)],
        );
        assert!(matches!(
            result,
            Err(CausalCommitError::UnknownCause { .. })
        ));
        assert!(store.is_empty());
    }

    #[test]
    fn duplicate_proposal_keys_are_rejected() {
        let mut store = CausalTraceStore::new();
        let result = store.commit_batch(
            SimulationTime::new(1),
            Phase::Physics,
            vec![proposal(0, vec![], 1), proposal(0, vec![], 2)],
        );
        assert!(matches!(
            result,
            Err(CausalCommitError::DuplicateProposalKey { .. })
        ));
    }

    // -- atomic local-cause DAG commit ------------------------------------

    const LIMITS: CausalDagBatchLimits = CausalDagBatchLimits {
        max_causes_per_event: 16,
        max_effects_per_event: 8,
    };

    fn dag_key(substage: u8, process: u32, carrier: &[u8], local: u32) -> CausalEventProposalKey {
        CausalEventProposalKey::new(substage, process, carrier, local).unwrap()
    }

    fn dag_proposal(
        key: CausalEventProposalKey,
        causes: Vec<CausalEventDagCause>,
        object_id: u64,
    ) -> CausalEventDagProposal {
        CausalEventDagProposal::new(
            key,
            EventKindId::new(36),
            causes,
            vec![effect(object_id, 1, 2)],
        )
        .unwrap()
    }

    /// Three substages of one cell: forcing, then infiltration citing it, then
    /// percolation citing infiltration. The shape hydrology actually commits.
    fn vertical_chain() -> Vec<CausalEventDagProposal> {
        let forcing = dag_key(1, 100, b"cell", 0);
        let infiltration = dag_key(2, 101, b"cell", 0);
        let percolation = dag_key(3, 102, b"cell", 0);
        vec![
            dag_proposal(forcing.clone(), vec![], 10),
            dag_proposal(
                infiltration.clone(),
                vec![CausalEventDagCause::Local(forcing)],
                11,
            ),
            dag_proposal(
                percolation,
                vec![CausalEventDagCause::Local(infiltration)],
                12,
            ),
        ]
    }

    #[test]
    fn a_local_cause_resolves_to_a_trace_committed_in_the_same_batch() {
        // Given: a store with nothing in it.
        let mut store = CausalTraceStore::new();

        // When: three chained substages commit as one atomic batch.
        let resolved = store
            .commit_dag_batch(
                SimulationTime::new(4),
                Phase::Physics,
                vertical_chain(),
                LIMITS,
            )
            .unwrap();

        // Then: each event cites the one before it by its real trace ID. This
        // is the dependency `commit_batch` cannot express: with only external
        // causes, infiltration could cite the forcing event's *previous* state
        // and never the water it actually consumed.
        assert_eq!(store.len(), 3);
        let forcing = resolved[&dag_key(1, 100, b"cell", 0)];
        let infiltration = resolved[&dag_key(2, 101, b"cell", 0)];
        let percolation = resolved[&dag_key(3, 102, b"cell", 0)];
        assert_eq!(store.event(forcing).unwrap().unwrap().causes(), &[]);
        assert_eq!(
            store.event(infiltration).unwrap().unwrap().causes(),
            &[forcing]
        );
        assert_eq!(
            store.event(percolation).unwrap().unwrap().causes(),
            &[infiltration]
        );
        assert_eq!(
            store
                .children(forcing, HistoryQuery::new(0, 4, 64).unwrap())
                .unwrap()
                .records(),
            &[infiltration]
        );
        assert!(forcing < infiltration && infiltration < percolation);
    }

    #[test]
    fn commit_order_is_the_canonical_one_regardless_of_insertion_order() {
        // Given: the same batch, handed over in every possible order.
        let mut baseline = CausalTraceStore::new();
        baseline
            .commit_dag_batch(
                SimulationTime::new(4),
                Phase::Physics,
                vertical_chain(),
                LIMITS,
            )
            .unwrap();

        // When/Then: every permutation produces a byte-identical store. The
        // ready set is ordered by complete key bytes, so producer order cannot
        // reach the trace IDs.
        let chain = vertical_chain();
        for permutation in [
            [0, 1, 2],
            [0, 2, 1],
            [1, 0, 2],
            [1, 2, 0],
            [2, 0, 1],
            [2, 1, 0],
        ] {
            let mut store = CausalTraceStore::new();
            store
                .commit_dag_batch(
                    SimulationTime::new(4),
                    Phase::Physics,
                    permutation.iter().map(|&i| chain[i].clone()).collect(),
                    LIMITS,
                )
                .unwrap();
            assert_eq!(store, baseline, "permutation {permutation:?}");
        }
    }

    #[test]
    fn independent_events_commit_in_lexicographic_key_order() {
        // Given: four independent proposals whose keys differ in each field of
        // the encoding, offered in reverse order.
        let keys = [
            dag_key(1, 1, b"aa", 0),
            dag_key(1, 1, b"aa", 1),
            dag_key(1, 1, b"ab", 0),
            dag_key(1, 2, b"aa", 0),
            dag_key(2, 0, b"aa", 0),
        ];
        let mut store = CausalTraceStore::new();
        let resolved = store
            .commit_dag_batch(
                SimulationTime::new(1),
                Phase::Physics,
                keys.iter()
                    .rev()
                    .enumerate()
                    .map(|(index, key)| dag_proposal(key.clone(), vec![], index as u64 + 1))
                    .collect(),
                LIMITS,
            )
            .unwrap();

        // Then: trace IDs ascend with key bytes — substage, then process kind,
        // then carrier, then local ordinal.
        let traces = keys.iter().map(|key| resolved[key]).collect::<Vec<_>>();
        let mut sorted = traces.clone();
        sorted.sort_unstable();
        assert_eq!(traces, sorted);
    }

    #[test]
    fn a_local_cause_from_a_later_substage_is_rejected() {
        // Substages are frozen: each reads what the previous produced. A
        // backwards edge would mean a substage read its own future.
        let early = dag_key(2, 0, b"c", 0);
        let late = dag_key(5, 0, b"c", 0);
        let mut store = CausalTraceStore::new();
        let result = store.commit_dag_batch(
            SimulationTime::new(1),
            Phase::Physics,
            vec![
                dag_proposal(
                    early.clone(),
                    vec![CausalEventDagCause::Local(late.clone())],
                    1,
                ),
                dag_proposal(late, vec![], 2),
            ],
            LIMITS,
        );
        assert!(matches!(
            result,
            Err(CausalDagCommitError::SubstageOrderViolated { .. })
        ));
        assert!(store.is_empty());
    }

    #[test]
    fn a_cycle_within_one_substage_is_rejected() {
        // Equal substage ordinals are legal — one substage is an arbitrary DAG
        // — so the ordinal rule cannot catch this and Kahn's algorithm must.
        let a = dag_key(3, 0, b"a", 0);
        let b = dag_key(3, 0, b"b", 0);
        let mut store = CausalTraceStore::new();
        let result = store.commit_dag_batch(
            SimulationTime::new(1),
            Phase::Physics,
            vec![
                dag_proposal(a.clone(), vec![CausalEventDagCause::Local(b.clone())], 1),
                dag_proposal(b, vec![CausalEventDagCause::Local(a)], 2),
            ],
            LIMITS,
        );
        assert!(matches!(
            result,
            Err(CausalDagCommitError::CyclicBatch { .. })
        ));
        assert!(store.is_empty());
    }

    #[test]
    fn a_self_referential_proposal_is_rejected() {
        let key = dag_key(3, 0, b"a", 0);
        let mut store = CausalTraceStore::new();
        let result = store.commit_dag_batch(
            SimulationTime::new(1),
            Phase::Physics,
            vec![dag_proposal(
                key.clone(),
                vec![CausalEventDagCause::Local(key)],
                1,
            )],
            LIMITS,
        );
        assert!(matches!(
            result,
            Err(CausalDagCommitError::CyclicBatch { .. })
        ));
        assert!(store.is_empty());
    }

    #[test]
    fn every_rejection_leaves_the_store_byte_identical() {
        // Given: a store with real history in it, so "unchanged" is a claim
        // about preserving something rather than about staying empty.
        let mut store = CausalTraceStore::new();
        let root = store
            .commit_batch(
                SimulationTime::new(1),
                Phase::Physics,
                vec![proposal(0, vec![], 1)],
            )
            .unwrap()[0];
        let before = store.clone();

        let key = dag_key(1, 0, b"k", 0);
        let other = dag_key(1, 0, b"k", 1);
        let rejections: Vec<Vec<CausalEventDagProposal>> = vec![
            // duplicate key
            vec![
                dag_proposal(key.clone(), vec![], 1),
                dag_proposal(key.clone(), vec![], 2),
            ],
            // unknown external cause
            vec![dag_proposal(
                key.clone(),
                vec![CausalEventDagCause::Existing(TraceId::new(999))],
                1,
            )],
            // unknown local cause
            vec![dag_proposal(
                key.clone(),
                vec![CausalEventDagCause::Local(other.clone())],
                1,
            )],
            // cycle
            vec![
                dag_proposal(
                    key.clone(),
                    vec![CausalEventDagCause::Local(other.clone())],
                    1,
                ),
                dag_proposal(
                    other.clone(),
                    vec![CausalEventDagCause::Local(key.clone())],
                    2,
                ),
            ],
            // cause cap
            vec![
                CausalEventDagProposal::new(
                    key.clone(),
                    EventKindId::new(36),
                    (0..17)
                        .map(|_| CausalEventDagCause::Existing(root))
                        .enumerate()
                        .map(|(index, _)| {
                            CausalEventDagCause::Local(dag_key(0, 0, b"c", index as u32))
                        })
                        .collect(),
                    vec![effect(1, 1, 2)],
                )
                .unwrap(),
            ],
            // effect cap
            vec![
                CausalEventDagProposal::new(
                    key,
                    EventKindId::new(36),
                    vec![],
                    (0..9).map(|index| effect(index, 1, 2)).collect(),
                )
                .unwrap(),
            ],
        ];

        for (index, batch) in rejections.into_iter().enumerate() {
            let result =
                store.commit_dag_batch(SimulationTime::new(2), Phase::Physics, batch, LIMITS);
            assert!(result.is_err(), "rejection case {index} must fail");
            assert_eq!(
                store, before,
                "rejection case {index} must not alter the store"
            );
        }

        // And: a valid batch after all of that still commits, so the failures
        // left no partial state behind that a later commit would trip over.
        assert!(
            store
                .commit_dag_batch(
                    SimulationTime::new(2),
                    Phase::Physics,
                    vec![dag_proposal(
                        dag_key(1, 0, b"ok", 0),
                        vec![CausalEventDagCause::Existing(root)],
                        7,
                    )],
                    LIMITS,
                )
                .is_ok()
        );
        assert_eq!(store.len(), 2);
    }

    #[test]
    fn an_empty_dag_batch_commits_nothing_and_succeeds() {
        let mut store = CausalTraceStore::new();
        assert!(
            store
                .commit_dag_batch(SimulationTime::new(1), Phase::Physics, vec![], LIMITS)
                .unwrap()
                .is_empty()
        );
        assert!(store.is_empty());
    }

    #[test]
    fn a_dag_batch_interleaves_external_and_local_causes() {
        // Given: a committed root the batch will also cite.
        let mut store = CausalTraceStore::new();
        let root = store
            .commit_batch(
                SimulationTime::new(1),
                Phase::Physics,
                vec![proposal(0, vec![], 1)],
            )
            .unwrap()[0];

        // When: a later event cites both the root and a sibling.
        let first = dag_key(1, 0, b"a", 0);
        let resolved = store
            .commit_dag_batch(
                SimulationTime::new(2),
                Phase::Physics,
                vec![
                    dag_proposal(first.clone(), vec![CausalEventDagCause::Existing(root)], 5),
                    dag_proposal(
                        dag_key(2, 0, b"a", 0),
                        vec![
                            CausalEventDagCause::Existing(root),
                            CausalEventDagCause::Local(first.clone()),
                        ],
                        6,
                    ),
                ],
                LIMITS,
            )
            .unwrap();

        // Then: the committed cause list is strictly ordered, mixing both.
        let second = resolved[&dag_key(2, 0, b"a", 0)];
        assert_eq!(
            store.event(second).unwrap().unwrap().causes(),
            &[root, resolved[&first]]
        );
    }

    #[test]
    fn dag_commits_do_not_disturb_legacy_commit_batch_identifiers() {
        // The whole point of adding a second commit path is that the first one
        // keeps issuing exactly the identifiers it always did.
        let mut legacy_only = CausalTraceStore::new();
        legacy_only
            .commit_batch(
                SimulationTime::new(1),
                Phase::Physics,
                vec![proposal(0, vec![], 1), proposal(1, vec![], 2)],
            )
            .unwrap();

        let mut mixed = CausalTraceStore::new();
        let legacy = mixed
            .commit_batch(
                SimulationTime::new(1),
                Phase::Physics,
                vec![proposal(0, vec![], 1), proposal(1, vec![], 2)],
            )
            .unwrap();
        assert_eq!(legacy, vec![TraceId::new(0), TraceId::new(1)]);
        assert_eq!(
            legacy_only
                .iter_hot()
                .map(|event| event.trace_id)
                .collect::<Vec<_>>(),
            mixed
                .iter_hot()
                .map(|event| event.trace_id)
                .collect::<Vec<_>>()
        );

        mixed
            .commit_dag_batch(
                SimulationTime::new(2),
                Phase::Physics,
                vertical_chain(),
                LIMITS,
            )
            .unwrap();
        // And a legacy commit after a DAG commit continues from where the DAG
        // batch left off rather than colliding with it.
        let after = mixed
            .commit_batch(
                SimulationTime::new(3),
                Phase::Physics,
                vec![proposal(0, vec![], 9)],
            )
            .unwrap();
        assert_eq!(after, vec![TraceId::new(5)]);
        assert_eq!(mixed.len(), 6);
    }

    #[test]
    fn a_dag_committed_store_reimports_from_its_own_snapshot() {
        // A DAG batch's local causes are ordinary forward edges once resolved,
        // so the existing import validation has to accept them unchanged.
        let mut store = CausalTraceStore::new();
        store
            .commit_dag_batch(
                SimulationTime::new(4),
                Phase::Physics,
                vertical_chain(),
                LIMITS,
            )
            .unwrap();
        let reimported = CausalTraceStore::import_snapshot(store.export_snapshot()).unwrap();
        assert_eq!(reimported, store);
    }

    #[test]
    fn proposal_keys_encode_exactly_and_read_back_their_fields() {
        let key = dag_key(7, 0x0102_0304, b"carrier", 0x0a0b_0c0d);
        assert_eq!(key.bytes()[0], CAUSAL_EVENT_PROPOSAL_KEY_VERSION);
        assert_eq!(key.substage_ordinal(), 7);
        assert_eq!(key.process_kind(), 0x0102_0304);
        assert_eq!(key.carrier_key(), b"carrier");
        assert_eq!(key.local_ordinal(), 0x0a0b_0c0d);
        assert_eq!(key.bytes().len(), 1 + 1 + 4 + 2 + 7 + 4);
        assert_eq!(
            &key.bytes()[2..8],
            &[0x01, 0x02, 0x03, 0x04, 0x00, 0x07],
            "process kind then carrier length, both big-endian"
        );

        // A carrier that does not fit its 16-bit length field is refused rather
        // than truncated into a key that names something else.
        assert_eq!(
            CausalEventProposalKey::new(0, 0, &vec![0_u8; 65_536], 0),
            Err(CausalEventProposalError::CarrierKeyTooLong { length: 65_536 })
        );
    }

    #[test]
    fn default_store_has_valid_flat_offsets() {
        let mut store = CausalTraceStore::default();
        store
            .commit_batch(
                SimulationTime::new(0),
                Phase::Physics,
                vec![proposal(0, vec![], 1)],
            )
            .unwrap();
        assert_eq!(
            store
                .event(TraceId::new(0))
                .unwrap()
                .unwrap()
                .effects()
                .len(),
            1
        );
    }

    #[test]
    fn transaction_rollback_restores_trace_frontier_and_child_index() {
        // Given: an existing causal graph whose child index already has edges.
        let mut store = CausalTraceStore::new();
        let roots = store
            .commit_batch(
                SimulationTime::new(1),
                Phase::Physics,
                vec![proposal(0, vec![], 100), proposal(1, vec![], 101)],
            )
            .unwrap();
        let parent = store
            .commit_batch(
                SimulationTime::new(2),
                Phase::Action,
                vec![proposal(2, vec![roots[0]], 102)],
            )
            .unwrap()[0];
        let before = store.clone();

        // When: a valid legacy append and a same-transaction local-cause DAG append,
        // each followed by a failed transaction, are rolled back.
        store.begin_transaction();
        let staged = store
            .commit_batch(
                SimulationTime::new(3),
                Phase::Action,
                vec![proposal(3, vec![parent], 103)],
            )
            .unwrap()[0];
        let dag = store
            .commit_dag_batch_for_transaction(
                SimulationTime::new(3),
                Phase::Physics,
                vertical_chain(),
                LIMITS,
            )
            .unwrap();
        let forcing = dag[&dag_key(1, 100, b"cell", 0)];
        assert_eq!(
            store
                .children(parent, HistoryQuery::new(0, 8, 128).unwrap())
                .unwrap()
                .records(),
            &[staged]
        );
        assert_eq!(
            store
                .children(forcing, HistoryQuery::new(0, 8, 128).unwrap())
                .unwrap()
                .records(),
            &[dag[&dag_key(2, 101, b"cell", 0)]]
        );
        assert!(store.event(staged).unwrap().is_some());
        store.rollback_transaction();

        // Then: every hot vector, frontier counter, and reverse edge is exact,
        // and the rejected IDs are reused by the next accepted append.
        assert_eq!(store, before);
        assert!(store.event(staged).unwrap().is_none());
        assert_eq!(
            store
                .children(parent, HistoryQuery::new(0, 8, 128).unwrap())
                .unwrap()
                .records(),
            &[]
        );
        let retry = store
            .commit_batch(
                SimulationTime::new(3),
                Phase::Action,
                vec![proposal(3, vec![parent], 103)],
            )
            .unwrap();
        assert_eq!(retry, vec![staged]);
        println!(
            "MARKER:trace_rollback len={} next_trace={} next_event={} hot_bytes={} child_parent={:?} retry_trace={}",
            store.len(),
            retry[0].raw() + 1,
            retry[0].raw() + 1,
            store.hot_bytes(),
            store
                .children(parent, HistoryQuery::new(0, 8, 128).unwrap())
                .unwrap()
                .records(),
            retry[0].raw()
        );
    }

    #[test]
    fn active_transaction_defers_hot_eviction_until_commit() {
        // Given: a hot budget that fits one record but no cold reader.
        let mut sizing = CausalTraceStore::new();
        sizing
            .commit_batch(
                SimulationTime::new(1),
                Phase::Physics,
                vec![proposal(0, vec![], 200)],
            )
            .unwrap();
        let budget = sizing.hot_bytes();
        let mut store = CausalTraceStore::with_hot_budget(budget);

        // When: two valid records exceed the budget inside one transaction and
        // the staged frame requests trimming.
        store.begin_transaction();
        store
            .commit_batch(
                SimulationTime::new(1),
                Phase::Physics,
                vec![proposal(0, vec![], 200), proposal(1, vec![], 201)],
            )
            .unwrap();
        let frame = store
            .iter_hot()
            .map(|event| {
                OwnedCausalRecord::new(
                    event.commit_sequence,
                    event.event_id,
                    event.trace_id,
                    event.time,
                    event.phase,
                    event.kind,
                    event.causes.to_vec(),
                    event.effects.to_vec(),
                )
                .map(HistoryRecord::Causal)
                .unwrap()
            })
            .collect::<Vec<_>>();
        store.trim_staged_frame_to_hot_budget(&frame).unwrap();

        // Then: active state remains fully readable, commit applies the deferred
        // eviction, and rollback would still have an untouched prefix to restore.
        assert_eq!(store.len(), 2);
        assert!(store.hot_bytes() > store.hot_budget());
        assert!(store.event(TraceId::new(0)).unwrap().is_some());
        store.end_transaction().unwrap();
        assert_eq!(store.len(), 1);
        assert!(store.hot_bytes() <= store.hot_budget());
        assert!(store.event(TraceId::new(0)).unwrap().is_none());
        assert!(store.event(TraceId::new(1)).unwrap().is_some());
        println!(
            "MARKER:trace_deferred_eviction before_records={} after_records={} hot_bytes={} budget={} retained_trace=1",
            frame.len(),
            store.len(),
            store.hot_bytes(),
            store.hot_budget()
        );
    }
}
