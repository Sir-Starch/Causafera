use std::collections::{BTreeMap, BTreeSet};

use causafera_core::{CausalEventDagCause, CausalEventProposalKey, StateFingerprint};
use causafera_geography::{
    FaceDirection, FluxBoundary, HydraulicFraction, HydraulicSubstrateCell,
    HydrologyBoundaryCondition, HydrologyCarrierKey, HydrologyCellKey, HydrologyCellState,
    HydrologyCellStorage, HydrologyConveyanceEdge, HydrologyConveyanceGraph, HydrologyEdgeKey,
    HydrologyExteriorFaceKey, HydrologyField, HydrologyFieldSet, HydrologyForcingRecord,
    HydrologyGridMetric, HydrologyGridMetrics, SURFACE_CELL_COUNT, TERRAIN_CELLS_PER_CHUNK,
};
use causafera_types::{
    ChartChunkCoord, TraceId, WaterAccumulator, WaterVolume, WaterVolumeError, checked_water_mul,
};

use super::{
    HydrologyBlockKey, HydrologyCoarseMember, HydrologyCoarseProcess, HydrologyConstitutiveKey,
    HydrologyResolutionPlan, allocate_capped, clamp_to_allocatable,
};
use super::{
    HydrologyBucket, HydrologyCellChange, HydrologyConservationParts, HydrologyConservationReceipt,
    HydrologyEdgeChange, HydrologyError, HydrologyEventEffect, HydrologyEventKind,
    HydrologyEventPlan, HydrologyEvolutionProposal, HydrologyEvolutionRequest,
    HydrologyForcingAllocation, HydrologyForcingSettlement, HydrologyProperty,
    HydrologyProposalParts, HydrologyTerminalLeaf, HydrologyTransferParts,
    HydrologyTransferReceipt, forcing_applied_fingerprint, forcing_settlement_fingerprint, process,
    substage, volume_fingerprint,
};

/// The hydrology lattice is the terrain lattice, so an ordinal means the same
/// cell in both. If that ever stopped holding, every head in the solver would be
/// read from the wrong ground.
const _: () = assert!(SURFACE_CELL_COUNT == TERRAIN_CELLS_PER_CHUNK);

/// Deterministic hydrology evolution over one frozen state.
///
/// Stateless: everything it needs arrives in the request, and everything it
/// decides leaves in the proposal. Nothing here mutates authoritative state —
/// the runtime installs the proposal only after the causal batch commits.
pub struct HydrologyEvolutionModel;

/// Proportional allocation by the largest-remainder rule.
///
/// `weights` must already be in canonical key order, because the tie-break for
/// equal remainders is ascending key and that is the order the caller holds.
///
/// The parts sum to exactly `total`. Rounding each share independently would
/// leave a shortfall that has to go somewhere, and "somewhere" is how a
/// quantisation sink gets built: this distributes it deterministically instead.
pub fn allocate_largest_remainder(
    total: u128,
    weights: &[u128],
) -> Result<Vec<u128>, HydrologyError> {
    if weights.is_empty() {
        if total == 0 {
            return Ok(Vec::new());
        }
        return Err(HydrologyError::UnallocatableTotal);
    }
    if total == 0 {
        return Ok(vec![0; weights.len()]);
    }

    let mut sum = 0_u128;
    for weight in weights {
        sum = sum.checked_add(*weight).ok_or(WaterVolumeError::Overflow)?;
    }
    if sum == 0 {
        return Err(HydrologyError::UnallocatableTotal);
    }

    let mut shares = Vec::with_capacity(weights.len());
    let mut remainders = Vec::with_capacity(weights.len());
    let mut assigned = 0_u128;
    for weight in weights {
        let product = weight
            .checked_mul(total)
            .ok_or(WaterVolumeError::Overflow)?;
        let share = product / sum;
        remainders.push(product % sum);
        assigned = assigned
            .checked_add(share)
            .ok_or(WaterVolumeError::Overflow)?;
        shares.push(share);
    }

    // `assigned` is a sum of floors, so the shortfall is strictly less than the
    // number of members and every unit of it lands on a distinct member.
    let mut shortfall = total
        .checked_sub(assigned)
        .ok_or(WaterVolumeError::Underflow)?;
    let mut order: Vec<usize> = (0..weights.len()).collect();
    order.sort_by(|&left, &right| {
        remainders[right]
            .cmp(&remainders[left])
            .then_with(|| left.cmp(&right))
    });
    for index in order {
        if shortfall == 0 {
            break;
        }
        shares[index] += 1;
        shortfall -= 1;
    }
    debug_assert_eq!(shares.iter().sum::<u128>(), total);
    Ok(shares)
}

/// The running causal references and pre-tick values of one cell.
#[derive(Clone, Debug)]
struct CellWork {
    before: HydrologyCellStorage,
    storage: HydrologyCellStorage,
    surface_ref: CausalEventDagCause,
    soil_ref: CausalEventDagCause,
    groundwater_ref: CausalEventDagCause,
    /// The anchors the cell arrived with. The forcing settlement cites these
    /// rather than the running references, because it is a substage-1 event even
    /// though its fingerprint is completed after substage 4.
    pre_tick_surface_ref: CausalEventDagCause,
    pre_tick_soil_ref: CausalEventDagCause,
    forcing_settlement: Option<CausalEventProposalKey>,
    terminal_surface: Option<CausalEventProposalKey>,
    terminal_soil: Option<CausalEventProposalKey>,
    terminal_groundwater: Option<CausalEventProposalKey>,
    terminal_forcing: Option<CausalEventProposalKey>,
    forcing_fingerprint: StateFingerprint,
    et_demand: WaterVolume,
}

/// One cell's accumulating forcing shares, before it is settled.
#[derive(Clone, Debug, Default)]
struct SettlementWork {
    allocations: Vec<HydrologyForcingAllocation>,
    accepted_source: WaterVolume,
    rejected_source: WaterVolume,
    accepted_et: WaterVolume,
    unmet_et: WaterVolume,
    /// The substage-1 surface change this settlement delivered, captured before
    /// any later substage moves the bucket again.
    surface_before: WaterVolume,
    surface_after: WaterVolume,
    origins: BTreeSet<TraceId>,
}

struct Batch {
    tick: u64,
    batch_sequence: u64,
    receipts: Vec<HydrologyTransferReceipt>,
    events: Vec<HydrologyEventPlan>,
    cell_changes: Vec<HydrologyCellChange>,
    edge_changes: Vec<HydrologyEdgeChange>,
    accepted_precipitation: WaterAccumulator,
    accepted_external_inflow: WaterAccumulator,
    accepted_evapotranspiration: WaterAccumulator,
    boundary_exports: WaterAccumulator,
    /// Exports accepted in substages 5 and 6, waiting for substage 8 to
    /// materialize their sink receipts. Substage 8 computes no new demand — the
    /// water has already left — so holding them here keeps the ledger's `sinks`
    /// term and its evidence in the substage the plan puts them in.
    pending_exports: Vec<PendingExport>,
}

/// One accepted open-boundary export, before substage 8 records it.
struct PendingExport {
    donor: HydrologyCellKey,
    face: HydrologyExteriorFaceKey,
    bucket: HydrologyBucket,
    process_kind: u32,
    requested: WaterVolume,
    accepted: WaterVolume,
    source_before: WaterVolume,
    source_after: WaterVolume,
    /// The routing settlement event that actually removed the water.
    event: CausalEventProposalKey,
}

/// One conveyance edge's running state through a tick.
#[derive(Clone, Debug)]
struct EdgeWork {
    source: HydrologyCellKey,
    outlet: HydrologyCellKey,
    before: WaterVolume,
    storage: WaterVolume,
    capacity: WaterVolume,
    release: HydraulicFraction,
    /// The configured per-tick inlet budget. A coefficient, not state: it resets
    /// every tick and is never persisted as "what is left".
    inlet_capacity: WaterVolume,
    /// How much of that budget is still unspent. Surface inflow spends it first,
    /// then baseflow, then conveyance release — the plan's substage order is
    /// what decides which process gets a contested inlet.
    inlet_remaining: WaterVolume,
    last_change: TraceId,
    last_change_before: WaterVolume,
    reference: CausalEventDagCause,
    terminal: Option<CausalEventProposalKey>,
}

fn build_edge_work(graph: &HydrologyConveyanceGraph) -> BTreeMap<HydrologyEdgeKey, EdgeWork> {
    graph
        .edges()
        .iter()
        .map(|(key, edge)| {
            (
                *key,
                EdgeWork {
                    source: edge.source(),
                    outlet: edge.outlet(),
                    before: edge.storage(),
                    storage: edge.storage(),
                    capacity: edge.capacity(),
                    release: edge.release(),
                    inlet_capacity: edge.inlet_capacity_per_tick(),
                    inlet_remaining: edge.inlet_capacity_per_tick(),
                    last_change: edge.last_change(),
                    last_change_before: edge.last_change_before(),
                    reference: CausalEventDagCause::Existing(edge.last_change()),
                    terminal: None,
                },
            )
        })
        .collect()
}

impl HydrologyEvolutionModel {
    /// Evolve one tick and return the complete proposal.
    ///
    /// The substages run in the plan's fixed order, each reading what the
    /// previous produced. No cell observes another cell's write within a
    /// substage, so the result cannot depend on iteration order — which is what
    /// makes a chunk seam behave like an interior face rather than like a
    /// direction the solver happens to sweep.
    pub fn propose(
        state: &HydrologyFieldSet,
        request: HydrologyEvolutionRequest<'_>,
    ) -> Result<HydrologyEvolutionProposal, HydrologyError> {
        let batch_sequence = state
            .batch_sequence()
            .checked_add(1)
            .ok_or(WaterVolumeError::Overflow)?;
        let mut batch = Batch {
            tick: request.tick,
            batch_sequence,
            receipts: Vec::new(),
            events: Vec::new(),
            cell_changes: Vec::new(),
            edge_changes: Vec::new(),
            accepted_precipitation: WaterAccumulator::ZERO,
            accepted_external_inflow: WaterAccumulator::ZERO,
            accepted_evapotranspiration: WaterAccumulator::ZERO,
            boundary_exports: WaterAccumulator::ZERO,
            pending_exports: Vec::new(),
        };

        // The chart of every resident chunk must have a registered metric, and
        // every resident chunk must have the terrain its heads are measured
        // against. The field set already checked the metric at construction;
        // re-checking here keeps the solver honest about its own preconditions
        // rather than trusting a caller-built value object.
        for chunk in state.fields().keys() {
            request.metrics.get(chunk.chart)?;
            let terrain = request
                .terrain
                .get(chunk)
                .ok_or(HydrologyError::TerrainMissing)?;
            if terrain.chunk() != chunk.chunk {
                return Err(HydrologyError::TerrainChunkMismatch);
            }
        }
        // The request's own idea of which chunks are resident has to be the field
        // set it was handed. Without this the active region would be decoration:
        // a solver that routed over a chunk the request does not consider
        // resident would be exchanging water across an edge of the world.
        if request.active.resident_chunks().len() != state.fields().len()
            || !state
                .fields()
                .keys()
                .all(|chunk| request.active.resident_chunks().contains(chunk))
        {
            return Err(HydrologyError::ResidencyMismatch);
        }

        let plan = HydrologyResolutionPlan::build(
            state,
            request.boundaries,
            request.metrics,
            request.resolution,
            request.resolution_policy,
        )?;

        let mut work = build_work(state);
        let mut edges = build_edge_work(request.conveyance);
        let mut applied_forcing = Vec::new();
        let mut coarse_processes = Vec::new();
        let mut settlements = substage_forcing(
            &mut batch,
            &mut work,
            state,
            &request,
            &plan,
            &mut coarse_processes,
            &mut applied_forcing,
        )?;
        substage_infiltration(&mut batch, &mut work, state, &plan)?;
        coarse_vertical_pass(
            &mut batch,
            &mut work,
            state,
            &plan,
            &mut coarse_processes,
            &COARSE_INFILTRATION,
        )?;
        substage_percolation(&mut batch, &mut work, state, &plan)?;
        coarse_vertical_pass(
            &mut batch,
            &mut work,
            state,
            &plan,
            &mut coarse_processes,
            &COARSE_PERCOLATION,
        )?;
        substage_evapotranspiration(&mut batch, &mut work, &mut settlements, &plan)?;
        coarse_evapotranspiration(
            &mut batch,
            &mut work,
            &plan,
            &mut settlements,
            &mut coarse_processes,
        )?;
        let settlements = finalise_settlements(&mut batch, &mut work, settlements)?;
        route(
            &mut batch,
            &mut work,
            &mut edges,
            state,
            &request,
            &plan,
            &SURFACE_CHANNEL,
        )?;
        route(
            &mut batch,
            &mut work,
            &mut edges,
            state,
            &request,
            &plan,
            &GROUNDWATER_CHANNEL,
        )?;
        substage_conveyance(&mut batch, &mut work, &mut edges, state, &request)?;
        substage_boundary_export(&mut batch)?;

        if batch.receipts.len() > request.limits.max_transfers_per_tick {
            return Err(HydrologyError::TransferLimitExceeded {
                count: batch.receipts.len(),
                max: request.limits.max_transfers_per_tick,
            });
        }
        // The trace store enforces these caps too, and would reject the batch
        // atomically. Checking here means a proposal the store cannot possibly
        // commit is never handed back as if it were valid.
        for event in &batch.events {
            // A fine allocation event gains one more cause when the runtime
            // resolves its coarse process, so the cap is checked against the
            // committed width and not against the width the domain can see.
            let causes = event.causes.len() + usize::from(event.coarse_process.is_some());
            if causes > request.limits.max_causes_per_event {
                return Err(HydrologyError::EventCauseLimitExceeded {
                    count: causes,
                    max: request.limits.max_causes_per_event,
                });
            }
            if event.effects.len() > request.limits.max_effects_per_event {
                return Err(HydrologyError::EventEffectLimitExceeded {
                    count: event.effects.len(),
                    max: request.limits.max_effects_per_event,
                });
            }
        }

        // The accepted group total and the sum of the fine grants have to agree
        // exactly. A coarse process that allocated a different amount than it
        // accepted would be a source or a sink wearing a group's name, and the
        // terminal residual alone could not say which group produced it.
        for coarse in &coarse_processes {
            let mut granted = WaterAccumulator::ZERO;
            for member in &coarse.members {
                granted = granted.add(member.granted)?;
            }
            if granted.get() != coarse.accepted_total {
                return Err(HydrologyError::ResolutionAllocationMismatch);
            }
        }

        let after_state = build_after_state(state, &work, request.metrics, batch_sequence)?;
        let after_conveyance = build_after_conveyance(&edges)?;
        let conservation = build_conservation(
            &batch,
            state,
            &after_state,
            request.conveyance,
            &after_conveyance,
        )?;
        conservation.require_balanced()?;

        let terminal_leaves = collect_terminal_leaves(&work, &edges, &batch.events);

        Ok(HydrologyEvolutionProposal::new(HydrologyProposalParts {
            tick: request.tick,
            batch_sequence,
            after_state,
            after_conveyance,
            applied_forcing,
            forcing_settlements: settlements,
            cell_changes: batch.cell_changes,
            edge_changes: batch.edge_changes,
            transfer_receipts: batch.receipts,
            conservation,
            events: batch.events,
            terminal_leaves,
            coarse_processes,
        }))
    }
}

