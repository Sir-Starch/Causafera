use std::collections::{BTreeMap, BTreeSet};

use causafera_types::{ThermalEnergy, TraceId};

use super::{
    ThermalCellChange, ThermalCellKey, ThermalCellTransferReceipt, ThermalConservationReceipt,
    ThermalError, ThermalFaceRecord, ThermalMaterialSite, ThermalMaterialTransferRecord,
    ThermalReservoir, ThermalReservoirId, ThermalReservoirTransferRecord,
    arithmetic::{energy_from_i128, sum_i128},
};

#[allow(clippy::too_many_arguments)]
pub(crate) fn records_for_cells(
    committed: &BTreeMap<ThermalCellKey, i128>,
    pre_state: &BTreeMap<ThermalCellKey, i128>,
    after_energy: &BTreeMap<ThermalCellKey, ThermalEnergy>,
    prior_traces: &BTreeMap<ThermalCellKey, TraceId>,
    reservoirs: &BTreeMap<ThermalReservoirId, ThermalReservoir>,
    faces: &BTreeMap<ThermalCellKey, Vec<ThermalFaceRecord>>,
    reservoir_records: &BTreeMap<ThermalCellKey, Vec<ThermalReservoirTransferRecord>>,
    materials: &BTreeMap<ThermalCellKey, ThermalMaterialSite>,
    material_records: &BTreeMap<ThermalCellKey, ThermalMaterialTransferRecord>,
) -> Result<(Vec<ThermalCellChange>, Vec<ThermalCellTransferReceipt>), ThermalError> {
    let mut changes = Vec::new();
    let mut receipts = Vec::new();
    for (key, committed_energy) in committed {
        let cell_faces = faces.get(key).cloned().unwrap_or_default();
        let cell_reservoirs = reservoir_records.get(key).cloned().unwrap_or_default();
        let cell_material = material_records.get(key).copied();
        if cell_faces.is_empty() && cell_reservoirs.is_empty() && cell_material.is_none() {
            continue;
        }
        let before = energy_from_i128(*committed_energy)?;
        let logical_pre_state = energy_from_i128(
            *pre_state
                .get(key)
                .ok_or(ThermalError::PositionOutsideField)?,
        )?;
        let after = *after_energy
            .get(key)
            .ok_or(ThermalError::PositionOutsideField)?;
        if before != after {
            let mut parents = BTreeSet::new();
            parents.insert(
                *prior_traces
                    .get(key)
                    .ok_or(ThermalError::PositionOutsideField)?,
            );
            for face in &cell_faces {
                parents.insert(
                    *prior_traces
                        .get(&face.neighbor)
                        .ok_or(ThermalError::PositionOutsideField)?,
                );
            }
            for record in &cell_reservoirs {
                if record.accepted_injection != ThermalEnergy::ZERO {
                    parents.insert(
                        reservoirs
                            .get(&record.id)
                            .ok_or(ThermalError::UnknownReservoir)?
                            .last_change,
                    );
                }
            }
            if cell_material.is_some()
                && let Some(trace) = materials
                    .get(key)
                    .ok_or(ThermalError::PositionOutsideField)?
                    .last_exchange
            {
                parents.insert(trace);
            }
            changes.push(ThermalCellChange {
                cell: *key,
                before,
                after,
                parent_traces: parents.into_iter().collect(),
                incident_faces: cell_faces.clone(),
                reservoirs: cell_reservoirs.clone(),
                material: cell_material,
            });
        }
        receipts.push(ThermalCellTransferReceipt {
            cell: *key,
            pre_state: logical_pre_state,
            post_state: after,
            cell_change_trace_id: None,
            faces: cell_faces,
            reservoirs: cell_reservoirs,
            material: cell_material,
        });
    }
    Ok((changes, receipts))
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn conservation_receipt(
    tick: u64,
    committed: &BTreeMap<ThermalCellKey, i128>,
    after: &BTreeMap<ThermalCellKey, ThermalEnergy>,
    reservoirs_before: &[ThermalReservoir],
    reservoirs_after: &BTreeMap<ThermalReservoirId, ThermalEnergy>,
    materials_before: &BTreeMap<ThermalCellKey, ThermalMaterialSite>,
    materials_after: &BTreeMap<ThermalCellKey, ThermalEnergy>,
) -> Result<ThermalConservationReceipt, ThermalError> {
    let total_cell_energy_before = sum_i128(committed.values().copied())?;
    let total_cell_energy_after = sum_i128(after.values().map(|value| i128::from(value.get())))?;
    let total_reservoir_budget_before = sum_i128(
        reservoirs_before
            .iter()
            .map(|reservoir| i128::from(reservoir.budget.get())),
    )?;
    let total_reservoir_budget_after = sum_i128(
        reservoirs_after
            .values()
            .map(|value| i128::from(value.get())),
    )?;
    let total_material_retained_before = sum_i128(
        materials_before
            .values()
            .map(|site| i128::from(site.retained_before.get())),
    )?;
    let total_material_retained_after = sum_i128(
        materials_after
            .values()
            .map(|value| i128::from(value.get())),
    )?;
    let total_before = total_cell_energy_before
        .checked_add(total_reservoir_budget_before)
        .ok_or(ThermalError::ArithmeticOverflow)?
        .checked_add(total_material_retained_before)
        .ok_or(ThermalError::ArithmeticOverflow)?;
    let total_after = total_cell_energy_after
        .checked_add(total_reservoir_budget_after)
        .ok_or(ThermalError::ArithmeticOverflow)?
        .checked_add(total_material_retained_after)
        .ok_or(ThermalError::ArithmeticOverflow)?;
    let residual = total_after
        .checked_sub(total_before)
        .ok_or(ThermalError::ArithmeticOverflow)?;
    if residual != 0 {
        return Err(ThermalError::ConservationViolation(residual));
    }
    Ok(ThermalConservationReceipt {
        tick,
        total_cell_energy_before,
        total_cell_energy_after,
        total_reservoir_budget_before,
        total_reservoir_budget_after,
        total_material_retained_before,
        total_material_retained_after,
        residual,
    })
}
