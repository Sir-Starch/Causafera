use super::support::{event_by_trace, find_unbound_cell, fixture};
use causafera_core::Phase;
use causafera_runtime::{
    MATERIAL_SURFACE_THERMAL_EXCHANGE_EVENT_KIND, THERMAL_CELL_CHANGE_EVENT_KIND,
    THERMAL_CONSERVATION_EVENT_KIND, THERMAL_FIELD_BOOTSTRAP_EVENT_KIND,
    THERMAL_RESERVOIR_BOOTSTRAP_EVENT_KIND, THERMAL_RESERVOIR_TRANSFER_EVENT_KIND,
};
use causafera_types::TraceId;
use std::collections::{BTreeMap, BTreeSet};

#[test]
fn fixture_satisfies_aggregate_validation_preconditions() {
    let snapshot = fixture();
    assert!(
        snapshot.thermal.field_set.batch_sequence >= 3,
        "fixture must have at least 3 thermal batches"
    );
    assert!(
        snapshot.thermal.reservoirs.iter().any(|r| r.budget > 0),
        "must have reservoir with residual budget"
    );
    assert!(
        snapshot
            .material_surfaces
            .records
            .iter()
            .any(|r| r.surface.thermal.retained_energy.get() > 0),
        "must have surface with retained energy"
    );
    assert!(
        find_unbound_cell(&snapshot).is_some(),
        "must contain unbound cell"
    );
}

#[test]
fn face_signed_flux_sums_to_zero_per_batch() {
    let snapshot = fixture();
    let mut batches: BTreeMap<TraceId, i128> = BTreeMap::new();
    for receipt in &snapshot.thermal.transfer_receipts {
        *batches.entry(receipt.conservation_trace).or_insert(0) += receipt
            .faces
            .iter()
            .map(|f| i128::from(f.signed_flux))
            .sum::<i128>();
    }
    for (trace, total) in batches {
        assert_eq!(
            total, 0,
            "batch {trace:?} face signed flux sum must be zero"
        );
    }
}

#[test]
fn non_physics_phases_preserve_thermal_buckets() {
    let snapshot = fixture();
    for receipt in &snapshot.thermal.conservation_receipts {
        let event = event_by_trace(&snapshot, receipt.trace).unwrap();
        assert_eq!(
            (event.phase, event.kind.raw()),
            (Phase::Physics, THERMAL_CONSERVATION_EVENT_KIND)
        );
    }
    for receipt in &snapshot.thermal.transfer_receipts {
        if let Some(trace) = receipt.cell_change_trace_id {
            let event = event_by_trace(&snapshot, trace).unwrap();
            assert_eq!(
                (event.phase, event.kind.raw()),
                (Phase::Physics, THERMAL_CELL_CHANGE_EVENT_KIND)
            );
        }
        for record in &receipt.reservoirs {
            if let Some(trace) = record.transfer_trace_id {
                let event = event_by_trace(&snapshot, trace).unwrap();
                assert_eq!(
                    (event.phase, event.kind.raw()),
                    (Phase::Physics, THERMAL_RESERVOIR_TRANSFER_EVENT_KIND)
                );
            }
        }
    }
    for transition in &snapshot.material_surfaces.thermal_transitions {
        let event = event_by_trace(&snapshot, transition.transition_trace).unwrap();
        assert_eq!(
            (event.phase, event.kind.raw()),
            (Phase::Physics, MATERIAL_SURFACE_THERMAL_EXCHANGE_EVENT_KIND)
        );
    }
    for reservoir in &snapshot.thermal.reservoirs {
        let bootstrap = event_by_trace(&snapshot, reservoir.bootstrap_trace).unwrap();
        assert_eq!(
            (bootstrap.phase, bootstrap.kind.raw()),
            (Phase::Lifecycle, THERMAL_RESERVOIR_BOOTSTRAP_EVENT_KIND)
        );
        let last_change = event_by_trace(&snapshot, reservoir.last_change).unwrap();
        assert!(last_change.phase == Phase::Physics || last_change.phase == Phase::Lifecycle);
    }
    let bootstrap_traces: BTreeSet<TraceId> = snapshot
        .traces
        .events
        .iter()
        .filter(|e| e.kind.raw() == THERMAL_FIELD_BOOTSTRAP_EVENT_KIND)
        .map(|e| e.trace_id)
        .collect();
    assert!(!bootstrap_traces.is_empty());
    for field in &snapshot.thermal.field_set.fields {
        for (&trace, _) in field.last_change.iter().zip(field.energy.iter()) {
            let event = event_by_trace(&snapshot, trace).unwrap();
            if bootstrap_traces.contains(&trace) {
                assert_eq!(
                    (event.phase, event.kind.raw()),
                    (Phase::Lifecycle, THERMAL_FIELD_BOOTSTRAP_EVENT_KIND)
                );
            } else {
                assert_eq!(
                    (event.phase, event.kind.raw()),
                    (Phase::Physics, THERMAL_CELL_CHANGE_EVENT_KIND)
                );
            }
        }
    }
}

