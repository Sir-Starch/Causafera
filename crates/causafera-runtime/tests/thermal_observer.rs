use causafera_explanation::{
    ClaimEvidenceState, NumericClaimValue, THERMAL_CARRIER_CONSERVATION_SCHEMA,
};
use causafera_runtime::{Runtime, RuntimeConfig};

#[test]
fn thermal_observer_projects_bounded_receipts_and_conservation() {
    // Given: a production-bootstrapped thermal reservoir.
    let mut runtime = Runtime::new(RuntimeConfig::new(2_301)).expect("runtime must bootstrap");

    // When: one physics tick produces a thermal conservation receipt.
    runtime.tick().expect("thermal tick must execute");
    let summary = runtime
        .snapshot()
        .expect("runtime snapshot must succeed")
        .observer_snapshot();
    let world = runtime
        .observer_world_snapshot()
        .expect("world projection must succeed");
    let explanation = runtime
        .observer_thermal_conservation_explanation()
        .expect("thermal explanation must succeed");

    // Then: observer and Explanation projections expose bounded authoritative evidence.
    assert!(summary.thermal_active_chunk_count > 0);
    assert!(summary.thermal_active_cell_count > 0);
    assert!(world.thermal_deltas.len() <= 64);
    let claim = explanation.frames[0]
        .claims
        .iter()
        .find(|claim| claim.schema == THERMAL_CARRIER_CONSERVATION_SCHEMA)
        .expect("thermal conservation claim must be present");
    assert_eq!(claim.value, NumericClaimValue::scalar(0));
}

#[test]
fn thermal_observer_recovers_evicted_conservation_evidence_from_history() {
    let mut runtime = Runtime::new(RuntimeConfig::new(2_302)).expect("runtime must bootstrap");
    runtime.tick().expect("first thermal tick must execute");
    let initial = runtime.export_snapshot().expect("snapshot must export");
    let trace = initial.thermal.conservation_receipts[0].trace;
    let initial_report = runtime
        .observer_thermal_conservation_explanation()
        .expect("current thermal explanation must succeed");
    let initial_claim = initial_report.frames[0]
        .claims
        .iter()
        .find(|claim| claim.schema == THERMAL_CARRIER_CONSERVATION_SCHEMA)
        .expect("current thermal conservation claim must be present");
    let expected_evidence_traces = initial_claim.evidence_traces.clone();
    assert!(
        expected_evidence_traces.len() > 1,
        "scenario must include transfer evidence beyond the conservation trace"
    );

    runtime
        .run_ticks(70)
        .expect("history must advance beyond the receipt retention window");
    let current = runtime.export_snapshot().expect("snapshot must export");
    assert!(
        current
            .thermal
            .conservation_receipts
            .iter()
            .all(|receipt| receipt.trace != trace)
    );

    let report = runtime
        .observer_thermal_conservation_explanation_for_trace(trace)
        .expect("an evicted receipt remains queryable from history");
    let claim = report.frames[0]
        .claims
        .iter()
        .find(|claim| claim.schema == THERMAL_CARRIER_CONSERVATION_SCHEMA)
        .expect("thermal conservation claim must be present");
    assert_eq!(claim.evidence_state, ClaimEvidenceState::Supported);
    assert_eq!(claim.evidence_traces, expected_evidence_traces);
}
