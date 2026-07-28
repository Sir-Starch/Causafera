use crate::*;
use causafera_core::*;
use causafera_domains::*;
use causafera_observer_api::*;
use causafera_resolution::*;
use causafera_types::*;

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
    pub thermal_total_cell_energy: i128,
    pub thermal_total_reservoir_budget: i128,
    pub thermal_active_chunk_count: u32,
    pub thermal_active_cell_count: u32,
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
            thermal_total_cell_energy: self.thermal_total_cell_energy,
            thermal_total_reservoir_budget: self.thermal_total_reservoir_budget,
            thermal_active_chunk_count: self.thermal_active_chunk_count,
            thermal_active_cell_count: self.thermal_active_cell_count,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct RuntimeSnapshotData {
    pub recipe: RuntimeRecipeSnapshot,
    pub spatial: SpatialChunkSnapshot,
    pub mana: ManaFieldSetSnapshot,
    pub thermal: ThermalSnapshot,
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
pub struct ThermalSnapshot {
    pub parameters: ThermalParameters,
    pub field_set: ThermalFieldSetSnapshot,
    pub active_region: ThermalActiveRegionSnapshot,
    pub reservoirs: Vec<ThermalReservoirSnapshot>,
    pub receipt_batches: Vec<TraceId>,
    pub transfer_receipts: Vec<ThermalCellTransferReceiptSnapshot>,
    pub conservation_receipts: Vec<ThermalConservationReceiptSnapshot>,
    pub boundary_records: Vec<ThermalBoundaryRecordSnapshot>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ThermalFieldSetSnapshot {
    pub fields: Vec<ThermalFieldSnapshot>,
    pub batch_sequence: u64,
    pub conservation_last_change: TraceId,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ThermalFieldSnapshot {
    pub chunk: ChartChunkCoord,
    pub extent: u8,
    pub energy: Vec<i64>,
    pub last_change: Vec<TraceId>,
    pub last_change_before: Vec<i64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ThermalActiveRegionSnapshot {
    pub active_chunks: Vec<ChartChunkCoord>,
    pub resident_chunks: Vec<ChartChunkCoord>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ThermalReservoirSnapshot {
    pub id: ThermalReservoirId,
    pub target: ThermalCellKey,
    pub budget: i64,
    pub schedule: ThermalReservoirScheduleSnapshot,
    pub bootstrap_trace: TraceId,
    pub last_change: TraceId,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThermalReservoirScheduleSnapshot {
    PerTick(i64),
    OneShot,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ThermalCellTransferReceiptSnapshot {
    pub conservation_trace: TraceId,
    pub cell: ThermalCellKey,
    pub pre_state: i64,
    pub post_state: i64,
    pub cell_change_trace_id: Option<TraceId>,
    pub faces: Vec<ThermalFaceRecordSnapshot>,
    pub reservoirs: Vec<ThermalReservoirTransferRecordSnapshot>,
    pub material: Option<ThermalMaterialTransferRecordSnapshot>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ThermalMaterialTransferRecordSnapshot {
    pub retained_before: i64,
    pub retained_after: i64,
    pub signed_flux: i64,
    pub rejected: i64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ThermalFaceRecordSnapshot {
    pub neighbor: ThermalCellKey,
    pub signed_flux: i64,
    pub neighbor_pre_state: i64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ThermalReservoirTransferRecordSnapshot {
    pub id: ThermalReservoirId,
    pub scheduled_injection: i64,
    pub accepted_injection: i64,
    pub rejected_injection: i64,
    pub transfer_trace_id: Option<TraceId>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ThermalConservationReceiptSnapshot {
    pub trace: TraceId,
    pub tick: u64,
    pub total_cell_energy_before: i128,
    pub total_cell_energy_after: i128,
    pub total_reservoir_budget_before: i128,
    pub total_reservoir_budget_after: i128,
    pub total_material_retained_before: i128,
    pub total_material_retained_after: i128,
    pub residual: i128,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ThermalBoundaryRecordSnapshot {
    pub cell: ThermalCellKey,
    pub neighbor: ThermalCellKey,
    pub cell_pre_state: i64,
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

/// The persisted canonical production bootstrap record.
///
/// This is the complete contract, not a stage/trace index: the plan the stages
/// were executed from, the bounded per-stage result state they left behind, and
/// one terminal receipt per stage. An empty receipt list is not a valid
/// production record and import rejects it rather than defaulting it in.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BootstrapReceiptSnapshot {
    pub plan: BootstrapPlanSnapshot,
    pub stage_results: Vec<BootstrapStageResultSnapshot>,
    pub receipts: Vec<BootstrapReceiptRecord>,
}

impl BootstrapReceiptSnapshot {
    /// The shape a state with no canonical record exports.
    ///
    /// Only reachable for a `RuntimeState` that has not completed bootstrap,
    /// which no constructor or import path hands out. It decodes back to itself
    /// and is then rejected by import, so it can never become production state.
    pub fn absent() -> Self {
        Self {
            plan: BootstrapPlanSnapshot {
                id: HistoricalBootstrapId::new(0),
                world_seed: 0,
                stages: Vec::new(),
            },
            stage_results: Vec::new(),
            receipts: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BootstrapPlanSnapshot {
    pub id: HistoricalBootstrapId,
    pub world_seed: u64,
    pub stages: Vec<BootstrapStageSnapshot>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BootstrapStageSnapshot {
    pub stage: HistoricalStageId,
    pub process: HistoricalProcessSchemaId,
    pub starts_at: SimulationTime,
    pub ends_at: SimulationTime,
    pub detail_ordinal: u8,
    pub targets: Vec<ChunkId>,
    pub dependencies: Vec<HistoricalStageId>,
    pub external_causes: Vec<TraceId>,
    pub parameters: StateFingerprint,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BootstrapStageResultSnapshot {
    pub stage: HistoricalStageId,
    pub result: StateFingerprint,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BootstrapReceiptRecord {
    pub stage: HistoricalStageId,
    pub completed_at: SimulationTime,
    pub result: StateFingerprint,
    pub trace: TraceId,
    pub causes: Vec<TraceId>,
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
