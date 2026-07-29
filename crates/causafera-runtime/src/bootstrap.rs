use std::collections::{BTreeMap, BTreeSet};

use crate::digests::mix64;
use crate::*;
use causafera_core::*;
use causafera_domains::{
    THERMAL_SCALE, ThermalEnergy, ThermalField, ThermalFieldSet, ThermalReservoir,
    ThermalReservoirId, ThermalReservoirSchedule,
};
use causafera_observer_api::{
    BOOTSTRAP_SUMMARY_SCHEMA_V1, MAX_BOOTSTRAP_RECEIPT_DEPENDENCIES,
    MAX_BOOTSTRAP_RECEIPT_SUMMARIES, ObserverBootstrapReceipt, ObserverBootstrapSummary,
};
use causafera_types::*;
use thiserror::Error;

pub use causafera_world::{
    HistoricalBootstrapError, HistoricalBootstrapPlan, HistoricalBootstrapRecord, HistoricalStage,
    HistoricalStageReceipt, MAX_HISTORICAL_STAGES, MAX_STAGE_DEPENDENCIES,
    MAX_STAGE_EXTERNAL_CAUSES, MAX_STAGE_TARGETS,
};

/// The six executable stages the current production bootstrap runs.
///
/// This is the complete implementation surface of the canonical plan today. It
/// is a bound on what the runtime executes, not a claim that historical
/// synthesis is only ever six steps.
pub const BOOTSTRAP_STAGE_COUNT: usize = 6;

pub const PHYSICAL_GEOGRAPHY_STAGE: HistoricalStageId = HistoricalStageId::new(1);
pub const MATERIAL_SURFACE_STAGE: HistoricalStageId = HistoricalStageId::new(2);
pub const POPULATION_LIFECYCLE_STAGE: HistoricalStageId = HistoricalStageId::new(3);
pub const ACTOR_PROMOTION_STAGE: HistoricalStageId = HistoricalStageId::new(4);
pub const MATERIAL_ACTIVITY_STAGE: HistoricalStageId = HistoricalStageId::new(5);
pub const THERMAL_RESERVOIR_STAGE: HistoricalStageId = HistoricalStageId::new(6);

/// Opaque process identities. They are numeric schema IDs, never human-language
/// event names, and carry no meaning a downstream reader may translate.
pub const PHYSICAL_GEOGRAPHY_PROCESS_SCHEMA: HistoricalProcessSchemaId =
    HistoricalProcessSchemaId::new(0x0B01);
pub const MATERIAL_SURFACE_PROCESS_SCHEMA: HistoricalProcessSchemaId =
    HistoricalProcessSchemaId::new(0x0B02);
pub const POPULATION_LIFECYCLE_PROCESS_SCHEMA: HistoricalProcessSchemaId =
    HistoricalProcessSchemaId::new(0x0B03);
pub const ACTOR_PROMOTION_PROCESS_SCHEMA: HistoricalProcessSchemaId =
    HistoricalProcessSchemaId::new(0x0B04);
pub const MATERIAL_ACTIVITY_PROCESS_SCHEMA: HistoricalProcessSchemaId =
    HistoricalProcessSchemaId::new(0x0B05);
pub const THERMAL_RESERVOIR_PROCESS_SCHEMA: HistoricalProcessSchemaId =
    HistoricalProcessSchemaId::new(0x0B06);

/// Domain separators. Each keeps one derivation from colliding with another that
/// happens to hash the same inputs.
const BOOTSTRAP_PLAN_ID_DOMAIN: u64 = 0x0B10_0000_0000_0001;
const BOOTSTRAP_TARGET_DOMAIN: u64 = 0x0B10_0000_0000_0002;
const BOOTSTRAP_PARAMETER_DOMAIN: u64 = 0x0B10_0000_0000_0003;
const BOOTSTRAP_RESULT_DOMAIN: u64 = 0x0B10_0000_0000_0004;

/// The bounded state a stage completion transitions away from.
///
/// A stage that commits no domain effect still commits this transition, so every
/// stage has one terminal receipt anchored to a real state change rather than to
/// decorative metadata.
const BOOTSTRAP_STAGE_RESULT_ABSENT_TAG: u64 = 0x0B11;

