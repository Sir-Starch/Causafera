use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{Arc, Mutex, MutexGuard},
};

use causafera_core::{
    CausalCommitError, CausalEffect, CausalEffectError, CausalEventProposal,
    CausalEventProposalError, CausalTarget, CausalTraceStore, EventProposalKey, Phase, Scheduler,
};
use causafera_domains::{
    ManaError, ManaField, ManaFieldSet, PhysicalPatternSample, ThermalActiveRegion,
    ThermalBoundaryRecord, ThermalCellKey, ThermalCellTransferReceipt, ThermalConservationReceipt,
    ThermalEnergy, ThermalError, ThermalField, ThermalFieldSet, ThermalInjectionProposal,
    ThermalParameters, ThermalReservoir, ThermalReservoirId, ThermalReservoirSchedule,
};
use causafera_explanation::{
    ComparisonContext, ExplanationClaim, ExplanationFrame, ExplanationReport,
    MATERIAL_SURFACE_LOOP_CLAIM_SCHEMA, MaterialSurfaceLocalManaTransitionClaim,
    MaterialSurfaceLoopClaim, NumericClaimValue, ThermalCarrierConservationClaim,
};
use causafera_observer_api::{
    MATERIAL_SURFACE_DELTA_SCHEMA_V3, MAX_MATERIAL_SURFACE_DELTAS, MAX_THERMAL_DELTAS,
    MaterialSurfaceDelta, MaterialSurfaceGateDelta, ObserverChunkSummary, ObserverWorldSnapshot,
    THERMAL_DELTA_SCHEMA_V1, ThermalFieldDelta,
};
use causafera_resolution::{ChannelWeight, ResolutionError, ResolutionField, ResolutionPolicy};
use causafera_types::{
    ChartChunkCoord, ChunkCoord, EventKindId, ExperimentId, ManaFieldId, ResolutionChannelId,
    ResolutionFieldId, SimulationTime, SpatialChartId, StateObjectKindId, StatePropertyId, TraceId,
};
use thiserror::Error;

use crate::*;

pub const MAX_RUNTIME_TICKS: u64 = 1_000_000;
pub const MAX_PATTERN_HISTORY_ENTRIES: usize = 512;
pub const MAX_PATTERN_HISTORY_PER_PATTERN: usize = 128;
pub const MANA_PATTERN_HISTORY_TICKS: u64 = 8;
pub const MAX_EXPERIMENT_RECIPE_MANA_SOURCES: usize = 16;
pub const EXPERIMENT_RECIPE_MANA_SOURCE_POLICY_SCHEMA_V1: u64 = 1;

pub const CURRENT_DIGEST_SCHEMA_VERSION: DigestSchemaVersion = DigestSchemaVersion::new(5);

const PHYSICAL_SYSTEM_ID: u64 = 10;
pub const EXPERIMENT_RECIPE_MANA_SOURCE_SYSTEM_ID: u64 = 19;
pub(crate) const MANA_SYSTEM_ID: u64 = 20;
pub(crate) const MANA_EFFECTS_SYSTEM_ID: u64 = 21;
pub(crate) const RESOLUTION_SYSTEM_ID: u64 = 30;
pub(crate) const ACTOR_ACTION_SYSTEM_ID: u64 = 42;
pub(crate) const LIFECYCLE_SYSTEM_ID: u64 = 60;
pub(crate) const BOOTSTRAP_SYSTEM_ID: u64 = 61;
pub(crate) const THERMAL_RESERVOIR_SYSTEM_ID: u64 = 11;
pub(crate) const THERMAL_EVOLUTION_SYSTEM_ID: u64 = 12;
const ROOT_EVENT_KIND: u64 = 1;
pub(crate) const MANA_EVENT_KIND: u64 = 3;
pub(crate) const RESOLUTION_EVENT_KIND: u64 = 4;
pub(crate) const ACTOR_CONTACT_ACTION_KIND: u64 = 6;
pub(crate) const ACTOR_REJECTION_EVENT_KIND: u64 = 7;
pub(crate) const POPULATION_BOOTSTRAP_EVENT_KIND: u64 = 8;
pub(crate) const POPULATION_LIFECYCLE_EVENT_KIND: u64 = 9;
pub(crate) const ACTOR_PROMOTION_EVENT_KIND: u64 = 10;
pub(crate) const ACTOR_DEMOTION_EVENT_KIND: u64 = 11;
pub(crate) const MATERIAL_ACTIVITY_EVENT_KIND: u64 = 12;
pub(crate) const MATERIAL_SURFACE_BOOTSTRAP_EVENT_KIND: u64 = 13;
pub(crate) const MATERIAL_SURFACE_CONTACT_EVENT_KIND: u64 = 14;
pub(crate) const MATERIAL_SURFACE_MANA_EVENT_KIND: u64 = 15;
pub const EXPERIMENT_RECIPE_MANA_SOURCE_EVENT_KIND: u64 = 17;
pub const THERMAL_RESERVOIR_TRANSFER_EVENT_KIND: u64 = 30;
pub const THERMAL_CELL_CHANGE_EVENT_KIND: u64 = 31;
pub const THERMAL_CONSERVATION_EVENT_KIND: u64 = 32;
pub const THERMAL_FIELD_BOOTSTRAP_EVENT_KIND: u64 = 28;
pub const THERMAL_RESERVOIR_BOOTSTRAP_EVENT_KIND: u64 = 29;
const RUNTIME_OBJECT_KIND: u64 = 1;
pub(crate) const PHYSICAL_OBJECT_KIND: u64 = 2;
pub(crate) const MANA_OBJECT_KIND: u64 = 3;
pub(crate) const RESOLUTION_OBJECT_KIND: u64 = 4;
pub(crate) const ACTOR_OBJECT_KIND: u64 = 5;
pub(crate) const POPULATION_OBJECT_KIND: u64 = 6;
pub(crate) const MATERIAL_OBJECT_KIND: u64 = 7;
pub(crate) const MATERIAL_SURFACE_OBJECT_KIND: u64 = 8;
pub const EXPERIMENT_RECIPE_MANA_SOURCE_OBJECT_KIND: u64 = 9;
pub const THERMAL_RESERVOIR_OBJECT_KIND: u64 = 10;
pub const THERMAL_CELL_OBJECT_KIND: u64 = 11;
pub const THERMAL_CARRIER_OBJECT_KIND: u64 = 12;
const ROOT_PROPERTY: u64 = 1;
pub(crate) const PHYSICAL_PROPERTY: u64 = 2;
pub(crate) const MANA_PROPERTY: u64 = 3;
pub(crate) const RESOLUTION_PROPERTY: u64 = 4;
pub(crate) const ACTOR_BODY_PROPERTY: u64 = 6;
pub(crate) const ACTOR_REJECTION_PROPERTY: u64 = 7;
pub(crate) const POPULATION_AGGREGATE_PROPERTY: u64 = 8;
pub(crate) const ACTOR_PROMOTION_PROPERTY: u64 = 9;
pub(crate) const MATERIAL_FLOW_PROPERTY: u64 = 10;
pub(crate) const MATERIAL_SURFACE_CONDITION_PROPERTY: u64 = 11;
pub(crate) const MATERIAL_SURFACE_MANA_GATE_PROPERTY: u64 = 12;
pub const EXPERIMENT_RECIPE_MANA_SOURCE_PROPERTY: u64 = 13;
pub const THERMAL_RESERVOIR_BUDGET_PROPERTY: u64 = 20;
pub const THERMAL_ENERGY_PROPERTY: u64 = 21;
pub const THERMAL_BATCH_SEQUENCE_PROPERTY: u64 = 22;
pub(crate) const RESOLUTION_CHANNEL: u64 = 1;
const PHYSICAL_DIGEST_DOMAIN: u64 = 0x5048_5953_4943_414C;
const HISTORY_DIGEST_DOMAIN: u64 = 0x4849_5354_4F52_595F;
const EXPERIMENT_DIGEST_DOMAIN: u64 = 0x4558_5045_5249_4D45;

/// Headless deterministic runtime for the first executable causal experiment.
pub struct Runtime {
    pub(crate) scheduler: Scheduler,
    pub(crate) state: Arc<Mutex<RuntimeState>>,
}

impl Runtime {
    pub fn new(config: RuntimeConfig) -> Result<Self, RuntimeError> {
        let config = config.validate()?;
        let state = Arc::new(Mutex::new(RuntimeState::new(&config)?));
        let mut scheduler = Scheduler::new(config.deterministic.clone());
        scheduler.register_system(
            Phase::Physics,
            Box::new(PhysicalPatternSystem::new(
                Arc::clone(&state),
                config.pattern_schedule,
            )),
        );
        scheduler.register_system(
            Phase::Mana,
            Box::new(ExperimentRecipeManaSourceSystem::new(Arc::clone(&state))),
        );
        scheduler.register_system(
            Phase::Mana,
            Box::new(ManaRuntimeSystem::new(
                Arc::clone(&state),
                config.mana_parameters,
            )),
        );
        scheduler.register_system(
            Phase::Mana,
            Box::new(ManaEffectsSystem::new(
                Arc::clone(&state),
                config.mana_parameters,
            )),
        );
        scheduler.register_system(
            Phase::Resolution,
            Box::new(ResolutionRuntimeSystem::new(Arc::clone(&state))),
        );
        scheduler.register_system(
            Phase::Perception,
            Box::new(ActorPerceptionSystem::new(Arc::clone(&state))),
        );
        scheduler.register_system(
            Phase::Cognition,
            Box::new(ActorCognitionSystem::new(Arc::clone(&state))),
        );
        scheduler.register_system(
            Phase::Action,
            Box::new(ActorActionSystem::new(Arc::clone(&state))),
        );
        scheduler.register_system(
            Phase::Lifecycle,
            Box::new(PopulationLifecycleSystem::new(Arc::clone(&state))),
        );
        scheduler.register_system(
            Phase::Physics,
            Box::new(ThermalReservoirSystem::new(Arc::clone(&state))),
        );
        scheduler.register_system(
            Phase::Physics,
            Box::new(ThermalEvolutionSystem::new(Arc::clone(&state))),
        );
        Ok(Self { scheduler, state })
    }

    pub fn from_seed(world_seed: u64) -> Result<Self, RuntimeError> {
        Self::new(RuntimeConfig::new(world_seed))
    }

    pub fn current_time(&self) -> SimulationTime {
        self.scheduler.current_time()
    }

    pub fn tick(&mut self) -> Result<RuntimeSnapshot, RuntimeError> {
        if self.scheduler.current_time().raw() >= MAX_RUNTIME_TICKS {
            return Err(RuntimeError::TickLimitExceeded);
        }
        self.scheduler.tick();
        let state = self.lock_state()?;
        if let Some(error) = state.failure.clone() {
            return Err(error);
        }
        if state.advanced_through != self.scheduler.current_time() {
            return Err(RuntimeError::PhaseDesynchronized);
        }
        Ok(state.snapshot(self.scheduler.current_time()))
    }

    pub fn run_ticks(&mut self, ticks: u64) -> Result<RuntimeSnapshot, RuntimeError> {
        if ticks == 0 || ticks > MAX_RUNTIME_TICKS.saturating_sub(self.current_time().raw()) {
            return Err(RuntimeError::InvalidTickCount { ticks });
        }
        let mut snapshot = self.snapshot()?;
        for _ in 0..ticks {
            snapshot = self.tick()?;
        }
        Ok(snapshot)
    }

    pub fn snapshot(&self) -> Result<RuntimeSnapshot, RuntimeError> {
        let state = self.lock_state()?;
        if let Some(error) = state.failure.clone() {
            return Err(error);
        }
        Ok(state.snapshot(self.scheduler.current_time()))
    }

    pub fn observer_world_snapshot(&self) -> Result<ObserverWorldSnapshot, RuntimeError> {
        let state = self.lock_state()?;
        if let Some(error) = state.failure.clone() {
            return Err(error);
        }
        Ok(state.observer_world_snapshot(self.scheduler.current_time()))
    }

    pub fn observer_material_surface_loop_explanation(
        &self,
    ) -> Result<ExplanationReport, RuntimeError> {
        let state = self.lock_state()?;
        if let Some(error) = state.failure.clone() {
            return Err(error);
        }
        state.material_surface_loop_explanation(self.scheduler.current_time(), None)
    }

    pub fn observer_material_surface_loop_explanation_for_surface(
        &self,
        surface: MaterialSurfaceId,
    ) -> Result<ExplanationReport, RuntimeError> {
        let state = self.lock_state()?;
        if let Some(error) = state.failure.clone() {
            return Err(error);
        }
        state.material_surface_loop_explanation(self.scheduler.current_time(), Some(surface))
    }

    pub fn observer_thermal_conservation_explanation(
        &self,
    ) -> Result<ExplanationReport, RuntimeError> {
        let state = self.lock_state()?;
        if let Some(error) = state.failure.clone() {
            return Err(error);
        }
        state.thermal_conservation_explanation(self.scheduler.current_time())
    }

    pub fn export_snapshot(&self) -> Result<RuntimeSnapshotData, RuntimeError> {
        let state = self.lock_state()?;
        if let Some(error) = state.failure.clone() {
            return Err(error);
        }
        if state.advanced_through != self.scheduler.current_time() {
            return Err(RuntimeError::PhaseDesynchronized);
        }
        Ok(state.export_snapshot())
    }

    pub fn executed_experiment_recipe_mana_sources(
        &self,
    ) -> Result<Vec<ExperimentRecipeManaSourceReceipt>, RuntimeError> {
        let state = self.lock_state()?;
        if let Some(error) = state.failure.clone() {
            return Err(error);
        }
        Ok(state.executed_experiment_recipe_mana_sources.clone())
    }

    /// Reconstruct a full `Runtime` from a completed-tick snapshot.
    pub fn from_snapshot(data: RuntimeSnapshotData) -> Result<Self, RuntimeError> {
        if data.recipe.system_registrations != runtime_system_registrations() {
            return Err(RuntimeError::InvalidSnapshot(
                "runtime system registrations do not match compiled registry",
            ));
        }
        let config = data.recipe.config.clone();
        let mut runtime = Self::new(config)?;
        runtime
            .scheduler
            .set_current_time(data.recipe.completed_time);
        runtime
            .scheduler
            .restore_system_times(data.recipe.completed_time.tick());
        let restored_state = RuntimeState::import_snapshot(data)?;
        *runtime.lock_state()? = restored_state;
        Ok(runtime)
    }

    pub fn import_snapshot(data: RuntimeSnapshotData) -> Result<RuntimeState, RuntimeError> {
        RuntimeState::import_snapshot(data)
    }

    fn lock_state(&self) -> Result<MutexGuard<'_, RuntimeState>, RuntimeError> {
        self.state.lock().map_err(|_| RuntimeError::StatePoisoned)
    }
}

impl Default for Runtime {
    fn default() -> Self {
        Self::from_seed(0).expect("default runtime configuration is valid")
    }
}

