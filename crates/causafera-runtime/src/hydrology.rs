//! The runtime's hydrology state and the Physics system that advances it.
//!
//! Geography owns what water is, the domain owns how it moves, and this module
//! owns the authoritative commit: it hands the solver one frozen state, turns the
//! resulting logical DAG into a causal batch, and installs the after-state only
//! once every event has committed.
//!
//! See `plans/hydrology.md` §5 and §10.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use causafera_core::provenance::{CausalDagBatchLimits, StateFingerprint};
use causafera_core::{Phase, RandomStream, System};
use causafera_domains::{
    HydrologyBucket, HydrologyEvolutionLimits, HydrologyEvolutionModel, HydrologyEvolutionProposal,
    HydrologyEvolutionRequest, HydrologyResolutionPolicy, HydrologyTransferReceipt,
};
use causafera_geography::{
    HydrologyActiveRegion, HydrologyBoundaryMap, HydrologyCellKey, HydrologyConveyanceGraph,
    HydrologyFieldSet, HydrologyForcingRecord, HydrologyGridMetrics, HydrologyResolutionState,
    MAX_HYDROLOGY_PERSISTED_TRANSFER_RECEIPTS, MAX_HYDROLOGY_STORED_RECEIPT_BATCHES, TerrainChunk,
};
use causafera_types::{ChartChunkCoord, SimulationTime, TraceId};

use crate::hydrology_events::{
    HydrologyBatchInputs, HydrologyObjectRegistry, build_hydrology_batch,
};
use crate::{RuntimeError, RuntimeState};

/// Everything the runtime holds for hydrology.
///
/// Grouped rather than spread across `RuntimeState`: the plan is explicit that
/// modules stay cohesive and that unrelated state does not accumulate in
/// `runtime.rs`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HydrologyRuntimeState {
    pub enabled: bool,
    pub fields: HydrologyFieldSet,
    pub conveyance: HydrologyConveyanceGraph,
    pub boundaries: HydrologyBoundaryMap,
    pub metrics: HydrologyGridMetrics,
    pub active: HydrologyActiveRegion,
    pub resolution: BTreeMap<ChartChunkCoord, HydrologyResolutionState>,
    pub resolution_policy: HydrologyResolutionPolicy,
    /// The whole schedule, including records already applied. An applied record
    /// stays persisted so its origin identity and allocation inputs survive typed
    /// receipt eviction.
    pub forcing: Vec<HydrologyForcingRecord>,
    pub registry: HydrologyObjectRegistry,
    /// The persisted counter every synthetic aggregation node draws from. Shared
    /// across coarse-input trees, coarse process events, and the terminal tree, in
    /// that order, so import can reproduce every identifier.
    pub next_node_id: u64,
    /// The latest retained typed batches, keyed by their conservation trace.
    pub receipts: BTreeMap<TraceId, Vec<HydrologyTransferReceipt>>,
    pub conservation_receipts: BTreeMap<TraceId, causafera_domains::HydrologyConservationReceipt>,
    /// Retained conservation traces in tick order, which is the eviction order.
    pub retained_batches: Vec<TraceId>,
}

impl HydrologyRuntimeState {
    /// The state a session that did not ask for water holds.
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            fields: HydrologyFieldSet::default(),
            conveyance: HydrologyConveyanceGraph::default(),
            boundaries: HydrologyBoundaryMap::default(),
            metrics: HydrologyGridMetrics::default(),
            active: HydrologyActiveRegion::default(),
            resolution: BTreeMap::new(),
            resolution_policy: HydrologyResolutionPolicy::DISABLED,
            forcing: Vec::new(),
            registry: HydrologyObjectRegistry::default(),
            // One, not zero: object ID zero belongs to the batch-sequence object
            // the conservation event settles.
            next_node_id: 1,
            receipts: BTreeMap::new(),
            conservation_receipts: BTreeMap::new(),
            retained_batches: Vec::new(),
        }
    }

    /// Retain the newest batch and evict whole older ones until both bounds hold.
    ///
    /// Whole batches, never partial ones: half a tick's receipts would answer a
    /// question about that tick with an incomplete ledger, which is worse than
    /// answering that the detail is gone. Causal ancestry is unaffected — the
    /// trace store keeps it, which is why eviction is safe at all.
    fn retain(&mut self, trace: TraceId) {
        self.retained_batches.push(trace);
        loop {
            let batches = self.retained_batches.len();
            let receipts: usize = self
                .retained_batches
                .iter()
                .map(|trace| self.receipts.get(trace).map_or(0, Vec::len))
                .sum();
            if batches <= MAX_HYDROLOGY_STORED_RECEIPT_BATCHES
                && receipts <= MAX_HYDROLOGY_PERSISTED_TRANSFER_RECEIPTS
            {
                break;
            }
            if self.retained_batches.len() <= 1 {
                break;
            }
            let evicted = self.retained_batches.remove(0);
            self.receipts.remove(&evicted);
            self.conservation_receipts.remove(&evicted);
        }
    }
}

impl Default for HydrologyRuntimeState {
    fn default() -> Self {
        Self::disabled()
    }
}

/// The Physics system that advances hydrology.
///
/// Registered after every existing system in `Runtime::new`. Appending is the
/// whole point: `StreamKey { world_seed, time, phase, system_id }` seeds each
/// system's `RandomStream`, so inserting anywhere but last would renumber every
/// later system and silently reseed it (plan risk R7).
pub(crate) struct HydrologyEvolutionSystem {
    state: Arc<Mutex<RuntimeState>>,
    next_time: SimulationTime,
}

