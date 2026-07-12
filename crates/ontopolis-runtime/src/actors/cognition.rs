use std::collections::BTreeMap;

use ontopolis_cognition::{AttentionCandidate, AttentionWeight, PerceptualCue};
use ontopolis_types::{SimulationTime, Velocity};

use super::{
    ActionKindId, ActionProposal, ActiveGoal, ActorId, ActorState, PerceivedSelf, SubjectiveScene,
    SubjectiveTarget,
};

pub fn actor_cognition_step(
    time: SimulationTime,
    actors: &mut BTreeMap<ActorId, ActorState>,
    action_kind: ActionKindId,
) -> Result<usize, ontopolis_cognition::SceneUpdateError> {
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
                .map_err(|_| ontopolis_cognition::SceneUpdateError::TooManyCues { count: 0 })?;
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
        let target = objects
            .first()
            .map_or(SubjectiveTarget::SelfBody, |object| {
                SubjectiveTarget::Object(object.id)
            });
        let active_goals = if objects.is_empty() {
            Vec::new()
        } else {
            vec![ActiveGoal {
                action_kind,
                target,
                urgency: actor.body.energy.max(0),
            }]
        };
        actor.proposals = active_goals
            .iter()
            .map(|goal| ActionProposal {
                action_kind: goal.action_kind,
                target: goal.target,
                intensity: goal.urgency.min(1),
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
    error: ontopolis_cognition::AttentionUpdateError,
) -> ontopolis_cognition::SceneUpdateError {
    match error {
        ontopolis_cognition::AttentionUpdateError::TimeRegression => {
            ontopolis_cognition::SceneUpdateError::TimeRegression
        }
        ontopolis_cognition::AttentionUpdateError::TooManyCandidates { count } => {
            ontopolis_cognition::SceneUpdateError::TooManyCues { count }
        }
        ontopolis_cognition::AttentionUpdateError::DuplicateTarget { .. } => {
            ontopolis_cognition::SceneUpdateError::DuplicatePercept
        }
    }
}

fn motion_band(velocity: Velocity) -> u8 {
    velocity.length_squared().sqrt().min(f64::from(u8::MAX)) as u8
}
