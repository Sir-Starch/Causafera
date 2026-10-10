use std::collections::BTreeSet;

use causafera_domains::{
    ManaFieldSet, PhysicalPatternSample, ThermalBoundaryRecord, ThermalFieldSet,
    ThermalInjectionProposal, ThermalReservoirId,
};
use causafera_resolution::ResolutionField;
use causafera_types::{ChartChunkCoord, SimulationTime, TraceId};

use crate::{
    ActorId, ActorPhysicalObject, ActorState, ExperimentRecipeManaSourceReceipt,
    HistoryDigestPrefix, MaterialSurface, MaterialSurfaceId, PopulationAggregate, RuntimeError,
    RuntimeState,
};

type UndoAction = Box<dyn FnOnce(&mut RuntimeState) + Send>;

#[derive(Default)]
pub(crate) struct TickUndoLog {
    active: bool,
    first_new_trace: TraceId,
    actions: Vec<UndoAction>,
}

struct TickScalarUndo {
    history_digest_prefix: HistoryDigestPrefix,
    advanced_through: SimulationTime,
    latest_physical_trace: TraceId,
    latest_mana_trace: Option<TraceId>,
    executed_sources_len: usize,
    physical_events: u64,
    mana_cell_changes: u64,
    mana_physical_effects: u64,
    resolution_changes: u64,
    resolution_transitions: u64,
    perceived_actor_features: u64,
    subjective_actor_objects: u64,
    actor_actions_committed: u64,
    actor_actions_rejected: u64,
    population_births: u64,
    population_deaths: u64,
    population_movements: u64,
    actor_promotions: u64,
    actor_demotions: u64,
    material_activity_events: u64,
    next_actor_id: u64,
    last_mana_changes: u32,
    physical_scan_count: u64,
    failure: Option<RuntimeError>,
}

impl TickScalarUndo {
    fn capture(state: &RuntimeState) -> Self {
        Self {
            history_digest_prefix: state.history_digest_prefix.clone(),
            advanced_through: state.advanced_through,
            latest_physical_trace: state.latest_physical_trace,
            latest_mana_trace: state.latest_mana_trace,
            executed_sources_len: state.executed_experiment_recipe_mana_sources.len(),
            physical_events: state.physical_events,
            mana_cell_changes: state.mana_cell_changes,
            mana_physical_effects: state.mana_physical_effects,
            resolution_changes: state.resolution_changes,
            resolution_transitions: state.resolution_transitions,
            perceived_actor_features: state.perceived_actor_features,
            subjective_actor_objects: state.subjective_actor_objects,
            actor_actions_committed: state.actor_actions_committed,
            actor_actions_rejected: state.actor_actions_rejected,
            population_births: state.population_births,
            population_deaths: state.population_deaths,
            population_movements: state.population_movements,
            actor_promotions: state.actor_promotions,
            actor_demotions: state.actor_demotions,
            material_activity_events: state.material_activity_events,
            next_actor_id: state.next_actor_id,
            last_mana_changes: state.last_mana_changes,
            physical_scan_count: state.physical_scan_count,
            failure: state.failure.clone(),
        }
    }

    fn restore(self, state: &mut RuntimeState) {
        state.history_digest_prefix = self.history_digest_prefix;
        state.advanced_through = self.advanced_through;
        state.latest_physical_trace = self.latest_physical_trace;
        state.latest_mana_trace = self.latest_mana_trace;
        state
            .executed_experiment_recipe_mana_sources
            .truncate(self.executed_sources_len);
        state.physical_events = self.physical_events;
        state.mana_cell_changes = self.mana_cell_changes;
        state.mana_physical_effects = self.mana_physical_effects;
        state.resolution_changes = self.resolution_changes;
        state.resolution_transitions = self.resolution_transitions;
        state.perceived_actor_features = self.perceived_actor_features;
        state.subjective_actor_objects = self.subjective_actor_objects;
        state.actor_actions_committed = self.actor_actions_committed;
        state.actor_actions_rejected = self.actor_actions_rejected;
        state.population_births = self.population_births;
        state.population_deaths = self.population_deaths;
        state.population_movements = self.population_movements;
        state.actor_promotions = self.actor_promotions;
        state.actor_demotions = self.actor_demotions;
        state.material_activity_events = self.material_activity_events;
        state.next_actor_id = self.next_actor_id;
        state.last_mana_changes = self.last_mana_changes;
        state.physical_scan_count = self.physical_scan_count;
        state.failure = self.failure;
    }
}

