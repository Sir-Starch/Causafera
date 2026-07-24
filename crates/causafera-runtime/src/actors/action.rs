use crate::*;
use causafera_core::*;
use causafera_types::*;
use std::sync::{Arc, Mutex};

use causafera_core::StateFingerprint;
use causafera_types::WorldCoord;

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
pub(crate) struct ActorActionSystem {
    pub(crate) state: Arc<Mutex<RuntimeState>>,
    pub(crate) next_time: SimulationTime,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct MaterialSurfaceContactProposal {
    pub(crate) actor: ActorId,
    pub(crate) surface: MaterialSurfaceId,
    pub(crate) next_actor: ActorState,
    pub(crate) before_surface: MaterialSurface,
    pub(crate) after_surface: MaterialSurface,
    pub(crate) causes: Vec<TraceId>,
}

impl ActorActionSystem {
    pub(crate) fn new(state: Arc<Mutex<RuntimeState>>) -> Self {
        Self {
            state,
            next_time: SimulationTime::new(1),
        }
    }

    pub(crate) fn execute(&mut self) -> Result<(), RuntimeError> {
        let mut state = self.state.lock().map_err(|_| RuntimeError::StatePoisoned)?;
        let actor_ids = state.actors.keys().copied().collect::<Vec<_>>();
        for actor_id in actor_ids {
            let proposals = state
                .actors
                .get(&actor_id)
                .map(|actor| actor.proposals.clone())
                .unwrap_or_default();
            for (ordinal, proposal) in proposals.into_iter().enumerate() {
                let actor = state
                    .actors
                    .get(&actor_id)
                    .expect("actor ID was collected from actor registry")
                    .clone();
                match validate_action(&actor, proposal, state.actor_action_bounds) {
                    Ok(relative) => {
                        let mut next_actor = actor.clone();
                        apply_action(&mut next_actor, relative, proposal.intensity);
                        let surface_id = resolve_material_surface(&state, next_actor.body.position)
                            .ok_or(RuntimeError::InvalidSnapshot("missing material surface"))?;
                        let before_surface = state
                            .material_surfaces
                            .get(&surface_id)
                            .copied()
                            .ok_or(RuntimeError::InvalidSnapshot("missing material surface"))?;
                        let after_surface = MaterialSurface {
                            condition: before_surface.condition.saturating_add(1),
                            contact_count: before_surface.contact_count.saturating_add(1),
                            last_transition: before_surface.last_transition,
                            last_contact_trace: before_surface.last_contact_trace,
                            gate: before_surface.gate,
                        };
                        let actor_cause = state
                            .actor_ancestry
                            .get(&actor_id)
                            .and_then(|traces| traces.last())
                            .copied()
                            .unwrap_or(state.latest_physical_trace);
                        let contact = MaterialSurfaceContactProposal {
                            actor: actor_id,
                            surface: surface_id,
                            next_actor: next_actor.clone(),
                            before_surface,
                            after_surface,
                            causes: ordered_trace_causes([
                                before_surface.last_transition,
                                actor_cause,
                            ]),
                        };
                        let trace = commit_material_surface_contact_events(
                            &mut state,
                            self.next_time,
                            ordinal as u64,
                            &actor,
                            contact,
                        )?;
                        next_actor
                            .validation_results
                            .push(ActionValidationResult::Valid { trace });
                        state.actors.insert(actor_id, next_actor);
                        if let Some(ancestry) = state.actor_ancestry.get_mut(&actor_id) {
                            ancestry.push(trace);
                        }
                        state.actor_actions_committed =
                            state.actor_actions_committed.saturating_add(1);
                        state.latest_physical_trace = trace;
                    }
                    Err(cause) => {
                        let event = rejected_action_event(
                            actor_id,
                            ordinal as u64,
                            cause,
                            state.latest_physical_trace,
                        )?;
                        let trace = state.traces.commit_batch(
                            self.next_time,
                            Phase::Action,
                            vec![event],
                        )?[0];
                        if let Some(actor) = state.actors.get_mut(&actor_id) {
                            actor
                                .validation_results
                                .push(ActionValidationResult::Invalid { cause, trace });
                        }
                        state.actor_actions_rejected =
                            state.actor_actions_rejected.saturating_add(1);
                    }
                }
            }
        }
        state.advanced_through = self.next_time;
        self.next_time = self.next_time.tick();
        Ok(())
    }
}

impl System for ActorActionSystem {
    fn run(&mut self, _stream: &mut RandomStream) {
        if let Err(error) = self.execute()
            && let Ok(mut state) = self.state.lock()
        {
            state.failure.get_or_insert(error);
        }
    }

    fn restore_time(&mut self, time: SimulationTime) {
        self.next_time = time;
    }
}

fn rejected_action_event(
    actor_id: ActorId,
    ordinal: u64,
    rejection: ActionRejection,
    cause: TraceId,
) -> Result<CausalEventProposal, RuntimeError> {
    let code = match rejection {
        ActionRejection::MissingSubjectiveTarget => 1,
        ActionRejection::OutOfBounds => 2,
        ActionRejection::InsufficientEnergy => 3,
    };
    Ok(CausalEventProposal::new(
        EventProposalKey::new(ACTOR_ACTION_SYSTEM_ID, actor_id.raw(), ordinal),
        EventKindId::new(ACTOR_REJECTION_EVENT_KIND),
        vec![cause],
        vec![CausalEffect::new(
            CausalTarget::new(
                StateObjectKindId::new(ACTOR_OBJECT_KIND),
                actor_id.raw(),
                StatePropertyId::new(ACTOR_REJECTION_PROPERTY),
            ),
            fingerprint_pair(0x0701, actor_id.raw() as i64, 0),
            fingerprint_pair(0x0701, actor_id.raw() as i64, code),
        )?],
    )?)
}