#[test]
fn engine_snapshots_satisfy_all_six_identities() {
    let snapshot = fixture();
    let mut receipts_by_trace: BTreeMap<
        TraceId,
        Vec<&causafera_runtime::ThermalCellTransferReceiptSnapshot>,
    > = BTreeMap::new();
    for receipt in &snapshot.thermal.transfer_receipts {
        receipts_by_trace
            .entry(receipt.conservation_trace)
            .or_default()
            .push(receipt);
    }
    let conservation_receipts: BTreeMap<TraceId, _> = snapshot
        .thermal
        .conservation_receipts
        .iter()
        .map(|r| (r.trace, r))
        .collect();
    let traces: Vec<TraceId> = conservation_receipts.keys().copied().collect();
    for trace in traces.iter() {
        let receipt = conservation_receipts[trace];
        let batch_receipts = &receipts_by_trace[trace];

        let accepted_injection: i128 = batch_receipts
            .iter()
            .flat_map(|r| &r.reservoirs)
            .map(|r| i128::from(r.accepted_injection))
            .sum();
        let budget_delta =
            receipt.total_reservoir_budget_before - receipt.total_reservoir_budget_after;
        assert_eq!(budget_delta, accepted_injection, "I1 failed");

        let cell_transition: i128 = batch_receipts
            .iter()
            .map(|r| i128::from(r.post_state) - i128::from(r.pre_state))
            .sum();
        let actual_cell_delta = receipt.total_cell_energy_after - receipt.total_cell_energy_before;
        assert_eq!(
            actual_cell_delta,
            cell_transition + accepted_injection,
            "I2 failed"
        );

        let material_flux: i128 = batch_receipts
            .iter()
            .filter_map(|r| r.material.as_ref())
            .map(|m| i128::from(m.signed_flux))
            .sum();
        let actual_material_delta =
            receipt.total_material_retained_after - receipt.total_material_retained_before;
        assert_eq!(actual_material_delta, material_flux, "I3 failed");

        for br in batch_receipts.iter() {
            if let Some(m) = &br.material {
                assert_eq!(
                    i128::from(m.retained_after) - i128::from(m.retained_before),
                    i128::from(m.signed_flux),
                    "I3a failed"
                );
            }
        }

        let residual = (receipt.total_cell_energy_after
            + receipt.total_reservoir_budget_after
            + receipt.total_material_retained_after)
            - (receipt.total_cell_energy_before
                + receipt.total_reservoir_budget_before
                + receipt.total_material_retained_before);
        assert_eq!(residual, 0, "I4 failed");
    }
    for window in traces.windows(2) {
        let prev = conservation_receipts[&window[0]];
        let next = conservation_receipts[&window[1]];
        assert_eq!(
            prev.total_cell_energy_after, next.total_cell_energy_before,
            "I5 failed"
        );
        assert_eq!(
            prev.total_reservoir_budget_after, next.total_reservoir_budget_before,
            "I5 failed"
        );
        assert_eq!(
            prev.total_material_retained_after, next.total_material_retained_before,
            "I5 failed"
        );
    }
    let last_receipt = conservation_receipts[traces.last().unwrap()];
    assert_eq!(
        snapshot
            .thermal
            .field_set
            .fields
            .iter()
            .flat_map(|f| &f.energy)
            .map(|e| i128::from(*e))
            .sum::<i128>(),
        last_receipt.total_cell_energy_after,
        "I6 failed"
    );
    assert_eq!(
        snapshot
            .thermal
            .reservoirs
            .iter()
            .map(|r| i128::from(r.budget))
            .sum::<i128>(),
        last_receipt.total_reservoir_budget_after,
        "I6 failed"
    );
    assert_eq!(
        snapshot
            .material_surfaces
            .records
            .iter()
            .map(|r| i128::from(r.surface.thermal.retained_energy.get()))
            .sum::<i128>(),
        last_receipt.total_material_retained_after,
        "I6 failed"
    );
}
