use std::{cmp::Ordering, collections::BTreeSet};

use causafera_types::{EventId, EventKindId, SimulationTime, TraceId};

use crate::{CausalEffect, Phase};

use super::{DomainEvidenceRecord, HistoryError, MAX_CAUSES_PER_EVENT, MAX_EFFECTS_PER_EVENT};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct OwnedCausalRecord {
    commit_sequence: u64,
    event_id: EventId,
    trace_id: TraceId,
    time: SimulationTime,
    phase: Phase,
    kind: EventKindId,
    causes: Vec<TraceId>,
    effects: Vec<CausalEffect>,
}

impl OwnedCausalRecord {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        commit_sequence: u64,
        event_id: EventId,
        trace_id: TraceId,
        time: SimulationTime,
        phase: Phase,
        kind: EventKindId,
        causes: Vec<TraceId>,
        effects: Vec<CausalEffect>,
    ) -> Result<Self, HistoryError> {
        if causes.len() > MAX_CAUSES_PER_EVENT {
            return Err(HistoryError::CauseLimitExceeded {
                count: causes.len(),
                max: MAX_CAUSES_PER_EVENT,
            });
        }
        if effects.len() > MAX_EFFECTS_PER_EVENT {
            return Err(HistoryError::EffectLimitExceeded {
                count: effects.len(),
                max: MAX_EFFECTS_PER_EVENT,
            });
        }
        if effects.is_empty() {
            return Err(HistoryError::NoEffects);
        }
        validate_order(&causes)
            .map_err(|index| HistoryError::CausesNotStrictlyOrdered { index })?;
        validate_order_by(&effects, |effect| effect.target())
            .map_err(|index| HistoryError::EffectsNotStrictlyOrdered { index })?;
        Ok(Self {
            commit_sequence,
            event_id,
            trace_id,
            time,
            phase,
            kind,
            causes,
            effects,
        })
    }

    pub const fn commit_sequence(&self) -> u64 {
        self.commit_sequence
    }
    pub const fn event_id(&self) -> EventId {
        self.event_id
    }
    pub const fn trace_id(&self) -> TraceId {
        self.trace_id
    }
    pub const fn time(&self) -> SimulationTime {
        self.time
    }
    pub const fn phase(&self) -> Phase {
        self.phase
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

    pub fn canonical_field_size(&self) -> Result<usize, HistoryError> {
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

    /// Canonical logical fields in the exact order used by `write_trace_event`.
    pub fn canonical_field_bytes(&self) -> Result<Vec<u8>, HistoryError> {
        let capacity = self.canonical_field_size()?;
        let mut bytes = Vec::with_capacity(capacity);
        push_u64(&mut bytes, self.event_id.raw());
        push_u64(&mut bytes, self.trace_id.raw());
        push_u64(&mut bytes, self.time.raw());
        push_u64(&mut bytes, u64::from(self.phase.id().0));
        push_u64(&mut bytes, self.kind.raw());
        push_u64(&mut bytes, self.causes.len() as u64);
        for cause in &self.causes {
            push_u64(&mut bytes, cause.raw());
        }
        push_u64(&mut bytes, self.effects.len() as u64);
        for effect in &self.effects {
            push_u64(&mut bytes, effect.target().object_kind().raw());
            push_u64(&mut bytes, effect.target().object_id());
            push_u64(&mut bytes, effect.target().property().raw());
            bytes.extend_from_slice(&effect.before().bytes());
            bytes.extend_from_slice(&effect.after().bytes());
        }
        Ok(bytes)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HistoryRecord {
    Causal(OwnedCausalRecord),
    Evidence(DomainEvidenceRecord),
}

impl HistoryRecord {
    const fn commit_sequence(&self) -> u64 {
        match self {
            Self::Causal(record) => record.commit_sequence(),
            Self::Evidence(record) => record.commit_sequence(),
        }
    }
}

impl Ord for HistoryRecord {
    fn cmp(&self, other: &Self) -> Ordering {
        self.commit_sequence()
            .cmp(&other.commit_sequence())
            .then_with(|| match (self, other) {
                (Self::Causal(left), Self::Causal(right)) => left.event_id().cmp(&right.event_id()),
                (Self::Causal(_), Self::Evidence(_)) => Ordering::Less,
                (Self::Evidence(_), Self::Causal(_)) => Ordering::Greater,
                (Self::Evidence(left), Self::Evidence(right)) => left.cmp(right),
            })
    }
}

impl PartialOrd for HistoryRecord {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

pub fn canonical_history_records(
    mut records: Vec<HistoryRecord>,
) -> Result<Vec<HistoryRecord>, HistoryError> {
    records.sort();
    for pair in records.windows(2) {
        if pair[0].cmp(&pair[1]) == Ordering::Equal {
            return Err(HistoryError::DuplicateRecordKey);
        }
    }
    Ok(records)
}

pub fn validate_causal_batch(
    records: &[OwnedCausalRecord],
    prior: &[OwnedCausalRecord],
) -> Result<(), HistoryError> {
    let mut events = BTreeSet::new();
    let mut traces = BTreeSet::new();
    for record in prior.iter().chain(records) {
        if !events.insert(record.event_id()) {
            return Err(HistoryError::DuplicateEventId(record.event_id()));
        }
        if !traces.insert(record.trace_id()) {
            return Err(HistoryError::DuplicateTraceId(record.trace_id()));
        }
    }
    let mut known: BTreeSet<TraceId> = prior.iter().map(OwnedCausalRecord::trace_id).collect();
    for (index, record) in records.iter().enumerate() {
        if index > 0 && records[index - 1] >= *record {
            return Err(HistoryError::NonCanonicalOrder { index });
        }
        for parent in record.causes() {
            if !known.contains(parent) {
                return Err(HistoryError::UnknownParent {
                    trace_id: record.trace_id(),
                    parent: *parent,
                });
            }
        }
        known.insert(record.trace_id());
    }
    Ok(())
}

fn validate_order<T: Ord>(values: &[T]) -> Result<(), usize> {
    for index in 1..values.len() {
        if values[index - 1] >= values[index] {
            return Err(index);
        }
    }
    Ok(())
}
fn validate_order_by<T, K: Ord>(values: &[T], key: impl Fn(&T) -> K) -> Result<(), usize> {
    for index in 1..values.len() {
        if key(&values[index - 1]) >= key(&values[index]) {
            return Err(index);
        }
    }
    Ok(())
}
fn push_u64(bytes: &mut Vec<u8>, value: u64) {
    bytes.extend_from_slice(&value.to_le_bytes());
}
