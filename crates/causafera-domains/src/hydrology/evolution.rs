use std::collections::{BTreeMap, BTreeSet};

use causafera_core::{CausalEventDagCause, CausalEventProposalKey, StateFingerprint};
use causafera_geography::{
    HydrologyCarrierKey, HydrologyCellKey, HydrologyCellState, HydrologyCellStorage,
    HydrologyField, HydrologyFieldSet, HydrologyForcingRecord, HydrologyGridMetrics,
};
use causafera_types::{ChartChunkCoord, TraceId, WaterAccumulator, WaterVolume, WaterVolumeError};

use super::{
    HydrologyBucket, HydrologyCellChange, HydrologyConservationParts, HydrologyConservationReceipt,
    HydrologyError, HydrologyEventEffect, HydrologyEventKind, HydrologyEventPlan,
    HydrologyEvolutionProposal, HydrologyEvolutionRequest, HydrologyForcingAllocation,
    HydrologyForcingSettlement, HydrologyProperty, HydrologyProposalParts, HydrologyTerminalLeaf,
    HydrologyTransferParts, HydrologyTransferReceipt, forcing_applied_fingerprint,
    forcing_settlement_fingerprint, process, substage, volume_fingerprint,
};

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
    origins: BTreeSet<TraceId>,
}

