use std::collections::{BTreeMap, BTreeSet};

use causafera_core::{Phase, provenance::CausalEventSnapshot};
use causafera_domains::ThermalCellKey;
use causafera_runtime::{
    MATERIAL_SURFACE_THERMAL_EXCHANGE_EVENT_KIND, Runtime, RuntimeConfig, RuntimeSnapshotData,
    THERMAL_CELL_CHANGE_EVENT_KIND, THERMAL_CONSERVATION_EVENT_KIND,
    THERMAL_FIELD_BOOTSTRAP_EVENT_KIND, THERMAL_RESERVOIR_BOOTSTRAP_EVENT_KIND,
    THERMAL_RESERVOIR_TRANSFER_EVENT_KIND,
};
use causafera_types::TraceId;

const FIXTURE_SEED: u64 = 2026;
const FIXTURE_TICKS: u64 = 7;

fn runtime_config(seed: u64) -> RuntimeConfig {
    RuntimeConfig::new(seed)
}

fn fixture() -> RuntimeSnapshotData {
    let mut runtime =
        Runtime::new(runtime_config(FIXTURE_SEED)).expect("runtime bootstrap must succeed");
    runtime
        .run_ticks(FIXTURE_TICKS)
        .expect("fixture ticks must execute");
    runtime.export_snapshot().expect("snapshot must export")
}

fn event_by_trace(snapshot: &RuntimeSnapshotData, trace: TraceId) -> Option<&CausalEventSnapshot> {
    snapshot
        .traces
        .events
        .iter()
        .find(|event| event.trace_id == trace)
}

#[test]
fn fixture_satisfies_aggregate_validation_preconditions() {
    // Given: an engine-produced snapshot after several ticks.
    let snapshot = fixture();

    // Then: the snapshot satisfies all four RISK-2 preconditions.
    let batch_sequence = snapshot.thermal.field_set.batch_sequence;
    assert!(
        batch_sequence >= 3,
        "fixture must have at least 3 thermal batches, got {batch_sequence}"
    );

    let live_reservoirs: Vec<_> = snapshot
        .thermal
        .reservoirs
        .iter()
        .filter(|reservoir| reservoir.budget > 0)
        .collect();
    assert!(
        !live_reservoirs.is_empty(),
        "fixture must have at least one reservoir with residual budget"
    );

    let retained_surfaces: Vec<_> = snapshot
        .material_surfaces
        .records
        .iter()
        .filter(|record| record.surface.thermal.retained_energy.get() > 0)
        .collect();
    assert!(
        !retained_surfaces.is_empty(),
        "fixture must have at least one surface with non-zero retained energy"
    );

    let receipt_cells: BTreeSet<ThermalCellKey> = snapshot
        .thermal
        .transfer_receipts
        .iter()
        .map(|receipt| receipt.cell)
        .collect();
    let mut uncovered = None;
    for field in &snapshot.thermal.field_set.fields {
        for (index, _) in field.energy.iter().enumerate() {
            let cell_index = u16::try_from(index).expect("cell index fits u16");
            let key = ThermalCellKey::new(field.chunk, cell_index);
            if !receipt_cells.contains(&key) {
                uncovered = Some(key);
                break;
            }
        }
        if uncovered.is_some() {
            break;
        }
    }
    assert!(
        uncovered.is_some(),
        "fixture must contain at least one cell appearing in no transfer receipt"
    );
}

#[test]
fn face_signed_flux_sums_to_zero_per_batch() {
    // Given: an engine-produced snapshot.
    let snapshot = fixture();

    // Then: for every batch, the sum of all face signed fluxes is zero.
    let mut batches: BTreeMap<TraceId, i128> = BTreeMap::new();
    for receipt in &snapshot.thermal.transfer_receipts {
        let sum = receipt
            .faces
            .iter()
            .map(|face| i128::from(face.signed_flux))
            .sum::<i128>();
        *batches.entry(receipt.conservation_trace).or_insert(0) += sum;
    }
    for (trace, total) in batches {
        assert_eq!(
            total, 0,
            "batch {trace:?} face signed flux sum must be zero (got {total})"
        );
    }
}

