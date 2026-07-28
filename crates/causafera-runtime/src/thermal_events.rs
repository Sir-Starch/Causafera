use crate::*;
use causafera_core::*;
use causafera_domains::*;
use causafera_types::*;
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn accepted_reservoirs(
    receipts: &[ThermalCellTransferReceipt],
) -> BTreeMap<ThermalReservoirId, ThermalEnergy> {
    receipts
        .iter()
        .flat_map(|receipt| &receipt.reservoirs)
        .filter(|record| record.accepted_injection != ThermalEnergy::ZERO)
        .map(|record| (record.id, record.accepted_injection))
        .collect()
}

pub(super) fn build_thermal_events(
    state: &RuntimeState,
    evolution: &ThermalEvolutionProposal,
    accepted: &BTreeMap<ThermalReservoirId, ThermalEnergy>,
) -> Result<Vec<ThermalEvent>, RuntimeError> {
    let mut events = Vec::new();
    let mut ordinal = 0_u64;
    for id in accepted.keys() {
        let reservoir = state
            .thermal_reservoirs
            .get(id)
            .ok_or(RuntimeError::Thermal(ThermalError::UnknownReservoir))?;
        let after = evolution
            .reservoir_budgets_after()
            .get(id)
            .ok_or(RuntimeError::Thermal(ThermalError::UnknownReservoir))?;
        events.push(ThermalEvent {
            proposal: thermal_event(ThermalEventData {
                ordinal,
                kind: THERMAL_RESERVOIR_TRANSFER_EVENT_KIND,
                causes: vec![reservoir.last_change],
                target: CausalTarget::new(
                    StateObjectKindId::new(THERMAL_RESERVOIR_OBJECT_KIND),
                    id.raw(),
                    StatePropertyId::new(THERMAL_RESERVOIR_BUDGET_PROPERTY),
                ),
                before: fingerprint_i64(0x1414, reservoir.budget.get()),
                after: fingerprint_i64(0x1414, after.get()),
            })?,
            subject: ThermalEventSubject::Reservoir(*id),
        });
        ordinal = next_ordinal(ordinal)?;
    }
    for receipt in evolution.transfer_receipts() {
        let Some(material) = &receipt.material else {
            continue;
        };
        let id = MaterialSurfaceId::new(receipt.cell.chunk, receipt.cell.cell_index);
        let cell_trace = state
            .thermal_fields
            .field(receipt.cell.chunk)
            .and_then(|field| {
                field
                    .last_change()
                    .get(usize::from(receipt.cell.cell_index))
            })
            .copied()
            .ok_or(RuntimeError::Thermal(ThermalError::PositionOutsideField))?;
        let mut causes = vec![cell_trace];
        if let Some(surface) = state.material_surfaces.get(&id)
            && let Some(prior_exchange) = surface.thermal.last_exchange
        {
            causes.push(prior_exchange);
        }
        events.push(ThermalEvent {
            proposal: thermal_event(ThermalEventData {
                ordinal,
                kind: MATERIAL_SURFACE_THERMAL_EXCHANGE_EVENT_KIND,
                causes,
                target: CausalTarget::new(
                    StateObjectKindId::new(MATERIAL_SURFACE_OBJECT_KIND),
                    material_surface_object_id(id),
                    StatePropertyId::new(MATERIAL_SURFACE_THERMAL_RETAINED_PROPERTY),
                ),
                before: material_surface_thermal_fingerprint(material.retained_before),
                after: material_surface_thermal_fingerprint(material.retained_after),
            })?,
            subject: ThermalEventSubject::Material(receipt.cell),
        });
        ordinal = next_ordinal(ordinal)?;
    }
    for change in evolution.cell_changes() {
        events.push(ThermalEvent {
            proposal: thermal_event(ThermalEventData {
                ordinal,
                kind: THERMAL_CELL_CHANGE_EVENT_KIND,
                causes: change.parent_traces.clone(),
                target: CausalTarget::new(
                    StateObjectKindId::new(THERMAL_CELL_OBJECT_KIND),
                    cell_object_id(change.cell.chunk, change.cell.cell_index),
                    StatePropertyId::new(THERMAL_ENERGY_PROPERTY),
                ),
                before: fingerprint_i64(0x1415, change.before.get()),
                after: fingerprint_i64(0x1415, change.after.get()),
            })?,
            subject: ThermalEventSubject::Cell(change.cell),
        });
        ordinal = next_ordinal(ordinal)?;
    }
    events.push(conservation_event(ConservationEventData {
        state,
        evolution,
        accepted,
        ordinal,
    })?);
    Ok(events)
}