impl TickUndoLog {
    pub(crate) fn begin(&mut self, state: &RuntimeState, first_new_trace: TraceId) {
        self.actions.clear();
        self.first_new_trace = first_new_trace;
        self.active = true;
        let scalars = TickScalarUndo::capture(state);
        self.push_undo(move |state| scalars.restore(state));
    }

    pub(crate) fn commit(&mut self) {
        self.actions.clear();
        self.active = false;
    }

    pub(crate) fn is_active(&self) -> bool {
        self.active
    }

    pub(crate) fn push_undo<F>(&mut self, undo: F)
    where
        F: FnOnce(&mut RuntimeState) + Send + 'static,
    {
        if self.active {
            self.actions.push(Box::new(undo));
        }
    }

    pub(crate) fn push_hydrology(&mut self, undo: crate::hydrology_undo::HydrologyUndo) {
        self.push_undo(move |state| undo.restore(&mut state.hydrology));
    }

    pub(crate) fn rollback(&mut self, state: &mut RuntimeState) {
        for undo in self.actions.drain(..).rev() {
            undo(state);
        }
        let _ = state.thermal_receipts.split_off(&self.first_new_trace);
        let _ = state
            .thermal_conservation_receipts
            .split_off(&self.first_new_trace);
        self.active = false;
    }
}

impl RuntimeState {
    pub(crate) fn begin_tick_undo(&mut self) -> Result<(), RuntimeError> {
        let first_new_trace = TraceId::new(
            self.history
                .visible_head()
                .raw()
                .checked_add(1)
                .ok_or(causafera_core::HistoryError::ArithmeticOverflow)?,
        );
        let mut undo = std::mem::take(&mut self.tick_undo);
        undo.begin(self, first_new_trace);
        self.tick_undo = undo;
        Ok(())
    }

    pub(crate) fn commit_tick_undo(&mut self) {
        self.tick_undo.commit();
    }

    pub(crate) fn rollback_tick_undo(&mut self) {
        let mut undo = std::mem::take(&mut self.tick_undo);
        undo.rollback(self);
        self.tick_undo = undo;
    }

    pub(crate) fn push_actor_ancestry(&mut self, actor: ActorId, trace: TraceId) {
        if let Some(ancestry) = self.actor_ancestry.get_mut(&actor) {
            if self.tick_undo.is_active() {
                let length = ancestry.len();
                self.tick_undo.push_undo(move |state| {
                    if let Some(traces) = state.actor_ancestry.get_mut(&actor) {
                        traces.truncate(length);
                    }
                });
            }
            ancestry.push(trace);
        }
    }

    pub(crate) fn insert_actor_ancestry(&mut self, actor: ActorId, traces: Vec<TraceId>) {
        let previous = self.actor_ancestry.insert(actor, traces);
        self.tick_undo.push_undo(move |state| match previous {
            Some(traces) => {
                state.actor_ancestry.insert(actor, traces);
            }
            None => {
                state.actor_ancestry.remove(&actor);
            }
        });
    }

    pub(crate) fn remove_actor_ancestry(&mut self, actor: ActorId) {
        let previous = self.actor_ancestry.remove(&actor);
        self.tick_undo.push_undo(move |state| {
            if let Some(traces) = previous {
                state.actor_ancestry.insert(actor, traces);
            }
        });
    }

    pub(crate) fn replace_mana(&mut self, next: ManaFieldSet) {
        let previous = std::mem::replace(&mut self.mana, next);
        self.tick_undo.push_undo(move |state| state.mana = previous);
    }

    pub(crate) fn replace_thermal_fields(&mut self, next: ThermalFieldSet) {
        let previous = std::mem::replace(&mut self.thermal_fields, next);
        self.tick_undo
            .push_undo(move |state| state.thermal_fields = previous);
    }

    pub(crate) fn replace_thermal_boundary_records(&mut self, next: Vec<ThermalBoundaryRecord>) {
        let previous = std::mem::replace(&mut self.thermal_boundary_records, next);
        self.tick_undo
            .push_undo(move |state| state.thermal_boundary_records = previous);
    }

    pub(crate) fn replace_pending_thermal_injections(
        &mut self,
        next: Vec<ThermalInjectionProposal>,
    ) {
        let previous = std::mem::replace(&mut self.pending_thermal_injections, next);
        self.tick_undo
            .push_undo(move |state| state.pending_thermal_injections = previous);
    }

    pub(crate) fn clear_pending_thermal_injections(&mut self) {
        self.replace_pending_thermal_injections(Vec::new());
    }

