use causafera_types::{PerceivedObjectId, PerceptId, SimulationTime};
use thiserror::Error;

use crate::{CognitiveWeight, PredictionError, SubjectiveScene};

pub const MAX_TEMPORAL_FRAMES: usize = 8;
pub const MAX_FRAME_OBJECTS: usize = 16;
pub const MAX_FRAME_ERRORS: usize = 8;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SubjectiveFrame {
    pub time: SimulationTime,
    object_ids: [PerceivedObjectId; MAX_FRAME_OBJECTS],
    object_len: u8,
    supporting_percepts: [PerceptId; MAX_FRAME_OBJECTS],
    prediction_error: CognitiveWeight,
}

impl SubjectiveFrame {
    pub fn object_ids(&self) -> &[PerceivedObjectId] {
        &self.object_ids[..self.object_len as usize]
    }

    pub fn supporting_percepts(&self) -> &[PerceptId] {
        &self.supporting_percepts[..self.object_len as usize]
    }

    pub const fn prediction_error(&self) -> CognitiveWeight {
        self.prediction_error
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TemporalEnvelope {
    frames: [SubjectiveFrame; MAX_TEMPORAL_FRAMES],
    len: u8,
}

impl TemporalEnvelope {
    pub const fn new() -> Self {
        Self {
            frames: [SubjectiveFrame {
                time: SimulationTime::new(0),
                object_ids: [PerceivedObjectId::new(0); MAX_FRAME_OBJECTS],
                object_len: 0,
                supporting_percepts: [PerceptId::new(0); MAX_FRAME_OBJECTS],
                prediction_error: CognitiveWeight::ZERO,
            }; MAX_TEMPORAL_FRAMES],
            len: 0,
        }
    }

    pub fn frames(&self) -> &[SubjectiveFrame] {
        &self.frames[..self.len as usize]
    }

    pub fn advance(
        &mut self,
        scene: &SubjectiveScene,
        errors: &[PredictionError],
    ) -> Result<(), ContinuityError> {
        if errors.len() > MAX_FRAME_ERRORS {
            return Err(ContinuityError::TooManyPredictionErrors);
        }
        if self
            .frames()
            .last()
            .is_some_and(|last| scene.time() <= last.time)
        {
            return Err(ContinuityError::NonIncreasingTime);
        }
        let mut frame = SubjectiveFrame {
            time: scene.time(),
            ..SubjectiveFrame::default()
        };
        frame.object_len = scene.objects().len() as u8;
        for (index, object) in scene.objects().iter().enumerate() {
            frame.object_ids[index] = object.id;
            frame.supporting_percepts[index] = object.supporting_percept;
        }
        let max_error = errors
            .iter()
            .map(|error| error.magnitude)
            .max()
            .unwrap_or(CognitiveWeight::ZERO);
        frame.prediction_error = max_error;
        if self.len as usize == MAX_TEMPORAL_FRAMES {
            self.frames.copy_within(1..MAX_TEMPORAL_FRAMES, 0);
            self.frames[MAX_TEMPORAL_FRAMES - 1] = frame;
        } else {
            self.frames[self.len as usize] = frame;
            self.len += 1;
        }
        Ok(())
    }
}

impl Default for TemporalEnvelope {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Copy, Debug, Error, PartialEq, Eq)]
pub enum ContinuityError {
    #[error("temporal frames require strictly increasing time")]
    NonIncreasingTime,
    #[error("prediction error count exceeds temporal-frame capacity")]
    TooManyPredictionErrors,
}

#[cfg(test)]
mod tests {
    use crate::{
        AppearanceSignature, AttentionCandidate, AttentionConfig, AttentionState, AttentionWeight,
        BodySchema, PerceptualCue, SceneContinuityState, SelfModel,
    };
    use causafera_types::{AttentionTargetId, PerceptId};

    use super::*;

    fn scene(time: u64) -> SubjectiveScene {
        let cue = PerceptualCue {
            percept: PerceptId::new(time),
            attention_target: AttentionTargetId::new(1),
            appearance: AppearanceSignature([1; 4]),
            relative_position: [0; 3],
            strength: CognitiveWeight::new(500_000).unwrap(),
            time: SimulationTime::new(time),
        };
        let mut attention = AttentionState::new(
            AttentionConfig::new(
                1,
                AttentionWeight::new(0).unwrap(),
                AttentionWeight::new(0).unwrap(),
            )
            .unwrap(),
        );
        attention
            .update(
                SimulationTime::new(time),
                &[AttentionCandidate::new(
                    cue.attention_target,
                    AttentionWeight::new(500_000).unwrap(),
                    cue.percept,
                )],
            )
            .unwrap();
        SceneContinuityState::new(4, 4)
            .reconstruct(
                SimulationTime::new(time),
                &[cue],
                &attention,
                &BodySchema::default(),
                &SelfModel::default(),
            )
            .unwrap()
    }

    #[test]
    fn temporal_envelope_evicts_oldest_frame_at_fixed_capacity() {
        let mut envelope = TemporalEnvelope::new();
        for time in 1..=10 {
            envelope.advance(&scene(time), &[]).unwrap();
        }
        assert_eq!(envelope.frames().len(), MAX_TEMPORAL_FRAMES);
        assert_eq!(envelope.frames()[0].time, SimulationTime::new(3));
        assert_eq!(
            envelope.frames().last().unwrap().time,
            SimulationTime::new(10)
        );
    }
}