/// Operation ordinal reserved for a stage's single completion event.
///
/// Per-object stage events number their operations from zero upwards, so the
/// completion cannot collide with them for any reachable chunk or reservoir
/// count.
const BOOTSTRAP_STAGE_COMPLETION_ORDINAL: u64 = u64::MAX;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TerrainBootstrapStage {
    pub stage: HistoricalStageId,
    pub terrain_seed: u64,
    /// The lattice edge the generated terrain is handed to its carrier at. It
    /// belongs to this stage's executable configuration, so it belongs in the
    /// stage's parameter fingerprint: two runs that differ only here are running
    /// different recipes and must not share a plan identity.
    pub chunk_extent: u8,
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
    /// How many sensor apertures each promoted actor is given.
    ///
    /// The stage hands this value to `promote_actor_from_aggregate` itself, so
    /// the equipment the promotion produces is the equipment this stage's
    /// parameter fingerprint covers. A promotion producing differently-equipped
    /// actors is a different stage outcome and must not share a fingerprint.
    pub sensor_count: u8,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ThermalReservoirBootstrap {
    pub id: ThermalReservoirId,
    pub target: causafera_domains::ThermalCellKey,
    pub budget: ThermalEnergy,
    pub schedule: ThermalReservoirSchedule,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ThermalReservoirBootstrapStage {
    pub stage: HistoricalStageId,
    pub reservoirs: Vec<ThermalReservoirBootstrap>,
    /// The lattice edge every thermal field is created at, and the bound the
    /// reservoir cell index is checked against.
    pub chunk_extent: u8,
}

/// The executable configuration of the current production bootstrap, bound to
/// one canonical [`HistoricalBootstrapPlan`].
///
/// The canonical plan in `causafera-world` stays the single contract for stage
/// identity, ordering, targets, and receipt validation. This recipe is the
/// runtime adapter that executes it: one canonical plan, six executable stage
/// adapters, and no second bootstrap model.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeBootstrapRecipe {
    pub physical_geography_init: TerrainBootstrapStage,
    pub material_surface: MaterialSurfaceBootstrapStage,
    pub population_lifecycle: PopulationLifecycleStage,
    pub actor_promotion: ActorPromotionStage,
    pub material_activity: MaterialActivityStage,
    pub thermal_reservoirs: ThermalReservoirBootstrapStage,
    plan: HistoricalBootstrapPlan,
}

/// One executable stage of the canonical plan.
///
/// `stage_seed` is the domain-separated contribution RFC-HIST-001 requires every
/// stage to receive, derived only from the world seed, the plan identity, and the
/// stage's own identity, process and start time. No stage in the current
/// bootstrap has stage-local deterministic variation, so all six ignore it — but
/// it is passed rather than left available on the plan, because a stage that
/// later needs randomness must take it from here instead of deriving a seed of
/// its own.
pub trait HistoricalBootstrapAdapter {
    fn bootstrap(
        &self,
        state: &mut RuntimeState,
        stage_seed: u64,
    ) -> Result<Vec<TraceId>, BootstrapError>;
}

/// The canonical bootstrap record a constructed or imported `RuntimeState` holds.
///
/// `record` is `None` only inside [`RuntimeState::new`], between the state struct
/// being built and the stage coordinator running against it. Every `RuntimeState`
/// handed to a caller carries a validated record; import rejects a snapshot that
/// does not, rather than defaulting an empty receipt list into production state.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BootstrapRuntimeState {
    record: Option<HistoricalBootstrapRecord>,
    stage_results: BTreeMap<HistoricalStageId, StateFingerprint>,
}

impl BootstrapRuntimeState {
    pub const fn record(&self) -> Option<&HistoricalBootstrapRecord> {
        self.record.as_ref()
    }

    pub fn plan(&self) -> Option<&HistoricalBootstrapPlan> {
        self.record.as_ref().map(HistoricalBootstrapRecord::plan)
    }

    pub fn receipts(&self) -> &[HistoricalStageReceipt] {
        self.record
            .as_ref()
            .map_or(&[], HistoricalBootstrapRecord::receipts)
    }

    pub const fn stage_results(&self) -> &BTreeMap<HistoricalStageId, StateFingerprint> {
        &self.stage_results
    }

    pub const fn is_complete(&self) -> bool {
        self.record.is_some()
    }

    /// The bounded read-only projection an observer session may receive.
    ///
    /// Read models are downstream and non-authoritative: this copies typed
    /// values and trace anchors out of the record, exposes no runtime handle and
    /// no authoritative actor or place identity, and renders no process name.
    pub fn observer_summary(&self, config: &RuntimeConfig) -> ObserverBootstrapSummary {
        let Some(record) = self.record.as_ref() else {
            return ObserverBootstrapSummary {
                schema_version: BOOTSTRAP_SUMMARY_SCHEMA_V1,
                complete: false,
                ..ObserverBootstrapSummary::default()
            };
        };
        let plan = record.plan();
        ObserverBootstrapSummary {
            schema_version: BOOTSTRAP_SUMMARY_SCHEMA_V1,
            plan_id: plan.id().raw(),
            world_seed: plan.world_seed(),
            // Counted from what is actually emitted below, not from the plan:
            // the receipt list is capped, so a plan larger than the cap would
            // otherwise declare a stage count its own payload contradicts, and
            // the decoder would reject a message this encoder produced.
            stage_count: plan.stages().len().min(MAX_BOOTSTRAP_RECEIPT_SUMMARIES) as u32,
            complete: true,
            configured_population: config.bootstrap_population,
            configured_promotion_limit: u32::from(config.actor_count),
            receipts: record
                .receipts()
                .iter()
                .take(MAX_BOOTSTRAP_RECEIPT_SUMMARIES)
                .map(|receipt| ObserverBootstrapReceipt {
                    stage: receipt.stage().raw(),
                    completed_at: receipt.completed_at(),
                    result: receipt.result().bytes(),
                    completion_trace: receipt.trace(),
                    dependency_traces: receipt
                        .causes()
                        .iter()
                        .take(MAX_BOOTSTRAP_RECEIPT_DEPENDENCIES)
                        .copied()
                        .collect(),
                })
                .collect(),
        }
    }

