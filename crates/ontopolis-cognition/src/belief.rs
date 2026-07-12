use ontopolis_types::{
    BeliefId, CausalHypothesisId, ConceptId, EvidenceId, PerceptId, SimulationTime,
    SubjectivePatternId, SubjectiveSourceId,
};
use thiserror::Error;

use crate::{COGNITIVE_WEIGHT_SCALE, CognitiveWeight};

pub const MAX_BELIEFS: usize = 32;
pub const MAX_EVIDENCE_BATCH: usize = 32;
pub const MAX_SOURCES: usize = 32;
pub const MAX_CAUSAL_HYPOTHESES: usize = 32;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Belief {
    pub id: BeliefId,
    pub subject: ConceptId,
    pub confidence: CognitiveWeight,
    pub inertia: CognitiveWeight,
    pub cumulative_support: i64,
    pub evidence_count: u32,
    pub last_evidence: EvidenceId,
    pub supporting_percept: PerceptId,
    pub revised_at: SimulationTime,
}

/// Signed direction is generic: positive supports the hypothesis, negative contradicts it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BeliefEvidence {
    pub id: EvidenceId,
    pub belief: BeliefId,
    pub direction: i32,
    pub strength: CognitiveWeight,
    pub salience: CognitiveWeight,
    pub source: SubjectiveSourceId,
    pub supporting_percept: PerceptId,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SourceTrust {
    pub source: SubjectiveSourceId,
    pub trust: CognitiveWeight,
    pub observations: u32,
    pub last_updated: SimulationTime,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TrustStore {
    sources: [SourceTrust; MAX_SOURCES],
    len: u8,
    default_trust: CognitiveWeight,
}

impl TrustStore {
    pub const fn new(default_trust: CognitiveWeight) -> Self {
        Self {
            sources: [SourceTrust {
                source: SubjectiveSourceId::new(0),
                trust: CognitiveWeight::ZERO,
                observations: 0,
                last_updated: SimulationTime::new(0),
            }; MAX_SOURCES],
            len: 0,
            default_trust,
        }
    }

    pub fn sources(&self) -> &[SourceTrust] {
        &self.sources[..self.len as usize]
    }

    pub fn trust(&self, source: SubjectiveSourceId) -> CognitiveWeight {
        self.sources()
            .iter()
            .find(|value| value.source == source)
            .map_or(self.default_trust, |value| value.trust)
    }

    pub fn observe(
        &mut self,
        time: SimulationTime,
        source: SubjectiveSourceId,
        correspondence: CognitiveWeight,
    ) -> Result<(), BeliefError> {
        if let Some(value) = self.sources[..self.len as usize]
            .iter_mut()
            .find(|value| value.source == source)
        {
            if time < value.last_updated {
                return Err(BeliefError::TimeRegression);
            }
            let total = u64::from(value.trust.raw())
                .saturating_mul(u64::from(value.observations))
                .saturating_add(u64::from(correspondence.raw()));
            value.observations = value.observations.saturating_add(1);
            value.trust = weight((total / u64::from(value.observations)) as u32);
            value.last_updated = time;
            return Ok(());
        }
        if self.len as usize == MAX_SOURCES {
            return Err(BeliefError::SourceCapacity);
        }
        self.sources[self.len as usize] = SourceTrust {
            source,
            trust: correspondence,
            observations: 1,
            last_updated: time,
        };
        self.len += 1;
        self.sources[..self.len as usize].sort_by_key(|value| value.source);
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BeliefStore {
    beliefs: [Belief; MAX_BELIEFS],
    len: u8,
    next_id: u64,
    last_update: Option<SimulationTime>,
}

impl BeliefStore {
    pub const fn new() -> Self {
        Self {
            beliefs: [Belief {
                id: BeliefId::new(0),
                subject: ConceptId::new(0),
                confidence: CognitiveWeight::ZERO,
                inertia: CognitiveWeight::ZERO,
                cumulative_support: 0,
                evidence_count: 0,
                last_evidence: EvidenceId::new(0),
                supporting_percept: PerceptId::new(0),
                revised_at: SimulationTime::new(0),
            }; MAX_BELIEFS],
            len: 0,
            next_id: 1,
            last_update: None,
        }
    }

    pub fn beliefs(&self) -> &[Belief] {
        &self.beliefs[..self.len as usize]
    }

    pub fn form(
        &mut self,
        time: SimulationTime,
        subject: ConceptId,
        confidence: CognitiveWeight,
        inertia: CognitiveWeight,
        supporting_percept: PerceptId,
    ) -> Result<BeliefId, BeliefError> {
        if self.last_update.is_some_and(|last| time < last) {
            return Err(BeliefError::TimeRegression);
        }
        if self.len as usize == MAX_BELIEFS {
            return Err(BeliefError::BeliefCapacity);
        }
        if self
            .beliefs()
            .iter()
            .any(|belief| belief.subject == subject)
        {
            return Err(BeliefError::DuplicateSubject);
        }
        let id = BeliefId::new(self.next_id);
        self.next_id = self
            .next_id
            .checked_add(1)
            .ok_or(BeliefError::IdentifierExhausted)?;
        self.beliefs[self.len as usize] = Belief {
            id,
            subject,
            confidence,
            inertia,
            cumulative_support: i64::from(confidence.raw()),
            evidence_count: 0,
            last_evidence: EvidenceId::new(0),
            supporting_percept,
            revised_at: time,
        };
        self.len += 1;
        self.beliefs[..self.len as usize].sort_by_key(|belief| belief.id);
        self.last_update = Some(time);
        Ok(id)
    }

    pub fn revise(
        &mut self,
        time: SimulationTime,
        evidence: &[BeliefEvidence],
        trust: &TrustStore,
    ) -> Result<(), BeliefError> {
        if self.last_update.is_some_and(|last| time < last) {
            return Err(BeliefError::TimeRegression);
        }
        if evidence.len() > MAX_EVIDENCE_BATCH {
            return Err(BeliefError::EvidenceCapacity);
        }
        let mut evidence = evidence.to_vec();
        evidence.sort_by_key(|value| value.id);
        if evidence.windows(2).any(|pair| pair[0].id == pair[1].id) {
            return Err(BeliefError::DuplicateEvidence);
        }
        for item in evidence {
            let belief = self.beliefs[..self.len as usize]
                .iter_mut()
                .find(|belief| belief.id == item.belief)
                .ok_or(BeliefError::UnknownBelief { id: item.belief })?;
            let magnitude = u64::from(item.strength.raw())
                .saturating_mul(u64::from(item.salience.raw()))
                / u64::from(COGNITIVE_WEIGHT_SCALE);
            let trusted = magnitude.saturating_mul(u64::from(trust.trust(item.source).raw()))
                / u64::from(COGNITIVE_WEIGHT_SCALE);
            let signed = i64::from(item.direction.signum()).saturating_mul(trusted as i64);
            belief.cumulative_support = belief.cumulative_support.saturating_add(signed);
            let inertia_gate = i64::from(belief.inertia.raw());
            let effective = if belief.cumulative_support >= 0 {
                belief.cumulative_support.saturating_sub(inertia_gate / 2)
            } else {
                belief.cumulative_support.saturating_add(inertia_gate)
            };
            let centered = i64::from(belief.confidence.raw()).saturating_add(effective / 4);
            belief.confidence = weight(centered.clamp(0, i64::from(COGNITIVE_WEIGHT_SCALE)) as u32);
            belief.evidence_count = belief.evidence_count.saturating_add(1);
            belief.last_evidence = item.id;
            belief.supporting_percept = item.supporting_percept;
            belief.revised_at = time;
        }
        self.last_update = Some(time);
        Ok(())
    }
}

impl Default for BeliefStore {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CausalObservation {
    pub cause: SubjectivePatternId,
    pub effect: SubjectivePatternId,
    pub proximity: CognitiveWeight,
    pub prediction_error: CognitiveWeight,
    pub supporting_percept: PerceptId,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CausalHypothesis {
    pub id: CausalHypothesisId,
    pub cause: SubjectivePatternId,
    pub effect: SubjectivePatternId,
    pub confidence: CognitiveWeight,
    pub observations: u32,
    pub supporting_percept: PerceptId,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CausalHypothesisStore {
    hypotheses: [CausalHypothesis; MAX_CAUSAL_HYPOTHESES],
    len: u8,
    next_id: u64,
}

impl CausalHypothesisStore {
    pub const fn new() -> Self {
        Self {
            hypotheses: [CausalHypothesis {
                id: CausalHypothesisId::new(0),
                cause: SubjectivePatternId::new(0),
                effect: SubjectivePatternId::new(0),
                confidence: CognitiveWeight::ZERO,
                observations: 0,
                supporting_percept: PerceptId::new(0),
            }; MAX_CAUSAL_HYPOTHESES],
            len: 0,
            next_id: 1,
        }
    }

    pub fn hypotheses(&self) -> &[CausalHypothesis] {
        &self.hypotheses[..self.len as usize]
    }

    pub fn observe(&mut self, observation: CausalObservation) -> Result<(), BeliefError> {
        let evidence = weight(
            (u64::from(observation.proximity.raw())
                .saturating_mul(u64::from(observation.prediction_error.raw()))
                / u64::from(COGNITIVE_WEIGHT_SCALE)) as u32,
        );
        if let Some(value) = self.hypotheses[..self.len as usize]
            .iter_mut()
            .find(|value| value.cause == observation.cause && value.effect == observation.effect)
        {
            let total = u64::from(value.confidence.raw())
                .saturating_mul(u64::from(value.observations))
                .saturating_add(u64::from(evidence.raw()));
            value.observations = value.observations.saturating_add(1);
            value.confidence = weight((total / u64::from(value.observations)) as u32);
            value.supporting_percept = observation.supporting_percept;
            return Ok(());
        }
        if self.len as usize == MAX_CAUSAL_HYPOTHESES {
            return Err(BeliefError::CausalCapacity);
        }
        let id = CausalHypothesisId::new(self.next_id);
        self.next_id = self
            .next_id
            .checked_add(1)
            .ok_or(BeliefError::IdentifierExhausted)?;
        self.hypotheses[self.len as usize] = CausalHypothesis {
            id,
            cause: observation.cause,
            effect: observation.effect,
            confidence: evidence,
            observations: 1,
            supporting_percept: observation.supporting_percept,
        };
        self.len += 1;
        self.hypotheses[..self.len as usize].sort_by_key(|value| (value.cause, value.effect));
        Ok(())
    }
}

impl Default for CausalHypothesisStore {
    fn default() -> Self {
        Self::new()
    }
}

fn weight(value: u32) -> CognitiveWeight {
    CognitiveWeight::new(value.min(COGNITIVE_WEIGHT_SCALE)).expect("weight is clamped")
}

#[derive(Clone, Copy, Debug, Error, PartialEq, Eq)]
pub enum BeliefError {
    #[error("belief update time regressed")]
    TimeRegression,
    #[error("belief capacity exceeded")]
    BeliefCapacity,
    #[error("one belief per subjective concept is allowed in the minimal store")]
    DuplicateSubject,
    #[error("evidence batch capacity exceeded")]
    EvidenceCapacity,
    #[error("evidence identifiers must be unique")]
    DuplicateEvidence,
    #[error("unknown belief {id}")]
    UnknownBelief { id: BeliefId },
    #[error("subjective source capacity exceeded")]
    SourceCapacity,
    #[error("causal hypothesis capacity exceeded")]
    CausalCapacity,
    #[error("subjective cognition identifier space is exhausted")]
    IdentifierExhausted,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn w(value: u32) -> CognitiveWeight {
        CognitiveWeight::new(value).unwrap()
    }

    #[test]
    fn inertia_preserves_a_stable_mistake_against_weak_contradiction() {
        let mut beliefs = BeliefStore::new();
        let id = beliefs
            .form(
                SimulationTime::new(1),
                ConceptId::new(4),
                w(800_000),
                w(900_000),
                PerceptId::new(1),
            )
            .unwrap();
        let trust = TrustStore::new(w(500_000));
        beliefs
            .revise(
                SimulationTime::new(2),
                &[BeliefEvidence {
                    id: EvidenceId::new(1),
                    belief: id,
                    direction: -1,
                    strength: w(100_000),
                    salience: w(500_000),
                    source: SubjectiveSourceId::new(8),
                    supporting_percept: PerceptId::new(2),
                }],
                &trust,
            )
            .unwrap();
        assert!(beliefs.beliefs()[0].confidence.raw() > 700_000);
    }

    #[test]
    fn trusted_evidence_has_more_effect_than_untrusted_evidence() {
        let mut trust = TrustStore::new(w(100_000));
        trust
            .observe(
                SimulationTime::new(1),
                SubjectiveSourceId::new(1),
                w(900_000),
            )
            .unwrap();
        assert!(trust.trust(SubjectiveSourceId::new(1)) > trust.trust(SubjectiveSourceId::new(2)));
    }

    #[test]
    fn causal_hypotheses_are_directional_and_subjective() {
        let mut store = CausalHypothesisStore::new();
        store
            .observe(CausalObservation {
                cause: SubjectivePatternId::new(1),
                effect: SubjectivePatternId::new(2),
                proximity: w(800_000),
                prediction_error: w(500_000),
                supporting_percept: PerceptId::new(3),
            })
            .unwrap();
        assert_eq!(store.hypotheses()[0].confidence, w(400_000));
        assert_eq!(store.hypotheses()[0].cause, SubjectivePatternId::new(1));
    }
}