struct Batch {
    tick: u64,
    batch_sequence: u64,
    receipts: Vec<HydrologyTransferReceipt>,
    events: Vec<HydrologyEventPlan>,
    cell_changes: Vec<HydrologyCellChange>,
    accepted_precipitation: WaterAccumulator,
    accepted_external_inflow: WaterAccumulator,
    accepted_evapotranspiration: WaterAccumulator,
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
            accepted_precipitation: WaterAccumulator::ZERO,
            accepted_external_inflow: WaterAccumulator::ZERO,
            accepted_evapotranspiration: WaterAccumulator::ZERO,
        };

        // The chart of every resident chunk must have a registered metric, or
        // nothing about it is computable. The field set already checked this at
        // construction; re-checking here keeps the solver honest about its own
        // preconditions rather than trusting a caller-built value object.
        for chunk in state.fields().keys() {
            request.metrics.get(chunk.chart)?;
        }

        let mut work = build_work(state);
        let mut applied_forcing = Vec::new();
        let settlements =
            substage_forcing(&mut batch, &mut work, state, &request, &mut applied_forcing)?;
        substage_infiltration(&mut batch, &mut work, state)?;
        substage_percolation(&mut batch, &mut work, state)?;
        let settlements = substage_evapotranspiration(&mut batch, &mut work, settlements)?;

        if batch.receipts.len() > request.limits.max_transfers_per_tick {
            return Err(HydrologyError::TransferLimitExceeded {
                count: batch.receipts.len(),
                max: request.limits.max_transfers_per_tick,
            });
        }

        let after_state = build_after_state(state, &work, request.metrics, batch_sequence)?;
        let conservation = build_conservation(&batch, state, &after_state, &request)?;
        conservation.require_balanced()?;

        let terminal_leaves = collect_terminal_leaves(&work);

        Ok(HydrologyEvolutionProposal::new(HydrologyProposalParts {
            tick: request.tick,
            batch_sequence,
            after_state,
            after_conveyance: request.conveyance.clone(),
            applied_forcing,
            forcing_settlements: settlements,
            cell_changes: batch.cell_changes,
            edge_changes: Vec::new(),
            transfer_receipts: batch.receipts,
            conservation,
            events: batch.events,
            terminal_leaves,
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
// Substage 1 — forcing acceptance
// ---------------------------------------------------------------------------

fn substage_forcing(
    batch: &mut Batch,
    work: &mut BTreeMap<ChartChunkCoord, Vec<CellWork>>,
    state: &HydrologyFieldSet,
    request: &HydrologyEvolutionRequest<'_>,
    applied: &mut Vec<(u64, u64)>,
) -> Result<Vec<HydrologyForcingSettlement>, HydrologyError> {
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

            let entry = settlements.entry(cell).or_default();
            entry.origins.insert(record.origin_trace());
            entry.accepted_source = entry
                .accepted_source
                .checked_add(accepted_precipitation)?
                .checked_add(accepted_external)?;
            entry.rejected_source = entry
                .rejected_source
                .checked_add(precipitation_share.checked_sub(accepted_precipitation)?)?
                .checked_add(external_share.checked_sub(accepted_external)?)?;
            entry.allocations.push(HydrologyForcingAllocation {
                scheduled_tick: record.scheduled_tick(),
                forcing_id: record.forcing_id(),
                origin: record.origin_trace(),
                precipitation: precipitation_share,
                external_inflow: external_share,
                potential_et: demand_share,
                accepted_source: accepted_precipitation.checked_add(accepted_external)?,
                // Filled in by substage 4, which is where ET is actually met.
                accepted_et: WaterVolume::ZERO,
            });

            let entry = cell_work(work, cell).expect("residency was validated above");
            entry.et_demand = entry.et_demand.checked_add(demand_share)?;
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

    // One settlement per targeted cell, in canonical cell order.
    let mut settled = Vec::with_capacity(settlements.len());
    for (cell, settlement) in settlements {
        let key = settlement_key(cell)?;
        let entry = cell_work(work, cell).expect("residency was validated above");
        let before = entry.forcing_fingerprint;
        let after = forcing_settlement_fingerprint(batch.tick, cell, &settlement.allocations);

        let mut causes: BTreeSet<CausalEventDagCause> = BTreeSet::new();
        causes.insert(entry.surface_ref.clone());
        causes.insert(entry.soil_ref.clone());
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
            let surface_before = entry.before.surface;
            let surface_after = entry.storage.surface;
            effects.push(bucket_effect(
                cell,
                HydrologyProperty::Surface,
                surface_before,
                surface_after,
            ));
            batch.cell_changes.push(HydrologyCellChange {
                cell,
                bucket: HydrologyBucket::Surface,
                before: surface_before,
                after: surface_after,
                settlement_event: key.clone(),
            });
        }

        batch.events.push(HydrologyEventPlan {
            key: key.clone(),
            kind: HydrologyEventKind::ForcingSettlement,
            causes: causes.into_iter().collect(),
            effects,
        });

        entry.forcing_fingerprint = after;
        entry.forcing_settlement = Some(key.clone());
        entry.terminal_forcing = Some(key.clone());
        if !settlement.accepted_source.is_zero() {
            entry.surface_ref = CausalEventDagCause::Local(key.clone());
            entry.terminal_surface = Some(key.clone());
        }

        settled.push(HydrologyForcingSettlement {
            cell,
            allocations: settlement.allocations,
            accepted_source: settlement.accepted_source,
            rejected_source: settlement.rejected_source,
            accepted_et: WaterVolume::ZERO,
            unmet_et: WaterVolume::ZERO,
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

// ---------------------------------------------------------------------------
// Substages 2-4 — the vertical cycle
// ---------------------------------------------------------------------------

fn substage_infiltration(
    batch: &mut Batch,
    work: &mut BTreeMap<ChartChunkCoord, Vec<CellWork>>,
    state: &HydrologyFieldSet,
) -> Result<(), HydrologyError> {
    for (chunk, field) in state.fields() {
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
) -> Result<(), HydrologyError> {
    for (chunk, field) in state.fields() {
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

fn substage_evapotranspiration(
    batch: &mut Batch,
    work: &mut BTreeMap<ChartChunkCoord, Vec<CellWork>>,
    mut settlements: Vec<HydrologyForcingSettlement>,
) -> Result<Vec<HydrologyForcingSettlement>, HydrologyError> {
    for settlement in &mut settlements {
        let cell = settlement.cell;
        let entry = cell_work(work, cell).expect("a settled cell is resident");
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
            cell,
        )?;

        push_et_receipt(
            batch,
            EtReceipt {
                cell,
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
                cell,
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
                cell,
                HydrologyProperty::Surface,
                surface_before,
                entry.storage.surface,
            ));
            entry.surface_ref = CausalEventDagCause::Local(key.clone());
            entry.terminal_surface = Some(key.clone());
            batch.cell_changes.push(HydrologyCellChange {
                cell,
                bucket: HydrologyBucket::Surface,
                before: surface_before,
                after: entry.storage.surface,
                settlement_event: key.clone(),
            });
        }
        if !soil_taken.is_zero() {
            effects.push(bucket_effect(
                cell,
                HydrologyProperty::Soil,
                soil_before,
                entry.storage.soil,
            ));
            entry.soil_ref = CausalEventDagCause::Local(key.clone());
            entry.terminal_soil = Some(key.clone());
            batch.cell_changes.push(HydrologyCellChange {
                cell,
                bucket: HydrologyBucket::Soil,
                before: soil_before,
                after: entry.storage.soil,
                settlement_event: key.clone(),
            });
        }
        batch.events.push(HydrologyEventPlan {
            key,
            kind: HydrologyEventKind::CellChange,
            causes: causes.into_iter().collect(),
            effects,
        });
    }
    Ok(settlements)
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
    request: &HydrologyEvolutionRequest<'_>,
) -> Result<HydrologyConservationReceipt, HydrologyError> {
    // Totals are summed from the two field sets, not from the running deltas
    // the solver accumulated. A ledger built from the solver's own bookkeeping
    // would agree with the solver by construction and could not catch it.
    let (surface_before, soil_before, groundwater_before) = bucket_totals(before)?;
    let (surface_after, soil_after, groundwater_after) = bucket_totals(after)?;
    let conveyance = request.conveyance.total_storage()?.get();

    HydrologyConservationReceipt::new(HydrologyConservationParts {
        tick: batch.tick,
        batch_sequence: batch.batch_sequence,
        surface_before,
        soil_before,
        groundwater_before,
        conveyance_before: conveyance,
        surface_after,
        soil_after,
        groundwater_after,
        conveyance_after: conveyance,
        accepted_precipitation: batch.accepted_precipitation.get(),
        accepted_external_inflow: batch.accepted_external_inflow.get(),
        accepted_evapotranspiration: batch.accepted_evapotranspiration.get(),
        boundary_exports: 0,
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
) -> Vec<HydrologyTerminalLeaf> {
    let mut leaves = Vec::new();
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