#[test]
fn non_physics_phases_preserve_thermal_buckets() {
    // Given: an engine-produced snapshot after several ticks.
    let snapshot = fixture();

    // Then: every post-bootstrap thermal mutation anchor resolves to a Physics-phase event.
    for receipt in &snapshot.thermal.conservation_receipts {
        let event =
            event_by_trace(&snapshot, receipt.trace).expect("conservation trace must exist");
        assert_eq!(event.phase, Phase::Physics);
        assert_eq!(event.kind.raw(), THERMAL_CONSERVATION_EVENT_KIND);
    }

    for receipt in &snapshot.thermal.transfer_receipts {
        if let Some(trace) = receipt.cell_change_trace_id {
            let event = event_by_trace(&snapshot, trace).expect("cell change trace must exist");
            assert_eq!(event.phase, Phase::Physics);
            assert_eq!(event.kind.raw(), THERMAL_CELL_CHANGE_EVENT_KIND);
        }
        for record in &receipt.reservoirs {
            if let Some(trace) = record.transfer_trace_id {
                let event =
                    event_by_trace(&snapshot, trace).expect("reservoir transfer trace must exist");
                assert_eq!(event.phase, Phase::Physics);
                assert_eq!(event.kind.raw(), THERMAL_RESERVOIR_TRANSFER_EVENT_KIND);
            }
        }
    }

    for transition in &snapshot.material_surfaces.thermal_transitions {
        let event = event_by_trace(&snapshot, transition.transition_trace)
            .expect("material thermal transition trace must exist");
        assert_eq!(event.phase, Phase::Physics);
        assert_eq!(
            event.kind.raw(),
            MATERIAL_SURFACE_THERMAL_EXCHANGE_EVENT_KIND
        );
    }

    for reservoir in &snapshot.thermal.reservoirs {
        let bootstrap = event_by_trace(&snapshot, reservoir.bootstrap_trace)
            .expect("reservoir bootstrap trace must exist");
        assert_eq!(bootstrap.phase, Phase::Lifecycle);
        assert_eq!(bootstrap.kind.raw(), THERMAL_RESERVOIR_BOOTSTRAP_EVENT_KIND);

        let last_change = event_by_trace(&snapshot, reservoir.last_change)
            .expect("reservoir last-change trace must exist");
        assert!(
            last_change.phase == Phase::Physics || last_change.phase == Phase::Lifecycle,
            "reservoir last-change trace must be Physics or Lifecycle, got {:?}",
            last_change.phase
        );
    }

    let bootstrap_traces: BTreeSet<TraceId> = snapshot
        .traces
        .events
        .iter()
        .filter(|event| event.kind.raw() == THERMAL_FIELD_BOOTSTRAP_EVENT_KIND)
        .map(|event| event.trace_id)
        .collect();
    assert!(
        !bootstrap_traces.is_empty(),
        "fixture must contain at least one thermal field bootstrap event"
    );

    for field in &snapshot.thermal.field_set.fields {
        for (&trace, _energy) in field.last_change.iter().zip(field.energy.iter()) {
            let event =
                event_by_trace(&snapshot, trace).expect("field last-change trace must exist");
            if bootstrap_traces.contains(&trace) {
                assert_eq!(event.phase, Phase::Lifecycle);
                assert_eq!(event.kind.raw(), THERMAL_FIELD_BOOTSTRAP_EVENT_KIND);
            } else {
                assert_eq!(
                    event.phase,
                    Phase::Physics,
                    "post-bootstrap field last-change trace must be Physics"
                );
                assert_eq!(event.kind.raw(), THERMAL_CELL_CHANGE_EVENT_KIND);
            }
        }
    }
}

