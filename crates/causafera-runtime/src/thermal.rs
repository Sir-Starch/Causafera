use crate::*;
use causafera_core::*;
use causafera_domains::*;
use causafera_types::*;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use crate::thermal_events::*;

pub(crate) const MAX_THERMAL_PROJECTION_BATCHES: usize = 64;
pub(crate) const THERMAL_TRANSFER_EVIDENCE_SCHEMA: EvidenceSchemaId =
    EvidenceSchemaId::new(0x0014_1701);
pub(crate) const THERMAL_CONSERVATION_EVIDENCE_SCHEMA: EvidenceSchemaId =
    EvidenceSchemaId::new(0x0014_1702);
pub(crate) const THERMAL_BOUNDARY_EVIDENCE_SCHEMA: EvidenceSchemaId =
    EvidenceSchemaId::new(0x0014_1703);
pub(crate) const THERMAL_EVIDENCE_VERSION: EvidenceSchemaVersion = EvidenceSchemaVersion::new(1, 0);

pub(crate) struct ThermalReservoirSystem {
    state: Arc<Mutex<RuntimeState>>,
    next_time: SimulationTime,
}

impl ThermalReservoirSystem {
    pub(crate) fn new(state: Arc<Mutex<RuntimeState>>) -> Self {
        Self {
            state,
            next_time: SimulationTime::new(1),
        }
    }

    fn execute(&mut self) -> Result<(), RuntimeError> {
        let mut state = self.state.lock().map_err(|_| RuntimeError::StatePoisoned)?;
        let mut injections = Vec::new();
        for reservoir in state.thermal_reservoirs.values() {
            if !state
                .thermal_active_region
                .resident_chunks()
                .contains(&reservoir.target.chunk)
                || state.thermal_fields.field(reservoir.target.chunk).is_none()
            {
                return Err(RuntimeError::ThermalRegionIncomplete);
            }
            let scheduled_amount = match reservoir.schedule {
                ThermalReservoirSchedule::PerTick(amount) => amount.min(reservoir.budget),
                ThermalReservoirSchedule::OneShot => reservoir.budget,
            };
            if scheduled_amount == ThermalEnergy::ZERO {
                continue;
            }
            injections.push(ThermalInjectionProposal {
                reservoir_id: reservoir.id,
                target: reservoir.target,
                scheduled_amount,
            });
        }
        injections.sort_unstable_by_key(|proposal| proposal.reservoir_id);
        state.replace_pending_thermal_injections(injections);
        self.next_time = self.next_time.tick();
        Ok(())
    }
}

impl System for ThermalReservoirSystem {
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

pub(crate) struct ThermalEvolutionSystem {
    state: Arc<Mutex<RuntimeState>>,
    next_time: SimulationTime,
}

#[derive(Clone, Copy)]
pub(super) enum ThermalEventSubject {
    Reservoir(ThermalReservoirId),
    Material(ThermalCellKey),
    Cell(ThermalCellKey),
    Conservation,
}

pub(super) struct ThermalEvent {
    pub(super) proposal: CausalEventProposal,
    pub(super) subject: ThermalEventSubject,
}

impl ThermalEvolutionSystem {
    pub(crate) fn new(state: Arc<Mutex<RuntimeState>>) -> Self {
        Self {
            state,
            next_time: SimulationTime::new(1),
        }
    }

