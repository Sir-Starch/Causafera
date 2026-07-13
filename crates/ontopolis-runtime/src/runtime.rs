use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{Arc, Mutex, MutexGuard},
};

use ontopolis_core::{
    CausalCommitError, CausalEffect, CausalEffectError, CausalEventProposal,
    CausalEventProposalError, CausalTarget, CausalTraceSnapshot, CausalTraceStore,
    DeterministicConfig, EventProposalKey, Phase, RandomStream, Scheduler, StateFingerprint,
    System,
};
use ontopolis_domains::{
    ManaError, ManaField, ManaFieldSet, ManaFieldSetSnapshot, ManaParameters,
    ManaPhysicalEffectProposal, ManaPhysicalEffectSchemaId, PhysicalCarrierAdapter,
    PhysicalPatternSample,
};
use ontopolis_observer_api::ObserverSnapshot;
use ontopolis_resolution::{
    CausalRelevanceSignal, ChannelWeight, ResolutionError, ResolutionField,
    ResolutionFieldSnapshot, ResolutionPolicy, ResolutionPolicySnapshot,
};
use ontopolis_types::{
    ChartChunkCoord, ChunkCoord, EventKindId, HistoricalStageId, ManaFieldId, ResolutionChannelId,
    ResolutionFieldId, SimulationTime, SpatialChartId, StateObjectKindId, StatePropertyId, TraceId,
    WorldCoord,
};
use thiserror::Error;

use crate::{
    ActionKindId, ActionRejection, ActionValidationResult, ActorId, ActorObjectiveSnapshot,
    ActorPhysicalObject, ActorRuntimeConfig, ActorState, ActorSubjectiveSnapshot, MinimalBodyState,
    PatternHistorySnapshot, PhysicalPatternHistory, TerrainCarrierAdapter, TerrainCarrierSnapshot,
    actor_cognition_step, actor_perception_step, actor_state_fingerprint, apply_action,
    deterministic_terrain_chunk, fixture_actors, fixture_sensors, validate_action,
};

pub const MAX_RUNTIME_TICKS: u64 = 1_000_000;
pub const MAX_PATTERN_HISTORY_ENTRIES: usize = 512;
pub const MAX_PATTERN_HISTORY_PER_PATTERN: usize = 128;
pub const MANA_PATTERN_HISTORY_TICKS: u64 = 8;

pub const CURRENT_DIGEST_SCHEMA_VERSION: DigestSchemaVersion = DigestSchemaVersion::new(1);

const PHYSICAL_SYSTEM_ID: u64 = 10;
const MANA_SYSTEM_ID: u64 = 20;
const MANA_EFFECTS_SYSTEM_ID: u64 = 21;
const RESOLUTION_SYSTEM_ID: u64 = 30;
const ACTOR_ACTION_SYSTEM_ID: u64 = 42;
const LIFECYCLE_SYSTEM_ID: u64 = 60;
const BOOTSTRAP_SYSTEM_ID: u64 = 61;
const ROOT_EVENT_KIND: u64 = 1;
const PHYSICAL_EVENT_KIND: u64 = 2;
const MANA_EVENT_KIND: u64 = 3;
const RESOLUTION_EVENT_KIND: u64 = 4;
const MANA_PHYSICAL_EFFECT_EVENT_KIND: u64 = 5;
const ACTOR_ACTION_EVENT_KIND: u64 = 6;
const ACTOR_REJECTION_EVENT_KIND: u64 = 7;
const POPULATION_BOOTSTRAP_EVENT_KIND: u64 = 8;
const POPULATION_LIFECYCLE_EVENT_KIND: u64 = 9;
const ACTOR_PROMOTION_EVENT_KIND: u64 = 10;
const ACTOR_DEMOTION_EVENT_KIND: u64 = 11;
const MATERIAL_ACTIVITY_EVENT_KIND: u64 = 12;
const RUNTIME_OBJECT_KIND: u64 = 1;
const PHYSICAL_OBJECT_KIND: u64 = 2;
const MANA_OBJECT_KIND: u64 = 3;
const RESOLUTION_OBJECT_KIND: u64 = 4;
const ACTOR_OBJECT_KIND: u64 = 5;
const POPULATION_OBJECT_KIND: u64 = 6;
const MATERIAL_OBJECT_KIND: u64 = 7;
const ROOT_PROPERTY: u64 = 1;
const PHYSICAL_PROPERTY: u64 = 2;
const MANA_PROPERTY: u64 = 3;
const RESOLUTION_PROPERTY: u64 = 4;
const MANA_PHYSICAL_EFFECT_PROPERTY: u64 = 5;
const ACTOR_BODY_PROPERTY: u64 = 6;
const ACTOR_REJECTION_PROPERTY: u64 = 7;
const POPULATION_AGGREGATE_PROPERTY: u64 = 8;
const ACTOR_PROMOTION_PROPERTY: u64 = 9;
const MATERIAL_FLOW_PROPERTY: u64 = 10;
const RESOLUTION_CHANNEL: u64 = 1;
const MANA_PHYSICAL_EFFECT_SCHEMA: u64 = 1;
const MAX_MANA_PHYSICAL_EFFECT_BOOST: u32 = 8;
const MANA_EFFECT_MAGNITUDE_STEP: u32 = 256;
const PHYSICAL_DIGEST_DOMAIN: u64 = 0x5048_5953_4943_414C;
const HISTORY_DIGEST_DOMAIN: u64 = 0x4849_5354_4F52_595F;
const EXPERIMENT_DIGEST_DOMAIN: u64 = 0x4558_5045_5249_4D45;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DigestSchemaVersion(u16);

impl DigestSchemaVersion {
    pub const fn new(raw: u16) -> Self {
        Self(raw)
    }

