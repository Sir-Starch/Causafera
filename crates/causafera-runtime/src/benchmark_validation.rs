use causafera_observer_wire::WireError;
use thiserror::Error;

use crate::RuntimeError;
use crate::benchmark::{
    MaterialSurfaceLoopBenchmarkConfig, MaterialSurfaceLoopBenchmarkMeasurement,
    MaterialSurfaceLoopBenchmarkMode,
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
}