    /// The persisted form of this record.
    pub fn export_snapshot(&self) -> BootstrapReceiptSnapshot {
        let Some(record) = self.record.as_ref() else {
            return BootstrapReceiptSnapshot::absent();
        };
        let plan = record.plan();
        BootstrapReceiptSnapshot {
            plan: BootstrapPlanSnapshot {
                id: plan.id(),
                world_seed: plan.world_seed(),
                stages: plan
                    .stages()
                    .iter()
                    .map(|stage| BootstrapStageSnapshot {
                        stage: stage.id(),
                        process: stage.process(),
                        starts_at: stage.starts_at(),
                        ends_at: stage.ends_at(),
                        detail_ordinal: stage.detail_ordinal(),
                        targets: stage.targets().to_vec(),
                        dependencies: stage.dependencies().to_vec(),
                        external_causes: stage.external_causes().to_vec(),
                        parameters: stage.parameters(),
                    })
                    .collect(),
            },
            stage_results: self
                .stage_results
                .iter()
                .map(|(stage, result)| BootstrapStageResultSnapshot {
                    stage: *stage,
                    result: *result,
                })
                .collect(),
            receipts: record
                .receipts()
                .iter()
                .map(|receipt| BootstrapReceiptRecord {
                    stage: receipt.stage(),
                    completed_at: receipt.completed_at(),
                    result: receipt.result(),
                    trace: receipt.trace(),
                    causes: receipt.causes().to_vec(),
                })
                .collect(),
        }
    }

    /// Rebuild a record from persisted parts, through canonical validation.
    ///
    /// The plan is reconstructed with the same constructors production uses, so
    /// a persisted plan with unsorted targets, a forward dependency, an
    /// overlapping span, or a receipt set that does not continue the declared
    /// ancestry fails here rather than becoming authoritative state.
    pub fn import_snapshot(
        snapshot: BootstrapReceiptSnapshot,
    ) -> Result<Self, HistoricalBootstrapError> {
        let stages = snapshot
            .plan
            .stages
            .into_iter()
            .map(|stage| {
                HistoricalStage::new(
                    stage.stage,
                    stage.process,
                    stage.starts_at,
                    stage.ends_at,
                    stage.detail_ordinal,
                    stage.targets,
                    stage.dependencies,
                    stage.external_causes,
                    stage.parameters,
                )
            })
            .collect::<Result<Vec<_>, _>>()?;
        let plan =
            HistoricalBootstrapPlan::new(snapshot.plan.id, snapshot.plan.world_seed, stages)?;
        let receipts = snapshot
            .receipts
            .into_iter()
            .map(|receipt| {
                HistoricalStageReceipt::new(
                    receipt.stage,
                    receipt.completed_at,
                    receipt.result,
                    receipt.trace,
                    receipt.causes,
                )
            })
            .collect::<Result<Vec<_>, _>>()?;
        let mut stage_results = BTreeMap::new();
        for entry in snapshot.stage_results {
            if stage_results.insert(entry.stage, entry.result).is_some() {
                return Err(HistoricalBootstrapError::DuplicateStage(entry.stage));
            }
        }
        Self::from_parts(plan, receipts, stage_results)
    }

    /// Reassemble a validated record from persisted parts.
    ///
    /// Validation runs through the canonical contract, so a plan or receipt set
    /// that does not satisfy it cannot become runtime state.
    pub fn from_parts(
        plan: HistoricalBootstrapPlan,
        receipts: Vec<HistoricalStageReceipt>,
        stage_results: BTreeMap<HistoricalStageId, StateFingerprint>,
    ) -> Result<Self, HistoricalBootstrapError> {
        let record = plan.validate_receipts(receipts)?;
        Ok(Self {
            record: Some(record),
            stage_results,
        })
    }
}

