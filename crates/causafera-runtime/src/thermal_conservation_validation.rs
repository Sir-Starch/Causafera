use std::collections::BTreeMap;

use causafera_domains::ThermalConservationReceipt;
use causafera_types::TraceId;

use crate::RuntimeError;
use crate::RuntimeState;

/// Per-batch receipt-side totals accumulated during snapshot import.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct ThermalBatchReceiptTotals {
    /// Σ (post_state - pre_state) across all transfer receipts in the batch.
    pub(crate) cell_transition: i128,
    /// Σ Σ accepted_injection across all reservoir records in all transfer receipts in the batch.
    pub(crate) accepted_injection: i128,
    /// Σ signed_flux across all material records in all transfer receipts in the batch.
    pub(crate) material_flux: i128,
}

pub(crate) fn accumulate_receipt_totals(
    receipt: &causafera_domains::ThermalCellTransferReceipt,
    totals: &mut ThermalBatchReceiptTotals,
) -> Result<(), RuntimeError> {
    let cell_delta = i128::from(receipt.post_state.get())
        .checked_sub(i128::from(receipt.pre_state.get()))
        .ok_or(RuntimeError::InvalidSnapshot(
            "thermal receipt cell transition overflows",
        ))?;
    totals.cell_transition =
        totals
            .cell_transition
            .checked_add(cell_delta)
            .ok_or(RuntimeError::InvalidSnapshot(
                "thermal batch cell transition total overflows",
            ))?;
    for record in &receipt.reservoirs {
        totals.accepted_injection = totals
            .accepted_injection
            .checked_add(i128::from(record.accepted_injection.get()))
            .ok_or(RuntimeError::InvalidSnapshot(
                "thermal batch accepted injection total overflows",
            ))?;
    }
    if let Some(material) = &receipt.material {
        totals.material_flux = totals
            .material_flux
            .checked_add(i128::from(material.signed_flux))
            .ok_or(RuntimeError::InvalidSnapshot(
                "thermal batch material flux total overflows",
            ))?;
    }
    Ok(())
}

