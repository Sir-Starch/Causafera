use causafera_observer_wire::WireError;
use thiserror::Error;

use crate::RuntimeError;
use crate::benchmark::{
    ExperimentRecipeManaSourceBenchmarkConfig, ExperimentRecipeManaSourceBenchmarkMeasurement,
    ExperimentRecipeManaSourceBenchmarkMode, MaterialSurfaceLoopBenchmarkConfig,
    MaterialSurfaceLoopBenchmarkMeasurement, MaterialSurfaceLoopBenchmarkMode,
    MaterialSurfaceLoopBenchmarkReport,
};

#[derive(Debug, Error)]
pub enum MaterialSurfaceLoopBenchmarkError {
    #[error("benchmark warmup ticks must be positive")]
    ZeroWarmupTicks,
    #[error("benchmark measurement ticks must be positive")]
    ZeroMeasurementTicks,
    #[error("runtime benchmark failed: {0}")]
    Runtime(#[from] RuntimeError),
    #[error("observer benchmark query failed: {0}")]
    Observer(#[from] WireError),
    #[error("snapshot benchmark encoding failed: {0}")]
    Persistence(#[from] causafera_persistence::PersistenceError),
    #[error("benchmark value does not fit its bounded metric")]
    MetricOverflow,
    #[error("benchmark did not observe material contact")]
    MissingMaterialContact,
    #[error("benchmark did not retain exactly one promoted actor and one material surface site")]
    InvalidProductionWorkload,
    #[error("benchmark did not observe a mana-to-material transition")]
    MissingManaMaterialTransition,
    #[error("benchmark did not produce a bounded observer payload")]
    MissingObserverPayload,
    #[error("benchmark did not produce a required measurement")]
    MissingMeasurement,
    #[error("experiment-recipe mana-source benchmark workload does not match its mode")]
    InvalidExperimentRecipeSourceWorkload,
    #[error(
        "observer_off and world_chunks_query canonical state diverged; the harness compared two non-equivalent runs"
    )]
    CanonicalStateDivergedAcrossObserverModes,
}

pub(crate) fn validate_benchmark_config(
    config: MaterialSurfaceLoopBenchmarkConfig,
) -> Result<(), MaterialSurfaceLoopBenchmarkError> {
    if config.warmup_ticks == 0 {
        return Err(MaterialSurfaceLoopBenchmarkError::ZeroWarmupTicks);
    }
    if config.measurement_ticks == 0 {
        return Err(MaterialSurfaceLoopBenchmarkError::ZeroMeasurementTicks);
    }
    Ok(())
}

pub(crate) fn validate_benchmark_measurement(
    measurement: &MaterialSurfaceLoopBenchmarkMeasurement,
) -> Result<(), MaterialSurfaceLoopBenchmarkError> {
    if measurement.promoted_actor_count != 1 || measurement.material_surface_site_count != 1 {
        return Err(MaterialSurfaceLoopBenchmarkError::InvalidProductionWorkload);
    }
    if measurement.material_contact_count == 0 {
        return Err(MaterialSurfaceLoopBenchmarkError::MissingMaterialContact);
    }
    if measurement.mana_material_transition_count == 0 {
        return Err(MaterialSurfaceLoopBenchmarkError::MissingManaMaterialTransition);
    }
    if measurement.tick_elapsed_ns == 0
        || measurement.encoded_snapshot_bytes == 0
        || measurement.provenance_event_growth == 0
    {
        return Err(MaterialSurfaceLoopBenchmarkError::MissingMeasurement);
    }
    if matches!(
        measurement.mode,
        MaterialSurfaceLoopBenchmarkMode::WorldChunksQuery
    ) && measurement.observer_response_bytes == 0
    {
        return Err(MaterialSurfaceLoopBenchmarkError::MissingObserverPayload);
    }
    Ok(())
}

/// `observer_off` and `world_chunks_query` share a seed and config and differ only in whether a
/// read-only observer query ran each tick, so their canonical state after the same number of
/// measured ticks must match. This is the cross-measurement check the plan's Finding "existing
/// benchmark-infrastructure gaps" section called for: without it, a harness bug that silently ran
/// two non-equivalent configs would still report a difference and label it "observer overhead".
pub(crate) fn validate_benchmark_report(
    report: &MaterialSurfaceLoopBenchmarkReport,
) -> Result<(), MaterialSurfaceLoopBenchmarkError> {
    if report.observer_off.canonical_state != report.world_chunks_query.canonical_state {
        return Err(MaterialSurfaceLoopBenchmarkError::CanonicalStateDivergedAcrossObserverModes);
    }
    Ok(())
}

pub(crate) fn validate_experiment_recipe_mana_source_benchmark_config(
    config: ExperimentRecipeManaSourceBenchmarkConfig,
) -> Result<(), MaterialSurfaceLoopBenchmarkError> {
    if config.measurement_ticks == 0 {
        return Err(MaterialSurfaceLoopBenchmarkError::ZeroMeasurementTicks);
    }
    Ok(())
}

pub(crate) fn validate_experiment_recipe_mana_source_benchmark_measurement(
    measurement: &ExperimentRecipeManaSourceBenchmarkMeasurement,
) -> Result<(), MaterialSurfaceLoopBenchmarkError> {
    match measurement.mode {
        ExperimentRecipeManaSourceBenchmarkMode::Disabled
            if measurement.source_receipt_count != 0 || measurement.source_event_count != 0 =>
        {
            return Err(MaterialSurfaceLoopBenchmarkError::InvalidExperimentRecipeSourceWorkload);
        }
        ExperimentRecipeManaSourceBenchmarkMode::Enabled
            if measurement.source_receipt_count != 1 || measurement.source_event_count != 1 =>
        {
            return Err(MaterialSurfaceLoopBenchmarkError::InvalidExperimentRecipeSourceWorkload);
        }
        ExperimentRecipeManaSourceBenchmarkMode::Disabled
        | ExperimentRecipeManaSourceBenchmarkMode::Enabled => {}
    }
    if measurement.tick_elapsed_ns == 0
        || measurement.encoded_snapshot_bytes == 0
        || measurement.provenance_event_growth == 0
    {
        return Err(MaterialSurfaceLoopBenchmarkError::MissingMeasurement);
    }
    if measurement.observer_response_bytes == 0 {
        return Err(MaterialSurfaceLoopBenchmarkError::MissingObserverPayload);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_measurement() -> MaterialSurfaceLoopBenchmarkMeasurement {
        MaterialSurfaceLoopBenchmarkMeasurement {
            mode: MaterialSurfaceLoopBenchmarkMode::ObserverOff,
            tick_elapsed_ns: 1,
            mean_tick_elapsed_ns: 1,
            peak_rss_kib: None,
            steady_rss_kib: None,
            provenance_event_growth: 1,
            encoded_snapshot_bytes: 1,
            observer_response_bytes: 0,
            promoted_actor_count: 1,
            material_surface_site_count: 1,
            material_contact_count: 1,
            mana_material_transition_count: 1,
            canonical_state: crate::ExperimentDigest {
                schema_version: crate::DigestSchemaVersion::new(0),
                fingerprint: causafera_core::StateFingerprint::new([0; 32]),
            },
        }
    }

    fn valid_source_measurement(
        mode: ExperimentRecipeManaSourceBenchmarkMode,
    ) -> ExperimentRecipeManaSourceBenchmarkMeasurement {
        let (source_receipt_count, source_event_count) = match mode {
            ExperimentRecipeManaSourceBenchmarkMode::Disabled => (0, 0),
            ExperimentRecipeManaSourceBenchmarkMode::Enabled => (1, 1),
        };
        ExperimentRecipeManaSourceBenchmarkMeasurement {
            mode,
            tick_elapsed_ns: 1,
            mean_tick_elapsed_ns: 1,
            provenance_event_growth: 1,
            encoded_snapshot_bytes: 1,
            observer_response_bytes: 1,
            source_receipt_count,
            source_event_count,
        }
    }

    #[test]
    fn measurement_rejects_missing_material_contact() {
        // Given: an otherwise complete benchmark measurement with no physical contact evidence.
        let mut measurement = valid_measurement();
        measurement.material_contact_count = 0;

        // When: the production benchmark validation boundary checks its evidence.
        let result = validate_benchmark_measurement(&measurement);

        // Then: the harness rejects the run instead of reporting a completed loop.
        assert!(matches!(
            result,
            Err(MaterialSurfaceLoopBenchmarkError::MissingMaterialContact)
        ));
    }

    #[test]
    fn measurement_rejects_missing_mana_material_transition() {
        // Given: an otherwise complete benchmark measurement with no mana consequence.
        let mut measurement = valid_measurement();
        measurement.mana_material_transition_count = 0;

        // When: the production benchmark validation boundary checks its evidence.
        let result = validate_benchmark_measurement(&measurement);

        // Then: the harness rejects the run instead of reporting a completed loop.
        assert!(matches!(
            result,
            Err(MaterialSurfaceLoopBenchmarkError::MissingManaMaterialTransition)
        ));
    }

    #[test]
    fn measurement_rejects_missing_required_metric() {
        // Given: an otherwise complete benchmark measurement without provenance growth.
        let mut measurement = valid_measurement();
        measurement.provenance_event_growth = 0;

        // When: the production benchmark validation boundary checks its metrics.
        let result = validate_benchmark_measurement(&measurement);

        // Then: the harness rejects the incomplete local envelope.
        assert!(matches!(
            result,
            Err(MaterialSurfaceLoopBenchmarkError::MissingMeasurement)
        ));
    }

    #[test]
    fn report_accepts_matching_canonical_state_across_modes() {
        // Given: observer_off and world_chunks_query measurements with the same canonical state,
        // as a same-seed, same-config, read-only-query-only pair of runs should produce.
        let observer_off = valid_measurement();
        let world_chunks_query = MaterialSurfaceLoopBenchmarkMeasurement {
            mode: MaterialSurfaceLoopBenchmarkMode::WorldChunksQuery,
            observer_response_bytes: 1,
            ..valid_measurement()
        };
        let report = MaterialSurfaceLoopBenchmarkReport {
            version: crate::MATERIAL_SURFACE_LOOP_BENCHMARK_VERSION,
            config: MaterialSurfaceLoopBenchmarkConfig::default(),
            observer_off,
            world_chunks_query,
            world_chunks_observer_overhead_ns: 0,
        };

        // When: the report-level validation boundary checks canonical equality.
        let result = validate_benchmark_report(&report);

        // Then: a genuinely equivalent pair of runs is accepted.
        assert!(result.is_ok());
    }

    #[test]
    fn report_rejects_diverged_canonical_state_across_modes() {
        // Given: a world_chunks_query measurement whose canonical state differs from
        // observer_off's — the failure mode this check exists to catch, where the harness would
        // otherwise silently compare two non-equivalent runs and call the difference "overhead".
        let observer_off = valid_measurement();
        let world_chunks_query = MaterialSurfaceLoopBenchmarkMeasurement {
            mode: MaterialSurfaceLoopBenchmarkMode::WorldChunksQuery,
            observer_response_bytes: 1,
            canonical_state: crate::ExperimentDigest {
                schema_version: crate::DigestSchemaVersion::new(0),
                fingerprint: causafera_core::StateFingerprint::new([1; 32]),
            },
            ..valid_measurement()
        };
        let report = MaterialSurfaceLoopBenchmarkReport {
            version: crate::MATERIAL_SURFACE_LOOP_BENCHMARK_VERSION,
            config: MaterialSurfaceLoopBenchmarkConfig::default(),
            observer_off,
            world_chunks_query,
            world_chunks_observer_overhead_ns: 0,
        };

        // When: the report-level validation boundary checks canonical equality.
        let result = validate_benchmark_report(&report);

        // Then: the harness rejects the report instead of reporting a comparison between
        // non-equivalent runs.
        assert!(matches!(
            result,
            Err(MaterialSurfaceLoopBenchmarkError::CanonicalStateDivergedAcrossObserverModes)
        ));
    }

    #[test]
    fn source_benchmark_rejects_zero_measurement_ticks() {
        // Given: a source benchmark configuration with an empty measurement window.
        let config = ExperimentRecipeManaSourceBenchmarkConfig {
            measurement_ticks: 0,
            ..ExperimentRecipeManaSourceBenchmarkConfig::default()
        };

        // When: source benchmark configuration validation runs.
        let result = validate_experiment_recipe_mana_source_benchmark_config(config);

        // Then: the bounded measurement cannot be reported.
        assert!(matches!(
            result,
            Err(MaterialSurfaceLoopBenchmarkError::ZeroMeasurementTicks)
        ));
    }

    #[test]
    fn source_benchmark_enabled_requires_one_receipt_and_event() {
        // Given: an enabled source measurement missing its committed source event.
        let mut measurement =
            valid_source_measurement(ExperimentRecipeManaSourceBenchmarkMode::Enabled);
        measurement.source_event_count = 0;

        // When: source benchmark measurement validation runs.
        let result = validate_experiment_recipe_mana_source_benchmark_measurement(&measurement);

        // Then: the enabled envelope is rejected instead of hiding a missing source commit.
        assert!(matches!(
            result,
            Err(MaterialSurfaceLoopBenchmarkError::InvalidExperimentRecipeSourceWorkload)
        ));
    }

    #[test]
    fn source_benchmark_disabled_requires_no_receipt_or_event() {
        // Given: a disabled source measurement containing an unexpected receipt.
        let mut measurement =
            valid_source_measurement(ExperimentRecipeManaSourceBenchmarkMode::Disabled);
        measurement.source_receipt_count = 1;

        // When: source benchmark measurement validation runs.
        let result = validate_experiment_recipe_mana_source_benchmark_measurement(&measurement);

        // Then: the disabled envelope is rejected instead of reporting source activity.
        assert!(matches!(
            result,
            Err(MaterialSurfaceLoopBenchmarkError::InvalidExperimentRecipeSourceWorkload)
        ));
    }

    #[test]
    fn source_benchmark_rejects_missing_bounded_metric() {
        // Given: an otherwise complete enabled source measurement without an observer payload.
        let mut measurement =
            valid_source_measurement(ExperimentRecipeManaSourceBenchmarkMode::Enabled);
        measurement.observer_response_bytes = 0;

        // When: source benchmark measurement validation runs.
        let result = validate_experiment_recipe_mana_source_benchmark_measurement(&measurement);

        // Then: the bounded observer metric is required.
        assert!(matches!(
            result,
            Err(MaterialSurfaceLoopBenchmarkError::MissingObserverPayload)
        ));
    }
}