fn build_work(state: &HydrologyFieldSet) -> BTreeMap<ChartChunkCoord, Vec<CellWork>> {
    state
        .fields()
        .iter()
        .map(|(chunk, field)| {
            let cells = field
                .cells()
                .iter()
                .map(|cell| CellWork {
                    before: cell.storage(),
                    storage: cell.storage(),
                    surface_ref: CausalEventDagCause::Existing(cell.surface_last_change()),
                    soil_ref: CausalEventDagCause::Existing(cell.soil_last_change()),
                    groundwater_ref: CausalEventDagCause::Existing(cell.groundwater_last_change()),
                    pre_tick_surface_ref: CausalEventDagCause::Existing(cell.surface_last_change()),
                    pre_tick_soil_ref: CausalEventDagCause::Existing(cell.soil_last_change()),
                    forcing_settlement: None,
                    terminal_surface: None,
                    terminal_soil: None,
                    terminal_groundwater: None,
                    terminal_forcing: None,
                    forcing_fingerprint: cell.forcing_input_fingerprint(),
                    et_demand: WaterVolume::ZERO,
                })
                .collect();
            (*chunk, cells)
        })
        .collect()
}

fn cell_work(
    work: &mut BTreeMap<ChartChunkCoord, Vec<CellWork>>,
    key: HydrologyCellKey,
) -> Option<&mut CellWork> {
    work.get_mut(&key.chunk())
        .and_then(|cells| cells.get_mut(usize::from(key.cell_ordinal())))
}

// ---------------------------------------------------------------------------
// Substages 1-4 — the vertical cycle, fine and coarse
// ---------------------------------------------------------------------------

/// One record's share of one cell, as requested before any capacity is consulted.
#[derive(Clone, Copy, Debug)]
struct RecordShare {
    cell: HydrologyCellKey,
    requested: WaterVolume,
}

/// A coarse invocation waiting for its group pass: the members that asked for
/// something, keyed so the map iterates in the plan's canonical order.
type CoarseQueue =
    BTreeMap<(u64, u64, u32, HydrologyBlockKey, HydrologyConstitutiveKey), Vec<RecordShare>>;

/// Grants the coarse passes decided, indexed the way the settlements read them.
#[derive(Default)]
struct CoarseGrants {
    source: BTreeMap<(u64, u64, HydrologyCellKey), WaterVolume>,
}

fn substage_forcing(
    batch: &mut Batch,
    work: &mut BTreeMap<ChartChunkCoord, Vec<CellWork>>,
    state: &HydrologyFieldSet,
    request: &HydrologyEvolutionRequest<'_>,
    plan: &HydrologyResolutionPlan,
    coarse: &mut Vec<HydrologyCoarseProcess>,
    applied: &mut Vec<(u64, u64)>,
) -> Result<BTreeMap<HydrologyCellKey, SettlementWork>, HydrologyError> {
    // Validate every record and every target before changing anything. A
    // partially applied schedule is not a weaker outcome, it is an
    // unreproducible one: the same inputs would land differently depending on
    // which record happened to be checked first.
    for record in request.forcing {
        if record.scheduled_tick() != request.tick {
            return Err(HydrologyError::ForcingTickMismatch {
                scheduled: record.scheduled_tick(),
                tick: request.tick,
            });
        }
        for member in record.targets() {
            if work
                .get(&member.cell.chunk())
                .and_then(|cells| cells.get(usize::from(member.cell.cell_ordinal())))
                .is_none()
            {
                return Err(HydrologyError::ForcingTargetNotResident);
            }
        }
    }

    let mut settlements: BTreeMap<HydrologyCellKey, SettlementWork> = BTreeMap::new();
    let mut queue: CoarseQueue = BTreeMap::new();
    let mut grants = CoarseGrants::default();

    for record in request.forcing {
        let weights: Vec<u128> = record
            .targets()
            .iter()
            .map(|member| u128::from(member.weight.get()))
            .collect();
        let precipitation =
            allocate_largest_remainder(u128::from(record.precipitation_volume().get()), &weights)?;
        let external = allocate_largest_remainder(
            u128::from(record.external_inflow_volume().get()),
            &weights,
        )?;
        let demand =
            allocate_largest_remainder(u128::from(record.potential_et_volume().get()), &weights)?;

        for (index, member) in record.targets().iter().enumerate() {
            let cell = member.cell;
            let precipitation_share = share_volume(precipitation[index])?;
            let external_share = share_volume(external[index])?;
            let demand_share = share_volume(demand[index])?;
            let coarse_cell = plan.is_coarse(cell);

            let (accepted_precipitation, accepted_external) = if coarse_cell {
                // A coarse member's share is decided by its constitutive group,
                // not by its own capacity: queue it and settle after every
                // member of the group has asked.
                for (process_kind, requested) in [
                    (process::PRECIPITATION, precipitation_share),
                    (process::EXTERNAL_INFLOW, external_share),
                ] {
                    if requested.is_zero() {
                        continue;
                    }
                    let (block, constitutive) = group_of(plan, cell)?;
                    queue
                        .entry((
                            record.scheduled_tick(),
                            record.forcing_id(),
                            process_kind,
                            block,
                            constitutive,
                        ))
                        .or_default()
                        .push(RecordShare { cell, requested });
                }
                (WaterVolume::ZERO, WaterVolume::ZERO)
            } else {
                // The receiving bucket's own capacity, read from the frozen
                // substrate. Overlapping records deplete it in canonical
                // `(scheduled_tick, forcing_id)` order, so what does not fit is
                // recorded as unaccepted rather than clamped away.
                let capacity = state
                    .ground(cell)
                    .ok_or(HydrologyError::ForcingTargetNotResident)?
                    .surface_capacity();
                let accepted_precipitation = accept_source(
                    batch,
                    work,
                    record,
                    cell,
                    capacity,
                    process::PRECIPITATION,
                    precipitation_share,
                )?;
                let accepted_external = accept_source(
                    batch,
                    work,
                    record,
                    cell,
                    capacity,
                    process::EXTERNAL_INFLOW,
                    external_share,
                )?;
                debug_assert!(accepted_precipitation <= precipitation_share);
                debug_assert!(accepted_external <= external_share);
                (accepted_precipitation, accepted_external)
            };

            let entry = settlements.entry(cell).or_default();
            entry.origins.insert(record.origin_trace());
            entry.allocations.push(HydrologyForcingAllocation {
                scheduled_tick: record.scheduled_tick(),
                forcing_id: record.forcing_id(),
                origin: record.origin_trace(),
                precipitation: precipitation_share,
                external_inflow: external_share,
                potential_et: demand_share,
                // Both are finalised once the coarse passes and substage 4 have
                // run; a fine cell's source total is already exact here.
                accepted_source: accepted_precipitation.checked_add(accepted_external)?,
                accepted_et: WaterVolume::ZERO,
            });

            if coarse_cell {
                if !demand_share.is_zero() {
                    let (block, constitutive) = group_of(plan, cell)?;
                    queue
                        .entry((
                            record.scheduled_tick(),
                            record.forcing_id(),
                            process::EVAPOTRANSPIRATION_SURFACE,
                            block,
                            constitutive,
                        ))
                        .or_default()
                        .push(RecordShare {
                            cell,
                            requested: demand_share,
                        });
                }
            } else {
                let entry = cell_work(work, cell).expect("residency was validated above");
                entry.et_demand = entry.et_demand.checked_add(demand_share)?;
            }
        }

        // The record's own application event: one effect, one cause, its origin.
        batch.events.push(HydrologyEventPlan {
            key: CausalEventProposalKey::new(
                substage::FORCING,
                process::FORCING_APPLICATION,
                &HydrologyCarrierKey::ForcingRecord {
                    scheduled_tick: record.scheduled_tick(),
                    forcing_id: record.forcing_id(),
                }
                .encode(),
                0,
            )?,
            kind: HydrologyEventKind::ForcingApplication,
            coarse_process: None,
            causes: vec![CausalEventDagCause::Existing(record.origin_trace())],
            effects: vec![HydrologyEventEffect {
                carrier: HydrologyCarrierKey::ForcingRecord {
                    scheduled_tick: record.scheduled_tick(),
                    forcing_id: record.forcing_id(),
                },
                property: HydrologyProperty::ForcingRecord,
                before: forcing_applied_fingerprint(
                    record.scheduled_tick(),
                    record.forcing_id(),
                    None,
                ),
                after: forcing_applied_fingerprint(
                    record.scheduled_tick(),
                    record.forcing_id(),
                    Some(request.tick),
                ),
            }],
        });
        applied.push((record.scheduled_tick(), record.forcing_id()));
    }

    coarse_source_pass(batch, work, state, plan, &queue, &mut grants, coarse)?;

    // Every settled cell now knows its exact accepted source, so the surface
    // delta and the running references can be fixed. The settlement *event* is
    // pushed by `finalise_settlements` after substage 4, because its fingerprint
    // covers each allocation's accepted ET and that is not known until then.
    for (cell, settlement) in &mut settlements {
        let entry = cell_work(work, *cell).expect("residency was validated above");
        for allocation in &mut settlement.allocations {
            if let Some(granted) =
                grants
                    .source
                    .get(&(allocation.scheduled_tick, allocation.forcing_id, *cell))
            {
                allocation.accepted_source = *granted;
            }
            settlement.accepted_source = settlement
                .accepted_source
                .checked_add(allocation.accepted_source)?;
            settlement.rejected_source = settlement.rejected_source.checked_add(
                allocation
                    .precipitation
                    .checked_add(allocation.external_inflow)?
                    .checked_sub(allocation.accepted_source)?,
            )?;
        }
        settlement.surface_before = entry.before.surface;
        settlement.surface_after = entry.storage.surface;

        let key = settlement_key(*cell)?;
        entry.forcing_settlement = Some(key.clone());
        entry.terminal_forcing = Some(key.clone());
        if !settlement.accepted_source.is_zero() {
            entry.surface_ref = CausalEventDagCause::Local(key.clone());
            entry.terminal_surface = Some(key);
        }
    }

    // Coarse ET is per record and per group, and has to be queued before
    // substage 4 so the settlements can carry its grants.
    Ok(settlements)
}

/// The block and constitutive group one coarse cell belongs to.
///
/// Read from the plan's reverse index rather than recomputed, so a coarse cell
/// cannot end up in one group when it is grouped and another when it is queued.
fn group_of(
    plan: &HydrologyResolutionPlan,
    cell: HydrologyCellKey,
) -> Result<(HydrologyBlockKey, HydrologyConstitutiveKey), HydrologyError> {
    plan.group_of(cell)
        .ok_or(HydrologyError::ResolutionEntryMissing)
}

/// One coarse group's members, in canonical cell order.
fn members_of(
    plan: &HydrologyResolutionPlan,
    block: HydrologyBlockKey,
    constitutive: HydrologyConstitutiveKey,
) -> &[HydrologyCellKey] {
    plan.groups()
        .get(&(block, constitutive))
        .map(Vec::as_slice)
        .unwrap_or_default()
}

/// Accept queued coarse source water, one `(record, kind, group)` at a time.
///
/// Weight is the member's already allocated fine request and the ceiling is its
/// remaining surface capacity, so the group accepts `min(sum requests, sum room)`
/// and the reducer decides which members hold it. That total can differ from what
/// the fine path would have accepted — that is the approximation resolution
/// introduces — but it can never differ in *total water*, which is why the
/// ceiling is a real capacity and not a scaling factor.
fn coarse_source_pass(
    batch: &mut Batch,
    work: &mut BTreeMap<ChartChunkCoord, Vec<CellWork>>,
    state: &HydrologyFieldSet,
    plan: &HydrologyResolutionPlan,
    queue: &CoarseQueue,
    grants: &mut CoarseGrants,
    coarse: &mut Vec<HydrologyCoarseProcess>,
) -> Result<(), HydrologyError> {
    for ((scheduled_tick, forcing_id, process_kind, block, constitutive), shares) in queue {
        if *process_kind != process::PRECIPITATION && *process_kind != process::EXTERNAL_INFLOW {
            continue;
        }
        let members = members_of(plan, *block, *constitutive);
        let mut weights = Vec::with_capacity(members.len());
        let mut ceilings = Vec::with_capacity(members.len());
        for member in members {
            let requested = shares
                .iter()
                .filter(|share| share.cell == *member)
                .map(|share| share.requested.as_i128())
                .sum::<i128>();
            let capacity = state
                .ground(*member)
                .ok_or(HydrologyError::ForcingTargetNotResident)?
                .surface_capacity();
            let held = cell_work(work, *member)
                .expect("a group member is resident")
                .storage
                .surface;
            weights.push(requested);
            ceilings.push(held.remaining_below(capacity).as_i128());
        }
        let candidate = weights.iter().try_fold(0_i128, |total, weight| {
            WaterAccumulator::new(total)
                .add(*weight)
                .map(|sum| sum.get())
        })?;
        let accepted_total = clamp_to_allocatable(candidate, &weights, &ceilings)?;
        let allocated = allocate_capped(accepted_total, &weights, &ceilings)?;

        let mut record_members = Vec::with_capacity(members.len());
        for (member, share) in members.iter().zip(&allocated) {
            let granted = WaterVolume::from_i128(share.granted)?;
            let entry = cell_work(work, *member).expect("a group member is resident");
            let before = entry.storage.surface;
            entry.storage.surface = before.checked_add(granted)?;
            let after = entry.storage.surface;

            let key = grants.source.entry((*scheduled_tick, *forcing_id, *member));
            let running = key.or_insert(WaterVolume::ZERO);
            *running = running.checked_add(granted)?;

            let requested = WaterVolume::from_i128(share.weight)?;
            if !requested.is_zero() || !granted.is_zero() {
                batch
                    .receipts
                    .push(HydrologyTransferReceipt::new(HydrologyTransferParts {
                        batch_sequence: batch.batch_sequence,
                        tick: batch.tick,
                        process_kind: *process_kind,
                        source: HydrologyCarrierKey::ForcingRecord {
                            scheduled_tick: *scheduled_tick,
                            forcing_id: *forcing_id,
                        },
                        source_bucket: HydrologyBucket::External,
                        target: HydrologyCarrierKey::Cell(*member),
                        target_bucket: HydrologyBucket::Surface,
                        requested,
                        accepted: granted,
                        source_before: WaterVolume::ZERO,
                        source_after: WaterVolume::ZERO,
                        target_before: before,
                        target_after: after,
                        causal_parents: Vec::new(),
                        forcing_origin: None,
                        transfer_event: None,
                        storage_event: (!granted.is_zero())
                            .then(|| settlement_key(*member))
                            .transpose()?,
                    })?);
            }
            match *process_kind {
                process::PRECIPITATION => {
                    batch.accepted_precipitation =
                        batch.accepted_precipitation.add_volume(granted)?;
                }
                _ => {
                    batch.accepted_external_inflow =
                        batch.accepted_external_inflow.add_volume(granted)?;
                }
            }
            record_members.push(HydrologyCoarseMember {
                cell: *member,
                weight: share.weight,
                ceiling: share.ceiling,
                granted: share.granted,
                references: vec![
                    cell_work(work, *member)
                        .expect("a group member is resident")
                        .surface_ref
                        .clone(),
                ],
            });
        }
        coarse.push(HydrologyCoarseProcess {
            tick: batch.tick,
            block: *block,
            constitutive: *constitutive,
            substage_ordinal: substage::FORCING,
            process_kind: *process_kind,
            forcing: Some((*scheduled_tick, *forcing_id)),
            raw_candidate: candidate,
            summed_ceilings: ceilings.iter().sum(),
            accepted_total,
            members: record_members,
        });
    }
    Ok(())
}

