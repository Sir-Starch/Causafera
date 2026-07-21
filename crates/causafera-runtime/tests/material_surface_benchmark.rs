use causafera_runtime::{MaterialSurfaceLoopBenchmarkConfig, run_material_surface_loop_benchmark};

#[test]
fn bounded_benchmark_reports_production_loop_evidence_and_measurements() {
    // Given: the fixed one-actor production historical bootstrap benchmark configuration.
    let config = MaterialSurfaceLoopBenchmarkConfig {
        warmup_ticks: 4,
        measurement_ticks: 4,
        ..MaterialSurfaceLoopBenchmarkConfig::default()
    };

    // When: the benchmark drives the production runtime and bounded observer query path.
    let report = run_material_surface_loop_benchmark(config)
        .expect("production material-surface benchmark must complete");

    // Then: both runs prove contact and mana consequences and emit the required local metrics.
    for measurement in [&report.observer_off, &report.world_chunks_query] {
        assert_eq!(measurement.promoted_actor_count, 1);
        assert_eq!(measurement.material_surface_site_count, 1);
        assert!(measurement.material_contact_count > 0);
        assert!(measurement.mana_material_transition_count > 0);
        assert!(measurement.tick_elapsed_ns > 0);
        assert!(measurement.provenance_event_growth > 0);
        assert!(measurement.encoded_snapshot_bytes > 0);
    }
    assert_eq!(report.observer_off.observer_response_bytes, 0);
    assert!(report.world_chunks_query.observer_response_bytes > 0);
}

#[test]
fn bounded_benchmark_rejects_zero_measurement_ticks() {
    // Given: an otherwise valid benchmark configuration with no measurement window.
    let config = MaterialSurfaceLoopBenchmarkConfig {
        measurement_ticks: 0,
        ..MaterialSurfaceLoopBenchmarkConfig::default()
    };

    // When: validation runs before any production runtime is constructed.
    let result = run_material_surface_loop_benchmark(config);

    // Then: the invalid workload is rejected rather than reporting an empty envelope.
    assert!(result.is_err());
}