    pub(crate) fn replace_resolution(&mut self, next: ResolutionField) {
        let previous = std::mem::replace(&mut self.resolution, next);
        self.tick_undo
            .push_undo(move |state| state.resolution = previous);
    }

    pub(crate) fn record_thermal_reservoir_before(&mut self, id: ThermalReservoirId) {
        let previous = self.thermal_reservoirs.get(&id).copied();
        self.tick_undo.push_undo(move |state| match previous {
            Some(reservoir) => {
                state.thermal_reservoirs.insert(id, reservoir);
            }
            None => {
                state.thermal_reservoirs.remove(&id);
            }
        });
    }

    pub(crate) fn record_active_chunk_before(&mut self, chunk: ChartChunkCoord) {
        let previous = self.active_chunks.get(&chunk).copied();
        self.tick_undo.push_undo(move |state| match previous {
            Some(active) => {
                state.active_chunks.insert(chunk, active);
            }
            None => {
                state.active_chunks.remove(&chunk);
            }
        });
    }

    pub(crate) fn record_all_actors_before(&mut self) {
        let actors = self.actors.keys().copied().collect::<Vec<_>>();
        for actor in actors {
            self.record_actor_before(actor);
        }
    }

    pub(crate) fn record_actor_before(&mut self, actor: ActorId) {
        let previous = self.actors.get(&actor).cloned();
        self.tick_undo.push_undo(move |state| match previous {
            Some(value) => {
                state.actors.insert(actor, value);
            }
            None => {
                state.actors.remove(&actor);
            }
        });
    }

    pub(crate) fn insert_actor(&mut self, actor: ActorId, value: ActorState) {
        let previous = self.actors.insert(actor, value);
        self.tick_undo.push_undo(move |state| match previous {
            Some(value) => {
                state.actors.insert(actor, value);
            }
            None => {
                state.actors.remove(&actor);
            }
        });
    }

    pub(crate) fn remove_actor(&mut self, actor: &ActorId) -> Option<ActorState> {
        let previous = self.actors.remove(actor);
        let key = *actor;
        let undo_previous = previous.clone();
        self.tick_undo.push_undo(move |state| {
            if let Some(value) = undo_previous {
                state.actors.insert(key, value);
            }
        });
        previous
    }

    pub(crate) fn insert_actor_object(&mut self, id: u64, value: ActorPhysicalObject) {
        let previous = self.actor_objects.insert(id, value);
        self.tick_undo.push_undo(move |state| match previous {
            Some(value) => {
                state.actor_objects.insert(id, value);
            }
            None => {
                state.actor_objects.remove(&id);
            }
        });
    }

    pub(crate) fn remove_actor_object(&mut self, id: &u64) -> Option<ActorPhysicalObject> {
        let previous = self.actor_objects.remove(id);
        let key = *id;
        let undo_previous = previous;
        self.tick_undo.push_undo(move |state| {
            if let Some(value) = undo_previous {
                state.actor_objects.insert(key, value);
            }
        });
        previous
    }

    pub(crate) fn insert_population_aggregate(
        &mut self,
        chunk: ChartChunkCoord,
        value: PopulationAggregate,
    ) {
        let previous = self.population_aggregates.insert(chunk, value);
        self.tick_undo.push_undo(move |state| match previous {
            Some(value) => {
                state.population_aggregates.insert(chunk, value);
            }
            None => {
                state.population_aggregates.remove(&chunk);
            }
        });
    }

    pub(crate) fn clear_pending_samples(&mut self) {
        let previous = std::mem::take(&mut self.pending_samples);
        self.tick_undo
            .push_undo(move |state| state.pending_samples = previous);
    }

    pub(crate) fn extend_pending_samples<I>(&mut self, samples: I)
    where
        I: IntoIterator<Item = PhysicalPatternSample>,
    {
        // Appending keeps what is already pending; the undo only has to drop the
        // appended tail, which later undos restore around.
        let previous_len = self.pending_samples.len();
        self.pending_samples.extend(samples);
        self.tick_undo
            .push_undo(move |state| state.pending_samples.truncate(previous_len));
    }

    pub(crate) fn take_pending_material_surface_changes(&mut self) -> BTreeSet<MaterialSurfaceId> {
        let previous = self.pending_material_surface_changes.clone();
        self.tick_undo
            .push_undo(move |state| state.pending_material_surface_changes = previous);
        std::mem::take(&mut self.pending_material_surface_changes)
    }

