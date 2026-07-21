use causafera_observer_api::MAX_QUERY_PAYLOAD_BYTES;
use causafera_persistence::MAX_TOTAL_SIZE;
use causafera_runtime::{
    ExperimentRecipeManaSourceBenchmarkConfig, ExperimentRecipeManaSourceBenchmarkMode,
    MaterialSurfaceLoopBenchmarkConfig, run_experiment_recipe_mana_source_benchmark,
    run_material_surface_loop_benchmark,
};

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

#[test]
fn experiment_recipe_mana_source_benchmark_reports_disabled_vs_enabled_envelope() {
    // Given: the bounded v1 benchmark window for the hard-cap source collection.
    let config = ExperimentRecipeManaSourceBenchmarkConfig {
        measurement_ticks: 4,
        ..ExperimentRecipeManaSourceBenchmarkConfig::default()
    };

    // When: the benchmark measures all-disabled and one-enabled source recipes.
    let report = run_experiment_recipe_mana_source_benchmark(config)
        .expect("experiment-recipe mana-source benchmark must complete");
    let disabled = &report.disabled;
    let enabled = &report.enabled;

    // Then: both measurements remain present and bounded, while only the enabled mode commits one
    // receipt/source event. One source record commits one event with two effects, so the
    // provenance growth delta is exactly one committed event, not two effect records.
    assert_eq!(
        disabled.mode,
        ExperimentRecipeManaSourceBenchmarkMode::Disabled
    );
    assert_eq!(
        enabled.mode,
        ExperimentRecipeManaSourceBenchmarkMode::Enabled
    );
    for measurement in [disabled, enabled] {
        assert!(measurement.tick_elapsed_ns > 0);
        assert!(measurement.encoded_snapshot_bytes > 0);
        assert!(measurement.encoded_snapshot_bytes <= MAX_TOTAL_SIZE);
        assert!(measurement.observer_response_bytes > 0);
        assert!(measurement.observer_response_bytes <= MAX_QUERY_PAYLOAD_BYTES as u64);
    }
    assert_eq!(disabled.source_receipt_count, 0);
    assert_eq!(disabled.source_event_count, 0);
    assert_eq!(enabled.source_receipt_count, 1);
    assert_eq!(enabled.source_event_count, 1);
    let source_event_count_delta = enabled.source_event_count - disabled.source_event_count;
    assert_eq!(source_event_count_delta, 1);
    assert_eq!(
        enabled.provenance_event_growth - disabled.provenance_event_growth,
        source_event_count_delta
    );
}