impl RuntimeBootstrapRecipe {
    pub fn from_runtime_config(config: &RuntimeConfig) -> Result<Self, BootstrapError> {
        let terrain_seed = match config.carrier_adapter {
            CarrierAdapterConfig::TerrainSeed { terrain_seed } => terrain_seed,
        };
        let physical_geography_init = TerrainBootstrapStage {
            stage: PHYSICAL_GEOGRAPHY_STAGE,
            terrain_seed,
            chunk_extent: config.chunk_extent,
        };
        let material_surface = MaterialSurfaceBootstrapStage {
            stage: MATERIAL_SURFACE_STAGE,
            initial_condition: 1,
        };
        let population_lifecycle = PopulationLifecycleStage {
            stage: POPULATION_LIFECYCLE_STAGE,
            initial_population: config.bootstrap_population,
        };
        let actor_promotion = ActorPromotionStage {
            stage: ACTOR_PROMOTION_STAGE,
            max_promotions: config.actor_count,
            sensor_count: config.sensor_count,
        };
        let material_activity = MaterialActivityStage {
            stage: MATERIAL_ACTIVITY_STAGE,
            flow_units: 1,
        };
        let thermal_reservoirs = ThermalReservoirBootstrapStage {
            stage: THERMAL_RESERVOIR_STAGE,
            chunk_extent: config.chunk_extent,
            reservoirs: vec![ThermalReservoirBootstrap {
                id: ThermalReservoirId::new(1),
                target: causafera_domains::ThermalCellKey::new(
                    ChartChunkCoord::new(config.chart_id, ChunkCoord::new(0, 0, 0)),
                    0,
                ),
                budget: ThermalEnergy::new(THERMAL_SCALE)
                    .map_err(|_| BootstrapError::InvalidThermalReservoir)?,
                schedule: ThermalReservoirSchedule::PerTick(
                    ThermalEnergy::new(THERMAL_SCALE / 8)
                        .map_err(|_| BootstrapError::InvalidThermalReservoir)?,
                ),
            }],
        };

        let targets = canonical_stage_targets(config)?;
        let parameters = [
            physical_geography_init.parameter_fingerprint(),
            material_surface.parameter_fingerprint(),
            population_lifecycle.parameter_fingerprint(),
            actor_promotion.parameter_fingerprint(),
            material_activity.parameter_fingerprint(),
            thermal_reservoirs.parameter_fingerprint(),
        ];
        let processes = [
            PHYSICAL_GEOGRAPHY_PROCESS_SCHEMA,
            MATERIAL_SURFACE_PROCESS_SCHEMA,
            POPULATION_LIFECYCLE_PROCESS_SCHEMA,
            ACTOR_PROMOTION_PROCESS_SCHEMA,
            MATERIAL_ACTIVITY_PROCESS_SCHEMA,
            THERMAL_RESERVOIR_PROCESS_SCHEMA,
        ];

        let mut stages = Vec::with_capacity(BOOTSTRAP_STAGE_COUNT);
        for ordinal in 0..BOOTSTRAP_STAGE_COUNT {
            let id = HistoricalStageId::new(ordinal as u64 + 1);
            // The canonical stage timeline is a bootstrap ordering timeline, not
            // a request to advance scheduler time: six non-overlapping one-unit
            // intervals give the canonical DAG the strict precedence it
            // validates while every runtime stage effect stays on the existing
            // Lifecycle timestamp convention.
            let starts_at = SimulationTime::new(ordinal as u64);
            let ends_at = SimulationTime::new(ordinal as u64 + 1);
            let dependencies = if ordinal == 0 {
                Vec::new()
            } else {
                vec![HistoricalStageId::new(ordinal as u64)]
            };
            stages.push(HistoricalStage::new(
                id,
                processes[ordinal],
                starts_at,
                ends_at,
                0,
                targets.clone(),
                dependencies,
                Vec::new(),
                parameters[ordinal],
            )?);
        }

        let plan = HistoricalBootstrapPlan::new(
            canonical_plan_id(config.deterministic.world_seed, &stages),
            config.deterministic.world_seed,
            stages,
        )?;

        Ok(Self {
            physical_geography_init,
            material_surface,
            population_lifecycle,
            actor_promotion,
            material_activity,
            thermal_reservoirs,
            plan,
        })
    }

    pub const fn plan(&self) -> &HistoricalBootstrapPlan {
        &self.plan
    }

    /// The domain-separated deterministic seed contribution a stage adapter may
    /// use for stage-local variation. No current stage needs one, so this is the
    /// canonical accessor rather than an unused local derivation.
    pub fn stage_seed(&self, stage: HistoricalStageId) -> Option<u64> {
        self.plan.stage_seed(stage)
    }

    fn adapters(&self) -> [&dyn HistoricalBootstrapAdapter; BOOTSTRAP_STAGE_COUNT] {
        [
            &self.physical_geography_init,
            &self.material_surface,
            &self.population_lifecycle,
            &self.actor_promotion,
            &self.material_activity,
            &self.thermal_reservoirs,
        ]
    }

    /// Run the six stages and close each one with exactly one terminal receipt.
    ///
    /// The completion event is authoritative, not decorative: it transitions the
    /// bounded bootstrap-stage-result state from its absent sentinel to the
    /// stage's result fingerprint, and its causes carry both the previous stage's
    /// receipt and every effect trace this stage committed, so the receipt cannot
    /// hide the detailed ancestry.
    pub(crate) fn execute(
        &self,
        state: &mut RuntimeState,
    ) -> Result<BootstrapRuntimeState, BootstrapError> {
        let adapters = self.adapters();
        let mut receipts = Vec::with_capacity(BOOTSTRAP_STAGE_COUNT);
        let mut stage_results = BTreeMap::new();
        let mut previous_receipt: Option<TraceId> = None;

        for (stage, adapter) in self.plan.stages().iter().zip(adapters) {
            // What a stage committed is read from the trace store rather than
            // taken on the adapter's word. A stage that commits more than it
            // reports — actor promotion commits both an actor transition and an
            // aggregate transition per promotion — would otherwise leave part of
            // its ancestry out of its own receipt.
            let stage_seed = self
                .plan
                .stage_seed(stage.id())
                .ok_or(BootstrapError::StageCompletionsDoNotMatchRecord)?;
            let committed_before = state.traces.len();
            let reported = adapter.bootstrap(state, stage_seed)?;
            let mut effects = state
                .traces
                .iter()
                .skip(committed_before)
                .map(|event| event.trace_id)
                .collect::<Vec<_>>();
            effects.sort_unstable();
            effects.dedup();
            if reported.iter().any(|trace| !effects.contains(trace)) {
                return Err(BootstrapError::MissingStageEffectTrace);
            }

            let result = stage_result_fingerprint(&state.traces, &self.plan, stage, &effects)?;
            let event_causes = expected_completion_causes(&effects, previous_receipt);

            let trace = commit_bootstrap_stage_completion(state, stage.id(), result, event_causes)?;
            let receipt = HistoricalStageReceipt::new(
                stage.id(),
                stage.ends_at(),
                result,
                trace,
                previous_receipt.into_iter().collect(),
            )?;
            stage_results.insert(stage.id(), result);
            receipts.push(receipt);
            previous_receipt = Some(trace);
        }

        Ok(BootstrapRuntimeState::from_parts(
            self.plan.clone(),
            receipts,
            stage_results,
        )?)
    }
}