fn substage_infiltration(
    batch: &mut Batch,
    work: &mut BTreeMap<ChartChunkCoord, Vec<CellWork>>,
    state: &HydrologyFieldSet,
    plan: &HydrologyResolutionPlan,
) -> Result<(), HydrologyError> {
    for (chunk, field) in state.fields() {
        if plan.level(*chunk) > 0 {
            continue;
        }
        for (ordinal, ground) in field.substrate().iter().enumerate() {
            let cell = HydrologyCellKey::new(*chunk, ordinal as u16)?;
            let entry = cell_work(work, cell).expect("the work map mirrors the field set");

            // What the process wants, before the receiving bucket is consulted.
            let requested = entry
                .storage
                .surface
                .min(ground.infiltration_limit_per_tick());
            if requested.is_zero() {
                continue;
            }
            let room = entry.storage.soil.remaining_below(ground.soil_capacity());
            let accepted = requested.min(room);
            let event = vertical_key(substage::INFILTRATION, process::INFILTRATION, cell)?;

            let surface_before = entry.storage.surface;
            let soil_before = entry.storage.soil;
            entry.storage.surface = surface_before.checked_sub(accepted)?;
            entry.storage.soil = soil_before.checked_add(accepted)?;

            batch
                .receipts
                .push(HydrologyTransferReceipt::new(HydrologyTransferParts {
                    batch_sequence: batch.batch_sequence,
                    tick: batch.tick,
                    process_kind: process::INFILTRATION,
                    source: HydrologyCarrierKey::Cell(cell),
                    source_bucket: HydrologyBucket::Surface,
                    target: HydrologyCarrierKey::Cell(cell),
                    target_bucket: HydrologyBucket::Soil,
                    requested,
                    accepted,
                    source_before: surface_before,
                    source_after: entry.storage.surface,
                    target_before: soil_before,
                    target_after: entry.storage.soil,
                    causal_parents: Vec::new(),
                    forcing_origin: None,
                    transfer_event: (!accepted.is_zero()).then(|| event.clone()),
                    storage_event: (!accepted.is_zero()).then(|| event.clone()),
                })?);

            if accepted.is_zero() {
                continue;
            }
            // Cites the forcing settlement when this cell had one, because that
            // is the event that produced the surface water being infiltrated;
            // otherwise the surface bucket's own prior trace.
            let surface_source = entry
                .forcing_settlement
                .clone()
                .map(CausalEventDagCause::Local)
                .unwrap_or_else(|| entry.surface_ref.clone());
            emit_vertical_event(
                batch,
                entry,
                cell,
                VerticalEvent {
                    substage_ordinal: substage::INFILTRATION,
                    process_kind: process::INFILTRATION,
                    causes: vec![surface_source, entry.soil_ref.clone()],
                    from: (HydrologyProperty::Surface, surface_before),
                    to: (HydrologyProperty::Soil, soil_before),
                },
            )?;
        }
    }
    Ok(())
}

fn substage_percolation(
    batch: &mut Batch,
    work: &mut BTreeMap<ChartChunkCoord, Vec<CellWork>>,
    state: &HydrologyFieldSet,
    plan: &HydrologyResolutionPlan,
) -> Result<(), HydrologyError> {
    for (chunk, field) in state.fields() {
        if plan.level(*chunk) > 0 {
            continue;
        }
        for (ordinal, ground) in field.substrate().iter().enumerate() {
            let cell = HydrologyCellKey::new(*chunk, ordinal as u16)?;
            let entry = cell_work(work, cell).expect("the work map mirrors the field set");

            let requested = ground
                .percolation_fraction()
                .apply_to_volume(entry.storage.soil)?;
            if requested.is_zero() {
                continue;
            }
            let room = entry
                .storage
                .groundwater
                .remaining_below(ground.groundwater_capacity());
            let accepted = requested.min(room);
            let event = vertical_key(substage::PERCOLATION, process::PERCOLATION, cell)?;

            let soil_before = entry.storage.soil;
            let groundwater_before = entry.storage.groundwater;
            entry.storage.soil = soil_before.checked_sub(accepted)?;
            entry.storage.groundwater = groundwater_before.checked_add(accepted)?;

            batch
                .receipts
                .push(HydrologyTransferReceipt::new(HydrologyTransferParts {
                    batch_sequence: batch.batch_sequence,
                    tick: batch.tick,
                    process_kind: process::PERCOLATION,
                    source: HydrologyCarrierKey::Cell(cell),
                    source_bucket: HydrologyBucket::Soil,
                    target: HydrologyCarrierKey::Cell(cell),
                    target_bucket: HydrologyBucket::Groundwater,
                    requested,
                    accepted,
                    source_before: soil_before,
                    source_after: entry.storage.soil,
                    target_before: groundwater_before,
                    target_after: entry.storage.groundwater,
                    causal_parents: Vec::new(),
                    forcing_origin: None,
                    transfer_event: (!accepted.is_zero()).then(|| event.clone()),
                    storage_event: (!accepted.is_zero()).then(|| event.clone()),
                })?);

            if accepted.is_zero() {
                continue;
            }
            emit_vertical_event(
                batch,
                entry,
                cell,
                VerticalEvent {
                    substage_ordinal: substage::PERCOLATION,
                    process_kind: process::PERCOLATION,
                    causes: vec![entry.soil_ref.clone(), entry.groundwater_ref.clone()],
                    from: (HydrologyProperty::Soil, soil_before),
                    to: (HydrologyProperty::Groundwater, groundwater_before),
                },
            )?;
        }
    }
    Ok(())
}

/// One coarse within-cell transfer: infiltration or percolation over a group.
struct CoarseVertical {
    substage_ordinal: u8,
    process_kind: u32,
    from: HydrologyProperty,
    to: HydrologyProperty,
}

const COARSE_INFILTRATION: CoarseVertical = CoarseVertical {
    substage_ordinal: substage::INFILTRATION,
    process_kind: process::INFILTRATION,
    from: HydrologyProperty::Surface,
    to: HydrologyProperty::Soil,
};

const COARSE_PERCOLATION: CoarseVertical = CoarseVertical {
    substage_ordinal: substage::PERCOLATION,
    process_kind: process::PERCOLATION,
    from: HydrologyProperty::Soil,
    to: HydrologyProperty::Groundwater,
};

/// Run one coarse within-cell transfer over every constitutive group.
///
/// The group's raw candidate comes from the Section 5 equation applied to the
/// group's totals, and each member's weight and ceiling come from the same
/// equation applied to that member — so the aggregate can round to a unit no
/// single member could receive, which `clamp_to_ceilings` is what stops from
/// becoming an invented transfer.
fn coarse_vertical_pass(
    batch: &mut Batch,
    work: &mut BTreeMap<ChartChunkCoord, Vec<CellWork>>,
    state: &HydrologyFieldSet,
    plan: &HydrologyResolutionPlan,
    coarse: &mut Vec<HydrologyCoarseProcess>,
    spec: &CoarseVertical,
) -> Result<(), HydrologyError> {
    for ((block, constitutive), members) in plan.groups() {
        let mut weights = Vec::with_capacity(members.len());
        let mut ceilings = Vec::with_capacity(members.len());
        let mut group_from = WaterAccumulator::ZERO;
        let mut group_limit = WaterAccumulator::ZERO;
        let mut group_room = WaterAccumulator::ZERO;
        let mut fraction = None;

        for member in members {
            let ground = state
                .ground(*member)
                .ok_or(HydrologyError::ForcingTargetNotResident)?;
            let entry = cell_work(work, *member).expect("a group member is resident");
            let held = bucket_value(&entry.storage, spec.from);
            let room = bucket_value(&entry.storage, spec.to)
                .remaining_below(bucket_capacity(ground, spec.to));
            let (weight, ceiling) = match spec.process_kind {
                process::INFILTRATION => {
                    let limit = ground.infiltration_limit_per_tick();
                    group_limit = group_limit.add_volume(limit)?;
                    let candidate = held.min(limit).min(room);
                    (candidate.as_i128(), candidate.as_i128())
                }
                _ => {
                    // All members of a group share one substrate, so one fraction.
                    fraction = Some(ground.percolation_fraction());
                    let raw = ground.percolation_fraction().apply_to_volume(held)?;
                    (raw.as_i128(), raw.min(room).as_i128())
                }
            };
            group_from = group_from.add_volume(held)?;
            group_room = group_room.add_volume(room)?;
            weights.push(weight);
            ceilings.push(ceiling);
        }

        let candidate = match spec.process_kind {
            process::INFILTRATION => group_from
                .get()
                .min(group_limit.get())
                .min(group_room.get()),
            _ => match fraction {
                Some(fraction) => fraction.apply_floor(group_from.get())?,
                None => 0,
            },
        };
        let accepted_total = clamp_to_allocatable(candidate, &weights, &ceilings)?;
        let allocated = allocate_capped(accepted_total, &weights, &ceilings)?;

        let mut record_members = Vec::with_capacity(members.len());
        for (member, share) in members.iter().zip(&allocated) {
            let granted = WaterVolume::from_i128(share.granted)?;
            let requested = WaterVolume::from_i128(share.weight)?;
            let entry = cell_work(work, *member).expect("a group member is resident");
            let from_before = bucket_value(&entry.storage, spec.from);
            let to_before = bucket_value(&entry.storage, spec.to);
            *bucket_slot(&mut entry.storage, spec.from) = from_before.checked_sub(granted)?;
            *bucket_slot(&mut entry.storage, spec.to) = to_before.checked_add(granted)?;
            let event = vertical_key(spec.substage_ordinal, spec.process_kind, *member)?;

            record_members.push(HydrologyCoarseMember {
                cell: *member,
                weight: share.weight,
                ceiling: share.ceiling,
                granted: share.granted,
                references: vec![
                    cell_bucket_reference(entry, spec.from),
                    cell_bucket_reference(entry, spec.to),
                ],
            });

            if requested.is_zero() && granted.is_zero() {
                continue;
            }
            batch
                .receipts
                .push(HydrologyTransferReceipt::new(HydrologyTransferParts {
                    batch_sequence: batch.batch_sequence,
                    tick: batch.tick,
                    process_kind: spec.process_kind,
                    source: HydrologyCarrierKey::Cell(*member),
                    source_bucket: spec.from.bucket(),
                    target: HydrologyCarrierKey::Cell(*member),
                    target_bucket: spec.to.bucket(),
                    requested,
                    accepted: granted,
                    source_before: from_before,
                    source_after: bucket_value(&entry.storage, spec.from),
                    target_before: to_before,
                    target_after: bucket_value(&entry.storage, spec.to),
                    causal_parents: Vec::new(),
                    forcing_origin: None,
                    transfer_event: (!granted.is_zero()).then(|| event.clone()),
                    storage_event: (!granted.is_zero()).then(|| event.clone()),
                })?);
            if granted.is_zero() {
                continue;
            }
            let causes = vec![
                cell_bucket_reference(entry, spec.from),
                cell_bucket_reference(entry, spec.to),
            ];
            emit_coarse_allocation(
                batch,
                entry,
                *member,
                VerticalEvent {
                    substage_ordinal: spec.substage_ordinal,
                    process_kind: spec.process_kind,
                    causes,
                    from: (spec.from, from_before),
                    to: (spec.to, to_before),
                },
                coarse.len(),
            )?;
        }
        coarse.push(HydrologyCoarseProcess {
            tick: batch.tick,
            block: *block,
            constitutive: *constitutive,
            substage_ordinal: spec.substage_ordinal,
            process_kind: spec.process_kind,
            forcing: None,
            raw_candidate: candidate,
            summed_ceilings: ceilings.iter().sum(),
            accepted_total,
            members: record_members,
        });
    }
    Ok(())
}

