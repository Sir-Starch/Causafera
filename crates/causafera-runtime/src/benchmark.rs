use std::time::Instant;

use causafera_observer_api::{ObserverQuery, QueryStatus};
use causafera_observer_wire::{ProtocolHandler, decode_response, encode_query};

use crate::benchmark_validation::{
    MaterialSurfaceLoopBenchmarkError, validate_benchmark_config, validate_benchmark_measurement,
};
use crate::{Runtime, RuntimeConfig, assemble_envelope};

pub const MATERIAL_SURFACE_LOOP_BENCHMARK_VERSION: u32 = 1;
pub const MATERIAL_SURFACE_LOOP_BENCHMARK_SEED: u64 = 0x4D41_5445_5249_414C;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MaterialSurfaceLoopBenchmarkConfig {
    pub seed: u64,
    pub warmup_ticks: u64,
    pub measurement_ticks: u64,
}

impl Default for MaterialSurfaceLoopBenchmarkConfig {
    fn default() -> Self {
        Self {
            seed: MATERIAL_SURFACE_LOOP_BENCHMARK_SEED,
            warmup_ticks: 4,
            measurement_ticks: 32,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MaterialSurfaceLoopBenchmarkMode {
    ObserverOff,
    WorldChunksQuery,
}

impl MaterialSurfaceLoopBenchmarkMode {
    pub const fn label(self) -> &'static str {
        match self {
            Self::ObserverOff => "observer_off",
            Self::WorldChunksQuery => "world_chunks_query",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MaterialSurfaceLoopBenchmarkMeasurement {
    pub mode: MaterialSurfaceLoopBenchmarkMode,
    pub tick_elapsed_ns: u128,
    pub mean_tick_elapsed_ns: u128,
    pub peak_rss_kib: Option<u64>,
    pub steady_rss_kib: Option<u64>,
    pub provenance_event_growth: u64,
    pub encoded_snapshot_bytes: u64,
    pub observer_response_bytes: u64,
    pub promoted_actor_count: u64,
    pub material_surface_site_count: u64,
    pub material_contact_count: u64,
    pub mana_material_transition_count: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MaterialSurfaceLoopBenchmarkReport {
    pub version: u32,
    pub config: MaterialSurfaceLoopBenchmarkConfig,
    pub observer_off: MaterialSurfaceLoopBenchmarkMeasurement,
    pub world_chunks_query: MaterialSurfaceLoopBenchmarkMeasurement,
    pub world_chunks_observer_overhead_ns: i128,
}

pub fn run_material_surface_loop_benchmark(
    config: MaterialSurfaceLoopBenchmarkConfig,
) -> Result<MaterialSurfaceLoopBenchmarkReport, MaterialSurfaceLoopBenchmarkError> {
    validate_benchmark_config(config)?;
    let observer_off = measure(config, MaterialSurfaceLoopBenchmarkMode::ObserverOff)?;
    let world_chunks_query = measure(config, MaterialSurfaceLoopBenchmarkMode::WorldChunksQuery)?;
    Ok(MaterialSurfaceLoopBenchmarkReport {
        version: MATERIAL_SURFACE_LOOP_BENCHMARK_VERSION,
        config,
        world_chunks_observer_overhead_ns: i128::try_from(world_chunks_query.tick_elapsed_ns)
            .map_err(|_| MaterialSurfaceLoopBenchmarkError::MetricOverflow)?
            .checked_sub(
                i128::try_from(observer_off.tick_elapsed_ns)
                    .map_err(|_| MaterialSurfaceLoopBenchmarkError::MetricOverflow)?,
            )
            .ok_or(MaterialSurfaceLoopBenchmarkError::MetricOverflow)?,
        observer_off,
        world_chunks_query,
    })
}

fn measure(
    config: MaterialSurfaceLoopBenchmarkConfig,
    mode: MaterialSurfaceLoopBenchmarkMode,
) -> Result<MaterialSurfaceLoopBenchmarkMeasurement, MaterialSurfaceLoopBenchmarkError> {
    let mut runtime = Runtime::new(production_loop_config(config.seed))?;
    runtime.run_ticks(config.warmup_ticks)?;
    require_loop_evidence(&runtime.export_snapshot()?)?;

    let provenance_before = trace_count(&runtime.export_snapshot()?)?;
    let started = Instant::now();
    let mut observer_response_bytes = 0_u64;
    for _ in 0..config.measurement_ticks {
        let snapshot = runtime.tick()?;
        if matches!(mode, MaterialSurfaceLoopBenchmarkMode::WorldChunksQuery) {
            let response_bytes = bounded_world_chunks_query(&runtime, snapshot)?;
            observer_response_bytes = observer_response_bytes
                .checked_add(response_bytes)
                .ok_or(MaterialSurfaceLoopBenchmarkError::MetricOverflow)?;
        }
    }
    let tick_elapsed_ns = started.elapsed().as_nanos();
    let exported = runtime.export_snapshot()?;
    let provenance_after = trace_count(&exported)?;
    let encoded_snapshot_bytes = u64::try_from(assemble_envelope(&exported)?.encode()?.len())
        .map_err(|_| MaterialSurfaceLoopBenchmarkError::MetricOverflow)?;
    let (
        promoted_actor_count,
        material_surface_site_count,
        material_contact_count,
        mana_material_transition_count,
    ) = require_loop_evidence(&exported)?;
    let mean_tick_elapsed_ns = tick_elapsed_ns / u128::from(config.measurement_ticks);
    let peak_rss_kib = linux_status_memory_kib("VmHWM:");
    let steady_rss_kib = linux_status_memory_kib("VmRSS:");
    let measurement = MaterialSurfaceLoopBenchmarkMeasurement {
        mode,
        tick_elapsed_ns,
        mean_tick_elapsed_ns,
        peak_rss_kib,
        steady_rss_kib,
        provenance_event_growth: provenance_after - provenance_before,
        encoded_snapshot_bytes,
        observer_response_bytes,
        promoted_actor_count,
        material_surface_site_count,
        material_contact_count,
        mana_material_transition_count,
    };
    validate_benchmark_measurement(&measurement)?;
    Ok(measurement)
}

fn production_loop_config(seed: u64) -> RuntimeConfig {
    let mut config = RuntimeConfig::new(seed);
    config.active_chunk_radius = 0;
    config.actor_count = 1;
    config.sensor_count = 1;
    config.bootstrap_population = 8;
    config.mana_parameters.effect_threshold = 1;
    config.mana_parameters.effect_hysteresis = 0;
    config
}

fn bounded_world_chunks_query(
    runtime: &Runtime,
    snapshot: crate::RuntimeSnapshot,
) -> Result<u64, MaterialSurfaceLoopBenchmarkError> {
    let mut handler = ProtocolHandler::new(snapshot.time);
    handler.set_runtime_snapshot(&snapshot.observer_snapshot());
    handler.set_world_snapshot(&runtime.observer_world_snapshot()?);
    let query = ObserverQuery::world_chunks(snapshot.time.raw());
    let encoded_response = handler.handle_query(&encode_query(&query))?;
    let response = decode_response(&encoded_response)?;
    if response.status != QueryStatus::Ok || response.payload.is_empty() {
        return Err(MaterialSurfaceLoopBenchmarkError::MissingObserverPayload);
    }
    u64::try_from(encoded_response.len())
        .map_err(|_| MaterialSurfaceLoopBenchmarkError::MetricOverflow)
}

fn trace_count(
    snapshot: &crate::RuntimeSnapshotData,
) -> Result<u64, MaterialSurfaceLoopBenchmarkError> {
    u64::try_from(snapshot.traces.events.len())
        .map_err(|_| MaterialSurfaceLoopBenchmarkError::MetricOverflow)
}

fn require_loop_evidence(
    snapshot: &crate::RuntimeSnapshotData,
) -> Result<(u64, u64, u64, u64), MaterialSurfaceLoopBenchmarkError> {
    let promoted_actor_count = u64::try_from(snapshot.actors_objective.actors.len())
        .map_err(|_| MaterialSurfaceLoopBenchmarkError::MetricOverflow)?;
    if promoted_actor_count != 1 {
        return Err(MaterialSurfaceLoopBenchmarkError::InvalidProductionWorkload);
    }
    let material_surface_site_count = u64::try_from(snapshot.material_surfaces.records.len())
        .map_err(|_| MaterialSurfaceLoopBenchmarkError::MetricOverflow)?;
    if material_surface_site_count != 1 {
        return Err(MaterialSurfaceLoopBenchmarkError::InvalidProductionWorkload);
    }
    let material_contact_count =
        snapshot
            .material_surfaces
            .records
            .iter()
            .try_fold(0_u64, |total, record| {
                total
                    .checked_add(record.surface.contact_count)
                    .ok_or(MaterialSurfaceLoopBenchmarkError::MetricOverflow)
            })?;
    if material_contact_count == 0 {
        return Err(MaterialSurfaceLoopBenchmarkError::MissingMaterialContact);
    }
    let mana_material_transition_count = u64::try_from(
        snapshot
            .material_surfaces
            .transitions
            .iter()
            .filter(|transition| transition.mana_effect_trace.is_some())
            .count(),
    )
    .map_err(|_| MaterialSurfaceLoopBenchmarkError::MetricOverflow)?;
    if mana_material_transition_count == 0 {
        return Err(MaterialSurfaceLoopBenchmarkError::MissingManaMaterialTransition);
    }
    Ok((
        promoted_actor_count,
        material_surface_site_count,
        material_contact_count,
        mana_material_transition_count,
    ))
}

fn linux_status_memory_kib(field: &str) -> Option<u64> {
    #[cfg(target_os = "linux")]
    {
        std::fs::read_to_string("/proc/self/status")
            .ok()?
            .lines()
            .find_map(|line| {
                line.strip_prefix(field)
                    .and_then(|value| value.split_whitespace().next())
                    .and_then(|value| value.parse::<u64>().ok())
            })
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = field;
        None
    }
}
