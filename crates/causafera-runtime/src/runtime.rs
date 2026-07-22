use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{Arc, Mutex, MutexGuard},
};

use causafera_core::{
    CausalCommitError, CausalEffect, CausalEffectError, CausalEventProposal,
    CausalEventProposalError, CausalTarget, CausalTraceSnapshot, CausalTraceStore,
    DeterministicConfig, EventProposalKey, Phase, RandomStream, Scheduler, StateFingerprint,
    System,
};
use causafera_domains::{
    ManaError, ManaField, ManaFieldSet, ManaFieldSetSnapshot, ManaParameters,
    PhysicalCarrierAdapter, PhysicalPatternSample,
};
use causafera_explanation::{
    ComparisonContext, ExplanationClaim, ExplanationFrame, ExplanationReport,
    MATERIAL_SURFACE_LOOP_CLAIM_SCHEMA, MaterialSurfaceLocalManaTransitionClaim,
    MaterialSurfaceLoopClaim, NumericClaimValue,
};
use causafera_observer_api::{
    MATERIAL_SURFACE_DELTA_SCHEMA_V3, MAX_MATERIAL_SURFACE_DELTAS, MaterialSurfaceDelta,
    MaterialSurfaceGateDelta, ObserverChunkSummary, ObserverSnapshot, ObserverWorldSnapshot,
};
use causafera_perception::{PhysicalSignal, SignalMagnitude};
use causafera_resolution::{
    CausalRelevanceSignal, ChannelWeight, ResolutionError, ResolutionField,
    ResolutionFieldSnapshot, ResolutionPolicy, ResolutionPolicySnapshot,
};
use causafera_types::{
    ChartChunkCoord, ChunkCoord, EntityId, EventKindId, ExperimentId, HistoricalStageId,
    LocalCoord, ManaFieldId, ResolutionChannelId, ResolutionFieldId, SignalChannelId,
    SimulationTime, SpatialChartId, StateObjectKindId, StatePropertyId, TraceId, WorldCoord,
};
use thiserror::Error;

use crate::{
    ACTOR_SIGNAL_CHANNEL, ActionKindId, ActionRejection, ActionValidationResult, ActorId,
    ActorObjectiveSnapshot, ActorPhysicalObject, ActorState, ActorSubjectiveSnapshot,
    MAX_MATERIAL_SURFACE_TRANSITIONS, MaterialSurface, MaterialSurfaceCarrierAdapter,
    MaterialSurfaceGateTransition, MaterialSurfaceId, MaterialSurfaceManaGate,
    MaterialSurfaceRecordSnapshot, MaterialSurfaceSnapshot, MaterialSurfaceTransition,
    MinimalBodyState, PatternHistorySnapshot, PhysicalPatternHistory, SensorAperture, SensorKindId,
    TerrainCarrierAdapter, TerrainCarrierSnapshot, actor_cognition_step, actor_perception_step,
    actor_state_fingerprint, apply_action, deterministic_terrain_chunk, validate_action,
};

pub const MAX_RUNTIME_TICKS: u64 = 1_000_000;
pub const MAX_PATTERN_HISTORY_ENTRIES: usize = 512;
pub const MAX_PATTERN_HISTORY_PER_PATTERN: usize = 128;
pub const MANA_PATTERN_HISTORY_TICKS: u64 = 8;
pub const MAX_EXPERIMENT_RECIPE_MANA_SOURCES: usize = 16;
pub const EXPERIMENT_RECIPE_MANA_SOURCE_POLICY_SCHEMA_V1: u64 = 1;

pub const CURRENT_DIGEST_SCHEMA_VERSION: DigestSchemaVersion = DigestSchemaVersion::new(4);