fn substage_evapotranspiration(
    batch: &mut Batch,
    work: &mut BTreeMap<ChartChunkCoord, Vec<CellWork>>,
    settlements: &mut BTreeMap<HydrologyCellKey, SettlementWork>,
    plan: &HydrologyResolutionPlan,
) -> Result<(), HydrologyError> {
    for (cell, settlement) in settlements.iter_mut() {
        if plan.is_coarse(*cell) {
            continue;
        }
        let entry = cell_work(work, *cell).expect("a settled cell is resident");
        let demand = entry.et_demand;
        if demand.is_zero() {
            continue;
        }

        // Surface first, then soil. Groundwater is never withdrawn directly in
        // this tranche: reaching it would need a root-zone model that does not
        // exist, and taking it anyway would be an invented process.
        let surface_before = entry.storage.surface;
        let surface_taken = demand.min(surface_before);
        entry.storage.surface = surface_before.checked_sub(surface_taken)?;

        let remaining = demand.checked_sub(surface_taken)?;
        let soil_before = entry.storage.soil;
        let soil_taken = remaining.min(soil_before);
        entry.storage.soil = soil_before.checked_sub(soil_taken)?;

        let accepted = surface_taken.checked_add(soil_taken)?;
        let unmet = demand.checked_sub(accepted)?;
        let key = vertical_key(
            substage::EVAPOTRANSPIRATION,
            process::EVAPOTRANSPIRATION,
            *cell,
        )?;

        push_et_receipt(
            batch,
            EtReceipt {
                cell: *cell,
                process_kind: process::EVAPOTRANSPIRATION_SURFACE,
                bucket: HydrologyBucket::Surface,
                requested: demand,
                accepted: surface_taken,
                before: surface_before,
                after: entry.storage.surface,
                event: key.clone(),
            },
        )?;
        push_et_receipt(
            batch,
            EtReceipt {
                cell: *cell,
                process_kind: process::EVAPOTRANSPIRATION_SOIL,
                bucket: HydrologyBucket::Soil,
                requested: remaining,
                accepted: soil_taken,
                before: soil_before,
                after: entry.storage.soil,
                event: key.clone(),
            },
        )?;
        batch.accepted_evapotranspiration =
            batch.accepted_evapotranspiration.add_volume(accepted)?;

        settlement.accepted_et = accepted;
        settlement.unmet_et = unmet;
        // Attribute the met demand back to the records that asked for it, in
        // canonical order, so a receipt stays traceable to its origin without
        // every origin becoming a cause.
        attribute_evapotranspiration(&mut settlement.allocations, accepted)?;

        if accepted.is_zero() {
            continue;
        }

        let mut causes: BTreeSet<CausalEventDagCause> = BTreeSet::new();
        causes.insert(entry.surface_ref.clone());
        causes.insert(entry.soil_ref.clone());
        if let Some(settlement_event) = &entry.forcing_settlement {
            causes.insert(CausalEventDagCause::Local(settlement_event.clone()));
        }

        let mut effects = Vec::new();
        if !surface_taken.is_zero() {
            effects.push(bucket_effect(
                *cell,
                HydrologyProperty::Surface,
                surface_before,
                entry.storage.surface,
            ));
            entry.surface_ref = CausalEventDagCause::Local(key.clone());
            entry.terminal_surface = Some(key.clone());
            batch.cell_changes.push(HydrologyCellChange {
                cell: *cell,
                bucket: HydrologyBucket::Surface,
                before: surface_before,
                after: entry.storage.surface,
                settlement_event: key.clone(),
            });
        }
        if !soil_taken.is_zero() {
            effects.push(bucket_effect(
                *cell,
                HydrologyProperty::Soil,
                soil_before,
                entry.storage.soil,
            ));
            entry.soil_ref = CausalEventDagCause::Local(key.clone());
            entry.terminal_soil = Some(key.clone());
            batch.cell_changes.push(HydrologyCellChange {
                cell: *cell,
                bucket: HydrologyBucket::Soil,
                before: soil_before,
                after: entry.storage.soil,
                settlement_event: key.clone(),
            });
        }
        batch.events.push(HydrologyEventPlan {
            key,
            kind: HydrologyEventKind::CellChange,
            coarse_process: None,
            causes: causes.into_iter().collect(),
            effects,
        });
    }
    Ok(())
}

/// Coarse evapotranspiration: a surface pass then a soil pass, per record and
/// per group, exactly as the plan orders them.
#[allow(clippy::too_many_arguments)]
fn coarse_evapotranspiration(
    batch: &mut Batch,
    work: &mut BTreeMap<ChartChunkCoord, Vec<CellWork>>,
    plan: &HydrologyResolutionPlan,
    settlements: &mut BTreeMap<HydrologyCellKey, SettlementWork>,
    coarse: &mut Vec<HydrologyCoarseProcess>,
) -> Result<(), HydrologyError> {
    // Rebuild the per-record demand queue from the settlements, which already
    // carry every allocated fine ET demand in canonical record order.
    let mut queue: BTreeMap<
        (u64, u64, HydrologyBlockKey, HydrologyConstitutiveKey),
        BTreeMap<HydrologyCellKey, WaterVolume>,
    > = BTreeMap::new();
    for (cell, settlement) in settlements.iter() {
        if !plan.is_coarse(*cell) {
            continue;
        }
        let (block, constitutive) = group_of(plan, *cell)?;
        for allocation in &settlement.allocations {
            if allocation.potential_et.is_zero() {
                continue;
            }
            *queue
                .entry((
                    allocation.scheduled_tick,
                    allocation.forcing_id,
                    block,
                    constitutive,
                ))
                .or_default()
                .entry(*cell)
                .or_insert(WaterVolume::ZERO) = allocation.potential_et;
        }
    }

    let mut met: BTreeMap<(u64, u64, HydrologyCellKey), WaterVolume> = BTreeMap::new();
    // Which event settles each cell's evapotranspiration. One event per cell
    // covers both bucket passes, and finding it by scanning the batch would be
    // quadratic in the number of coarse cells.
    let mut et_events: BTreeMap<HydrologyCellKey, usize> = BTreeMap::new();
    for ((scheduled_tick, forcing_id, block, constitutive), demands) in &queue {
        let members = members_of(plan, *block, *constitutive);
        let mut remaining_demand: Vec<i128> = members
            .iter()
            .map(|member| {
                demands
                    .get(member)
                    .copied()
                    .unwrap_or(WaterVolume::ZERO)
                    .as_i128()
            })
            .collect();

        for (process_kind, property) in [
            (
                process::EVAPOTRANSPIRATION_SURFACE,
                HydrologyProperty::Surface,
            ),
            (process::EVAPOTRANSPIRATION_SOIL, HydrologyProperty::Soil),
        ] {
            let mut ceilings = Vec::with_capacity(members.len());
            for member in members {
                let entry = cell_work(work, *member).expect("a group member is resident");
                ceilings.push(bucket_value(&entry.storage, property).as_i128());
            }
            let candidate = remaining_demand.iter().try_fold(0_i128, |total, demand| {
                WaterAccumulator::new(total)
                    .add(*demand)
                    .map(|sum| sum.get())
            })?;
            let accepted_total = clamp_to_allocatable(candidate, &remaining_demand, &ceilings)?;
            let allocated = allocate_capped(accepted_total, &remaining_demand, &ceilings)?;

            let mut record_members = Vec::with_capacity(members.len());
            for (index, (member, share)) in members.iter().zip(&allocated).enumerate() {
                let granted = WaterVolume::from_i128(share.granted)?;
                let requested = WaterVolume::from_i128(share.weight)?;
                let entry = cell_work(work, *member).expect("a group member is resident");
                let before = bucket_value(&entry.storage, property);
                *bucket_slot(&mut entry.storage, property) = before.checked_sub(granted)?;
                let after = bucket_value(&entry.storage, property);
                remaining_demand[index] -= share.granted;

                let event = vertical_key(
                    substage::EVAPOTRANSPIRATION,
                    process::EVAPOTRANSPIRATION,
                    *member,
                )?;
                record_members.push(HydrologyCoarseMember {
                    cell: *member,
                    weight: share.weight,
                    ceiling: share.ceiling,
                    granted: share.granted,
                    references: vec![cell_bucket_reference(entry, property)],
                });
                if requested.is_zero() && granted.is_zero() {
                    continue;
                }
                push_et_receipt(
                    batch,
                    EtReceipt {
                        cell: *member,
                        process_kind,
                        bucket: property.bucket(),
                        requested,
                        accepted: granted,
                        before,
                        after,
                        event: event.clone(),
                    },
                )?;
                if granted.is_zero() {
                    continue;
                }
                batch.accepted_evapotranspiration =
                    batch.accepted_evapotranspiration.add_volume(granted)?;
                let running = met
                    .entry((*scheduled_tick, *forcing_id, *member))
                    .or_insert(WaterVolume::ZERO);
                *running = running.checked_add(granted)?;

                let causes = vec![cell_bucket_reference(entry, property)];
                emit_coarse_withdrawal(
                    batch,
                    &mut et_events,
                    entry,
                    *member,
                    property,
                    before,
                    after,
                    event,
                    causes,
                    coarse.len(),
                )?;
            }
            coarse.push(HydrologyCoarseProcess {
                tick: batch.tick,
                block: *block,
                constitutive: *constitutive,
                substage_ordinal: substage::EVAPOTRANSPIRATION,
                process_kind,
                forcing: Some((*scheduled_tick, *forcing_id)),
                raw_candidate: candidate,
                summed_ceilings: ceilings.iter().sum(),
                accepted_total,
                members: record_members,
            });
        }
    }

    for (cell, settlement) in settlements.iter_mut() {
        if !plan.is_coarse(*cell) {
            continue;
        }
        let mut accepted = WaterVolume::ZERO;
        let mut demanded = WaterVolume::ZERO;
        for allocation in &mut settlement.allocations {
            let granted = met
                .get(&(allocation.scheduled_tick, allocation.forcing_id, *cell))
                .copied()
                .unwrap_or(WaterVolume::ZERO);
            allocation.accepted_et = granted;
            accepted = accepted.checked_add(granted)?;
            demanded = demanded.checked_add(allocation.potential_et)?;
        }
        settlement.accepted_et = accepted;
        settlement.unmet_et = demanded.checked_sub(accepted)?;
    }
    Ok(())
}

/// Emit one fine allocation event for a coarse within-cell transfer.
fn emit_coarse_allocation(
    batch: &mut Batch,
    entry: &mut CellWork,
    cell: HydrologyCellKey,
    event: VerticalEvent,
    coarse_index: usize,
) -> Result<(), HydrologyError> {
    emit_vertical_event(batch, entry, cell, event)?;
    if let Some(last) = batch.events.last_mut() {
        last.coarse_process = Some(coarse_index);
    }
    Ok(())
}

/// Emit one fine allocation event for a coarse withdrawal to the outside world.
#[allow(clippy::too_many_arguments)]
fn emit_coarse_withdrawal(
    batch: &mut Batch,
    index: &mut BTreeMap<HydrologyCellKey, usize>,
    entry: &mut CellWork,
    cell: HydrologyCellKey,
    property: HydrologyProperty,
    before: WaterVolume,
    after: WaterVolume,
    key: CausalEventProposalKey,
    causes: Vec<CausalEventDagCause>,
    coarse_index: usize,
) -> Result<(), HydrologyError> {
    // One event per cell settles whichever buckets the two ET passes drew from,
    // so a second pass extends the existing event rather than adding one that
    // would claim a separate change to the same bucket.
    if let Some(existing) = index.get(&cell).map(|at| &mut batch.events[*at]) {
        if let Some(effect) = existing
            .effects
            .iter_mut()
            .find(|effect| effect.property == property)
        {
            effect.after = volume_fingerprint(&HydrologyCarrierKey::Cell(cell), property, after);
        } else {
            existing
                .effects
                .push(bucket_effect(cell, property, before, after));
        }
        for cause in causes {
            if !existing.causes.contains(&cause) {
                existing.causes.push(cause);
            }
        }
        existing.causes.sort();
        existing.coarse_process = Some(coarse_index);
    } else {
        let mut ordered: BTreeSet<CausalEventDagCause> = BTreeSet::new();
        for cause in causes {
            ordered.insert(cause);
        }
        index.insert(cell, batch.events.len());
        batch.events.push(HydrologyEventPlan {
            key: key.clone(),
            kind: HydrologyEventKind::CellChange,
            coarse_process: Some(coarse_index),
            causes: ordered.into_iter().collect(),
            effects: vec![bucket_effect(cell, property, before, after)],
        });
    }
    set_reference(entry, property, key.clone());
    batch.cell_changes.push(HydrologyCellChange {
        cell,
        bucket: property.bucket(),
        before,
        after,
        settlement_event: key,
    });
    Ok(())
}

/// Push every settlement event, now that accepted ET is known.
///
/// The settlement fingerprint covers each allocation's accepted ET, so computing
/// it in substage 1 — before substage 4 has run — would persist a value that a
/// later recomputation from the same allocations could not reproduce. The event's
/// proposal key is deterministic, so substages 2 through 4 can cite it as a local
/// cause before it is pushed; only the fingerprint has to wait.
fn finalise_settlements(
    batch: &mut Batch,
    work: &mut BTreeMap<ChartChunkCoord, Vec<CellWork>>,
    settlements: BTreeMap<HydrologyCellKey, SettlementWork>,
) -> Result<Vec<HydrologyForcingSettlement>, HydrologyError> {
    let mut settled = Vec::with_capacity(settlements.len());
    for (cell, settlement) in settlements {
        let key = settlement_key(cell)?;
        let entry = cell_work(work, cell).expect("a settled cell is resident");
        let before = entry.forcing_fingerprint;
        let after = forcing_settlement_fingerprint(batch.tick, cell, &settlement.allocations);

        let mut causes: BTreeSet<CausalEventDagCause> = BTreeSet::new();
        causes.insert(entry.pre_tick_surface_ref.clone());
        causes.insert(entry.pre_tick_soil_ref.clone());
        for origin in &settlement.origins {
            causes.insert(CausalEventDagCause::Existing(*origin));
        }

        // The settlement always transitions the forcing-input property. It also
        // accounts for the surface water it delivered, because substage 1 is
        // where accepted precipitation and external inflow actually land: with
        // no surface effect here, a cell that was only rained on would end the
        // tick holding more water than it started with while its
        // `surface_last_change` still pointed at a previous tick. The transfer
        // receipts carry that attribution too, but they are evicted after eight
        // batches and the bucket anchor is what has to survive (INV-014).
        let mut effects = vec![HydrologyEventEffect {
            carrier: HydrologyCarrierKey::Cell(cell),
            property: HydrologyProperty::ForcingInput,
            before,
            after,
        }];
        if !settlement.accepted_source.is_zero() {
            effects.push(bucket_effect(
                cell,
                HydrologyProperty::Surface,
                settlement.surface_before,
                settlement.surface_after,
            ));
            batch.cell_changes.push(HydrologyCellChange {
                cell,
                bucket: HydrologyBucket::Surface,
                before: settlement.surface_before,
                after: settlement.surface_after,
                settlement_event: key.clone(),
            });
        }

        batch.events.push(HydrologyEventPlan {
            key: key.clone(),
            kind: HydrologyEventKind::ForcingSettlement,
            coarse_process: None,
            causes: causes.into_iter().collect(),
            effects,
        });
        entry.forcing_fingerprint = after;

        settled.push(HydrologyForcingSettlement {
            cell,
            allocations: settlement.allocations,
            accepted_source: settlement.accepted_source,
            rejected_source: settlement.rejected_source,
            accepted_et: settlement.accepted_et,
            unmet_et: settlement.unmet_et,
            fingerprint_before: before,
            fingerprint_after: after,
            settlement_event: key,
        });
    }
    Ok(settled)
}

