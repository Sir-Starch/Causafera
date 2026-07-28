use causafera_domains::{
    ThermalCellKey, ThermalCellTransferReceipt, ThermalEnergy, ThermalFieldSet,
    ThermalMaterialTransferRecord, ThermalReservoirId, ThermalReservoirTransferRecord,
};
use causafera_types::{ChartChunkCoord, ChunkCoord, SpatialChartId, TraceId};

use super::*;
use crate::config::RuntimeConfig;
use crate::{Runtime, RuntimeError, RuntimeState};
use std::collections::BTreeMap;

fn energy(value: i64) -> ThermalEnergy {
    ThermalEnergy::new(value).expect("test energy must be non-negative")
}

fn cell_key() -> ThermalCellKey {
    ThermalCellKey::new(
        ChartChunkCoord::new(SpatialChartId::new(1), ChunkCoord::new(0, 0, 0)),
        0,
    )
}

fn receipt(
    pre: i64,
    post: i64,
    accepted_injection: i64,
    material_flux: Option<i64>,
) -> ThermalCellTransferReceipt {
    ThermalCellTransferReceipt {
        cell: cell_key(),
        pre_state: energy(pre),
        post_state: energy(post),
        cell_change_trace_id: None,
        faces: vec![],
        reservoirs: vec![ThermalReservoirTransferRecord {
            id: ThermalReservoirId::new(1),
            scheduled_injection: energy(accepted_injection),
            accepted_injection: energy(accepted_injection),
            rejected_injection: ThermalEnergy::ZERO,
            transfer_trace_id: None,
        }],
        material: material_flux.map(|flux| ThermalMaterialTransferRecord {
            retained_before: energy(0),
            retained_after: energy(0),
            signed_flux: flux,
            rejected: ThermalEnergy::ZERO,
        }),
    }
}

#[test]
fn accumulator_is_order_independent() {
    let first = receipt(10, 12, 5, Some(3));
    let second = receipt(20, 18, 0, Some(-1));

    let mut forward = ThermalBatchReceiptTotals::default();
    accumulate_receipt_totals(&first, &mut forward).unwrap();
    accumulate_receipt_totals(&second, &mut forward).unwrap();

    let mut reverse = ThermalBatchReceiptTotals::default();
    accumulate_receipt_totals(&second, &mut reverse).unwrap();
    accumulate_receipt_totals(&first, &mut reverse).unwrap();

    assert_eq!(forward, reverse);
}

#[test]
fn accumulator_computes_expected_totals() {
    let first = receipt(10, 12, 5, Some(3));
    let second = receipt(20, 18, 0, Some(-1));

    let mut totals = ThermalBatchReceiptTotals::default();
    accumulate_receipt_totals(&first, &mut totals).unwrap();
    accumulate_receipt_totals(&second, &mut totals).unwrap();

    assert_eq!(totals.cell_transition, (12 - 10) + (18 - 20));
    assert_eq!(totals.accepted_injection, 5);
    assert_eq!(totals.material_flux, 3 + (-1));
}

#[test]
fn accumulator_allows_missing_material_record() {
    let without_material = receipt(7, 9, 2, None);
    let mut totals = ThermalBatchReceiptTotals::default();
    accumulate_receipt_totals(&without_material, &mut totals).unwrap();
    assert_eq!(totals.cell_transition, 2);
    assert_eq!(totals.accepted_injection, 2);
    assert_eq!(totals.material_flux, 0);
}

#[test]
fn validator_rejects_missing_terminal_conservation_receipt() {
    let config = RuntimeConfig::new(42);
    let mut state = RuntimeState::new(&config).unwrap();

    // Set batch_sequence > 0 using from_snapshot_parts
    let old_fields = state.thermal_fields.fields().values().cloned().collect();
    state.thermal_fields = ThermalFieldSet::from_snapshot_parts(
        old_fields,
        1, // batch_sequence
        TraceId::new(1),
    )
    .unwrap();

    // Clear receipts while batch_sequence > 0
    state.thermal_conservation_receipts.clear();

    let receipt_totals = BTreeMap::new();
    let result = validate_thermal_aggregate_conservation(&state, &receipt_totals);

    assert!(matches!(
        result,
        Err(RuntimeError::InvalidSnapshot(
            "thermal batch sequence has no conservation receipt"
        ))
    ));
}

#[test]
fn validator_rejects_i4_aggregate_imbalance() {
    let mut runtime = Runtime::new(RuntimeConfig::new(2026)).unwrap();
    runtime.run_ticks(2).unwrap();
    let snapshot = runtime.export_snapshot().unwrap();
    let mut state = RuntimeState::import_snapshot(snapshot).unwrap();

    let mut receipt_totals = BTreeMap::new();
    for (trace, receipts) in &state.thermal_receipts {
        let mut totals = ThermalBatchReceiptTotals::default();
        for receipt in receipts {
            accumulate_receipt_totals(receipt, &mut totals).unwrap();
        }
        receipt_totals.insert(*trace, totals);
    }

    let first_trace = *state
        .thermal_conservation_receipts
        .keys()
        .next()
        .expect("fixture must contain a conservation receipt");
    let first_receipt = state
        .thermal_conservation_receipts
        .get_mut(&first_trace)
        .expect("fixture receipt must remain addressable");
    first_receipt.total_reservoir_budget_before = first_receipt
        .total_reservoir_budget_before
        .checked_add(1)
        .expect("test mutation must remain representable");

    let result = validate_thermal_aggregate_conservation(&state, &receipt_totals);
    assert!(matches!(
        result,
        Err(RuntimeError::InvalidSnapshot(
            "thermal conservation receipt has non-zero residual"
        ))
    ));
}