/// Bootstrap stage targets, as sorted canonical chunk identities.
///
/// This is addressing only. A `ChunkId` derived here names which chunk a stage
/// acted on; it is never a distance, an extent, an ownership claim, or geometry.
fn canonical_stage_targets(config: &RuntimeConfig) -> Result<Vec<ChunkId>, BootstrapError> {
    let active = active_chunk_keys(
        config.chart_id,
        config.active_chunk_radius,
        config.active_chunk_shape,
    );
    if active.is_empty() || active.len() > MAX_STAGE_TARGETS {
        return Err(BootstrapError::InvalidStageTargets);
    }
    let unique = active.iter().copied().collect::<BTreeSet<_>>();
    if unique.len() != active.len() {
        return Err(BootstrapError::InvalidStageTargets);
    }
    let mut targets = unique
        .into_iter()
        .map(bootstrap_chunk_target)
        .collect::<Vec<_>>();
    targets.sort_unstable();
    let distinct = targets.len();
    targets.dedup();
    if targets.len() != distinct {
        return Err(BootstrapError::InvalidStageTargets);
    }
    Ok(targets)
}

/// A chunk's canonical bootstrap-target identity.
///
/// Domain-separated from every other derivation that hashes a chunk, so a target
/// identity can never be mistaken for a mana or population object identity.
pub fn bootstrap_chunk_target(chunk: ChartChunkCoord) -> ChunkId {
    ChunkId::new(mix64(BOOTSTRAP_TARGET_DOMAIN ^ chart_chunk_hash(chunk)))
}

/// The plan's opaque content-addressed identity.
///
/// Two runs with the same seed and recipe produce the same plan ID; changing any
/// stage parameter, target, or dependency changes it. It is an equality identity,
/// never a distance or an ordering.
fn canonical_plan_id(world_seed: u64, stages: &[HistoricalStage]) -> HistoricalBootstrapId {
    let mut digest = CanonicalDigest::new();
    digest.write(BOOTSTRAP_PLAN_ID_DOMAIN);
    digest.write(world_seed);
    digest.write(stages.len() as u64);
    for stage in stages {
        digest.write(stage.id().raw());
        digest.write(stage.process().raw());
        digest.write(stage.starts_at().raw());
        digest.write(stage.ends_at().raw());
        digest.write(u64::from(stage.detail_ordinal()));
        digest.write(stage.targets().len() as u64);
        for target in stage.targets() {
            digest.write(target.raw());
        }
        digest.write(stage.dependencies().len() as u64);
        for dependency in stage.dependencies() {
            digest.write(dependency.raw());
        }
        digest.write(stage.external_causes().len() as u64);
        for cause in stage.external_causes() {
            digest.write(cause.raw());
        }
        digest.write_bytes(stage.parameters().bytes());
    }
    let bytes = digest.finish().bytes();
    HistoricalBootstrapId::new(u64::from_le_bytes(
        bytes[..8].try_into().expect("digest yields 32 bytes"),
    ))
}

fn parameter_digest(
    stage: HistoricalStageId,
    process: HistoricalProcessSchemaId,
) -> CanonicalDigest {
    let mut digest = CanonicalDigest::new();
    digest.write(BOOTSTRAP_PARAMETER_DOMAIN);
    digest.write(stage.raw());
    digest.write(process.raw());
    digest
}

impl TerrainBootstrapStage {
    fn parameter_fingerprint(&self) -> StateFingerprint {
        let mut digest = parameter_digest(self.stage, PHYSICAL_GEOGRAPHY_PROCESS_SCHEMA);
        digest.write(self.terrain_seed);
        digest.write(u64::from(self.chunk_extent));
        digest.finish()
    }
}

impl MaterialSurfaceBootstrapStage {
    fn parameter_fingerprint(&self) -> StateFingerprint {
        let mut digest = parameter_digest(self.stage, MATERIAL_SURFACE_PROCESS_SCHEMA);
        digest.write(self.initial_condition as u64);
        digest.finish()
    }
}

impl PopulationLifecycleStage {
    fn parameter_fingerprint(&self) -> StateFingerprint {
        let mut digest = parameter_digest(self.stage, POPULATION_LIFECYCLE_PROCESS_SCHEMA);
        digest.write(self.initial_population);
        digest.finish()
    }
}