#[test]
fn engine_snapshots_satisfy_all_six_identities() {
    // Given: an engine-produced snapshot.
    let snapshot = fixture();

    // Build a map from conservation trace -> receipt list.
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
        .map(|receipt| (receipt.trace, receipt))
        .collect();

    let traces: Vec<TraceId> = conservation_receipts.keys().copied().collect();

    for (index, trace) in traces.iter().enumerate() {
        let receipt = conservation_receipts[trace];
        let batch_receipts = &receipts_by_trace[trace];

        // I1: reservoir budget delta equals total accepted injection.
        let accepted_injection: i128 = batch_receipts
            .iter()
            .flat_map(|receipt| &receipt.reservoirs)
            .map(|record| i128::from(record.accepted_injection))
            .sum();
        let budget_delta =
            receipt.total_reservoir_budget_before - receipt.total_reservoir_budget_after;
        assert_eq!(
            budget_delta, accepted_injection,
            "I1 failed for batch {index} ({trace:?})"
        );

        // I2: cell delta equals (post - pre) sum + accepted injection.
        let cell_transition: i128 = batch_receipts
            .iter()
            .map(|receipt| i128::from(receipt.post_state) - i128::from(receipt.pre_state))
            .sum();
        let expected_cell_delta = cell_transition + accepted_injection;
        let actual_cell_delta = receipt.total_cell_energy_after - receipt.total_cell_energy_before;
        assert_eq!(
            actual_cell_delta, expected_cell_delta,
            "I2 failed for batch {index} ({trace:?})"
        );

        // I3: material retained delta equals total signed material flux.
        let material_flux: i128 = batch_receipts
            .iter()
            .filter_map(|receipt| receipt.material.as_ref())
            .map(|material| i128::from(material.signed_flux))
            .sum();
        let actual_material_delta =
            receipt.total_material_retained_after - receipt.total_material_retained_before;
        assert_eq!(
            actual_material_delta, material_flux,
            "I3 failed for batch {index} ({trace:?})"
        );

        // I3a: per-receipt material retained delta equals signed flux.
        for receipt in batch_receipts.iter() {
            if let Some(material) = &receipt.material {
                let retained_delta =
                    i128::from(material.retained_after) - i128::from(material.retained_before);
                assert_eq!(
                    retained_delta,
                    i128::from(material.signed_flux),
                    "I3a failed for cell {:?} in batch {index} ({trace:?})",
                    receipt.cell
                );
            }
        }

        // I4: total residual is zero.
        let residual = (receipt.total_cell_energy_after
            + receipt.total_reservoir_budget_after
            + receipt.total_material_retained_after)
            - (receipt.total_cell_energy_before
                + receipt.total_reservoir_budget_before
                + receipt.total_material_retained_before);
        assert_eq!(residual, 0, "I4 failed for batch {index} ({trace:?})");
    }

    // I5: chain identity between consecutive batches.
    for window in traces.windows(2) {
        let prev = conservation_receipts[&window[0]];
        let next = conservation_receipts[&window[1]];
        assert_eq!(
            prev.total_cell_energy_after, next.total_cell_energy_before,
            "I5 chain failed for cell energy between {:?} and {:?}",
            prev.trace, next.trace
        );
        assert_eq!(
            prev.total_reservoir_budget_after, next.total_reservoir_budget_before,
            "I5 chain failed for reservoir budget between {:?} and {:?}",
            prev.trace, next.trace
        );
        assert_eq!(
            prev.total_material_retained_after, next.total_material_retained_before,
            "I5 chain failed for material retained between {:?} and {:?}",
            prev.trace, next.trace
        );
    }

    // I6: terminal anchor against materialized final state.
    let last_trace = traces.last().expect("at least one conservation receipt");
    let last_receipt = conservation_receipts[last_trace];

    let total_cell_energy: i128 = snapshot
        .thermal
        .field_set
        .fields
        .iter()
        .flat_map(|field| &field.energy)
        .map(|energy| i128::from(*energy))
        .sum();
    assert_eq!(
        total_cell_energy, last_receipt.total_cell_energy_after,
        "I6 terminal anchor failed for cell energy"
    );

    let total_reservoir_budget: i128 = snapshot
        .thermal
        .reservoirs
        .iter()
        .map(|reservoir| i128::from(reservoir.budget))
        .sum();
    assert_eq!(
        total_reservoir_budget, last_receipt.total_reservoir_budget_after,
        "I6 terminal anchor failed for reservoir budget"
    );

    let total_material_retained: i128 = snapshot
        .material_surfaces
        .records
        .iter()
        .map(|record| i128::from(record.surface.thermal.retained_energy.get()))
        .sum();
    assert_eq!(
        total_material_retained, last_receipt.total_material_retained_after,
        "I6 terminal anchor failed for material retained"
    );
}
