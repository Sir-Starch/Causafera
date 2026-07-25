use std::collections::{BTreeMap, BTreeSet};

use causafera_types::{ThermalEnergy, TraceId};

use super::{
    ThermalCellChange, ThermalCellKey, ThermalCellTransferReceipt, ThermalConservationReceipt,
    ThermalError, ThermalFaceRecord, ThermalReservoir, ThermalReservoirId,
    ThermalReservoirTransferRecord,
    arithmetic::{energy_from_i128, sum_i128},
};

pub(crate) fn records_for_cells(
    committed: &BTreeMap<ThermalCellKey, i128>,
    pre_state: &BTreeMap<ThermalCellKey, i128>,
    after_energy: &BTreeMap<ThermalCellKey, ThermalEnergy>,
    prior_traces: &BTreeMap<ThermalCellKey, TraceId>,
    reservoirs: &BTreeMap<ThermalReservoirId, ThermalReservoir>,
    faces: &BTreeMap<ThermalCellKey, Vec<ThermalFaceRecord>>,
    reservoir_records: &BTreeMap<ThermalCellKey, Vec<ThermalReservoirTransferRecord>>,
) -> Result<(Vec<ThermalCellChange>, Vec<ThermalCellTransferReceipt>), ThermalError> {
    let mut changes = Vec::new();
    let mut receipts = Vec::new();
    for (key, committed_energy) in committed {
        let cell_faces = faces.get(key).cloned().unwrap_or_default();
        let cell_reservoirs = reservoir_records.get(key).cloned().unwrap_or_default();
        if cell_faces.is_empty() && cell_reservoirs.is_empty() {
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
            changes.push(ThermalCellChange {
                cell: *key,
                before,
                after,
                parent_traces: parents.into_iter().collect(),
                incident_faces: cell_faces.clone(),
                reservoirs: cell_reservoirs.clone(),
            });
        }
        receipts.push(ThermalCellTransferReceipt {
            cell: *key,
            pre_state: logical_pre_state,
            post_state: after,
            cell_change_trace_id: None,
            faces: cell_faces,
            reservoirs: cell_reservoirs,
        });
    }
    Ok((changes, receipts))
}

pub(crate) fn conservation_receipt(
    tick: u64,
    committed: &BTreeMap<ThermalCellKey, i128>,
    after: &BTreeMap<ThermalCellKey, ThermalEnergy>,
    reservoirs_before: &[ThermalReservoir],
    reservoirs_after: &BTreeMap<ThermalReservoirId, ThermalEnergy>,
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
    let total_before = total_cell_energy_before
        .checked_add(total_reservoir_budget_before)
        .ok_or(ThermalError::ArithmeticOverflow)?;
    let total_after = total_cell_energy_after
        .checked_add(total_reservoir_budget_after)
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
        residual,
    })
}