/// Validate every `ThermalConservationReceipt` aggregate literal against checked sums of the
/// materialized final state and the per-receipt data collected during import.
///
/// Implements identities I2, I3, I3a, I5, and I6 from the thermal aggregate validation plan.
/// I1 (reservoir budget identity) is already enforced by `import_thermal_snapshot`.
/// I4 (zero residual) is also already enforced there; this function re-checks it defensively.
pub(crate) fn validate_thermal_aggregate_conservation(
    state: &RuntimeState,
    receipt_totals: &BTreeMap<TraceId, ThermalBatchReceiptTotals>,
) -> Result<(), RuntimeError> {
    if state.thermal_fields.batch_sequence() == 0 {
        return Ok(());
    }

    let receipts_by_trace: BTreeMap<TraceId, &ThermalConservationReceipt> = state
        .thermal_conservation_receipts
        .iter()
        .map(|(trace, receipt)| (*trace, receipt))
        .collect();
    let traces: Vec<TraceId> = receipts_by_trace.keys().copied().collect();

    // I6 — terminal anchor against fully materialized final state.
    let total_cell_energy = state
        .thermal_fields
        .fields()
        .values()
        .flat_map(|field| field.energy())
        .try_fold(0_i128, |sum, energy| {
            sum.checked_add(i128::from(energy.get()))
                .ok_or(RuntimeError::InvalidSnapshot(
                    "thermal cell energy total overflows",
                ))
        })?;
    let total_reservoir_budget =
        state
            .thermal_reservoirs
            .values()
            .try_fold(0_i128, |sum, reservoir| {
                sum.checked_add(i128::from(reservoir.budget.get())).ok_or(
                    RuntimeError::InvalidSnapshot("thermal reservoir budget total overflows"),
                )
            })?;
    let total_material_retained =
        state
            .material_surfaces
            .values()
            .try_fold(0_i128, |sum, surface| {
                sum.checked_add(i128::from(surface.thermal.retained_energy.get()))
                    .ok_or(RuntimeError::InvalidSnapshot(
                        "thermal material retained total overflows",
                    ))
            })?;

    let last_trace = traces.last().ok_or(RuntimeError::InvalidSnapshot(
        "thermal batch sequence has no conservation receipt",
    ))?;
    let last_receipt = receipts_by_trace[last_trace];

    if total_cell_energy != last_receipt.total_cell_energy_after {
        return Err(RuntimeError::InvalidSnapshot(
            "thermal cell energy total does not match terminal conservation receipt",
        ));
    }
    if total_reservoir_budget != last_receipt.total_reservoir_budget_after {
        return Err(RuntimeError::InvalidSnapshot(
            "thermal reservoir budget total does not match terminal conservation receipt",
        ));
    }
    if total_material_retained != last_receipt.total_material_retained_after {
        return Err(RuntimeError::InvalidSnapshot(
            "thermal material retained total does not match terminal conservation receipt",
        ));
    }

    // I5 — chain identity between consecutive batches.
    for window in traces.windows(2) {
        let prev = receipts_by_trace[&window[0]];
        let next = receipts_by_trace[&window[1]];
        if prev.total_cell_energy_after != next.total_cell_energy_before {
            return Err(RuntimeError::InvalidSnapshot(
                "thermal cell energy chain broken between conservation receipts",
            ));
        }
        if prev.total_reservoir_budget_after != next.total_reservoir_budget_before {
            return Err(RuntimeError::InvalidSnapshot(
                "thermal reservoir budget chain broken between conservation receipts",
            ));
        }
        if prev.total_material_retained_after != next.total_material_retained_before {
            return Err(RuntimeError::InvalidSnapshot(
                "thermal material retained chain broken between conservation receipts",
            ));
        }
    }

    // I2, I3, I4 — per-batch aggregate identities.
    for trace in &traces {
        let receipt = receipts_by_trace[trace];
        let totals = receipt_totals
            .get(trace)
            .ok_or(RuntimeError::InvalidSnapshot(
                "thermal conservation receipt missing receipt-side totals",
            ))?;

        let expected_cell_delta = totals
            .cell_transition
            .checked_add(totals.accepted_injection)
            .ok_or(RuntimeError::InvalidSnapshot(
                "thermal cell delta computation overflows",
            ))?;
        let actual_cell_delta = receipt
            .total_cell_energy_after
            .checked_sub(receipt.total_cell_energy_before)
            .ok_or(RuntimeError::InvalidSnapshot(
                "thermal cell energy delta overflows",
            ))?;
        if expected_cell_delta != actual_cell_delta {
            return Err(RuntimeError::InvalidSnapshot(
                "thermal cell energy delta does not match receipt totals",
            ));
        }

        let actual_material_delta = receipt
            .total_material_retained_after
            .checked_sub(receipt.total_material_retained_before)
            .ok_or(RuntimeError::InvalidSnapshot(
                "thermal material retained delta overflows",
            ))?;
        if totals.material_flux != actual_material_delta {
            return Err(RuntimeError::InvalidSnapshot(
                "thermal material retained delta does not match receipt totals",
            ));
        }

        let before = receipt
            .total_cell_energy_before
            .checked_add(receipt.total_reservoir_budget_before)
            .and_then(|sum| sum.checked_add(receipt.total_material_retained_before))
            .ok_or(RuntimeError::InvalidSnapshot(
                "thermal conservation before-total overflows",
            ))?;
        let after = receipt
            .total_cell_energy_after
            .checked_add(receipt.total_reservoir_budget_after)
            .and_then(|sum| sum.checked_add(receipt.total_material_retained_after))
            .ok_or(RuntimeError::InvalidSnapshot(
                "thermal conservation after-total overflows",
            ))?;
        if before != after {
            return Err(RuntimeError::InvalidSnapshot(
                "thermal conservation receipt has non-zero residual",
            ));
        }
    }

    Ok(())
}

#[cfg(test)]
#[path = "thermal_conservation_validation_test.rs"]
mod tests;
