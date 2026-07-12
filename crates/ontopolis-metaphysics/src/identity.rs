use ontopolis_core::StateFingerprint;
use ontopolis_types::{
    ContinuityCandidateId, ContinuityCriterionSchemaId, ContinuityEvidenceChannelId,
    ContinuityObservationId, IdentityExperimentId, SimulationTime, TraceId,
};
use thiserror::Error;

pub const CONTINUITY_SCALE: i64 = 1_000;
pub const MAX_CONTINUITY_OBSERVATIONS: usize = 256;
pub const MAX_CONTINUITY_CRITERIA: usize = 32;
pub const MAX_CONTINUITY_CHANNELS: usize = 64;

/// Objective evidence relevant to a continuity question.
///
/// The opaque channel identifies a registered measurement contract. It is not
/// a semantic declaration that biological, psychological, social, or other
/// continuity is the correct definition of a person.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ContinuityObservation {
    pub id: ContinuityObservationId,
    pub channel: ContinuityEvidenceChannelId,
    pub observed_at: SimulationTime,
    pub earlier_state: StateFingerprint,
    pub later_state: StateFingerprint,
    pub strength: i64,
    pub trace: TraceId,
}

impl ContinuityObservation {
    pub fn new(
        id: ContinuityObservationId,
        channel: ContinuityEvidenceChannelId,
        observed_at: SimulationTime,
        earlier_state: StateFingerprint,
        later_state: StateFingerprint,
        strength: i64,
        trace: TraceId,
    ) -> Result<Self, IdentityResearchError> {
        if !(-CONTINUITY_SCALE..=CONTINUITY_SCALE).contains(&strength) {
            return Err(IdentityResearchError::InvalidStrength { strength });
        }
        Ok(Self {
            id,
            channel,
            observed_at,
            earlier_state,
            later_state,
            strength,
            trace,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ContinuityChannelWeight {
    pub channel: ContinuityEvidenceChannelId,
    pub weight: i64,
}

impl ContinuityChannelWeight {
    pub fn new(
        channel: ContinuityEvidenceChannelId,
        weight: i64,
    ) -> Result<Self, IdentityResearchError> {
        if weight == 0 || !(-CONTINUITY_SCALE..=CONTINUITY_SCALE).contains(&weight) {
            return Err(IdentityResearchError::InvalidWeight { weight });
        }
        Ok(Self { channel, weight })
    }
}

/// One explicitly supplied research criterion over opaque evidence channels.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContinuityCriterion {
    schema: ContinuityCriterionSchemaId,
    weights: Vec<ContinuityChannelWeight>,
}

impl ContinuityCriterion {
    pub fn new(
        schema: ContinuityCriterionSchemaId,
        mut weights: Vec<ContinuityChannelWeight>,
    ) -> Result<Self, IdentityResearchError> {
        if weights.is_empty() || weights.len() > MAX_CONTINUITY_CHANNELS {
            return Err(IdentityResearchError::InvalidChannelCount {
                count: weights.len(),
            });
        }
        weights.sort_unstable();
        if weights
            .windows(2)
            .any(|pair| pair[0].channel == pair[1].channel)
        {
            return Err(IdentityResearchError::DuplicateChannel);
        }
        Ok(Self { schema, weights })
    }

    pub const fn schema(&self) -> ContinuityCriterionSchemaId {
        self.schema
    }
}

/// Bounded evidence set for one candidate continuity relation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IdentityContinuityExperiment {
    id: IdentityExperimentId,
    candidate: ContinuityCandidateId,
    observations: Vec<ContinuityObservation>,
}

impl IdentityContinuityExperiment {
    pub fn new(
        id: IdentityExperimentId,
        candidate: ContinuityCandidateId,
        mut observations: Vec<ContinuityObservation>,
    ) -> Result<Self, IdentityResearchError> {
        if observations.is_empty() || observations.len() > MAX_CONTINUITY_OBSERVATIONS {
            return Err(IdentityResearchError::InvalidObservationCount {
                count: observations.len(),
            });
        }
        observations.sort_unstable();
        if observations.windows(2).any(|pair| pair[0].id == pair[1].id) {
            return Err(IdentityResearchError::DuplicateObservation);
        }
        Ok(Self {
            id,
            candidate,
            observations,
        })
    }

    pub fn evaluate(
        &self,
        criteria: &[ContinuityCriterion],
    ) -> Result<ContinuityAssessment, IdentityResearchError> {
        if criteria.is_empty() || criteria.len() > MAX_CONTINUITY_CRITERIA {
            return Err(IdentityResearchError::InvalidCriterionCount {
                count: criteria.len(),
            });
        }
        let mut ordered = criteria.to_vec();
        ordered.sort_unstable_by_key(ContinuityCriterion::schema);
        if ordered
            .windows(2)
            .any(|pair| pair[0].schema == pair[1].schema)
        {
            return Err(IdentityResearchError::DuplicateCriterion);
        }

        let scores = ordered
            .iter()
            .map(|criterion| {
                let weighted = self
                    .observations
                    .iter()
                    .filter_map(|observation| {
                        criterion
                            .weights
                            .binary_search_by_key(&observation.channel, |entry| entry.channel)
                            .ok()
                            .map(|index| {
                                observation
                                    .strength
                                    .saturating_mul(criterion.weights[index].weight)
                                    / CONTINUITY_SCALE
                            })
                    })
                    .fold(0_i64, i64::saturating_add)
                    .clamp(-CONTINUITY_SCALE, CONTINUITY_SCALE);
                CriterionScore {
                    schema: criterion.schema,
                    score: weighted,
                }
            })
            .collect();
        let mut supporting_traces = self
            .observations
            .iter()
            .map(|observation| observation.trace)
            .collect::<Vec<_>>();
        supporting_traces.sort_unstable();
        supporting_traces.dedup();
        Ok(ContinuityAssessment {
            experiment: self.id,
            candidate: self.candidate,
            scores,
            supporting_traces,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CriterionScore {
    pub schema: ContinuityCriterionSchemaId,
    pub score: i64,
}

/// Numeric evidence under supplied criteria, never an authoritative identity verdict.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContinuityAssessment {
    pub experiment: IdentityExperimentId,
    pub candidate: ContinuityCandidateId,
    pub scores: Vec<CriterionScore>,
    pub supporting_traces: Vec<TraceId>,
}

#[derive(Clone, Copy, Debug, Error, PartialEq, Eq)]
pub enum IdentityResearchError {
    #[error("continuity strength is outside the fixed-point range: {strength}")]
    InvalidStrength { strength: i64 },
    #[error("continuity weight is zero or outside the fixed-point range: {weight}")]
    InvalidWeight { weight: i64 },
    #[error("invalid continuity channel count: {count}")]
    InvalidChannelCount { count: usize },
    #[error("duplicate continuity evidence channel")]
    DuplicateChannel,
    #[error("invalid continuity observation count: {count}")]
    InvalidObservationCount { count: usize },
    #[error("duplicate continuity observation")]
    DuplicateObservation,
    #[error("invalid continuity criterion count: {count}")]
    InvalidCriterionCount { count: usize },
    #[error("duplicate continuity criterion")]
    DuplicateCriterion,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fingerprint(value: u8) -> StateFingerprint {
        StateFingerprint::new([value; 32])
    }

    #[test]
    fn competing_criteria_remain_distinct_numeric_hypotheses() {
        let observation = ContinuityObservation::new(
            ContinuityObservationId::new(1),
            ContinuityEvidenceChannelId::new(7),
            SimulationTime::new(10),
            fingerprint(1),
            fingerprint(2),
            800,
            TraceId::new(4),
        )
        .unwrap();
        let experiment = IdentityContinuityExperiment::new(
            IdentityExperimentId::new(1),
            ContinuityCandidateId::new(9),
            vec![observation],
        )
        .unwrap();
        let positive = ContinuityCriterion::new(
            ContinuityCriterionSchemaId::new(1),
            vec![ContinuityChannelWeight::new(ContinuityEvidenceChannelId::new(7), 1_000).unwrap()],
        )
        .unwrap();
        let negative = ContinuityCriterion::new(
            ContinuityCriterionSchemaId::new(2),
            vec![
                ContinuityChannelWeight::new(ContinuityEvidenceChannelId::new(7), -1_000).unwrap(),
            ],
        )
        .unwrap();
        let assessment = experiment.evaluate(&[negative, positive]).unwrap();
        assert_eq!(assessment.scores[0].score, 800);
        assert_eq!(assessment.scores[1].score, -800);
        assert_eq!(assessment.supporting_traces, vec![TraceId::new(4)]);
    }
}
