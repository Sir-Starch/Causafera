use causafera_types::{
    ActionPatternId, OutcomePatternId, PerceptId, PredictionId, SimulationTime, WorkingItemId,
};
use thiserror::Error;

use crate::{AppearanceSignature, CognitiveWeight, MAX_SCENE_CUES, PerceptualCue};

pub const MAX_PREDICTIONS: usize = 8;
pub const MAX_PREDICTION_ERRORS: usize = 8;
pub const MAX_AGENCY_ASSOCIATIONS: usize = 16;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NearFuturePrediction {
    pub id: PredictionId,
    pub expected: AppearanceSignature,
    pub due: SimulationTime,
    pub confidence: CognitiveWeight,
    pub basis: WorkingItemId,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PredictionError {
    pub prediction: PredictionId,
    pub magnitude: CognitiveWeight,
    pub observed_percept: Option<PerceptId>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PredictiveState {
    predictions: [NearFuturePrediction; MAX_PREDICTIONS],
    prediction_len: u8,
    errors: [PredictionError; MAX_PREDICTION_ERRORS],
    error_len: u8,
    last_update: Option<SimulationTime>,
}

impl PredictiveState {
    pub const fn new() -> Self {
        Self {
            predictions: [NearFuturePrediction {
                id: PredictionId::new(0),
                expected: AppearanceSignature([0; 4]),
                due: SimulationTime::new(0),
                confidence: CognitiveWeight::ZERO,
                basis: WorkingItemId::new(0),
            }; MAX_PREDICTIONS],
            prediction_len: 0,
            errors: [PredictionError {
                prediction: PredictionId::new(0),
                magnitude: CognitiveWeight::ZERO,
                observed_percept: None,
            }; MAX_PREDICTION_ERRORS],
            error_len: 0,
            last_update: None,
        }
    }

    pub fn predictions(&self) -> &[NearFuturePrediction] {
        &self.predictions[..self.prediction_len as usize]
    }

    pub fn errors(&self) -> &[PredictionError] {
        &self.errors[..self.error_len as usize]
    }

    pub fn replace_predictions(
        &mut self,
        mut predictions: Vec<NearFuturePrediction>,
    ) -> Result<(), PredictionErrorKind> {
        if predictions.len() > MAX_PREDICTIONS {
            return Err(PredictionErrorKind::Capacity);
        }
        predictions.sort_by_key(|prediction| (prediction.due, prediction.id));
        if predictions.windows(2).any(|pair| pair[0].id == pair[1].id) {
            return Err(PredictionErrorKind::DuplicatePrediction);
        }
        self.prediction_len = predictions.len() as u8;
        self.predictions[..predictions.len()].copy_from_slice(&predictions);
        Ok(())
    }

    /// Resolve due expectations against identity-free observations.
    pub fn observe(
        &mut self,
        time: SimulationTime,
        cues: &[PerceptualCue],
    ) -> Result<(), PredictionErrorKind> {
        if self.last_update.is_some_and(|last| time < last) {
            return Err(PredictionErrorKind::TimeRegression);
        }
        if cues.len() > MAX_SCENE_CUES {
            return Err(PredictionErrorKind::TooManyCues);
        }
        let mut cues = cues.to_vec();
        cues.sort_by_key(|cue| cue.percept);
        if cues
            .windows(2)
            .any(|pair| pair[0].percept == pair[1].percept)
        {
            return Err(PredictionErrorKind::DuplicatePercept);
        }
        if cues.iter().any(|cue| cue.time != time) {
            return Err(PredictionErrorKind::CueTimeMismatch);
        }
        self.error_len = 0;
        let due: Vec<_> = self
            .predictions()
            .iter()
            .copied()
            .filter(|prediction| prediction.due <= time)
            .collect();
        for prediction in due.into_iter().take(MAX_PREDICTION_ERRORS) {
            let best = cues
                .iter()
                .map(|cue| {
                    (
                        signature_distance(prediction.expected, cue.appearance),
                        cue.percept,
                    )
                })
                .min();
            let (distance, observed_percept) = best.map_or((4_000, None), |(distance, percept)| {
                (distance, Some(percept))
            });
            let mismatch = distance.saturating_mul(250).min(1_000_000);
            let magnitude = u64::from(mismatch)
                .saturating_mul(u64::from(prediction.confidence.raw()))
                / 1_000_000;
            self.errors[self.error_len as usize] = PredictionError {
                prediction: prediction.id,
                magnitude: CognitiveWeight::new(magnitude as u32).expect("bounded error"),
                observed_percept,
            };
            self.error_len += 1;
        }
        let retained: Vec<_> = self
            .predictions()
            .iter()
            .copied()
            .filter(|prediction| prediction.due > time)
            .collect();
        self.prediction_len = retained.len() as u8;
        self.predictions[..retained.len()].copy_from_slice(&retained);
        self.last_update = Some(time);
        Ok(())
    }
}

impl Default for PredictiveState {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AgencyAssociation {
    pub action: ActionPatternId,
    pub outcome: OutcomePatternId,
    pub strength: CognitiveWeight,
    pub observations: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AgencyModel {
    associations: [AgencyAssociation; MAX_AGENCY_ASSOCIATIONS],
    len: u8,
}

impl AgencyModel {
    pub const fn new() -> Self {
        Self {
            associations: [AgencyAssociation {
                action: ActionPatternId::new(0),
                outcome: OutcomePatternId::new(0),
                strength: CognitiveWeight::ZERO,
                observations: 0,
            }; MAX_AGENCY_ASSOCIATIONS],
            len: 0,
        }
    }

    pub fn associations(&self) -> &[AgencyAssociation] {
        &self.associations[..self.len as usize]
    }

    pub fn observe(
        &mut self,
        action: ActionPatternId,
        outcome: OutcomePatternId,
        proximity: CognitiveWeight,
    ) {
        if let Some(value) = self.associations[..self.len as usize]
            .iter_mut()
            .find(|value| value.action == action && value.outcome == outcome)
        {
            let total = u64::from(value.strength.raw())
                .saturating_mul(u64::from(value.observations))
                .saturating_add(u64::from(proximity.raw()));
            value.observations = value.observations.saturating_add(1);
            value.strength = CognitiveWeight::new((total / u64::from(value.observations)) as u32)
                .expect("mean of bounded weights");
            return;
        }
        let index = if self.len as usize == MAX_AGENCY_ASSOCIATIONS {
            self.associations[..self.len as usize]
                .iter()
                .enumerate()
                .min_by_key(|(_, value)| (value.strength, value.action, value.outcome))
                .map(|(index, _)| index)
                .expect("agency capacity is non-zero")
        } else {
            let index = self.len as usize;
            self.len += 1;
            index
        };
        self.associations[index] = AgencyAssociation {
            action,
            outcome,
            strength: proximity,
            observations: 1,
        };
        self.associations[..self.len as usize].sort_by_key(|value| (value.action, value.outcome));
    }
}

impl Default for AgencyModel {
    fn default() -> Self {
        Self::new()
    }
}

fn signature_distance(a: AppearanceSignature, b: AppearanceSignature) -> u32 {
    a.0.into_iter()
        .zip(b.0)
        .map(|(left, right)| u32::from(left.abs_diff(right)))
        .sum()
}

#[derive(Clone, Copy, Debug, Error, PartialEq, Eq)]
pub enum PredictionErrorKind {
    #[error("prediction capacity exceeded")]
    Capacity,
    #[error("prediction identifiers must be unique")]
    DuplicatePrediction,
    #[error("prediction time regressed")]
    TimeRegression,
    #[error("prediction observation cue capacity exceeded")]
    TooManyCues,
    #[error("prediction observation percept identifiers must be unique")]
    DuplicatePercept,
    #[error("prediction observation cue time differs from update time")]
    CueTimeMismatch,
}

#[cfg(test)]
mod tests {
    use causafera_types::AttentionTargetId;

    use super::*;

    fn weight(value: u32) -> CognitiveWeight {
        CognitiveWeight::new(value).unwrap()
    }

    #[test]
    fn due_predictions_emit_bounded_numeric_error() {
        let mut state = PredictiveState::new();
        state
            .replace_predictions(vec![NearFuturePrediction {
                id: PredictionId::new(1),
                expected: AppearanceSignature([10; 4]),
                due: SimulationTime::new(2),
                confidence: weight(1_000_000),
                basis: WorkingItemId::new(2),
            }])
            .unwrap();
        state
            .observe(
                SimulationTime::new(2),
                &[PerceptualCue {
                    percept: PerceptId::new(4),
                    attention_target: AttentionTargetId::new(3),
                    appearance: AppearanceSignature([110; 4]),
                    relative_position: [0; 3],
                    strength: weight(1),
                    time: SimulationTime::new(2),
                }],
            )
            .unwrap();
        assert_eq!(state.errors()[0].magnitude, weight(100_000));
        assert!(state.predictions().is_empty());
    }

    #[test]
    fn agency_is_learned_as_a_bounded_running_association() {
        let mut model = AgencyModel::new();
        model.observe(
            ActionPatternId::new(1),
            OutcomePatternId::new(2),
            weight(800_000),
        );
        model.observe(
            ActionPatternId::new(1),
            OutcomePatternId::new(2),
            weight(400_000),
        );
        assert_eq!(model.associations()[0].strength, weight(600_000));
        assert_eq!(model.associations()[0].observations, 2);
    }
}