fn share_volume(share: u128) -> Result<WaterVolume, HydrologyError> {
    Ok(WaterVolume::from_i128(
        i128::try_from(share).map_err(|_| WaterVolumeError::Overflow)?,
    )?)
}

/// Accept one record's share of one source process into surface storage.
///
/// Capacity-limited rather than clamped: what does not fit is recorded as
/// unaccepted and never enters the world, so the conservation ledger's `sources`
/// term counts only what was actually accepted.
#[allow(clippy::too_many_arguments)]
fn accept_source(
    batch: &mut Batch,
    work: &mut BTreeMap<ChartChunkCoord, Vec<CellWork>>,
    record: &HydrologyForcingRecord,
    cell: HydrologyCellKey,
    capacity: WaterVolume,
    process_kind: u32,
    requested: WaterVolume,
) -> Result<WaterVolume, HydrologyError> {
    if requested.is_zero() {
        return Ok(WaterVolume::ZERO);
    }
    let entry = cell_work(work, cell).expect("residency was validated above");
    let before = entry.storage.surface;
    let accepted = requested.min(before.remaining_below(capacity));
    entry.storage.surface = before.checked_add(accepted)?;
    let after = entry.storage.surface;

    let application = CausalEventProposalKey::new(
        substage::FORCING,
        process::FORCING_APPLICATION,
        &HydrologyCarrierKey::ForcingRecord {
            scheduled_tick: record.scheduled_tick(),
            forcing_id: record.forcing_id(),
        }
        .encode(),
        0,
    )?;
    let settlement = settlement_key(cell)?;

    batch
        .receipts
        .push(HydrologyTransferReceipt::new(HydrologyTransferParts {
            batch_sequence: batch.batch_sequence,
            tick: batch.tick,
            process_kind,
            source: HydrologyCarrierKey::ForcingRecord {
                scheduled_tick: record.scheduled_tick(),
                forcing_id: record.forcing_id(),
            },
            source_bucket: HydrologyBucket::External,
            target: HydrologyCarrierKey::Cell(cell),
            target_bucket: HydrologyBucket::Surface,
            requested,
            accepted,
            source_before: WaterVolume::ZERO,
            source_after: WaterVolume::ZERO,
            target_before: before,
            target_after: after,
            causal_parents: vec![record.origin_trace()],
            forcing_origin: Some(record.origin_trace()),
            transfer_event: Some(application),
            // Only when water actually landed: with nothing accepted the
            // settlement carries no surface effect, so pointing at it would
            // claim a storage change that did not happen.
            storage_event: (!accepted.is_zero()).then_some(settlement),
        })?);

    match process_kind {
        process::PRECIPITATION => {
            batch.accepted_precipitation = batch.accepted_precipitation.add_volume(accepted)?;
        }
        process::EXTERNAL_INFLOW => {
            batch.accepted_external_inflow = batch.accepted_external_inflow.add_volume(accepted)?;
        }
        _ => unreachable!("accept_source is only called for the two source processes"),
    }
    Ok(accepted)
}

/// Spread accepted ET back over the records that demanded it, largest first by
/// canonical order, so every unit is attributable to exactly one record.
fn attribute_evapotranspiration(
    allocations: &mut [HydrologyForcingAllocation],
    accepted: WaterVolume,
) -> Result<(), HydrologyError> {
    let weights: Vec<u128> = allocations
        .iter()
        .map(|allocation| u128::from(allocation.potential_et.get()))
        .collect();
    if weights.iter().all(|weight| *weight == 0) {
        return Ok(());
    }
    let shares = allocate_largest_remainder(u128::from(accepted.get()), &weights)?;
    for (allocation, share) in allocations.iter_mut().zip(shares) {
        allocation.accepted_et = share_volume(share)?;
    }
    Ok(())
}

struct EtReceipt {
    cell: HydrologyCellKey,
    process_kind: u32,
    bucket: HydrologyBucket,
    requested: WaterVolume,
    accepted: WaterVolume,
    before: WaterVolume,
    after: WaterVolume,
    event: CausalEventProposalKey,
}

fn push_et_receipt(batch: &mut Batch, receipt: EtReceipt) -> Result<(), HydrologyError> {
    let EtReceipt {
        cell,
        process_kind,
        bucket,
        requested,
        accepted,
        before,
        after,
        event,
    } = receipt;
    if requested.is_zero() {
        return Ok(());
    }
    // Emitted even when nothing was taken. Unmet demand is the evidence that a
    // limiter engaged, and a receipt that only appeared when water moved would
    // make a dry cell indistinguishable from a cell nobody asked anything of.
    batch
        .receipts
        .push(HydrologyTransferReceipt::new(HydrologyTransferParts {
            batch_sequence: batch.batch_sequence,
            tick: batch.tick,
            process_kind,
            source: HydrologyCarrierKey::Cell(cell),
            source_bucket: bucket,
            target: HydrologyCarrierKey::Cell(cell),
            target_bucket: HydrologyBucket::External,
            requested,
            accepted,
            source_before: before,
            source_after: after,
            target_before: WaterVolume::ZERO,
            target_after: WaterVolume::ZERO,
            causal_parents: Vec::new(),
            forcing_origin: None,
            transfer_event: (!accepted.is_zero()).then(|| event.clone()),
            storage_event: (!accepted.is_zero()).then_some(event),
        })?);
    Ok(())
}

/// The deterministic proposal key of one within-cell process event.
///
/// A pure function of substage, process, and carrier, so a receipt can name the
/// event that will carry it before either has been emitted — and so the runtime
/// can resolve that name to a committed trace after the batch commits. Without
/// it a receipt would be an orphan the moment its transfer trace was asked for.
fn vertical_key(
    substage_ordinal: u8,
    process_kind: u32,
    cell: HydrologyCellKey,
) -> Result<CausalEventProposalKey, HydrologyError> {
    Ok(CausalEventProposalKey::new(
        substage_ordinal,
        process_kind,
        &HydrologyCarrierKey::Cell(cell).encode(),
        0,
    )?)
}

/// The deterministic proposal key of one cell's forcing settlement.
fn settlement_key(cell: HydrologyCellKey) -> Result<CausalEventProposalKey, HydrologyError> {
    vertical_key(substage::FORCING, process::FORCING_SETTLEMENT, cell)
}

struct VerticalEvent {
    substage_ordinal: u8,
    process_kind: u32,
    causes: Vec<CausalEventDagCause>,
    from: (HydrologyProperty, WaterVolume),
    to: (HydrologyProperty, WaterVolume),
}

/// Emit one within-cell transfer event and advance both buckets' references.
fn emit_vertical_event(
    batch: &mut Batch,
    entry: &mut CellWork,
    cell: HydrologyCellKey,
    event: VerticalEvent,
) -> Result<(), HydrologyError> {
    let key = vertical_key(event.substage_ordinal, event.process_kind, cell)?;
    let (from_property, from_before) = event.from;
    let (to_property, to_before) = event.to;
    let from_after = bucket_value(&entry.storage, from_property);
    let to_after = bucket_value(&entry.storage, to_property);

    let mut causes: BTreeSet<CausalEventDagCause> = BTreeSet::new();
    for cause in event.causes {
        causes.insert(cause);
    }
    batch.events.push(HydrologyEventPlan {
        key: key.clone(),
        kind: HydrologyEventKind::CellChange,
        coarse_process: None,
        causes: causes.into_iter().collect(),
        effects: vec![
            bucket_effect(cell, from_property, from_before, from_after),
            bucket_effect(cell, to_property, to_before, to_after),
        ],
    });
    for (property, before, after) in [
        (from_property, from_before, from_after),
        (to_property, to_before, to_after),
    ] {
        set_reference(entry, property, key.clone());
        batch.cell_changes.push(HydrologyCellChange {
            cell,
            bucket: property.bucket(),
            before,
            after,
            settlement_event: key.clone(),
        });
    }
    Ok(())
}

const fn bucket_value(storage: &HydrologyCellStorage, property: HydrologyProperty) -> WaterVolume {
    match property {
        HydrologyProperty::Surface => storage.surface,
        HydrologyProperty::Soil => storage.soil,
        HydrologyProperty::Groundwater => storage.groundwater,
        _ => WaterVolume::ZERO,
    }
}

fn set_reference(entry: &mut CellWork, property: HydrologyProperty, key: CausalEventProposalKey) {
    match property {
        HydrologyProperty::Surface => {
            entry.surface_ref = CausalEventDagCause::Local(key.clone());
            entry.terminal_surface = Some(key);
        }
        HydrologyProperty::Soil => {
            entry.soil_ref = CausalEventDagCause::Local(key.clone());
            entry.terminal_soil = Some(key);
        }
        HydrologyProperty::Groundwater => {
            entry.groundwater_ref = CausalEventDagCause::Local(key.clone());
            entry.terminal_groundwater = Some(key);
        }
        _ => {}
    }
}

fn bucket_effect(
    cell: HydrologyCellKey,
    property: HydrologyProperty,
    before: WaterVolume,
    after: WaterVolume,
) -> HydrologyEventEffect {
    let carrier = HydrologyCarrierKey::Cell(cell);
    HydrologyEventEffect {
        carrier,
        property,
        before: volume_fingerprint(&carrier, property, before),
        after: volume_fingerprint(&carrier, property, after),
    }
}

// ---------------------------------------------------------------------------
// Substages 5-8 — lateral routing, conveyance, and boundary export
// ---------------------------------------------------------------------------

/// Where an accepted lateral transfer lands.
///
/// Ordered, because the receiver reduction groups by destination and the plan's
/// tie-break is `(receiver_key, donor_key, edge_key)`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum FlowTarget {
    /// The neighbour's own bucket of the same kind.
    Cell(HydrologyCellKey),
    /// A conveyance edge's storage.
    Edge(HydrologyEdgeKey),
    /// Outside the modelled world, through an open boundary channel.
    Boundary(HydrologyExteriorFaceKey),
}

/// One directed demand across one face, from raw demand to applied transfer.
#[derive(Clone, Debug)]
struct Outflow {
    donor: HydrologyCellKey,
    /// The face crossed. Interior faces sort before exterior ones because the
    /// carrier variants do, and within each group the order is the plan's
    /// ascending canonical key.
    face: HydrologyCarrierKey,
    target: FlowTarget,
    process_kind: u32,
    raw: u128,
    accepted: u128,
    source_before: WaterVolume,
    source_after: WaterVolume,
    target_before: WaterVolume,
    target_after: WaterVolume,
}

impl Outflow {
    /// The canonical order competing demands are reduced and applied in.
    ///
    /// `(donor, face)` is the plan's rule. `process_kind` breaks the one tie it
    /// leaves open: a cell's groundwater lateral outflow and its baseflow can
    /// cross the same face, and two demands with one name would make their
    /// reduction order an accident of insertion.
    fn order(&self) -> (HydrologyCellKey, HydrologyCarrierKey, u32) {
        (self.donor, self.face, self.process_kind)
    }
}

/// Which of the two lateral channels one routing pass moves.
struct RoutingChannel {
    substage_ordinal: u8,
    /// The per-cell settlement event's process identity.
    settlement_process: u32,
    /// A cell-to-cell transfer of this channel's bucket.
    lateral_process: u32,
    /// A transfer of this channel's bucket into a conveyance edge.
    edge_process: u32,
    /// An export through this channel of an open boundary face.
    boundary_process: u32,
    property: HydrologyProperty,
    /// Whether this channel also offers water to a cell's outgoing edge
    /// independently of head, which is what baseflow is.
    baseflow: bool,
}

const SURFACE_CHANNEL: RoutingChannel = RoutingChannel {
    substage_ordinal: substage::SURFACE_ROUTING,
    settlement_process: process::SURFACE_ROUTING,
    lateral_process: process::SURFACE_LATERAL,
    edge_process: process::CONVEYANCE_INFLOW,
    boundary_process: process::SURFACE_BOUNDARY_EXPORT,
    property: HydrologyProperty::Surface,
    baseflow: false,
};

const GROUNDWATER_CHANNEL: RoutingChannel = RoutingChannel {
    substage_ordinal: substage::GROUNDWATER_ROUTING,
    settlement_process: process::GROUNDWATER_ROUTING,
    lateral_process: process::GROUNDWATER_LATERAL,
    edge_process: process::BASEFLOW,
    boundary_process: process::GROUNDWATER_BOUNDARY_EXPORT,
    property: HydrologyProperty::Groundwater,
    baseflow: true,
};

impl RoutingChannel {
    const fn bucket(&self) -> HydrologyBucket {
        self.property.bucket()
    }

    const fn conductance(&self, ground: &HydraulicSubstrateCell) -> u64 {
        match self.property {
            HydrologyProperty::Groundwater => ground.groundwater_conductance_mm2_per_tick(),
            _ => ground.surface_conductance_mm2_per_tick(),
        }
    }

    const fn boundary(&self, condition: &HydrologyBoundaryCondition) -> FluxBoundary {
        match self.property {
            HydrologyProperty::Groundwater => condition.groundwater,
            _ => condition.surface,
        }
    }

    /// The head this channel drives flow with, in millimetres.
    ///
    /// Surface head is the ponded water's own top surface; groundwater head is
    /// the water table implied by the stored volume and the specific yield. Both
    /// are measured against the same absolute reference as terrain, so a face
    /// between them is a comparison of two elevations and not of two volumes.
    fn head(
        &self,
        metric: HydrologyGridMetric,
        ground: &HydraulicSubstrateCell,
        elevation_mm: i32,
        value: WaterVolume,
    ) -> Result<i128, HydrologyError> {
        match self.property {
            HydrologyProperty::Groundwater => {
                if ground.groundwater_capacity().is_zero() && value.is_zero() {
                    // No aquifer and nothing in it: the head is the base, and
                    // nothing can flow because the conductance rule needs both
                    // endpoints anyway.
                    return Ok(i128::from(ground.aquifer_base_elevation_mm()));
                }
                let yield_fraction = ground.specific_yield();
                if yield_fraction.numerator() == 0 {
                    // Defence in depth with no reachable case: the substrate
                    // constructor refuses a zero yield over real capacity, and
                    // the field constructor refuses storage above capacity, so
                    // stored groundwater always arrives with a yield. Kept as
                    // the divisor's precondition rather than as an assumption.
                    return Err(HydrologyError::GroundwaterWithoutSpecificYield);
                }
                let scaled = checked_water_mul(
                    value.as_i128(),
                    i128::from(yield_fraction.denominator().get()),
                )?;
                let divisor = checked_water_mul(
                    i128::from(metric.cell_area_mm2().get()),
                    i128::from(yield_fraction.numerator()),
                )?;
                let saturated_depth = causafera_types::checked_water_div_floor(scaled, divisor)?;
                Ok(
                    WaterAccumulator::new(i128::from(ground.aquifer_base_elevation_mm()))
                        .add(saturated_depth)?
                        .get(),
                )
            }
            _ => Ok(WaterAccumulator::new(i128::from(elevation_mm))
                .add(metric.depth_of(value)?.as_i128())?
                .get()),
        }
    }
}