    pub(crate) fn insert_pending_material_surface_change(&mut self, id: MaterialSurfaceId) {
        if self.pending_material_surface_changes.insert(id) {
            self.tick_undo.push_undo(move |state| {
                state.pending_material_surface_changes.remove(&id);
            });
        }
    }

    pub(crate) fn push_pattern_sample(&mut self, sample: PhysicalPatternSample) {
        let undo = self.pattern_history.push_with_undo(sample);
        self.tick_undo
            .push_undo(move |state| state.pattern_history.undo_push(undo));
    }

    pub(crate) fn insert_material_surface(
        &mut self,
        id: MaterialSurfaceId,
        value: MaterialSurface,
    ) {
        let previous = self.material_surfaces.insert(id, value);
        self.tick_undo.push_undo(move |state| match previous {
            Some(value) => {
                state.material_surfaces.insert(id, value);
            }
            None => {
                state.material_surfaces.remove(&id);
            }
        });
    }

    pub(crate) fn record_material_surface_before(&mut self, id: MaterialSurfaceId) {
        let previous = self.material_surfaces.get(&id).copied();
        self.tick_undo.push_undo(move |state| match previous {
            Some(value) => {
                state.material_surfaces.insert(id, value);
            }
            None => {
                state.material_surfaces.remove(&id);
            }
        });
    }

    pub(crate) fn set_latest_physical_trace(&mut self, trace: TraceId) {
        let previous = self.latest_physical_trace;
        self.latest_physical_trace = trace;
        self.tick_undo
            .push_undo(move |state| state.latest_physical_trace = previous);
    }

    pub(crate) fn set_latest_mana_trace(&mut self, trace: Option<TraceId>) {
        let previous = self.latest_mana_trace;
        self.latest_mana_trace = trace;
        self.tick_undo
            .push_undo(move |state| state.latest_mana_trace = previous);
    }

    pub(crate) fn set_advanced_through(&mut self, time: SimulationTime) {
        let previous = self.advanced_through;
        self.advanced_through = time;
        self.tick_undo
            .push_undo(move |state| state.advanced_through = previous);
    }

    pub(crate) fn push_executed_source(&mut self, receipt: ExperimentRecipeManaSourceReceipt) {
        self.executed_experiment_recipe_mana_sources.push(receipt);
    }

    pub(crate) fn advance_history_digest_prefix<'a>(
        &mut self,
        records: impl Iterator<Item = &'a causafera_core::OwnedCausalRecord>,
    ) {
        let previous = self.history_digest_prefix.clone();
        self.history_digest_prefix.advance_history_records(records);
        self.tick_undo
            .push_undo(move |state| state.history_digest_prefix = previous);
    }
}

#[cfg(test)]
mod tests {
    use causafera_types::{
        ChartChunkCoord, ChunkCoord, LocalCoord, PhysicalPatternId, SimulationTime, SpatialChartId,
        TraceId,
    };

    use crate::{RuntimeConfig, RuntimeState};
    use causafera_domains::PhysicalPatternSample;

    fn sample(ordinal: u32) -> PhysicalPatternSample {
        PhysicalPatternSample {
            chunk: ChartChunkCoord::new(SpatialChartId::new(1), ChunkCoord::new(0, 0, 0)),
            pattern: PhysicalPatternId::new(1),
            position: LocalCoord::new(0, 0, 0),
            observed_at: SimulationTime::new(1),
            magnitude: 1,
            source_ordinal: ordinal,
            cause: TraceId::new(0),
        }
    }

    #[test]
    fn extending_pending_samples_keeps_earlier_samples_and_undoes_only_the_tail() {
        // Given: one sample already pending when the tick opens.
        let mut state = RuntimeState::new(&RuntimeConfig::new(13)).expect("state");
        state.pending_samples = vec![sample(9)];
        state.begin_tick_undo().expect("tick undo opens");

        // When: the tick clears, then appends in two steps, as the pattern system does
        // (changed material first, terrain second).
        state.clear_pending_samples();
        state.extend_pending_samples([sample(0)]);
        state.extend_pending_samples([sample(1), sample(2)]);

        // Then: the second append did not discard the first.
        let ordinals = state
            .pending_samples
            .iter()
            .map(|sample| sample.source_ordinal)
            .collect::<Vec<_>>();
        assert_eq!(ordinals, vec![0, 1, 2]);

        // And a refused tick restores exactly what was pending before it.
        state.rollback_tick_undo();
        let restored = state
            .pending_samples
            .iter()
            .map(|sample| sample.source_ordinal)
            .collect::<Vec<_>>();
        assert_eq!(restored, vec![9]);
    }
}
