use crate::*;
use causafera_core::*;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use causafera_cognition::{AttentionCandidate, AttentionWeight, PerceptualCue};
use causafera_types::{SimulationTime, Velocity};

use super::{
    ActionKindId, ActionProposal, ActiveGoal, ActorId, ActorState, PerceivedSelf, SubjectiveScene,
    SubjectiveTarget,
};

pub fn actor_cognition_step(
    time: SimulationTime,
    actors: &mut BTreeMap<ActorId, ActorState>,
    action_kind: ActionKindId,
) -> Result<usize, causafera_cognition::SceneUpdateError> {
    let mut object_total = 0;
    for actor in actors.values_mut() {
        let cues = actor
            .features
            .iter()
            .map(|feature| PerceptualCue {
                percept: feature.percept,
                attention_target: feature.attention_target,
                appearance: feature.appearance,
                relative_position: feature.relative_position,
                strength: feature.strength,
                time,
            })
            .collect::<Vec<_>>();
        let mut candidates = Vec::with_capacity(actor.features.len());
        for feature in &actor.features {
            let weight = AttentionWeight::new(feature.strength.raw())
                .map_err(|_| causafera_cognition::SceneUpdateError::TooManyCues { count: 0 })?;
            candidates.push(AttentionCandidate::new(
                feature.attention_target,
                weight,
                feature.percept,
            ));
        }
        actor
            .attention
            .update(time, &candidates)
            .map_err(attention_error_to_scene_error)?;
        let inner = actor.continuity.reconstruct(
            time,
            &cues,
            &actor.attention,
            &actor.body_schema,
            &actor.self_model,
        )?;
        let objects = inner.objects().to_vec();
        let selected_object = actor.attention.focus(0).and_then(|focus| {
            objects
                .iter()
                .find(|object| object.supporting_percept == focus.supporting_percept)
        });
        let target = selected_object.map_or(SubjectiveTarget::SelfBody, |object| {
            SubjectiveTarget::Object(object.id)
        });
        let perceived_urgency = selected_object.map_or(0, |object| {
            i64::from(object.confidence.raw()).saturating_div(400_000)
        });
        let action_urgency = if actor.body.energy > 0 {
            perceived_urgency.clamp(1, actor.body.energy)
        } else {
            0
        };
        let active_goals = if objects.is_empty() {
            Vec::new()
        } else {
            vec![ActiveGoal {
                action_kind,
                target,
                urgency: action_urgency,
            }]
        };
        actor.proposals = active_goals
            .iter()
            .map(|goal| ActionProposal {
                action_kind: goal.action_kind,
                target: goal.target,
                intensity: goal.urgency,
            })
            .collect();
        actor.subjective_scene = Some(SubjectiveScene {
            perceived_self: PerceivedSelf {
                energy_band: actor.body.energy.clamp(0, i64::from(u8::MAX)) as u8,
                motion_band: motion_band(actor.body.velocity),
            },
            objects,
            body_schema: actor.body_schema.clone(),
            active_goals,
            inner,
        });
        object_total += actor
            .subjective_scene
            .as_ref()
            .map_or(0, |scene| scene.objects.len());
    }
    Ok(object_total)
}

fn attention_error_to_scene_error(
    error: causafera_cognition::AttentionUpdateError,
) -> causafera_cognition::SceneUpdateError {
    match error {
        causafera_cognition::AttentionUpdateError::TimeRegression => {
            causafera_cognition::SceneUpdateError::TimeRegression
        }
        causafera_cognition::AttentionUpdateError::TooManyCandidates { count } => {
            causafera_cognition::SceneUpdateError::TooManyCues { count }
        }
        causafera_cognition::AttentionUpdateError::DuplicateTarget { .. } => {
            causafera_cognition::SceneUpdateError::DuplicatePercept
        }
    }
}

pub(crate) fn motion_band(velocity: Velocity) -> u8 {
    velocity.length_squared().sqrt().min(f64::from(u8::MAX)) as u8
}
pub(crate) struct ActorCognitionSystem {
    pub(crate) state: Arc<Mutex<RuntimeState>>,
    pub(crate) next_time: SimulationTime,
}

impl ActorCognitionSystem {
    pub(crate) fn new(state: Arc<Mutex<RuntimeState>>) -> Self {
        Self {
            state,
            next_time: SimulationTime::new(1),
        }
    }

    pub(crate) fn execute(&mut self) -> Result<(), RuntimeError> {
        let mut state = self.state.lock().map_err(|_| RuntimeError::StatePoisoned)?;
        let object_count = actor_cognition_step(
            self.next_time,
            &mut state.actors,
            ActionKindId::new(ACTOR_CONTACT_ACTION_KIND),
        )?;
        state.subjective_actor_objects = state
            .subjective_actor_objects
            .saturating_add(object_count as u64);
        self.next_time = self.next_time.tick();
        Ok(())
    }
}

impl System for ActorCognitionSystem {
    fn run(&mut self, _stream: &mut RandomStream) {
        if let Err(error) = self.execute()
            && let Ok(mut state) = self.state.lock() {
                state.failure.get_or_insert(error);
            }
    }

    fn restore_time(&mut self, time: SimulationTime) {
        self.next_time = time;
    }
}