impl ActorPromotionStage {
    fn parameter_fingerprint(&self) -> StateFingerprint {
        let mut digest = parameter_digest(self.stage, ACTOR_PROMOTION_PROCESS_SCHEMA);
        digest.write(u64::from(self.max_promotions));
        digest.write(u64::from(self.sensor_count));
        digest.finish()
    }
}

impl MaterialActivityStage {
    fn parameter_fingerprint(&self) -> StateFingerprint {
        let mut digest = parameter_digest(self.stage, MATERIAL_ACTIVITY_PROCESS_SCHEMA);
        digest.write(self.flow_units as u64);
        digest.finish()
    }
}

impl ThermalReservoirBootstrapStage {
    fn parameter_fingerprint(&self) -> StateFingerprint {
        let mut digest = parameter_digest(self.stage, THERMAL_RESERVOIR_PROCESS_SCHEMA);
        digest.write(u64::from(self.chunk_extent));
        let mut reservoirs = self.reservoirs.clone();
        reservoirs.sort_unstable_by_key(|reservoir| reservoir.id);
        digest.write(reservoirs.len() as u64);
        for reservoir in &reservoirs {
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
        }
        digest.finish()
    }
}

/// The canonical projection of what one stage committed.
///
/// Built from the stage's own committed effects in sorted trace order, never
/// from digest-byte distance and never from a semantic label. An empty stage
/// still yields a stage-specific fingerprint, so its completion is a real
/// transition rather than a repeated sentinel.
fn stage_result_fingerprint(
    traces: &CausalTraceStore,
    plan: &HistoricalBootstrapPlan,
    stage: &HistoricalStage,
    effects: &[TraceId],
) -> Result<StateFingerprint, BootstrapError> {
    let mut digest = CanonicalDigest::new();
    digest.write(BOOTSTRAP_RESULT_DOMAIN);
    digest.write(plan.id().raw());
    digest.write(plan.world_seed());
    digest.write(stage.id().raw());
    digest.write(stage.process().raw());
    digest.write_bytes(stage.parameters().bytes());
    digest.write(effects.len() as u64);
    for trace in effects {
        let event = traces
            .event(*trace)
            .ok_or(BootstrapError::MissingStageEffectTrace)?;
        digest.write(event.kind.raw());
        digest.write(event.effects.len() as u64);
        for effect in event.effects {
            digest.write(effect.target().object_kind().raw());
            digest.write(effect.target().object_id());
            digest.write(effect.target().property().raw());
            digest.write_bytes(effect.before().bytes());
            digest.write_bytes(effect.after().bytes());
        }
    }
    Ok(digest.finish())
}

/// Re-derive, from the trace store alone, which events each stage committed.
///
/// The store is append-only and bootstrap runs entirely before the first tick,
/// so the completions partition its prefix: the runtime root, stage one's
/// effects, completion one, stage two's effects, completion two, and so on. That
/// is exactly the window the coordinator measured when it built the record, so
/// replaying it here reproduces the same inputs rather than trusting the
/// record's account of them.
///
/// Every stage-completion event in the store must be one of the record's
/// receipts, in receipt order. A seventh completion, or a completion the record
/// does not name, means the store does not describe the run the record claims.
pub(crate) fn replay_stage_effects(
    traces: &CausalTraceStore,
    receipts: &[HistoricalStageReceipt],
) -> Result<Vec<Vec<TraceId>>, BootstrapError> {
    let committed = traces
        .iter()
        .enumerate()
        .filter(|(_, event)| event.kind.raw() == BOOTSTRAP_STAGE_COMPLETION_EVENT_KIND)
        .map(|(index, event)| (index, event.trace_id))
        .collect::<Vec<_>>();
    if committed.len() != receipts.len()
        || committed
            .iter()
            .zip(receipts)
            .any(|((_, trace), receipt)| *trace != receipt.trace())
    {
        return Err(BootstrapError::StageCompletionsDoNotMatchRecord);
    }

    let ordered = traces
        .iter()
        .map(|event| event.trace_id)
        .collect::<Vec<_>>();
    let mut windows = Vec::with_capacity(receipts.len());
    // The runtime root precedes stage one and is not one of its effects.
    let mut cursor = 1_usize;
    for (index, _) in committed {
        if index < cursor {
            return Err(BootstrapError::StageCompletionsDoNotMatchRecord);
        }
        windows.push(ordered[cursor..index].to_vec());
        cursor = index + 1;
    }
    Ok(windows)
}

/// Recompute one stage's result from what the store says it committed.
///
/// Used by import, where the persisted result, the persisted completion effect,
/// and the persisted stage-result state can all have been rewritten together. A
/// coherent forgery of the three still has to survive this, and surviving it
/// means forging every stage effect's committed payload as well — at which point
/// the snapshot is a different history rather than a false account of this one.
pub(crate) fn recompute_stage_result(
    traces: &CausalTraceStore,
    plan: &HistoricalBootstrapPlan,
    stage: &HistoricalStage,
    effects: &[TraceId],
) -> Result<StateFingerprint, BootstrapError> {
    stage_result_fingerprint(traces, plan, stage, effects)
}

