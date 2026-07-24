use crate::*;
use causafera_core::*;
use causafera_types::*;
use std::sync::{Arc, Mutex};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PopulationAggregate {
    pub chart: ChartChunkCoord,
    pub count: u64,
    pub births: u64,
    pub deaths: u64,
    pub material_inflow: i64,
    pub material_outflow: i64,
    pub causal_ancestry: Vec<TraceId>,
}

impl PopulationAggregate {
    pub fn new(
        chart: ChartChunkCoord,
        count: u64,
        causal_ancestry: Vec<TraceId>,
    ) -> Result<Self, RuntimeError> {
        validate_trace_ancestry(&causal_ancestry)?;
        Ok(Self {
            chart,
            count,
            births: 0,
            deaths: 0,
            material_inflow: 0,
            material_outflow: 0,
            causal_ancestry,
        })
    }
}

pub(crate) struct PopulationLifecycleSystem {
    pub(crate) state: Arc<Mutex<RuntimeState>>,
    pub(crate) next_time: SimulationTime,
}

impl PopulationLifecycleSystem {
    pub(crate) fn new(state: Arc<Mutex<RuntimeState>>) -> Self {
        Self {
            state,
            next_time: SimulationTime::new(1),
        }
    }

    pub(crate) fn execute(&mut self) -> Result<(), RuntimeError> {
        let mut state = self.state.lock().map_err(|_| RuntimeError::StatePoisoned)?;
        lifecycle_births_and_deaths(&mut state, self.next_time)?;
        lifecycle_movement(&mut state, self.next_time)?;
        lifecycle_actor_resolution(&mut state, self.next_time)?;
        lifecycle_material_activity(&mut state, self.next_time)?;
        state.advanced_through = self.next_time;
        self.next_time = self.next_time.tick();
        Ok(())
    }
}