impl HydrologyEvolutionSystem {
    pub(crate) fn new(state: Arc<Mutex<RuntimeState>>) -> Self {
        Self {
            state,
            next_time: SimulationTime::new(1),
        }
    }

    fn execute(&mut self) -> Result<(), RuntimeError> {
        let mut state = self.state.lock().map_err(|_| RuntimeError::StatePoisoned)?;
        if state.failure.is_some() || !state.hydrology.enabled {
            self.next_time = self.next_time.tick();
            return Ok(());
        }
        let tick = self.next_time.raw();
        let terrain = hydrology_terrain(&state);

        // Only records scheduled for this tick and not yet applied. A record is
        // handed to the solver once; the schedule keeps it afterwards so its
        // origin and its allocation inputs stay inspectable.
        let due: Vec<HydrologyForcingRecord> = state
            .hydrology
            .forcing
            .iter()
            .filter(|record| record.scheduled_tick() == tick && record.applied_at().is_none())
            .cloned()
            .collect();

        let proposal = {
            let hydrology = &state.hydrology;
            HydrologyEvolutionModel::propose(
                &hydrology.fields,
                HydrologyEvolutionRequest {
                    tick,
                    metrics: &hydrology.metrics,
                    terrain: &terrain,
                    active: &hydrology.active,
                    conveyance: &hydrology.conveyance,
                    boundaries: &hydrology.boundaries,
                    forcing: &due,
                    resolution: &hydrology.resolution,
                    resolution_policy: hydrology.resolution_policy,
                    previous_conservation: hydrology.fields.conservation_last_change(),
                    limits: HydrologyEvolutionLimits::default(),
                },
            )?
        };

        self.commit(&mut state, &proposal)?;
        self.next_time = self.next_time.tick();
        Ok(())
    }

    /// Commit the whole batch, then install the after-state.
    ///
    /// In that order and never the reverse: `commit_dag_batch` either appends the
    /// complete batch or leaves the store byte-identical, so a refusal here leaves
    /// the world exactly as the tick found it.
    fn commit(
        &self,
        state: &mut RuntimeState,
        proposal: &HydrologyEvolutionProposal,
    ) -> Result<(), RuntimeError> {
        let origins: Vec<TraceId> = state
            .hydrology
            .forcing
            .iter()
            .filter(|record| {
                proposal
                    .applied_forcing()
                    .contains(&(record.scheduled_tick(), record.forcing_id()))
            })
            .map(HydrologyForcingRecord::origin_trace)
            .collect();

        let batch = build_hydrology_batch(HydrologyBatchInputs {
            registry: &state.hydrology.registry,
            events: proposal.events(),
            terminal_leaves: proposal.terminal_leaves(),
            coarse_processes: proposal.coarse_processes(),
            conservation: proposal.conservation(),
            batch_sequence: proposal.batch_sequence(),
            previous_conservation: state.hydrology.fields.conservation_last_change(),
            forcing_origins: origins,
            next_node_id: state.hydrology.next_node_id,
        })?;

        let limits = CausalDagBatchLimits {
            max_causes_per_event: causafera_geography::MAX_HYDROLOGY_CAUSES_PER_EVENT,
            max_effects_per_event: causafera_geography::MAX_HYDROLOGY_EFFECTS_PER_EVENT,
        };
        let traces = state.traces.commit_dag_batch(
            self.next_time,
            Phase::Physics,
            batch.proposals.clone(),
            limits,
        )?;
        let conservation_trace = *traces
            .get(&batch.conservation_key)
            .ok_or(RuntimeError::HydrologyConservationNotCommitted)?;

        // Every anchor the tick moved, resolved from the committed batch. A
        // bucket whose settlement event is missing from the map would mean the
        // store committed a different batch than the one that was built.
        let mut after_fields = proposal.after_state().clone();
        for change in proposal.cell_changes() {
            let trace = *traces
                .get(&change.settlement_event)
                .ok_or(RuntimeError::HydrologyAnchorNotCommitted)?;
            // Named per bucket rather than dispatched on a tag: only three of the
            // receipt buckets are cell storage, and the match says which.
            match change.bucket {
                HydrologyBucket::Surface => {
                    after_fields.install_surface_trace(change.cell, trace)?;
                }
                HydrologyBucket::Soil => after_fields.install_soil_trace(change.cell, trace)?,
                HydrologyBucket::Groundwater => {
                    after_fields.install_groundwater_trace(change.cell, trace)?;
                }
                HydrologyBucket::ForcingInput => {
                    after_fields.install_forcing_trace(change.cell, trace)?;
                }
                other => return Err(RuntimeError::HydrologyBucketNotCellStorage(other.tag())),
            }
        }
        for settlement in proposal.forcing_settlements() {
            let trace = *traces
                .get(&settlement.settlement_event)
                .ok_or(RuntimeError::HydrologyAnchorNotCommitted)?;
            after_fields.install_forcing_trace(settlement.cell, trace)?;
        }
        after_fields.install_conservation_trace(conservation_trace);

        let mut after_conveyance = proposal.after_conveyance().clone();
        for change in proposal.edge_changes() {
            let trace = *traces
                .get(&change.settlement_event)
                .ok_or(RuntimeError::HydrologyAnchorNotCommitted)?;
            after_conveyance.install_edge_trace(change.edge, trace)?;
        }

        for (scheduled_tick, forcing_id) in proposal.applied_forcing() {
            let record = state
                .hydrology
                .forcing
                .iter_mut()
                .find(|record| {
                    record.scheduled_tick() == *scheduled_tick && record.forcing_id() == *forcing_id
                })
                .ok_or(RuntimeError::HydrologyForcingRecordUnknown)?;
            record.mark_applied(*scheduled_tick)?;
        }

        state.hydrology.fields = after_fields;
        state.hydrology.conveyance = after_conveyance;
        state.hydrology.next_node_id = batch.next_node_id;
        state
            .hydrology
            .receipts
            .insert(conservation_trace, proposal.transfer_receipts().to_vec());
        state
            .hydrology
            .conservation_receipts
            .insert(conservation_trace, *proposal.conservation());
        state.hydrology.retain(conservation_trace);
        Ok(())
    }
}