/// The causes a stage's completion event must carry, exactly.
///
/// Both halves matter. The previous receipt is what chains the record; the stage
/// effects are what stops a receipt summarizing a stage whose detailed ancestry
/// has been cut out from under it.
pub(crate) fn expected_completion_causes(
    effects: &[TraceId],
    previous_completion: Option<TraceId>,
) -> Vec<TraceId> {
    let mut causes = effects.to_vec();
    if let Some(previous) = previous_completion {
        causes.push(previous);
    }
    causes.sort_unstable();
    causes.dedup();
    causes
}

/// The transition a reservoir's bootstrap event commits.
///
/// Import recomputes this from the imported reservoir and compares it against
/// the committed effect: the budget and target are otherwise believed on the
/// snapshot's word, and a reservoir that injects a forged budget into a cell the
/// run never targeted would be accepted.
pub(crate) fn thermal_reservoir_bootstrap_after(
    budget: i64,
    target: causafera_domains::ThermalCellKey,
) -> StateFingerprint {
    fingerprint_u64(
        0x1411,
        budget as u64 ^ cell_object_id(target.chunk, target.cell_index),
    )
}

/// The absent value a stage's bounded result state holds before completion.
pub(crate) fn bootstrap_stage_absent_result(stage: HistoricalStageId) -> StateFingerprint {
    fingerprint_u64(BOOTSTRAP_STAGE_RESULT_ABSENT_TAG, stage.raw())
}

fn commit_bootstrap_stage_completion(
    state: &mut RuntimeState,
    stage: HistoricalStageId,
    result: StateFingerprint,
    causes: Vec<TraceId>,
) -> Result<TraceId, RuntimeError> {
    let event = CausalEventProposal::new(
        EventProposalKey::new(
            BOOTSTRAP_SYSTEM_ID,
            stage.raw(),
            BOOTSTRAP_STAGE_COMPLETION_ORDINAL,
        ),
        EventKindId::new(BOOTSTRAP_STAGE_COMPLETION_EVENT_KIND),
        causes,
        vec![CausalEffect::new(
            CausalTarget::new(
                StateObjectKindId::new(BOOTSTRAP_STAGE_OBJECT_KIND),
                stage.raw(),
                StatePropertyId::new(BOOTSTRAP_STAGE_RESULT_PROPERTY),
            ),
            bootstrap_stage_absent_result(stage),
            result,
        )?],
    )?;
    let trace = state
        .traces
        .commit_batch(SimulationTime::new(0), Phase::Lifecycle, vec![event])?[0];
    state.latest_physical_trace = trace;
    Ok(trace)
}

impl HistoricalBootstrapAdapter for TerrainBootstrapStage {
    fn bootstrap(
        &self,
        state: &mut RuntimeState,
        _stage_seed: u64,
    ) -> Result<Vec<TraceId>, BootstrapError> {
        let chunks = state.carrier_adapters.keys().copied().collect::<Vec<_>>();
        let mut traces = Vec::with_capacity(chunks.len());
        // Every chunk's terrain is generated and its trace committed first, so
        // the second pass below can give each adapter the real ground its
        // neighbours hold rather than an empty map (`TODO-GEO-006`).
        let mut generated = BTreeMap::new();
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
                // The event fingerprint must differ from the `0` sentinel above even
                // when `terrain_seed` is exactly 0 (the default observer session's
                // seed), or `CausalEffect::new` rejects the bootstrap event as an
                // unchanged state. `chart_chunk_hash` is safe to use here — it is
                // event-identity bookkeeping, not the value handed to generation, so
                // it cannot reintroduce the per-chunk seam `terrain_cells` no longer
                // has.
                fingerprint_u64(0x0B01, self.terrain_seed ^ chart_chunk_hash(chunk)),
            )?;
            let terrain = deterministic_terrain_chunk(self.terrain_seed, chunk, trace);
            generated.insert(chunk, terrain);
            traces.push(trace);
        }
        let field_extent = self.chunk_extent;
        for (chunk, terrain) in &generated {
            state.carrier_adapters.insert(
                *chunk,
                TerrainCarrierAdapter::new(*chunk, terrain.clone(), field_extent, &generated),
            );
        }
        Ok(traces)
    }
}

