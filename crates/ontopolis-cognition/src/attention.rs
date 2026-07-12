use ontopolis_types::{AttentionTargetId, PerceptId, SimulationTime};
use thiserror::Error;

pub const ATTENTION_WEIGHT_SCALE: u32 = 1_000_000;
pub const MAX_ATTENTION_FOCI: usize = 8;
pub const MAX_ATTENTION_CANDIDATES: usize = 64;

/// Fixed-point attention weight in parts per million.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AttentionWeight(u32);

impl AttentionWeight {
    pub fn new(parts_per_million: u32) -> Result<Self, AttentionConfigError> {
        if parts_per_million > ATTENTION_WEIGHT_SCALE {
            return Err(AttentionConfigError::WeightOutOfRange {
                value: parts_per_million,
            });
        }
        Ok(Self(parts_per_million))
    }

    pub const fn raw(self) -> u32 {
        self.0
    }
}

/// Bounded attention configuration.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AttentionConfig {
    capacity: u8,
    salience_threshold: AttentionWeight,
    continuity_bonus: AttentionWeight,
}

impl AttentionConfig {
    pub fn new(
        capacity: u8,
        salience_threshold: AttentionWeight,
        continuity_bonus: AttentionWeight,
    ) -> Result<Self, AttentionConfigError> {
        if capacity == 0 || usize::from(capacity) > MAX_ATTENTION_FOCI {
            return Err(AttentionConfigError::CapacityOutOfRange { capacity });
        }
        Ok(Self {
            capacity,
            salience_threshold,
            continuity_bonus,
        })
    }

    pub const fn capacity(self) -> u8 {
        self.capacity
    }
}

#[derive(Clone, Copy, Debug, Error, PartialEq, Eq)]
pub enum AttentionConfigError {
    #[error("attention weight {value} exceeds {ATTENTION_WEIGHT_SCALE}")]
    WeightOutOfRange { value: u32 },
    #[error("attention capacity {capacity} must be between 1 and {MAX_ATTENTION_FOCI}")]
    CapacityOutOfRange { capacity: u8 },
}

/// Subjective attention candidate.
///
/// `AttentionTargetId` is an agent-local subjective identity. It is not an
/// `EntityId`, `FeatureId`, or other Ground Truth identity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AttentionCandidate {
    target: AttentionTargetId,
    salience: AttentionWeight,
    supporting_percept: PerceptId,
}

impl AttentionCandidate {
    pub const fn new(
        target: AttentionTargetId,
        salience: AttentionWeight,
        supporting_percept: PerceptId,
    ) -> Self {
        Self {
            target,
            salience,
            supporting_percept,
        }
    }

    pub const fn target(self) -> AttentionTargetId {
        self.target
    }

    pub const fn salience(self) -> AttentionWeight {
        self.salience
    }

    pub const fn supporting_percept(self) -> PerceptId {
        self.supporting_percept
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AttentionFocus {
    pub target: AttentionTargetId,
    pub active_since: SimulationTime,
    pub supporting_percept: PerceptId,
}

/// Fixed-capacity active attention state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AttentionState {
    config: AttentionConfig,
    targets: [AttentionTargetId; MAX_ATTENTION_FOCI],
    active_since: [SimulationTime; MAX_ATTENTION_FOCI],
    supporting_percepts: [PerceptId; MAX_ATTENTION_FOCI],
    len: u8,
    last_update: Option<SimulationTime>,
}

impl AttentionState {
    pub const fn new(config: AttentionConfig) -> Self {
        Self {
            config,
            targets: [AttentionTargetId::new(0); MAX_ATTENTION_FOCI],
            active_since: [SimulationTime::new(0); MAX_ATTENTION_FOCI],
            supporting_percepts: [PerceptId::new(0); MAX_ATTENTION_FOCI],
            len: 0,
            last_update: None,
        }
    }

    pub const fn config(&self) -> AttentionConfig {
        self.config
    }

    pub const fn len(&self) -> usize {
        self.len as usize
    }

    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn focus(&self, index: usize) -> Option<AttentionFocus> {
        (index < self.len()).then(|| AttentionFocus {
            target: self.targets[index],
            active_since: self.active_since[index],
            supporting_percept: self.supporting_percepts[index],
        })
    }