/// `floor(2 * a * b / (a + b))`, and zero when either endpoint cannot conduct.
///
/// Symmetric, so a face has one conductance whichever side asks. A zero endpoint
/// gives zero rather than the harmonic limit, because ground that cannot pass
/// water does not become passable by being next to ground that can.
fn harmonic_face_conductance(a: u64, b: u64) -> Result<i128, HydrologyError> {
    if a == 0 || b == 0 {
        return Ok(0);
    }
    let product = checked_water_mul(2, checked_water_mul(i128::from(a), i128::from(b))?)?;
    let sum = WaterAccumulator::new(i128::from(a))
        .add(i128::from(b))?
        .get();
    Ok(causafera_types::checked_water_div_floor(product, sum)?)
}

const fn bucket_slot(
    storage: &mut HydrologyCellStorage,
    property: HydrologyProperty,
) -> &mut WaterVolume {
    match property {
        HydrologyProperty::Soil => &mut storage.soil,
        HydrologyProperty::Groundwater => &mut storage.groundwater,
        _ => &mut storage.surface,
    }
}

const fn bucket_capacity(
    ground: &HydraulicSubstrateCell,
    property: HydrologyProperty,
) -> WaterVolume {
    match property {
        HydrologyProperty::Soil => ground.soil_capacity(),
        HydrologyProperty::Groundwater => ground.groundwater_capacity(),
        _ => ground.surface_capacity(),
    }
}

fn cell_bucket_reference(entry: &CellWork, property: HydrologyProperty) -> CausalEventDagCause {
    match property {
        HydrologyProperty::Soil => entry.soil_ref.clone(),
        HydrologyProperty::Groundwater => entry.groundwater_ref.clone(),
        _ => entry.surface_ref.clone(),
    }
}

/// One lateral routing pass: substage 5 for surface water, substage 6 for
/// groundwater and baseflow.
///
/// Every demand is computed from one frozen state before any of it is applied,
/// so no cell observes another cell's write within the pass. That single property
/// is what makes a chunk seam an ordinary face, a three-cell chain unable to move
/// one unit through two faces, and the result independent of the order the cells
/// happen to be visited in.
#[allow(clippy::too_many_arguments)]
fn route(
    batch: &mut Batch,
    work: &mut BTreeMap<ChartChunkCoord, Vec<CellWork>>,
    edges: &mut BTreeMap<HydrologyEdgeKey, EdgeWork>,
    state: &HydrologyFieldSet,
    request: &HydrologyEvolutionRequest<'_>,
    plan: &HydrologyResolutionPlan,
    channel: &RoutingChannel,
) -> Result<(), HydrologyError> {
    let mut heads: BTreeMap<HydrologyCellKey, i128> = BTreeMap::new();
    let mut frozen: BTreeMap<HydrologyCellKey, WaterVolume> = BTreeMap::new();
    for (chunk, field) in state.fields() {
        let metric = request.metrics.get(chunk.chart)?;
        let terrain = request
            .terrain
            .get(chunk)
            .ok_or(HydrologyError::TerrainMissing)?;
        let entries = &work[chunk];
        for (ordinal, ground) in field.substrate().iter().enumerate() {
            let cell = HydrologyCellKey::new(*chunk, ordinal as u16)?;
            let value = bucket_value(&entries[ordinal].storage, channel.property);
            frozen.insert(cell, value);
            heads.insert(
                cell,
                channel.head(
                    metric,
                    ground,
                    terrain.elevations()[ordinal].millimetres(),
                    value,
                )?,
            );
        }
    }

    let mut outflows = collect_demands(state, request, plan, channel, &heads, &frozen, edges)?;
    outflows.sort_by_key(Outflow::order);
    reduce_donors(&mut outflows, &frozen)?;
    reduce_receivers(&mut outflows, state, edges, channel, &frozen)?;
    apply_transfers(batch, work, edges, &mut outflows, channel)?;
    emit_routing_events(batch, work, edges, request, channel, &outflows, &frozen)?;
    Ok(())
}

/// Every raw demand of one channel, computed from the frozen state.
#[allow(clippy::too_many_arguments)]
fn collect_demands(
    state: &HydrologyFieldSet,
    request: &HydrologyEvolutionRequest<'_>,
    plan: &HydrologyResolutionPlan,
    channel: &RoutingChannel,
    heads: &BTreeMap<HydrologyCellKey, i128>,
    frozen: &BTreeMap<HydrologyCellKey, WaterVolume>,
    edges: &BTreeMap<HydrologyEdgeKey, EdgeWork>,
) -> Result<Vec<Outflow>, HydrologyError> {
    let mut outflows = Vec::new();
    for (chunk, field) in state.fields() {
        for (ordinal, ground) in field.substrate().iter().enumerate() {
            let cell = HydrologyCellKey::new(*chunk, ordinal as u16)?;
            let head = heads[&cell];
            for direction in FaceDirection::ALL {
                match cell.neighbor(direction) {
                    Some(neighbor) if state.is_resident(neighbor) => {
                        let edge_key = HydrologyEdgeKey::new(cell, neighbor)?;
                        // The canonical pair is processed exactly once, from its
                        // low endpoint. Visiting it from both sides would give
                        // one face two demands and move the water twice.
                        if cell != edge_key.low() {
                            continue;
                        }
                        // A face internal to one coarse block is not evaluated:
                        // that is where the work reduction comes from. Every
                        // other face — including every block boundary and every
                        // face touching a level-zero cell — stays authoritative
                        // and is computed from its own fine endpoints, so
                        // heterogeneous boundary conductance is never averaged.
                        if plan.is_internal_face(cell, neighbor)? {
                            continue;
                        }
                        let neighbor_head = heads[&neighbor];
                        if neighbor_head == head {
                            continue;
                        }
                        let (donor, receiver) = if head > neighbor_head {
                            (cell, neighbor)
                        } else {
                            (neighbor, cell)
                        };
                        let neighbor_ground = state
                            .ground(neighbor)
                            .ok_or(HydrologyError::ForcingTargetNotResident)?;
                        let conductance = harmonic_face_conductance(
                            channel.conductance(ground),
                            channel.conductance(neighbor_ground),
                        )?;
                        if conductance == 0 {
                            continue;
                        }
                        let drop = WaterAccumulator::new(heads[&donor])
                            .sub(heads[&receiver])?
                            .get();
                        let raw = u128::try_from(checked_water_mul(conductance, drop)?)
                            .map_err(|_| WaterVolumeError::Overflow)?;
                        if raw == 0 {
                            continue;
                        }
                        // Surface water enters the face's conveyance edge only
                        // when that edge is directed the way the head already
                        // points. A reverse-head transfer takes the ordinary
                        // surface path rather than reversing a directed channel.
                        let target = match edges.get(&edge_key) {
                            Some(edge) if !channel.baseflow && edge.source == donor => {
                                FlowTarget::Edge(edge_key)
                            }
                            _ => FlowTarget::Cell(receiver),
                        };
                        let process_kind = match target {
                            FlowTarget::Edge(_) => channel.edge_process,
                            _ => channel.lateral_process,
                        };
                        outflows.push(Outflow {
                            donor,
                            face: HydrologyCarrierKey::Edge(edge_key),
                            target,
                            process_kind,
                            raw,
                            accepted: 0,
                            source_before: WaterVolume::ZERO,
                            source_after: WaterVolume::ZERO,
                            target_before: WaterVolume::ZERO,
                            target_after: WaterVolume::ZERO,
                        });
                    }
                    // No resident neighbour: an exterior face, which must carry
                    // an explicit boundary record. Neither exporting nor
                    // blocking may be assumed here (V13).
                    _ => {
                        let face = HydrologyExteriorFaceKey::new(cell, direction);
                        let condition = request
                            .boundaries
                            .get(face)
                            .ok_or(HydrologyError::UnspecifiedBoundaryFace)?;
                        let FluxBoundary::Open {
                            external_head_mm,
                            conductance_mm2_per_tick,
                        } = channel.boundary(&condition)
                        else {
                            continue;
                        };
                        if conductance_mm2_per_tick == 0 {
                            continue;
                        }
                        let drop = WaterAccumulator::new(head)
                            .sub(i128::from(external_head_mm))?
                            .get();
                        if drop <= 0 {
                            continue;
                        }
                        let raw = u128::try_from(checked_water_mul(
                            i128::from(conductance_mm2_per_tick),
                            drop,
                        )?)
                        .map_err(|_| WaterVolumeError::Overflow)?;
                        if raw == 0 {
                            continue;
                        }
                        outflows.push(Outflow {
                            donor: cell,
                            face: HydrologyCarrierKey::ExteriorFace(face),
                            target: FlowTarget::Boundary(face),
                            process_kind: channel.boundary_process,
                            raw,
                            accepted: 0,
                            source_before: WaterVolume::ZERO,
                            source_after: WaterVolume::ZERO,
                            target_before: WaterVolume::ZERO,
                            target_after: WaterVolume::ZERO,
                        });
                    }
                }
            }

            if !channel.baseflow {
                continue;
            }
            // Baseflow does not read head: it is the aquifer draining into the
            // one channel that leaves the cell. A cell with no outgoing edge
            // retains its groundwater.
            let stored = frozen[&cell];
            let threshold = ground.baseflow_threshold();
            let excess = if stored > threshold {
                stored.checked_sub(threshold)?
            } else {
                WaterVolume::ZERO
            };
            let requested = ground.baseflow_fraction().apply_to_volume(excess)?;
            if requested.is_zero() {
                continue;
            }
            let Some(edge) = request.conveyance.outgoing(cell) else {
                continue;
            };
            outflows.push(Outflow {
                donor: cell,
                face: HydrologyCarrierKey::Edge(edge.key()),
                target: FlowTarget::Edge(edge.key()),
                process_kind: channel.edge_process,
                raw: u128::from(requested.get()),
                accepted: 0,
                source_before: WaterVolume::ZERO,
                source_after: WaterVolume::ZERO,
                target_before: WaterVolume::ZERO,
                target_after: WaterVolume::ZERO,
            });
        }
    }
    Ok(outflows)
}

/// Scale every donor's demands down to what the donor actually owns.
///
/// A donor whose faces jointly ask for more than it holds cannot pay them all.
/// Reducing them proportionally by the largest-remainder rule makes the accepted
/// total exactly the available volume — not one unit more, which would be a
/// source, and not one less, which would be a quantisation sink.
fn reduce_donors(
    outflows: &mut [Outflow],
    frozen: &BTreeMap<HydrologyCellKey, WaterVolume>,
) -> Result<(), HydrologyError> {
    let mut index = 0;
    while index < outflows.len() {
        let donor = outflows[index].donor;
        let end = index + outflows[index..].partition_point(|flow| flow.donor == donor);
        let available = u128::from(frozen[&donor].get());
        let raws: Vec<u128> = outflows[index..end].iter().map(|flow| flow.raw).collect();
        let mut total = 0_u128;
        for raw in &raws {
            total = total.checked_add(*raw).ok_or(WaterVolumeError::Overflow)?;
        }
        if total > available {
            let shares = allocate_largest_remainder(available, &raws)?;
            for (flow, share) in outflows[index..end].iter_mut().zip(shares) {
                flow.accepted = share;
            }
        } else {
            for flow in outflows[index..end].iter_mut() {
                flow.accepted = flow.raw;
            }
        }
        index = end;
    }
    Ok(())
}

/// Scale every receiver's already donor-limited inflows down to what it can hold.
///
/// Receiver capacity is read from the frozen state, not from the receiver's
/// post-outflow volume: a receiver that also donates would otherwise accept more
/// or less depending on whether its own outflow had been applied yet, which is
/// exactly the order dependence the frozen substage exists to prevent. Because
/// this pass only lowers already-bounded numbers, one donor pass and one receiver
/// pass satisfy both constraints without iterating.
fn reduce_receivers(
    outflows: &mut [Outflow],
    state: &HydrologyFieldSet,
    edges: &BTreeMap<HydrologyEdgeKey, EdgeWork>,
    channel: &RoutingChannel,
    frozen: &BTreeMap<HydrologyCellKey, WaterVolume>,
) -> Result<(), HydrologyError> {
    let mut grouped: BTreeMap<FlowTarget, Vec<usize>> = BTreeMap::new();
    for (index, flow) in outflows.iter().enumerate() {
        if matches!(flow.target, FlowTarget::Boundary(_)) {
            continue;
        }
        grouped.entry(flow.target).or_default().push(index);
    }

    for (target, indices) in grouped {
        let room = match target {
            FlowTarget::Cell(cell) => {
                let ground = state
                    .ground(cell)
                    .ok_or(HydrologyError::ForcingTargetNotResident)?;
                u128::from(
                    frozen[&cell]
                        .remaining_below(bucket_capacity(ground, channel.property))
                        .get(),
                )
            }
            FlowTarget::Edge(key) => {
                let edge = &edges[&key];
                u128::from(
                    edge.storage
                        .remaining_below(edge.capacity)
                        .min(edge.inlet_remaining)
                        .get(),
                )
            }
            FlowTarget::Boundary(_) => continue,
        };
        let demands: Vec<u128> = indices
            .iter()
            .map(|&index| outflows[index].accepted)
            .collect();
        let mut total = 0_u128;
        for demand in &demands {
            total = total
                .checked_add(*demand)
                .ok_or(WaterVolumeError::Overflow)?;
        }
        if total <= room {
            continue;
        }
        let shares = allocate_largest_remainder(room, &demands)?;
        for (&index, share) in indices.iter().zip(shares) {
            outflows[index].accepted = share;
        }
    }
    Ok(())
}