const PHYSICAL_SYSTEM_ID: u64 = 10;
pub const EXPERIMENT_RECIPE_MANA_SOURCE_SYSTEM_ID: u64 = 19;
const MANA_SYSTEM_ID: u64 = 20;
const MANA_EFFECTS_SYSTEM_ID: u64 = 21;
const RESOLUTION_SYSTEM_ID: u64 = 30;
const ACTOR_ACTION_SYSTEM_ID: u64 = 42;
const LIFECYCLE_SYSTEM_ID: u64 = 60;
const BOOTSTRAP_SYSTEM_ID: u64 = 61;
const ROOT_EVENT_KIND: u64 = 1;
const MANA_EVENT_KIND: u64 = 3;
const RESOLUTION_EVENT_KIND: u64 = 4;
const ACTOR_CONTACT_ACTION_KIND: u64 = 6;
const ACTOR_REJECTION_EVENT_KIND: u64 = 7;
const POPULATION_BOOTSTRAP_EVENT_KIND: u64 = 8;
const POPULATION_LIFECYCLE_EVENT_KIND: u64 = 9;
const ACTOR_PROMOTION_EVENT_KIND: u64 = 10;
const ACTOR_DEMOTION_EVENT_KIND: u64 = 11;
const MATERIAL_ACTIVITY_EVENT_KIND: u64 = 12;
const MATERIAL_SURFACE_BOOTSTRAP_EVENT_KIND: u64 = 13;
const MATERIAL_SURFACE_CONTACT_EVENT_KIND: u64 = 14;
const MATERIAL_SURFACE_MANA_EVENT_KIND: u64 = 15;
pub const EXPERIMENT_RECIPE_MANA_SOURCE_EVENT_KIND: u64 = 17;
const RUNTIME_OBJECT_KIND: u64 = 1;
const PHYSICAL_OBJECT_KIND: u64 = 2;
const MANA_OBJECT_KIND: u64 = 3;
const RESOLUTION_OBJECT_KIND: u64 = 4;
const ACTOR_OBJECT_KIND: u64 = 5;
const POPULATION_OBJECT_KIND: u64 = 6;
const MATERIAL_OBJECT_KIND: u64 = 7;
const MATERIAL_SURFACE_OBJECT_KIND: u64 = 8;
pub const EXPERIMENT_RECIPE_MANA_SOURCE_OBJECT_KIND: u64 = 9;
const ROOT_PROPERTY: u64 = 1;
const PHYSICAL_PROPERTY: u64 = 2;
const MANA_PROPERTY: u64 = 3;
const RESOLUTION_PROPERTY: u64 = 4;
const ACTOR_BODY_PROPERTY: u64 = 6;
const ACTOR_REJECTION_PROPERTY: u64 = 7;
const POPULATION_AGGREGATE_PROPERTY: u64 = 8;
const ACTOR_PROMOTION_PROPERTY: u64 = 9;
const MATERIAL_FLOW_PROPERTY: u64 = 10;
const MATERIAL_SURFACE_CONDITION_PROPERTY: u64 = 11;
const MATERIAL_SURFACE_MANA_GATE_PROPERTY: u64 = 12;
pub const EXPERIMENT_RECIPE_MANA_SOURCE_PROPERTY: u64 = 13;
const RESOLUTION_CHANNEL: u64 = 1;
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
pub struct ExperimentRecipeManaSource {
    pub source_record_id: u64,
    pub enabled: bool,
    pub scheduled_tick: u64,
    pub target_chunk: ChartChunkCoord,
    pub cell_index: u16,
    pub amount: i64,
    pub per_record_maximum: i64,
    pub policy_schema_id: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExperimentRecipeManaSourceRecipe {
    pub records: Vec<ExperimentRecipeManaSource>,
    pub recipe_budget: i64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExperimentRecipeManaSourceReceipt {
    pub source_record_id: u64,
    pub scheduled_tick: u64,
    pub executed_tick: u64,
    pub source_trace: TraceId,
    pub before_intensity: i64,
    pub after_intensity: i64,
    pub recipe_hash: StateFingerprint,
    pub policy_schema_id: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExperimentRecipeManaSourceReceiptSnapshot {
    pub source_record_id: u64,
    pub scheduled_tick: u64,
    pub executed_tick: u64,
    pub source_trace: TraceId,
    pub before_intensity: i64,
    pub after_intensity: i64,
    pub recipe_hash: StateFingerprint,
    pub policy_schema_id: u64,
}

impl ExperimentRecipeManaSourceRecipe {
    pub fn recipe_hash(&self) -> StateFingerprint {
        let mut digest = CanonicalDigest::new();
        digest.write(u64::try_from(self.records.len()).unwrap_or(u64::MAX));
        let mut records = self.records.clone();
        records.sort_unstable_by_key(|record| (record.scheduled_tick, record.source_record_id));
        for record in records {
            digest.write(record.source_record_id);
            digest.write(u64::from(record.enabled));
            digest.write(record.scheduled_tick);
            digest.write(record.target_chunk.chart.raw());
            digest.write(u64::from_le_bytes(
                i64::from(record.target_chunk.chunk.x).to_le_bytes(),
            ));
            digest.write(u64::from_le_bytes(
                i64::from(record.target_chunk.chunk.y).to_le_bytes(),
            ));
            digest.write(u64::from_le_bytes(
                i64::from(record.target_chunk.chunk.z).to_le_bytes(),
            ));
            digest.write(u64::from(record.cell_index));
            digest.write(u64::from_le_bytes(record.amount.to_le_bytes()));
            digest.write(u64::from_le_bytes(record.per_record_maximum.to_le_bytes()));
            digest.write(record.policy_schema_id);
        }
        digest.write(u64::from_le_bytes(self.recipe_budget.to_le_bytes()));
        digest.finish()
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
    pub material_surface_signals_enabled: bool,
    pub experiment_recipe_mana_sources: ExperimentRecipeManaSourceRecipe,
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
                effect_threshold: 4_096,
                effect_hysteresis: 2_000,
            },
            carrier_adapter: CarrierAdapterConfig::terrain_seed(world_seed),
            actor_count: 0,
            sensor_count: 0,
            action_bounds: 8,
            bootstrap_population: 0,
            material_surface_signals_enabled: true,
            experiment_recipe_mana_sources: ExperimentRecipeManaSourceRecipe {
                records: Vec::new(),
                recipe_budget: 0,
            },
        }
    }

    fn validate(mut self) -> Result<Self, RuntimeError> {
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
        if self.experiment_recipe_mana_sources.records.len() > MAX_EXPERIMENT_RECIPE_MANA_SOURCES {
            return Err(RuntimeError::ExperimentRecipeSourceCountExceeded {
                count: self.experiment_recipe_mana_sources.records.len(),
            });
        }
        if self.experiment_recipe_mana_sources.recipe_budget < 0 {
            return Err(RuntimeError::InvalidExperimentRecipeBudget {
                budget: self.experiment_recipe_mana_sources.recipe_budget,
            });
        }
        let active_chunks = active_chunk_keys(self.chart_id, self.active_chunk_radius);
        let active_chunks = active_chunks.into_iter().collect::<BTreeSet<_>>();
        let mut source_ids = BTreeSet::new();
        let mut canonical_keys = BTreeSet::new();
        let mut enabled_amount = 0_i128;
        let cells_per_extent = u32::from(self.chunk_extent).pow(3);
        for record in &self.experiment_recipe_mana_sources.records {
            if record.source_record_id == 0 {
                return Err(RuntimeError::InvalidExperimentRecipeSourceId {
                    source_record_id: record.source_record_id,
                });
            }
            if !source_ids.insert(record.source_record_id) {
                return Err(RuntimeError::DuplicateExperimentRecipeSourceId {
                    source_record_id: record.source_record_id,
                });
            }
            if !(1..=MAX_RUNTIME_TICKS).contains(&record.scheduled_tick) {
                return Err(RuntimeError::InvalidExperimentRecipeScheduledTick {
                    scheduled_tick: record.scheduled_tick,
                });
            }
            if !canonical_keys.insert((
                record.scheduled_tick,
                record.target_chunk,
                record.cell_index,
            )) {
                return Err(RuntimeError::DuplicateExperimentRecipeCanonicalKey {
                    scheduled_tick: record.scheduled_tick,
                    target_chunk: record.target_chunk,
                    cell_index: record.cell_index,
                });
            }
            if record.amount < 0 {
                return Err(RuntimeError::InvalidExperimentRecipeAmount {
                    amount: record.amount,
                });
            }
            if record.per_record_maximum < 0 {
                return Err(RuntimeError::InvalidExperimentRecipeMaximum {
                    maximum: record.per_record_maximum,
                });
            }
            if record.amount > record.per_record_maximum {
                return Err(RuntimeError::ExperimentRecipeAmountExceedsMaximum {
                    amount: record.amount,
                    maximum: record.per_record_maximum,
                });
            }
            if record.policy_schema_id != EXPERIMENT_RECIPE_MANA_SOURCE_POLICY_SCHEMA_V1 {
                return Err(RuntimeError::InvalidExperimentRecipePolicySchema {
                    policy_schema_id: record.policy_schema_id,
                });
            }
            if record.target_chunk.chart != self.chart_id {
                return Err(RuntimeError::InvalidExperimentRecipeTargetChart {
                    source_record_id: record.source_record_id,
                    chart: record.target_chunk.chart.raw(),
                });
            }
            if !active_chunks.contains(&record.target_chunk) {
                return Err(RuntimeError::InactiveExperimentRecipeTargetChunk {
                    source_record_id: record.source_record_id,
                    target_chunk: record.target_chunk,
                });
            }
            if u32::from(record.cell_index) >= cells_per_extent {
                return Err(RuntimeError::InvalidExperimentRecipeCellIndex {
                    source_record_id: record.source_record_id,
                    cell_index: record.cell_index,
                    cell_count: cells_per_extent,
                });
            }
            if record.enabled && record.amount != 0 {
                enabled_amount += i128::from(record.amount);
            }
        }
        if enabled_amount > i128::from(self.experiment_recipe_mana_sources.recipe_budget) {
            return Err(RuntimeError::ExperimentRecipeBudgetExceeded {
                enabled_amount,
                recipe_budget: self.experiment_recipe_mana_sources.recipe_budget,
            });
        }
        self.experiment_recipe_mana_sources
            .records
            .sort_unstable_by_key(|record| (record.scheduled_tick, record.source_record_id));
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
    pub material_surfaces: MaterialSurfaceSnapshot,
    pub actors_objective: ActorObjectiveStateSnapshot,
    pub actors_subjective: ActorSubjectiveStateSnapshot,
    pub population: PopulationAggregateSnapshot,
    pub bootstrap: BootstrapReceiptSnapshot,
    pub traces: CausalTraceSnapshot,
    pub experiment_recipe_mana_source_receipts: Vec<ExperimentRecipeManaSourceReceiptSnapshot>,
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
    fn for_runtime_config(config: &RuntimeConfig) -> Self {
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
    material_surfaces: BTreeMap<MaterialSurfaceId, MaterialSurface>,
    pending_material_surface_changes: BTreeSet<MaterialSurfaceId>,
    material_surface_transitions: Vec<MaterialSurfaceTransition>,
    material_surface_gate_transitions: Vec<MaterialSurfaceGateTransition>,
    latest_physical_trace: TraceId,
    latest_mana_trace: Option<TraceId>,
    pub executed_experiment_recipe_mana_sources: Vec<ExperimentRecipeManaSourceReceipt>,
    advanced_through: SimulationTime,
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
        let mut state = Self {
            config: config.clone(),
            traces,
            mana,
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
            .bootstrap(&mut state)
            .map_err(|error| match error {
                BootstrapError::Runtime(error) => error,
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
            if surface.gate.last_transition.is_some_and(|trace| {
                !self
                    .material_surface_gate_transitions
                    .iter()
                    .any(|transition| transition.transition_trace == trace)
            }) {
                return Err(RuntimeError::InvalidSnapshot(
                    "material surface gate state is missing its transition record",
                ));
            }
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
        }
    }

    fn observer_world_snapshot(&self, time: SimulationTime) -> ObserverWorldSnapshot {
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
        }
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
    for (id, surface) in &surfaces {
        let latest = latest_gate_trace_by_surface.get(id).copied();
        if surface.gate.last_transition != latest {
            return Err(RuntimeError::InvalidSnapshot(
                "material surface gate state is not the latest retained gate transition",
            ));
        }
    }
    Ok((
        surfaces,
        pending_changes,
        snapshot.transitions,
        snapshot.gate_transitions,
    ))
}

fn validate_material_surface_object_ids(
    ids: impl Iterator<Item = MaterialSurfaceId>,
) -> Result<(), RuntimeError> {
    let mut object_ids = BTreeSet::new();
    for id in ids {
        if !object_ids.insert(material_surface_object_id(id)) {
            return Err(RuntimeError::InvalidSnapshot(
                "material surface object ID collision",
            ));
        }
    }
    Ok(())
}

fn validate_mana_cell_object_ids(fields: &ManaFieldSet) -> Result<(), RuntimeError> {
    let mut object_ids = BTreeSet::new();
    for (chunk, field) in fields.fields() {
        for index in 0..field.intensity().len() {
            let cell_index = u16::try_from(index)
                .map_err(|_| RuntimeError::InvalidSnapshot("mana field cell index exceeds u16"))?;
            if !object_ids.insert(cell_object_id(*chunk, cell_index)) {
                return Err(RuntimeError::InvalidSnapshot(
                    "mana cell object ID collision",
                ));
            }
        }
    }
    Ok(())
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

struct ExperimentRecipeManaSourceSystem {
    state: Arc<Mutex<RuntimeState>>,
    next_time: SimulationTime,
}

impl ExperimentRecipeManaSourceSystem {
    fn new(state: Arc<Mutex<RuntimeState>>) -> Self {
        Self {
            state,
            next_time: SimulationTime::new(1),
        }
    }

    fn execute(&mut self) -> Result<(), RuntimeError> {
        let mut state = self.state.lock().map_err(|_| RuntimeError::StatePoisoned)?;
        let recipe_hash = state.config.experiment_recipe_mana_sources.recipe_hash();
        let due_records = state
            .config
            .experiment_recipe_mana_sources
            .records
            .iter()
            .filter(|record| {
                record.enabled
                    && record.amount > 0
                    && record.scheduled_tick == self.next_time.raw()
                    && !state
                        .executed_experiment_recipe_mana_sources
                        .iter()
                        .any(|receipt| receipt.source_record_id == record.source_record_id)
            })
            .cloned()
            .collect::<Vec<_>>();
        let mut pending = due_records
            .into_iter()
            .map(|record| {
                let proposal = state.mana.propose_experiment_recipe_mana_source(
                    record.target_chunk,
                    record.cell_index,
                    record.amount,
                )?;
                Ok::<_, RuntimeError>((record, proposal))
            })
            .collect::<Result<Vec<_>, _>>()?;
        pending.sort_unstable_by_key(|(_, proposal)| {
            EventProposalKey::new(
                EXPERIMENT_RECIPE_MANA_SOURCE_SYSTEM_ID,
                cell_object_id(proposal.chunk, proposal.cell_index),
                0,
            )
        });
        if pending.is_empty() {
            self.next_time = self.next_time.tick();
            return Ok(());
        }
        if state
            .executed_experiment_recipe_mana_sources
            .len()
            .saturating_add(pending.len())
            > MAX_EXPERIMENT_RECIPE_MANA_SOURCES
        {
            return Err(RuntimeError::ExperimentRecipeSourceCountExceeded {
                count: state
                    .executed_experiment_recipe_mana_sources
                    .len()
                    .saturating_add(pending.len()),
            });
        }

        let next_trace_id = state.traces.export_snapshot().next_trace_id;
        let events = pending
            .iter()
            .enumerate()
            .map(|(index, (record, proposal))| {
                let trace_offset = u64::try_from(index).map_err(|_| {
                    RuntimeError::CausalCommit(CausalCommitError::IdentifierExhausted)
                })?;
                let source_trace = TraceId::new(next_trace_id.checked_add(trace_offset).ok_or(
                    RuntimeError::CausalCommit(CausalCommitError::IdentifierExhausted),
                )?);
                let cell_effect = CausalEffect::new(
                    CausalTarget::new(
                        StateObjectKindId::new(EXPERIMENT_RECIPE_MANA_SOURCE_OBJECT_KIND),
                        cell_object_id(proposal.chunk, proposal.cell_index),
                        StatePropertyId::new(EXPERIMENT_RECIPE_MANA_SOURCE_PROPERTY),
                    ),
                    fingerprint_i64(0x0302, proposal.before),
                    fingerprint_i64(0x0302, proposal.after),
                )?;
                let receipt_effect = CausalEffect::new(
                    CausalTarget::new(
                        StateObjectKindId::new(EXPERIMENT_RECIPE_MANA_SOURCE_OBJECT_KIND),
                        record.source_record_id,
                        StatePropertyId::new(EXPERIMENT_RECIPE_MANA_SOURCE_PROPERTY),
                    ),
                    fingerprint_u64(0x0303, 0),
                    experiment_recipe_mana_source_receipt_fingerprint(
                        record,
                        self.next_time.raw(),
                        source_trace,
                        proposal.before,
                        proposal.after,
                        recipe_hash,
                    ),
                )?;
                let mut effects = vec![cell_effect, receipt_effect];
                effects.sort_unstable_by_key(|effect| effect.target());
                CausalEventProposal::new(
                    EventProposalKey::new(
                        EXPERIMENT_RECIPE_MANA_SOURCE_SYSTEM_ID,
                        cell_object_id(proposal.chunk, proposal.cell_index),
                        0,
                    ),
                    EventKindId::new(EXPERIMENT_RECIPE_MANA_SOURCE_EVENT_KIND),
                    Vec::new(),
                    effects,
                )
                .map_err(RuntimeError::from)
            })
            .collect::<Result<Vec<_>, _>>()?;
        let traces = state
            .traces
            .commit_batch(self.next_time, Phase::Mana, events)?;
        for ((record, proposal), trace) in pending.iter().zip(traces.iter().copied()) {
            state.mana = state
                .mana
                .clone()
                .commit_experiment_recipe_mana_source(*proposal, trace)?;
            state.latest_mana_trace = Some(trace);
            state
                .executed_experiment_recipe_mana_sources
                .push(ExperimentRecipeManaSourceReceipt {
                    source_record_id: record.source_record_id,
                    scheduled_tick: record.scheduled_tick,
                    executed_tick: self.next_time.raw(),
                    source_trace: trace,
                    before_intensity: proposal.before,
                    after_intensity: proposal.after,
                    recipe_hash,
                    policy_schema_id: record.policy_schema_id,
                });
        }
        state
            .executed_experiment_recipe_mana_sources
            .sort_unstable_by_key(|receipt| (receipt.executed_tick, receipt.source_record_id));
        self.next_time = self.next_time.tick();
        Ok(())
    }
}

impl System for ExperimentRecipeManaSourceSystem {
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
            let changed = std::mem::take(&mut state.pending_material_surface_changes);
            let adapter = MaterialSurfaceCarrierAdapter::new(state.config.chunk_extent);
            let emitted = changed
                .into_iter()
                .filter_map(|id| {
                    state
                        .material_surfaces
                        .get(&id)
                        .copied()
                        .map(|surface| (id, surface))
                })
                .flat_map(|(id, surface)| adapter.emit_samples(id, surface, self.next_time))
                .map(|sample| PhysicalPatternSample {
                    magnitude: sample.magnitude.saturating_add(self.schedule.magnitude),
                    ..sample
                })
                .collect::<Vec<_>>();
            state.physical_events = state
                .physical_events
                .saturating_add(u64::try_from(emitted.len()).unwrap_or(u64::MAX));
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
        let mut ordered_changes = changes.clone();
        ordered_changes.sort_unstable_by_key(|(chunk, change)| {
            EventProposalKey::new(MANA_SYSTEM_ID, cell_object_id(*chunk, change.cell_index), 0)
        });
        let events = ordered_changes
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
        let traces_by_change = ordered_changes
            .iter()
            .zip(traces.iter().copied())
            .map(|((chunk, change), trace)| ((*chunk, change.cell_index), trace))
            .collect::<BTreeMap<_, _>>();
        let mut traces_by_chunk = BTreeMap::<ChartChunkCoord, Vec<TraceId>>::new();
        for (chunk, field_proposal) in proposal.field_proposals() {
            let field_traces = field_proposal
                .changes()
                .iter()
                .map(|change| {
                    traces_by_change
                        .get(&(*chunk, change.cell_index))
                        .copied()
                        .ok_or(RuntimeError::InvalidSnapshot(
                            "mana proposal is missing its committed trace",
                        ))
                })
                .collect::<Result<Vec<_>, _>>()?;
            traces_by_chunk.insert(*chunk, field_traces);
        }
        state.mana = proposal.commit(&traces_by_chunk)?;
        state.pending_samples.clear();
        state.last_mana_changes = changed_count;
        state.mana_cell_changes = state
            .mana_cell_changes
            .saturating_add(u64::from(changed_count));
        let source_descendant = state.latest_mana_trace.and_then(|source_trace| {
            traces
                .iter()
                .copied()
                .filter(|trace| trace_descends_from(&state.traces, *trace, source_trace))
                .min()
        });
        if let Some(trace) = source_descendant.or_else(|| traces.last().copied()) {
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

#[derive(Clone, Debug, PartialEq, Eq)]
struct LocalManaMaterialSurfaceProposal {
    key: EventProposalKey,
    surface: MaterialSurfaceId,
    before: MaterialSurface,
    after_active: bool,
    after_condition: Option<i64>,
    local_mana_before: i64,
    local_mana_after: i64,
    local_mana_trace: TraceId,
    contact_trace: Option<TraceId>,
    causes: Vec<TraceId>,
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
        if self.parameters.effect_threshold <= 0 {
            self.next_time = self.next_time.tick();
            return Ok(());
        }
        let mut proposals = Vec::new();
        for (ordinal, (surface_id, surface)) in state
            .material_surfaces
            .iter()
            .filter(|(_, surface)| surface.contact_count > 0)
            .enumerate()
        {
            let field = state
                .mana
                .field(surface_id.chunk)
                .ok_or(RuntimeError::InvalidSnapshot(
                    "contacted material surface has no matching mana field",
                ))?;
            let index = usize::from(surface_id.cell_index);
            let local_mana_after =
                *field
                    .intensity()
                    .get(index)
                    .ok_or(RuntimeError::InvalidSnapshot(
                        "material surface cell is outside matching mana field",
                    ))?;
            let transition = if surface.gate.active {
                (local_mana_after
                    < self
                        .parameters
                        .effect_threshold
                        .saturating_sub(self.parameters.effect_hysteresis))
                .then_some((false, None))
            } else {
                (local_mana_after > self.parameters.effect_threshold)
                    .then_some((true, Some(surface.condition.saturating_add(1))))
            };
            let Some((after_active, after_condition)) = transition else {
                continue;
            };
            let local_mana_trace = field.last_change().get(index).copied().flatten().ok_or(
                RuntimeError::UnknownManaPhysicalEffectCause {
                    cause: surface.last_transition,
                },
            )?;
            let contact_trace = surface.last_contact_trace;
            let mut causes = vec![local_mana_trace];
            if after_active {
                let trace = contact_trace.ok_or(RuntimeError::InvalidSnapshot(
                    "contacted material surface is missing its contact trace",
                ))?;
                causes.push(trace);
                if surface.last_transition != trace {
                    causes.push(surface.last_transition);
                }
            }
            if let Some(trace) = surface.gate.last_transition {
                causes.push(trace);
            }
            causes.sort_unstable();
            causes.dedup();
            proposals.push(LocalManaMaterialSurfaceProposal {
                key: EventProposalKey::new(
                    MANA_EFFECTS_SYSTEM_ID,
                    material_surface_object_id(*surface_id),
                    u64::try_from(ordinal).map_err(|_| {
                        RuntimeError::CausalCommit(CausalCommitError::IdentifierExhausted)
                    })?,
                ),
                surface: *surface_id,
                before: *surface,
                after_active,
                after_condition,
                local_mana_before: *field.last_change_before().get(index).ok_or(
                    RuntimeError::InvalidSnapshot("mana prior value is outside matching field"),
                )?,
                local_mana_after,
                local_mana_trace,
                contact_trace: after_active.then_some(contact_trace).flatten(),
                causes,
            });
        }
        proposals.sort_unstable_by_key(|proposal| proposal.key);
        let traces =
            commit_mana_material_surface_effect_events(&mut state, self.next_time, &proposals)?;
        for (proposal, trace) in proposals.iter().zip(traces) {
            if proposal.after_condition.is_some() {
                state.mana_physical_effects = state.mana_physical_effects.saturating_add(1);
            }
            state.latest_physical_trace = trace;
            apply_local_mana_material_surface_transition(
                &mut state,
                self.next_time,
                proposal,
                trace,
            );
        }
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
        let material_signals = material_surface_physical_signals(&state, self.next_time);
        let feature_count = actor_perception_step(
            self.next_time,
            &mut state.actors,
            &objects,
            &material_signals,
        )?;
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

#[derive(Clone, Debug, PartialEq)]
struct MaterialSurfaceContactProposal {
    actor: ActorId,
    surface: MaterialSurfaceId,
    next_actor: ActorState,
    before_surface: MaterialSurface,
    after_surface: MaterialSurface,
    causes: Vec<TraceId>,
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

fn trace_descends_from(store: &CausalTraceStore, trace: TraceId, ancestor: TraceId) -> bool {
    let mut pending = vec![trace];
    let mut visited = BTreeSet::new();
    while let Some(candidate) = pending.pop() {
        if candidate == ancestor {
            return true;
        }
        if candidate < ancestor {
            continue;
        }
        if !visited.insert(candidate) {
            continue;
        }
        if let Some(event) = store.event(candidate) {
            pending.extend(event.causes.iter().copied());
        }
    }
    false
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
    changes: &[causafera_resolution::ResolutionChange],
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

fn validate_material_surface_transition(
    traces: &CausalTraceStore,
    transition: &MaterialSurfaceTransition,
) -> Result<(), RuntimeError> {
    let event = traces
        .event(transition.transition_trace)
        .ok_or(RuntimeError::InvalidSnapshot("unknown trace reference"))?;
    if event.time != transition.occurred_at {
        return Err(RuntimeError::InvalidSnapshot(
            "material surface transition time does not match anchor",
        ));
    }
    match (transition.contact_trace, transition.mana_effect_trace) {
        (None, None) => {
            if event.kind != EventKindId::new(MATERIAL_SURFACE_BOOTSTRAP_EVENT_KIND)
                || event.phase != Phase::Lifecycle
                || transition.mana_total != 0
            {
                return Err(RuntimeError::InvalidSnapshot(
                    "material surface bootstrap transition has invalid lifecycle semantics",
                ));
            }
        }
        (Some(contact_trace), None) => {
            if contact_trace != transition.transition_trace
                || event.kind != EventKindId::new(MATERIAL_SURFACE_CONTACT_EVENT_KIND)
                || event.phase != Phase::Action
                || transition.mana_total != 0
            {
                return Err(RuntimeError::InvalidSnapshot(
                    "material surface contact anchor is not an actor contact",
                ));
            }
        }
        (Some(contact_trace), Some(mana_effect_trace)) => {
            if mana_effect_trace != transition.transition_trace
                || event.kind != EventKindId::new(MATERIAL_SURFACE_MANA_EVENT_KIND)
                || event.phase != Phase::Mana
            {
                return Err(RuntimeError::InvalidSnapshot(
                    "material surface mana anchor is not a mana effect",
                ));
            }
            if !event.causes.contains(&contact_trace) {
                return Err(RuntimeError::InvalidSnapshot(
                    "material surface mana anchor does not cite contact trace",
                ));
            }
        }
        (None, Some(_)) => {
            return Err(RuntimeError::InvalidSnapshot(
                "material surface mana transition is missing a contact anchor",
            ));
        }
    }
    let material_effect = event
        .effects
        .iter()
        .find(|effect| {
            effect.target().object_kind() == StateObjectKindId::new(MATERIAL_SURFACE_OBJECT_KIND)
                && effect.target().object_id() == material_surface_object_id(transition.id)
        })
        .ok_or(RuntimeError::InvalidSnapshot(
            "material surface transition effect target does not match surface",
        ))?;
    if material_effect.target().property()
        != StatePropertyId::new(MATERIAL_SURFACE_CONDITION_PROPERTY)
    {
        return Err(RuntimeError::InvalidSnapshot(
            "material surface transition effect property is not condition",
        ));
    }
    if !material_surface_fingerprint_matches_condition(
        material_effect.before(),
        transition.before_condition,
    ) || !material_surface_fingerprint_matches_condition(
        material_effect.after(),
        transition.after_condition,
    ) {
        return Err(RuntimeError::InvalidSnapshot(
            "material surface transition effect fingerprint does not match declared condition",
        ));
    }
    let before_contact_count = material_surface_fingerprint_contact_count(material_effect.before());
    let after_contact_count = material_surface_fingerprint_contact_count(material_effect.after());
    let contact_count_is_valid = match (transition.contact_trace, transition.mana_effect_trace) {
        (None, None) => before_contact_count == 0 && after_contact_count == 0,
        (Some(_), None) => after_contact_count == before_contact_count.saturating_add(1),
        (Some(_), Some(_)) => after_contact_count == before_contact_count,
        (None, Some(_)) => false,
    };
    if !contact_count_is_valid {
        return Err(RuntimeError::InvalidSnapshot(
            "material surface transition effect has invalid contact count semantics",
        ));
    }
    if transition.mana_effect_trace.is_some() {
        validate_material_surface_mana_contact_parent(traces, transition, material_effect)?;
    }
    Ok(())
}

fn validate_material_surface_last_transition(
    traces: &CausalTraceStore,
    id: MaterialSurfaceId,
    surface: MaterialSurface,
) -> Result<(), RuntimeError> {
    let event = traces
        .event(surface.last_transition)
        .ok_or(RuntimeError::InvalidSnapshot("unknown trace reference"))?;
    let material_effect = event
        .effects
        .iter()
        .find(|effect| {
            effect.target().object_kind() == StateObjectKindId::new(MATERIAL_SURFACE_OBJECT_KIND)
                && effect.target().object_id() == material_surface_object_id(id)
        })
        .ok_or(RuntimeError::InvalidSnapshot(
            "material surface last transition effect target does not match surface",
        ))?;
    if material_effect.target().property()
        != StatePropertyId::new(MATERIAL_SURFACE_CONDITION_PROPERTY)
    {
        return Err(RuntimeError::InvalidSnapshot(
            "material surface last transition effect property is not condition",
        ));
    }
    if material_effect.after() != material_surface_fingerprint(surface) {
        return Err(RuntimeError::InvalidSnapshot(
            "material surface last transition effect does not match persisted surface",
        ));
    }
    let before_contact_count = material_surface_fingerprint_contact_count(material_effect.before());
    let after_contact_count = material_surface_fingerprint_contact_count(material_effect.after());
    let semantics_are_valid = match event.kind.raw() {
        MATERIAL_SURFACE_BOOTSTRAP_EVENT_KIND => {
            event.phase == Phase::Lifecycle
                && material_effect.before()
                    == material_surface_fingerprint(MaterialSurface {
                        condition: 0,
                        contact_count: 0,
                        last_transition: TraceId::new(0),
                        last_contact_trace: None,
                        gate: MaterialSurfaceManaGate {
                            active: false,
                            last_transition: None,
                        },
                    })
                && before_contact_count == 0
                && after_contact_count == 0
        }
        MATERIAL_SURFACE_CONTACT_EVENT_KIND => {
            event.phase == Phase::Action
                && after_contact_count == before_contact_count.saturating_add(1)
        }
        MATERIAL_SURFACE_MANA_EVENT_KIND => {
            event.phase == Phase::Mana && after_contact_count == before_contact_count
        }
        _ => false,
    };
    if !semantics_are_valid {
        return Err(RuntimeError::InvalidSnapshot(
            "material surface last transition has invalid event semantics",
        ));
    }
    if event.kind == EventKindId::new(MATERIAL_SURFACE_MANA_EVENT_KIND) {
        let contact_event = event
            .causes
            .iter()
            .find_map(|trace| {
                let candidate = traces.event(*trace)?;
                (candidate.kind == EventKindId::new(MATERIAL_SURFACE_CONTACT_EVENT_KIND)
                    && candidate.phase == Phase::Action)
                    .then_some(candidate)
            })
            .ok_or(RuntimeError::InvalidSnapshot(
                "material surface last mana transition has no contact parent",
            ))?;
        let contact_effect = contact_event
            .effects
            .iter()
            .find(|effect| {
                effect.target().object_kind()
                    == StateObjectKindId::new(MATERIAL_SURFACE_OBJECT_KIND)
                    && effect.target().object_id() == material_surface_object_id(id)
                    && effect.target().property()
                        == StatePropertyId::new(MATERIAL_SURFACE_CONDITION_PROPERTY)
            })
            .ok_or(RuntimeError::InvalidSnapshot(
                "material surface last mana transition contact parent does not target surface",
            ))?;
        if material_surface_fingerprint_contact_count(contact_effect.after())
            != material_surface_fingerprint_contact_count(contact_effect.before()).saturating_add(1)
        {
            return Err(RuntimeError::InvalidSnapshot(
                "material surface last mana transition has invalid contact parent",
            ));
        }
        let has_condition_parent = event.causes.iter().any(|trace| {
            traces.event(*trace).is_some_and(|candidate| {
                candidate.effects.iter().any(|effect| {
                    effect.target().object_kind()
                        == StateObjectKindId::new(MATERIAL_SURFACE_OBJECT_KIND)
                        && effect.target().object_id() == material_surface_object_id(id)
                        && effect.target().property()
                            == StatePropertyId::new(MATERIAL_SURFACE_CONDITION_PROPERTY)
                        && effect.after() == material_effect.before()
                })
            })
        });
        if !has_condition_parent {
            return Err(RuntimeError::InvalidSnapshot(
                "material surface last mana transition has no matching condition parent",
            ));
        }
    }
    Ok(())
}

fn validate_material_surface_last_contact_trace(
    traces: &CausalTraceStore,
    id: MaterialSurfaceId,
    surface: MaterialSurface,
) -> Result<(), RuntimeError> {
    match (surface.contact_count, surface.last_contact_trace) {
        (0, None) => Ok(()),
        (0, Some(_)) => Err(RuntimeError::InvalidSnapshot(
            "uncontacted material surface has a contact trace",
        )),
        (_, None) => Err(RuntimeError::InvalidSnapshot(
            "contacted material surface is missing a contact trace",
        )),
        (contact_count, Some(trace)) => {
            let event = traces
                .event(trace)
                .ok_or(RuntimeError::InvalidSnapshot("unknown trace reference"))?;
            let effect = event
                .effects
                .iter()
                .find(|effect| {
                    effect.target().object_kind()
                        == StateObjectKindId::new(MATERIAL_SURFACE_OBJECT_KIND)
                        && effect.target().object_id() == material_surface_object_id(id)
                        && effect.target().property()
                            == StatePropertyId::new(MATERIAL_SURFACE_CONDITION_PROPERTY)
                })
                .ok_or(RuntimeError::InvalidSnapshot(
                    "contact trace does not target material surface condition",
                ))?;
            if event.kind != EventKindId::new(MATERIAL_SURFACE_CONTACT_EVENT_KIND)
                || event.phase != Phase::Action
                || material_surface_fingerprint_contact_count(effect.after()) != contact_count
                || material_surface_fingerprint_contact_count(effect.before())
                    != contact_count.saturating_sub(1)
            {
                return Err(RuntimeError::InvalidSnapshot(
                    "contact trace does not match persisted material contact count",
                ));
            }
            Ok(())
        }
    }
}

fn validate_material_surface_gate_state(
    traces: &CausalTraceStore,
    id: MaterialSurfaceId,
    surface: MaterialSurface,
) -> Result<(), RuntimeError> {
    if surface.contact_count == 0 && surface.gate.last_transition.is_some() {
        return Err(RuntimeError::InvalidSnapshot(
            "uncontacted material surface has a gate transition",
        ));
    }
    let Some(trace) = surface.gate.last_transition else {
        return if surface.gate.active {
            Err(RuntimeError::InvalidSnapshot(
                "active material surface gate is missing a transition",
            ))
        } else {
            Ok(())
        };
    };
    let event = traces
        .event(trace)
        .ok_or(RuntimeError::InvalidSnapshot("unknown trace reference"))?;
    let gate_effect = event
        .effects
        .iter()
        .find(|effect| {
            effect.target().object_kind() == StateObjectKindId::new(MATERIAL_SURFACE_OBJECT_KIND)
                && effect.target().object_id() == material_surface_object_id(id)
                && effect.target().property()
                    == StatePropertyId::new(MATERIAL_SURFACE_MANA_GATE_PROPERTY)
        })
        .ok_or(RuntimeError::InvalidSnapshot(
            "material surface gate transition does not target gate property",
        ))?;
    let expected_before = material_surface_gate_fingerprint(!surface.gate.active);
    let expected_after = material_surface_gate_fingerprint(surface.gate.active);
    if event.kind != EventKindId::new(MATERIAL_SURFACE_MANA_EVENT_KIND)
        || event.phase != Phase::Mana
        || gate_effect.before() != expected_before
        || gate_effect.after() != expected_after
    {
        return Err(RuntimeError::InvalidSnapshot(
            "material surface gate state does not match its transition",
        ));
    }
    Ok(())
}

fn expected_gate_transition_causes(
    transition: &MaterialSurfaceGateTransition,
    prior_gate_trace: Option<TraceId>,
    prior_condition_trace: Option<TraceId>,
) -> Result<Vec<TraceId>, RuntimeError> {
    let mut causes = BTreeSet::new();
    causes.insert(transition.local_mana_trace);
    if let Some(trace) = prior_gate_trace {
        causes.insert(trace);
    }
    if transition.after_active {
        let Some(contact_trace) = transition.contact_trace else {
            return Err(RuntimeError::InvalidSnapshot(
                "rising material surface gate transition is missing contact trace",
            ));
        };
        causes.insert(contact_trace);
        if let Some(prior_condition) = prior_condition_trace {
            if prior_condition != contact_trace {
                causes.insert(prior_condition);
            }
        }
    }
    Ok(causes.into_iter().collect())
}

fn prior_material_surface_gate_trace(
    traces: &CausalTraceStore,
    id: MaterialSurfaceId,
    before: TraceId,
) -> Option<TraceId> {
    let object_id = material_surface_object_id(id);
    let object_kind = StateObjectKindId::new(MATERIAL_SURFACE_OBJECT_KIND);
    let gate_property = StatePropertyId::new(MATERIAL_SURFACE_MANA_GATE_PROPERTY);
    traces
        .iter()
        .filter(|event| {
            event.trace_id < before
                && event.kind == EventKindId::new(MATERIAL_SURFACE_MANA_EVENT_KIND)
                && event.phase == Phase::Mana
                && event.effects.iter().any(|effect| {
                    effect.target().object_kind() == object_kind
                        && effect.target().object_id() == object_id
                        && effect.target().property() == gate_property
                })
        })
        .map(|event| event.trace_id)
        .max()
}

fn prior_material_surface_condition_trace(
    traces: &CausalTraceStore,
    id: MaterialSurfaceId,
    before: TraceId,
) -> Option<TraceId> {
    let object_id = material_surface_object_id(id);
    let object_kind = StateObjectKindId::new(MATERIAL_SURFACE_OBJECT_KIND);
    let condition_property = StatePropertyId::new(MATERIAL_SURFACE_CONDITION_PROPERTY);
    traces
        .iter()
        .filter(|event| {
            event.trace_id < before
                && event.kind == EventKindId::new(MATERIAL_SURFACE_MANA_EVENT_KIND)
                && event.phase == Phase::Mana
                && event.effects.iter().any(|effect| {
                    effect.target().object_kind() == object_kind
                        && effect.target().object_id() == object_id
                        && effect.target().property() == condition_property
                })
        })
        .map(|event| event.trace_id)
        .max()
}

fn validate_material_surface_gate_transition_history(
    traces: &CausalTraceStore,
    surfaces: &BTreeMap<MaterialSurfaceId, MaterialSurface>,
    _material_transitions: &[MaterialSurfaceTransition],
    gate_transitions: &[MaterialSurfaceGateTransition],
) -> Result<(), RuntimeError> {
    let mut latest_gate_by_surface: BTreeMap<MaterialSurfaceId, &MaterialSurfaceGateTransition> =
        BTreeMap::new();
    for transition in gate_transitions {
        latest_gate_by_surface.insert(transition.id, transition);
    }
    for (id, latest) in latest_gate_by_surface {
        let _surface = surfaces.get(&id).ok_or(RuntimeError::InvalidSnapshot(
            "material surface gate transition references unknown surface",
        ))?;
        let event = traces
            .event(latest.transition_trace)
            .ok_or(RuntimeError::InvalidSnapshot("unknown trace reference"))?;
        let prior_condition_trace = latest
            .after_active
            .then(|| prior_material_surface_condition_trace(traces, id, latest.transition_trace))
            .flatten();
        let prior_gate_trace =
            prior_material_surface_gate_trace(traces, id, latest.transition_trace);
        let expected_causes =
            expected_gate_transition_causes(latest, prior_gate_trace, prior_condition_trace)?;
        if event.causes != expected_causes {
            return Err(RuntimeError::InvalidSnapshot(
                "latest material surface gate transition has incorrect causal parent set",
            ));
        }
    }
    Ok(())
}

fn validate_material_surface_gate_transition(
    traces: &CausalTraceStore,
    material_transitions: &[MaterialSurfaceTransition],
    transition: &MaterialSurfaceGateTransition,
) -> Result<(), RuntimeError> {
    let event = traces
        .event(transition.transition_trace)
        .ok_or(RuntimeError::InvalidSnapshot("unknown trace reference"))?;
    if event.kind != EventKindId::new(MATERIAL_SURFACE_MANA_EVENT_KIND)
        || event.phase != Phase::Mana
        || event.time != transition.occurred_at
    {
        return Err(RuntimeError::InvalidSnapshot(
            "material surface gate transition has invalid event semantics",
        ));
    }
    let gate_effect = event
        .effects
        .iter()
        .find(|effect| {
            effect.target().object_kind() == StateObjectKindId::new(MATERIAL_SURFACE_OBJECT_KIND)
                && effect.target().object_id() == material_surface_object_id(transition.id)
                && effect.target().property()
                    == StatePropertyId::new(MATERIAL_SURFACE_MANA_GATE_PROPERTY)
        })
        .ok_or(RuntimeError::InvalidSnapshot(
            "material surface gate transition is missing its gate effect",
        ))?;
    if gate_effect.before() != material_surface_gate_fingerprint(transition.before_active)
        || gate_effect.after() != material_surface_gate_fingerprint(transition.after_active)
    {
        return Err(RuntimeError::InvalidSnapshot(
            "material surface gate transition gate effect does not match record",
        ));
    }
    validate_local_mana_transition(traces, transition)?;
    if !event.causes.contains(&transition.local_mana_trace) {
        return Err(RuntimeError::InvalidSnapshot(
            "material surface gate transition does not cite local mana trace",
        ));
    }
    match (
        transition.before_active,
        transition.after_active,
        transition.contact_trace,
    ) {
        (false, true, Some(contact_trace)) => {
            if !event.causes.contains(&contact_trace) {
                return Err(RuntimeError::InvalidSnapshot(
                    "rising material surface gate transition does not cite contact trace",
                ));
            }
            validate_material_surface_last_contact_event(traces, transition.id, contact_trace)?;
            if event.effects.len() != 2
                || !material_transitions.iter().any(|condition| {
                    condition.id == transition.id
                        && condition.transition_trace == transition.transition_trace
                        && condition.mana_effect_trace == Some(transition.transition_trace)
                })
            {
                return Err(RuntimeError::InvalidSnapshot(
                    "rising material surface gate transition is missing condition evidence",
                ));
            }
        }
        (true, false, None) if event.effects.len() == 1 => {}
        _ => {
            return Err(RuntimeError::InvalidSnapshot(
                "material surface gate transition has invalid rising or falling shape",
            ));
        }
    }
    Ok(())
}

fn validate_material_surface_last_contact_event(
    traces: &CausalTraceStore,
    id: MaterialSurfaceId,
    trace: TraceId,
) -> Result<(), RuntimeError> {
    let event = traces
        .event(trace)
        .ok_or(RuntimeError::InvalidSnapshot("unknown trace reference"))?;
    if event.kind != EventKindId::new(MATERIAL_SURFACE_CONTACT_EVENT_KIND)
        || event.phase != Phase::Action
        || !event.effects.iter().any(|effect| {
            effect.target().object_kind() == StateObjectKindId::new(MATERIAL_SURFACE_OBJECT_KIND)
                && effect.target().object_id() == material_surface_object_id(id)
                && effect.target().property()
                    == StatePropertyId::new(MATERIAL_SURFACE_CONDITION_PROPERTY)
        })
    {
        return Err(RuntimeError::InvalidSnapshot(
            "material surface gate transition contact trace is invalid",
        ));
    }
    Ok(())
}

fn validate_local_mana_transition(
    traces: &CausalTraceStore,
    transition: &MaterialSurfaceGateTransition,
) -> Result<(), RuntimeError> {
    let event = traces
        .event(transition.local_mana_trace)
        .ok_or(RuntimeError::InvalidSnapshot("unknown trace reference"))?;
    let object_id = cell_object_id(transition.id.chunk, transition.id.cell_index);
    let ordinary = event.kind == EventKindId::new(MANA_EVENT_KIND)
        && event.phase == Phase::Mana
        && event.effects.iter().any(|effect| {
            effect.target().object_kind() == StateObjectKindId::new(MANA_OBJECT_KIND)
                && effect.target().object_id() == object_id
                && effect.target().property() == StatePropertyId::new(MANA_PROPERTY)
                && effect.before() == fingerprint_i64(0x0301, transition.local_mana_before)
                && effect.after() == fingerprint_i64(0x0301, transition.local_mana_after)
        });
    let source = event.kind == EventKindId::new(EXPERIMENT_RECIPE_MANA_SOURCE_EVENT_KIND)
        && event.phase == Phase::Mana
        && event.effects.iter().any(|effect| {
            effect.target().object_kind()
                == StateObjectKindId::new(EXPERIMENT_RECIPE_MANA_SOURCE_OBJECT_KIND)
                && effect.target().object_id() == object_id
                && effect.target().property()
                    == StatePropertyId::new(EXPERIMENT_RECIPE_MANA_SOURCE_PROPERTY)
                && effect.before() == fingerprint_i64(0x0302, transition.local_mana_before)
                && effect.after() == fingerprint_i64(0x0302, transition.local_mana_after)
        });
    if ordinary || source {
        Ok(())
    } else {
        Err(RuntimeError::InvalidSnapshot(
            "material surface gate transition local mana evidence is invalid",
        ))
    }
}

fn material_surface_mana_transition_evidence(
    traces: &CausalTraceStore,
    receipts: &[ExperimentRecipeManaSourceReceipt],
    transition: &MaterialSurfaceTransition,
) -> (Option<TraceId>, Option<i64>, Option<i64>) {
    let Some(mana_effect_trace) = transition.mana_effect_trace else {
        return (None, None, None);
    };
    let Some(mana_effect_event) = traces.event(mana_effect_trace) else {
        return (None, None, None);
    };
    let Some(mana_transition_trace) = mana_effect_event.causes.iter().copied().find(|cause| {
        traces.event(*cause).is_some_and(|event| {
            event.kind == EventKindId::new(MANA_EVENT_KIND)
                || event.kind == EventKindId::new(EXPERIMENT_RECIPE_MANA_SOURCE_EVENT_KIND)
        })
    }) else {
        return (None, None, None);
    };
    let receipt = receipts
        .iter()
        .find(|receipt| receipt.source_trace == mana_transition_trace);
    (
        Some(mana_transition_trace),
        receipt.map(|receipt| receipt.before_intensity),
        receipt.map(|receipt| receipt.after_intensity),
    )
}

fn validate_material_surface_mana_contact_parent(
    traces: &CausalTraceStore,
    mana_transition: &MaterialSurfaceTransition,
    mana_effect: &CausalEffect,
) -> Result<(), RuntimeError> {
    let contact_trace = mana_transition
        .contact_trace
        .ok_or(RuntimeError::InvalidSnapshot(
            "material surface mana transition is missing a contact anchor",
        ))?;
    let contact_event = traces
        .event(contact_trace)
        .ok_or(RuntimeError::InvalidSnapshot("unknown trace reference"))?;
    if contact_event.phase != Phase::Action
        || contact_event.kind != EventKindId::new(MATERIAL_SURFACE_CONTACT_EVENT_KIND)
    {
        return Err(RuntimeError::InvalidSnapshot(
            "material surface mana contact parent is not an actor contact",
        ));
    }
    let contact_effect = contact_event
        .effects
        .iter()
        .find(|effect| {
            effect.target().object_kind() == StateObjectKindId::new(MATERIAL_SURFACE_OBJECT_KIND)
                && effect.target().object_id() == material_surface_object_id(mana_transition.id)
                && effect.target().property()
                    == StatePropertyId::new(MATERIAL_SURFACE_CONDITION_PROPERTY)
        })
        .ok_or(RuntimeError::InvalidSnapshot(
            "material surface mana contact parent does not target declared condition",
        ))?;
    if material_surface_fingerprint_contact_count(contact_effect.after())
        != material_surface_fingerprint_contact_count(contact_effect.before()).saturating_add(1)
    {
        return Err(RuntimeError::InvalidSnapshot(
            "material surface mana contact parent has invalid contact count semantics",
        ));
    }
    let mana_event = traces
        .event(mana_transition.transition_trace)
        .ok_or(RuntimeError::InvalidSnapshot("unknown trace reference"))?;
    let has_condition_parent = mana_event.causes.iter().any(|trace| {
        traces.event(*trace).is_some_and(|event| {
            event.effects.iter().any(|effect| {
                effect.target().object_kind()
                    == StateObjectKindId::new(MATERIAL_SURFACE_OBJECT_KIND)
                    && effect.target().object_id() == material_surface_object_id(mana_transition.id)
                    && effect.target().property()
                        == StatePropertyId::new(MATERIAL_SURFACE_CONDITION_PROPERTY)
                    && effect.after() == mana_effect.before()
            })
        })
    });
    if !has_condition_parent {
        return Err(RuntimeError::InvalidSnapshot(
            "material surface mana transition has no matching prior condition parent",
        ));
    }
    Ok(())
}

fn material_surface_fingerprint_matches_condition(
    fingerprint: StateFingerprint,
    condition: i64,
) -> bool {
    material_surface_fingerprint(MaterialSurface {
        condition,
        contact_count: material_surface_fingerprint_contact_count(fingerprint),
        last_transition: TraceId::new(0),
        last_contact_trace: None,
        gate: MaterialSurfaceManaGate {
            active: false,
            last_transition: None,
        },
    }) == fingerprint
}

fn material_surface_fingerprint_contact_count(fingerprint: StateFingerprint) -> u64 {
    let bytes = fingerprint.bytes();
    let mut encoded = [0_u8; 8];
    encoded.copy_from_slice(&bytes[16..24]);
    u64::from_le_bytes(encoded)
}

fn write_chart_chunk(digest: &mut CanonicalDigest, chunk: ChartChunkCoord) {
    digest.write(chunk.chart.raw());
    digest.write(chunk.chunk.x as u64);
    digest.write(chunk.chunk.y as u64);
    digest.write(chunk.chunk.z as u64);
}

fn write_optional_trace(digest: &mut CanonicalDigest, trace: Option<TraceId>) {
    match trace {
        Some(trace) => {
            digest.write(1);
            digest.write(trace.raw());
        }
        None => digest.write(0),
    }
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

fn material_surface_object_id(id: MaterialSurfaceId) -> u64 {
    cell_object_id(id.chunk, id.cell_index)
}

fn material_surface_fingerprint(surface: MaterialSurface) -> StateFingerprint {
    fingerprint_pair(
        0x0D01,
        surface.condition,
        i64::try_from(surface.contact_count).unwrap_or(i64::MAX),
    )
}

fn material_surface_gate_fingerprint(active: bool) -> StateFingerprint {
    fingerprint_u64(0x0D03, u64::from(active))
}

fn ordered_trace_causes(causes: [TraceId; 2]) -> Vec<TraceId> {
    let mut ordered = causes.to_vec();
    ordered.sort_unstable();
    ordered.dedup();
    ordered
}

fn commit_material_surface_bootstrap_event(
    state: &mut RuntimeState,
    stage: HistoricalStageId,
    ordinal: u64,
    id: MaterialSurfaceId,
    initial_condition: i64,
) -> Result<TraceId, RuntimeError> {
    let before = MaterialSurface {
        condition: 0,
        contact_count: 0,
        last_transition: state.latest_physical_trace,
        last_contact_trace: None,
        gate: MaterialSurfaceManaGate {
            active: false,
            last_transition: None,
        },
    };
    let after = MaterialSurface {
        condition: initial_condition,
        contact_count: 0,
        last_transition: state.latest_physical_trace,
        last_contact_trace: None,
        gate: MaterialSurfaceManaGate {
            active: false,
            last_transition: None,
        },
    };
    let event = CausalEventProposal::new(
        EventProposalKey::new(BOOTSTRAP_SYSTEM_ID, stage.raw(), ordinal),
        EventKindId::new(MATERIAL_SURFACE_BOOTSTRAP_EVENT_KIND),
        vec![state.latest_physical_trace],
        vec![CausalEffect::new(
            CausalTarget::new(
                StateObjectKindId::new(MATERIAL_SURFACE_OBJECT_KIND),
                material_surface_object_id(id),
                StatePropertyId::new(MATERIAL_SURFACE_CONDITION_PROPERTY),
            ),
            material_surface_fingerprint(before),
            material_surface_fingerprint(after),
        )?],
    )?;
    let trace = state
        .traces
        .commit_batch(SimulationTime::new(0), Phase::Lifecycle, vec![event])?[0];
    state.latest_physical_trace = trace;
    Ok(trace)
}

fn commit_material_surface_contact_events(
    state: &mut RuntimeState,
    time: SimulationTime,
    ordinal: u64,
    actor: &ActorState,
    proposal: MaterialSurfaceContactProposal,
) -> Result<TraceId, RuntimeError> {
    let event = CausalEventProposal::new(
        EventProposalKey::new(ACTOR_ACTION_SYSTEM_ID, proposal.actor.raw(), ordinal),
        EventKindId::new(MATERIAL_SURFACE_CONTACT_EVENT_KIND),
        proposal.causes,
        vec![
            CausalEffect::new(
                CausalTarget::new(
                    StateObjectKindId::new(ACTOR_OBJECT_KIND),
                    proposal.actor.raw(),
                    StatePropertyId::new(ACTOR_BODY_PROPERTY),
                ),
                actor_state_fingerprint(actor),
                actor_state_fingerprint(&proposal.next_actor),
            )?,
            CausalEffect::new(
                CausalTarget::new(
                    StateObjectKindId::new(MATERIAL_SURFACE_OBJECT_KIND),
                    material_surface_object_id(proposal.surface),
                    StatePropertyId::new(MATERIAL_SURFACE_CONDITION_PROPERTY),
                ),
                material_surface_fingerprint(proposal.before_surface),
                material_surface_fingerprint(proposal.after_surface),
            )?,
        ],
    )?;
    let trace = state
        .traces
        .commit_batch(time, Phase::Action, vec![event])?[0];
    state.material_surfaces.insert(
        proposal.surface,
        MaterialSurface {
            last_transition: trace,
            last_contact_trace: Some(trace),
            ..proposal.after_surface
        },
    );
    state
        .pending_material_surface_changes
        .insert(proposal.surface);
    record_material_surface_transition(
        state,
        MaterialSurfaceTransition {
            id: proposal.surface,
            occurred_at: time,
            before_condition: proposal.before_surface.condition,
            after_condition: proposal.after_surface.condition,
            mana_total: 0,
            contact_trace: Some(trace),
            mana_effect_trace: None,
            transition_trace: trace,
        },
    );
    Ok(trace)
}

fn commit_mana_material_surface_effect_events(
    state: &mut RuntimeState,
    time: SimulationTime,
    proposals: &[LocalManaMaterialSurfaceProposal],
) -> Result<Vec<TraceId>, RuntimeError> {
    let events = proposals
        .iter()
        .map(|proposal| {
            let mut effects = vec![CausalEffect::new(
                CausalTarget::new(
                    StateObjectKindId::new(MATERIAL_SURFACE_OBJECT_KIND),
                    material_surface_object_id(proposal.surface),
                    StatePropertyId::new(MATERIAL_SURFACE_MANA_GATE_PROPERTY),
                ),
                material_surface_gate_fingerprint(proposal.before.gate.active),
                material_surface_gate_fingerprint(proposal.after_active),
            )?];
            if let Some(after_condition) = proposal.after_condition {
                let mut after = proposal.before;
                after.condition = after_condition;
                effects.push(CausalEffect::new(
                    CausalTarget::new(
                        StateObjectKindId::new(MATERIAL_SURFACE_OBJECT_KIND),
                        material_surface_object_id(proposal.surface),
                        StatePropertyId::new(MATERIAL_SURFACE_CONDITION_PROPERTY),
                    ),
                    material_surface_fingerprint(proposal.before),
                    material_surface_fingerprint(after),
                )?);
            }
            effects.sort_unstable_by_key(|effect| effect.target());
            CausalEventProposal::new(
                proposal.key,
                EventKindId::new(MATERIAL_SURFACE_MANA_EVENT_KIND),
                proposal.causes.clone(),
                effects,
            )
            .map_err(RuntimeError::from)
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(state.traces.commit_batch(time, Phase::Mana, events)?)
}

fn apply_local_mana_material_surface_transition(
    state: &mut RuntimeState,
    time: SimulationTime,
    proposal: &LocalManaMaterialSurfaceProposal,
    trace: TraceId,
) {
    let mut after = proposal.before;
    after.gate.active = proposal.after_active;
    after.gate.last_transition = Some(trace);
    if let Some(condition) = proposal.after_condition {
        after.condition = condition;
        after.last_transition = trace;
        state
            .pending_material_surface_changes
            .insert(proposal.surface);
        record_material_surface_transition(
            state,
            MaterialSurfaceTransition {
                id: proposal.surface,
                occurred_at: time,
                before_condition: proposal.before.condition,
                after_condition: condition,
                mana_total: state.mana.total_intensity(),
                contact_trace: proposal.contact_trace,
                mana_effect_trace: Some(trace),
                transition_trace: trace,
            },
        );
    }
    state.material_surfaces.insert(proposal.surface, after);
    record_material_surface_gate_transition(
        state,
        MaterialSurfaceGateTransition {
            id: proposal.surface,
            occurred_at: time,
            before_active: proposal.before.gate.active,
            after_active: proposal.after_active,
            local_mana_before: proposal.local_mana_before,
            local_mana_after: proposal.local_mana_after,
            local_mana_trace: proposal.local_mana_trace,
            contact_trace: proposal.contact_trace,
            transition_trace: trace,
        },
    );
}

fn record_material_surface_transition(
    state: &mut RuntimeState,
    transition: MaterialSurfaceTransition,
) {
    if state.material_surface_transitions.len() == MAX_MATERIAL_SURFACE_TRANSITIONS {
        let evicted = state
            .material_surface_transitions
            .iter()
            .position(|existing| existing.mana_effect_trace.is_none())
            .unwrap_or(0);
        state.material_surface_transitions.remove(evicted);
    }
    state.material_surface_transitions.push(transition);
}

fn record_material_surface_gate_transition(
    state: &mut RuntimeState,
    transition: MaterialSurfaceGateTransition,
) {
    if state.material_surface_gate_transitions.len() == MAX_MATERIAL_SURFACE_TRANSITIONS {
        let evicted = state
            .material_surface_gate_transitions
            .iter()
            .position(|existing| !existing.after_active)
            .unwrap_or(0);
        state.material_surface_gate_transitions.remove(evicted);
    }
    state.material_surface_gate_transitions.push(transition);
}

fn resolve_material_surface(
    state: &RuntimeState,
    position: WorldCoord,
) -> Option<MaterialSurfaceId> {
    state.material_surfaces.keys().copied().min_by_key(|id| {
        let surface_position = WorldCoord::new(
            i64::from(id.chunk.chunk.x),
            i64::from(id.chunk.chunk.y),
            i64::from(id.chunk.chunk.z),
        );
        (
            position.x.abs_diff(surface_position.x)
                + position.y.abs_diff(surface_position.y)
                + position.z.abs_diff(surface_position.z),
            *id,
        )
    })
}

fn material_surface_physical_signals(
    state: &RuntimeState,
    time: SimulationTime,
) -> Vec<PhysicalSignal> {
    const MATERIAL_SURFACE_SIGNAL_GAIN: i64 = 16;
    if !state.config.material_surface_signals_enabled {
        return Vec::new();
    }
    state
        .material_surfaces
        .iter()
        .filter(|(_, surface)| surface.contact_count > 0)
        .map(|(id, surface)| {
            PhysicalSignal::new(
                EntityId::new(material_surface_object_id(*id)),
                SignalChannelId::new(
                    SensorKindId::new(1)
                        .raw()
                        .saturating_add(ACTOR_SIGNAL_CHANNEL),
                ),
                WorldCoord::new(
                    i64::from(id.chunk.chunk.x),
                    i64::from(id.chunk.chunk.y),
                    i64::from(id.chunk.chunk.z),
                ),
                SignalMagnitude::new(
                    surface
                        .condition
                        .saturating_mul(MATERIAL_SURFACE_SIGNAL_GAIN),
                ),
                time,
                surface.last_transition,
            )
        })
        .collect()
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

fn experiment_recipe_mana_source_receipt_fingerprint(
    record: &ExperimentRecipeManaSource,
    executed_tick: u64,
    source_trace: TraceId,
    before_intensity: i64,
    after_intensity: i64,
    recipe_hash: StateFingerprint,
) -> StateFingerprint {
    let mut digest = CanonicalDigest::new();
    digest.write(0x0303);
    digest.write(record.source_record_id);
    digest.write(record.scheduled_tick);
    digest.write(executed_tick);
    digest.write(source_trace.raw());
    digest.write(before_intensity as u64);
    digest.write(after_intensity as u64);
    digest.write_bytes(recipe_hash.bytes());
    digest.write(record.policy_schema_id);
    digest.finish()
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
            use super::{CausalTarget, StateFingerprint};
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
