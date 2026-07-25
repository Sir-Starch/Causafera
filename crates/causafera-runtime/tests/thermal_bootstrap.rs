use causafera_core::Phase;
use causafera_runtime::{
    Runtime, RuntimeConfig, RuntimeError, RuntimeState, THERMAL_RESERVOIR_BOOTSTRAP_EVENT_KIND,
};
use causafera_types::{ChunkCoord, SpatialChartId};

fn runtime_config(seed: u64) -> RuntimeConfig {
    RuntimeConfig::new(seed)
}

#[test]
fn outside_active_region_boundary_rejects_bootstrap_reservoir() {
    // Given: a production thermal reservoir snapshot.
    let runtime = Runtime::new(runtime_config(1_811)).expect("runtime bootstrap must succeed");
    let mut snapshot = runtime.export_snapshot().expect("snapshot must export");

    // When: its target is moved outside the static active region.
    snapshot.thermal.reservoirs[0].target.chunk =
        causafera_types::ChartChunkCoord::new(SpatialChartId::new(1), ChunkCoord::new(99, 0, 0));

    // Then: import rejects the invalid reservoir residency before installation.
    assert!(matches!(
        RuntimeState::import_snapshot(snapshot),
        Err(RuntimeError::InvalidSnapshot(_))
    ));
}

#[test]
fn active_neighbor_missing_rejects_snapshot() {
    // Given: a complete static thermal active region.
    let runtime = Runtime::new(runtime_config(1_812)).expect("runtime bootstrap must succeed");
    let mut snapshot = runtime.export_snapshot().expect("snapshot must export");

    // When: the active neighbor's thermal field is omitted.
    snapshot.thermal.field_set.fields.pop();

    // Then: restoration rejects the active-region gap.
    assert!(matches!(
        RuntimeState::import_snapshot(snapshot),
        Err(RuntimeError::InvalidSnapshot(_))
    ));
}

#[test]
fn production_bootstrap_retains_reservoir_provenance() {
    // Given: a runtime created exclusively through the production bootstrap path.
    let mut runtime = Runtime::new(runtime_config(1_813)).expect("runtime bootstrap must succeed");
    let bootstrap = runtime
        .export_snapshot()
        .expect("bootstrap state must export");

    // When: the first physics batch executes.
    runtime.tick().expect("first physics batch must execute");
    let after_tick = runtime
        .export_snapshot()
        .expect("thermal state must export");

    // Then: reservoirs are lifecycle-provenanced and their finite budget is conserved.
    assert!(!bootstrap.thermal.reservoirs.is_empty());
    assert!(bootstrap.thermal.reservoirs.iter().all(|reservoir| {
        bootstrap.traces.events.iter().any(|event| {
            event.trace_id == reservoir.bootstrap_trace
                && event.phase == Phase::Lifecycle
                && event.kind.raw() == THERMAL_RESERVOIR_BOOTSTRAP_EVENT_KIND
        })
    }));
    assert_eq!(after_tick.thermal.field_set.batch_sequence, 1);
    assert_eq!(
        after_tick
            .thermal
            .conservation_receipts
            .last()
            .expect("first thermal batch must retain a conservation receipt")
            .residual,
        0
    );
}