/// Debit every donor, credit every receiver, and record what each transfer did.
///
/// The two passes run in different canonical orders — donors in `(donor, face,
/// process)` order, receivers grouped by destination — so each receipt's
/// before/after pair is the exact step that transfer took, and a receipt whose
/// withdrawal and deposit disagree cannot be produced.
fn apply_transfers(
    batch: &mut Batch,
    work: &mut BTreeMap<ChartChunkCoord, Vec<CellWork>>,
    edges: &mut BTreeMap<HydrologyEdgeKey, EdgeWork>,
    outflows: &mut [Outflow],
    channel: &RoutingChannel,
) -> Result<(), HydrologyError> {
    for flow in outflows.iter_mut() {
        let accepted = share_volume(flow.accepted)?;
        let entry = cell_work(work, flow.donor).expect("a donor is resident");
        let slot = bucket_slot(&mut entry.storage, channel.property);
        flow.source_before = *slot;
        flow.source_after = slot.checked_sub(accepted)?;
        *slot = flow.source_after;
    }

    let mut grouped: BTreeMap<FlowTarget, Vec<usize>> = BTreeMap::new();
    for (index, flow) in outflows.iter().enumerate() {
        grouped.entry(flow.target).or_default().push(index);
    }
    for (target, indices) in grouped {
        for index in indices {
            let accepted = share_volume(outflows[index].accepted)?;
            let (before, after) = match target {
                FlowTarget::Cell(cell) => {
                    let entry = cell_work(work, cell).expect("a receiver is resident");
                    let slot = bucket_slot(&mut entry.storage, channel.property);
                    let before = *slot;
                    *slot = before.checked_add(accepted)?;
                    (before, *slot)
                }
                FlowTarget::Edge(key) => {
                    let edge = edges.get_mut(&key).expect("the edge was found by key");
                    let before = edge.storage;
                    edge.storage = before.checked_add(accepted)?;
                    edge.inlet_remaining = edge.inlet_remaining.checked_sub(accepted)?;
                    (before, edge.storage)
                }
                // An export leaves the world. Its sink receipt is materialized in
                // substage 8; nothing further is removed there.
                FlowTarget::Boundary(_) => (WaterVolume::ZERO, WaterVolume::ZERO),
            };
            outflows[index].target_before = before;
            outflows[index].target_after = after;
            if matches!(target, FlowTarget::Boundary(_)) {
                batch.boundary_exports = batch.boundary_exports.add_volume(accepted)?;
            }
        }
    }
    Ok(())
}

/// Emit one settlement event per changed cell, one inflow event per receiving
/// edge, and every transfer receipt of one routing pass.
#[allow(clippy::too_many_arguments)]
fn emit_routing_events(
    batch: &mut Batch,
    work: &mut BTreeMap<ChartChunkCoord, Vec<CellWork>>,
    edges: &mut BTreeMap<HydrologyEdgeKey, EdgeWork>,
    request: &HydrologyEvolutionRequest<'_>,
    channel: &RoutingChannel,
    outflows: &[Outflow],
    frozen: &BTreeMap<HydrologyCellKey, WaterVolume>,
) -> Result<(), HydrologyError> {
    // Every cause is taken from this snapshot, so a sibling settlement in the
    // same substage is never cited: each of them was computed from the frozen
    // state, and citing one another would claim a dependency that the frozen
    // substage specifically does not have.
    let mut references: BTreeMap<HydrologyCellKey, CausalEventDagCause> = BTreeMap::new();
    for (chunk, entries) in work.iter() {
        for (ordinal, entry) in entries.iter().enumerate() {
            let cell = HydrologyCellKey::new(*chunk, ordinal as u16)?;
            references.insert(cell, cell_bucket_reference(entry, channel.property));
        }
    }
    let edge_references: BTreeMap<HydrologyEdgeKey, CausalEventDagCause> = edges
        .iter()
        .map(|(key, edge)| (*key, edge.reference.clone()))
        .collect();

    // Only cells whose bucket actually moved settle. A cell that passed ten units
    // through — ten in, ten out — holds what it started with, so it has no state
    // change to anchor and `CausalEffect` refuses an effect that claims one. Its
    // two transfers are still attributable: the inbound one names the donor's
    // event and the outbound one names the receiver's, so neither is an orphan,
    // and the cell's `last_change` correctly still points at whenever its stored
    // volume last differed.
    let mut changed: BTreeMap<HydrologyCellKey, (WaterVolume, WaterVolume)> = BTreeMap::new();
    for (chunk, entries) in work.iter() {
        for (ordinal, entry) in entries.iter().enumerate() {
            let cell = HydrologyCellKey::new(*chunk, ordinal as u16)?;
            let after = bucket_value(&entry.storage, channel.property);
            let before = frozen[&cell];
            if before != after {
                changed.insert(cell, (before, after));
            }
        }
    }

    let mut emitted: BTreeMap<HydrologyCellKey, CausalEventProposalKey> = BTreeMap::new();
    for (cell, (before, after)) in &changed {
        let key = vertical_key(channel.substage_ordinal, channel.settlement_process, *cell)?;
        let mut causes: BTreeSet<CausalEventDagCause> = BTreeSet::new();
        causes.insert(references[cell].clone());
        for direction in FaceDirection::ALL {
            if let Some(neighbor) = cell.neighbor(direction)
                && let Some(reference) = references.get(&neighbor)
            {
                causes.insert(reference.clone());
            }
        }
        if let Some(edge) = request.conveyance.outgoing(*cell) {
            causes.insert(CausalEventDagCause::Existing(edge.last_change()));
        }
        batch.events.push(HydrologyEventPlan {
            key: key.clone(),
            kind: HydrologyEventKind::CellChange,
            coarse_process: None,
            causes: causes.into_iter().collect(),
            effects: vec![bucket_effect(*cell, channel.property, *before, *after)],
        });
        batch.cell_changes.push(HydrologyCellChange {
            cell: *cell,
            bucket: channel.bucket(),
            before: *before,
            after: *after,
            settlement_event: key.clone(),
        });
        let entry = cell_work(work, *cell).expect("a changed cell is resident");
        set_reference(entry, channel.property, key.clone());
        emitted.insert(*cell, key);
    }

    // One inflow event per edge that actually received. Cites the edge's own
    // reference plus the local routing event of the cell the water came from.
    let mut edge_inflows: BTreeMap<HydrologyEdgeKey, WaterVolume> = BTreeMap::new();
    for flow in outflows {
        if let FlowTarget::Edge(key) = flow.target {
            let accepted = share_volume(flow.accepted)?;
            let total = edge_inflows.entry(key).or_insert(WaterVolume::ZERO);
            *total = total.checked_add(accepted)?;
        }
    }
    for (edge_key, total) in edge_inflows {
        if total.is_zero() {
            continue;
        }
        let edge = edges.get_mut(&edge_key).expect("the edge received water");
        let before = edge.storage.checked_sub(total)?;
        let after = edge.storage;
        let key = CausalEventProposalKey::new(
            channel.substage_ordinal,
            channel.edge_process,
            &HydrologyCarrierKey::Edge(edge_key).encode(),
            0,
        )?;
        let mut causes: BTreeSet<CausalEventDagCause> = BTreeSet::new();
        causes.insert(edge_references[&edge_key].clone());
        if let Some(source_event) = emitted.get(&edge.source) {
            causes.insert(CausalEventDagCause::Local(source_event.clone()));
        }
        batch.events.push(HydrologyEventPlan {
            key: key.clone(),
            kind: HydrologyEventKind::EdgeTransfer,
            coarse_process: None,
            causes: causes.into_iter().collect(),
            effects: vec![edge_effect(edge_key, before, after)],
        });
        batch.edge_changes.push(HydrologyEdgeChange {
            edge: edge_key,
            before,
            after,
            settlement_event: key.clone(),
        });
        edge.reference = CausalEventDagCause::Local(key.clone());
        edge.terminal = Some(key);
    }

    for flow in outflows {
        let requested = share_volume(flow.raw)?;
        let accepted = share_volume(flow.accepted)?;
        let donor_event = emitted.get(&flow.donor).cloned();
        match flow.target {
            FlowTarget::Boundary(face) => {
                batch.pending_exports.push(PendingExport {
                    donor: flow.donor,
                    face,
                    bucket: channel.bucket(),
                    process_kind: flow.process_kind,
                    requested,
                    accepted,
                    source_before: flow.source_before,
                    source_after: flow.source_after,
                    event: vertical_key(
                        channel.substage_ordinal,
                        channel.settlement_process,
                        flow.donor,
                    )?,
                });
            }
            FlowTarget::Cell(receiver) => {
                let receiver_event = emitted.get(&receiver).cloned();
                batch
                    .receipts
                    .push(HydrologyTransferReceipt::new(HydrologyTransferParts {
                        batch_sequence: batch.batch_sequence,
                        tick: batch.tick,
                        process_kind: flow.process_kind,
                        source: HydrologyCarrierKey::Cell(flow.donor),
                        source_bucket: channel.bucket(),
                        target: HydrologyCarrierKey::Cell(receiver),
                        target_bucket: channel.bucket(),
                        requested,
                        accepted,
                        source_before: flow.source_before,
                        source_after: flow.source_after,
                        target_before: flow.target_before,
                        target_after: flow.target_after,
                        causal_parents: Vec::new(),
                        forcing_origin: None,
                        transfer_event: donor_event,
                        storage_event: receiver_event,
                    })?);
            }
            FlowTarget::Edge(edge_key) => {
                let edge_event = edges[&edge_key].terminal.clone();
                batch
                    .receipts
                    .push(HydrologyTransferReceipt::new(HydrologyTransferParts {
                        batch_sequence: batch.batch_sequence,
                        tick: batch.tick,
                        process_kind: flow.process_kind,
                        source: HydrologyCarrierKey::Cell(flow.donor),
                        source_bucket: channel.bucket(),
                        target: HydrologyCarrierKey::Edge(edge_key),
                        target_bucket: HydrologyBucket::Conveyance,
                        requested,
                        accepted,
                        source_before: flow.source_before,
                        source_after: flow.source_after,
                        target_before: flow.target_before,
                        target_after: flow.target_after,
                        causal_parents: Vec::new(),
                        forcing_origin: None,
                        transfer_event: donor_event,
                        storage_event: edge_event,
                    })?);
            }
        }
    }
    Ok(())
}