    /// Re-rank bounded candidates deterministically.
    ///
    /// Existing foci receive only a numeric continuity bonus. Equal scores are
    /// resolved by subjective target ID, never input order or a hash traversal.
    pub fn update(
        &mut self,
        time: SimulationTime,
        candidates: &[AttentionCandidate],
    ) -> Result<(), AttentionUpdateError> {
        if self.last_update.is_some_and(|last| time < last) {
            return Err(AttentionUpdateError::TimeRegression);
        }
        if candidates.len() > MAX_ATTENTION_CANDIDATES {
            return Err(AttentionUpdateError::TooManyCandidates {
                count: candidates.len(),
            });
        }

        let mut ranked = candidates.to_vec();
        ranked.sort_by_key(|candidate| candidate.target());
        for index in 1..ranked.len() {
            if ranked[index - 1].target() == ranked[index].target() {
                return Err(AttentionUpdateError::DuplicateTarget {
                    target: ranked[index].target(),
                });
            }
        }
        ranked
            .retain(|candidate| candidate.salience().raw() >= self.config.salience_threshold.raw());
        ranked.sort_by(|a, b| {
            self.effective_weight(*b)
                .cmp(&self.effective_weight(*a))
                .then_with(|| a.target().cmp(&b.target()))
        });
        ranked.truncate(usize::from(self.config.capacity));

        let old_targets = self.targets;
        let old_since = self.active_since;
        let old_len = self.len();
        self.len = ranked.len() as u8;
        for (index, candidate) in ranked.into_iter().enumerate() {
            self.targets[index] = candidate.target();
            self.active_since[index] = old_targets[..old_len]
                .iter()
                .position(|target| *target == candidate.target())
                .map_or(time, |old_index| old_since[old_index]);
            self.supporting_percepts[index] = candidate.supporting_percept();
        }
        self.last_update = Some(time);
        Ok(())
    }

    fn effective_weight(&self, candidate: AttentionCandidate) -> u32 {
        let continuity = if self.targets[..self.len()].contains(&candidate.target()) {
            self.config.continuity_bonus.raw()
        } else {
            0
        };
        candidate.salience().raw().saturating_add(continuity)
    }
}

#[derive(Clone, Copy, Debug, Error, PartialEq, Eq)]
pub enum AttentionUpdateError {
    #[error("attention update time regressed")]
    TimeRegression,
    #[error("attention candidate count {count} exceeds {MAX_ATTENTION_CANDIDATES}")]
    TooManyCandidates { count: usize },
    #[error("attention target {target} appears more than once")]
    DuplicateTarget { target: AttentionTargetId },
}

#[cfg(test)]
mod tests {
    use super::*;

    fn weight(raw: u32) -> AttentionWeight {
        AttentionWeight::new(raw).unwrap()
    }

    fn candidate(target: u64, salience: u32, trace: u64) -> AttentionCandidate {
        AttentionCandidate::new(
            AttentionTargetId::new(target),
            weight(salience),
            PerceptId::new(trace),
        )
    }

    fn state(capacity: u8, continuity: u32) -> AttentionState {
        AttentionState::new(
            AttentionConfig::new(capacity, weight(100), weight(continuity)).unwrap(),
        )
    }

    #[test]
    fn configuration_enforces_fixed_bounds() {
        assert_eq!(
            AttentionWeight::new(ATTENTION_WEIGHT_SCALE + 1),
            Err(AttentionConfigError::WeightOutOfRange {
                value: ATTENTION_WEIGHT_SCALE + 1
            })
        );
        assert!(matches!(
            AttentionConfig::new(0, weight(0), weight(0)),
            Err(AttentionConfigError::CapacityOutOfRange { .. })
        ));
    }

    #[test]
    fn ranking_is_bounded_and_input_order_independent() {
        let candidates = [
            candidate(9, 400, 9),
            candidate(3, 800, 3),
            candidate(5, 600, 5),
        ];
        let mut a = state(2, 0);
        let mut b = state(2, 0);
        a.update(SimulationTime::new(4), &candidates).unwrap();
        b.update(
            SimulationTime::new(4),
            &candidates.into_iter().rev().collect::<Vec<_>>(),
        )
        .unwrap();
        assert_eq!(a, b);
        assert_eq!(a.len(), 2);
        assert_eq!(a.focus(0).unwrap().target, AttentionTargetId::new(3));
        assert_eq!(a.focus(1).unwrap().target, AttentionTargetId::new(5));
    }

    #[test]
    fn continuity_bonus_preserves_focus_without_authoritative_identity() {
        let mut state = state(1, 300);
        state
            .update(SimulationTime::new(1), &[candidate(4, 500, 1)])
            .unwrap();
        state
            .update(
                SimulationTime::new(2),
                &[candidate(4, 450, 2), candidate(7, 600, 3)],
            )
            .unwrap();
        let focus = state.focus(0).unwrap();
        assert_eq!(focus.target, AttentionTargetId::new(4));
        assert_eq!(focus.active_since, SimulationTime::new(1));
        assert_eq!(focus.supporting_percept, PerceptId::new(2));
    }

    #[test]
    fn duplicate_subjective_targets_are_rejected() {
        let mut state = state(1, 0);
        assert_eq!(
            state.update(
                SimulationTime::new(1),
                &[candidate(2, 500, 1), candidate(2, 600, 2)],
            ),
            Err(AttentionUpdateError::DuplicateTarget {
                target: AttentionTargetId::new(2)
            })
        );
    }
}
