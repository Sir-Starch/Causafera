use causafera_explanation::{ClaimEvidenceState, MATERIAL_SURFACE_THERMAL_EXCHANGE_SCHEMA};
use causafera_runtime::{Runtime, RuntimeConfig, RuntimeError};

#[test]
fn world_snapshot_projects_bounded_material_surface_thermal_deltas() {
    // Given: a production-bootstrapped runtime with its default active material coupling.
    let mut runtime = Runtime::new(RuntimeConfig::new(2_401)).expect("runtime must bootstrap");

    // When: one physics tick commits at least one material/cell thermal exchange.
    runtime.tick().expect("thermal tick must execute");
    let world = runtime
        .observer_world_snapshot()
        .expect("world projection must succeed");

    // Then: the bounded delta list is populated, capped, and the shared schema version reflects
    // the newest addressed-object family it now carries.
    assert!(!world.material_surface_thermal_deltas.is_empty());
    assert!(world.material_surface_thermal_deltas.len() <= 64);
    assert_eq!(world.material_surface_delta_schema_version, 4);
    for delta in &world.material_surface_thermal_deltas {
        assert_ne!(delta.before_retained, delta.after_retained);
    }
}

#[test]
fn thermal_explanation_rejects_an_unknown_surface() {
    // Given: a production-bootstrapped runtime and a surface ID that names no real surface.
    let mut runtime = Runtime::new(RuntimeConfig::new(2_402)).expect("runtime must bootstrap");
    runtime.tick().expect("thermal tick must execute");
    let exported = runtime
        .export_snapshot()
        .expect("bootstrap state must export");
    let real_surface = exported
        .material_surfaces
        .records
        .first()
        .expect("production bootstrap must create at least one material surface")
        .id;
    let bogus_surface = causafera_runtime::MaterialSurfaceId::new(
        causafera_types::ChartChunkCoord::new(
            causafera_types::SpatialChartId::new(real_surface.chunk.chart.raw() + 1),
            real_surface.chunk.chunk,
        ),
        real_surface.cell_index,
    );

    // When: the queried surface names no entry in `material_surfaces`.
    let result = runtime.observer_material_surface_thermal_explanation_for_surface(bogus_surface);

    // Then: the query is rejected as invalid rather than answered with an `Unknown` claim.
    assert!(matches!(result, Err(RuntimeError::InvalidSnapshot(_))));
}

#[test]
fn thermal_explanation_reports_unknown_before_any_exchange_then_supported_after() {
    // Given: a production-bootstrapped runtime, queried before its first thermal batch commits.
    let mut runtime = Runtime::new(RuntimeConfig::new(2_403)).expect("runtime must bootstrap");
    let bootstrap_exported = runtime
        .export_snapshot()
        .expect("bootstrap state must export");
    let surface = bootstrap_exported
        .material_surfaces
        .records
        .first()
        .expect("production bootstrap must create at least one material surface")
        .id;

    // When: the surface is queried before it has any recorded thermal exchange.
    let bootstrap_report = runtime
        .observer_material_surface_thermal_explanation_for_surface(surface)
        .expect("a real surface must always be queryable");

    // Then: the claim is `Unknown`, carrying the surface's current (bootstrap) retained energy.
    let bootstrap_claim = &bootstrap_report.frames[0].claims[0];
    assert_eq!(
        bootstrap_claim.schema,
        MATERIAL_SURFACE_THERMAL_EXCHANGE_SCHEMA
    );
    assert_eq!(bootstrap_claim.evidence_state, ClaimEvidenceState::Unknown);
    assert!(bootstrap_claim.evidence_traces.is_empty());

    // When: a real thermal batch commits at least one exchange, for whichever surface diffusion
    // reaches first (not necessarily the arbitrary surface queried above).
    runtime.tick().expect("thermal tick must execute");
    let exported = runtime.export_snapshot().expect("ticked state must export");
    let exchanged_surface = exported
        .material_surfaces
        .thermal_transitions
        .last()
        .expect("one thermal tick must record at least one material exchange")
        .id;
    let report = runtime
        .observer_material_surface_thermal_explanation_for_surface(exchanged_surface)
        .expect("a real surface must always be queryable");

    // Then: the claim becomes `Supported`, citing the transition and its cell trace.
    let claim = &report.frames[0].claims[0];
    assert_eq!(claim.schema, MATERIAL_SURFACE_THERMAL_EXCHANGE_SCHEMA);
    assert_eq!(claim.evidence_state, ClaimEvidenceState::Supported);
    assert_eq!(claim.evidence_traces.len(), 2);
}