#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum RuntimeError {
    #[error("runtime field extent must be at least three")]
    InvalidFieldExtent,
    #[error("runtime active chunk radius is outside the bounded Stage 6 range")]
    InvalidActiveChunkRadius,
    #[error("physical pattern schedule is invalid")]
    InvalidPatternSchedule,
    #[error("actor runtime configuration is invalid")]
    InvalidActorConfig,
    #[error(
        "experiment recipe contains {count} mana sources, maximum is {MAX_EXPERIMENT_RECIPE_MANA_SOURCES}"
    )]
    ExperimentRecipeSourceCountExceeded { count: usize },
    #[error("experiment recipe source record ID must be nonzero: {source_record_id}")]
    InvalidExperimentRecipeSourceId { source_record_id: u64 },
    #[error("experiment recipe source record ID is duplicated: {source_record_id}")]
    DuplicateExperimentRecipeSourceId { source_record_id: u64 },
    #[error(
        "experiment recipe canonical key is duplicated at tick {scheduled_tick}, chunk {target_chunk:?}, cell {cell_index}"
    )]
    DuplicateExperimentRecipeCanonicalKey {
        scheduled_tick: u64,
        target_chunk: ChartChunkCoord,
        cell_index: u16,
    },
    #[error("experiment recipe source scheduled tick is invalid: {scheduled_tick}")]
    InvalidExperimentRecipeScheduledTick { scheduled_tick: u64 },
    #[error("experiment recipe source amount is negative: {amount}")]
    InvalidExperimentRecipeAmount { amount: i64 },
    #[error("experiment recipe source amount {amount} exceeds maximum {maximum}")]
    ExperimentRecipeAmountExceedsMaximum { amount: i64, maximum: i64 },
    #[error("experiment recipe source maximum is negative: {maximum}")]
    InvalidExperimentRecipeMaximum { maximum: i64 },
    #[error("experiment recipe budget is negative: {budget}")]
    InvalidExperimentRecipeBudget { budget: i64 },
    #[error(
        "enabled experiment recipe amount {enabled_amount} exceeds recipe budget {recipe_budget}"
    )]
    ExperimentRecipeBudgetExceeded {
        enabled_amount: i128,
        recipe_budget: i64,
    },
    #[error("experiment recipe policy schema is unsupported: {policy_schema_id}")]
    InvalidExperimentRecipePolicySchema { policy_schema_id: u64 },
    #[error(
        "experiment recipe source {source_record_id} targets chart {chart} instead of configured chart"
    )]
    InvalidExperimentRecipeTargetChart { source_record_id: u64, chart: u64 },
    #[error(
        "experiment recipe source {source_record_id} targets an inactive chunk {target_chunk:?}"
    )]
    InactiveExperimentRecipeTargetChunk {
        source_record_id: u64,
        target_chunk: ChartChunkCoord,
    },
    #[error(
        "experiment recipe source {source_record_id} cell {cell_index} is outside {cell_count} cells"
    )]
    InvalidExperimentRecipeCellIndex {
        source_record_id: u64,
        cell_index: u16,
        cell_count: u32,
    },
    #[error("population aggregate state is invalid")]
    InvalidPopulationAggregate,
    #[error("invalid runtime tick count: {ticks}")]
    InvalidTickCount { ticks: u64 },
    #[error("runtime tick limit exceeded")]
    TickLimitExceeded,
    #[error("runtime state lock was poisoned")]
    StatePoisoned,
    #[error("runtime systems did not advance through the scheduler time")]
    PhaseDesynchronized,
    #[error("causal effect construction failed: {0}")]
    CausalEffect(#[from] CausalEffectError),
    #[error("causal proposal construction failed: {0}")]
    CausalProposal(#[from] CausalEventProposalError),
    #[error("causal commit failed: {0}")]
    CausalCommit(#[from] CausalCommitError),
    #[error("mana physical effect refers to unknown cause {cause}")]
    UnknownManaPhysicalEffectCause { cause: TraceId },
    #[error("mana evolution failed: {0:?}")]
    Mana(ManaError),
    #[error("thermal active region is incomplete")]
    ThermalRegionIncomplete,
    #[error("thermal arithmetic failed")]
    ThermalArithmeticError,
    #[error("thermal conservation residual is non-zero")]
    ThermalConservationViolation,
    #[error("thermal evolution failed: {0}")]
    Thermal(ThermalError),
    #[error("resolution evolution failed: {0}")]
    Resolution(#[from] ResolutionError),
    #[error("actor initialization failed: {0}")]
    ActorInit(#[from] crate::ActorInitError),
    #[error("actor perception failed: {0}")]
    ActorPerception(#[from] causafera_perception::AcquisitionError),
    #[error("actor cognition failed: {0}")]
    ActorCognition(#[from] causafera_cognition::SceneUpdateError),
    #[error("snapshot is invalid: {0}")]
    InvalidSnapshot(&'static str),
}

impl From<ManaError> for RuntimeError {
    fn from(error: ManaError) -> Self {
        Self::Mana(error)
    }
}

impl From<ThermalError> for RuntimeError {
    fn from(error: ThermalError) -> Self {
        match error {
            ThermalError::ActiveRegionIncomplete(_) => Self::ThermalRegionIncomplete,
            ThermalError::ArithmeticOverflow | ThermalError::EnergyOutOfBounds => {
                Self::ThermalArithmeticError
            }
            ThermalError::ConservationViolation(_) => Self::ThermalConservationViolation,
            other => Self::Thermal(other),
        }
    }
}

pub struct RuntimeState {
    pub(crate) config: RuntimeConfig,
    pub(crate) traces: CausalTraceStore,
    pub(crate) mana: ManaFieldSet,
    pub(crate) thermal_fields: ThermalFieldSet,
    pub(crate) thermal_active_region: ThermalActiveRegion,
    pub(crate) thermal_boundary_records: Vec<ThermalBoundaryRecord>,
    pub(crate) thermal_reservoirs: BTreeMap<ThermalReservoirId, ThermalReservoir>,
    pub(crate) thermal_parameters: ThermalParameters,
    pub(crate) pending_thermal_injections: Vec<ThermalInjectionProposal>,
    pub(crate) thermal_receipts: BTreeMap<TraceId, Vec<ThermalCellTransferReceipt>>,
    pub(crate) thermal_conservation_receipts: BTreeMap<TraceId, ThermalConservationReceipt>,
    pub(crate) resolution: ResolutionField,
    pub(crate) resolution_policy: ResolutionPolicy,
    pub(crate) carrier_adapters: BTreeMap<ChartChunkCoord, TerrainCarrierAdapter>,
    pub(crate) active_chunks: BTreeMap<ChartChunkCoord, ActiveChunkState>,
    pub(crate) actors: BTreeMap<ActorId, ActorState>,
    pub(crate) actor_ancestry: BTreeMap<ActorId, Vec<TraceId>>,
    pub(crate) actor_objects: BTreeMap<u64, ActorPhysicalObject>,
    pub(crate) population_aggregates: BTreeMap<ChartChunkCoord, PopulationAggregate>,
    pub(crate) aggregate_actor_pool: BTreeMap<ChartChunkCoord, Vec<ActorId>>,
    pub(crate) actor_action_bounds: i64,
    pub(crate) pending_samples: Vec<PhysicalPatternSample>,
    pub(crate) pattern_history: PhysicalPatternHistory,
    pub(crate) material_surfaces: BTreeMap<MaterialSurfaceId, MaterialSurface>,
    pub(crate) pending_material_surface_changes: BTreeSet<MaterialSurfaceId>,
    pub(crate) material_surface_transitions: Vec<MaterialSurfaceTransition>,
    pub(crate) material_surface_gate_transitions: Vec<MaterialSurfaceGateTransition>,
    pub(crate) latest_physical_trace: TraceId,
    pub(crate) latest_mana_trace: Option<TraceId>,
    pub executed_experiment_recipe_mana_sources: Vec<ExperimentRecipeManaSourceReceipt>,
    pub(crate) advanced_through: SimulationTime,
    pub(crate) physical_events: u64,
    pub(crate) mana_cell_changes: u64,
    pub(crate) mana_physical_effects: u64,
    pub(crate) resolution_changes: u64,
    pub(crate) resolution_transitions: u64,
    pub(crate) perceived_actor_features: u64,
    pub(crate) subjective_actor_objects: u64,
    pub(crate) actor_actions_committed: u64,
    pub(crate) actor_actions_rejected: u64,
    pub(crate) population_births: u64,
    pub(crate) population_deaths: u64,
    pub(crate) population_movements: u64,
    pub(crate) actor_promotions: u64,
    pub(crate) actor_demotions: u64,
    pub(crate) material_activity_events: u64,
    pub(crate) next_actor_id: u64,
    pub(crate) last_mana_changes: u32,
    pub(crate) failure: Option<RuntimeError>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActiveChunkState {
    pub relevance: i64,
    pub level: u8,
    pub total_mana: i64,
    pub event_count: u64,
    pub last_transition: Option<TraceId>,
}

impl RuntimeState {
    pub(crate) fn new(config: &RuntimeConfig) -> Result<Self, RuntimeError> {
        let mut traces = CausalTraceStore::new();
        let root = CausalEventProposal::new(
            EventProposalKey::new(0, 0, 0),
            EventKindId::new(ROOT_EVENT_KIND),
            Vec::new(),
            vec![CausalEffect::new(
                CausalTarget::new(
                    StateObjectKindId::new(RUNTIME_OBJECT_KIND),
                    0,
                    StatePropertyId::new(ROOT_PROPERTY),
                ),
                fingerprint_u64(0x0101, 0),
                fingerprint_u64(0x0102, config.deterministic.world_seed),
            )?],
        )?;
        let root_trace =
            traces.commit_batch(SimulationTime::new(0), Phase::Physics, vec![root])?[0];
        let active_chunk_keys = active_chunk_keys(config.chart_id, config.active_chunk_radius);
        let carrier_adapters = runtime_carrier_adapters(
            config.carrier_adapter,
            config.chunk_extent,
            root_trace,
            &active_chunk_keys,
        );
        let mana = ManaFieldSet::new(
            active_chunk_keys
                .iter()
                .enumerate()
                .map(|(index, chunk)| {
                    ManaField::new(
                        ManaFieldId::new(index as u64 + 1),
                        *chunk,
                        config.chunk_extent,
                    )
                })
                .collect::<Result<Vec<_>, _>>()?,
        )?;
        validate_mana_cell_object_ids(&mana)?;
        let thermal_parameters = ThermalParameters::new(
            128,
            causafera_domains::THERMAL_SCALE,
            causafera_domains::THERMAL_SCALE,
        )?;
        let thermal_fields = ThermalFieldSet::new(
            active_chunk_keys
                .iter()
                .map(|chunk| ThermalField::new(*chunk, config.chunk_extent, root_trace))
                .collect::<Result<Vec<_>, _>>()?,
            root_trace,
        )?;
        let active_thermal_chunks = active_chunk_keys.iter().copied().collect::<BTreeSet<_>>();
        let thermal_active_region =
            ThermalActiveRegion::new(active_thermal_chunks.clone(), active_thermal_chunks)?;
        let resolution = ResolutionField::new(
            ResolutionFieldId::new(1),
            SimulationTime::new(0),
            active_chunk_keys.clone(),
            vec![root_trace; active_chunk_keys.len()],
        )?;
        let active_chunks = active_chunk_keys
            .iter()
            .map(|chunk| {
                (
                    *chunk,
                    ActiveChunkState {
                        relevance: 0,
                        level: 0,
                        total_mana: 0,
                        event_count: 0,
                        last_transition: None,
                    },
                )
            })
            .collect();
        let resolution_policy = ResolutionPolicy::new(
            10_000,
            900,
            100,
            vec![500, 2_000, 5_000],
            vec![ChannelWeight::new(
                ResolutionChannelId::new(RESOLUTION_CHANNEL),
                1_000,
            )?],
        )?;
        let mut state = Self {
            config: config.clone(),
            traces,
            mana,
            thermal_fields,
            thermal_active_region,
            thermal_boundary_records: Vec::new(),
            thermal_reservoirs: BTreeMap::new(),
            thermal_parameters,
            pending_thermal_injections: Vec::new(),
            thermal_receipts: BTreeMap::new(),
            thermal_conservation_receipts: BTreeMap::new(),
            resolution,
            resolution_policy,
            carrier_adapters,
            active_chunks,
            actors: BTreeMap::new(),
            actor_ancestry: BTreeMap::new(),
            actor_objects: BTreeMap::new(),
            population_aggregates: BTreeMap::new(),
            aggregate_actor_pool: BTreeMap::new(),
            actor_action_bounds: config.action_bounds,
            pending_samples: Vec::with_capacity(causafera_geography::TERRAIN_CELLS_PER_CHUNK),
            pattern_history: PhysicalPatternHistory::new(
                MAX_PATTERN_HISTORY_ENTRIES,
                MAX_PATTERN_HISTORY_PER_PATTERN,
            ),
            material_surfaces: BTreeMap::new(),
            pending_material_surface_changes: BTreeSet::new(),
            material_surface_transitions: Vec::new(),
            material_surface_gate_transitions: Vec::new(),
            latest_physical_trace: root_trace,
            latest_mana_trace: None,
            executed_experiment_recipe_mana_sources: Vec::new(),
            advanced_through: SimulationTime::new(0),
            physical_events: 0,
            mana_cell_changes: 0,
            mana_physical_effects: 0,
            resolution_changes: 0,
            resolution_transitions: 0,
            perceived_actor_features: 0,
            subjective_actor_objects: 0,
            actor_actions_committed: 0,
            actor_actions_rejected: 0,
            population_births: 0,
            population_deaths: 0,
            population_movements: 0,
            actor_promotions: 0,
            actor_demotions: 0,
            material_activity_events: 0,
            next_actor_id: 1,
            last_mana_changes: 0,
            failure: None,
        };
        HistoricalBootstrapPlan::for_runtime_config(config)
            .map_err(|error| match error {
                BootstrapError::Runtime(error) => error,
                BootstrapError::ThermalReservoirOutsideActiveRegion { .. }
                | BootstrapError::InvalidThermalReservoir
                | BootstrapError::DuplicateThermalReservoir { .. } => {
                    RuntimeError::InvalidSnapshot("invalid thermal bootstrap plan")
                }
            })?
            .bootstrap(&mut state)
            .map_err(|error| match error {
                BootstrapError::Runtime(error) => error,
                BootstrapError::ThermalReservoirOutsideActiveRegion { .. }
                | BootstrapError::InvalidThermalReservoir
                | BootstrapError::DuplicateThermalReservoir { .. } => {
                    RuntimeError::InvalidSnapshot("invalid thermal bootstrap state")
                }
            })?;
        Ok(state)
    }

    pub fn export_snapshot(&self) -> RuntimeSnapshotData {
        RuntimeSnapshotData {
            recipe: RuntimeRecipeSnapshot {
                seed: self.config.deterministic.world_seed,
                config: self.config.clone(),
                system_registrations: runtime_system_registrations(),
                completed_time: self.advanced_through,
            },
            spatial: SpatialChunkSnapshot {
                active_chunks: self
                    .active_chunks
                    .iter()
                    .map(|(chunk, state)| ActiveChunkSnapshot {
                        chunk: *chunk,
                        relevance: state.relevance,
                        level: state.level,
                        total_mana: state.total_mana,
                        event_count: state.event_count,
                        last_transition: state.last_transition,
                    })
                    .collect(),
                carrier_adapters: self
                    .carrier_adapters
                    .values()
                    .map(TerrainCarrierAdapter::export_snapshot)
                    .collect(),
            },
            mana: self.mana.export_snapshot(),
            thermal: ThermalSnapshot {
                parameters: self.thermal_parameters,
                field_set: ThermalFieldSetSnapshot {
                    fields: self
                        .thermal_fields
                        .fields()
                        .values()
                        .map(|field| ThermalFieldSnapshot {
                            chunk: field.chunk(),
                            extent: field.extent(),
                            energy: field.energy().iter().map(|energy| energy.get()).collect(),
                            last_change: field.last_change().to_vec(),
                            last_change_before: field
                                .last_change_before()
                                .iter()
                                .map(|energy| energy.get())
                                .collect(),
                        })
                        .collect(),
                    batch_sequence: self.thermal_fields.batch_sequence(),
                    conservation_last_change: self.thermal_fields.conservation_last_change(),
                },
                active_region: ThermalActiveRegionSnapshot {
                    active_chunks: self
                        .thermal_active_region
                        .active_chunks()
                        .iter()
                        .copied()
                        .collect(),
                    resident_chunks: self
                        .thermal_active_region
                        .resident_chunks()
                        .iter()
                        .copied()
                        .collect(),
                },
                reservoirs: self
                    .thermal_reservoirs
                    .values()
                    .map(|reservoir| ThermalReservoirSnapshot {
                        id: reservoir.id,
                        target: reservoir.target,
                        budget: reservoir.budget.get(),
                        schedule: match reservoir.schedule {
                            causafera_domains::ThermalReservoirSchedule::PerTick(amount) => {
                                ThermalReservoirScheduleSnapshot::PerTick(amount.get())
                            }
                            causafera_domains::ThermalReservoirSchedule::OneShot => {
                                ThermalReservoirScheduleSnapshot::OneShot
                            }
                        },
                        bootstrap_trace: reservoir.bootstrap_trace,
                        last_change: reservoir.last_change,
                    })
                    .collect(),
                receipt_batches: self.thermal_receipts.keys().copied().collect(),
                transfer_receipts: self
                    .thermal_receipts
                    .iter()
                    .flat_map(|(conservation_trace, receipts)| {
                        receipts
                            .iter()
                            .map(move |receipt| ThermalCellTransferReceiptSnapshot {
                                conservation_trace: *conservation_trace,
                                cell: receipt.cell,
                                pre_state: receipt.pre_state.get(),
                                post_state: receipt.post_state.get(),
                                cell_change_trace_id: receipt.cell_change_trace_id,
                                faces: receipt
                                    .faces
                                    .iter()
                                    .map(|face| ThermalFaceRecordSnapshot {
                                        neighbor: face.neighbor,
                                        signed_flux: face.signed_flux,
                                        neighbor_pre_state: face.neighbor_pre_state.get(),
                                    })
                                    .collect(),
                                reservoirs: receipt
                                    .reservoirs
                                    .iter()
                                    .map(|record| ThermalReservoirTransferRecordSnapshot {
                                        id: record.id,
                                        scheduled_injection: record.scheduled_injection.get(),
                                        accepted_injection: record.accepted_injection.get(),
                                        rejected_injection: record.rejected_injection.get(),
                                        transfer_trace_id: record.transfer_trace_id,
                                    })
                                    .collect(),
                            })
                    })
                    .collect(),
                conservation_receipts: self
                    .thermal_conservation_receipts
                    .iter()
                    .map(|(trace, receipt)| ThermalConservationReceiptSnapshot {
                        trace: *trace,
                        tick: receipt.tick,
                        total_cell_energy_before: receipt.total_cell_energy_before,
                        total_cell_energy_after: receipt.total_cell_energy_after,
                        total_reservoir_budget_before: receipt.total_reservoir_budget_before,
                        total_reservoir_budget_after: receipt.total_reservoir_budget_after,
                        residual: receipt.residual,
                    })
                    .collect(),
                boundary_records: self
                    .thermal_boundary_records
                    .iter()
                    .map(|record| ThermalBoundaryRecordSnapshot {
                        cell: record.cell,
                        neighbor: record.neighbor,
                        cell_pre_state: record.cell_pre_state.get(),
                    })
                    .collect(),
            },
            resolution: self.resolution.export_snapshot(),
            resolution_policy: self.resolution_policy.export_snapshot(),
            pattern_history: self.pattern_history.export_snapshot(),
            physical_counters: PhysicalCountersSnapshot {
                pending_samples: self.pending_samples.clone(),
                latest_physical_trace: self.latest_physical_trace,
                latest_mana_trace: self.latest_mana_trace,
                advanced_through: self.advanced_through,
                physical_events: self.physical_events,
                mana_cell_changes: self.mana_cell_changes,
                mana_physical_effects: self.mana_physical_effects,
                resolution_changes: self.resolution_changes,
                resolution_transitions: self.resolution_transitions,
                perceived_actor_features: self.perceived_actor_features,
                subjective_actor_objects: self.subjective_actor_objects,
                actor_actions_committed: self.actor_actions_committed,
                actor_actions_rejected: self.actor_actions_rejected,
                population_births: self.population_births,
                population_deaths: self.population_deaths,
                population_movements: self.population_movements,
                actor_promotions: self.actor_promotions,
                actor_demotions: self.actor_demotions,
                material_activity_events: self.material_activity_events,
                next_actor_id: self.next_actor_id,
                last_mana_changes: self.last_mana_changes,
            },
            material_surfaces: MaterialSurfaceSnapshot {
                records: self
                    .material_surfaces
                    .iter()
                    .map(|(id, surface)| MaterialSurfaceRecordSnapshot {
                        id: *id,
                        surface: *surface,
                    })
                    .collect(),
                pending_physical_changes: self
                    .pending_material_surface_changes
                    .iter()
                    .copied()
                    .collect(),
                transitions: self.material_surface_transitions.clone(),
                gate_transitions: self.material_surface_gate_transitions.clone(),
            },
            actors_objective: ActorObjectiveStateSnapshot {
                actors: self
                    .actors
                    .iter()
                    .map(|(id, actor)| (*id, actor.export_objective_snapshot()))
                    .collect(),
                actor_ancestry: self
                    .actor_ancestry
                    .iter()
                    .map(|(id, ancestry)| (*id, ancestry.clone()))
                    .collect(),
                actor_objects: self
                    .actor_objects
                    .iter()
                    .map(|(key, object)| (*key, *object))
                    .collect(),
                actor_action_bounds: self.actor_action_bounds,
            },
            actors_subjective: ActorSubjectiveStateSnapshot {
                actors: self
                    .actors
                    .iter()
                    .map(|(id, actor)| (*id, actor.export_subjective_snapshot()))
                    .collect(),
            },
            population: PopulationAggregateSnapshot {
                aggregates: self.population_aggregates.values().cloned().collect(),
                aggregate_actor_pool: self
                    .aggregate_actor_pool
                    .iter()
                    .map(|(chunk, actors)| (*chunk, actors.clone()))
                    .collect(),
            },
            bootstrap: BootstrapReceiptSnapshot {
                receipts: Vec::new(),
            },
            traces: self.traces.export_snapshot(),
            experiment_recipe_mana_source_receipts: self
                .executed_experiment_recipe_mana_sources
                .iter()
                .map(|receipt| ExperimentRecipeManaSourceReceiptSnapshot {
                    source_record_id: receipt.source_record_id,
                    scheduled_tick: receipt.scheduled_tick,
                    executed_tick: receipt.executed_tick,
                    source_trace: receipt.source_trace,
                    before_intensity: receipt.before_intensity,
                    after_intensity: receipt.after_intensity,
                    recipe_hash: receipt.recipe_hash,
                    policy_schema_id: receipt.policy_schema_id,
                })
                .collect(),
            experiment_manifest: None,
        }
    }

    pub fn import_snapshot(data: RuntimeSnapshotData) -> Result<Self, RuntimeError> {
        let config = data.recipe.config.validate()?;
        let imported_receipts = import_experiment_recipe_mana_source_receipts(
            data.experiment_recipe_mana_source_receipts,
        )?;
        let traces = CausalTraceStore::import_snapshot(data.traces)
            .map_err(|_| RuntimeError::InvalidSnapshot("trace store failed validation"))?;
        let mana = ManaFieldSet::import_snapshot(data.mana)?;
        validate_mana_cell_object_ids(&mana)?;
        let resolution = ResolutionField::import_snapshot(data.resolution)?;
        let resolution_policy = ResolutionPolicy::import_snapshot(data.resolution_policy)?;
        let carrier_adapters = import_carrier_adapters(data.spatial.carrier_adapters)?;
        let active_chunks = import_active_chunks(data.spatial.active_chunks)?;
        let thermal_parameters = data.thermal.parameters;
        let (
            thermal_fields,
            thermal_active_region,
            thermal_boundary_records,
            thermal_reservoirs,
            thermal_receipts,
            thermal_conservation_receipts,
        ) = import_thermal_snapshot(data.thermal, config.chunk_extent, &active_chunks)?;
        let pattern_history = PhysicalPatternHistory::import_snapshot(data.pattern_history);
        let subjective_count = data.actors_subjective.actors.len();
        let subjective_by_actor = data
            .actors_subjective
            .actors
            .into_iter()
            .collect::<BTreeMap<_, _>>();
        if subjective_by_actor.len() != subjective_count {
            return Err(RuntimeError::InvalidSnapshot(
                "duplicate actor subjective id",
            ));
        }
        let mut actors = BTreeMap::new();
        for (actor_id, objective) in data.actors_objective.actors {
            let subjective = subjective_by_actor.get(&actor_id).cloned().ok_or(
                RuntimeError::InvalidSnapshot("missing actor subjective state"),
            )?;
            if actors
                .insert(
                    actor_id,
                    ActorState::import_snapshots(objective, subjective)?,
                )
                .is_some()
            {
                return Err(RuntimeError::InvalidSnapshot("duplicate actor id"));
            }
        }
        if actors.len() != subjective_by_actor.len() {
            return Err(RuntimeError::InvalidSnapshot(
                "orphan actor subjective state",
            ));
        }
        let actor_ancestry = import_actor_ancestry(data.actors_objective.actor_ancestry)?;
        let actor_objects = import_actor_objects(data.actors_objective.actor_objects)?;
        let population_aggregates = import_population_aggregates(data.population.aggregates)?;
        let aggregate_actor_pool =
            import_aggregate_actor_pool(data.population.aggregate_actor_pool)?;
        let (
            material_surfaces,
            pending_material_surface_changes,
            material_surface_transitions,
            material_surface_gate_transitions,
        ) = import_material_surfaces(data.material_surfaces, config.chunk_extent)?;
        let counters = data.physical_counters;
        let state = Self {
            config,
            traces,
            mana,
            thermal_fields,
            thermal_active_region,
            thermal_boundary_records,
            thermal_reservoirs,
            thermal_parameters,
            pending_thermal_injections: Vec::new(),
            thermal_receipts,
            thermal_conservation_receipts,
            resolution,
            resolution_policy,
            carrier_adapters,
            active_chunks,
            actors,
            actor_ancestry,
            actor_objects,
            population_aggregates,
            aggregate_actor_pool,
            actor_action_bounds: data.actors_objective.actor_action_bounds,
            pending_samples: counters.pending_samples,
            pattern_history,
            material_surfaces,
            pending_material_surface_changes,
            material_surface_transitions,
            material_surface_gate_transitions,
            latest_physical_trace: counters.latest_physical_trace,
            latest_mana_trace: counters.latest_mana_trace,
            executed_experiment_recipe_mana_sources: imported_receipts,
            advanced_through: counters.advanced_through,
            physical_events: counters.physical_events,
            mana_cell_changes: counters.mana_cell_changes,
            mana_physical_effects: counters.mana_physical_effects,
            resolution_changes: counters.resolution_changes,
            resolution_transitions: counters.resolution_transitions,
            perceived_actor_features: counters.perceived_actor_features,
            subjective_actor_objects: counters.subjective_actor_objects,
            actor_actions_committed: counters.actor_actions_committed,
            actor_actions_rejected: counters.actor_actions_rejected,
            population_births: counters.population_births,
            population_deaths: counters.population_deaths,
            population_movements: counters.population_movements,
            actor_promotions: counters.actor_promotions,
            actor_demotions: counters.actor_demotions,
            material_activity_events: counters.material_activity_events,
            next_actor_id: counters.next_actor_id,
            last_mana_changes: counters.last_mana_changes,
            failure: None,
        };
        state.validate_snapshot_references()?;
        Ok(state)
    }

    fn validate_snapshot_references(&self) -> Result<(), RuntimeError> {
        self.validate_experiment_recipe_mana_source_receipts()?;
        validate_trace_exists(&self.traces, self.latest_physical_trace)?;
        if let Some(trace) = self.latest_mana_trace {
            validate_trace_exists(&self.traces, trace)?;
        }
        for sample in self
            .pattern_history
            .samples()
            .chain(self.pending_samples.iter())
        {
            validate_trace_exists(&self.traces, sample.cause)?;
        }
        for (id, surface) in &self.material_surfaces {
            validate_material_surface_last_transition(&self.traces, *id, *surface)?;
            validate_material_surface_last_contact_trace(&self.traces, *id, *surface)?;
            validate_material_surface_gate_state(&self.traces, *id, *surface)?;
            if !self.active_chunks.contains_key(&id.chunk) {
                return Err(RuntimeError::InvalidSnapshot(
                    "material surface outside active chunks",
                ));
            }
        }
        for id in &self.pending_material_surface_changes {
            if !self.material_surfaces.contains_key(id) {
                return Err(RuntimeError::InvalidSnapshot(
                    "missing changed material surface",
                ));
            }
        }
        for transition in &self.material_surface_transitions {
            validate_material_surface_transition(&self.traces, transition)?;
        }
        validate_material_surface_gate_transition_history(
            &self.traces,
            &self.material_surfaces,
            &self.material_surface_transitions,
            &self.material_surface_gate_transitions,
        )?;
        for transition in &self.material_surface_gate_transitions {
            validate_material_surface_gate_transition(
                &self.traces,
                &self.material_surface_transitions,
                transition,
            )?;
        }
        for field in self.mana.fields().values() {
            for trace in field.last_change().iter().flatten().copied() {
                validate_trace_exists(&self.traces, trace)?;
            }
        }
        for field in self.thermal_fields.fields().values() {
            for trace in field.last_change() {
                let event = self
                    .traces
                    .event(*trace)
                    .ok_or(RuntimeError::InvalidSnapshot(
                        "thermal field references unknown trace",
                    ))?;
                if !matches!(
                    event.kind.raw(),
                    THERMAL_FIELD_BOOTSTRAP_EVENT_KIND
                        | THERMAL_CELL_CHANGE_EVENT_KIND
                        | THERMAL_RESERVOIR_TRANSFER_EVENT_KIND
                ) {
                    return Err(RuntimeError::InvalidSnapshot(
                        "thermal field has mismatched trace anchor",
                    ));
                }
            }
        }
        let conservation_event = self
            .traces
            .event(self.thermal_fields.conservation_last_change())
            .ok_or(RuntimeError::InvalidSnapshot(
                "thermal conservation anchor references unknown trace",
            ))?;
        let expected_conservation_kind = if self.thermal_fields.batch_sequence() == 0 {
            THERMAL_FIELD_BOOTSTRAP_EVENT_KIND
        } else {
            THERMAL_CONSERVATION_EVENT_KIND
        };
        if conservation_event.kind.raw() != expected_conservation_kind {
            return Err(RuntimeError::InvalidSnapshot(
                "thermal conservation anchor has mismatched event kind",
            ));
        }
        for reservoir in self.thermal_reservoirs.values() {
            let bootstrap = self.traces.event(reservoir.bootstrap_trace).ok_or(
                RuntimeError::InvalidSnapshot("thermal reservoir bootstrap trace is unknown"),
            )?;
            if bootstrap.phase != Phase::Lifecycle
                || bootstrap.kind.raw() != THERMAL_RESERVOIR_BOOTSTRAP_EVENT_KIND
            {
                return Err(RuntimeError::InvalidSnapshot(
                    "thermal reservoir bootstrap anchor is invalid",
                ));
            }
            let last_change =
                self.traces
                    .event(reservoir.last_change)
                    .ok_or(RuntimeError::InvalidSnapshot(
                        "thermal reservoir last-change trace is unknown",
                    ))?;
            if !matches!(
                last_change.kind.raw(),
                THERMAL_RESERVOIR_BOOTSTRAP_EVENT_KIND | THERMAL_RESERVOIR_TRANSFER_EVENT_KIND
            ) {
                return Err(RuntimeError::InvalidSnapshot(
                    "thermal reservoir has mismatched trace anchor",
                ));
            }
        }
        for (trace, receipt) in &self.thermal_conservation_receipts {
            let event = self
                .traces
                .event(*trace)
                .ok_or(RuntimeError::InvalidSnapshot(
                    "thermal conservation receipt references unknown trace",
                ))?;
            if event.phase != Phase::Physics || event.kind.raw() != THERMAL_CONSERVATION_EVENT_KIND
            {
                return Err(RuntimeError::InvalidSnapshot(
                    "thermal conservation receipt has mismatched trace anchor",
                ));
            }
            if receipt.residual != 0 {
                return Err(RuntimeError::InvalidSnapshot(
                    "thermal conservation receipt has non-zero residual",
                ));
            }
        }
        for receipts in self.thermal_receipts.values() {
            for receipt in receipts {
                if let Some(trace) = receipt.cell_change_trace_id {
                    validate_trace_exists(&self.traces, trace)?;
                }
                for reservoir in &receipt.reservoirs {
                    if let Some(trace) = reservoir.transfer_trace_id {
                        validate_trace_exists(&self.traces, trace)?;
                    }
                }
            }
        }
        for entry in self.resolution.entries() {
            validate_trace_exists(&self.traces, entry.last_trace)?;
        }
        for active in self.active_chunks.values() {
            if let Some(trace) = active.last_transition {
                validate_trace_exists(&self.traces, trace)?;
            }
        }
        for ancestry in self.actor_ancestry.values() {
            for trace in ancestry {
                validate_trace_exists(&self.traces, *trace)?;
            }
        }
        for object in self.actor_objects.values() {
            validate_trace_exists(&self.traces, object.trace)?;
        }
        for aggregate in self.population_aggregates.values() {
            for trace in &aggregate.causal_ancestry {
                validate_trace_exists(&self.traces, *trace)?;
            }
        }
        Ok(())
    }

    fn validate_experiment_recipe_mana_source_receipts(&self) -> Result<(), RuntimeError> {
        let recipe_hash = self.config.experiment_recipe_mana_sources.recipe_hash();
        for receipt in &self.executed_experiment_recipe_mana_sources {
            let record = self
                .config
                .experiment_recipe_mana_sources
                .records
                .iter()
                .find(|record| record.source_record_id == receipt.source_record_id)
                .ok_or(RuntimeError::InvalidSnapshot(
                    "source receipt references unknown source record",
                ))?;
            if !record.enabled || record.amount <= 0 {
                return Err(RuntimeError::InvalidSnapshot(
                    "source receipt references disabled or zero source record",
                ));
            }
            if receipt.scheduled_tick != record.scheduled_tick {
                return Err(RuntimeError::InvalidSnapshot(
                    "source receipt scheduled tick does not match source record",
                ));
            }
            if receipt.executed_tick != receipt.scheduled_tick {
                return Err(RuntimeError::InvalidSnapshot(
                    "source receipt executed tick does not match scheduled tick",
                ));
            }
            if receipt.recipe_hash != recipe_hash {
                return Err(RuntimeError::InvalidSnapshot(
                    "source receipt recipe hash does not match recipe",
                ));
            }
            if receipt.policy_schema_id != record.policy_schema_id {
                return Err(RuntimeError::InvalidSnapshot(
                    "source receipt policy schema does not match source record",
                ));
            }
            let event =
                self.traces
                    .event(receipt.source_trace)
                    .ok_or(RuntimeError::InvalidSnapshot(
                        "source receipt references unknown source trace",
                    ))?;
            if event.kind != EventKindId::new(EXPERIMENT_RECIPE_MANA_SOURCE_EVENT_KIND)
                || event.phase != Phase::Mana
                || !event.causes.is_empty()
                || event.time != SimulationTime::new(receipt.executed_tick)
            {
                return Err(RuntimeError::InvalidSnapshot(
                    "source receipt source trace is not a root source event",
                ));
            }
            let expected_before = fingerprint_i64(0x0302, receipt.before_intensity);
            let expected_after = fingerprint_i64(0x0302, receipt.after_intensity);
            let cell_id = cell_object_id(record.target_chunk, record.cell_index);
            if !event.effects.iter().any(|effect| {
                effect.target().object_kind()
                    == StateObjectKindId::new(EXPERIMENT_RECIPE_MANA_SOURCE_OBJECT_KIND)
                    && effect.target().object_id() == cell_id
                    && effect.target().property()
                        == StatePropertyId::new(EXPERIMENT_RECIPE_MANA_SOURCE_PROPERTY)
                    && effect.before() == expected_before
                    && effect.after() == expected_after
            }) {
                return Err(RuntimeError::InvalidSnapshot(
                    "source receipt source trace does not match cell transition",
                ));
            }
        }
        Ok(())
    }

    pub(crate) fn snapshot(&self, time: SimulationTime) -> RuntimeSnapshot {
        let mana_total = self.mana.total_intensity();
        let mana_maximum = self.mana.maximum_intensity();
        let (resolution_relevance, resolution_level) = self
            .active_chunks
            .values()
            .max_by_key(|state| (state.level, state.relevance))
            .map(|state| (state.relevance, state.level))
            .unwrap_or((0, 0));
        let latest_trace = self
            .traces
            .iter()
            .last()
            .expect("runtime always retains a root trace")
            .trace_id;
        let physical_state_digest = self.physical_state_digest(time);
        let history_digest = self.history_digest();
        let population_total = self
            .population_aggregates
            .values()
            .map(|aggregate| aggregate.count)
            .sum();
        let thermal_total_cell_energy = self
            .thermal_fields
            .fields()
            .values()
            .flat_map(|field| field.energy())
            .map(|energy| i128::from(energy.get()))
            .sum();
        let thermal_total_reservoir_budget = self
            .thermal_reservoirs
            .values()
            .map(|reservoir| i128::from(reservoir.budget.get()))
            .sum();
        let thermal_active_chunk_count =
            u32::try_from(self.thermal_active_region.active_chunks().len()).unwrap_or(u32::MAX);
        let thermal_active_cell_count =
            self.thermal_fields
                .fields()
                .values()
                .fold(0_u32, |count, field| {
                    count.saturating_add(u32::try_from(field.energy().len()).unwrap_or(u32::MAX))
                });
        RuntimeSnapshot {
            time,
            physical_state_digest,
            history_digest,
            canonical_state: RuntimeState::canonical_state(physical_state_digest, history_digest),
            mana_total,
            mana_maximum,
            mana_changed_components: self.last_mana_changes,
            active_chunk_count: self.active_chunks.len() as u32,
            resolution_relevance,
            resolution_level,
            causal_trace_count: self.traces.len() as u64,
            physical_events: self.physical_events,
            mana_cell_changes: self.mana_cell_changes,
            mana_physical_effects: self.mana_physical_effects,
            resolution_changes: self.resolution_changes,
            resolution_transitions: self.resolution_transitions,
            actor_count: self.actors.len() as u32,
            perceived_actor_features: self.perceived_actor_features,
            subjective_actor_objects: self.subjective_actor_objects,
            actor_actions_committed: self.actor_actions_committed,
            actor_actions_rejected: self.actor_actions_rejected,
            population_total,
            population_births: self.population_births,
            population_deaths: self.population_deaths,
            population_movements: self.population_movements,
            actor_promotions: self.actor_promotions,
            actor_demotions: self.actor_demotions,
            material_activity_events: self.material_activity_events,
            bytes_per_chunk: self.bytes_per_chunk(),
            latest_trace,
            thermal_total_cell_energy,
            thermal_total_reservoir_budget,
            thermal_active_chunk_count,
            thermal_active_cell_count,
        }
    }

    pub(crate) fn observer_world_snapshot(&self, time: SimulationTime) -> ObserverWorldSnapshot {
        let chunks = self
            .active_chunks
            .iter()
            .map(|(chart_chunk, active)| {
                let terrain = self
                    .carrier_adapters
                    .get(chart_chunk)
                    .map(TerrainCarrierAdapter::export_snapshot);
                let (minimum_elevation_mm, maximum_elevation_mm, mean_roughness_mm) = terrain
                    .as_ref()
                    .map(|terrain| {
                        let minimum = terrain.elevations_mm.iter().copied().min().unwrap_or(0);
                        let maximum = terrain.elevations_mm.iter().copied().max().unwrap_or(0);
                        let roughness = if terrain.roughness_mm.is_empty() {
                            0
                        } else {
                            (terrain
                                .roughness_mm
                                .iter()
                                .copied()
                                .map(u64::from)
                                .sum::<u64>()
                                / terrain.roughness_mm.len() as u64)
                                as u32
                        };
                        (minimum, maximum, roughness)
                    })
                    .unwrap_or((0, 0, 0));
                let population_total = self
                    .population_aggregates
                    .get(chart_chunk)
                    .map(|aggregate| aggregate.count)
                    .unwrap_or(0);
                let latest_trace = active
                    .last_transition
                    .or_else(|| terrain.as_ref().map(|terrain| terrain.generation_trace))
                    .unwrap_or(self.latest_physical_trace);
                ObserverChunkSummary {
                    chart_id: chart_chunk.chart.raw(),
                    chunk_x: chart_chunk.chunk.x,
                    chunk_y: chart_chunk.chunk.y,
                    chunk_z: chart_chunk.chunk.z,
                    minimum_elevation_mm,
                    maximum_elevation_mm,
                    mean_roughness_mm,
                    mana_total: active.total_mana,
                    resolution_relevance: active.relevance,
                    resolution_level: u32::from(active.level),
                    population_total,
                    causal_event_count: active.event_count,
                    latest_trace,
                }
            })
            .collect();
        let latest_mana_transition = self
            .material_surface_transitions
            .iter()
            .rev()
            .find(|transition| transition.mana_effect_trace.is_some())
            .copied();
        let mut material_surface_transitions = self
            .material_surface_transitions
            .iter()
            .rev()
            .take(MAX_MATERIAL_SURFACE_DELTAS)
            .copied()
            .collect::<Vec<_>>();
        if let Some(mana_transition) = latest_mana_transition {
            if !material_surface_transitions
                .iter()
                .any(|transition| transition.transition_trace == mana_transition.transition_trace)
            {
                material_surface_transitions.pop();
                material_surface_transitions.push(mana_transition);
            }
        }
        material_surface_transitions
            .sort_by_key(|transition| (transition.id, transition.transition_trace));
        let material_surface_deltas = material_surface_transitions
            .into_iter()
            .map(|transition| {
                let (mana_transition_trace, mana_before, mana_after) =
                    material_surface_mana_transition_evidence(
                        &self.traces,
                        &self.executed_experiment_recipe_mana_sources,
                        &transition,
                    );
                let local_gate_transition = self
                    .material_surface_gate_transitions
                    .iter()
                    .find(|gate| gate.transition_trace == transition.transition_trace);
                MaterialSurfaceDelta {
                    chart_id: transition.id.chunk.chart.raw(),
                    chunk_x: transition.id.chunk.chunk.x,
                    chunk_y: transition.id.chunk.chunk.y,
                    chunk_z: transition.id.chunk.chunk.z,
                    cell_ordinal: transition.id.cell_index,
                    before_condition: transition.before_condition,
                    after_condition: transition.after_condition,
                    mana_total: transition.mana_total,
                    contact_trace: transition.contact_trace,
                    mana_effect_trace: transition.mana_effect_trace,
                    transition_tick: transition.occurred_at.raw(),
                    mana_transition_trace,
                    mana_before,
                    mana_after,
                    local_mana_before: local_gate_transition.map(|gate| gate.local_mana_before),
                    local_mana_after: local_gate_transition.map(|gate| gate.local_mana_after),
                    local_mana_transition_trace_id: local_gate_transition
                        .map(|gate| gate.local_mana_trace),
                }
            })
            .collect::<Vec<_>>();
        let material_surface_gate_deltas = self
            .material_surface_gate_transitions
            .iter()
            .rev()
            .filter(|transition| !transition.after_active)
            .take(MAX_MATERIAL_SURFACE_DELTAS)
            .copied()
            .collect::<Vec<_>>()
            .into_iter()
            .map(|transition| MaterialSurfaceGateDelta {
                chart_id: transition.id.chunk.chart.raw(),
                chunk_x: transition.id.chunk.chunk.x,
                chunk_y: transition.id.chunk.chunk.y,
                chunk_z: transition.id.chunk.chunk.z,
                cell_ordinal: transition.id.cell_index,
                before_active: transition.before_active,
                after_active: transition.after_active,
                local_mana_before: transition.local_mana_before,
                local_mana_after: transition.local_mana_after,
                local_mana_transition_trace_id: transition.local_mana_trace,
                gate_transition_trace_id: transition.transition_trace,
                contact_trace_id: transition.contact_trace,
                transition_tick: transition.occurred_at.raw(),
            })
            .collect::<Vec<_>>();
        let mut thermal_deltas = self
            .thermal_receipts
            .iter()
            .next_back()
            .map(|(_, receipts)| receipts.iter())
            .into_iter()
            .flatten()
            .take(MAX_THERMAL_DELTAS)
            .map(|receipt| ThermalFieldDelta {
                chart_id: receipt.cell.chunk.chart.raw(),
                chunk_x: receipt.cell.chunk.chunk.x,
                chunk_y: receipt.cell.chunk.chunk.y,
                chunk_z: receipt.cell.chunk.chunk.z,
                cell_ordinal: receipt.cell.cell_index,
                pre_state_energy: receipt.pre_state.get(),
                post_state_energy: receipt.post_state.get(),
                reservoir_scheduled_injection: receipt
                    .reservoirs
                    .iter()
                    .fold(0_i64, |total, record| {
                        total.saturating_add(record.scheduled_injection.get())
                    }),
                reservoir_accepted_injection: receipt
                    .reservoirs
                    .iter()
                    .fold(0_i64, |total, record| {
                        total.saturating_add(record.accepted_injection.get())
                    }),
                reservoir_rejected_injection: receipt
                    .reservoirs
                    .iter()
                    .fold(0_i64, |total, record| {
                        total.saturating_add(record.rejected_injection.get())
                    }),
                net_face_flux: receipt
                    .faces
                    .iter()
                    .fold(0_i64, |total, face| total.saturating_add(face.signed_flux)),
                face_count: u32::try_from(receipt.faces.len()).unwrap_or(u32::MAX),
            })
            .collect::<Vec<_>>();
        thermal_deltas.sort_by_key(|delta| {
            (
                delta.chart_id,
                delta.chunk_x,
                delta.chunk_y,
                delta.chunk_z,
                delta.cell_ordinal,
            )
        });
        ObserverWorldSnapshot {
            time,
            chunks,
            material_surface_delta_schema_version: if material_surface_deltas.is_empty()
                && material_surface_gate_deltas.is_empty()
            {
                0
            } else {
                MATERIAL_SURFACE_DELTA_SCHEMA_V3
            },
            material_surface_deltas,
            material_surface_gate_deltas,
            thermal_delta_schema_version: if thermal_deltas.is_empty() {
                0
            } else {
                THERMAL_DELTA_SCHEMA_V1
            },
            thermal_deltas,
        }
    }

    fn thermal_conservation_explanation(
        &self,
        time: SimulationTime,
    ) -> Result<ExplanationReport, RuntimeError> {
        let (conservation_trace, receipt) = self
            .thermal_conservation_receipts
            .iter()
            .next_back()
            .ok_or(RuntimeError::InvalidSnapshot(
            "missing thermal conservation receipt",
        ))?;
        let transfer_receipts =
            self.thermal_receipts
                .get(conservation_trace)
                .ok_or(RuntimeError::InvalidSnapshot(
                    "missing thermal transfer receipts",
                ))?;
        let mut reservoir_transfer_traces = BTreeSet::new();
        let mut neighbor_transfer_traces = BTreeSet::new();
        for transfer_receipt in transfer_receipts {
            if let Some(trace) = transfer_receipt.cell_change_trace_id {
                neighbor_transfer_traces.insert(trace);
            }
            for reservoir in &transfer_receipt.reservoirs {
                if let Some(trace) = reservoir.transfer_trace_id {
                    reservoir_transfer_traces.insert(trace);
                }
            }
        }
        let claim = ThermalCarrierConservationClaim {
            receipt: *receipt,
            observation_start: SimulationTime::new(receipt.tick),
            observation_end: time,
            conservation_trace: *conservation_trace,
            reservoir_transfer_traces: reservoir_transfer_traces.into_iter().collect(),
            neighbor_transfer_traces: neighbor_transfer_traces.into_iter().collect(),
        }
        .to_explanation_claim()
        .map_err(|_| {
            RuntimeError::InvalidSnapshot("invalid thermal conservation Explanation claim")
        })?;
        let frame = ExplanationFrame::new(time, vec![claim]).map_err(|_| {
            RuntimeError::InvalidSnapshot("invalid thermal conservation Explanation frame")
        })?;
        ExplanationReport::new(
            ExperimentId::new(self.config.deterministic.world_seed),
            vec![frame],
        )
        .map_err(|_| {
            RuntimeError::InvalidSnapshot("invalid thermal conservation Explanation report")
        })
    }

    fn material_surface_loop_explanation(
        &self,
        time: SimulationTime,
        surface: Option<MaterialSurfaceId>,
    ) -> Result<ExplanationReport, RuntimeError> {
        let explicit_surface = surface;
        let scoped_surface = surface.or_else(|| {
            self.material_surface_gate_transitions
                .last()
                .map(|transition| transition.id)
        });
        let transition = self
            .material_surface_transitions
            .iter()
            .rev()
            .filter(|transition| scoped_surface.is_none_or(|id| transition.id == id))
            .find(|transition| transition.mana_effect_trace.is_some())
            .or_else(|| {
                self.material_surface_transitions
                    .iter()
                    .rev()
                    .filter(|transition| scoped_surface.is_none_or(|id| transition.id == id))
                    .find(|transition| transition.contact_trace.is_some())
            })
            .or_else(|| {
                scoped_surface.and_then(|id| {
                    self.material_surface_transitions
                        .iter()
                        .rev()
                        .find(|transition| transition.id == id)
                })
            })
            .or_else(|| {
                explicit_surface
                    .is_none()
                    .then(|| self.material_surface_transitions.last())
                    .flatten()
            })
            .copied()
            .ok_or(RuntimeError::InvalidSnapshot(
                "missing material surface transition history",
            ))?;
        let observation_start = self
            .material_surface_transitions
            .first()
            .map(|transition| transition.occurred_at)
            .unwrap_or(time);
        let repeated_structure_observed = self.pattern_history.samples().count() >= 2;
        let Some(contact_trace) = transition.contact_trace else {
            let claim = ExplanationClaim::unknown(
                MATERIAL_SURFACE_LOOP_CLAIM_SCHEMA,
                NumericClaimValue::range(transition.before_condition, transition.after_condition)
                    .map_err(|_| {
                    RuntimeError::InvalidSnapshot("invalid material surface Explanation claim")
                })?,
                ComparisonContext::None,
            )
            .map_err(|_| {
                RuntimeError::InvalidSnapshot("invalid material surface Explanation claim")
            })?;
            let local_claim = MaterialSurfaceLocalManaTransitionClaim::unknown(
                transition.mana_total,
            )
            .map_err(|_| RuntimeError::InvalidSnapshot("invalid local mana Explanation claim"))?;
            let frame = ExplanationFrame::new(time, vec![claim, local_claim]).map_err(|_| {
                RuntimeError::InvalidSnapshot("invalid material surface Explanation frame")
            })?;
            return ExplanationReport::new(
                ExperimentId::new(self.config.deterministic.world_seed),
                vec![frame],
            )
            .map_err(|_| {
                RuntimeError::InvalidSnapshot("invalid material surface Explanation report")
            });
        };
        let (mana_transition_trace, mana_before, mana_after) =
            material_surface_mana_transition_evidence(
                &self.traces,
                &self.executed_experiment_recipe_mana_sources,
                &transition,
            );
        let claim = MaterialSurfaceLoopClaim {
            before_condition: transition.before_condition,
            after_condition: transition.after_condition,
            mana_total: transition.mana_total,
            observation_start,
            observation_end: time,
            contact_trace,
            mana_effect_trace: transition.mana_effect_trace,
            mana_transition_trace,
            mana_before,
            mana_after,
            repeated_structure_observed,
        };
        let mut claims = claim.to_explanation_claims().map_err(|_| {
            RuntimeError::InvalidSnapshot("invalid material surface Explanation claim")
        })?;
        let local_claim = self
            .material_surface_gate_transitions
            .iter()
            .rev()
            .find(|gate| gate.id == transition.id)
            .map(|gate| {
                MaterialSurfaceLocalManaTransitionClaim {
                    local_mana_before: gate.local_mana_before,
                    local_mana_after: gate.local_mana_after,
                    local_mana_trace: gate.local_mana_trace,
                    gate_transition_trace: gate.transition_trace,
                    contact_trace: gate.contact_trace,
                }
                .to_explanation_claim()
            })
            .transpose()
            .map_err(|_| RuntimeError::InvalidSnapshot("invalid local mana Explanation claim"))?;
        let local_claim = match local_claim {
            Some(local_claim) => local_claim,
            None => MaterialSurfaceLocalManaTransitionClaim::unknown(transition.mana_total)
                .map_err(|_| {
                    RuntimeError::InvalidSnapshot("invalid local mana Explanation claim")
                })?,
        };
        claims.push(local_claim);
        let frame = ExplanationFrame::new(time, claims).map_err(|_| {
            RuntimeError::InvalidSnapshot("invalid material surface Explanation frame")
        })?;
        ExplanationReport::new(
            ExperimentId::new(self.config.deterministic.world_seed),
            vec![frame],
        )
        .map_err(|_| RuntimeError::InvalidSnapshot("invalid material surface Explanation report"))
    }

    pub(crate) fn bytes_per_chunk(&self) -> u64 {
        let Some((_, field)) = self.mana.fields().iter().next() else {
            return 0;
        };
        std::mem::size_of_val(field.intensity()) as u64
    }

    pub(crate) fn physical_state_digest(&self, time: SimulationTime) -> PhysicalStateDigest {
        let mut digest = CanonicalDigest::new();
        digest.write(u64::from(CURRENT_DIGEST_SCHEMA_VERSION.raw()));
        digest.write(PHYSICAL_DIGEST_DOMAIN);
        digest.write(time.raw());
        digest.write(self.executed_experiment_recipe_mana_sources.len() as u64);
        for receipt in &self.executed_experiment_recipe_mana_sources {
            digest.write(receipt.source_record_id);
            digest.write(receipt.scheduled_tick);
            digest.write(receipt.executed_tick);
            digest.write(receipt.source_trace.raw());
            digest.write(receipt.before_intensity as u64);
            digest.write(receipt.after_intensity as u64);
            digest.write_bytes(receipt.recipe_hash.bytes());
            digest.write(receipt.policy_schema_id);
        }
        digest.write(self.material_surfaces.len() as u64);
        for (id, surface) in &self.material_surfaces {
            write_chart_chunk(&mut digest, id.chunk);
            digest.write(u64::from(id.cell_index));
            digest.write(surface.condition as u64);
            digest.write(surface.contact_count);
            digest.write(surface.last_transition.raw());
            write_optional_trace(&mut digest, surface.last_contact_trace);
            digest.write(u64::from(surface.gate.active));
            write_optional_trace(&mut digest, surface.gate.last_transition);
        }
        digest.write(self.pending_material_surface_changes.len() as u64);
        for id in &self.pending_material_surface_changes {
            write_chart_chunk(&mut digest, id.chunk);
            digest.write(u64::from(id.cell_index));
        }
        digest.write(self.material_surface_transitions.len() as u64);
        for transition in &self.material_surface_transitions {
            write_chart_chunk(&mut digest, transition.id.chunk);
            digest.write(u64::from(transition.id.cell_index));
            digest.write(transition.occurred_at.raw());
            digest.write(transition.before_condition as u64);
            digest.write(transition.after_condition as u64);
            digest.write(transition.mana_total as u64);
            write_optional_trace(&mut digest, transition.contact_trace);
            write_optional_trace(&mut digest, transition.mana_effect_trace);
            digest.write(transition.transition_trace.raw());
        }
        digest.write(self.material_surface_gate_transitions.len() as u64);
        for transition in &self.material_surface_gate_transitions {
            write_chart_chunk(&mut digest, transition.id.chunk);
            digest.write(u64::from(transition.id.cell_index));
            digest.write(transition.occurred_at.raw());
            digest.write(u64::from(transition.before_active));
            digest.write(u64::from(transition.after_active));
            digest.write(transition.local_mana_before as u64);
            digest.write(transition.local_mana_after as u64);
            digest.write(transition.local_mana_trace.raw());
            write_optional_trace(&mut digest, transition.contact_trace);
            digest.write(transition.transition_trace.raw());
        }
        digest.write(self.pattern_history.len() as u64);
        for sample in self.pattern_history.samples() {
            write_chart_chunk(&mut digest, sample.chunk);
            digest.write(sample.pattern.raw());
            digest.write(u64::from(sample.position.x));
            digest.write(u64::from(sample.position.y));
            digest.write(u64::from(sample.position.z));
            digest.write(sample.observed_at.raw());
            digest.write(u64::from(sample.magnitude));
            digest.write(u64::from(sample.source_ordinal));
            digest.write(sample.cause.raw());
        }
        digest.write(self.mana.observed_through().map_or(0, SimulationTime::raw));
        for (chunk, field) in self.mana.fields() {
            write_chart_chunk(&mut digest, *chunk);
            for value in field.intensity() {
                digest.write(*value as u64);
            }
            for value in field.last_change_before() {
                digest.write(*value as u64);
            }
        }
        digest.write(self.thermal_parameters.transfer_fraction as u64);
        digest.write(self.thermal_parameters.heat_capacity as u64);
        digest.write(self.thermal_parameters.scale as u64);
        digest.write(self.thermal_fields.batch_sequence());
        digest.write(self.thermal_fields.conservation_last_change().raw());
        digest.write(self.thermal_active_region.active_chunks().len() as u64);
        for chunk in self.thermal_active_region.active_chunks() {
            write_chart_chunk(&mut digest, *chunk);
        }
        digest.write(self.thermal_active_region.resident_chunks().len() as u64);
        for chunk in self.thermal_active_region.resident_chunks() {
            write_chart_chunk(&mut digest, *chunk);
        }
        digest.write(self.thermal_fields.fields().len() as u64);
        for field in self.thermal_fields.fields().values() {
            write_chart_chunk(&mut digest, field.chunk());
            digest.write(u64::from(field.extent()));
            for energy in field.energy() {
                digest.write(energy.get() as u64);
            }
            for trace in field.last_change() {
                digest.write(trace.raw());
            }
            for energy in field.last_change_before() {
                digest.write(energy.get() as u64);
            }
        }
        digest.write(self.thermal_reservoirs.len() as u64);
        for reservoir in self.thermal_reservoirs.values() {
            digest.write(reservoir.id.raw());
            write_chart_chunk(&mut digest, reservoir.target.chunk);
            digest.write(u64::from(reservoir.target.cell_index));
            digest.write(reservoir.budget.get() as u64);
            match reservoir.schedule {
                ThermalReservoirSchedule::PerTick(amount) => {
                    digest.write(1);
                    digest.write(amount.get() as u64);
                }
                ThermalReservoirSchedule::OneShot => digest.write(2),
            }
            digest.write(reservoir.bootstrap_trace.raw());
            digest.write(reservoir.last_change.raw());
        }
        digest.write(self.thermal_receipts.len() as u64);
        for (conservation_trace, receipts) in &self.thermal_receipts {
            digest.write(conservation_trace.raw());
            digest.write(receipts.len() as u64);
            for receipt in receipts {
                write_chart_chunk(&mut digest, receipt.cell.chunk);
                digest.write(u64::from(receipt.cell.cell_index));
                digest.write(receipt.pre_state.get() as u64);
                digest.write(receipt.post_state.get() as u64);
                write_optional_trace(&mut digest, receipt.cell_change_trace_id);
                digest.write(receipt.faces.len() as u64);
                for face in &receipt.faces {
                    write_chart_chunk(&mut digest, face.neighbor.chunk);
                    digest.write(u64::from(face.neighbor.cell_index));
                    digest.write(face.signed_flux as u64);
                    digest.write(face.neighbor_pre_state.get() as u64);
                }
                digest.write(receipt.reservoirs.len() as u64);
                for reservoir in &receipt.reservoirs {
                    digest.write(reservoir.id.raw());
                    digest.write(reservoir.scheduled_injection.get() as u64);
                    digest.write(reservoir.accepted_injection.get() as u64);
                    digest.write(reservoir.rejected_injection.get() as u64);
                    write_optional_trace(&mut digest, reservoir.transfer_trace_id);
                }
            }
        }
        digest.write(self.thermal_conservation_receipts.len() as u64);
        for (trace, receipt) in &self.thermal_conservation_receipts {
            digest.write(trace.raw());
            digest.write(receipt.tick);
            write_i128(&mut digest, receipt.total_cell_energy_before);
            write_i128(&mut digest, receipt.total_cell_energy_after);
            write_i128(&mut digest, receipt.total_reservoir_budget_before);
            write_i128(&mut digest, receipt.total_reservoir_budget_after);
            write_i128(&mut digest, receipt.residual);
        }
        digest.write(self.thermal_boundary_records.len() as u64);
        for record in &self.thermal_boundary_records {
            write_chart_chunk(&mut digest, record.cell.chunk);
            digest.write(u64::from(record.cell.cell_index));
            write_chart_chunk(&mut digest, record.neighbor.chunk);
            digest.write(u64::from(record.neighbor.cell_index));
            digest.write(record.cell_pre_state.get() as u64);
        }
        digest.write(self.resolution.evaluated_through().raw());
        for chunk in self.active_chunks.keys().copied() {
            write_chart_chunk(&mut digest, chunk);
            let entry = self
                .resolution
                .entry(chunk)
                .expect("runtime resolution field always contains every active chunk");
            digest.write(entry.relevance as u64);
            digest.write(u64::from(entry.level));
        }
        for (chunk, active) in &self.active_chunks {
            write_chart_chunk(&mut digest, *chunk);
            digest.write(active.total_mana as u64);
            digest.write(active.event_count);
            digest.write(active.last_transition.map_or(0, TraceId::raw));
        }
        digest.write(self.actors.len() as u64);
        for (actor, state) in &self.actors {
            digest.write(actor.raw());
            digest.write(state.body.position.x as u64);
            digest.write(state.body.position.y as u64);
            digest.write(state.body.position.z as u64);
            digest.write(state.body.energy as u64);
            digest.write(state.features.len() as u64);
            digest.write(state.proposals.len() as u64);
            if let Some(ancestry) = self.actor_ancestry.get(actor) {
                digest.write(ancestry.len() as u64);
                for trace in ancestry {
                    digest.write(trace.raw());
                }
            } else {
                digest.write(0);
            }
        }
        digest.write(self.actor_objects.len() as u64);
        for (key, object) in &self.actor_objects {
            digest.write(*key);
            digest.write(object.position.x as u64);
            digest.write(object.position.y as u64);
            digest.write(object.position.z as u64);
            digest.write(object.magnitude as u64);
            digest.write(if object.accessible { 1 } else { 0 });
            digest.write(if object.occluded { 1 } else { 0 });
            digest.write(object.trace.raw());
        }
        digest.write(self.population_aggregates.len() as u64);
        for aggregate in self.population_aggregates.values() {
            write_population_aggregate(&mut digest, aggregate);
        }
        digest.write(self.aggregate_actor_pool.len() as u64);
        for (chunk, actors) in &self.aggregate_actor_pool {
            write_chart_chunk(&mut digest, *chunk);
            digest.write(actors.len() as u64);
            for actor in actors {
                digest.write(actor.raw());
            }
        }
        PhysicalStateDigest {
            schema_version: CURRENT_DIGEST_SCHEMA_VERSION,
            fingerprint: digest.finish(),
        }
    }

    pub(crate) fn history_digest(&self) -> HistoryDigest {
        let mut digest = CanonicalDigest::new();
        digest.write(u64::from(CURRENT_DIGEST_SCHEMA_VERSION.raw()));
        digest.write(HISTORY_DIGEST_DOMAIN);
        for event in self.traces.iter() {
            digest.write(event.event_id.raw());
            digest.write(event.trace_id.raw());
            digest.write(event.time.raw());
            digest.write(u64::from(event.phase.id().0));
            digest.write(event.kind.raw());
            digest.write(event.causes.len() as u64);
            for cause in event.causes {
                digest.write(cause.raw());
            }
            digest.write(event.effects.len() as u64);
            for effect in event.effects {
                digest.write(effect.target().object_kind().raw());
                digest.write(effect.target().object_id());
                digest.write(effect.target().property().raw());
                digest.write_bytes(effect.before().bytes());
                digest.write_bytes(effect.after().bytes());
            }
        }
        digest.write(self.material_surfaces.len() as u64);
        for (id, surface) in &self.material_surfaces {
            write_chart_chunk(&mut digest, id.chunk);
            digest.write(u64::from(id.cell_index));
            digest.write(u64::from(surface.gate.active));
            write_optional_trace(&mut digest, surface.gate.last_transition);
        }
        digest.write(self.material_surface_gate_transitions.len() as u64);
        for transition in &self.material_surface_gate_transitions {
            write_chart_chunk(&mut digest, transition.id.chunk);
            digest.write(u64::from(transition.id.cell_index));
            digest.write(transition.occurred_at.raw());
            digest.write(u64::from(transition.before_active));
            digest.write(u64::from(transition.after_active));
            digest.write(transition.local_mana_before as u64);
            digest.write(transition.local_mana_after as u64);
            digest.write(transition.local_mana_trace.raw());
            write_optional_trace(&mut digest, transition.contact_trace);
            digest.write(transition.transition_trace.raw());
        }
        HistoryDigest {
            schema_version: CURRENT_DIGEST_SCHEMA_VERSION,
            fingerprint: digest.finish(),
        }
    }

    pub(crate) fn canonical_state(
        physical_state_digest: PhysicalStateDigest,
        history_digest: HistoryDigest,
    ) -> ExperimentDigest {
        let mut digest = CanonicalDigest::new();
        digest.write(u64::from(CURRENT_DIGEST_SCHEMA_VERSION.raw()));
        digest.write(EXPERIMENT_DIGEST_DOMAIN);
        digest.write(u64::from(physical_state_digest.schema_version.raw()));
        digest.write_bytes(physical_state_digest.bytes());
        digest.write(u64::from(history_digest.schema_version.raw()));
        digest.write_bytes(history_digest.bytes());
        ExperimentDigest {
            schema_version: CURRENT_DIGEST_SCHEMA_VERSION,
            fingerprint: digest.finish(),
        }
    }
}

fn write_i128(digest: &mut CanonicalDigest, value: i128) {
    let bits = value as u128;
    digest.write(bits as u64);
    digest.write((bits >> 64) as u64);
}

fn runtime_system_registrations() -> Vec<SystemRegistrationSnapshot> {
    vec![
        SystemRegistrationSnapshot {
            phase: Phase::Physics,
            system_schema_id: PHYSICAL_SYSTEM_ID,
            revision: 1,
            registration_order: 0,
        },
        SystemRegistrationSnapshot {
            phase: Phase::Mana,
            system_schema_id: EXPERIMENT_RECIPE_MANA_SOURCE_SYSTEM_ID,
            revision: 1,
            registration_order: 1,
        },
        SystemRegistrationSnapshot {
            phase: Phase::Mana,
            system_schema_id: MANA_SYSTEM_ID,
            revision: 1,
            registration_order: 2,
        },
        SystemRegistrationSnapshot {
            phase: Phase::Mana,
            system_schema_id: MANA_EFFECTS_SYSTEM_ID,
            revision: 2,
            registration_order: 3,
        },
        SystemRegistrationSnapshot {
            phase: Phase::Resolution,
            system_schema_id: RESOLUTION_SYSTEM_ID,
            revision: 1,
            registration_order: 4,
        },
        SystemRegistrationSnapshot {
            phase: Phase::Perception,
            system_schema_id: 40,
            revision: 1,
            registration_order: 5,
        },
        SystemRegistrationSnapshot {
            phase: Phase::Cognition,
            system_schema_id: 41,
            revision: 1,
            registration_order: 6,
        },
        SystemRegistrationSnapshot {
            phase: Phase::Action,
            system_schema_id: ACTOR_ACTION_SYSTEM_ID,
            revision: 1,
            registration_order: 7,
        },
        SystemRegistrationSnapshot {
            phase: Phase::Lifecycle,
            system_schema_id: LIFECYCLE_SYSTEM_ID,
            revision: 1,
            registration_order: 8,
        },
        SystemRegistrationSnapshot {
            phase: Phase::Physics,
            system_schema_id: THERMAL_RESERVOIR_SYSTEM_ID,
            revision: 1,
            registration_order: 9,
        },
        SystemRegistrationSnapshot {
            phase: Phase::Physics,
            system_schema_id: THERMAL_EVOLUTION_SYSTEM_ID,
            revision: 1,
            registration_order: 10,
        },
    ]
}

fn import_carrier_adapters(
    snapshots: Vec<TerrainCarrierSnapshot>,
) -> Result<BTreeMap<ChartChunkCoord, TerrainCarrierAdapter>, RuntimeError> {
    let mut adapters = BTreeMap::new();
    for snapshot in snapshots {
        let chunk = snapshot.chunk;
        let adapter = TerrainCarrierAdapter::import_snapshot(snapshot)
            .map_err(|_| RuntimeError::InvalidSnapshot("invalid terrain carrier"))?;
        if adapters.insert(chunk, adapter).is_some() {
            return Err(RuntimeError::InvalidSnapshot("duplicate carrier chunk"));
        }
    }
    Ok(adapters)
}

fn import_active_chunks(
    snapshots: Vec<ActiveChunkSnapshot>,
) -> Result<BTreeMap<ChartChunkCoord, ActiveChunkState>, RuntimeError> {
    let mut chunks = BTreeMap::new();
    for snapshot in snapshots {
        if chunks
            .insert(
                snapshot.chunk,
                ActiveChunkState {
                    relevance: snapshot.relevance,
                    level: snapshot.level,
                    total_mana: snapshot.total_mana,
                    event_count: snapshot.event_count,
                    last_transition: snapshot.last_transition,
                },
            )
            .is_some()
        {
            return Err(RuntimeError::InvalidSnapshot("duplicate active chunk"));
        }
    }
    Ok(chunks)
}

type ImportedThermal = (
    ThermalFieldSet,
    ThermalActiveRegion,
    Vec<ThermalBoundaryRecord>,
    BTreeMap<ThermalReservoirId, ThermalReservoir>,
    BTreeMap<TraceId, Vec<ThermalCellTransferReceipt>>,
    BTreeMap<TraceId, ThermalConservationReceipt>,
);

fn import_thermal_snapshot(
    snapshot: ThermalSnapshot,
    chunk_extent: u8,
    active_chunks: &BTreeMap<ChartChunkCoord, ActiveChunkState>,
) -> Result<ImportedThermal, RuntimeError> {
    snapshot.parameters.validate()?;
    let expected_chunks = active_chunks.keys().copied().collect::<BTreeSet<_>>();
    let active_region_chunks = snapshot
        .active_region
        .active_chunks
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    let resident_region_chunks = snapshot
        .active_region
        .resident_chunks
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    if active_region_chunks.len() != snapshot.active_region.active_chunks.len()
        || resident_region_chunks.len() != snapshot.active_region.resident_chunks.len()
        || active_region_chunks != expected_chunks
        || resident_region_chunks != expected_chunks
    {
        return Err(RuntimeError::InvalidSnapshot(
            "thermal active region is incomplete",
        ));
    }
    let thermal_active_region =
        ThermalActiveRegion::new(active_region_chunks, resident_region_chunks)?;
    let field_count = snapshot.field_set.fields.len();
    let mut fields = Vec::with_capacity(field_count);
    for field in snapshot.field_set.fields {
        if field.extent != chunk_extent {
            return Err(RuntimeError::InvalidSnapshot(
                "thermal field extent mismatch",
            ));
        }
        let energy = field
            .energy
            .into_iter()
            .map(ThermalEnergy::new)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| RuntimeError::InvalidSnapshot("thermal field has negative energy"))?;
        let last_change_before = field
            .last_change_before
            .into_iter()
            .map(ThermalEnergy::new)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| {
                RuntimeError::InvalidSnapshot("thermal field has negative prior energy")
            })?;
        fields.push(
            ThermalField::from_snapshot_parts(
                field.chunk,
                field.extent,
                energy,
                field.last_change,
                last_change_before,
            )
            .map_err(|_| RuntimeError::InvalidSnapshot("thermal field state is malformed"))?,
        );
    }
    let thermal_fields = ThermalFieldSet::from_snapshot_parts(
        fields,
        snapshot.field_set.batch_sequence,
        snapshot.field_set.conservation_last_change,
    )
    .map_err(|_| RuntimeError::InvalidSnapshot("thermal field set is malformed"))?;
    if thermal_fields.fields().len() != field_count
        || thermal_fields
            .fields()
            .keys()
            .copied()
            .collect::<BTreeSet<_>>()
            != expected_chunks
    {
        return Err(RuntimeError::InvalidSnapshot(
            "thermal fields have active-region gaps",
        ));
    }

    let mut reservoirs = BTreeMap::new();
    for reservoir in snapshot.reservoirs {
        let budget = ThermalEnergy::new(reservoir.budget)
            .map_err(|_| RuntimeError::InvalidSnapshot("thermal reservoir has negative budget"))?;
        let schedule = match reservoir.schedule {
            ThermalReservoirScheduleSnapshot::PerTick(amount) => {
                ThermalReservoirSchedule::PerTick(ThermalEnergy::new(amount).map_err(|_| {
                    RuntimeError::InvalidSnapshot("thermal reservoir has negative schedule")
                })?)
            }
            ThermalReservoirScheduleSnapshot::OneShot => ThermalReservoirSchedule::OneShot,
        };
        let field =
            thermal_fields
                .field(reservoir.target.chunk)
                .ok_or(RuntimeError::InvalidSnapshot(
                    "thermal reservoir target lies outside active region",
                ))?;
        if usize::from(reservoir.target.cell_index) >= field.energy().len() {
            return Err(RuntimeError::InvalidSnapshot(
                "thermal reservoir target cell lies outside field",
            ));
        }
        if reservoirs
            .insert(
                reservoir.id,
                ThermalReservoir {
                    id: reservoir.id,
                    target: reservoir.target,
                    budget,
                    schedule,
                    bootstrap_trace: reservoir.bootstrap_trace,
                    last_change: reservoir.last_change,
                },
            )
            .is_some()
        {
            return Err(RuntimeError::InvalidSnapshot("duplicate thermal reservoir"));
        }
    }

    if snapshot.conservation_receipts.len() != thermal_fields.batch_sequence() as usize {
        return Err(RuntimeError::InvalidSnapshot(
            "thermal batch sequence does not match conservation receipts",
        ));
    }
    let mut conservation_receipts = BTreeMap::new();
    for receipt in snapshot.conservation_receipts {
        if receipt.residual != 0 {
            return Err(RuntimeError::InvalidSnapshot(
                "thermal conservation receipt has non-zero residual",
            ));
        }
        if conservation_receipts
            .insert(
                receipt.trace,
                ThermalConservationReceipt {
                    tick: receipt.tick,
                    total_cell_energy_before: receipt.total_cell_energy_before,
                    total_cell_energy_after: receipt.total_cell_energy_after,
                    total_reservoir_budget_before: receipt.total_reservoir_budget_before,
                    total_reservoir_budget_after: receipt.total_reservoir_budget_after,
                    residual: receipt.residual,
                },
            )
            .is_some()
        {
            return Err(RuntimeError::InvalidSnapshot(
                "duplicate thermal conservation receipt trace",
            ));
        }
    }
    if let Some(last_trace) = conservation_receipts.keys().next_back() {
        if *last_trace != thermal_fields.conservation_last_change() {
            return Err(RuntimeError::InvalidSnapshot(
                "thermal conservation anchor does not match latest receipt",
            ));
        }
    }

    let mut receipts = BTreeMap::<TraceId, Vec<ThermalCellTransferReceipt>>::new();
    for trace in snapshot.receipt_batches {
        if !conservation_receipts.contains_key(&trace)
            || receipts.insert(trace, Vec::new()).is_some()
        {
            return Err(RuntimeError::InvalidSnapshot(
                "thermal receipt batch is invalid",
            ));
        }
    }
    for receipt in snapshot.transfer_receipts {
        if !conservation_receipts.contains_key(&receipt.conservation_trace) {
            return Err(RuntimeError::InvalidSnapshot(
                "thermal transfer receipt lacks conservation receipt",
            ));
        }
        if thermal_fields.field(receipt.cell.chunk).is_none() {
            return Err(RuntimeError::InvalidSnapshot(
                "thermal transfer receipt references inactive field",
            ));
        }
        let pre_state = ThermalEnergy::new(receipt.pre_state)
            .map_err(|_| RuntimeError::InvalidSnapshot("thermal receipt has negative pre-state"))?;
        let post_state = ThermalEnergy::new(receipt.post_state).map_err(|_| {
            RuntimeError::InvalidSnapshot("thermal receipt has negative post-state")
        })?;
        if receipt.faces.len() > 6 {
            return Err(RuntimeError::InvalidSnapshot(
                "thermal receipt has too many faces",
            ));
        }
        let faces = receipt
            .faces
            .into_iter()
            .map(|face| {
                Ok(causafera_domains::ThermalFaceRecord {
                    neighbor: face.neighbor,
                    signed_flux: face.signed_flux,
                    neighbor_pre_state: ThermalEnergy::new(face.neighbor_pre_state).map_err(
                        |_| {
                            RuntimeError::InvalidSnapshot(
                                "thermal face has negative neighbor energy",
                            )
                        },
                    )?,
                })
            })
            .collect::<Result<Vec<_>, RuntimeError>>()?;
        let mut reservoir_records = Vec::with_capacity(receipt.reservoirs.len());
        for record in receipt.reservoirs {
            if !reservoirs.contains_key(&record.id) {
                return Err(RuntimeError::InvalidSnapshot(
                    "thermal receipt references unknown reservoir",
                ));
            }
            let scheduled = ThermalEnergy::new(record.scheduled_injection).map_err(|_| {
                RuntimeError::InvalidSnapshot("thermal receipt has negative scheduled injection")
            })?;
            let accepted = ThermalEnergy::new(record.accepted_injection).map_err(|_| {
                RuntimeError::InvalidSnapshot("thermal receipt has negative accepted injection")
            })?;
            let rejected = ThermalEnergy::new(record.rejected_injection).map_err(|_| {
                RuntimeError::InvalidSnapshot("thermal receipt has negative rejected injection")
            })?;
            if i128::from(accepted.get()) + i128::from(rejected.get())
                != i128::from(scheduled.get())
            {
                return Err(RuntimeError::InvalidSnapshot(
                    "thermal receipt injection does not balance",
                ));
            }
            reservoir_records.push(causafera_domains::ThermalReservoirTransferRecord {
                id: record.id,
                scheduled_injection: scheduled,
                accepted_injection: accepted,
                rejected_injection: rejected,
                transfer_trace_id: record.transfer_trace_id,
            });
        }
        receipts
            .get_mut(&receipt.conservation_trace)
            .ok_or(RuntimeError::InvalidSnapshot(
                "thermal transfer receipt lacks receipt batch",
            ))?
            .push(ThermalCellTransferReceipt {
                cell: receipt.cell,
                pre_state,
                post_state,
                cell_change_trace_id: receipt.cell_change_trace_id,
                faces,
                reservoirs: reservoir_records,
            });
    }
    if receipts.keys().copied().collect::<BTreeSet<_>>()
        != conservation_receipts.keys().copied().collect()
    {
        return Err(RuntimeError::InvalidSnapshot(
            "thermal receipt batches do not cover conservation receipts",
        ));
    }
    for (trace, receipt) in &conservation_receipts {
        let accepted = receipts
            .get(trace)
            .into_iter()
            .flatten()
            .flat_map(|entry| entry.reservoirs.iter())
            .try_fold(0_i128, |total, record| {
                total
                    .checked_add(i128::from(record.accepted_injection.get()))
                    .ok_or(RuntimeError::InvalidSnapshot(
                        "thermal receipt injection total overflows",
                    ))
            })?;
        if receipt.total_reservoir_budget_before - receipt.total_reservoir_budget_after != accepted
        {
            return Err(RuntimeError::InvalidSnapshot(
                "thermal reservoir budgets do not balance receipts",
            ));
        }
    }
    let boundary_records =
        import_thermal_boundary_records(snapshot.boundary_records, &thermal_fields, &receipts)?;
    Ok((
        thermal_fields,
        thermal_active_region,
        boundary_records,
        reservoirs,
        receipts,
        conservation_receipts,
    ))
}

fn import_thermal_boundary_records(
    snapshots: Vec<ThermalBoundaryRecordSnapshot>,
    fields: &ThermalFieldSet,
    receipts: &BTreeMap<TraceId, Vec<ThermalCellTransferReceipt>>,
) -> Result<Vec<ThermalBoundaryRecord>, RuntimeError> {
    if fields.batch_sequence() == 0 {
        return if snapshots.is_empty() {
            Ok(Vec::new())
        } else {
            Err(RuntimeError::InvalidSnapshot(
                "thermal boundary records exist before the first batch",
            ))
        };
    }

    let mut latest_pre_state = BTreeMap::new();
    for receipt in
        receipts
            .get(&fields.conservation_last_change())
            .ok_or(RuntimeError::InvalidSnapshot(
                "thermal boundary records lack latest receipt batch",
            ))?
    {
        if latest_pre_state
            .insert(receipt.cell, receipt.pre_state)
            .is_some()
        {
            return Err(RuntimeError::InvalidSnapshot(
                "duplicate thermal cell receipt in latest batch",
            ));
        }
    }

    let mut expected = Vec::new();
    for field in fields.fields().values() {
        let side = usize::from(field.extent());
        let plane = side * side;
        for (index, energy) in field.energy().iter().copied().enumerate() {
            let cell_index = u16::try_from(index).map_err(|_| {
                RuntimeError::InvalidSnapshot("thermal boundary source index is invalid")
            })?;
            let cell = ThermalCellKey::new(field.chunk(), cell_index);
            let position = [index % side, (index % plane) / side, index / plane];
            for direction in [
                [-1_i8, 0, 0],
                [1, 0, 0],
                [0, -1, 0],
                [0, 1, 0],
                [0, 0, -1],
                [0, 0, 1],
            ] {
                let mut neighbor_position = position;
                let mut chunk_delta = [0_i32; 3];
                for axis in 0..3 {
                    match direction[axis] {
                        -1 if position[axis] == 0 => {
                            neighbor_position[axis] = side - 1;
                            chunk_delta[axis] = -1;
                        }
                        -1 => neighbor_position[axis] -= 1,
                        0 => {}
                        1 if position[axis] + 1 == side => {
                            neighbor_position[axis] = 0;
                            chunk_delta[axis] = 1;
                        }
                        1 => neighbor_position[axis] += 1,
                        _ => {
                            return Err(RuntimeError::InvalidSnapshot(
                                "thermal boundary face direction is invalid",
                            ));
                        }
                    }
                }
                let neighbor_chunk = field.chunk().same_chart_neighbor(
                    chunk_delta[0],
                    chunk_delta[1],
                    chunk_delta[2],
                );
                if fields.field(neighbor_chunk).is_some() {
                    continue;
                }
                let neighbor_index = neighbor_position[0]
                    + neighbor_position[1] * side
                    + neighbor_position[2] * plane;
                let neighbor_cell_index = u16::try_from(neighbor_index).map_err(|_| {
                    RuntimeError::InvalidSnapshot("thermal boundary neighbor index is invalid")
                })?;
                expected.push(ThermalBoundaryRecord {
                    cell,
                    neighbor: ThermalCellKey::new(neighbor_chunk, neighbor_cell_index),
                    cell_pre_state: latest_pre_state.get(&cell).copied().unwrap_or(energy),
                });
            }
        }
    }
    expected.sort_unstable_by_key(|record| (record.cell, record.neighbor));

    let mut records = Vec::with_capacity(snapshots.len());
    let mut previous_key = None;
    for snapshot in snapshots {
        let key = (snapshot.cell, snapshot.neighbor);
        if previous_key.is_some_and(|previous| previous >= key) {
            return Err(RuntimeError::InvalidSnapshot(
                "thermal boundary records must be strictly ordered",
            ));
        }
        previous_key = Some(key);
        records.push(ThermalBoundaryRecord {
            cell: snapshot.cell,
            neighbor: snapshot.neighbor,
            cell_pre_state: ThermalEnergy::new(snapshot.cell_pre_state).map_err(|_| {
                RuntimeError::InvalidSnapshot("thermal boundary record has negative pre-state")
            })?,
        });
    }
    if records != expected {
        return Err(RuntimeError::InvalidSnapshot(
            "thermal boundary records do not match the current boundary face set",
        ));
    }
    Ok(records)
}

fn import_experiment_recipe_mana_source_receipts(
    snapshots: Vec<ExperimentRecipeManaSourceReceiptSnapshot>,
) -> Result<Vec<ExperimentRecipeManaSourceReceipt>, RuntimeError> {
    if snapshots.len() > MAX_EXPERIMENT_RECIPE_MANA_SOURCES {
        return Err(RuntimeError::InvalidSnapshot(
            "too many experiment recipe mana source receipts",
        ));
    }
    let mut receipts = Vec::with_capacity(snapshots.len());
    let mut previous_key = None;
    let mut source_ids = BTreeSet::new();
    for snapshot in snapshots {
        let key = (snapshot.executed_tick, snapshot.source_record_id);
        if previous_key.is_some_and(|previous| previous >= key) {
            return Err(RuntimeError::InvalidSnapshot(
                "experiment recipe mana source receipts must be strictly ordered",
            ));
        }
        if !source_ids.insert(snapshot.source_record_id) {
            return Err(RuntimeError::InvalidSnapshot(
                "duplicate experiment recipe mana source receipt ID",
            ));
        }
        previous_key = Some(key);
        receipts.push(ExperimentRecipeManaSourceReceipt {
            source_record_id: snapshot.source_record_id,
            scheduled_tick: snapshot.scheduled_tick,
            executed_tick: snapshot.executed_tick,
            source_trace: snapshot.source_trace,
            before_intensity: snapshot.before_intensity,
            after_intensity: snapshot.after_intensity,
            recipe_hash: snapshot.recipe_hash,
            policy_schema_id: snapshot.policy_schema_id,
        });
    }
    Ok(receipts)
}

type ImportedMaterialSurfaces = (
    BTreeMap<MaterialSurfaceId, MaterialSurface>,
    BTreeSet<MaterialSurfaceId>,
    Vec<MaterialSurfaceTransition>,
    Vec<MaterialSurfaceGateTransition>,
);

fn import_material_surfaces(
    snapshot: MaterialSurfaceSnapshot,
    chunk_extent: u8,
) -> Result<ImportedMaterialSurfaces, RuntimeError> {
    let mut surfaces = BTreeMap::new();
    for record in snapshot.records {
        if !record.id.is_within_extent(chunk_extent) {
            return Err(RuntimeError::InvalidSnapshot(
                "material surface cell ordinal outside chunk extent",
            ));
        }
        if surfaces.insert(record.id, record.surface).is_some() {
            return Err(RuntimeError::InvalidSnapshot("duplicate material surface"));
        }
    }
    validate_material_surface_object_ids(surfaces.keys().copied())?;
    let mut pending_changes = BTreeSet::new();
    for id in snapshot.pending_physical_changes {
        if !id.is_within_extent(chunk_extent) {
            return Err(RuntimeError::InvalidSnapshot(
                "material surface cell ordinal outside chunk extent",
            ));
        }
        if !pending_changes.insert(id) {
            return Err(RuntimeError::InvalidSnapshot(
                "duplicate changed material surface",
            ));
        }
    }
    if snapshot.transitions.len() > MAX_MATERIAL_SURFACE_TRANSITIONS {
        return Err(RuntimeError::InvalidSnapshot(
            "too many material surface transitions",
        ));
    }
    let mut previous_trace = None;
    for transition in &snapshot.transitions {
        if !transition.id.is_within_extent(chunk_extent) || !surfaces.contains_key(&transition.id) {
            return Err(RuntimeError::InvalidSnapshot(
                "material surface transition references invalid surface",
            ));
        }
        if transition.before_condition == transition.after_condition {
            return Err(RuntimeError::InvalidSnapshot(
                "material surface transition has no state change",
            ));
        }
        if previous_trace.is_some_and(|previous| previous >= transition.transition_trace) {
            return Err(RuntimeError::InvalidSnapshot(
                "material surface transitions must be strictly trace ordered",
            ));
        }
        if transition
            .mana_effect_trace
            .is_some_and(|trace| trace != transition.transition_trace)
            || (transition.mana_effect_trace.is_none() && transition.mana_total != 0)
        {
            return Err(RuntimeError::InvalidSnapshot(
                "invalid material surface mana transition anchors",
            ));
        }
        previous_trace = Some(transition.transition_trace);
    }
    if snapshot.gate_transitions.len() > MAX_MATERIAL_SURFACE_TRANSITIONS {
        return Err(RuntimeError::InvalidSnapshot(
            "too many material surface gate transitions",
        ));
    }
    let mut previous_gate_trace = None;
    let mut latest_gate_trace_by_surface: BTreeMap<MaterialSurfaceId, TraceId> = BTreeMap::new();
    for transition in &snapshot.gate_transitions {
        if !transition.id.is_within_extent(chunk_extent) || !surfaces.contains_key(&transition.id) {
            return Err(RuntimeError::InvalidSnapshot(
                "material surface gate transition references invalid surface",
            ));
        }
        if transition.before_active == transition.after_active {
            return Err(RuntimeError::InvalidSnapshot(
                "material surface gate transition has no state change",
            ));
        }
        if previous_gate_trace.is_some_and(|previous| previous >= transition.transition_trace) {
            return Err(RuntimeError::InvalidSnapshot(
                "material surface gate transitions must be strictly trace ordered",
            ));
        }
        latest_gate_trace_by_surface
            .entry(transition.id)
            .and_modify(|current| {
                if transition.transition_trace > *current {
                    *current = transition.transition_trace;
                }
            })
            .or_insert(transition.transition_trace);
        previous_gate_trace = Some(transition.transition_trace);
    }
    Ok((
        surfaces,
        pending_changes,
        snapshot.transitions,
        snapshot.gate_transitions,
    ))
}

fn import_actor_ancestry(
    entries: Vec<(ActorId, Vec<TraceId>)>,
) -> Result<BTreeMap<ActorId, Vec<TraceId>>, RuntimeError> {
    let mut ancestry = BTreeMap::new();
    for (actor, traces) in entries {
        validate_trace_ancestry(&traces)?;
        if ancestry.insert(actor, traces).is_some() {
            return Err(RuntimeError::InvalidSnapshot("duplicate actor ancestry"));
        }
    }
    Ok(ancestry)
}

fn import_actor_objects(
    entries: Vec<(u64, ActorPhysicalObject)>,
) -> Result<BTreeMap<u64, ActorPhysicalObject>, RuntimeError> {
    let mut objects = BTreeMap::new();
    for (key, object) in entries {
        if key != object.object_key || objects.insert(key, object).is_some() {
            return Err(RuntimeError::InvalidSnapshot("duplicate actor object"));
        }
    }
    Ok(objects)
}

fn import_population_aggregates(
    entries: Vec<PopulationAggregate>,
) -> Result<BTreeMap<ChartChunkCoord, PopulationAggregate>, RuntimeError> {
    let mut aggregates = BTreeMap::new();
    for aggregate in entries {
        validate_trace_ancestry(&aggregate.causal_ancestry)?;
        if aggregates.insert(aggregate.chart, aggregate).is_some() {
            return Err(RuntimeError::InvalidSnapshot(
                "duplicate population aggregate",
            ));
        }
    }
    Ok(aggregates)
}

fn import_aggregate_actor_pool(
    entries: Vec<(ChartChunkCoord, Vec<ActorId>)>,
) -> Result<BTreeMap<ChartChunkCoord, Vec<ActorId>>, RuntimeError> {
    let mut pool = BTreeMap::new();
    for (chunk, actors) in entries {
        if actors.windows(2).any(|pair| pair[0] >= pair[1]) || pool.insert(chunk, actors).is_some()
        {
            return Err(RuntimeError::InvalidSnapshot(
                "invalid aggregate actor pool",
            ));
        }
    }
    Ok(pool)
}

fn validate_trace_exists(store: &CausalTraceStore, trace: TraceId) -> Result<(), RuntimeError> {
    if store.event(trace).is_some() {
        Ok(())
    } else {
        Err(RuntimeError::InvalidSnapshot("unknown trace reference"))
    }
}

fn runtime_carrier_adapters(
    config: CarrierAdapterConfig,
    field_extent: u8,
    root_trace: TraceId,
    chunks: &[ChartChunkCoord],
) -> BTreeMap<ChartChunkCoord, TerrainCarrierAdapter> {
    match config {
        CarrierAdapterConfig::TerrainSeed { terrain_seed } => chunks
            .iter()
            .map(|chunk| {
                let seed = terrain_seed ^ chart_chunk_hash(*chunk);
                (
                    *chunk,
                    TerrainCarrierAdapter::new(
                        *chunk,
                        deterministic_terrain_chunk(seed, *chunk, root_trace),
                        field_extent,
                    ),
                )
            })
            .collect(),
    }
}

pub(crate) fn active_chunk_keys(chart_id: SpatialChartId, radius: u8) -> Vec<ChartChunkCoord> {
    let radius = i32::from(radius);
    (-radius..=radius)
        .map(|x| ChartChunkCoord::new(chart_id, ChunkCoord::new(x, 0, 0)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use causafera_types::{LocalCoord, PhysicalPatternId, WorldCoord};

    fn test_chunk() -> ChartChunkCoord {
        ChartChunkCoord::new(SpatialChartId::new(1), ChunkCoord::new(0, 0, 0))
    }

    #[test]
    #[ignore = "expensive benchmark"]
    fn runtime_executes_a_long_causal_run_without_errors() {
        let mut runtime = Runtime::new(production_loop_config(42)).unwrap();
        let snapshot = runtime.run_ticks(512).unwrap();
        assert_eq!(snapshot.time, SimulationTime::new(512));
        assert!(snapshot.mana_total > 0);
        assert!(snapshot.causal_trace_count > 512);
        assert!(snapshot.resolution_level > 0);
    }

    #[test]
    fn runtime_executes_a_short_causal_run_without_errors() {
        let mut runtime = Runtime::new(production_loop_config(42)).unwrap();
        let snapshot = runtime.run_ticks(64).unwrap();
        assert_eq!(snapshot.time, SimulationTime::new(64));
        assert!(snapshot.mana_total > 0);
        assert!(snapshot.causal_trace_count > 64);
        assert!(snapshot.resolution_level > 0);
    }

    #[test]
    fn material_transition_digest_distinguishes_absent_and_zero_trace_anchor() {
        // Given: equivalent authoritative material histories except for a contact anchor.
        let state = RuntimeState::new(&production_loop_config(701)).unwrap();
        let mut zero_anchor = RuntimeState::new(&production_loop_config(701)).unwrap();
        zero_anchor.material_surface_transitions[0].contact_trace = Some(TraceId::new(0));

        // When: physical-state digests are computed for the same completed time.
        let absent_digest = state.physical_state_digest(SimulationTime::new(0));
        let zero_digest = zero_anchor.physical_state_digest(SimulationTime::new(0));

        // Then: Option presence is part of the canonical authoritative representation.
        assert_ne!(absent_digest, zero_digest);
    }

    #[test]
    fn physical_digest_includes_source_receipt_recipe_hash() {
        // Given: equivalent states whose only receipt difference is the canonical recipe hash.
        let config = production_loop_config(703);
        let mut first = RuntimeState::new(&config).unwrap();
        let mut second = RuntimeState::new(&config).unwrap();
        let receipt = ExperimentRecipeManaSourceReceipt {
            source_record_id: 1,
            scheduled_tick: 2,
            executed_tick: 2,
            source_trace: TraceId::new(1),
            before_intensity: 0,
            after_intensity: 3,
            recipe_hash: fingerprint_u64(0x0303, 1),
            policy_schema_id: EXPERIMENT_RECIPE_MANA_SOURCE_POLICY_SCHEMA_V1,
        };
        first.executed_experiment_recipe_mana_sources.push(receipt);
        let mut altered = receipt;
        altered.recipe_hash = fingerprint_u64(0x0303, 2);
        second.executed_experiment_recipe_mana_sources.push(altered);

        // When: physical digests are computed at the same completed time.
        let first_digest = first.physical_state_digest(SimulationTime::new(2));
        let second_digest = second.physical_state_digest(SimulationTime::new(2));

        // Then: the persisted recipe identity contributes to authoritative physical identity.
        assert_ne!(first_digest, second_digest);
    }

    #[test]
    fn mana_effect_gate_transition_is_committed_in_mana_phase() {
        // Given: a production loop configured to activate its mana-to-material gate.
        let mut config = production_loop_config(702);
        config.mana_parameters.effect_threshold = 1;
        config.mana_parameters.effect_hysteresis = 0;
        let mut runtime = Runtime::new(config).unwrap();

        // When: repeated material structure reaches the Mana phase.
        runtime.run_ticks(4).unwrap();
        let state = runtime.lock_state().unwrap();

        // Then: every persisted local gate transition has Mana-phase provenance.
        assert!(state.traces.iter().any(|event| {
            event.phase == Phase::Mana
                && event.kind == EventKindId::new(MATERIAL_SURFACE_MANA_EVENT_KIND)
                && event.effects.iter().any(|effect| {
                    effect.target().property()
                        == StatePropertyId::new(MATERIAL_SURFACE_MANA_GATE_PROPERTY)
                })
        }));
    }

    #[test]
    fn strict_replay_has_identical_canonical_state() {
        let mut first = Runtime::from_seed(7).unwrap();
        let mut second = Runtime::from_seed(7).unwrap();
        let first = first.run_ticks(256).unwrap();
        let second = second.run_ticks(256).unwrap();
        assert_eq!(first, second);
        assert_eq!(first.physical_state_digest, second.physical_state_digest);
        assert_eq!(first.history_digest, second.history_digest);
        assert_eq!(first.canonical_state, second.canonical_state);
        assert_eq!(
            first.canonical_state.schema_version,
            CURRENT_DIGEST_SCHEMA_VERSION
        );
        assert!(first.active_chunk_count > 1);
    }

    #[test]
    fn same_seed_replay_is_preserved_with_multi_chunk_fields() {
        let mut first = Runtime::new(production_loop_config(8)).unwrap();
        let mut second = Runtime::new(production_loop_config(8)).unwrap();

        let first = first.run_ticks(96).unwrap();
        let second = second.run_ticks(96).unwrap();

        assert_eq!(first, second);
        assert_eq!(first.active_chunk_count, 3);
        assert!(first.mana_total > 0);
    }

    #[test]
    fn causally_strong_distant_chunk_reaches_higher_detail_than_weak_nearby_chunk() {
        let nearby = ChartChunkCoord::new(SpatialChartId::new(1), ChunkCoord::new(0, 0, 0));
        let distant = ChartChunkCoord::new(SpatialChartId::new(1), ChunkCoord::new(8, 0, 0));
        let field = ResolutionField::new(
            ResolutionFieldId::new(1),
            SimulationTime::new(0),
            vec![nearby, distant],
            vec![TraceId::new(1), TraceId::new(2)],
        )
        .unwrap();
        let policy = ResolutionPolicy::new(
            10_000,
            900,
            100,
            vec![500, 2_000, 5_000],
            vec![ChannelWeight::new(ResolutionChannelId::new(RESOLUTION_CHANNEL), 1_000).unwrap()],
        )
        .unwrap();

        let committed = field
            .propose_evaluation(
                SimulationTime::new(1),
                &policy,
                &[
                    causafera_resolution::CausalRelevanceSignal::new(
                        nearby,
                        nearby,
                        ResolutionChannelId::new(RESOLUTION_CHANNEL),
                        400,
                        TraceId::new(3),
                        0,
                    )
                    .unwrap(),
                    causafera_resolution::CausalRelevanceSignal::new(
                        distant,
                        distant,
                        ResolutionChannelId::new(RESOLUTION_CHANNEL),
                        1_000,
                        TraceId::new(4),
                        1,
                    )
                    .unwrap(),
                ],
            )
            .unwrap()
            .commit(&[TraceId::new(5), TraceId::new(6)])
            .unwrap();

        assert_eq!(committed.entry(nearby).unwrap().level, 0);
        assert_eq!(committed.entry(distant).unwrap().level, 1);
    }

    #[test]
    fn promotion_preserves_total_mana_across_resolution_transitions() {
        let mut runtime = Runtime::from_seed(12).unwrap();
        let snapshot = runtime.run_ticks(96).unwrap();
        let state = runtime.lock_state().unwrap();
        let active_mana_total = state
            .active_chunks
            .values()
            .map(|chunk| chunk.total_mana)
            .sum::<i64>();

        assert!(snapshot.resolution_transitions > 0);
        assert_eq!(active_mana_total, snapshot.mana_total);
        assert!(
            state
                .active_chunks
                .values()
                .any(|chunk| chunk.last_transition.is_some())
        );
    }

    #[test]
    fn runtime_reports_stage_six_active_chunk_metrics() {
        let mut runtime = Runtime::from_seed(14).unwrap();
        let snapshot = runtime.run_ticks(64).unwrap();

        assert_eq!(snapshot.active_chunk_count, 3);
        assert!(snapshot.resolution_transitions > 0);
        assert!(snapshot.bytes_per_chunk > 0);
    }

    #[test]
    #[ignore = "expensive benchmark"]
    fn physical_suppression_changes_the_causal_trajectory() {
        let control_config = production_loop_config(9);
        let mut intervention_config = control_config.clone();
        intervention_config.pattern_schedule = intervention_config
            .pattern_schedule
            .with_suppression(SimulationTime::new(80), SimulationTime::new(160))
            .unwrap();
        let mut control = Runtime::new(control_config).unwrap();
        let mut intervention = Runtime::new(intervention_config).unwrap();
        let control = control.run_ticks(256).unwrap();
        let intervention = intervention.run_ticks(256).unwrap();
        assert_ne!(
            control.physical_state_digest,
            intervention.physical_state_digest
        );
        assert_ne!(control.history_digest, intervention.history_digest);
        assert_ne!(control.canonical_state, intervention.canonical_state);
        assert!(control.physical_events > intervention.physical_events);
    }

    #[test]
    fn physical_suppression_changes_the_causal_trajectory_short() {
        let control_config = production_loop_config(9);
        let mut intervention_config = control_config.clone();
        intervention_config.pattern_schedule = intervention_config
            .pattern_schedule
            .with_suppression(SimulationTime::new(32), SimulationTime::new(64))
            .unwrap();
        let mut control = Runtime::new(control_config).unwrap();
        let mut intervention = Runtime::new(intervention_config).unwrap();
        let control = control.run_ticks(96).unwrap();
        let intervention = intervention.run_ticks(96).unwrap();
        assert_ne!(
            control.physical_state_digest,
            intervention.physical_state_digest
        );
        assert_ne!(control.history_digest, intervention.history_digest);
        assert_ne!(control.canonical_state, intervention.canonical_state);
        assert!(control.physical_events > intervention.physical_events);
    }

    #[test]
    fn identical_physical_state_can_have_different_history_digest() {
        let config = RuntimeConfig::new(13);
        let first = RuntimeState::new(&config).unwrap();
        let mut second = RuntimeState::new(&config).unwrap();
        let event = CausalEventProposal::new(
            EventProposalKey::new(99, 0, 0),
            EventKindId::new(ROOT_EVENT_KIND),
            vec![second.latest_physical_trace],
            vec![
                CausalEffect::new(
                    CausalTarget::new(
                        StateObjectKindId::new(RUNTIME_OBJECT_KIND),
                        99,
                        StatePropertyId::new(ROOT_PROPERTY),
                    ),
                    fingerprint_u64(0x0901, 0),
                    fingerprint_u64(0x0901, 1),
                )
                .unwrap(),
            ],
        )
        .unwrap();
        second
            .traces
            .commit_batch(SimulationTime::new(0), Phase::Physics, vec![event])
            .unwrap();
        let time = SimulationTime::new(0);
        assert_eq!(
            first.physical_state_digest(time),
            second.physical_state_digest(time)
        );
        assert_ne!(first.history_digest(), second.history_digest());
        assert_ne!(
            RuntimeState::canonical_state(
                first.physical_state_digest(time),
                first.history_digest()
            ),
            RuntimeState::canonical_state(
                second.physical_state_digest(time),
                second.history_digest()
            ),
        );
    }

    fn runtime_sample(
        pattern: PhysicalPatternId,
        tick: u64,
        ordinal: u32,
        cause: TraceId,
    ) -> PhysicalPatternSample {
        PhysicalPatternSample {
            chunk: test_chunk(),
            pattern,
            position: LocalCoord::new(1, 1, 1),
            observed_at: SimulationTime::new(tick),
            magnitude: 1_024,
            source_ordinal: ordinal,
            cause,
        }
    }

    fn run_mana_with_history(history_ticks: &[u64]) -> i64 {
        let config = RuntimeConfig::new(21);
        let mut state = RuntimeState::new(&config).unwrap();
        let pattern = PhysicalPatternId::new(21);
        let cause = state.latest_physical_trace;
        state
            .pending_samples
            .push(runtime_sample(pattern, 7, 100, cause));
        state.pattern_history.extend(
            history_ticks
                .iter()
                .enumerate()
                .map(|(index, tick)| runtime_sample(pattern, *tick, index as u32, cause)),
        );
        state
            .pattern_history
            .push(runtime_sample(pattern, 7, 100, cause));
        let state = Arc::new(Mutex::new(state));
        let mut system = ManaRuntimeSystem {
            state: Arc::clone(&state),
            next_time: SimulationTime::new(7),
            parameters: config.mana_parameters,
        };

        system.execute().unwrap();

        state.lock().unwrap().mana.total_intensity()
    }

    #[test]
    fn runtime_history_makes_periodic_and_irregular_sources_diverge() {
        let periodic_total = run_mana_with_history(&[3, 4, 5, 6]);
        let irregular_total = run_mana_with_history(&[1, 2, 4, 6]);

        assert!(periodic_total > irregular_total);
    }

    fn config_with_effect_threshold(seed: u64, threshold: i64, hysteresis: i64) -> RuntimeConfig {
        let mut config = RuntimeConfig::new(seed);
        config.mana_parameters.effect_threshold = threshold;
        config.mana_parameters.effect_hysteresis = hysteresis;
        config.actor_count = 1;
        config.sensor_count = 1;
        config.bootstrap_population = 8;
        config
    }

    fn config_with_actor(seed: u64) -> RuntimeConfig {
        let mut config = RuntimeConfig::new(seed);
        config.actor_count = 1;
        config.sensor_count = 1;
        config.action_bounds = 8;
        config.bootstrap_population = 8;
        config
    }

    fn production_loop_config(seed: u64) -> RuntimeConfig {
        config_with_actor(seed)
    }

    fn insert_actor_object(runtime: &Runtime, key: u64, x: i64) {
        let mut state = runtime.lock_state().unwrap();
        let trace = state.latest_physical_trace;
        state.actor_objects.insert(
            key,
            ActorPhysicalObject::new(key, WorldCoord::new(x, 0, 0), 9, trace),
        );
    }

    #[test]
    fn actor_loop_commits_physical_outcome_after_subjective_scene() {
        let mut runtime = Runtime::new(config_with_actor(41)).unwrap();
        insert_actor_object(&runtime, 100, 2);
        let initial_position = runtime
            .lock_state()
            .unwrap()
            .actors
            .get(&ActorId::new(1))
            .unwrap()
            .body
            .position;

        let snapshot = runtime.run_ticks(1).unwrap();
        let state = runtime.lock_state().unwrap();
        let actor = state.actors.get(&ActorId::new(1)).unwrap();

        assert_eq!(snapshot.actor_count, 1);
        assert!(snapshot.perceived_actor_features > 0);
        assert!(snapshot.subjective_actor_objects > 0);
        assert_eq!(snapshot.actor_actions_committed, 1);
        assert_eq!(
            actor.body.position,
            WorldCoord::new(
                initial_position.x.saturating_add(1),
                initial_position.y,
                initial_position.z
            )
        );
        assert!(
            state
                .traces
                .iter()
                .any(|event| event.phase == Phase::Action)
        );
    }

    #[test]
    fn inaccessible_actor_state_cannot_reach_cognition() {
        let mut runtime = Runtime::new(config_with_actor(42)).unwrap();
        {
            let mut state = runtime.lock_state().unwrap();
            let trace = state.latest_physical_trace;
            state.actor_objects.insert(
                1,
                ActorPhysicalObject {
                    object_key: 1,
                    position: WorldCoord::new(2, 0, 0),
                    magnitude: 9,
                    accessible: false,
                    occluded: false,
                    trace,
                },
            );
            state.actor_objects.insert(
                2,
                ActorPhysicalObject {
                    object_key: 2,
                    position: WorldCoord::new(3, 0, 0),
                    magnitude: 9,
                    accessible: true,
                    occluded: true,
                    trace,
                },
            );
            state.actor_objects.insert(
                3,
                ActorPhysicalObject::new(3, WorldCoord::new(99, 0, 0), 9, trace),
            );
        }

        let snapshot = runtime.run_ticks(1).unwrap();
        let state = runtime.lock_state().unwrap();
        let actor = state.actors.get(&ActorId::new(1)).unwrap();

        assert_eq!(snapshot.perceived_actor_features, 0);
        assert_eq!(snapshot.subjective_actor_objects, 0);
        assert_eq!(snapshot.actor_actions_committed, 0);
        assert!(actor.subjective_scene.as_ref().unwrap().objects.is_empty());
    }

    #[test]
    fn subjective_scene_debug_contains_no_authoritative_identity_meaning() {
        let mut runtime = Runtime::new(config_with_actor(43)).unwrap();
        insert_actor_object(&runtime, 100, 2);
        runtime.run_ticks(1).unwrap();
        let state = runtime.lock_state().unwrap();
        let actor = state.actors.get(&ActorId::new(1)).unwrap();
        let scene_text = format!("{:?}", actor.subjective_scene.as_ref().unwrap());

        assert!(!scene_text.contains("EntityId("));
        assert!(!scene_text.contains("PlaceId("));
        assert!(!scene_text.contains("BodySegmentId("));
        assert!(!scene_text.contains("SpatialChartId("));
        assert!(!scene_text.contains("LocalFrameId("));
        assert!(!scene_text.contains("TraceId("));
    }

    #[test]
    fn actor_loop_replays_exactly_with_same_seed_and_inputs() {
        let mut first = Runtime::new(config_with_actor(44)).unwrap();
        let mut second = Runtime::new(config_with_actor(44)).unwrap();
        insert_actor_object(&first, 100, 2);
        insert_actor_object(&second, 100, 2);

        let first = first.run_ticks(4).unwrap();
        let second = second.run_ticks(4).unwrap();

        assert_eq!(first, second);
        assert_eq!(
            first.actor_actions_committed,
            second.actor_actions_committed
        );
    }

    #[test]
    fn production_runtime_does_not_create_demo_residents() {
        let runtime = Runtime::from_seed(45).unwrap();
        let snapshot = runtime.snapshot().unwrap();
        let state = runtime.lock_state().unwrap();

        assert_eq!(snapshot.actor_count, 0);
        assert!(state.actors.is_empty());
    }

    #[test]
    fn observer_world_projection_is_bounded_chart_qualified_and_causal() {
        let mut config = RuntimeConfig::new(450);
        config.bootstrap_population = 128;
        let mut runtime = Runtime::new(config).unwrap();
        let runtime_snapshot = runtime.run_ticks(2).unwrap();

        let world = runtime.observer_world_snapshot().unwrap();
        assert_eq!(world.time, SimulationTime::new(2));
        assert!(!world.chunks.is_empty());
        assert!(world.chunks.len() <= 9);
        assert!(world.chunks.iter().all(|chunk| chunk.chart_id == 1));
        let projected_population = world
            .chunks
            .iter()
            .map(|chunk| chunk.population_total)
            .sum::<u64>();
        assert_eq!(projected_population, runtime_snapshot.population_total);
        assert!(projected_population <= 128);
        assert!(world.chunks.iter().all(|chunk| {
            chunk.minimum_elevation_mm <= chunk.maximum_elevation_mm && chunk.latest_trace.raw() > 0
        }));
        assert!(world.chunks.windows(2).all(|pair| {
            (
                pair[0].chart_id,
                pair[0].chunk_x,
                pair[0].chunk_y,
                pair[0].chunk_z,
            ) < (
                pair[1].chart_id,
                pair[1].chunk_x,
                pair[1].chunk_y,
                pair[1].chunk_z,
            )
        }));
    }

    #[test]
    fn mistaken_subjective_identity_can_still_drive_valid_physical_action() {
        let mut config = config_with_actor(46);
        config.material_surface_signals_enabled = false;
        let mut runtime = Runtime::new(config).unwrap();
        insert_actor_object(&runtime, 100, 2);
        let initial_position = runtime
            .lock_state()
            .unwrap()
            .actors
            .get(&ActorId::new(1))
            .unwrap()
            .body
            .position;
        runtime.run_ticks(1).unwrap();
        let first_id = {
            let state = runtime.lock_state().unwrap();
            state
                .actors
                .get(&ActorId::new(1))
                .unwrap()
                .subjective_scene
                .as_ref()
                .unwrap()
                .objects[0]
                .id
        };
        {
            let mut state = runtime.lock_state().unwrap();
            let trace = state.latest_physical_trace;
            state.actor_objects.clear();
            state.actor_objects.insert(
                200,
                ActorPhysicalObject::new(200, WorldCoord::new(3, 0, 0), 9, trace),
            );
        }

        runtime.run_ticks(1).unwrap();
        let state = runtime.lock_state().unwrap();
        let actor = state.actors.get(&ActorId::new(1)).unwrap();
        let second_id = actor.subjective_scene.as_ref().unwrap().objects[0].id;

        assert_eq!(first_id, second_id);
        assert_eq!(
            actor.body.position,
            WorldCoord::new(
                initial_position.x.saturating_add(2),
                initial_position.y,
                initial_position.z
            )
        );
        assert_eq!(state.actor_actions_committed, 2);
        assert!(state.actor_objects.contains_key(&200));
    }

    #[test]
    fn invalid_actor_action_is_rejected_with_trace() {
        let mut config = config_with_actor(47);
        config.action_bounds = 0;
        let mut runtime = Runtime::new(config).unwrap();
        insert_actor_object(&runtime, 100, 2);

        let snapshot = runtime.run_ticks(1).unwrap();
        let state = runtime.lock_state().unwrap();
        let actor = state.actors.get(&ActorId::new(1)).unwrap();

        assert_eq!(snapshot.actor_actions_committed, 0);
        assert_eq!(snapshot.actor_actions_rejected, 1);
        assert!(matches!(
            actor.validation_results.as_slice(),
            [ActionValidationResult::Invalid {
                cause: ActionRejection::OutOfBounds,
                ..
            }]
        ));
        assert!(
            state
                .traces
                .iter()
                .any(|event| event.kind == EventKindId::new(ACTOR_REJECTION_EVENT_KIND))
        );
    }

    #[test]
    fn mana_effect_system_commits_material_surface_trace_above_threshold() {
        let mut runtime = Runtime::new(config_with_effect_threshold(31, 1, 0)).unwrap();
        let snapshot = runtime.run_ticks(8).unwrap();
        let state = runtime.lock_state().unwrap();

        assert!(snapshot.mana_total > 1);
        assert!(snapshot.mana_physical_effects > 0);
        assert!(state.traces.iter().any(|event| {
            event.kind == EventKindId::new(MATERIAL_SURFACE_MANA_EVENT_KIND)
                && event.phase == Phase::Mana
                && event.effects.iter().any(|effect| {
                    effect.target().object_kind()
                        == StateObjectKindId::new(MATERIAL_SURFACE_OBJECT_KIND)
                        && effect.target().property()
                            == StatePropertyId::new(MATERIAL_SURFACE_CONDITION_PROPERTY)
                })
        }));
    }

    #[test]
    fn committed_mana_effect_changes_later_physical_samples() {
        let mut enabled = Runtime::new(config_with_effect_threshold(32, 1, 0)).unwrap();
        let mut disabled = Runtime::new(config_with_effect_threshold(32, 0, 0)).unwrap();
        enabled.run_ticks(8).unwrap();
        disabled.run_ticks(8).unwrap();
        let enabled_state = enabled.lock_state().unwrap();
        let disabled_state = disabled.lock_state().unwrap();
        let mana_effect = enabled_state
            .traces
            .iter()
            .find(|event| event.kind == EventKindId::new(MATERIAL_SURFACE_MANA_EVENT_KIND))
            .unwrap();
        let effected_object = mana_effect.effects[0].target().object_id();
        let effected_surface = enabled_state
            .material_surfaces
            .iter()
            .find_map(|(id, surface)| {
                (material_surface_object_id(*id) == effected_object).then_some((*id, *surface))
            })
            .unwrap();
        let enabled_condition = enabled_state
            .material_surfaces
            .get(&effected_surface.0)
            .map(|surface| surface.condition)
            .unwrap();
        let disabled_condition = disabled_state
            .material_surfaces
            .get(&effected_surface.0)
            .map(|surface| surface.condition)
            .unwrap();

        assert!(enabled_state.mana_physical_effects > 0);
        assert!(enabled_condition > disabled_condition);
        assert!(enabled_state.pattern_history.samples().any(|sample| {
            enabled_state
                .traces
                .event(sample.cause)
                .is_some_and(|event| event.causes.contains(&mana_effect.trace_id))
        }));
    }

    #[test]
    fn below_threshold_mana_produces_no_physical_effects() {
        let mut runtime =
            Runtime::new(config_with_effect_threshold(33, 1_000_000_000, 100)).unwrap();
        let snapshot = runtime.run_ticks(32).unwrap();

        assert!(snapshot.mana_total < 1_000_000_000);
        assert_eq!(snapshot.mana_physical_effects, 0);
    }

    #[test]
    fn disabled_effect_threshold_preserves_deterministic_field_evolution() {
        let mut first = Runtime::new(config_with_effect_threshold(34, 0, 0)).unwrap();
        let mut second = Runtime::new(config_with_effect_threshold(34, 0, 0)).unwrap();
        let first = first.run_ticks(64).unwrap();
        let second = second.run_ticks(64).unwrap();

        assert!(first.mana_total > 0);
        assert_eq!(first.mana_physical_effects, 0);
        assert_eq!(second.mana_physical_effects, 0);
        assert_eq!(first, second);
    }

    #[test]
    fn same_seed_replay_is_preserved_with_mana_effects_active() {
        let mut first = Runtime::new(config_with_effect_threshold(35, 1, 0)).unwrap();
        let mut second = Runtime::new(config_with_effect_threshold(35, 1, 0)).unwrap();
        let first = first.run_ticks(96).unwrap();
        let second = second.run_ticks(96).unwrap();

        assert!(first.mana_physical_effects > 0);
        assert_eq!(first, second);
    }

    #[test]
    fn mana_material_surface_effect_is_bounded_by_hysteresis() {
        let config = config_with_effect_threshold(36, 2, 1);
        let mut runtime = Runtime::new(config).unwrap();
        let snapshot = runtime.run_ticks(128).unwrap();
        let state = runtime.lock_state().unwrap();

        assert!(snapshot.mana_physical_effects > 0);
        assert_eq!(snapshot.mana_physical_effects, 3);
        assert!(
            state
                .material_surfaces
                .values()
                .all(|surface| surface.condition >= 1)
        );
    }

    #[test]
    fn trace_descends_from_covers_dag_topologies() {
        use super::{EventProposalKey, Phase};
        use causafera_core::provenance::{CausalEventProposal, CausalTraceStore};
        use causafera_types::ids::EventKindId;
        use causafera_types::time::SimulationTime;

        let mut store = CausalTraceStore::new();

        let mut commit = |causes: Vec<causafera_types::ids::TraceId>, obj: u64| {
            use causafera_core::{CausalTarget, StateFingerprint};
            use causafera_types::ids::{StateObjectKindId, StatePropertyId};
            let key = EventProposalKey::new(1, obj, 0);
            let dummy_effect = causafera_core::provenance::CausalEffect::new(
                CausalTarget::new(StateObjectKindId::new(1), 1, StatePropertyId::new(1)),
                StateFingerprint::new([1; 32]),
                StateFingerprint::new([2; 32]),
            )
            .unwrap();
            let proposal =
                CausalEventProposal::new(key, EventKindId::new(1), causes, vec![dummy_effect])
                    .unwrap();
            let mut committed = store
                .commit_batch(SimulationTime::new(1), Phase::Mana, vec![proposal])
                .unwrap();
            committed.pop().unwrap()
        };

        let a = commit(vec![], 1);
        let b = commit(vec![a], 2);
        let c = commit(vec![a], 3);
        let d = commit(vec![b, c], 4);
        let e = commit(vec![d], 5);
        let unrelated = commit(vec![], 6);

        // candidate equals ancestor
        assert!(super::trace_descends_from(&store, a, a));
        assert!(super::trace_descends_from(&store, d, d));

        // candidate older than ancestor (impossible ordering structurally, but tested here)
        assert!(!super::trace_descends_from(&store, a, e));

        // direct and transitive ancestry
        assert!(super::trace_descends_from(&store, b, a));
        assert!(super::trace_descends_from(&store, e, a));
        assert!(super::trace_descends_from(&store, d, c));

        // unrelated branches
        assert!(!super::trace_descends_from(&store, b, c));
        assert!(!super::trace_descends_from(&store, c, b));
        assert!(!super::trace_descends_from(&store, e, unrelated));
        assert!(!super::trace_descends_from(&store, unrelated, a));

        // multi-parent DAG ancestry
        assert!(super::trace_descends_from(&store, d, a));
    }

    #[test]
    fn gate_transition_history_uses_trace_store_for_evicted_predecessors() {
        use causafera_core::provenance::{CausalEffect, CausalEventProposal, CausalTraceStore};
        use causafera_types::{
            ChartChunkCoord, ChunkCoord, EventKindId, SimulationTime, SpatialChartId,
            StateObjectKindId, StatePropertyId,
        };

        let surface_id = MaterialSurfaceId::new(
            ChartChunkCoord::new(SpatialChartId::new(1), ChunkCoord::new(0, 0, 0)),
            0,
        );
        let object_id = material_surface_object_id(surface_id);
        let object_kind = StateObjectKindId::new(MATERIAL_SURFACE_OBJECT_KIND);
        let condition_property = StatePropertyId::new(MATERIAL_SURFACE_CONDITION_PROPERTY);
        let gate_property = StatePropertyId::new(MATERIAL_SURFACE_MANA_GATE_PROPERTY);
        let mana_object_kind = StateObjectKindId::new(MANA_OBJECT_KIND);
        let mana_property = StatePropertyId::new(MANA_PROPERTY);
        let mana_object_id = cell_object_id(surface_id.chunk, surface_id.cell_index);

        let mut store = CausalTraceStore::new();
        let root = CausalEventProposal::new(
            EventProposalKey::new(0, 0, 0),
            EventKindId::new(ROOT_EVENT_KIND),
            Vec::new(),
            vec![
                CausalEffect::new(
                    CausalTarget::new(
                        StateObjectKindId::new(RUNTIME_OBJECT_KIND),
                        0,
                        StatePropertyId::new(ROOT_PROPERTY),
                    ),
                    fingerprint_u64(0x0901, 0),
                    fingerprint_u64(0x0901, 1),
                )
                .unwrap(),
            ],
        )
        .unwrap();
        let root_trace = store
            .commit_batch(SimulationTime::new(0), Phase::Physics, vec![root])
            .unwrap()[0];

        let surface_at = |condition: i64, contact_count: u64| MaterialSurface {
            condition,
            contact_count,
            last_transition: root_trace,
            last_contact_trace: None,
            gate: MaterialSurfaceManaGate {
                active: false,
                last_transition: None,
            },
        };

        let contact = CausalEventProposal::new(
            EventProposalKey::new(1, object_id, 0),
            EventKindId::new(MATERIAL_SURFACE_CONTACT_EVENT_KIND),
            vec![root_trace],
            vec![
                CausalEffect::new(
                    CausalTarget::new(object_kind, object_id, condition_property),
                    material_surface_fingerprint(surface_at(0, 0)),
                    material_surface_fingerprint(surface_at(1, 1)),
                )
                .unwrap(),
            ],
        )
        .unwrap();
        let contact_trace = store
            .commit_batch(SimulationTime::new(1), Phase::Action, vec![contact])
            .unwrap()[0];

        let local_mana = CausalEventProposal::new(
            EventProposalKey::new(2, mana_object_id, 0),
            EventKindId::new(MANA_EVENT_KIND),
            vec![root_trace],
            vec![
                CausalEffect::new(
                    CausalTarget::new(mana_object_kind, mana_object_id, mana_property),
                    fingerprint_i64(0x0301, 0),
                    fingerprint_i64(0x0301, 3),
                )
                .unwrap(),
            ],
        )
        .unwrap();
        let local_mana_trace = store
            .commit_batch(SimulationTime::new(2), Phase::Mana, vec![local_mana])
            .unwrap()[0];

        let rising = CausalEventProposal::new(
            EventProposalKey::new(3, object_id, 0),
            EventKindId::new(MATERIAL_SURFACE_MANA_EVENT_KIND),
            vec![contact_trace, local_mana_trace],
            vec![
                CausalEffect::new(
                    CausalTarget::new(object_kind, object_id, condition_property),
                    material_surface_fingerprint(surface_at(1, 1)),
                    material_surface_fingerprint(surface_at(2, 1)),
                )
                .unwrap(),
                CausalEffect::new(
                    CausalTarget::new(object_kind, object_id, gate_property),
                    material_surface_gate_fingerprint(false),
                    material_surface_gate_fingerprint(true),
                )
                .unwrap(),
            ],
        )
        .unwrap();
        let rising_trace = store
            .commit_batch(SimulationTime::new(3), Phase::Mana, vec![rising])
            .unwrap()[0];

        let falling = CausalEventProposal::new(
            EventProposalKey::new(4, object_id, 0),
            EventKindId::new(MATERIAL_SURFACE_MANA_EVENT_KIND),
            vec![local_mana_trace, rising_trace],
            vec![
                CausalEffect::new(
                    CausalTarget::new(object_kind, object_id, gate_property),
                    material_surface_gate_fingerprint(true),
                    material_surface_gate_fingerprint(false),
                )
                .unwrap(),
            ],
        )
        .unwrap();
        let falling_trace = store
            .commit_batch(SimulationTime::new(4), Phase::Mana, vec![falling])
            .unwrap()[0];

        let surface = MaterialSurface {
            condition: 2,
            contact_count: 1,
            last_transition: rising_trace,
            last_contact_trace: Some(contact_trace),
            gate: MaterialSurfaceManaGate {
                active: false,
                last_transition: Some(falling_trace),
            },
        };
        let mut surfaces = BTreeMap::new();
        surfaces.insert(surface_id, surface);

        let gate_transitions = vec![MaterialSurfaceGateTransition {
            id: surface_id,
            occurred_at: SimulationTime::new(4),
            before_active: true,
            after_active: false,
            local_mana_before: 3,
            local_mana_after: 0,
            local_mana_trace,
            contact_trace: None,
            transition_trace: falling_trace,
        }];

        validate_material_surface_gate_transition_history(
            &store,
            &surfaces,
            &[],
            &gate_transitions,
        )
        .expect("evicted predecessor must be resolved from the authoritative trace store");
    }
}
