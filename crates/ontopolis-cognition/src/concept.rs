use ontopolis_types::{ConceptId, PerceptId, SimulationTime};
use thiserror::Error;

use crate::{AppearanceSignature, COGNITIVE_WEIGHT_SCALE, CognitiveWeight};

pub const MAX_CONCEPTS: usize = 32;
pub const MAX_CONCEPT_OBSERVATIONS: usize = 32;
pub const MAX_ACTIVE_CONCEPTS: usize = 8;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ConceptObservation {
    pub signature: AppearanceSignature,
    pub salience: CognitiveWeight,
    pub predictive_utility: CognitiveWeight,
    pub supporting_percept: PerceptId,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SubjectiveConcept {
    pub id: ConceptId,
    pub prototype: AppearanceSignature,
    pub confidence: CognitiveWeight,
    pub predictive_utility: CognitiveWeight,
    pub recent_activation: CognitiveWeight,
    pub exemplars: u32,
    pub supporting_percept: PerceptId,
    pub last_activated: SimulationTime,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConceptStore {
    concepts: [SubjectiveConcept; MAX_CONCEPTS],
    len: u8,
    next_id: u64,
    similarity_tolerance: u32,
    formation_threshold: CognitiveWeight,
    activation_decay_per_tick: u32,
    last_update: Option<SimulationTime>,
}

impl ConceptStore {
    pub fn new(
        similarity_tolerance: u32,
        formation_threshold: CognitiveWeight,
        activation_decay_per_tick: u32,
    ) -> Self {
        Self {
            concepts: [SubjectiveConcept::default(); MAX_CONCEPTS],
            len: 0,
            next_id: 1,
            similarity_tolerance,
            formation_threshold,
            activation_decay_per_tick,
            last_update: None,
        }
    }

    pub fn concepts(&self) -> &[SubjectiveConcept] {
        &self.concepts[..self.len as usize]
    }

    pub fn active_concepts(&self) -> Vec<SubjectiveConcept> {
        let mut active = self.concepts().to_vec();
        active.sort_by(|a, b| {
            b.recent_activation
                .cmp(&a.recent_activation)
                .then_with(|| a.id.cmp(&b.id))
        });
        active.truncate(MAX_ACTIVE_CONCEPTS);
        active.retain(|concept| concept.recent_activation > CognitiveWeight::ZERO);
        active
    }

    pub fn observe(
        &mut self,
        time: SimulationTime,
        observations: &[ConceptObservation],
    ) -> Result<(), ConceptError> {
        if self.last_update.is_some_and(|last| time < last) {
            return Err(ConceptError::TimeRegression);
        }
        if observations.len() > MAX_CONCEPT_OBSERVATIONS {
            return Err(ConceptError::TooManyObservations);
        }
        let elapsed = self
            .last_update
            .map_or(0, |last| time.raw().saturating_sub(last.raw()));
        let decay = u64::from(self.activation_decay_per_tick)
            .saturating_mul(elapsed)
            .min(u64::from(u32::MAX)) as u32;
        for concept in &mut self.concepts[..self.len as usize] {
            concept.recent_activation =
                weight(concept.recent_activation.raw().saturating_sub(decay));
        }

        let mut observations = observations.to_vec();
        observations.sort_by_key(|value| value.supporting_percept);
        if observations
            .windows(2)
            .any(|pair| pair[0].supporting_percept == pair[1].supporting_percept)
        {
            return Err(ConceptError::DuplicatePercept);
        }
        for observation in observations {
            if let Some(index) = self.best_match(observation.signature) {
                self.revise(index, observation, time);
            } else if observation.salience >= self.formation_threshold {
                self.allocate(observation, time)?;
            }
        }
        self.last_update = Some(time);
        Ok(())
    }

    fn best_match(&self, signature: AppearanceSignature) -> Option<usize> {
        self.concepts()
            .iter()
            .enumerate()
            .filter_map(|(index, concept)| {
                let distance = signature_distance(signature, concept.prototype);
                (distance <= self.similarity_tolerance).then_some((distance, concept.id, index))
            })
            .min()
            .map(|(_, _, index)| index)
    }

    fn revise(&mut self, index: usize, observation: ConceptObservation, time: SimulationTime) {
        let concept = &mut self.concepts[index];
        let old_count = u64::from(concept.exemplars);
        let new_count = concept.exemplars.saturating_add(1);
        for (prototype, observed) in concept.prototype.0.iter_mut().zip(observation.signature.0) {
            let total = u64::from(*prototype)
                .saturating_mul(old_count)
                .saturating_add(u64::from(observed));
            *prototype = (total / u64::from(new_count)) as u16;
        }
        concept.exemplars = new_count;
        concept.confidence = weight(
            concept
                .confidence
                .raw()
                .saturating_add(observation.salience.raw() / new_count.max(1)),
        );
        concept.predictive_utility = running_mean(
            concept.predictive_utility,
            observation.predictive_utility,
            old_count,
            new_count,
        );
        concept.recent_activation = observation.salience;
        concept.supporting_percept = observation.supporting_percept;
        concept.last_activated = time;
    }

    fn allocate(
        &mut self,
        observation: ConceptObservation,
        time: SimulationTime,
    ) -> Result<(), ConceptError> {
        if self.len as usize == MAX_CONCEPTS {
            return Err(ConceptError::Capacity);
        }
        let id = self.next_id;
        self.next_id = self
            .next_id
            .checked_add(1)
            .ok_or(ConceptError::IdentifierExhausted)?;
        self.concepts[self.len as usize] = SubjectiveConcept {
            id: ConceptId::new(id),
            prototype: observation.signature,
            confidence: observation.salience,
            predictive_utility: observation.predictive_utility,
            recent_activation: observation.salience,
            exemplars: 1,
            supporting_percept: observation.supporting_percept,
            last_activated: time,
        };
        self.len += 1;
        Ok(())
    }
}

fn signature_distance(left: AppearanceSignature, right: AppearanceSignature) -> u32 {
    left.0
        .into_iter()
        .zip(right.0)
        .map(|(a, b)| u32::from(a.abs_diff(b)))
        .sum()
}

fn running_mean(
    previous: CognitiveWeight,
    observed: CognitiveWeight,
    old_count: u64,
    new_count: u32,
) -> CognitiveWeight {
    let total = u64::from(previous.raw())
        .saturating_mul(old_count)
        .saturating_add(u64::from(observed.raw()));
    weight((total / u64::from(new_count)) as u32)
}

fn weight(value: u32) -> CognitiveWeight {
    CognitiveWeight::new(value.min(COGNITIVE_WEIGHT_SCALE)).expect("weight is clamped")
}

#[derive(Clone, Copy, Debug, Error, PartialEq, Eq)]
pub enum ConceptError {
    #[error("concept update time regressed")]
    TimeRegression,
    #[error("concept observation capacity exceeded")]
    TooManyObservations,
    #[error("concept observations require unique supporting percepts")]
    DuplicatePercept,
    #[error("concept capacity exceeded")]
    Capacity,
    #[error("concept identifier space is exhausted")]
    IdentifierExhausted,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn w(value: u32) -> CognitiveWeight {
        CognitiveWeight::new(value).unwrap()
    }

    fn observation(percept: u64, value: u16) -> ConceptObservation {
        ConceptObservation {
            signature: AppearanceSignature([value; 4]),
            salience: w(800_000),
            predictive_utility: w(500_000),
            supporting_percept: PerceptId::new(percept),
        }
    }

    #[test]
    fn similar_attended_observations_revise_one_subjective_prototype() {
        let mut store = ConceptStore::new(40, w(400_000), 10);
        store
            .observe(SimulationTime::new(1), &[observation(1, 10)])
            .unwrap();
        store
            .observe(SimulationTime::new(2), &[observation(2, 14)])
            .unwrap();
        assert_eq!(store.concepts().len(), 1);
        assert_eq!(store.concepts()[0].prototype, AppearanceSignature([12; 4]));
        assert_eq!(store.concepts()[0].exemplars, 2);
    }

    #[test]
    fn observation_order_is_canonicalized() {
        let observations = [observation(2, 100), observation(1, 10)];
        let mut a = ConceptStore::new(10, w(1), 0);
        let mut b = ConceptStore::new(10, w(1), 0);
        a.observe(SimulationTime::new(1), &observations).unwrap();
        b.observe(SimulationTime::new(1), &[observations[1], observations[0]])
            .unwrap();
        assert_eq!(a, b);
        assert_eq!(a.concepts().len(), 2);
    }

    #[test]
    fn low_salience_does_not_allocate_a_concept() {
        let mut store = ConceptStore::new(10, w(900_000), 0);
        store
            .observe(SimulationTime::new(1), &[observation(1, 10)])
            .unwrap();
        assert!(store.concepts().is_empty());
    }
}