    pub const fn raw(self) -> u16 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PhysicalStateDigest {
    pub schema_version: DigestSchemaVersion,
    pub fingerprint: StateFingerprint,
}

impl PhysicalStateDigest {
    pub const fn bytes(self) -> [u8; 32] {
        self.fingerprint.bytes()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HistoryDigest {
    pub schema_version: DigestSchemaVersion,
    pub fingerprint: StateFingerprint,
}

impl HistoryDigest {
    pub const fn bytes(self) -> [u8; 32] {
        self.fingerprint.bytes()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ExperimentDigest {
    pub schema_version: DigestSchemaVersion,
    pub fingerprint: StateFingerprint,
}

impl ExperimentDigest {
    pub const fn bytes(self) -> [u8; 32] {
        self.fingerprint.bytes()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PhysicalPatternSchedule {
    pub interval_ticks: u64,
    pub magnitude: u32,
    pub suppressed_from: Option<SimulationTime>,
    pub suppressed_through: Option<SimulationTime>,
}

impl PhysicalPatternSchedule {
    pub const fn continuous(magnitude: u32) -> Self {
        Self {
            interval_ticks: 1,
            magnitude,
            suppressed_from: None,
            suppressed_through: None,
        }
    }

    pub fn with_suppression(
        mut self,
        from: SimulationTime,
        through: SimulationTime,
    ) -> Result<Self, RuntimeError> {
        if from.raw() == 0 || through < from {
            return Err(RuntimeError::InvalidPatternSchedule);
        }
        self.suppressed_from = Some(from);
        self.suppressed_through = Some(through);
        Ok(self)
    }

    fn validate(self) -> Result<Self, RuntimeError> {
        if self.interval_ticks == 0 || self.magnitude == 0 {
            return Err(RuntimeError::InvalidPatternSchedule);
        }
        match (self.suppressed_from, self.suppressed_through) {
            (None, None) => {}
            (Some(from), Some(through)) if from.raw() > 0 && through >= from => {}
            _ => return Err(RuntimeError::InvalidPatternSchedule),
        }
        Ok(self)
    }

    fn emits_at(self, time: SimulationTime) -> bool {
        if time.raw() % self.interval_ticks != 0 {
            return false;
        }
        !matches!(
            (self.suppressed_from, self.suppressed_through),
            (Some(from), Some(through)) if time >= from && time <= through
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeConfig {
    pub deterministic: DeterministicConfig,
    pub chunk_extent: u8,
    pub active_chunk_radius: u8,
    pub chart_id: SpatialChartId,
    pub pattern_schedule: PhysicalPatternSchedule,
    pub mana_parameters: ManaParameters,
    pub carrier_adapter: CarrierAdapterConfig,
    pub actor_count: u8,
    pub sensor_count: u8,
    pub action_bounds: i64,
    pub bootstrap_population: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CarrierAdapterConfig {
    TerrainSeed { terrain_seed: u64 },
}

impl CarrierAdapterConfig {
    pub const fn terrain_seed(terrain_seed: u64) -> Self {
        Self::TerrainSeed { terrain_seed }
    }
}

impl RuntimeConfig {
    pub fn new(world_seed: u64) -> Self {
        Self {
            deterministic: DeterministicConfig::new(world_seed),
            chunk_extent: 3,
            active_chunk_radius: 1,
            chart_id: SpatialChartId::new(1),
            pattern_schedule: PhysicalPatternSchedule::continuous(1_024),
            mana_parameters: ManaParameters {
                base_response: 128,
                recurrence_response: 128,
                periodicity_response: 128,
                synchrony_response: 128,
                spatial_response: 128,
                diffusion: 128,
                decay: 24,
                maximum_intensity: 1_000_000,
                effect_threshold: 16_000,
                effect_hysteresis: 2_000,
            },
            carrier_adapter: CarrierAdapterConfig::terrain_seed(world_seed),
            actor_count: 0,
            sensor_count: 0,
            action_bounds: 8,
            bootstrap_population: 0,
        }
    }

    fn validate(self) -> Result<Self, RuntimeError> {
        if self.chunk_extent < 3 {
            return Err(RuntimeError::InvalidFieldExtent);
        }
        if self.active_chunk_radius > 4 {
            return Err(RuntimeError::InvalidActiveChunkRadius);
        }
        self.pattern_schedule.validate()?;
        self.mana_parameters.validate()?;
        if self.actor_count > 128
            || self.sensor_count > 16
            || self.action_bounds < 0
            || self.bootstrap_population > 10_000
        {
            return Err(RuntimeError::InvalidActorConfig);
        }
        Ok(self)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RuntimeSnapshot {
    pub time: SimulationTime,
    pub physical_state_digest: PhysicalStateDigest,
    pub history_digest: HistoryDigest,
    pub canonical_state: ExperimentDigest,
    pub mana_total: i64,
    pub mana_maximum: i64,
    pub mana_changed_components: u32,
    pub active_chunk_count: u32,
    pub resolution_relevance: i64,
    pub resolution_level: u8,
    pub causal_trace_count: u64,
    pub physical_events: u64,
    pub mana_cell_changes: u64,
    pub mana_physical_effects: u64,
    pub mana_physical_effect_boost: u32,
    pub resolution_changes: u64,
    pub resolution_transitions: u64,
    pub actor_count: u32,
    pub perceived_actor_features: u64,
    pub subjective_actor_objects: u64,
    pub actor_actions_committed: u64,
    pub actor_actions_rejected: u64,
    pub population_total: u64,
    pub population_births: u64,
    pub population_deaths: u64,
    pub population_movements: u64,
    pub actor_promotions: u64,
    pub actor_demotions: u64,
    pub material_activity_events: u64,
    pub bytes_per_chunk: u64,
    pub latest_trace: TraceId,
}

impl RuntimeSnapshot {
    /// Builds the bounded, read-only observer projection. Locale and delivery state are
    /// deliberately absent so observation cannot alter authoritative identity.
    pub fn observer_snapshot(&self) -> ObserverSnapshot {
        ObserverSnapshot {
            time: self.time,
            digest_schema_version: u32::from(self.physical_state_digest.schema_version.raw()),
            physical_digest: self.physical_state_digest.bytes(),
            history_digest: self.history_digest.bytes(),
            mana_total: self.mana_total,
            mana_maximum: self.mana_maximum,
            active_chunk_count: self.active_chunk_count,
            resolution_relevance: self.resolution_relevance,
            resolution_level: u32::from(self.resolution_level),
            causal_trace_count: self.causal_trace_count,
            actor_count: self.actor_count,
            population_total: self.population_total,
            physical_events: self.physical_events,
            mana_cell_changes: self.mana_cell_changes,
            mana_physical_effects: self.mana_physical_effects,
            resolution_transitions: self.resolution_transitions,
            actor_actions_committed: self.actor_actions_committed,
            actor_actions_rejected: self.actor_actions_rejected,
            population_births: self.population_births,
            population_deaths: self.population_deaths,
            population_movements: self.population_movements,
            bytes_per_chunk: self.bytes_per_chunk,
            latest_trace: self.latest_trace,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct RuntimeSnapshotData {
    pub recipe: RuntimeRecipeSnapshot,
    pub spatial: SpatialChunkSnapshot,
    pub mana: ManaFieldSetSnapshot,
    pub resolution: ResolutionFieldSnapshot,
    pub resolution_policy: ResolutionPolicySnapshot,
    pub pattern_history: PatternHistorySnapshot,
    pub physical_counters: PhysicalCountersSnapshot,
    pub actors_objective: ActorObjectiveStateSnapshot,
    pub actors_subjective: ActorSubjectiveStateSnapshot,
    pub population: PopulationAggregateSnapshot,
    pub bootstrap: BootstrapReceiptSnapshot,
    pub traces: CausalTraceSnapshot,
    pub experiment_manifest: Option<ExperimentManifestSnapshot>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeRecipeSnapshot {
    pub seed: u64,
    pub config: RuntimeConfig,
    pub system_registrations: Vec<SystemRegistrationSnapshot>,
    pub completed_time: SimulationTime,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SystemRegistrationSnapshot {
    pub phase: Phase,
    pub system_schema_id: u64,
    pub revision: u16,
    pub registration_order: u16,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpatialChunkSnapshot {
    pub active_chunks: Vec<ActiveChunkSnapshot>,
    pub carrier_adapters: Vec<TerrainCarrierSnapshot>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActiveChunkSnapshot {
    pub chunk: ChartChunkCoord,
    pub relevance: i64,
    pub level: u8,
    pub total_mana: i64,
    pub event_count: u64,
    pub last_transition: Option<TraceId>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PhysicalCountersSnapshot {
    pub pending_samples: Vec<PhysicalPatternSample>,
    pub latest_physical_trace: TraceId,
    pub latest_mana_trace: Option<TraceId>,
    pub advanced_through: SimulationTime,
    pub physical_counter: u64,
    pub physical_events: u64,
    pub mana_cell_changes: u64,
    pub mana_physical_effects: u64,
    pub resolution_changes: u64,
    pub resolution_transitions: u64,
    pub perceived_actor_features: u64,
    pub subjective_actor_objects: u64,
    pub actor_actions_committed: u64,
    pub actor_actions_rejected: u64,
    pub population_births: u64,
    pub population_deaths: u64,
    pub population_movements: u64,
    pub actor_promotions: u64,
    pub actor_demotions: u64,
    pub material_activity_events: u64,
    pub next_actor_id: u64,
    pub last_mana_changes: u32,
    pub mana_effect_active: bool,
    pub physical_mana_effect_boost: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ActorObjectiveStateSnapshot {
    pub actors: Vec<(ActorId, ActorObjectiveSnapshot)>,
    pub actor_ancestry: Vec<(ActorId, Vec<TraceId>)>,
    pub actor_objects: Vec<(u64, ActorPhysicalObject)>,
    pub actor_action_bounds: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActorSubjectiveStateSnapshot {
    pub actors: Vec<(ActorId, ActorSubjectiveSnapshot)>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PopulationAggregateSnapshot {
    pub aggregates: Vec<PopulationAggregate>,
    pub aggregate_actor_pool: Vec<(ChartChunkCoord, Vec<ActorId>)>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BootstrapReceiptSnapshot {
    pub receipts: Vec<BootstrapReceiptRecord>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BootstrapReceiptRecord {
    pub stage: HistoricalStageId,
    pub trace: TraceId,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExperimentManifestSnapshot {
    pub format_version: u16,
    pub seed_set: Vec<u64>,
    pub checkpoint_interval: u64,
    pub bootstrap_population: u64,
    pub suppression_from: SimulationTime,
    pub suppression_through: SimulationTime,
    pub warm_up_ticks: u64,
    pub duration_ticks: u64,
    pub physical_digest: PhysicalStateDigest,
    pub history_digest: HistoryDigest,
    pub supporting_traces: Vec<TraceId>,
    pub evidence_sufficient: bool,
}

/// Headless deterministic runtime for the first executable causal experiment.
pub struct Runtime {
    scheduler: Scheduler,
    state: Arc<Mutex<RuntimeState>>,
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

    /// Reconstruct a full `Runtime` from a completed-tick snapshot.
    pub fn from_snapshot(data: RuntimeSnapshotData) -> Result<Self, RuntimeError> {
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
    #[error("resolution evolution failed: {0}")]
    Resolution(#[from] ResolutionError),
    #[error("actor initialization failed: {0}")]
    ActorInit(#[from] crate::ActorInitError),
    #[error("actor perception failed: {0}")]
    ActorPerception(#[from] ontopolis_perception::AcquisitionError),
    #[error("actor cognition failed: {0}")]
    ActorCognition(#[from] ontopolis_cognition::SceneUpdateError),
    #[error("snapshot is invalid: {0}")]
    InvalidSnapshot(&'static str),
}

impl From<ManaError> for RuntimeError {
    fn from(error: ManaError) -> Self {
        Self::Mana(error)
    }
}

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
pub struct HistoricalBootstrapPlan {
    pub physical_geography_init: TerrainBootstrapStage,
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
        traces.extend(self.population_lifecycle.bootstrap(state)?);
        traces.extend(self.actor_promotion.bootstrap(state)?);
        traces.extend(self.material_activity.bootstrap(state)?);
        Ok(traces)
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

pub struct RuntimeState {
    config: RuntimeConfig,
    traces: CausalTraceStore,
    mana: ManaFieldSet,
    resolution: ResolutionField,
    resolution_policy: ResolutionPolicy,
    carrier_adapters: BTreeMap<ChartChunkCoord, TerrainCarrierAdapter>,
    active_chunks: BTreeMap<ChartChunkCoord, ActiveChunkState>,
    actors: BTreeMap<ActorId, ActorState>,
    actor_ancestry: BTreeMap<ActorId, Vec<TraceId>>,
    actor_objects: BTreeMap<u64, ActorPhysicalObject>,
    population_aggregates: BTreeMap<ChartChunkCoord, PopulationAggregate>,
    aggregate_actor_pool: BTreeMap<ChartChunkCoord, Vec<ActorId>>,
    actor_action_bounds: i64,
    pending_samples: Vec<PhysicalPatternSample>,
    pattern_history: PhysicalPatternHistory,
    latest_physical_trace: TraceId,
    latest_mana_trace: Option<TraceId>,
    advanced_through: SimulationTime,
    physical_counter: u64,
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
    mana_effect_active: bool,
    physical_mana_effect_boost: u32,
    failure: Option<RuntimeError>,
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
    fn new(config: &RuntimeConfig) -> Result<Self, RuntimeError> {
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
        let (population_aggregates, aggregate_actor_pool, next_actor_id) =
            bootstrap_population_aggregates(
                &mut traces,
                config.bootstrap_population,
                &active_chunk_keys,
                root_trace,
            )?;
        let actor_config = ActorRuntimeConfig {
            actor_count: config.actor_count,
            sensor_count: config.sensor_count,
            action_bounds: config.action_bounds,
        };
        let actors = fixture_actors(actor_config)?;
        let next_actor_id = next_actor_id.max(u64::from(config.actor_count) + 1);
        Ok(Self {
            config: config.clone(),
            traces,
            mana,
            resolution,
            resolution_policy,
            carrier_adapters,
            active_chunks,
            actors,
            actor_ancestry: BTreeMap::new(),
            actor_objects: BTreeMap::new(),
            population_aggregates,
            aggregate_actor_pool,
            actor_action_bounds: actor_config.action_bounds,
            pending_samples: Vec::with_capacity(ontopolis_geography::TERRAIN_CELLS_PER_CHUNK),
            pattern_history: PhysicalPatternHistory::new(
                MAX_PATTERN_HISTORY_ENTRIES,
                MAX_PATTERN_HISTORY_PER_PATTERN,
            ),
            latest_physical_trace: root_trace,
            latest_mana_trace: None,
            advanced_through: SimulationTime::new(0),
            physical_counter: 0,
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
            next_actor_id,
            last_mana_changes: 0,
            mana_effect_active: false,
            physical_mana_effect_boost: 0,
            failure: None,
        })
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
            resolution: self.resolution.export_snapshot(),
            resolution_policy: self.resolution_policy.export_snapshot(),
            pattern_history: self.pattern_history.export_snapshot(),
            physical_counters: PhysicalCountersSnapshot {
                pending_samples: self.pending_samples.clone(),
                latest_physical_trace: self.latest_physical_trace,
                latest_mana_trace: self.latest_mana_trace,
                advanced_through: self.advanced_through,
                physical_counter: self.physical_counter,
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
                mana_effect_active: self.mana_effect_active,
                physical_mana_effect_boost: self.physical_mana_effect_boost,
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
            experiment_manifest: None,
        }
    }

    pub fn import_snapshot(data: RuntimeSnapshotData) -> Result<Self, RuntimeError> {
        let config = data.recipe.config.validate()?;
        let traces = CausalTraceStore::import_snapshot(data.traces)
            .map_err(|_| RuntimeError::InvalidSnapshot("trace store failed validation"))?;
        let mana = ManaFieldSet::import_snapshot(data.mana)?;
        let resolution = ResolutionField::import_snapshot(data.resolution)?;
        let resolution_policy = ResolutionPolicy::import_snapshot(data.resolution_policy)?;
        let carrier_adapters = import_carrier_adapters(data.spatial.carrier_adapters)?;
        let active_chunks = import_active_chunks(data.spatial.active_chunks)?;
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
        let counters = data.physical_counters;
        let state = Self {
            config,
            traces,
            mana,
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
            latest_physical_trace: counters.latest_physical_trace,
            latest_mana_trace: counters.latest_mana_trace,
            advanced_through: counters.advanced_through,
            physical_counter: counters.physical_counter,
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
            mana_effect_active: counters.mana_effect_active,
            physical_mana_effect_boost: counters.physical_mana_effect_boost,
            failure: None,
        };
        state.validate_snapshot_references()?;
        Ok(state)
    }

    fn validate_snapshot_references(&self) -> Result<(), RuntimeError> {
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
        for field in self.mana.fields().values() {
            for trace in field.last_change().iter().flatten().copied() {
                validate_trace_exists(&self.traces, trace)?;
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
            mana_physical_effect_boost: self.physical_mana_effect_boost,
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
        }
    }

    fn bytes_per_chunk(&self) -> u64 {
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
        digest.write(self.physical_counter);
        digest.write(u64::from(self.physical_mana_effect_boost));
        digest.write(if self.mana_effect_active { 1 } else { 0 });
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
        HistoryDigest {
            schema_version: CURRENT_DIGEST_SCHEMA_VERSION,
            fingerprint: digest.finish(),
        }
    }

    fn canonical_state(
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
            system_schema_id: MANA_SYSTEM_ID,
            revision: 1,
            registration_order: 1,
        },
        SystemRegistrationSnapshot {
            phase: Phase::Mana,
            system_schema_id: MANA_EFFECTS_SYSTEM_ID,
            revision: 1,
            registration_order: 2,
        },
        SystemRegistrationSnapshot {
            phase: Phase::Resolution,
            system_schema_id: RESOLUTION_SYSTEM_ID,
            revision: 1,
            registration_order: 3,
        },
        SystemRegistrationSnapshot {
            phase: Phase::Perception,
            system_schema_id: 40,
            revision: 1,
            registration_order: 4,
        },
        SystemRegistrationSnapshot {
            phase: Phase::Cognition,
            system_schema_id: 41,
            revision: 1,
            registration_order: 5,
        },
        SystemRegistrationSnapshot {
            phase: Phase::Action,
            system_schema_id: ACTOR_ACTION_SYSTEM_ID,
            revision: 1,
            registration_order: 6,
        },
        SystemRegistrationSnapshot {
            phase: Phase::Lifecycle,
            system_schema_id: LIFECYCLE_SYSTEM_ID,
            revision: 1,
            registration_order: 7,
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

struct PhysicalPatternSystem {
    state: Arc<Mutex<RuntimeState>>,
    schedule: PhysicalPatternSchedule,
    next_time: SimulationTime,
}

impl PhysicalPatternSystem {
    fn new(state: Arc<Mutex<RuntimeState>>, schedule: PhysicalPatternSchedule) -> Self {
        Self {
            state,
            schedule,
            next_time: SimulationTime::new(1),
        }
    }

    fn execute(&mut self) -> Result<(), RuntimeError> {
        let mut state = self.state.lock().map_err(|_| RuntimeError::StatePoisoned)?;
        state.pending_samples.clear();
        if self.schedule.emits_at(self.next_time) {
            let boost = state.physical_mana_effect_boost;
            let next_counter = state.physical_counter.saturating_add(1 + u64::from(boost));
            let event = CausalEventProposal::new(
                EventProposalKey::new(PHYSICAL_SYSTEM_ID, 0, 0),
                EventKindId::new(PHYSICAL_EVENT_KIND),
                vec![state.latest_physical_trace],
                vec![CausalEffect::new(
                    CausalTarget::new(
                        StateObjectKindId::new(PHYSICAL_OBJECT_KIND),
                        0,
                        StatePropertyId::new(PHYSICAL_PROPERTY),
                    ),
                    fingerprint_pair(0x0201, state.physical_counter as i64, i64::from(boost)),
                    fingerprint_pair(0x0201, next_counter as i64, i64::from(boost)),
                )?],
            )?;
            let trace = state
                .traces
                .commit_batch(self.next_time, Phase::Physics, vec![event])?[0];
            state.physical_counter = next_counter;
            state.physical_events += 1;
            state.latest_physical_trace = trace;
            let magnitude_boost = boost.saturating_mul(MANA_EFFECT_MAGNITUDE_STEP);
            let emitted = state
                .carrier_adapters
                .values()
                .flat_map(|adapter| adapter.emit_samples(self.next_time, trace))
                .map(|sample| PhysicalPatternSample {
                    magnitude: sample
                        .magnitude
                        .saturating_add(self.schedule.magnitude)
                        .saturating_add(magnitude_boost),
                    ..sample
                })
                .collect::<Vec<_>>();
            state.pending_samples.extend(emitted.iter().copied());
            state.pattern_history.extend(emitted);
        }
        self.next_time = self.next_time.tick();
        Ok(())
    }
}

impl System for PhysicalPatternSystem {
    fn run(&mut self, _stream: &mut RandomStream) {
        if let Err(error) = self.execute() {
            if let Ok(mut state) = self.state.lock() {
                state.failure.get_or_insert(error);
            }
        }
    }

    fn restore_time(&mut self, time: SimulationTime) {
        self.next_time = time;
    }
}

struct ManaRuntimeSystem {
    state: Arc<Mutex<RuntimeState>>,
    next_time: SimulationTime,
    parameters: ManaParameters,
}

impl ManaRuntimeSystem {
    fn new(state: Arc<Mutex<RuntimeState>>, parameters: ManaParameters) -> Self {
        Self {
            state,
            next_time: SimulationTime::new(1),
            parameters,
        }
    }

    fn execute(&mut self) -> Result<(), RuntimeError> {
        let mut state = self.state.lock().map_err(|_| RuntimeError::StatePoisoned)?;
        let pending_patterns = state
            .pending_samples
            .iter()
            .map(|sample| sample.pattern)
            .collect::<BTreeSet<_>>();
        let mut history = Vec::new();
        for pattern in pending_patterns {
            history.extend(
                state
                    .pattern_history
                    .get_window(pattern, MANA_PATTERN_HISTORY_TICKS)
                    .into_iter()
                    .filter(|sample| sample.observed_at < self.next_time),
            );
        }
        history.sort_unstable();
        let pending_samples = state.pending_samples.clone();
        let proposal = state.mana.propose_evolution(
            self.next_time,
            self.parameters,
            &pending_samples,
            &history,
        )?;
        let changes = proposal.changes();
        let events = changes
            .iter()
            .map(|(chunk, change)| {
                let object_id = cell_object_id(*chunk, change.cell_index);
                CausalEventProposal::new(
                    EventProposalKey::new(MANA_SYSTEM_ID, object_id, 0),
                    EventKindId::new(MANA_EVENT_KIND),
                    change.causes.clone(),
                    vec![CausalEffect::new(
                        CausalTarget::new(
                            StateObjectKindId::new(MANA_OBJECT_KIND),
                            object_id,
                            StatePropertyId::new(MANA_PROPERTY),
                        ),
                        fingerprint_i64(0x0301, change.before),
                        fingerprint_i64(0x0301, change.after),
                    )?],
                )
                .map_err(RuntimeError::from)
            })
            .collect::<Result<Vec<_>, _>>()?;
        let traces = state
            .traces
            .commit_batch(self.next_time, Phase::Mana, events)?;
        let changed_count = changes.len() as u32;
        let mut traces_by_chunk = BTreeMap::<ChartChunkCoord, Vec<TraceId>>::new();
        for ((chunk, _change), trace) in changes.iter().zip(traces.iter().copied()) {
            traces_by_chunk.entry(*chunk).or_default().push(trace);
        }
        state.mana = proposal.commit(&traces_by_chunk)?;
        state.pending_samples.clear();
        state.last_mana_changes = changed_count;
        state.mana_cell_changes = state
            .mana_cell_changes
            .saturating_add(u64::from(changed_count));
        if let Some(trace) = traces.last().copied() {
            state.latest_mana_trace = Some(trace);
        }
        self.next_time = self.next_time.tick();
        Ok(())
    }
}

struct ManaEffectsSystem {
    state: Arc<Mutex<RuntimeState>>,
    next_time: SimulationTime,
    parameters: ManaParameters,
}

impl ManaEffectsSystem {
    fn new(state: Arc<Mutex<RuntimeState>>, parameters: ManaParameters) -> Self {
        Self {
            state,
            next_time: SimulationTime::new(1),
            parameters,
        }
    }

    fn execute(&mut self) -> Result<(), RuntimeError> {
        let mut state = self.state.lock().map_err(|_| RuntimeError::StatePoisoned)?;
        let mana_total = state.mana.total_intensity();
        let active = mana_effect_active(
            state.mana_effect_active,
            mana_total,
            self.parameters.effect_threshold,
            self.parameters.effect_hysteresis,
        );
        let desired_boost = mana_effect_boost(active, mana_total, self.parameters.effect_threshold);
        state.mana_effect_active = active;
        if desired_boost == state.physical_mana_effect_boost {
            self.next_time = self.next_time.tick();
            return Ok(());
        }
        let Some(cause) = state.latest_mana_trace else {
            self.next_time = self.next_time.tick();
            return Ok(());
        };
        if state.traces.event(cause).is_none() {
            return Err(RuntimeError::UnknownManaPhysicalEffectCause { cause });
        }
        let target = CausalTarget::new(
            StateObjectKindId::new(PHYSICAL_OBJECT_KIND),
            0,
            StatePropertyId::new(MANA_PHYSICAL_EFFECT_PROPERTY),
        );
        let proposal = ManaPhysicalEffectProposal {
            schema: ManaPhysicalEffectSchemaId::new(MANA_PHYSICAL_EFFECT_SCHEMA),
            target,
            before: fingerprint_u64(0x0501, u64::from(state.physical_mana_effect_boost)),
            after: fingerprint_u64(0x0501, u64::from(desired_boost)),
            causes: vec![cause],
        };
        let event = CausalEventProposal::new(
            EventProposalKey::new(MANA_EFFECTS_SYSTEM_ID, proposal.schema.raw(), 0),
            EventKindId::new(MANA_PHYSICAL_EFFECT_EVENT_KIND),
            proposal.causes.clone(),
            vec![CausalEffect::new(
                proposal.target,
                proposal.before,
                proposal.after,
            )?],
        )?;
        let trace = state
            .traces
            .commit_batch(self.next_time, Phase::Mana, vec![event])?[0];
        state.physical_mana_effect_boost = desired_boost;
        state.mana_physical_effects = state.mana_physical_effects.saturating_add(1);
        state.latest_physical_trace = trace;
        self.next_time = self.next_time.tick();
        Ok(())
    }
}

impl System for ManaEffectsSystem {
    fn run(&mut self, _stream: &mut RandomStream) {
        if let Err(error) = self.execute() {
            if let Ok(mut state) = self.state.lock() {
                state.failure.get_or_insert(error);
            }
        }
    }

    fn restore_time(&mut self, time: SimulationTime) {
        self.next_time = time;
    }
}

impl System for ManaRuntimeSystem {
    fn run(&mut self, _stream: &mut RandomStream) {
        if let Err(error) = self.execute() {
            if let Ok(mut state) = self.state.lock() {
                state.failure.get_or_insert(error);
            }
        }
    }

    fn restore_time(&mut self, time: SimulationTime) {
        self.next_time = time;
    }
}

fn mana_effect_active(
    previously_active: bool,
    total: i64,
    threshold: i64,
    hysteresis: i64,
) -> bool {
    if threshold <= 0 {
        return false;
    }
    if previously_active {
        return total >= threshold.saturating_sub(hysteresis);
    }
    total > threshold
}

fn mana_effect_boost(active: bool, total: i64, threshold: i64) -> u32 {
    if !active || threshold <= 0 {
        return 0;
    }
    let excess = total.saturating_sub(threshold).max(0);
    let scaled = excess.saturating_div(threshold).saturating_add(1);
    u32::try_from(scaled)
        .unwrap_or(MAX_MANA_PHYSICAL_EFFECT_BOOST)
        .min(MAX_MANA_PHYSICAL_EFFECT_BOOST)
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

fn active_chunk_keys(chart_id: SpatialChartId, radius: u8) -> Vec<ChartChunkCoord> {
    let radius = i32::from(radius);
    (-radius..=radius)
        .map(|x| ChartChunkCoord::new(chart_id, ChunkCoord::new(x, 0, 0)))
        .collect()
}

struct ResolutionRuntimeSystem {
    state: Arc<Mutex<RuntimeState>>,
    next_time: SimulationTime,
}

impl ResolutionRuntimeSystem {
    fn new(state: Arc<Mutex<RuntimeState>>) -> Self {
        Self {
            state,
            next_time: SimulationTime::new(1),
        }
    }

    fn execute(&mut self) -> Result<(), RuntimeError> {
        let mut state = self.state.lock().map_err(|_| RuntimeError::StatePoisoned)?;
        let mana_totals = mana_totals_by_chunk(&state.mana);
        let event_counts = pattern_event_counts_by_chunk(&state.pattern_history);
        refresh_active_chunk_conserved_state(&mut state.active_chunks, &event_counts, &mana_totals);
        let signals = resolution_signals(&state, &mana_totals, self.next_time)?;
        let proposal = state.resolution.propose_evaluation(
            self.next_time,
            &state.resolution_policy,
            &signals,
        )?;
        let changes = proposal.changes().to_vec();
        let events = changes
            .iter()
            .map(|change| {
                let object_id = chart_chunk_hash(change.chunk);
                CausalEventProposal::new(
                    EventProposalKey::new(RESOLUTION_SYSTEM_ID, object_id, 0),
                    EventKindId::new(RESOLUTION_EVENT_KIND),
                    change.causes().to_vec(),
                    vec![CausalEffect::new(
                        CausalTarget::new(
                            StateObjectKindId::new(RESOLUTION_OBJECT_KIND),
                            object_id,
                            StatePropertyId::new(RESOLUTION_PROPERTY),
                        ),
                        fingerprint_pair(
                            0x0401,
                            change.before_relevance,
                            i64::from(change.before_level),
                        ),
                        fingerprint_pair(
                            0x0401,
                            change.after_relevance,
                            i64::from(change.after_level),
                        ),
                    )?],
                )
                .map_err(RuntimeError::from)
            })
            .collect::<Result<Vec<_>, _>>()?;
        let traces = state
            .traces
            .commit_batch(self.next_time, Phase::Resolution, events)?;
        state.resolution = proposal.commit(&traces)?;
        apply_resolution_transitions(&mut state.active_chunks, &changes, &traces, &mana_totals);
        state.resolution_changes = state
            .resolution_changes
            .saturating_add(changes.len() as u64);
        state.resolution_transitions = state.resolution_transitions.saturating_add(
            changes
                .iter()
                .filter(|change| change.before_level != change.after_level)
                .count() as u64,
        );
        state.advanced_through = self.next_time;
        self.next_time = self.next_time.tick();
        Ok(())
    }
}

impl System for ResolutionRuntimeSystem {
    fn run(&mut self, _stream: &mut RandomStream) {
        if let Err(error) = self.execute() {
            if let Ok(mut state) = self.state.lock() {
                state.failure.get_or_insert(error);
            }
        }
    }

    fn restore_time(&mut self, time: SimulationTime) {
        self.next_time = time;
    }
}

struct ActorPerceptionSystem {
    state: Arc<Mutex<RuntimeState>>,
    next_time: SimulationTime,
}

impl ActorPerceptionSystem {
    fn new(state: Arc<Mutex<RuntimeState>>) -> Self {
        Self {
            state,
            next_time: SimulationTime::new(1),
        }
    }

    fn execute(&mut self) -> Result<(), RuntimeError> {
        let mut state = self.state.lock().map_err(|_| RuntimeError::StatePoisoned)?;
        let objects = state.actor_objects.clone();
        let feature_count = actor_perception_step(self.next_time, &mut state.actors, &objects)?;
        state.perceived_actor_features = state
            .perceived_actor_features
            .saturating_add(feature_count as u64);
        self.next_time = self.next_time.tick();
        Ok(())
    }
}

impl System for ActorPerceptionSystem {
    fn run(&mut self, _stream: &mut RandomStream) {
        if let Err(error) = self.execute() {
            if let Ok(mut state) = self.state.lock() {
                state.failure.get_or_insert(error);
            }
        }
    }

    fn restore_time(&mut self, time: SimulationTime) {
        self.next_time = time;
    }
}

struct ActorCognitionSystem {
    state: Arc<Mutex<RuntimeState>>,
    next_time: SimulationTime,
}

impl ActorCognitionSystem {
    fn new(state: Arc<Mutex<RuntimeState>>) -> Self {
        Self {
            state,
            next_time: SimulationTime::new(1),
        }
    }

    fn execute(&mut self) -> Result<(), RuntimeError> {
        let mut state = self.state.lock().map_err(|_| RuntimeError::StatePoisoned)?;
        let object_count = actor_cognition_step(
            self.next_time,
            &mut state.actors,
            ActionKindId::new(ACTOR_ACTION_EVENT_KIND),
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
        if let Err(error) = self.execute() {
            if let Ok(mut state) = self.state.lock() {
                state.failure.get_or_insert(error);
            }
        }
    }

    fn restore_time(&mut self, time: SimulationTime) {
        self.next_time = time;
    }
}

struct ActorActionSystem {
    state: Arc<Mutex<RuntimeState>>,
    next_time: SimulationTime,
}

impl ActorActionSystem {
    fn new(state: Arc<Mutex<RuntimeState>>) -> Self {
        Self {
            state,
            next_time: SimulationTime::new(1),
        }
    }

    fn execute(&mut self) -> Result<(), RuntimeError> {
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
                        let event = CausalEventProposal::new(
                            EventProposalKey::new(
                                ACTOR_ACTION_SYSTEM_ID,
                                actor_id.raw(),
                                ordinal as u64,
                            ),
                            EventKindId::new(ACTOR_ACTION_EVENT_KIND),
                            vec![state.latest_physical_trace],
                            vec![CausalEffect::new(
                                CausalTarget::new(
                                    StateObjectKindId::new(ACTOR_OBJECT_KIND),
                                    actor_id.raw(),
                                    StatePropertyId::new(ACTOR_BODY_PROPERTY),
                                ),
                                actor_state_fingerprint(&actor),
                                actor_state_fingerprint(&next_actor),
                            )?],
                        )?;
                        let trace = state.traces.commit_batch(
                            self.next_time,
                            Phase::Action,
                            vec![event],
                        )?[0];
                        next_actor
                            .validation_results
                            .push(ActionValidationResult::Valid { trace });
                        state.actors.insert(actor_id, next_actor);
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
        if let Err(error) = self.execute() {
            if let Ok(mut state) = self.state.lock() {
                state.failure.get_or_insert(error);
            }
        }
    }

    fn restore_time(&mut self, time: SimulationTime) {
        self.next_time = time;
    }
}

struct PopulationLifecycleSystem {
    state: Arc<Mutex<RuntimeState>>,
    next_time: SimulationTime,
}

impl PopulationLifecycleSystem {
    fn new(state: Arc<Mutex<RuntimeState>>) -> Self {
        Self {
            state,
            next_time: SimulationTime::new(1),
        }
    }

    fn execute(&mut self) -> Result<(), RuntimeError> {
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
        if let Err(error) = self.execute() {
            if let Ok(mut state) = self.state.lock() {
                state.failure.get_or_insert(error);
            }
        }
    }

    fn restore_time(&mut self, time: SimulationTime) {
        self.next_time = time;
    }
}

#[allow(clippy::type_complexity)]
fn bootstrap_population_aggregates(
    traces: &mut CausalTraceStore,
    population: u64,
    chunks: &[ChartChunkCoord],
    root_trace: TraceId,
) -> Result<
    (
        BTreeMap<ChartChunkCoord, PopulationAggregate>,
        BTreeMap<ChartChunkCoord, Vec<ActorId>>,
        u64,
    ),
    RuntimeError,
> {
    let mut aggregates = BTreeMap::new();
    let pools = BTreeMap::new();
    if population == 0 {
        return Ok((aggregates, pools, 1));
    }
    let Some(chunk) = chunks.first().copied() else {
        return Ok((aggregates, pools, 1));
    };
    let event = CausalEventProposal::new(
        EventProposalKey::new(BOOTSTRAP_SYSTEM_ID, chart_chunk_hash(chunk), 0),
        EventKindId::new(POPULATION_BOOTSTRAP_EVENT_KIND),
        vec![root_trace],
        vec![CausalEffect::new(
            CausalTarget::new(
                StateObjectKindId::new(POPULATION_OBJECT_KIND),
                chart_chunk_hash(chunk),
                StatePropertyId::new(POPULATION_AGGREGATE_PROPERTY),
            ),
            fingerprint_u64(0x0801, 0),
            fingerprint_u64(0x0801, population),
        )?],
    )?;
    let trace = traces.commit_batch(SimulationTime::new(0), Phase::Lifecycle, vec![event])?[0];
    aggregates.insert(
        chunk,
        PopulationAggregate::new(chunk, population, vec![root_trace, trace])?,
    );
    Ok((aggregates, pools, 1))
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

fn lifecycle_births_and_deaths(
    state: &mut RuntimeState,
    time: SimulationTime,
) -> Result<(), RuntimeError> {
    if state.population_aggregates.is_empty() {
        return Ok(());
    }
    if time.raw() % 11 == 0 && total_population(state) < 16 {
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
    if time.raw() % 17 == 0 && total_population(state) > 1 {
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
    if time.raw() % 5 != 0 {
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
            (time.raw() % 7 == 0)
                .then(|| first_population_chunk(state).ok())
                .flatten()
        });
    if let Some(chunk) = promote_chunk {
        promote_actor_from_aggregate(state, time, chunk)?;
    }
    if time.raw() % 13 == 0 && state.actors.len() > 2 {
        if let Some(actor_id) = state.actors.keys().next_back().copied() {
            demote_actor_to_aggregate(state, time, actor_id)?;
        }
    }
    Ok(())
}

fn lifecycle_material_activity(
    state: &mut RuntimeState,
    time: SimulationTime,
) -> Result<(), RuntimeError> {
    if time.raw() % 3 != 0 {
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

fn promote_actor_from_aggregate(
    state: &mut RuntimeState,
    time: SimulationTime,
    chunk: ChartChunkCoord,
) -> Result<(), RuntimeError> {
    if state.actors.len() >= 16 {
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
        fixture_sensors(1),
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
                i64::from(chunk.chunk.x) + 2,
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

fn commit_material_event(
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

fn first_population_chunk(state: &RuntimeState) -> Result<ChartChunkCoord, RuntimeError> {
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

fn validate_trace_ancestry(ancestry: &[TraceId]) -> Result<(), RuntimeError> {
    if ancestry.windows(2).any(|window| window[0] >= window[1]) {
        return Err(RuntimeError::InvalidPopulationAggregate);
    }
    Ok(())
}

fn append_trace(ancestry: &[TraceId], trace: TraceId) -> Result<Vec<TraceId>, RuntimeError> {
    let mut next = ancestry.to_vec();
    next.push(trace);
    next.sort_unstable();
    next.dedup();
    validate_trace_ancestry(&next)?;
    Ok(next)
}

fn merge_trace_ancestry(
    left: &[TraceId],
    right: &[TraceId],
    trace: TraceId,
) -> Result<Vec<TraceId>, RuntimeError> {
    let mut next = Vec::with_capacity(left.len() + right.len() + 1);
    next.extend_from_slice(left);
    next.extend_from_slice(right);
    next.push(trace);
    next.sort_unstable();
    next.dedup();
    validate_trace_ancestry(&next)?;
    Ok(next)
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

fn mana_totals_by_chunk(mana: &ManaFieldSet) -> BTreeMap<ChartChunkCoord, i64> {
    mana.fields()
        .iter()
        .map(|(chunk, field)| (*chunk, field.intensity().iter().copied().sum()))
        .collect()
}

fn refresh_active_chunk_conserved_state(
    active_chunks: &mut BTreeMap<ChartChunkCoord, ActiveChunkState>,
    event_counts: &BTreeMap<ChartChunkCoord, u64>,
    mana_totals: &BTreeMap<ChartChunkCoord, i64>,
) {
    for (chunk, active) in active_chunks {
        active.total_mana = mana_totals.get(chunk).copied().unwrap_or(0);
        active.event_count = event_counts.get(chunk).copied().unwrap_or(0);
    }
}

fn pattern_event_counts_by_chunk(
    history: &PhysicalPatternHistory,
) -> BTreeMap<ChartChunkCoord, u64> {
    let mut event_counts = BTreeMap::<ChartChunkCoord, u64>::new();
    for sample in history.samples() {
        *event_counts.entry(sample.chunk).or_default() += 1;
    }
    event_counts
}

fn resolution_signals(
    state: &RuntimeState,
    mana_totals: &BTreeMap<ChartChunkCoord, i64>,
    time: SimulationTime,
) -> Result<Vec<CausalRelevanceSignal>, ResolutionError> {
    let mut signals = Vec::new();
    let Some(mana_trace) = state.latest_mana_trace else {
        return Ok(terrain_relevance_signals(state, time));
    };
    let mut ordinal = 0_u32;
    for (chunk, total) in mana_totals {
        if *total > 0 {
            signals.push(CausalRelevanceSignal::new(
                *chunk,
                *chunk,
                ResolutionChannelId::new(RESOLUTION_CHANNEL),
                (*total).clamp(1, 1_000),
                mana_trace,
                ordinal,
            )?);
            ordinal = ordinal.saturating_add(1);
        }
    }
    for (chunk, total) in mana_totals {
        for neighbor in [
            chunk.same_chart_neighbor(1, 0, 0),
            chunk.same_chart_neighbor(0, 1, 0),
            chunk.same_chart_neighbor(0, 0, 1),
        ] {
            if let Some(neighbor_total) = mana_totals.get(&neighbor) {
                let strength = (*total)
                    .saturating_add(*neighbor_total)
                    .saturating_div(2)
                    .clamp(1, 1_000);
                signals.push(CausalRelevanceSignal::new(
                    *chunk,
                    neighbor,
                    ResolutionChannelId::new(RESOLUTION_CHANNEL),
                    strength,
                    mana_trace,
                    ordinal,
                )?);
                ordinal = ordinal.saturating_add(1);
                signals.push(CausalRelevanceSignal::new(
                    neighbor,
                    *chunk,
                    ResolutionChannelId::new(RESOLUTION_CHANNEL),
                    strength,
                    mana_trace,
                    ordinal,
                )?);
                ordinal = ordinal.saturating_add(1);
            }
        }
    }
    signals.extend(
        terrain_relevance_signals(state, time)
            .into_iter()
            .map(|signal| {
                let next = CausalRelevanceSignal::new(
                    signal.source(),
                    signal.target(),
                    ResolutionChannelId::new(RESOLUTION_CHANNEL),
                    1_000,
                    state.latest_physical_trace,
                    ordinal,
                );
                ordinal = ordinal.saturating_add(1);
                next
            })
            .collect::<Result<Vec<_>, _>>()?,
    );
    Ok(signals)
}

fn terrain_relevance_signals(
    state: &RuntimeState,
    time: SimulationTime,
) -> Vec<CausalRelevanceSignal> {
    state
        .carrier_adapters
        .iter()
        .enumerate()
        .filter_map(|(index, (chunk, adapter))| {
            let strength = adapter
                .emit_samples(time, state.latest_physical_trace)
                .iter()
                .map(|sample| i64::from(sample.magnitude))
                .sum::<i64>()
                .saturating_div(1_024)
                .clamp(1, 1_000);
            CausalRelevanceSignal::new(
                *chunk,
                *chunk,
                ResolutionChannelId::new(RESOLUTION_CHANNEL),
                strength,
                state.latest_physical_trace,
                index as u32,
            )
            .ok()
        })
        .collect()
}

fn apply_resolution_transitions(
    active_chunks: &mut BTreeMap<ChartChunkCoord, ActiveChunkState>,
    changes: &[ontopolis_resolution::ResolutionChange],
    traces: &[TraceId],
    mana_totals: &BTreeMap<ChartChunkCoord, i64>,
) {
    for (change, trace) in changes.iter().zip(traces.iter().copied()) {
        if let Some(active) = active_chunks.get_mut(&change.chunk) {
            let conserved_mana = mana_totals.get(&change.chunk).copied().unwrap_or(0);
            active.relevance = change.after_relevance;
            active.level = change.after_level;
            active.total_mana = conserved_mana;
            active.last_transition = Some(trace);
        }
    }
}

fn write_chart_chunk(digest: &mut CanonicalDigest, chunk: ChartChunkCoord) {
    digest.write(chunk.chart.raw());
    digest.write(chunk.chunk.x as u64);
    digest.write(chunk.chunk.y as u64);
    digest.write(chunk.chunk.z as u64);
}

fn write_population_aggregate(digest: &mut CanonicalDigest, aggregate: &PopulationAggregate) {
    write_chart_chunk(digest, aggregate.chart);
    digest.write(aggregate.count);
    digest.write(aggregate.births);
    digest.write(aggregate.deaths);
    digest.write(aggregate.material_inflow as u64);
    digest.write(aggregate.material_outflow as u64);
    digest.write(aggregate.causal_ancestry.len() as u64);
    for trace in &aggregate.causal_ancestry {
        digest.write(trace.raw());
    }
}

fn chart_chunk_hash(chunk: ChartChunkCoord) -> u64 {
    mix64(
        chunk.chart.raw()
            ^ (chunk.chunk.x as u64).rotate_left(7)
            ^ (chunk.chunk.y as u64).rotate_left(19)
            ^ (chunk.chunk.z as u64).rotate_left(31),
    )
}

fn cell_object_id(chunk: ChartChunkCoord, cell_index: u16) -> u64 {
    chart_chunk_hash(chunk) ^ u64::from(cell_index)
}

fn fingerprint_u64(tag: u64, value: u64) -> StateFingerprint {
    fingerprint_words([tag, value, mix64(tag ^ value), tag.rotate_left(17) ^ value])
}

fn fingerprint_i64(tag: u64, value: i64) -> StateFingerprint {
    fingerprint_u64(tag, value as u64)
}

fn fingerprint_pair(tag: u64, first: i64, second: i64) -> StateFingerprint {
    fingerprint_words([
        tag,
        first as u64,
        second as u64,
        mix64(tag ^ first as u64 ^ (second as u64).rotate_left(23)),
    ])
}

fn fingerprint_population_aggregate(aggregate: &PopulationAggregate) -> StateFingerprint {
    fingerprint_words([
        0x0803,
        chart_chunk_hash(aggregate.chart),
        aggregate.count ^ aggregate.births.rotate_left(11) ^ aggregate.deaths.rotate_left(23),
        (aggregate.material_inflow as u64) ^ (aggregate.material_outflow as u64).rotate_left(31),
    ])
}

fn fingerprint_material_flow(aggregate: &PopulationAggregate) -> StateFingerprint {
    fingerprint_pair(
        0x0C01,
        aggregate.material_inflow,
        aggregate.material_outflow,
    )
}

fn fingerprint_words(words: [u64; 4]) -> StateFingerprint {
    let mut bytes = [0_u8; 32];
    for (index, word) in words.into_iter().enumerate() {
        bytes[index * 8..(index + 1) * 8].copy_from_slice(&word.to_le_bytes());
    }
    StateFingerprint::new(bytes)
}

fn mix64(mut value: u64) -> u64 {
    value ^= value >> 30;
    value = value.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value ^= value >> 27;
    value = value.wrapping_mul(0x94D0_49BB_1331_11EB);
    value ^ (value >> 31)
}

struct CanonicalDigest([u64; 4]);

impl CanonicalDigest {
    fn new() -> Self {
        Self([
            0x243F_6A88_85A3_08D3,
            0x1319_8A2E_0370_7344,
            0xA409_3822_299F_31D0,
            0x082E_FA98_EC4E_6C89,
        ])
    }

    fn write(&mut self, value: u64) {
        for (index, lane) in self.0.iter_mut().enumerate() {
            *lane = mix64(
                lane.wrapping_add(value.rotate_left((index as u32 * 13) + 1))
                    .wrapping_add(index as u64),
            );
        }
    }

    fn write_bytes(&mut self, bytes: [u8; 32]) {
        for chunk in bytes.chunks_exact(8) {
            self.write(u64::from_le_bytes(chunk.try_into().expect("exact chunk")));
        }
    }

    fn finish(self) -> StateFingerprint {
        fingerprint_words(self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ontopolis_domains::PhysicalCarrierAdapter;
    use ontopolis_types::{LocalCoord, PhysicalPatternId, WorldCoord};

    fn test_chunk() -> ChartChunkCoord {
        ChartChunkCoord::new(SpatialChartId::new(1), ChunkCoord::new(0, 0, 0))
    }

    #[test]
    fn runtime_executes_a_long_causal_run_without_errors() {
        let mut runtime = Runtime::from_seed(42).unwrap();
        let snapshot = runtime.run_ticks(512).unwrap();
        assert_eq!(snapshot.time, SimulationTime::new(512));
        assert!(snapshot.mana_total > 0);
        assert!(snapshot.causal_trace_count > 512);
        assert!(snapshot.resolution_level > 0);
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
        let mut first = Runtime::from_seed(8).unwrap();
        let mut second = Runtime::from_seed(8).unwrap();

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
                    CausalRelevanceSignal::new(
                        nearby,
                        nearby,
                        ResolutionChannelId::new(RESOLUTION_CHANNEL),
                        400,
                        TraceId::new(3),
                        0,
                    )
                    .unwrap(),
                    CausalRelevanceSignal::new(
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
    fn physical_suppression_changes_the_causal_trajectory() {
        let control_config = RuntimeConfig::new(9);
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
        config
    }

    fn config_with_actor(seed: u64) -> RuntimeConfig {
        let mut config = RuntimeConfig::new(seed);
        config.actor_count = 1;
        config.sensor_count = 1;
        config.action_bounds = 8;
        config
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

        let snapshot = runtime.run_ticks(1).unwrap();
        let state = runtime.lock_state().unwrap();
        let actor = state.actors.get(&ActorId::new(1)).unwrap();

        assert_eq!(snapshot.actor_count, 1);
        assert!(snapshot.perceived_actor_features > 0);
        assert!(snapshot.subjective_actor_objects > 0);
        assert_eq!(snapshot.actor_actions_committed, 1);
        assert_eq!(actor.body.position, WorldCoord::new(1, 0, 0));
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
    fn mistaken_subjective_identity_can_still_drive_valid_physical_action() {
        let mut runtime = Runtime::new(config_with_actor(46)).unwrap();
        insert_actor_object(&runtime, 100, 2);
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
        assert_eq!(actor.body.position, WorldCoord::new(2, 0, 0));
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
    fn mana_effect_system_commits_physical_trace_above_threshold() {
        let mut runtime = Runtime::new(config_with_effect_threshold(31, 1, 0)).unwrap();
        let snapshot = runtime.run_ticks(8).unwrap();
        let state = runtime.lock_state().unwrap();

        assert!(snapshot.mana_total > 1);
        assert!(snapshot.mana_physical_effects > 0);
        assert!(state.traces.iter().any(|event| {
            event.kind == EventKindId::new(MANA_PHYSICAL_EFFECT_EVENT_KIND)
                && event.phase == Phase::Mana
                && event.effects.iter().any(|effect| {
                    effect.target()
                        == CausalTarget::new(
                            StateObjectKindId::new(PHYSICAL_OBJECT_KIND),
                            0,
                            StatePropertyId::new(MANA_PHYSICAL_EFFECT_PROPERTY),
                        )
                })
        }));
    }

    #[test]
    fn committed_mana_effect_changes_later_physical_samples() {
        let mut enabled = Runtime::new(config_with_effect_threshold(32, 1, 0)).unwrap();
        let mut disabled = Runtime::new(config_with_effect_threshold(32, 0, 0)).unwrap();
        enabled.run_ticks(32).unwrap();
        disabled.run_ticks(32).unwrap();
        let enabled_state = enabled.lock_state().unwrap();
        let disabled_state = disabled.lock_state().unwrap();
        let enabled_magnitude = enabled_state
            .pattern_history
            .samples()
            .last()
            .unwrap()
            .magnitude;
        let disabled_magnitude = disabled_state
            .pattern_history
            .samples()
            .last()
            .unwrap()
            .magnitude;

        assert!(enabled_state.mana_physical_effects > 0);
        assert!(enabled_state.physical_counter > disabled_state.physical_counter);
        assert!(enabled_magnitude > disabled_magnitude);
    }

    #[test]
    fn below_threshold_mana_produces_no_physical_effects() {
        let mut runtime =
            Runtime::new(config_with_effect_threshold(33, 1_000_000_000, 100)).unwrap();
        let snapshot = runtime.run_ticks(32).unwrap();

        assert!(snapshot.mana_total < 1_000_000_000);
        assert_eq!(snapshot.mana_physical_effects, 0);
        assert_eq!(snapshot.mana_physical_effect_boost, 0);
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
    fn mana_feedback_is_bounded_by_hysteresis_and_boost_cap() {
        let config = config_with_effect_threshold(36, 2, 1);
        let base_magnitude = config.pattern_schedule.magnitude;
        let mut runtime = Runtime::new(config).unwrap();
        let snapshot = runtime.run_ticks(128).unwrap();
        let state = runtime.lock_state().unwrap();
        let latest_magnitude = state.pattern_history.samples().last().unwrap().magnitude;
        let terrain_magnitude = state
            .carrier_adapters
            .values()
            .flat_map(|adapter| {
                adapter.emit_samples(SimulationTime::new(129), state.latest_physical_trace)
            })
            .map(|sample| sample.magnitude)
            .max()
            .unwrap();

        assert!(snapshot.mana_physical_effects > 0);
        assert!(snapshot.mana_physical_effect_boost <= MAX_MANA_PHYSICAL_EFFECT_BOOST);
        assert!(state.physical_counter <= 128 * (1 + u64::from(MAX_MANA_PHYSICAL_EFFECT_BOOST)));
        assert!(
            latest_magnitude
                <= base_magnitude
                    .saturating_add(
                        MAX_MANA_PHYSICAL_EFFECT_BOOST.saturating_mul(MANA_EFFECT_MAGNITUDE_STEP),
                    )
                    .saturating_add(terrain_magnitude)
        );
    }
}
