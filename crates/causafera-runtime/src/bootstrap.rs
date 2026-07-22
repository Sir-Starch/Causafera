use crate::*;
use causafera_core::*;
use causafera_types::*;
use thiserror::Error;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TerrainBootstrapStage {
    pub stage: HistoricalStageId,
    pub terrain_seed: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PopulationLifecycleStage {
    pub stage: HistoricalStageId,
    pub initial_population: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActorPromotionStage {
    pub stage: HistoricalStageId,
    pub max_promotions: u8,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MaterialActivityStage {
    pub stage: HistoricalStageId,
    pub flow_units: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MaterialSurfaceBootstrapStage {
    pub stage: HistoricalStageId,
    pub initial_condition: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HistoricalBootstrapPlan {
    pub physical_geography_init: TerrainBootstrapStage,
    pub material_surface: MaterialSurfaceBootstrapStage,
    pub population_lifecycle: PopulationLifecycleStage,
    pub actor_promotion: ActorPromotionStage,
    pub material_activity: MaterialActivityStage,
}

pub trait HistoricalBootstrapAdapter {
    fn bootstrap(&self, state: &mut RuntimeState) -> Result<Vec<TraceId>, BootstrapError>;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConcreteHistoricalBootstrapAdapter {
    Plan(HistoricalBootstrapPlan),
}

impl HistoricalBootstrapAdapter for ConcreteHistoricalBootstrapAdapter {
    fn bootstrap(&self, state: &mut RuntimeState) -> Result<Vec<TraceId>, BootstrapError> {
        match self {
            Self::Plan(plan) => plan.bootstrap(state),
        }
    }
}

impl HistoricalBootstrapAdapter for HistoricalBootstrapPlan {
    fn bootstrap(&self, state: &mut RuntimeState) -> Result<Vec<TraceId>, BootstrapError> {
        let mut traces = Vec::new();
        traces.extend(self.physical_geography_init.bootstrap(state)?);
        traces.extend(self.material_surface.bootstrap(state)?);
        traces.extend(self.population_lifecycle.bootstrap(state)?);
        traces.extend(self.actor_promotion.bootstrap(state)?);
        traces.extend(self.material_activity.bootstrap(state)?);
        Ok(traces)
    }
}

impl HistoricalBootstrapPlan {
    pub(crate) fn for_runtime_config(config: &RuntimeConfig) -> Self {
        let terrain_seed = match config.carrier_adapter {
            CarrierAdapterConfig::TerrainSeed { terrain_seed } => terrain_seed,
        };
        Self {
            physical_geography_init: TerrainBootstrapStage {
                stage: HistoricalStageId::new(1),
                terrain_seed,
            },
            material_surface: MaterialSurfaceBootstrapStage {
                stage: HistoricalStageId::new(2),
                initial_condition: 1,
            },
            population_lifecycle: PopulationLifecycleStage {
                stage: HistoricalStageId::new(3),
                initial_population: config.bootstrap_population,
            },
            actor_promotion: ActorPromotionStage {
                stage: HistoricalStageId::new(4),
                max_promotions: config.actor_count,
            },
            material_activity: MaterialActivityStage {
                stage: HistoricalStageId::new(5),
                flow_units: 1,
            },
        }
    }
}

impl HistoricalBootstrapAdapter for TerrainBootstrapStage {
    fn bootstrap(&self, state: &mut RuntimeState) -> Result<Vec<TraceId>, BootstrapError> {
        let chunks = state.carrier_adapters.keys().copied().collect::<Vec<_>>();
        let mut traces = Vec::with_capacity(chunks.len());
        for (ordinal, chunk) in chunks.into_iter().enumerate() {
            let trace = commit_bootstrap_stage_event(
                state,
                self.stage,
                chunk,
                ordinal as u64,
                POPULATION_BOOTSTRAP_EVENT_KIND,
                PHYSICAL_OBJECT_KIND,
                PHYSICAL_PROPERTY,
                fingerprint_u64(0x0B01, 0),
                fingerprint_u64(0x0B01, self.terrain_seed ^ chart_chunk_hash(chunk)),
            )?;
            let terrain = deterministic_terrain_chunk(
                self.terrain_seed ^ chart_chunk_hash(chunk),
                chunk,
                trace,
            );
            state
                .carrier_adapters
                .insert(chunk, TerrainCarrierAdapter::new(chunk, terrain, 3));
            traces.push(trace);
        }
        Ok(traces)
    }
}

impl HistoricalBootstrapAdapter for MaterialSurfaceBootstrapStage {
    fn bootstrap(&self, state: &mut RuntimeState) -> Result<Vec<TraceId>, BootstrapError> {
        let chunks = state.active_chunks.keys().copied().collect::<Vec<_>>();
        validate_material_surface_object_ids(
            chunks
                .iter()
                .copied()
                .map(|chunk| MaterialSurfaceId::new(chunk, 0)),
        )?;
        let mut traces = Vec::with_capacity(chunks.len());
        for (ordinal, chunk) in chunks.into_iter().enumerate() {
            let id = MaterialSurfaceId::new(chunk, 0);
            let trace = commit_material_surface_bootstrap_event(
                state,
                self.stage,
                ordinal as u64,
                id,
                self.initial_condition,
            )?;
            state.material_surfaces.insert(
                id,
                MaterialSurface {
                    condition: self.initial_condition,
                    contact_count: 0,
                    last_transition: trace,
                    last_contact_trace: None,
                    gate: MaterialSurfaceManaGate {
                        active: false,
                        last_transition: None,
                    },
                },
            );
            record_material_surface_transition(
                state,
                MaterialSurfaceTransition {
                    id,
                    occurred_at: SimulationTime::new(0),
                    before_condition: 0,
                    after_condition: self.initial_condition,
                    mana_total: 0,
                    contact_trace: None,
                    mana_effect_trace: None,
                    transition_trace: trace,
                },
            );
            traces.push(trace);
        }
        Ok(traces)
    }
}

impl HistoricalBootstrapAdapter for PopulationLifecycleStage {
    fn bootstrap(&self, state: &mut RuntimeState) -> Result<Vec<TraceId>, BootstrapError> {
        if self.initial_population == 0 || !state.population_aggregates.is_empty() {
            return Ok(Vec::new());
        }
        let chunks = state.active_chunks.keys().copied().collect::<Vec<_>>();
        let Some(chunk) = chunks.first().copied() else {
            return Ok(Vec::new());
        };
        let trace = commit_bootstrap_stage_event(
            state,
            self.stage,
            chunk,
            0,
            POPULATION_BOOTSTRAP_EVENT_KIND,
            POPULATION_OBJECT_KIND,
            POPULATION_AGGREGATE_PROPERTY,
            fingerprint_u64(0x0801, 0),
            fingerprint_u64(0x0801, self.initial_population),
        )?;
        state.population_aggregates.insert(
            chunk,
            PopulationAggregate::new(chunk, self.initial_population, vec![trace])?,
        );
        Ok(vec![trace])
    }
}

impl HistoricalBootstrapAdapter for ActorPromotionStage {
    fn bootstrap(&self, state: &mut RuntimeState) -> Result<Vec<TraceId>, BootstrapError> {
        let mut traces = Vec::new();
        for _ in 0..self.max_promotions {
            let before = state.actor_promotions;
            let chunk = first_population_chunk(state)?;
            promote_actor_from_aggregate(state, SimulationTime::new(0), chunk)?;
            if state.actor_promotions > before {
                traces.push(state.latest_physical_trace);
            }
        }
        Ok(traces)
    }
}

impl HistoricalBootstrapAdapter for MaterialActivityStage {
    fn bootstrap(&self, state: &mut RuntimeState) -> Result<Vec<TraceId>, BootstrapError> {
        let chunks = state
            .population_aggregates
            .keys()
            .copied()
            .collect::<Vec<_>>();
        let mut traces = Vec::new();
        for (ordinal, chunk) in chunks.into_iter().enumerate() {
            let mut aggregate = state
                .population_aggregates
                .get(&chunk)
                .expect("chunk selected from aggregate map")
                .clone();
            let before = aggregate.clone();
            aggregate.material_inflow = aggregate.material_inflow.saturating_add(self.flow_units);
            aggregate.material_outflow = aggregate
                .material_outflow
                .saturating_add(self.flow_units / 2);
            let trace = commit_material_event(
                state,
                SimulationTime::new(0),
                chunk,
                ordinal as u64,
                &before,
                &aggregate,
            )?;
            aggregate.causal_ancestry = append_trace(&aggregate.causal_ancestry, trace)?;
            state.population_aggregates.insert(chunk, aggregate);
            traces.push(trace);
        }
        Ok(traces)
    }
}

#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum BootstrapError {
    #[error(transparent)]
    Runtime(#[from] RuntimeError),
}

#[allow(clippy::too_many_arguments)]
fn commit_bootstrap_stage_event(
    state: &mut RuntimeState,
    stage: HistoricalStageId,
    chunk: ChartChunkCoord,
    ordinal: u64,
    kind: u64,
    object_kind: u64,
    property: u64,
    before: StateFingerprint,
    after: StateFingerprint,
) -> Result<TraceId, RuntimeError> {
    let causes = vec![state.latest_physical_trace];
    let event = CausalEventProposal::new(
        EventProposalKey::new(BOOTSTRAP_SYSTEM_ID, stage.raw(), ordinal),
        EventKindId::new(kind),
        causes,
        vec![CausalEffect::new(
            CausalTarget::new(
                StateObjectKindId::new(object_kind),
                chart_chunk_hash(chunk),
                StatePropertyId::new(property),
            ),
            before,
            after,
        )?],
    )?;
    let trace = state
        .traces
        .commit_batch(SimulationTime::new(0), Phase::Lifecycle, vec![event])?[0];
    state.latest_physical_trace = trace;
    Ok(trace)
}
