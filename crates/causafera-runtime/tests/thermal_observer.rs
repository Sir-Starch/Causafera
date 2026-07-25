use causafera_explanation::{NumericClaimValue, THERMAL_CARRIER_CONSERVATION_SCHEMA};
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
