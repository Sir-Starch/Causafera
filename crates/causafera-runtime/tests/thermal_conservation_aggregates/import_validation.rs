use causafera_runtime::{RuntimeError, RuntimeState, measure_import_wall_time};

use super::support::{find_unbound_cell, fixture};

#[test]
fn runtime_import_rejects_forged_cell_and_material_aggregate_totals() {
    // Given: a valid engine-produced snapshot.
    let forged = fixture();
    let trace_count = forged.thermal.conservation_receipts.len();
    assert!(
        trace_count >= 2,
        "control requires at least two conservation receipts"
    );
    let latest_index = trace_count - 1;
    let non_latest_index = 0;

    // When: each of the four unanchored aggregate literals is perturbed independently
    // in both the latest and a non-latest batch.
    let assert_rejects = |name: &str, index: usize, mutate: fn(&mut _, i128)| {
        for delta in [-1_i128, 1_i128] {
            let mut variant = forged.clone();
            mutate(&mut variant.thermal.conservation_receipts[index], delta);
            let imported = RuntimeState::import_snapshot(variant);
            assert!(
                matches!(imported, Err(RuntimeError::InvalidSnapshot(_))),
                "forging {name} by {delta:+} in batch {index} must be rejected"
            );
        }
    };

    assert_rejects("cell_energy_before", non_latest_index, |receipt, delta| {
        receipt.total_cell_energy_before = receipt
            .total_cell_energy_before
            .checked_add(delta)
            .expect("test mutation must remain representable")
    });
    assert_rejects("cell_energy_before", latest_index, |receipt, delta| {
        receipt.total_cell_energy_before = receipt
            .total_cell_energy_before
            .checked_add(delta)
            .expect("test mutation must remain representable")
    });
    assert_rejects("cell_energy_after", non_latest_index, |receipt, delta| {
        receipt.total_cell_energy_after = receipt
            .total_cell_energy_after
            .checked_add(delta)
            .expect("test mutation must remain representable")
    });
    assert_rejects("cell_energy_after", latest_index, |receipt, delta| {
        receipt.total_cell_energy_after = receipt
            .total_cell_energy_after
            .checked_add(delta)
            .expect("test mutation must remain representable")
    });
    assert_rejects(
        "material_retained_before",
        non_latest_index,
        |receipt, delta| {
            receipt.total_material_retained_before = receipt
                .total_material_retained_before
                .checked_add(delta)
                .expect("test mutation must remain representable")
        },
    );
    assert_rejects(
        "material_retained_before",
        latest_index,
        |receipt, delta| {
            receipt.total_material_retained_before = receipt
                .total_material_retained_before
                .checked_add(delta)
                .expect("test mutation must remain representable")
        },
    );
    assert_rejects(
        "material_retained_after",
        non_latest_index,
        |receipt, delta| {
            receipt.total_material_retained_after = receipt
                .total_material_retained_after
                .checked_add(delta)
                .expect("test mutation must remain representable")
        },
    );
    assert_rejects("material_retained_after", latest_index, |receipt, delta| {
        receipt.total_material_retained_after = receipt
            .total_material_retained_after
            .checked_add(delta)
            .expect("test mutation must remain representable")
    });
}

#[test]
fn runtime_import_rejects_untouched_cell_energy_tampering() {
    // Given: a valid engine-produced snapshot.
    let mut forged = fixture();

    // Find a cell that is neither in a transfer receipt nor on the active-region boundary.
    let (field_index, cell_index) =
        find_unbound_cell(&forged).expect("fixture must have an unbound cell");

    // When: the unbound cell's energy is incremented without touching any receipt.
    forged.thermal.field_set.fields[field_index].energy[cell_index] =
        forged.thermal.field_set.fields[field_index].energy[cell_index]
            .checked_add(1)
            .expect("test shift must remain representable");

    // Then: import must reject the tampered snapshot.
    let imported = RuntimeState::import_snapshot(forged);
    assert!(
        matches!(imported, Err(RuntimeError::InvalidSnapshot(_))),
        "tampering an unbound cell energy must be rejected"
    );
}

