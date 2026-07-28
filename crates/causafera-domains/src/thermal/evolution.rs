use std::collections::{BTreeMap, BTreeSet};

use causafera_types::{ChartChunkCoord, TraceId};

use super::{
    ThermalCellKey, ThermalError, ThermalEvolutionProposal, ThermalEvolutionRequest,
    ThermalFieldSet,
    diffusion::preflight_faces,
    injection::{accept_injections, index_reservoirs},
    receipts::{conservation_receipt, records_for_cells},
};

type CellEnergy = BTreeMap<ThermalCellKey, i128>;
type CellTraces = BTreeMap<ThermalCellKey, TraceId>;

impl ThermalFieldSet {
    pub fn propose_evolution(
        &self,
        request: ThermalEvolutionRequest<'_>,
    ) -> Result<ThermalEvolutionProposal, ThermalError> {
        let parameters = request.parameters.validate()?;
        validate_region(
            self,
            request.active_region.active_chunks(),
            request.active_region.resident_chunks(),
        )?;
        let (committed, prior_traces) = collect_cell_state(self)?;
        let reservoirs = index_reservoirs(request.reservoirs)?;
        let (pre_state, budgets_after, reservoir_records) =
            accept_injections(&committed, &reservoirs, request.injections)?;
        let (after_energy, faces, boundary_records, materials_after, material_records) =
            preflight_faces(
                self,
                request.active_region,
                &pre_state,
                parameters,
                request.materials,
            )?;
        let after_state = self.with_energy(&after_energy)?;
        let (cell_changes, transfer_receipts) = records_for_cells(
            &committed,
            &pre_state,
            &after_energy,
            &prior_traces,
            &reservoirs,
            &faces,
            &reservoir_records,
            request.materials,
            &material_records,
        )?;
        let conservation_receipt = conservation_receipt(
            request.tick,
            &committed,
            &after_energy,
            request.reservoirs,
            &budgets_after,
            request.materials,
            &materials_after,
        )?;
        Ok(ThermalEvolutionProposal::new(
            after_state,
            cell_changes,
            conservation_receipt,
            transfer_receipts,
            budgets_after,
            boundary_records,
            materials_after,
        ))
    }
}

fn validate_region(
    fields: &ThermalFieldSet,
    active: &BTreeSet<ChartChunkCoord>,
    resident: &BTreeSet<ChartChunkCoord>,
) -> Result<(), ThermalError> {
    for chunk in fields.fields().keys() {
        if !active.contains(chunk) {
            return Err(ThermalError::FieldOutsideActiveRegion(*chunk));
        }
    }
    for chunk in active {
        if !resident.contains(chunk) || fields.field(*chunk).is_none() {
            return Err(ThermalError::ActiveRegionIncomplete(*chunk));
        }
    }
    Ok(())
}

fn collect_cell_state(fields: &ThermalFieldSet) -> Result<(CellEnergy, CellTraces), ThermalError> {
    let mut energy = BTreeMap::new();
    let mut traces = BTreeMap::new();
    for (chunk, field) in fields.fields() {
        for (index, value) in field.energy().iter().enumerate() {
            let cell_index =
                u16::try_from(index).map_err(|_| ThermalError::PositionOutsideField)?;
            let key = ThermalCellKey::new(*chunk, cell_index);
            energy.insert(key, i128::from(value.get()));
            let trace = field
                .last_change()
                .get(index)
                .copied()
                .ok_or(ThermalError::PositionOutsideField)?;
            traces.insert(key, trace);
        }
    }
    Ok((energy, traces))
}