impl System for PopulationLifecycleSystem {
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

fn lifecycle_births_and_deaths(
    state: &mut RuntimeState,
    time: SimulationTime,
) -> Result<(), RuntimeError> {
    if state.population_aggregates.is_empty() {
        return Ok(());
    }
    if time.raw().is_multiple_of(11) && total_population(state) < 16 {
        let chunk = first_population_chunk(state)?;
        let mut aggregate = state
            .population_aggregates
            .get(&chunk)
            .expect("population chunk selected from aggregate map")
            .clone();
        let before = aggregate.clone();
        aggregate.count = aggregate.count.saturating_add(1);
        aggregate.births = aggregate.births.saturating_add(1);
        let trace = commit_population_event(
            state,
            time,
            POPULATION_LIFECYCLE_EVENT_KIND,
            chunk,
            1,
            &before,
            &aggregate,
        )?;
        aggregate.causal_ancestry = append_trace(&aggregate.causal_ancestry, trace)?;
        state.population_aggregates.insert(chunk, aggregate);
        state.population_births = state.population_births.saturating_add(1);
    }
    if time.raw().is_multiple_of(17) && total_population(state) > 1 {
        if let Some(actor_id) = state.actors.keys().next().copied() {
            lifecycle_actor_death(state, time, actor_id)?;
        } else {
            let chunk = first_population_chunk(state)?;
            let mut aggregate = state
                .population_aggregates
                .get(&chunk)
                .expect("population chunk selected from aggregate map")
                .clone();
            if aggregate.count > 0 {
                let before = aggregate.clone();
                aggregate.count -= 1;
                aggregate.deaths = aggregate.deaths.saturating_add(1);
                let trace = commit_population_event(
                    state,
                    time,
                    POPULATION_LIFECYCLE_EVENT_KIND,
                    chunk,
                    2,
                    &before,
                    &aggregate,
                )?;
                aggregate.causal_ancestry = append_trace(&aggregate.causal_ancestry, trace)?;
                state.population_aggregates.insert(chunk, aggregate);
                state.population_deaths = state.population_deaths.saturating_add(1);
            }
        }
    }
    Ok(())
}

fn lifecycle_movement(state: &mut RuntimeState, time: SimulationTime) -> Result<(), RuntimeError> {
    if !time.raw().is_multiple_of(5) {
        return Ok(());
    }
    let Some(from) = state
        .population_aggregates
        .iter()
        .find_map(|(chunk, aggregate)| (aggregate.count > 1).then_some(*chunk))
    else {
        return Ok(());
    };
    let Some(to) = state
        .population_aggregates
        .keys()
        .copied()
        .find(|chunk| *chunk != from)
        .or_else(|| {
            state
                .active_chunks
                .keys()
                .copied()
                .find(|chunk| *chunk != from)
        })
    else {
        return Ok(());
    };
    let mut source = state
        .population_aggregates
        .get(&from)
        .expect("source selected from aggregate map")
        .clone();
    let mut target = state
        .population_aggregates
        .get(&to)
        .cloned()
        .unwrap_or_else(|| PopulationAggregate {
            chart: to,
            count: 0,
            births: 0,
            deaths: 0,
            material_inflow: 0,
            material_outflow: 0,
            causal_ancestry: source.causal_ancestry.clone(),
        });
    let before_source = source.clone();
    let before_target = target.clone();
    source.count -= 1;
    target.count = target.count.saturating_add(1);
    let source_trace = commit_population_event(
        state,
        time,
        POPULATION_LIFECYCLE_EVENT_KIND,
        from,
        3,
        &before_source,
        &source,
    )?;
    let target_trace = commit_population_event(
        state,
        time,
        POPULATION_LIFECYCLE_EVENT_KIND,
        to,
        4,
        &before_target,
        &target,
    )?;
    source.causal_ancestry = append_trace(&source.causal_ancestry, source_trace)?;
    target.causal_ancestry = append_trace(&target.causal_ancestry, target_trace)?;
    state.population_aggregates.insert(from, source);
    state.population_aggregates.insert(to, target);
    state.population_movements = state.population_movements.saturating_add(1);
    Ok(())
}

fn lifecycle_actor_resolution(
    state: &mut RuntimeState,
    time: SimulationTime,
) -> Result<(), RuntimeError> {
    let promote_chunk = state
        .active_chunks
        .iter()
        .find_map(|(chunk, active)| (active.level > 0).then_some(*chunk))
        .or_else(|| {
            time.raw().is_multiple_of(7)
                .then(|| first_population_chunk(state).ok())
                .flatten()
        });
    if let Some(chunk) = promote_chunk {
        promote_actor_from_aggregate(state, time, chunk)?;
    }
    if time.raw().is_multiple_of(13) && state.actors.len() > 2
        && let Some(actor_id) = state.actors.keys().next_back().copied() {
            demote_actor_to_aggregate(state, time, actor_id)?;
        }
    Ok(())
}

fn lifecycle_material_activity(
    state: &mut RuntimeState,
    time: SimulationTime,
) -> Result<(), RuntimeError> {
    if !time.raw().is_multiple_of(3) {
        return Ok(());
    }
    let chunks = state
        .population_aggregates
        .keys()
        .copied()
        .collect::<Vec<_>>();
    for (ordinal, chunk) in chunks.into_iter().enumerate() {
        let mut aggregate = state
            .population_aggregates
            .get(&chunk)
            .expect("chunk selected from aggregate map")
            .clone();
        if aggregate.count == 0 {
            continue;
        }
        let before = aggregate.clone();
        let flow = i64::try_from(aggregate.count.min(16)).unwrap_or(16);
        aggregate.material_inflow = aggregate.material_inflow.saturating_add(flow);
        aggregate.material_outflow = aggregate.material_outflow.saturating_add(flow / 2);
        let trace = commit_material_event(state, time, chunk, ordinal as u64, &before, &aggregate)?;
        aggregate.causal_ancestry = append_trace(&aggregate.causal_ancestry, trace)?;
        state.population_aggregates.insert(chunk, aggregate);
        state.material_activity_events = state.material_activity_events.saturating_add(1);
    }
    Ok(())
}

pub(crate) fn promote_actor_from_aggregate(
    state: &mut RuntimeState,
    time: SimulationTime,
    chunk: ChartChunkCoord,
) -> Result<(), RuntimeError> {
    if state.actors.len() >= usize::from(state.config.actor_count) {
        return Ok(());
    }
    let Some(mut aggregate) = state.population_aggregates.get(&chunk).cloned() else {
        return Ok(());
    };
    if aggregate.count == 0 {
        return Ok(());
    }
    let before = aggregate.clone();
    aggregate.count -= 1;
    let actor_id = ActorId::new(state.next_actor_id);
    let actor = ActorState::new(
        MinimalBodyState::stationary(
            WorldCoord::new(
                i64::from(chunk.chunk.x),
                i64::from(chunk.chunk.y),
                i64::from(chunk.chunk.z),
            ),
            crate::ACTOR_BASE_ENERGY,
        ),
        bootstrap_sensors(state.config.sensor_count),
    )?;
    let trace = commit_actor_transition(
        state,
        time,
        ACTOR_PROMOTION_EVENT_KIND,
        ACTOR_PROMOTION_PROPERTY,
        actor_id,
        &aggregate.causal_ancestry,
        fingerprint_u64(0x0901, 0),
        actor_state_fingerprint(&actor),
    )?;
    aggregate.causal_ancestry = append_trace(&aggregate.causal_ancestry, trace)?;
    let aggregate_trace = commit_population_event(
        state,
        time,
        ACTOR_PROMOTION_EVENT_KIND,
        chunk,
        actor_id.raw(),
        &before,
        &aggregate,
    )?;
    aggregate.causal_ancestry = append_trace(&aggregate.causal_ancestry, aggregate_trace)?;
    state.population_aggregates.insert(chunk, aggregate);
    state
        .actor_ancestry
        .insert(actor_id, vec![trace, aggregate_trace]);
    state.actor_objects.insert(
        actor_id.raw(),
        ActorPhysicalObject::new(
            actor_id.raw(),
            WorldCoord::new(
                i64::from(chunk.chunk.x).saturating_add(2),
                i64::from(chunk.chunk.y),
                i64::from(chunk.chunk.z),
            ),
            9,
            trace,
        ),
    );
    state.actors.insert(actor_id, actor);
    state.next_actor_id = state.next_actor_id.saturating_add(1);
    state.actor_promotions = state.actor_promotions.saturating_add(1);
    Ok(())
}

fn bootstrap_sensors(sensor_count: u8) -> Vec<SensorAperture> {
    (0..sensor_count)
        .map(|index| SensorAperture::new(LocalCoord::new(index, 0, 0), 8, SensorKindId::new(1)))
        .collect()
}

fn demote_actor_to_aggregate(
    state: &mut RuntimeState,
    time: SimulationTime,
    actor_id: ActorId,
) -> Result<(), RuntimeError> {
    let Some(actor) = state.actors.get(&actor_id).cloned() else {
        return Ok(());
    };
    let chunk = first_population_chunk(state)?;
    let mut aggregate = state
        .population_aggregates
        .get(&chunk)
        .expect("population chunk selected from aggregate map")
        .clone();
    let before = aggregate.clone();
    aggregate.count = aggregate.count.saturating_add(1);
    let causes = state
        .actor_ancestry
        .get(&actor_id)
        .cloned()
        .unwrap_or_default();
    let trace = commit_actor_transition(
        state,
        time,
        ACTOR_DEMOTION_EVENT_KIND,
        ACTOR_PROMOTION_PROPERTY,
        actor_id,
        &causes,
        actor_state_fingerprint(&actor),
        fingerprint_u64(0x0A01, actor_id.raw()),
    )?;
    aggregate.causal_ancestry = merge_trace_ancestry(&aggregate.causal_ancestry, &causes, trace)?;
    let aggregate_trace = commit_population_event(
        state,
        time,
        ACTOR_DEMOTION_EVENT_KIND,
        chunk,
        actor_id.raw(),
        &before,
        &aggregate,
    )?;
    aggregate.causal_ancestry = append_trace(&aggregate.causal_ancestry, aggregate_trace)?;
    state.population_aggregates.insert(chunk, aggregate);
    state.actors.remove(&actor_id);
    state.actor_ancestry.remove(&actor_id);
    state.actor_objects.remove(&actor_id.raw());
    state.actor_demotions = state.actor_demotions.saturating_add(1);
    Ok(())
}

fn lifecycle_actor_death(
    state: &mut RuntimeState,
    time: SimulationTime,
    actor_id: ActorId,
) -> Result<(), RuntimeError> {
    let Some(actor) = state.actors.get(&actor_id).cloned() else {
        return Ok(());
    };
    let causes = state
        .actor_ancestry
        .get(&actor_id)
        .cloned()
        .unwrap_or_default();
    commit_actor_transition(
        state,
        time,
        POPULATION_LIFECYCLE_EVENT_KIND,
        ACTOR_BODY_PROPERTY,
        actor_id,
        &causes,
        actor_state_fingerprint(&actor),
        fingerprint_u64(0x0802, actor_id.raw()),
    )?;
    state.actors.remove(&actor_id);
    state.actor_ancestry.remove(&actor_id);
    state.actor_objects.remove(&actor_id.raw());
    state.population_deaths = state.population_deaths.saturating_add(1);
    Ok(())
}

fn commit_population_event(
    state: &mut RuntimeState,
    time: SimulationTime,
    kind: u64,
    chunk: ChartChunkCoord,
    ordinal: u64,
    before: &PopulationAggregate,
    after: &PopulationAggregate,
) -> Result<TraceId, RuntimeError> {
    let causes = append_trace(&after.causal_ancestry, state.latest_physical_trace)?;
    let event = CausalEventProposal::new(
        EventProposalKey::new(LIFECYCLE_SYSTEM_ID, chart_chunk_hash(chunk), ordinal),
        EventKindId::new(kind),
        causes,
        vec![CausalEffect::new(
            CausalTarget::new(
                StateObjectKindId::new(POPULATION_OBJECT_KIND),
                chart_chunk_hash(chunk),
                StatePropertyId::new(POPULATION_AGGREGATE_PROPERTY),
            ),
            fingerprint_population_aggregate(before),
            fingerprint_population_aggregate(after),
        )?],
    )?;
    let trace = state
        .traces
        .commit_batch(time, Phase::Lifecycle, vec![event])?[0];
    state.latest_physical_trace = trace;
    Ok(trace)
}

#[allow(clippy::too_many_arguments)]
fn commit_actor_transition(
    state: &mut RuntimeState,
    time: SimulationTime,
    kind: u64,
    property: u64,
    actor: ActorId,
    ancestry: &[TraceId],
    before: StateFingerprint,
    after: StateFingerprint,
) -> Result<TraceId, RuntimeError> {
    let causes = append_trace(ancestry, state.latest_physical_trace)?;
    let event = CausalEventProposal::new(
        EventProposalKey::new(LIFECYCLE_SYSTEM_ID, actor.raw(), property),
        EventKindId::new(kind),
        causes,
        vec![CausalEffect::new(
            CausalTarget::new(
                StateObjectKindId::new(ACTOR_OBJECT_KIND),
                actor.raw(),
                StatePropertyId::new(property),
            ),
            before,
            after,
        )?],
    )?;
    let trace = state
        .traces
        .commit_batch(time, Phase::Lifecycle, vec![event])?[0];
    state.latest_physical_trace = trace;
    Ok(trace)
}

pub(crate) fn commit_material_event(
    state: &mut RuntimeState,
    time: SimulationTime,
    chunk: ChartChunkCoord,
    ordinal: u64,
    before: &PopulationAggregate,
    after: &PopulationAggregate,
) -> Result<TraceId, RuntimeError> {
    let causes = append_trace(&after.causal_ancestry, state.latest_physical_trace)?;
    let event = CausalEventProposal::new(
        EventProposalKey::new(LIFECYCLE_SYSTEM_ID, chart_chunk_hash(chunk), 100 + ordinal),
        EventKindId::new(MATERIAL_ACTIVITY_EVENT_KIND),
        causes,
        vec![CausalEffect::new(
            CausalTarget::new(
                StateObjectKindId::new(MATERIAL_OBJECT_KIND),
                chart_chunk_hash(chunk),
                StatePropertyId::new(MATERIAL_FLOW_PROPERTY),
            ),
            fingerprint_material_flow(before),
            fingerprint_material_flow(after),
        )?],
    )?;
    let trace = state
        .traces
        .commit_batch(time, Phase::Lifecycle, vec![event])?[0];
    state.latest_physical_trace = trace;
    Ok(trace)
}

pub(crate) fn first_population_chunk(
    state: &RuntimeState,
) -> Result<ChartChunkCoord, RuntimeError> {
    state
        .population_aggregates
        .keys()
        .next()
        .copied()
        .ok_or(RuntimeError::InvalidPopulationAggregate)
}

fn total_population(state: &RuntimeState) -> u64 {
    state
        .population_aggregates
        .values()
        .map(|aggregate| aggregate.count)
        .sum::<u64>()
        .saturating_add(state.actors.len() as u64)
}
