use crate::*;
use causafera_core::*;
use causafera_domains::*;
use causafera_types::*;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use crate::thermal_events::*;

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
        state.pending_thermal_injections = injections;
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
        for (id, reservoir) in &mut state.thermal_reservoirs {
            if let Some(budget) = evolution.reservoir_budgets_after().get(id) {
                reservoir.budget = *budget;
            }
            if let Some(trace) = reservoir_traces.get(id) {
                reservoir.last_change = *trace;
            }
        }
        state.thermal_fields = after_fields;
        state.thermal_boundary_records = evolution.boundary_records().to_vec();
        state
            .thermal_receipts
            .entry(conservation_trace)
            .or_default()
            .extend(receipts);
        state
            .thermal_conservation_receipts
            .insert(conservation_trace, *evolution.conservation_receipt());
        state.pending_thermal_injections.clear();
        self.next_time = self.next_time.tick();
        Ok(())
    }
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