    fn execute(&mut self) -> Result<(), RuntimeError> {
        let mut state = self.state.lock().map_err(|_| RuntimeError::StatePoisoned)?;
        if state.failure.is_some() {
            self.next_time = self.next_time.tick();
            return Ok(());
        }
        let reservoirs = state
            .thermal_reservoirs
            .values()
            .copied()
            .collect::<Vec<_>>();
        let materials = state
            .material_surfaces
            .iter()
            .map(|(id, surface)| {
                (
                    ThermalCellKey::new(id.chunk, id.cell_index),
                    ThermalMaterialSite {
                        retained_before: surface.thermal.retained_energy,
                        last_exchange: surface.thermal.last_exchange,
                    },
                )
            })
            .collect::<BTreeMap<_, _>>();
        let evolution = state
            .thermal_fields
            .propose_evolution(ThermalEvolutionRequest {
                tick: self.next_time.raw(),
                parameters: state.thermal_parameters,
                active_region: &state.thermal_active_region,
                boundary_behavior: ThermalBoundaryBehavior::NoFluxOutsideActiveRegion,
                reservoirs: &reservoirs,
                injections: &state.pending_thermal_injections,
                materials: &materials,
            })?;
        let accepted = accepted_reservoirs(evolution.transfer_receipts());
        let mut events = build_thermal_events(&state, &evolution, &accepted)?;
        events.sort_unstable_by_key(|event| event.proposal.key());
        let proposals = events.iter().map(|event| event.proposal.clone()).collect();
        let traces = state
            .traces
            .commit_batch(self.next_time, Phase::Physics, proposals)?;
        let mut reservoir_traces = BTreeMap::new();
        let mut material_traces = BTreeMap::new();
        let mut cell_traces = BTreeMap::new();
        let mut conservation_trace = state.thermal_fields.conservation_last_change();
        for (event, trace) in events.iter().zip(traces) {
            match event.subject {
                ThermalEventSubject::Reservoir(id) => {
                    reservoir_traces.insert(id, trace);
                }
                ThermalEventSubject::Material(cell) => {
                    material_traces.insert(cell, trace);
                }
                ThermalEventSubject::Cell(cell) => {
                    cell_traces.insert(cell, trace);
                }
                ThermalEventSubject::Conservation => conservation_trace = trace,
            }
        }
        let mut receipts = evolution.transfer_receipts().to_vec();
        install_receipt_traces(&mut receipts, &cell_traces, &reservoir_traces);
        let evidence = thermal_evidence_records(
            self.next_time,
            conservation_trace,
            &receipts,
            evolution.conservation_receipt(),
            evolution.boundary_records(),
        )?;
        state.stage_history_evidence(&evidence)?;
        // Collect the bounded transition records first (reading `state.thermal_fields` and
        // `state.material_surfaces` immutably) so recording them afterward does not overlap
        // with the mutable surface-state update below.
        let mut thermal_transitions = Vec::new();
        for receipt in evolution.transfer_receipts() {
            let Some(material) = &receipt.material else {
                continue;
            };
            let Some(trace) = material_traces.get(&receipt.cell).copied() else {
                continue;
            };
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
            thermal_transitions.push(MaterialSurfaceThermalTransition {
                id: MaterialSurfaceId::new(receipt.cell.chunk, receipt.cell.cell_index),
                occurred_at: self.next_time,
                before_retained: material.retained_before.get(),
                after_retained: material.retained_after.get(),
                cell_pre_state: receipt.pre_state.get(),
                signed_flux: material.signed_flux,
                cell_trace,
                transition_trace: trace,
            });
        }
        for transition in &thermal_transitions {
            state.record_material_surface_before(transition.id);
            if let Some(surface) = state.material_surfaces.get_mut(&transition.id) {
                surface.thermal.retained_energy = ThermalEnergy::new(transition.after_retained)
                    .map_err(|_| RuntimeError::Thermal(ThermalError::EnergyOutOfBounds))?;
                surface.thermal.last_exchange = Some(transition.transition_trace);
            }
        }
        for transition in thermal_transitions {
            record_material_surface_thermal_transition(&mut state, transition);
        }
        let mut after_fields = evolution.after_state().clone();
        after_fields.install_committed_traces(ThermalCommittedTraces {
            changes: evolution.cell_changes(),
            receipts: &receipts,
            cell_traces: &cell_traces,
            reservoir_traces: &reservoir_traces,
            conservation_trace,
        });
        let changed_reservoirs = state
            .thermal_reservoirs
            .keys()
            .filter(|id| {
                evolution.reservoir_budgets_after().contains_key(id)
                    || reservoir_traces.contains_key(id)
            })
            .copied()
            .collect::<Vec<_>>();
        for id in changed_reservoirs {
            state.record_thermal_reservoir_before(id);
        }
        for (id, reservoir) in &mut state.thermal_reservoirs {
            if let Some(budget) = evolution.reservoir_budgets_after().get(id) {
                reservoir.budget = *budget;
            }
            if let Some(trace) = reservoir_traces.get(id) {
                reservoir.last_change = *trace;
            }
        }
        state.replace_thermal_fields(after_fields);
        state.replace_thermal_boundary_records(evolution.boundary_records().to_vec());
        install_bounded_thermal_projection(
            &mut state,
            conservation_trace,
            receipts,
            *evolution.conservation_receipt(),
        );
        state.clear_pending_thermal_injections();
        self.next_time = self.next_time.tick();
        Ok(())
    }
}

fn thermal_evidence_records(
    time: SimulationTime,
    conservation_trace: TraceId,
    receipts: &[ThermalCellTransferReceipt],
    conservation: &ThermalConservationReceipt,
    boundaries: &[ThermalBoundaryRecord],
) -> Result<Vec<DomainEvidenceRecord>, RuntimeError> {
    let mut records = Vec::with_capacity(
        receipts
            .len()
            .checked_add(boundaries.len())
            .and_then(|count| count.checked_add(1))
            .ok_or(HistoryError::ArithmeticOverflow)?,
    );
    let mut cell_anchors = BTreeMap::<ThermalCellKey, Vec<TraceId>>::new();
    for receipt in receipts {
        let mut anchors = vec![conservation_trace];
        anchors.extend(receipt.cell_change_trace_id);
        anchors.extend(
            receipt
                .reservoirs
                .iter()
                .filter_map(|record| record.transfer_trace_id),
        );
        anchors.sort_unstable();
        anchors.dedup();
        cell_anchors.insert(receipt.cell, anchors.clone());
        records.push(thermal_evidence_record(
            conservation_trace,
            THERMAL_TRANSFER_EVIDENCE_SCHEMA,
            time,
            anchors,
            thermal_transfer_key(conservation_trace, receipt.cell),
            encode_thermal_transfer_payload(receipt)?,
        )?);
    }
    records.push(thermal_evidence_record(
        conservation_trace,
        THERMAL_CONSERVATION_EVIDENCE_SCHEMA,
        time,
        vec![conservation_trace],
        conservation_trace.raw().to_le_bytes().to_vec(),
        encode_thermal_conservation_payload(conservation),
    )?);
    for boundary in boundaries {
        records.push(thermal_evidence_record(
            conservation_trace,
            THERMAL_BOUNDARY_EVIDENCE_SCHEMA,
            time,
            cell_anchors
                .get(&boundary.cell)
                .cloned()
                .unwrap_or_else(|| vec![conservation_trace]),
            thermal_boundary_key(conservation_trace, boundary),
            encode_thermal_boundary_payload(boundary),
        )?);
    }
    records.sort_unstable();
    Ok(records)
}

fn thermal_evidence_record(
    conservation_trace: TraceId,
    schema: EvidenceSchemaId,
    time: SimulationTime,
    anchors: Vec<TraceId>,
    key: Vec<u8>,
    payload: Vec<u8>,
) -> Result<DomainEvidenceRecord, RuntimeError> {
    let payload_digest = StateFingerprint::new(*blake3::hash(&payload).as_bytes());
    Ok(DomainEvidenceRecord::new(
        conservation_trace.raw(),
        schema,
        THERMAL_EVIDENCE_VERSION,
        Criticality::ExplanationOnly,
        time,
        anchors,
        key,
        payload,
        payload_digest,
    )?)
}

fn thermal_transfer_key(conservation_trace: TraceId, cell: ThermalCellKey) -> Vec<u8> {
    let mut key = conservation_trace.raw().to_le_bytes().to_vec();
    encode_thermal_cell_key(&mut key, cell);
    key
}

fn thermal_boundary_key(conservation_trace: TraceId, boundary: &ThermalBoundaryRecord) -> Vec<u8> {
    let mut key = thermal_transfer_key(conservation_trace, boundary.cell);
    encode_thermal_cell_key(&mut key, boundary.neighbor);
    key
}

fn encode_thermal_transfer_payload(
    receipt: &ThermalCellTransferReceipt,
) -> Result<Vec<u8>, RuntimeError> {
    let mut payload = Vec::new();
    encode_thermal_cell_key(&mut payload, receipt.cell);
    encode_thermal_energy(&mut payload, receipt.pre_state);
    encode_thermal_energy(&mut payload, receipt.post_state);
    encode_optional_trace(&mut payload, receipt.cell_change_trace_id);
    encode_len(&mut payload, receipt.faces.len())?;
    for face in &receipt.faces {
        encode_thermal_cell_key(&mut payload, face.neighbor);
        payload.extend_from_slice(&face.signed_flux.to_le_bytes());
        encode_thermal_energy(&mut payload, face.neighbor_pre_state);
    }
    encode_len(&mut payload, receipt.reservoirs.len())?;
    for reservoir in &receipt.reservoirs {
        payload.extend_from_slice(&reservoir.id.raw().to_le_bytes());
        encode_thermal_energy(&mut payload, reservoir.scheduled_injection);
        encode_thermal_energy(&mut payload, reservoir.accepted_injection);
        encode_thermal_energy(&mut payload, reservoir.rejected_injection);
        encode_optional_trace(&mut payload, reservoir.transfer_trace_id);
    }
    match receipt.material {
        Some(material) => {
            payload.push(1);
            encode_thermal_energy(&mut payload, material.retained_before);
            encode_thermal_energy(&mut payload, material.retained_after);
            payload.extend_from_slice(&material.signed_flux.to_le_bytes());
            encode_thermal_energy(&mut payload, material.rejected);
        }
        None => payload.push(0),
    }
    Ok(payload)
}

fn encode_thermal_conservation_payload(receipt: &ThermalConservationReceipt) -> Vec<u8> {
    let mut payload = receipt.tick.to_le_bytes().to_vec();
    for value in [
        receipt.total_cell_energy_before,
        receipt.total_cell_energy_after,
        receipt.total_reservoir_budget_before,
        receipt.total_reservoir_budget_after,
        receipt.total_material_retained_before,
        receipt.total_material_retained_after,
        receipt.residual,
    ] {
        payload.extend_from_slice(&value.to_le_bytes());
    }
    payload
}

fn encode_thermal_boundary_payload(record: &ThermalBoundaryRecord) -> Vec<u8> {
    let mut payload = Vec::new();
    encode_thermal_cell_key(&mut payload, record.cell);
    encode_thermal_cell_key(&mut payload, record.neighbor);
    encode_thermal_energy(&mut payload, record.cell_pre_state);
    payload
}

fn encode_thermal_cell_key(output: &mut Vec<u8>, cell: ThermalCellKey) {
    output.extend_from_slice(&cell.chunk.chart.raw().to_le_bytes());
    output.extend_from_slice(&cell.chunk.chunk.x.to_le_bytes());
    output.extend_from_slice(&cell.chunk.chunk.y.to_le_bytes());
    output.extend_from_slice(&cell.chunk.chunk.z.to_le_bytes());
    output.extend_from_slice(&cell.cell_index.to_le_bytes());
}

fn encode_thermal_energy(output: &mut Vec<u8>, energy: ThermalEnergy) {
    output.extend_from_slice(&energy.get().to_le_bytes());
}

fn encode_optional_trace(output: &mut Vec<u8>, trace: Option<TraceId>) {
    match trace {
        Some(trace) => {
            output.push(1);
            output.extend_from_slice(&trace.raw().to_le_bytes());
        }
        None => output.push(0),
    }
}

fn encode_len(output: &mut Vec<u8>, len: usize) -> Result<(), RuntimeError> {
    output.extend_from_slice(
        &u64::try_from(len)
            .map_err(|_| HistoryError::ArithmeticOverflow)?
            .to_le_bytes(),
    );
    Ok(())
}

fn install_bounded_thermal_projection(
    state: &mut RuntimeState,
    conservation_trace: TraceId,
    receipts: Vec<ThermalCellTransferReceipt>,
    conservation: ThermalConservationReceipt,
) {
    state.thermal_receipts.insert(conservation_trace, receipts);
    state
        .thermal_conservation_receipts
        .insert(conservation_trace, conservation);
    let evicted_receipts = (state.thermal_receipts.len() > MAX_THERMAL_PROJECTION_BATCHES)
        .then(|| state.thermal_receipts.pop_first())
        .flatten();
    let evicted_conservation = (state.thermal_conservation_receipts.len()
        > MAX_THERMAL_PROJECTION_BATCHES)
        .then(|| state.thermal_conservation_receipts.pop_first())
        .flatten();
    state.tick_undo.push_undo(move |state| {
        if let Some((trace, receipts)) = evicted_receipts {
            state.thermal_receipts.insert(trace, receipts);
        }
        if let Some((trace, receipt)) = evicted_conservation {
            state.thermal_conservation_receipts.insert(trace, receipt);
        }
    });
}

impl System for ThermalEvolutionSystem {
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

#[cfg(test)]
mod tests;