impl HistoricalBootstrapAdapter for MaterialSurfaceBootstrapStage {
    fn bootstrap(
        &self,
        state: &mut RuntimeState,
        _stage_seed: u64,
    ) -> Result<Vec<TraceId>, BootstrapError> {
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
                    thermal: MaterialSurfaceThermalState {
                        retained_energy: ThermalEnergy::ZERO,
                        last_exchange: None,
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
    fn bootstrap(
        &self,
        state: &mut RuntimeState,
        _stage_seed: u64,
    ) -> Result<Vec<TraceId>, BootstrapError> {
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
    fn bootstrap(
        &self,
        state: &mut RuntimeState,
        _stage_seed: u64,
    ) -> Result<Vec<TraceId>, BootstrapError> {
        let mut traces = Vec::new();
        for _ in 0..self.max_promotions {
            let before = state.actor_promotions;
            let chunk = first_population_chunk(state)?;
            promote_actor_from_aggregate(state, SimulationTime::new(0), chunk, self.sensor_count)?;
            if state.actor_promotions > before {
                traces.push(state.latest_physical_trace);
            }
        }
        Ok(traces)
    }
}

impl HistoricalBootstrapAdapter for MaterialActivityStage {
    fn bootstrap(
        &self,
        state: &mut RuntimeState,
        _stage_seed: u64,
    ) -> Result<Vec<TraceId>, BootstrapError> {
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

impl HistoricalBootstrapAdapter for ThermalReservoirBootstrapStage {
    fn bootstrap(
        &self,
        state: &mut RuntimeState,
        _stage_seed: u64,
    ) -> Result<Vec<TraceId>, BootstrapError> {
        let chunks = state.active_chunks.keys().copied().collect::<Vec<_>>();
        let mut traces = Vec::with_capacity(chunks.len() + self.reservoirs.len());
        let mut fields = Vec::with_capacity(chunks.len());
        for (ordinal, chunk) in chunks.iter().copied().enumerate() {
            let trace = commit_bootstrap_stage_event(
                state,
                self.stage,
                chunk,
                ordinal as u64,
                THERMAL_FIELD_BOOTSTRAP_EVENT_KIND,
                THERMAL_CARRIER_OBJECT_KIND,
                THERMAL_ENERGY_PROPERTY,
                fingerprint_u64(0x1410, 0),
                fingerprint_u64(0x1410, chart_chunk_hash(chunk)),
            )?;
            fields.push(
                ThermalField::new(chunk, self.chunk_extent, trace).map_err(RuntimeError::from)?,
            );
            traces.push(trace);
        }
        let conservation_last_change = traces
            .last()
            .copied()
            .ok_or(BootstrapError::InvalidThermalReservoir)?;
        state.thermal_fields =
            ThermalFieldSet::new(fields, conservation_last_change).map_err(RuntimeError::from)?;

        let mut reservoirs = self.reservoirs.clone();
        reservoirs.sort_unstable_by_key(|reservoir| reservoir.id);
        for (ordinal, reservoir) in reservoirs.into_iter().enumerate() {
            if !state.active_chunks.contains_key(&reservoir.target.chunk) {
                return Err(BootstrapError::ThermalReservoirOutsideActiveRegion {
                    target_chunk: reservoir.target.chunk,
                });
            }
            if usize::from(reservoir.target.cell_index) >= usize::from(self.chunk_extent).pow(3) {
                return Err(BootstrapError::InvalidThermalReservoir);
            }
            if state.thermal_reservoirs.contains_key(&reservoir.id) {
                return Err(BootstrapError::DuplicateThermalReservoir { id: reservoir.id });
            }
            let trace = commit_thermal_reservoir_bootstrap_event(
                state,
                self.stage,
                ordinal as u64,
                reservoir,
            )?;
            state.thermal_reservoirs.insert(
                reservoir.id,
                ThermalReservoir {
                    id: reservoir.id,
                    target: reservoir.target,
                    budget: reservoir.budget,
                    schedule: reservoir.schedule,
                    bootstrap_trace: trace,
                    last_change: trace,
                },
            );
            traces.push(trace);
        }
        Ok(traces)
    }
}

#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum BootstrapError {
    #[error(transparent)]
    Runtime(#[from] RuntimeError),
    #[error(transparent)]
    Historical(#[from] HistoricalBootstrapError),
    #[error("thermal reservoir target lies outside the active region: {target_chunk:?}")]
    ThermalReservoirOutsideActiveRegion { target_chunk: ChartChunkCoord },
    #[error("thermal reservoir bootstrap configuration is invalid")]
    InvalidThermalReservoir,
    #[error("thermal reservoir ID is duplicated: {id:?}")]
    DuplicateThermalReservoir { id: ThermalReservoirId },
    #[error("bootstrap stage targets are empty, duplicated, or out of bounds")]
    InvalidStageTargets,
    #[error("bootstrap stage effect trace is absent from the causal trace store")]
    MissingStageEffectTrace,
    #[error("committed stage completions do not match the bootstrap record")]
    StageCompletionsDoNotMatchRecord,
}

fn commit_thermal_reservoir_bootstrap_event(
    state: &mut RuntimeState,
    stage: HistoricalStageId,
    ordinal: u64,
    reservoir: ThermalReservoirBootstrap,
) -> Result<TraceId, RuntimeError> {
    let event = CausalEventProposal::new(
        EventProposalKey::new(BOOTSTRAP_SYSTEM_ID, stage.raw(), ordinal),
        EventKindId::new(THERMAL_RESERVOIR_BOOTSTRAP_EVENT_KIND),
        vec![state.latest_physical_trace],
        vec![CausalEffect::new(
            CausalTarget::new(
                StateObjectKindId::new(THERMAL_RESERVOIR_OBJECT_KIND),
                reservoir.id.raw(),
                StatePropertyId::new(THERMAL_RESERVOIR_BUDGET_PROPERTY),
            ),
            fingerprint_u64(0x1411, 0),
            thermal_reservoir_bootstrap_after(reservoir.budget.get(), reservoir.target),
        )?],
    )?;
    let trace = state
        .traces
        .commit_batch(SimulationTime::new(0), Phase::Lifecycle, vec![event])?[0];
    state.latest_physical_trace = trace;
    Ok(trace)
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