#[test]
fn runtime_import_rejects_coordinated_untouched_cell_and_terminal_total_forgery() {
    // Given: a valid engine-produced snapshot.
    let mut forged = fixture();
    let latest_index = forged.thermal.conservation_receipts.len() - 1;

    // Find an unbound cell and mutate its energy.
    let (field_index, cell_index) =
        find_unbound_cell(&forged).expect("fixture must have an unbound cell");

    // Adjust the terminal total_cell_energy_after by the same amount.
    forged.thermal.field_set.fields[field_index].energy[cell_index] =
        forged.thermal.field_set.fields[field_index].energy[cell_index]
            .checked_add(1)
            .expect("test shift must remain representable");
    forged.thermal.conservation_receipts[latest_index].total_cell_energy_after += 1;

    // Then: import must still reject (via I2 or I5, not via I6).
    let imported = RuntimeState::import_snapshot(forged);
    assert!(
        matches!(
            imported,
            Err(RuntimeError::InvalidSnapshot(
                "thermal cell energy delta does not match receipt totals"
            ))
        ),
        "coordinated untouched-cell + terminal total forgery must be rejected by I2"
    );
}

#[test]
fn runtime_import_rejects_historical_batch_material_total_forgery() {
    // Given: a valid engine-produced snapshot with at least two batches.
    let mut forged = fixture();
    assert!(
        forged.thermal.conservation_receipts.len() >= 2,
        "control requires at least two conservation receipts"
    );

    // When: batch 0's total_material_retained_after is perturbed.
    forged.thermal.conservation_receipts[0].total_material_retained_after += 1;

    // Then: import must reject via the I5 chain.
    let imported = RuntimeState::import_snapshot(forged);
    assert!(
        matches!(
            imported,
            Err(RuntimeError::InvalidSnapshot(
                "thermal material retained chain broken between conservation receipts"
            ))
        ),
        "historical batch material total forgery must be rejected by I5"
    );
}

#[test]
fn import_benchmark_rejects_zero_iterations() {
    let result = measure_import_wall_time(&fixture(), 0);
    assert!(matches!(
        result,
        Err(RuntimeError::InvalidSnapshot(
            "import benchmark iterations must be non-zero"
        ))
    ));
}

#[test]
fn reservoir_aggregate_totals_are_already_rejected() {
    // Given: a valid engine-produced snapshot.
    let mut forged = fixture();
    let latest_index = forged.thermal.conservation_receipts.len() - 1;

    // When: the already-enforced reservoir aggregate literals are forged.
    forged.thermal.conservation_receipts[latest_index].total_reservoir_budget_before += 1;

    // Then: import rejects today (confirming existing coverage, not a new gap).
    let imported = RuntimeState::import_snapshot(forged);
    assert!(
        matches!(imported, Err(RuntimeError::InvalidSnapshot(_))),
        "forging reservoir budget totals must already be rejected"
    );
}

#[test]
fn runtime_import_rejects_i3_material_aggregate_mismatch() {
    let mut forged = fixture();
    assert!(
        !forged.thermal.conservation_receipts.is_empty(),
        "fixture must have at least one receipt"
    );

    forged.thermal.conservation_receipts[0].total_material_retained_before =
        forged.thermal.conservation_receipts[0]
            .total_material_retained_before
            .checked_add(1)
            .expect("test mutation representable");

    let imported = RuntimeState::import_snapshot(forged);
    assert!(
        matches!(
            imported,
            Err(RuntimeError::InvalidSnapshot(
                "thermal material retained delta does not match receipt totals"
            ))
        ),
        "I3 material aggregate mismatch must be rejected precisely"
    );
}

#[test]
fn runtime_import_rejects_i3a_material_receipt_mismatch() {
    let mut forged = fixture();

    let first_material_idx = forged
        .thermal
        .transfer_receipts
        .iter()
        .position(|r| r.material.is_some())
        .expect("fixture must have a material transfer");

    let mat = forged.thermal.transfer_receipts[first_material_idx]
        .material
        .as_mut()
        .unwrap();
    mat.signed_flux = mat
        .signed_flux
        .checked_add(1)
        .expect("test mutation representable");

    let imported = RuntimeState::import_snapshot(forged);
    assert!(
        matches!(
            imported,
            Err(RuntimeError::InvalidSnapshot(
                "thermal receipt material retained delta does not match signed flux"
            ))
        ),
        "I3a material receipt mismatch must be rejected precisely"
    );
}

#[test]
fn runtime_import_rejects_persisted_non_zero_residual() {
    let mut forged = fixture();

    forged.thermal.conservation_receipts[0].residual = 1;

    let imported = RuntimeState::import_snapshot(forged);
    assert!(
        matches!(
            imported,
            Err(RuntimeError::InvalidSnapshot(
                "thermal conservation receipt has non-zero residual"
            ))
        ),
        "persisted non-zero residual must be rejected precisely"
    );
}
