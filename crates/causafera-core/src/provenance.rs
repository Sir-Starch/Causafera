use std::collections::BTreeMap;

use causafera_types::{
    EventId, EventKindId, SimulationTime, StateObjectKindId, StatePropertyId, TraceId,
};
use thiserror::Error;

use crate::Phase;

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
}

/// Borrowed view of one committed event in the structure-of-arrays store.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CausalEventRef<'a> {
    pub event_id: EventId,
    pub trace_id: TraceId,
    pub time: SimulationTime,
    pub phase: Phase,
    pub kind: EventKindId,
    pub causes: &'a [TraceId],
    pub effects: &'a [CausalEffect],
}

/// Canonical append-only event graph.
///
/// Hot event fields and forward edges are flat. Reverse edges are a cold,
/// deterministic side index used for direct descendant traversal.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CausalTraceStore {
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
}

impl CausalTraceStore {
    pub fn new() -> Self {
        Self {
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
        }
    }

    pub fn len(&self) -> usize {
        self.trace_ids.len()
    }

    pub fn is_empty(&self) -> bool {
        self.trace_ids.is_empty()
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
                if self.trace_index(cause).is_none() {
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

        let mut committed = Vec::with_capacity(proposals.len());
        for proposal in proposals {
            let event_id = EventId::new(self.next_event_id);
            let trace_id = TraceId::new(self.next_trace_id);
            self.next_event_id += 1;
            self.next_trace_id += 1;

            self.event_ids.push(event_id);
            self.trace_ids.push(trace_id);
            self.times.push(time);
            self.phases.push(phase);
            self.kinds.push(proposal.kind());
            self.causes.extend_from_slice(proposal.causes());
            self.cause_offsets.push(self.causes.len() as u32);
            self.effects.extend_from_slice(proposal.effects());
            self.effect_offsets.push(self.effects.len() as u32);
            for &cause in proposal.causes() {
                self.children.entry(cause).or_default().push(trace_id);
            }
            committed.push(trace_id);
        }
        Ok(committed)
    }

    pub fn event(&self, trace_id: TraceId) -> Option<CausalEventRef<'_>> {
        self.trace_index(trace_id).map(|index| self.event_at(index))
    }

    pub fn event_by_id(&self, event_id: EventId) -> Option<CausalEventRef<'_>> {
        self.event_ids
            .binary_search(&event_id)
            .ok()
            .map(|index| self.event_at(index))
    }

    pub fn children(&self, trace_id: TraceId) -> &[TraceId] {
        self.children.get(&trace_id).map_or(&[], Vec::as_slice)
    }

    pub fn iter(&self) -> impl ExactSizeIterator<Item = CausalEventRef<'_>> + '_ {
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
            event_id: self.event_ids[index],
            trace_id: self.trace_ids[index],
            time: self.times[index],
            phase: self.phases[index],
            kind: self.kinds[index],
            causes: &self.causes[cause_start..cause_end],
            effects: &self.effects[effect_start..effect_end],
        }
    }
}

impl Default for CausalTraceStore {
    fn default() -> Self {
        Self::new()
    }
}

/// Logical snapshot of a `CausalTraceStore` for persistence.
///
/// Contains all forward-edge state needed to reconstruct the store
/// deterministically. Reverse child indexes are rebuilt after import.
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
    #[error("cause refers to unknown trace ID: {0}")]
    UnknownCause(TraceId),
}

impl CausalTraceStore {
    /// Export a logical snapshot of the entire trace store.
    pub fn export_snapshot(&self) -> CausalTraceSnapshot {
        let events = self
            .iter()
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

    /// Reconstruct a trace store from a logical snapshot.
    ///
    /// Validates monotonic IDs, parent-before-child ordering, and
    /// cause existence. Rebuilds the child index deterministically.
    pub fn import_snapshot(snapshot: CausalTraceSnapshot) -> Result<Self, CausalSnapshotError> {
        let event_count = snapshot.events.len();
        if event_count > usize::MAX / 2 {
            return Err(CausalSnapshotError::EventCountExceeded);
        }

        let mut store = Self::new();
        store.next_event_id = snapshot.next_event_id;
        store.next_trace_id = snapshot.next_trace_id;

        let mut last_trace_id: Option<TraceId> = None;
        let mut last_event_id: Option<EventId> = None;
        for event in &snapshot.events {
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
        Ok(store)
    }
}

#[derive(Clone, Copy, Debug, Error, PartialEq, Eq)]
pub enum CausalCommitError {
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
            a.event(TraceId::new(0)).unwrap().effects[0]
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

        assert_eq!(store.event(child).unwrap().causes, roots);
        assert_eq!(store.children(TraceId::new(0)), &[child]);
        assert_eq!(store.children(TraceId::new(1)), &[child]);
        assert!(store.children(child).is_empty());
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
        assert_eq!(store.event(TraceId::new(0)).unwrap().effects.len(), 1);
    }
}