/// Substage 7 — release stored conveyance water toward each edge's outlet.
///
/// Every release is computed from the complete frozen pre-release edge state, so
/// water that arrived this tick cannot leave again in the same tick and a chain
/// of edges cannot cascade. A flat or closed depression simply keeps its storage.
fn substage_conveyance(
    batch: &mut Batch,
    work: &mut BTreeMap<ChartChunkCoord, Vec<CellWork>>,
    edges: &mut BTreeMap<HydrologyEdgeKey, EdgeWork>,
    state: &HydrologyFieldSet,
    request: &HydrologyEvolutionRequest<'_>,
) -> Result<(), HydrologyError> {
    let frozen: BTreeMap<HydrologyEdgeKey, WaterVolume> = edges
        .iter()
        .map(|(key, edge)| (*key, edge.storage))
        .collect();

    struct Release {
        source: HydrologyEdgeKey,
        target: FlowTarget,
        raw: u128,
        accepted: u128,
        source_before: WaterVolume,
        source_after: WaterVolume,
        target_before: WaterVolume,
        target_after: WaterVolume,
    }

    let mut releases: Vec<Release> = Vec::new();
    for (key, edge) in edges.iter() {
        let stored = frozen[key];
        let raw = edge.release.apply_to_volume(stored)?.min(stored);
        if raw.is_zero() {
            continue;
        }
        // The outlet's own outgoing edge continues the channel; a local minimum
        // spills onto the outlet cell's surface instead.
        let target = match request.conveyance.outgoing(edge.outlet) {
            Some(downstream) => FlowTarget::Edge(downstream.key()),
            None => FlowTarget::Cell(edge.outlet),
        };
        releases.push(Release {
            source: *key,
            target,
            raw: u128::from(raw.get()),
            accepted: u128::from(raw.get()),
            source_before: WaterVolume::ZERO,
            source_after: WaterVolume::ZERO,
            target_before: WaterVolume::ZERO,
            target_after: WaterVolume::ZERO,
        });
    }
    // Ascending source edge key: the plan's tie-break for equal remainders when
    // several edges compete for one downstream capacity.
    releases.sort_by_key(|release| release.source);

    let mut grouped: BTreeMap<FlowTarget, Vec<usize>> = BTreeMap::new();
    for (index, release) in releases.iter().enumerate() {
        grouped.entry(release.target).or_default().push(index);
    }
    for (target, indices) in &grouped {
        let room = match target {
            FlowTarget::Edge(key) => {
                let edge = &edges[key];
                u128::from(
                    frozen[key]
                        .remaining_below(edge.capacity)
                        .min(edge.inlet_remaining)
                        .get(),
                )
            }
            FlowTarget::Cell(cell) => {
                let ground = state
                    .ground(*cell)
                    .ok_or(HydrologyError::ForcingTargetNotResident)?;
                let entry = cell_work(work, *cell).expect("an outlet is resident");
                u128::from(
                    entry
                        .storage
                        .surface
                        .remaining_below(ground.surface_capacity())
                        .get(),
                )
            }
            FlowTarget::Boundary(_) => continue,
        };
        let demands: Vec<u128> = indices
            .iter()
            .map(|&index| releases[index].accepted)
            .collect();
        let mut total = 0_u128;
        for demand in &demands {
            total = total
                .checked_add(*demand)
                .ok_or(WaterVolumeError::Overflow)?;
        }
        if total <= room {
            continue;
        }
        let shares = allocate_largest_remainder(room, &demands)?;
        for (&index, share) in indices.iter().zip(shares) {
            releases[index].accepted = share;
        }
    }

    for release in releases.iter_mut() {
        let accepted = share_volume(release.accepted)?;
        let edge = edges
            .get_mut(&release.source)
            .expect("the source edge exists");
        release.source_before = edge.storage;
        release.source_after = edge.storage.checked_sub(accepted)?;
        edge.storage = release.source_after;
    }
    // The net change each receiver settles, captured as it happens: the first
    // credit's `before` and the last credit's `after`.
    let mut settled: BTreeMap<FlowTarget, (WaterVolume, WaterVolume)> = BTreeMap::new();
    for (target, indices) in &grouped {
        for &index in indices {
            let accepted = share_volume(releases[index].accepted)?;
            let (before, after) = match target {
                FlowTarget::Edge(key) => {
                    let edge = edges.get_mut(key).expect("the downstream edge exists");
                    let before = edge.storage;
                    edge.storage = before.checked_add(accepted)?;
                    edge.inlet_remaining = edge.inlet_remaining.checked_sub(accepted)?;
                    (before, edge.storage)
                }
                FlowTarget::Cell(cell) => {
                    let entry = cell_work(work, *cell).expect("an outlet is resident");
                    let before = entry.storage.surface;
                    entry.storage.surface = before.checked_add(accepted)?;
                    (before, entry.storage.surface)
                }
                FlowTarget::Boundary(_) => continue,
            };
            releases[index].target_before = before;
            releases[index].target_after = after;
            settled
                .entry(*target)
                .and_modify(|range| range.1 = after)
                .or_insert((before, after));
        }
    }

    // Causes come from the pre-release snapshot for the same reason the lateral
    // passes snapshot theirs: the allocation depended on what every competitor
    // was holding before any of them released.
    let pre_release_edge_refs: BTreeMap<HydrologyEdgeKey, CausalEventDagCause> = edges
        .iter()
        .map(|(key, edge)| (*key, edge.reference.clone()))
        .collect();
    let mut pre_release_cell_refs: BTreeMap<HydrologyCellKey, CausalEventDagCause> =
        BTreeMap::new();
    for (chunk, entries) in work.iter() {
        for (ordinal, entry) in entries.iter().enumerate() {
            let cell = HydrologyCellKey::new(*chunk, ordinal as u16)?;
            pre_release_cell_refs.insert(cell, entry.surface_ref.clone());
        }
    }

    let mut allocation_events: BTreeMap<FlowTarget, Vec<CausalEventProposalKey>> = BTreeMap::new();
    for release in &releases {
        if release.accepted == 0 {
            continue;
        }
        let key = CausalEventProposalKey::new(
            substage::CONVEYANCE_ROUTING,
            process::CONVEYANCE_RELEASE,
            &HydrologyCarrierKey::Edge(release.source).encode(),
            0,
        )?;
        let mut causes: BTreeSet<CausalEventDagCause> = BTreeSet::new();
        causes.insert(pre_release_edge_refs[&release.source].clone());
        match release.target {
            FlowTarget::Edge(downstream) => {
                causes.insert(pre_release_edge_refs[&downstream].clone());
            }
            FlowTarget::Cell(cell) => {
                causes.insert(pre_release_cell_refs[&cell].clone());
            }
            FlowTarget::Boundary(_) => {}
        }
        // At most three competitors: a receiver has at most four faces, so at
        // most four edges can name it as their outlet.
        for &index in &grouped[&release.target] {
            let competitor = releases[index].source;
            if competitor != release.source {
                causes.insert(pre_release_edge_refs[&competitor].clone());
            }
        }
        batch.events.push(HydrologyEventPlan {
            key: key.clone(),
            kind: HydrologyEventKind::EdgeTransfer,
            coarse_process: None,
            causes: causes.into_iter().collect(),
            effects: vec![edge_effect(
                release.source,
                release.source_before,
                release.source_after,
            )],
        });
        batch.edge_changes.push(HydrologyEdgeChange {
            edge: release.source,
            before: release.source_before,
            after: release.source_after,
            settlement_event: key.clone(),
        });
        let edge = edges
            .get_mut(&release.source)
            .expect("the source edge exists");
        edge.reference = CausalEventDagCause::Local(key.clone());
        edge.terminal = Some(key.clone());
        allocation_events
            .entry(release.target)
            .or_default()
            .push(key);
    }

    for (target, allocations) in allocation_events {
        let carrier = match target {
            FlowTarget::Edge(key) => HydrologyCarrierKey::Edge(key),
            FlowTarget::Cell(cell) => HydrologyCarrierKey::Cell(cell),
            FlowTarget::Boundary(_) => continue,
        };
        let Some(&(before, after)) = settled.get(&target) else {
            continue;
        };
        if before == after {
            continue;
        }
        let key = CausalEventProposalKey::new(
            substage::CONVEYANCE_ROUTING,
            process::CONVEYANCE_SETTLEMENT,
            &carrier.encode(),
            0,
        )?;
        let mut causes: BTreeSet<CausalEventDagCause> = BTreeSet::new();
        match target {
            FlowTarget::Edge(edge_key) => {
                causes.insert(edges[&edge_key].reference.clone());
            }
            FlowTarget::Cell(cell) => {
                let entry = cell_work(work, cell).expect("an outlet is resident");
                causes.insert(entry.surface_ref.clone());
            }
            FlowTarget::Boundary(_) => {}
        }
        for allocation in &allocations {
            causes.insert(CausalEventDagCause::Local(allocation.clone()));
        }
        let effects = match target {
            FlowTarget::Edge(edge_key) => vec![edge_effect(edge_key, before, after)],
            FlowTarget::Cell(cell) => {
                vec![bucket_effect(
                    cell,
                    HydrologyProperty::Surface,
                    before,
                    after,
                )]
            }
            FlowTarget::Boundary(_) => Vec::new(),
        };
        batch.events.push(HydrologyEventPlan {
            key: key.clone(),
            kind: HydrologyEventKind::EdgeTransfer,
            coarse_process: None,
            causes: causes.into_iter().collect(),
            effects,
        });
        match target {
            FlowTarget::Edge(edge_key) => {
                batch.edge_changes.push(HydrologyEdgeChange {
                    edge: edge_key,
                    before,
                    after,
                    settlement_event: key.clone(),
                });
                let edge = edges.get_mut(&edge_key).expect("the edge exists");
                edge.reference = CausalEventDagCause::Local(key.clone());
                edge.terminal = Some(key);
            }
            FlowTarget::Cell(cell) => {
                batch.cell_changes.push(HydrologyCellChange {
                    cell,
                    bucket: HydrologyBucket::Surface,
                    before,
                    after,
                    settlement_event: key.clone(),
                });
                let entry = cell_work(work, cell).expect("an outlet is resident");
                set_reference(entry, HydrologyProperty::Surface, key);
            }
            FlowTarget::Boundary(_) => {}
        }
    }

    for release in &releases {
        let (target_carrier, target_bucket) = match release.target {
            FlowTarget::Edge(key) => (HydrologyCarrierKey::Edge(key), HydrologyBucket::Conveyance),
            FlowTarget::Cell(cell) => (HydrologyCarrierKey::Cell(cell), HydrologyBucket::Surface),
            FlowTarget::Boundary(face) => (
                HydrologyCarrierKey::ExteriorFace(face),
                HydrologyBucket::External,
            ),
        };
        batch
            .receipts
            .push(HydrologyTransferReceipt::new(HydrologyTransferParts {
                batch_sequence: batch.batch_sequence,
                tick: batch.tick,
                process_kind: process::CONVEYANCE_RELEASE,
                source: HydrologyCarrierKey::Edge(release.source),
                source_bucket: HydrologyBucket::Conveyance,
                target: target_carrier,
                target_bucket,
                requested: share_volume(release.raw)?,
                accepted: share_volume(release.accepted)?,
                source_before: release.source_before,
                source_after: release.source_after,
                target_before: release.target_before,
                target_after: release.target_after,
                causal_parents: Vec::new(),
                forcing_origin: None,
                transfer_event: edges[&release.source].terminal.clone(),
                storage_event: None,
            })?);
    }
    Ok(())
}

/// Substage 8 — materialize the sink receipts for exports already accepted.
///
/// No new demand and no further withdrawal: the donor reduction in substages 5
/// and 6 already removed this water, and its own before/after pair is carried
/// here so the ledger's `sinks` term has per-face evidence.
fn substage_boundary_export(batch: &mut Batch) -> Result<(), HydrologyError> {
    for export in std::mem::take(&mut batch.pending_exports) {
        batch
            .receipts
            .push(HydrologyTransferReceipt::new(HydrologyTransferParts {
                batch_sequence: batch.batch_sequence,
                tick: batch.tick,
                process_kind: export.process_kind,
                source: HydrologyCarrierKey::Cell(export.donor),
                source_bucket: export.bucket,
                target: HydrologyCarrierKey::ExteriorFace(export.face),
                target_bucket: HydrologyBucket::External,
                requested: export.requested,
                accepted: export.accepted,
                source_before: export.source_before,
                source_after: export.source_after,
                target_before: WaterVolume::ZERO,
                target_after: WaterVolume::ZERO,
                causal_parents: Vec::new(),
                forcing_origin: None,
                transfer_event: (!export.accepted.is_zero()).then(|| export.event.clone()),
                storage_event: (!export.accepted.is_zero()).then_some(export.event),
            })?);
    }
    Ok(())
}

fn edge_effect(
    edge: HydrologyEdgeKey,
    before: WaterVolume,
    after: WaterVolume,
) -> HydrologyEventEffect {
    let carrier = HydrologyCarrierKey::Edge(edge);
    HydrologyEventEffect {
        carrier,
        property: HydrologyProperty::Conveyance,
        before: volume_fingerprint(&carrier, HydrologyProperty::Conveyance, before),
        after: volume_fingerprint(&carrier, HydrologyProperty::Conveyance, after),
    }
}

fn build_after_conveyance(
    edges: &BTreeMap<HydrologyEdgeKey, EdgeWork>,
) -> Result<HydrologyConveyanceGraph, HydrologyError> {
    let mut rebuilt = Vec::with_capacity(edges.len());
    for (key, edge) in edges {
        rebuilt.push(HydrologyConveyanceEdge::new(
            *key,
            edge.outlet,
            edge.storage,
            edge.capacity,
            edge.release,
            edge.inlet_capacity,
            edge.last_change,
            if edge.storage == edge.before {
                edge.last_change_before
            } else {
                edge.before
            },
        )?);
    }
    Ok(HydrologyConveyanceGraph::new(rebuilt)?)
}

// ---------------------------------------------------------------------------
// Substage 9 — conservation preflight, and the after-state
// ---------------------------------------------------------------------------

fn build_after_state(
    state: &HydrologyFieldSet,
    work: &BTreeMap<ChartChunkCoord, Vec<CellWork>>,
    metrics: &HydrologyGridMetrics,
    batch_sequence: u64,
) -> Result<HydrologyFieldSet, HydrologyError> {
    let mut fields = Vec::with_capacity(state.fields().len());
    for (chunk, field) in state.fields() {
        let entries = &work[chunk];
        // The trace anchors carried through here are the *pre-tick* ones. The
        // events that will replace them have proposal keys, not trace IDs, until
        // the batch commits, so the runtime installs the new anchors from
        // `cell_changes` afterwards — the same seam the thermal carrier uses.
        let cells = field
            .cells()
            .iter()
            .zip(entries)
            .map(|(cell, entry)| {
                let changed = entry.storage != entry.before;
                HydrologyCellState::from_parts(
                    entry.storage,
                    cell.surface_last_change(),
                    cell.soil_last_change(),
                    cell.groundwater_last_change(),
                    entry.forcing_fingerprint,
                    cell.forcing_last_change(),
                    if changed {
                        entry.before
                    } else {
                        cell.last_change_before()
                    },
                )
            })
            .collect();
        fields.push(HydrologyField::from_parts(
            *chunk,
            cells,
            field.substrate().to_vec(),
        )?);
    }
    Ok(HydrologyFieldSet::from_parts(
        fields,
        metrics,
        batch_sequence,
        state.conservation_last_change(),
    )?)
}

fn build_conservation(
    batch: &Batch,
    before: &HydrologyFieldSet,
    after: &HydrologyFieldSet,
    conveyance_before: &HydrologyConveyanceGraph,
    conveyance_after: &HydrologyConveyanceGraph,
) -> Result<HydrologyConservationReceipt, HydrologyError> {
    // Totals are summed from the two field sets and the two conveyance graphs,
    // not from the running deltas the solver accumulated. A ledger built from the
    // solver's own bookkeeping would agree with the solver by construction and
    // could not catch it.
    let (surface_before, soil_before, groundwater_before) = bucket_totals(before)?;
    let (surface_after, soil_after, groundwater_after) = bucket_totals(after)?;

    HydrologyConservationReceipt::new(HydrologyConservationParts {
        tick: batch.tick,
        batch_sequence: batch.batch_sequence,
        surface_before,
        soil_before,
        groundwater_before,
        conveyance_before: conveyance_before.total_storage()?.get(),
        surface_after,
        soil_after,
        groundwater_after,
        conveyance_after: conveyance_after.total_storage()?.get(),
        accepted_precipitation: batch.accepted_precipitation.get(),
        accepted_external_inflow: batch.accepted_external_inflow.get(),
        accepted_evapotranspiration: batch.accepted_evapotranspiration.get(),
        boundary_exports: batch.boundary_exports.get(),
    })
}

fn bucket_totals(state: &HydrologyFieldSet) -> Result<(i128, i128, i128), HydrologyError> {
    let mut surface = WaterAccumulator::ZERO;
    let mut soil = WaterAccumulator::ZERO;
    let mut groundwater = WaterAccumulator::ZERO;
    for field in state.fields().values() {
        for cell in field.cells() {
            surface = surface.add_volume(cell.surface_water())?;
            soil = soil.add_volume(cell.soil_water())?;
            groundwater = groundwater.add_volume(cell.groundwater())?;
        }
    }
    Ok((surface.get(), soil.get(), groundwater.get()))
}

/// Every bucket that ended the tick anchored to a local event, in the canonical
/// leaf order `(carrier bytes, bucket tag, proposal key bytes)`.
fn collect_terminal_leaves(
    work: &BTreeMap<ChartChunkCoord, Vec<CellWork>>,
    edges: &BTreeMap<HydrologyEdgeKey, EdgeWork>,
    events: &[HydrologyEventPlan],
) -> Vec<HydrologyTerminalLeaf> {
    let mut leaves = Vec::new();
    // A record becoming spent is a terminal change like any other. Without its
    // leaf the conservation event would not reach the event that says the tick's
    // water was allowed in, and the schedule's consumption would sit outside the
    // ancestry the ledger closes over.
    for event in events {
        if event.kind != HydrologyEventKind::ForcingApplication {
            continue;
        }
        for effect in &event.effects {
            leaves.push(HydrologyTerminalLeaf {
                carrier_bytes: effect.carrier.encode(),
                bucket_tag: HydrologyBucket::ForcingRecord.tag(),
                event: event.key.clone(),
            });
        }
    }
    for (key, edge) in edges {
        if let Some(event) = &edge.terminal {
            leaves.push(HydrologyTerminalLeaf {
                carrier_bytes: HydrologyCarrierKey::Edge(*key).encode(),
                bucket_tag: HydrologyBucket::Conveyance.tag(),
                event: event.clone(),
            });
        }
    }
    for (chunk, entries) in work {
        for (ordinal, entry) in entries.iter().enumerate() {
            let Ok(cell) = HydrologyCellKey::new(*chunk, ordinal as u16) else {
                continue;
            };
            let carrier_bytes = HydrologyCarrierKey::Cell(cell).encode();
            for (bucket, terminal) in [
                (HydrologyBucket::Surface, &entry.terminal_surface),
                (HydrologyBucket::Soil, &entry.terminal_soil),
                (HydrologyBucket::Groundwater, &entry.terminal_groundwater),
                (HydrologyBucket::ForcingInput, &entry.terminal_forcing),
            ] {
                if let Some(event) = terminal {
                    leaves.push(HydrologyTerminalLeaf {
                        carrier_bytes: carrier_bytes.clone(),
                        bucket_tag: bucket.tag(),
                        event: event.clone(),
                    });
                }
            }
        }
    }
    leaves.sort();
    leaves
}