impl System for HydrologyEvolutionSystem {
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

/// The terrain every resident hydrology chunk sits on.
///
/// Generated from the same deterministic function the terrain carrier uses, so
/// hydrology reads the ground the rest of the engine reads rather than a second
/// copy that could drift from it.
fn hydrology_terrain(state: &RuntimeState) -> BTreeMap<ChartChunkCoord, TerrainChunk> {
    state
        .hydrology
        .fields
        .fields()
        .keys()
        .map(|chunk| {
            (
                *chunk,
                crate::deterministic_terrain_chunk(
                    state.config.deterministic.world_seed,
                    *chunk,
                    state.hydrology.fields.conservation_last_change(),
                ),
            )
        })
        .collect()
}

/// Every cell the registry has to address, in canonical order.
pub(crate) fn registry_cells(fields: &HydrologyFieldSet) -> Vec<HydrologyCellKey> {
    let mut cells = Vec::with_capacity(fields.cell_count());
    for (chunk, field) in fields.fields() {
        for ordinal in 0..field.cells().len() {
            if let Ok(cell) = HydrologyCellKey::new(*chunk, ordinal as u16) {
                cells.push(cell);
            }
        }
    }
    cells
}

// ---------------------------------------------------------------------------
// Causal initialization from configuration
// ---------------------------------------------------------------------------

/// Build hydrology state from the configured numbers and the terrain under it.
///
/// Purely numeric. Nothing here names a basin, a river, a supply, or a rainfall
/// event: what a session configures are measured quantities, and what this
/// produces is the per-cell substrate those quantities and the real ground imply.
/// Surface roughness, cell area, edge length, and the timestep are all causal
/// inputs rather than recorded metadata (§4).
pub(crate) fn build_hydrology_state(
    config: &crate::HydrologyConfig,
    world_seed: u64,
    active_chunks: &[ChartChunkCoord],
    bootstrap_trace: TraceId,
) -> Result<HydrologyRuntimeState, RuntimeError> {
    use causafera_geography::{
        FaceDirection, HydraulicFraction, HydraulicSubstrateCell, HydraulicSubstrateParts,
        HydrologyCellState, HydrologyCellStorage, HydrologyConveyanceEdge, HydrologyEdgeKey,
        HydrologyExteriorFaceKey, HydrologyField, SURFACE_CELL_COUNT,
    };
    use causafera_types::WaterVolume;

    let mut state = HydrologyRuntimeState::disabled();
    if !config.enabled {
        return Ok(state);
    }
    let parameters = config
        .bootstrap_parameters
        .as_ref()
        .ok_or(RuntimeError::HydrologyEnabledWithoutParameters)?;

    let resident: Vec<ChartChunkCoord> = {
        let mut chunks: Vec<ChartChunkCoord> = active_chunks.to_vec();
        chunks.sort_unstable();
        chunks.dedup();
        chunks
    };
    let metrics = HydrologyGridMetrics::new(
        config
            .grid_metrics
            .iter()
            .map(|(chart, metric)| (*chart, *metric))
            .collect(),
    )?;

    let terrain: BTreeMap<ChartChunkCoord, TerrainChunk> = resident
        .iter()
        .map(|chunk| {
            (
                *chunk,
                crate::deterministic_terrain_chunk(world_seed, *chunk, bootstrap_trace),
            )
        })
        .collect();

    let mut fields = Vec::with_capacity(resident.len());
    for chunk in &resident {
        let metric = metrics.get(chunk.chart)?;
        let ground = &terrain[chunk];
        let mut cells = Vec::with_capacity(SURFACE_CELL_COUNT);
        let mut substrate = Vec::with_capacity(SURFACE_CELL_COUNT);
        for ordinal in 0..SURFACE_CELL_COUNT {
            let cell = HydrologyCellKey::new(*chunk, ordinal as u16)?;
            let resolved = resolve_override(parameters, chunk.chart, cell);
            let elevation = i64::from(ground.elevations()[ordinal].millimetres());
            let roughness = u64::from(ground.roughness()[ordinal].millimetres());

            // Per-second configuration to per-tick solver coefficients. Floor
            // division throughout: a rate that does not divide evenly becomes a
            // smaller per-tick coefficient, never a larger one.
            let infiltration_limit = floor_div(
                checked_mul3(
                    resolved.infiltration_rate,
                    metric.cell_area_mm2().get(),
                    metric.timestep_millis().get(),
                )?,
                1_000,
            )?;
            let adjusted_surface = floor_div(
                checked_mul(
                    resolved.surface_transmissivity,
                    resolved.roughness_reference,
                )?,
                u128::from(
                    resolved
                        .roughness_reference
                        .checked_add(roughness)
                        .ok_or(RuntimeError::HydrologyCoefficientOverflow)?,
                ),
            )?;
            let surface_conductance = floor_div(
                checked_mul(adjusted_surface, metric.timestep_millis().get())?,
                checked_mul(1_000, metric.orthogonal_edge_length_mm().get())?,
            )?;
            // Groundwater transmissivity is not roughness-adjusted: roughness is a
            // property of the surface water flows over, not of the aquifer.
            let groundwater_conductance = floor_div(
                checked_mul(
                    resolved.groundwater_transmissivity,
                    metric.timestep_millis().get(),
                )?,
                checked_mul(1_000, metric.orthogonal_edge_length_mm().get())?,
            )?;

            substrate.push(HydraulicSubstrateCell::new(HydraulicSubstrateParts {
                surface_capacity: WaterVolume::new(resolved.surface_capacity),
                soil_capacity: WaterVolume::new(resolved.soil_capacity),
                groundwater_capacity: WaterVolume::new(resolved.groundwater_capacity),
                infiltration_limit_per_tick: WaterVolume::new(infiltration_limit),
                percolation_fraction: HydraulicFraction::from_parts(
                    resolved.percolation.0,
                    resolved.percolation.1,
                )?,
                specific_yield: HydraulicFraction::from_parts(
                    resolved.specific_yield.0,
                    resolved.specific_yield.1,
                )?,
                aquifer_base_elevation_mm: elevation
                    .checked_add(resolved.aquifer_base_offset)
                    .ok_or(RuntimeError::HydrologyCoefficientOverflow)?,
                baseflow_threshold: WaterVolume::new(resolved.baseflow_threshold),
                baseflow_fraction: HydraulicFraction::from_parts(
                    resolved.baseflow.0,
                    resolved.baseflow.1,
                )?,
                surface_conductance_mm2_per_tick: surface_conductance,
                groundwater_conductance_mm2_per_tick: groundwater_conductance,
            })?);
            cells.push(HydrologyCellState::initial(
                HydrologyCellStorage::new(
                    WaterVolume::new(resolved.initial_surface),
                    WaterVolume::new(resolved.initial_soil),
                    WaterVolume::new(resolved.initial_groundwater),
                ),
                bootstrap_trace,
                causafera_domains::absent_fingerprint(
                    &causafera_geography::HydrologyCarrierKey::Cell(cell),
                    causafera_domains::HydrologyProperty::ForcingInput,
                ),
            ));
        }
        fields.push(HydrologyField::from_parts(*chunk, cells, substrate)?);
    }
    let fields = HydrologyFieldSet::new(fields, &metrics, bootstrap_trace)?;

    // One outgoing edge per cell, toward its lowest strictly lower four-face
    // neighbour, ties broken by canonical cell key. Every edge therefore strictly
    // lowers elevation and the graph is acyclic; a local minimum has no outlet and
    // keeps its water. Edges are only built between two resident cells — an edge
    // leaving the world would let baseflow drain into somewhere that does not
    // exist, which no boundary record would account for.
    let mut edges = Vec::new();
    for chunk in &resident {
        let ground = &terrain[chunk];
        for ordinal in 0..SURFACE_CELL_COUNT {
            let cell = HydrologyCellKey::new(*chunk, ordinal as u16)?;
            let here = ground.elevations()[ordinal].millimetres();
            let mut best: Option<(i32, HydrologyCellKey)> = None;
            for direction in FaceDirection::ALL {
                let Some(neighbor) = cell.neighbor(direction) else {
                    continue;
                };
                if !fields.is_resident(neighbor) {
                    continue;
                }
                let elevation = terrain[&neighbor.chunk()].elevations()
                    [usize::from(neighbor.cell_ordinal())]
                .millimetres();
                if elevation >= here {
                    continue;
                }
                let better = match best {
                    None => true,
                    Some((lowest, key)) => {
                        elevation < lowest || (elevation == lowest && neighbor < key)
                    }
                };
                if better {
                    best = Some((elevation, neighbor));
                }
            }
            let Some((_, outlet)) = best else {
                continue;
            };
            let resolved = resolve_override(parameters, chunk.chart, cell);
            edges.push(HydrologyConveyanceEdge::new(
                HydrologyEdgeKey::new(cell, outlet)?,
                outlet,
                WaterVolume::new(resolved.conveyance_initial_storage),
                WaterVolume::new(resolved.conveyance_capacity),
                HydraulicFraction::from_parts(resolved.release.0, resolved.release.1)?,
                WaterVolume::new(resolved.conveyance_inlet),
                bootstrap_trace,
                WaterVolume::new(resolved.conveyance_initial_storage),
            )?);
        }
    }
    let conveyance = HydrologyConveyanceGraph::new(edges)?;

    // Every exterior face gets an explicit record. A face with no resident
    // neighbour and no record refuses the tick, so leaving one out would not be a
    // permissive default — it would be an unrunnable world.
    let mut boundary_records = Vec::new();
    for chunk in &resident {
        for ordinal in 0..SURFACE_CELL_COUNT {
            let cell = HydrologyCellKey::new(*chunk, ordinal as u16)?;
            let resolved_faces = face_overrides(parameters, chunk.chart, cell);
            for direction in FaceDirection::ALL {
                let exterior = match cell.neighbor(direction) {
                    Some(neighbor) => !fields.is_resident(neighbor),
                    None => true,
                };
                if !exterior {
                    continue;
                }
                let condition = resolved_faces
                    .get(&direction)
                    .copied()
                    .unwrap_or(parameters.default_boundary);
                boundary_records.push((HydrologyExteriorFaceKey::new(cell, direction), condition));
            }
        }
    }
    let boundaries = HydrologyBoundaryMap::new(boundary_records)?;

    let region: std::collections::BTreeSet<ChartChunkCoord> = resident.iter().copied().collect();
    let active = HydrologyActiveRegion::new(region.clone(), region)?;
    // Every chunk starts at level zero whether or not resolution is enabled.
    // Promotion is a causal event committed in `Phase::Resolution`, so bootstrap
    // cannot hand a world a detail level nothing decided.
    let level = 0_u8;
    let resolution: BTreeMap<ChartChunkCoord, HydrologyResolutionState> = resident
        .iter()
        .map(|chunk| {
            Ok((
                *chunk,
                HydrologyResolutionState::new(level, bootstrap_trace)?,
            ))
        })
        .collect::<Result<_, causafera_geography::HydrologyStateError>>()?;

    state.registry = HydrologyObjectRegistry::assign(
        registry_cells(&fields),
        conveyance.edges().keys().copied(),
        config
            .forcing_schedule
            .iter()
            .map(|spec| (spec.scheduled_tick, spec.forcing_id)),
        resident.iter().copied(),
    );
    state.enabled = true;
    state.fields = fields;
    state.conveyance = conveyance;
    state.boundaries = boundaries;
    state.metrics = metrics;
    state.active = active;
    state.resolution = resolution;
    state.resolution_policy = config.resolution_policy;
    let _ = &mut state.forcing;
    Ok(state)
}

/// The numbers one cell resolves to after cell, chart, and default precedence.
struct ResolvedCell {
    surface_capacity: u64,
    soil_capacity: u64,
    groundwater_capacity: u64,
    initial_surface: u64,
    initial_soil: u64,
    initial_groundwater: u64,
    infiltration_rate: u64,
    percolation: (u32, u32),
    specific_yield: (u32, u32),
    aquifer_base_offset: i64,
    baseflow_threshold: u64,
    baseflow: (u32, u32),
    surface_transmissivity: u64,
    groundwater_transmissivity: u64,
    roughness_reference: u64,
    conveyance_capacity: u64,
    conveyance_initial_storage: u64,
    conveyance_inlet: u64,
    release: (u32, u32),
}

/// Cell override, then chart override, then default — in that order and no other.
fn resolve_override(
    parameters: &crate::HydrologyBootstrapParameters,
    chart: causafera_types::SpatialChartId,
    cell: HydrologyCellKey,
) -> ResolvedCell {
    let chart_override = parameters.chart_overrides.get(&chart);
    let cell_override = parameters.cell_overrides.get(&cell);
    macro_rules! pick {
        ($field:ident, $default:expr) => {
            cell_override
                .and_then(|o| o.$field)
                .or_else(|| chart_override.and_then(|o| o.$field))
                .unwrap_or($default)
        };
    }
    macro_rules! pick_volume {
        ($field:ident, $default:expr) => {
            pick!($field, $default).get()
        };
    }
    macro_rules! pick_fraction {
        ($num:ident, $den:ident, $default_num:expr, $default_den:expr) => {
            (pick!($num, $default_num), pick!($den, $default_den).get())
        };
    }
    ResolvedCell {
        surface_capacity: pick_volume!(surface_capacity, parameters.default_surface_capacity),
        soil_capacity: pick_volume!(soil_capacity, parameters.default_soil_capacity),
        groundwater_capacity: pick_volume!(
            groundwater_capacity,
            parameters.default_groundwater_capacity
        ),
        initial_surface: pick_volume!(initial_surface, parameters.initial_surface),
        initial_soil: pick_volume!(initial_soil, parameters.initial_soil),
        initial_groundwater: pick_volume!(initial_groundwater, parameters.initial_groundwater),
        infiltration_rate: pick!(
            infiltration_rate_mm_per_second,
            parameters.infiltration_rate_mm_per_second
        ),
        percolation: pick_fraction!(
            percolation_fraction_num,
            percolation_fraction_den,
            parameters.percolation_fraction_num,
            parameters.percolation_fraction_den
        ),
        specific_yield: pick_fraction!(
            specific_yield_num,
            specific_yield_den,
            parameters.specific_yield_num,
            parameters.specific_yield_den
        ),
        aquifer_base_offset: pick!(aquifer_base_offset_mm, parameters.aquifer_base_offset_mm),
        baseflow_threshold: pick_volume!(baseflow_threshold, parameters.baseflow_threshold),
        baseflow: pick_fraction!(
            baseflow_fraction_num,
            baseflow_fraction_den,
            parameters.baseflow_fraction_num,
            parameters.baseflow_fraction_den
        ),
        surface_transmissivity: pick!(
            base_surface_transmissivity_mm3_per_second,
            parameters.base_surface_transmissivity_mm3_per_second
        ),
        groundwater_transmissivity: pick!(
            base_groundwater_transmissivity_mm3_per_second,
            parameters.base_groundwater_transmissivity_mm3_per_second
        ),
        roughness_reference: pick!(roughness_reference_mm, parameters.roughness_reference_mm).get(),
        conveyance_capacity: pick_volume!(conveyance_capacity, parameters.conveyance_capacity),
        conveyance_initial_storage: pick_volume!(
            conveyance_initial_storage,
            parameters.conveyance_initial_storage
        ),
        conveyance_inlet: pick_volume!(
            conveyance_inlet_capacity_per_tick,
            parameters.conveyance_inlet_capacity_per_tick
        ),
        release: pick_fraction!(
            conveyance_release_fraction_num,
            conveyance_release_fraction_den,
            parameters.conveyance_release_fraction_num,
            parameters.conveyance_release_fraction_den
        ),
    }
}

/// Face-specific boundary conditions, with cell overrides taking precedence.
fn face_overrides(
    parameters: &crate::HydrologyBootstrapParameters,
    chart: causafera_types::SpatialChartId,
    cell: HydrologyCellKey,
) -> BTreeMap<causafera_geography::FaceDirection, causafera_geography::HydrologyBoundaryCondition> {
    let mut faces = parameters
        .chart_overrides
        .get(&chart)
        .map(|o| o.face_boundaries.clone())
        .unwrap_or_default();
    if let Some(cell_override) = parameters.cell_overrides.get(&cell) {
        for (direction, condition) in &cell_override.face_boundaries {
            faces.insert(*direction, *condition);
        }
    }
    faces
}

fn checked_mul(left: u64, right: u64) -> Result<u128, RuntimeError> {
    u128::from(left)
        .checked_mul(u128::from(right))
        .ok_or(RuntimeError::HydrologyCoefficientOverflow)
}

fn checked_mul3(a: u64, b: u64, c: u64) -> Result<u128, RuntimeError> {
    checked_mul(a, b)?
        .checked_mul(u128::from(c))
        .ok_or(RuntimeError::HydrologyCoefficientOverflow)
}

/// Floor division into a `u64` coefficient, refusing rather than saturating.
///
/// A coefficient that does not fit its destination type is a configuration the
/// solver cannot execute; clamping it would run a different world than the one
/// that was asked for.
fn floor_div(numerator: u128, denominator: u128) -> Result<u64, RuntimeError> {
    if denominator == 0 {
        return Err(RuntimeError::HydrologyCoefficientOverflow);
    }
    u64::try_from(numerator / denominator).map_err(|_| RuntimeError::HydrologyCoefficientOverflow)
}

// ---------------------------------------------------------------------------
// The seventh production bootstrap stage
// ---------------------------------------------------------------------------

const DOMAIN_BOOTSTRAP_METRICS: &[u8] = b"causafera.hydrology.bootstrap-metrics.v1";
const DOMAIN_BOOTSTRAP_SUBSTRATE: &[u8] = b"causafera.hydrology.bootstrap-substrate.v1";
const DOMAIN_BOOTSTRAP_STORAGE: &[u8] = b"causafera.hydrology.bootstrap-storage.v1";
const DOMAIN_BOOTSTRAP_EDGES: &[u8] = b"causafera.hydrology.bootstrap-edges.v1";
const DOMAIN_BOOTSTRAP_RESOLUTION: &[u8] = b"causafera.hydrology.bootstrap-resolution.v1";
const DOMAIN_BOOTSTRAP_FORCING: &[u8] = b"causafera.hydrology.bootstrap-forcing.v1";
const DOMAIN_BOOTSTRAP_BOUNDARIES: &[u8] = b"causafera.hydrology.bootstrap-boundaries.v1";

fn hash_prefixed(hasher: &mut blake3::Hasher, bytes: &[u8]) {
    hasher.update(&(bytes.len() as u64).to_be_bytes());
    hasher.update(bytes);
}

/// The seven canonical payloads the bootstrap event's effects digest.
///
/// Seven aggregates rather than one effect per cell: bootstrap initialises up to
/// 131 072 cells and the eight-effect cap is a hard bound, so each payload covers
/// one whole collection in canonical order. Every initialised carrier still has a
/// verifiable path to its exact bytes, because its bytes are in one of these.
fn bootstrap_payloads(
    state: &HydrologyRuntimeState,
    specs: &[crate::HydrologyForcingSpec],
) -> [(u64, StateFingerprint); 7] {
    use causafera_geography::FluxBoundary;

    let mut metrics = blake3::Hasher::new();
    hash_prefixed(&mut metrics, DOMAIN_BOOTSTRAP_METRICS);
    metrics.update(&(state.metrics.len() as u64).to_be_bytes());
    for (chart, metric) in state.metrics.entries() {
        metrics.update(&chart.raw().to_be_bytes());
        metrics.update(&metric.schema_version().to_be_bytes());
        metrics.update(&metric.cell_area_mm2().get().to_be_bytes());
        metrics.update(&metric.orthogonal_edge_length_mm().get().to_be_bytes());
        metrics.update(&metric.timestep_millis().get().to_be_bytes());
    }

    let mut substrate = blake3::Hasher::new();
    hash_prefixed(&mut substrate, DOMAIN_BOOTSTRAP_SUBSTRATE);
    let mut storage = blake3::Hasher::new();
    hash_prefixed(&mut storage, DOMAIN_BOOTSTRAP_STORAGE);
    substrate.update(&(state.fields.cell_count() as u64).to_be_bytes());
    storage.update(&(state.fields.cell_count() as u64).to_be_bytes());
    for (chunk, field) in state.fields.fields() {
        for (ordinal, ground) in field.substrate().iter().enumerate() {
            let cell = field.cells()[ordinal];
            hash_prefixed(&mut substrate, &chunk_bytes(*chunk));
            substrate.update(&(ordinal as u16).to_be_bytes());
            hash_prefixed(&mut substrate, ground.constitutive_key().bytes());
            hash_prefixed(&mut storage, &chunk_bytes(*chunk));
            storage.update(&(ordinal as u16).to_be_bytes());
            storage.update(&cell.surface_water().get().to_be_bytes());
            storage.update(&cell.soil_water().get().to_be_bytes());
            storage.update(&cell.groundwater().get().to_be_bytes());
        }
    }

    let mut edges = blake3::Hasher::new();
    hash_prefixed(&mut edges, DOMAIN_BOOTSTRAP_EDGES);
    edges.update(&(state.conveyance.len() as u64).to_be_bytes());
    for (key, edge) in state.conveyance.edges() {
        hash_prefixed(
            &mut edges,
            &causafera_geography::HydrologyCarrierKey::Edge(*key).encode(),
        );
        hash_prefixed(
            &mut edges,
            &causafera_geography::HydrologyCarrierKey::Cell(edge.outlet()).encode(),
        );
        edges.update(&edge.storage().get().to_be_bytes());
        edges.update(&edge.capacity().get().to_be_bytes());
        edges.update(&edge.release().numerator().to_be_bytes());
        edges.update(&edge.release().denominator().get().to_be_bytes());
        edges.update(&edge.inlet_capacity_per_tick().get().to_be_bytes());
    }

    let mut resolution = blake3::Hasher::new();
    hash_prefixed(&mut resolution, DOMAIN_BOOTSTRAP_RESOLUTION);
    resolution.update(&(state.resolution.len() as u64).to_be_bytes());
    for (chunk, entry) in &state.resolution {
        hash_prefixed(&mut resolution, &chunk_bytes(*chunk));
        resolution.update(&[entry.level()]);
    }
    resolution.update(&[u8::from(state.resolution_policy.enabled)]);
    resolution.update(&[state.resolution_policy.max_level]);
    resolution.update(&(state.active.active_chunks().len() as u64).to_be_bytes());
    for chunk in state.active.active_chunks() {
        hash_prefixed(&mut resolution, &chunk_bytes(*chunk));
    }

    let mut boundaries = blake3::Hasher::new();
    hash_prefixed(&mut boundaries, DOMAIN_BOOTSTRAP_BOUNDARIES);
    boundaries.update(&(state.boundaries.len() as u64).to_be_bytes());
    for (face, condition) in state.boundaries.records() {
        hash_prefixed(
            &mut boundaries,
            &causafera_geography::HydrologyCarrierKey::ExteriorFace(*face).encode(),
        );
        for channel in [condition.surface, condition.groundwater] {
            match channel {
                FluxBoundary::NoFlux => boundaries.update(&[0]),
                FluxBoundary::Open {
                    external_head_mm,
                    conductance_mm2_per_tick,
                } => {
                    boundaries.update(&[1]);
                    boundaries.update(&external_head_mm.to_be_bytes());
                    boundaries.update(&conductance_mm2_per_tick.to_be_bytes())
                }
            };
        }
    }

    // The *spec* schedule, not the installed records: the records carry the origin
    // trace of the very event this digest is an effect of, so hashing them would
    // require the event to exist before it could be built. The specs are the
    // configuration's whole bounded contribution, which is what the plan says this
    // aggregate covers.
    let mut forcing = blake3::Hasher::new();
    hash_prefixed(&mut forcing, DOMAIN_BOOTSTRAP_FORCING);
    forcing.update(&(specs.len() as u64).to_be_bytes());
    for spec in specs {
        forcing.update(&spec.scheduled_tick.to_be_bytes());
        forcing.update(&spec.forcing_id.to_be_bytes());
        forcing.update(&spec.precipitation_volume.get().to_be_bytes());
        forcing.update(&spec.potential_et_volume.get().to_be_bytes());
        forcing.update(&spec.external_inflow_volume.get().to_be_bytes());
        forcing.update(&(spec.targets.len() as u64).to_be_bytes());
        for (cell, weight) in &spec.targets {
            hash_prefixed(
                &mut forcing,
                &causafera_geography::HydrologyCarrierKey::Cell(*cell).encode(),
            );
            forcing.update(&weight.get().to_be_bytes());
        }
    }
    // The policy is the bootstrap producer's, fixed rather than chosen, so it is
    // hashed once for the schedule instead of once per record.
    forcing.update(&causafera_geography::BOOTSTRAP_HYDROLOGY_FORCING_POLICY_V1.to_be_bytes());

    [
        (
            crate::HYDROLOGY_BOOTSTRAP_METRICS_PROPERTY,
            StateFingerprint::new(*metrics.finalize().as_bytes()),
        ),
        (
            crate::HYDROLOGY_BOOTSTRAP_SUBSTRATE_PROPERTY,
            StateFingerprint::new(*substrate.finalize().as_bytes()),
        ),
        (
            crate::HYDROLOGY_BOOTSTRAP_STORAGE_PROPERTY,
            StateFingerprint::new(*storage.finalize().as_bytes()),
        ),
        (
            crate::HYDROLOGY_BOOTSTRAP_EDGES_PROPERTY,
            StateFingerprint::new(*edges.finalize().as_bytes()),
        ),
        (
            crate::HYDROLOGY_BOOTSTRAP_RESOLUTION_PROPERTY,
            StateFingerprint::new(*resolution.finalize().as_bytes()),
        ),
        (
            crate::HYDROLOGY_BOOTSTRAP_FORCING_PROPERTY,
            StateFingerprint::new(*forcing.finalize().as_bytes()),
        ),
        (
            crate::HYDROLOGY_BOOTSTRAP_BOUNDARIES_PROPERTY,
            StateFingerprint::new(*boundaries.finalize().as_bytes()),
        ),
    ]
}

fn chunk_bytes(chunk: ChartChunkCoord) -> Vec<u8> {
    let mut out = Vec::with_capacity(20);
    out.extend_from_slice(&chunk.chart.raw().to_be_bytes());
    out.extend_from_slice(&chunk.chunk.x.to_be_bytes());
    out.extend_from_slice(&chunk.chunk.y.to_be_bytes());
    out.extend_from_slice(&chunk.chunk.z.to_be_bytes());
    out
}

/// The fingerprint a bootstrap aggregate target holds before bootstrap runs.
fn bootstrap_absent(property: u64) -> StateFingerprint {
    let mut hasher = blake3::Hasher::new();
    hash_prefixed(&mut hasher, b"causafera.hydrology.bootstrap-absent.v1");
    hasher.update(&property.to_be_bytes());
    StateFingerprint::new(*hasher.finalize().as_bytes())
}

/// The seventh production bootstrap stage.
///
/// Runs only when hydrology is enabled. A disabled session keeps the legacy
/// six-stage plan byte for byte, which is what lets every pre-hydrology snapshot
/// stay comparable (V22).
pub(crate) struct HydrologyBootstrapStage;

impl HydrologyBootstrapStage {
    /// Preflight the whole proposal, commit one origin event, then install.
    ///
    /// In that order and no other: the installation step after the commit is
    /// infallible, so a failed preflight or a refused origin event installs no
    /// hydrology state at all rather than half of it.
    pub(crate) fn run(
        state: &mut RuntimeState,
        previous_receipt: Option<TraceId>,
    ) -> Result<Vec<TraceId>, RuntimeError> {
        let config = state.config.hydrology.clone();
        if !config.enabled {
            return Ok(Vec::new());
        }
        let cause = previous_receipt.ok_or(RuntimeError::HydrologyBootstrapWithoutPredecessor)?;

        // Built with the origin trace it is about to be anchored to, so no anchor
        // ever names a placeholder that later has to be rewritten.
        let world_seed = state.config.deterministic.world_seed;
        let active: Vec<ChartChunkCoord> = state.active_chunks.keys().copied().collect();
        let mut hydrology = build_hydrology_state(&config, world_seed, &active, cause)?;
        hydrology.registry = HydrologyObjectRegistry::assign(
            registry_cells(&hydrology.fields),
            hydrology.conveyance.edges().keys().copied(),
            config
                .forcing_schedule
                .iter()
                .map(|spec| (spec.scheduled_tick, spec.forcing_id)),
            hydrology.resolution.keys().copied(),
        );

        let payloads = bootstrap_payloads(&hydrology, &config.forcing_schedule);
        let effects = payloads
            .iter()
            .map(|(property, fingerprint)| {
                causafera_core::provenance::CausalEffect::new(
                    causafera_core::provenance::CausalTarget::new(
                        causafera_types::StateObjectKindId::new(
                            crate::HYDROLOGY_BOOTSTRAP_OBJECT_KIND,
                        ),
                        0,
                        causafera_types::StatePropertyId::new(*property),
                    ),
                    bootstrap_absent(*property),
                    *fingerprint,
                )
            })
            .collect::<Result<Vec<_>, _>>()?;

        let proposal = causafera_core::provenance::CausalEventProposal::new(
            causafera_core::EventProposalKey::new(
                crate::HYDROLOGY_SYSTEM_ID,
                crate::HYDROLOGY_STAGE.raw(),
                0,
            ),
            causafera_types::EventKindId::new(crate::HYDROLOGY_BOOTSTRAP_EVENT_KIND),
            vec![cause],
            effects,
        )?;
        let origin = state
            .traces
            .commit_batch(SimulationTime::new(0), Phase::Lifecycle, vec![proposal])?
            .first()
            .copied()
            .ok_or(RuntimeError::HydrologyConservationNotCommitted)?;

        // The event committed, so the origin exists. Everything after this point is
        // installation against a trace that is already durable: specs become
        // canonical records — configuration never contains a trace, and letting it
        // choose its own producer policy would let a session declare itself an
        // authorized producer — and every initialised carrier is re-anchored from
        // the cause it was built against to the origin event that created it.
        hydrology.forcing = install_forcing(&config, origin)?;
        reanchor(&mut hydrology, origin)?;
        state.hydrology = hydrology;
        Ok(vec![origin])
    }
}

/// Turn configured specs into canonical records under the bootstrap policy.
fn install_forcing(
    config: &crate::HydrologyConfig,
    origin: TraceId,
) -> Result<Vec<HydrologyForcingRecord>, RuntimeError> {
    use causafera_geography::{
        BOOTSTRAP_HYDROLOGY_FORCING_POLICY_V1, HydrologyForcingMember, HydrologyForcingParts,
    };

    let mut records = Vec::with_capacity(config.forcing_schedule.len());
    for spec in &config.forcing_schedule {
        records.push(HydrologyForcingRecord::new(HydrologyForcingParts {
            forcing_id: spec.forcing_id,
            scheduled_tick: spec.scheduled_tick,
            targets: spec
                .targets
                .iter()
                .map(|(cell, weight)| HydrologyForcingMember::new(*cell, *weight))
                .collect(),
            precipitation_volume: spec.precipitation_volume,
            potential_et_volume: spec.potential_et_volume,
            external_inflow_volume: spec.external_inflow_volume,
            origin_trace: origin,
            producer_policy_schema: BOOTSTRAP_HYDROLOGY_FORCING_POLICY_V1,
            // Canonical records start pending and transition exactly once.
            applied_at: None,
        })?);
    }
    Ok(records)
}

/// Point every initial anchor at the committed origin event.
///
/// Fallible on purpose. An anchor that could not be written is a carrier whose
/// provenance would silently still name the stage that preceded its creation, and
/// discarding that error would leave the world describing itself incorrectly.
fn reanchor(state: &mut HydrologyRuntimeState, origin: TraceId) -> Result<(), RuntimeError> {
    let cells: Vec<HydrologyCellKey> = registry_cells(&state.fields);
    for cell in cells {
        state.fields.install_surface_trace(cell, origin)?;
        state.fields.install_soil_trace(cell, origin)?;
        state.fields.install_groundwater_trace(cell, origin)?;
        state.fields.install_forcing_trace(cell, origin)?;
    }
    state.fields.install_conservation_trace(origin);
    let edges: Vec<causafera_geography::HydrologyEdgeKey> =
        state.conveyance.edges().keys().copied().collect();
    for edge in edges {
        state.conveyance.install_edge_trace(edge, origin)?;
    }
    for entry in state.resolution.values_mut() {
        *entry = HydrologyResolutionState::new(entry.level(), origin)?;
    }
    Ok(())
}
