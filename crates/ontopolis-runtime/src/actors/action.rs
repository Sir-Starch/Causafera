use ontopolis_core::StateFingerprint;
use ontopolis_types::WorldCoord;

use super::{ActionProposal, ActionRejection, ActorState, SubjectiveTarget};

pub fn validate_action(
    actor: &ActorState,
    proposal: ActionProposal,
    action_bounds: i64,
) -> Result<[i64; 3], ActionRejection> {
    if actor.body.energy < proposal.intensity || proposal.intensity <= 0 {
        return Err(ActionRejection::InsufficientEnergy);
    }
    if action_bounds < 0 {
        return Err(ActionRejection::OutOfBounds);
    }
    let Some(scene) = actor.subjective_scene.as_ref() else {
        return Err(ActionRejection::MissingSubjectiveTarget);
    };
    let relative = match proposal.target {
        SubjectiveTarget::SelfBody => [0, 0, 0],
        SubjectiveTarget::Relative(relative) => [
            i64::from(relative[0]),
            i64::from(relative[1]),
            i64::from(relative[2]),
        ],
        SubjectiveTarget::Object(object_id) => scene
            .objects
            .iter()
            .find(|object| object.id == object_id)
            .map(|object| {
                [
                    i64::from(object.relative_position[0]),
                    i64::from(object.relative_position[1]),
                    i64::from(object.relative_position[2]),
                ]
            })
            .ok_or(ActionRejection::MissingSubjectiveTarget)?,
    };
    if relative
        .into_iter()
        .any(|component| component.unsigned_abs() > action_bounds as u64)
    {
        return Err(ActionRejection::OutOfBounds);
    }
    Ok(relative)
}

pub fn apply_action(actor: &mut ActorState, relative: [i64; 3], intensity: i64) {
    actor.body.position = WorldCoord::new(
        actor.body.position.x + relative[0].signum() * intensity,
        actor.body.position.y + relative[1].signum() * intensity,
        actor.body.position.z + relative[2].signum() * intensity,
    );
    actor.body.energy = actor.body.energy.saturating_sub(intensity);
}

pub fn actor_state_fingerprint(actor: &ActorState) -> StateFingerprint {
    let mut bytes = [0_u8; 32];
    let words = [
        actor.body.position.x as u64,
        actor.body.position.y as u64,
        actor.body.position.z as u64,
        actor.body.energy as u64,
    ];
    for (index, word) in words.into_iter().enumerate() {
        bytes[index * 8..(index + 1) * 8].copy_from_slice(&word.to_le_bytes());
    }
    StateFingerprint::new(bytes)
}