pub(super) fn install_receipt_traces(
    receipts: &mut [ThermalCellTransferReceipt],
    cell_traces: &BTreeMap<ThermalCellKey, TraceId>,
    reservoir_traces: &BTreeMap<ThermalReservoirId, TraceId>,
) {
    for receipt in receipts {
        receipt.cell_change_trace_id = cell_traces.get(&receipt.cell).copied();
        for record in &mut receipt.reservoirs {
            record.transfer_trace_id = reservoir_traces.get(&record.id).copied();
        }
    }
}

struct ConservationEventData<'a> {
    state: &'a RuntimeState,
    evolution: &'a ThermalEvolutionProposal,
    accepted: &'a BTreeMap<ThermalReservoirId, ThermalEnergy>,
    ordinal: u64,
}

fn conservation_event(data: ConservationEventData<'_>) -> Result<ThermalEvent, RuntimeError> {
    Ok(ThermalEvent {
        proposal: thermal_event(ThermalEventData {
            ordinal: data.ordinal,
            kind: THERMAL_CONSERVATION_EVENT_KIND,
            causes: conservation_parents(data.state, data.accepted),
            target: CausalTarget::new(
                StateObjectKindId::new(THERMAL_CARRIER_OBJECT_KIND),
                thermal_carrier_id(&data.state.thermal_active_region),
                StatePropertyId::new(THERMAL_BATCH_SEQUENCE_PROPERTY),
            ),
            before: fingerprint_u64(0x1416, data.state.thermal_fields.batch_sequence()),
            after: fingerprint_u64(0x1416, data.evolution.after_state().batch_sequence()),
        })?,
        subject: ThermalEventSubject::Conservation,
    })
}

struct ThermalEventData {
    ordinal: u64,
    kind: u64,
    causes: Vec<TraceId>,
    target: CausalTarget,
    before: StateFingerprint,
    after: StateFingerprint,
}

fn thermal_event(mut data: ThermalEventData) -> Result<CausalEventProposal, RuntimeError> {
    data.causes.sort_unstable();
    data.causes.dedup();
    Ok(CausalEventProposal::new(
        EventProposalKey::new(THERMAL_EVOLUTION_SYSTEM_ID, data.ordinal, 0),
        EventKindId::new(data.kind),
        data.causes,
        vec![CausalEffect::new(data.target, data.before, data.after)?],
    )?)
}

fn conservation_parents(
    state: &RuntimeState,
    accepted: &BTreeMap<ThermalReservoirId, ThermalEnergy>,
) -> Vec<TraceId> {
    let mut parents = BTreeSet::new();
    if state.thermal_fields.batch_sequence() == 0 {
        for field in state.thermal_fields.fields().values() {
            parents.extend(field.last_change().iter().copied());
        }
    } else {
        parents.insert(state.thermal_fields.conservation_last_change());
    }
    for id in accepted.keys() {
        if let Some(reservoir) = state.thermal_reservoirs.get(id)
            && (state.thermal_fields.batch_sequence() == 0
                || reservoir.last_change == reservoir.bootstrap_trace)
        {
            parents.insert(reservoir.bootstrap_trace);
        }
    }
    parents.into_iter().collect()
}

fn thermal_carrier_id(active_region: &ThermalActiveRegion) -> u64 {
    active_region
        .active_chunks()
        .iter()
        .fold(0x0054_4845_524D_414C_u64, |identity, chunk| {
            crate::digests::mix64(identity ^ chart_chunk_hash(*chunk))
        })
}

fn next_ordinal(ordinal: u64) -> Result<u64, RuntimeError> {
    ordinal.checked_add(1).ok_or(RuntimeError::CausalCommit(
        CausalCommitError::IdentifierExhausted,
    ))
}
