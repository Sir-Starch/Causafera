use std::time::Instant;

use causafera_observer_api::{ObserverQuery, QueryStatus};
use causafera_observer_wire::{ProtocolHandler, decode_response, encode_query};
use causafera_types::{ChartChunkCoord, ChunkCoord};

use crate::benchmark_validation::{
    MaterialSurfaceLoopBenchmarkError, validate_benchmark_config, validate_benchmark_measurement,
    validate_benchmark_report, validate_experiment_recipe_mana_source_benchmark_config,
    validate_experiment_recipe_mana_source_benchmark_measurement,
};
use crate::{
    EXPERIMENT_RECIPE_MANA_SOURCE_EVENT_KIND, EXPERIMENT_RECIPE_MANA_SOURCE_POLICY_SCHEMA_V1,
    ExperimentDigest, ExperimentRecipeManaSource, ExperimentRecipeManaSourceRecipe,
    MAX_EXPERIMENT_RECIPE_MANA_SOURCES, Runtime, RuntimeConfig, RuntimeError, RuntimeSnapshotData,
    RuntimeState, assemble_envelope,
};

pub const MATERIAL_SURFACE_LOOP_BENCHMARK_VERSION: u32 = 1;
pub const MATERIAL_SURFACE_LOOP_BENCHMARK_SEED: u64 = 0x4D41_5445_5249_414C;
pub const EXPERIMENT_RECIPE_MANA_SOURCE_BENCHMARK_VERSION: u32 = 1;
pub const EXPERIMENT_RECIPE_MANA_SOURCE_BENCHMARK_WARMUP_TICKS: u64 = 4;
pub const EXPERIMENT_RECIPE_MANA_SOURCE_BENCHMARK_AMOUNT: i64 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExperimentRecipeManaSourceBenchmarkConfig {
    pub seed: u64,
    pub measurement_ticks: u64,
}

impl Default for ExperimentRecipeManaSourceBenchmarkConfig {
    fn default() -> Self {
        Self {
            seed: MATERIAL_SURFACE_LOOP_BENCHMARK_SEED,
            measurement_ticks: 32,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExperimentRecipeManaSourceBenchmarkMode {
    Disabled,
    Enabled,
}

impl ExperimentRecipeManaSourceBenchmarkMode {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Disabled => "disabled",
            Self::Enabled => "enabled",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExperimentRecipeManaSourceBenchmarkMeasurement {
    pub mode: ExperimentRecipeManaSourceBenchmarkMode,
    pub tick_elapsed_ns: u128,
    pub mean_tick_elapsed_ns: u128,
    pub provenance_event_growth: u64,
    pub encoded_snapshot_bytes: u64,
    pub observer_response_bytes: u64,
    pub source_receipt_count: u64,
    pub source_event_count: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExperimentRecipeManaSourceBenchmarkReport {
    pub version: u32,
    pub config: ExperimentRecipeManaSourceBenchmarkConfig,
    pub disabled: ExperimentRecipeManaSourceBenchmarkMeasurement,
    pub enabled: ExperimentRecipeManaSourceBenchmarkMeasurement,
}

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
    /// The canonical digest after the measured ticks. `ObserverOff` and `WorldChunksQuery` runs
    /// share a seed and config and differ only in whether a read-only observer query ran each
    /// tick, so this is required to match between them (`validate_benchmark_report`) — a canonical
    /// difference here would mean the harness is silently comparing two non-equivalent runs and
    /// mislabeling the difference as "observer overhead".
    pub canonical_state: ExperimentDigest,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MaterialSurfaceLoopBenchmarkReport {
    pub version: u32,
    pub config: MaterialSurfaceLoopBenchmarkConfig,
    pub observer_off: MaterialSurfaceLoopBenchmarkMeasurement,
    pub world_chunks_query: MaterialSurfaceLoopBenchmarkMeasurement,
    pub world_chunks_observer_overhead_ns: i128,
}

/// The wall-clock cost, in nanoseconds, of computing each of `RuntimeState`'s two digests once.
/// Measured by calling the same `pub(crate)` functions `RuntimeState::snapshot` calls on every
/// tick; read-only and side-effect-free, so calling it in addition to a normal tick does not
/// change simulation state or determinism.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DigestCostSample {
    pub physical_state_digest_ns: u128,
    pub history_digest_ns: u128,
}

/// Measure `physical_state_digest`/`history_digest` cost at the runtime's current tick, in
/// isolation from the rest of `RuntimeState::snapshot`. Same-crate access to both `pub(crate)`
/// functions is exactly why this lives in `benchmark.rs` rather than a standalone example crate.
pub fn measure_digest_cost(runtime: &Runtime) -> Result<DigestCostSample, RuntimeError> {
    let mut state = runtime
        .state
        .lock()
        .map_err(|_| RuntimeError::StatePoisoned)?;
    let time = runtime.current_time();
    let started = Instant::now();
    std::hint::black_box(state.physical_state_digest(time));
    let physical_state_digest_ns = started.elapsed().as_nanos();
    let started = Instant::now();
    std::hint::black_box(state.history_digest());
    let history_digest_ns = started.elapsed().as_nanos();
    Ok(DigestCostSample {
        physical_state_digest_ns,
        history_digest_ns,
    })
}

pub fn run_material_surface_loop_benchmark(
    config: MaterialSurfaceLoopBenchmarkConfig,
) -> Result<MaterialSurfaceLoopBenchmarkReport, MaterialSurfaceLoopBenchmarkError> {
    validate_benchmark_config(config)?;
    let observer_off = measure(config, MaterialSurfaceLoopBenchmarkMode::ObserverOff)?;
    let world_chunks_query = measure(config, MaterialSurfaceLoopBenchmarkMode::WorldChunksQuery)?;
    let report = MaterialSurfaceLoopBenchmarkReport {
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
    };
    validate_benchmark_report(&report)?;
    Ok(report)
}

pub fn run_experiment_recipe_mana_source_benchmark(
    config: ExperimentRecipeManaSourceBenchmarkConfig,
) -> Result<ExperimentRecipeManaSourceBenchmarkReport, MaterialSurfaceLoopBenchmarkError> {
    validate_experiment_recipe_mana_source_benchmark_config(config)?;
    let disabled = measure_experiment_recipe_mana_source(
        config,
        ExperimentRecipeManaSourceBenchmarkMode::Disabled,
    )?;
    let enabled = measure_experiment_recipe_mana_source(
        config,
        ExperimentRecipeManaSourceBenchmarkMode::Enabled,
    )?;
    Ok(ExperimentRecipeManaSourceBenchmarkReport {
        version: EXPERIMENT_RECIPE_MANA_SOURCE_BENCHMARK_VERSION,
        config,
        disabled,
        enabled,
    })
}

fn measure_experiment_recipe_mana_source(
    config: ExperimentRecipeManaSourceBenchmarkConfig,
    mode: ExperimentRecipeManaSourceBenchmarkMode,
) -> Result<ExperimentRecipeManaSourceBenchmarkMeasurement, MaterialSurfaceLoopBenchmarkError> {
    let mut runtime = Runtime::new(experiment_recipe_config(config.seed, mode)?)?;
    runtime.run_ticks(EXPERIMENT_RECIPE_MANA_SOURCE_BENCHMARK_WARMUP_TICKS)?;
    let provenance_before = trace_count(&runtime.export_snapshot()?)?;
    let started = Instant::now();
    let mut observer_response_bytes = 0_u64;
    for _ in 0..config.measurement_ticks {
        let snapshot = runtime.tick()?;
        observer_response_bytes = observer_response_bytes
            .checked_add(bounded_world_chunks_query(&runtime, snapshot)?)
            .ok_or(MaterialSurfaceLoopBenchmarkError::MetricOverflow)?;
    }
    let tick_elapsed_ns = started.elapsed().as_nanos();
    let exported = runtime.export_snapshot()?;
    let provenance_after = trace_count(&exported)?;
    let encoded_snapshot_bytes = u64::try_from(assemble_envelope(&exported)?.encode()?.len())
        .map_err(|_| MaterialSurfaceLoopBenchmarkError::MetricOverflow)?;
    let source_receipt_count =
        u64::try_from(runtime.executed_experiment_recipe_mana_sources()?.len())
            .map_err(|_| MaterialSurfaceLoopBenchmarkError::MetricOverflow)?;
    let source_event_count = source_event_count(&exported)?;
    let measurement = ExperimentRecipeManaSourceBenchmarkMeasurement {
        mode,
        tick_elapsed_ns,
        mean_tick_elapsed_ns: tick_elapsed_ns / u128::from(config.measurement_ticks),
        provenance_event_growth: provenance_after.saturating_sub(provenance_before),
        encoded_snapshot_bytes,
        observer_response_bytes,
        source_receipt_count,
        source_event_count,
    };
    validate_experiment_recipe_mana_source_benchmark_measurement(&measurement)?;
    Ok(measurement)
}

fn experiment_recipe_config(
    seed: u64,
    mode: ExperimentRecipeManaSourceBenchmarkMode,
) -> Result<RuntimeConfig, MaterialSurfaceLoopBenchmarkError> {
    let mut config = production_loop_config(seed);
    config.material_surface_signals_enabled = false;
    config.mana_parameters.effect_threshold = i64::MAX;
    config.mana_parameters.diffusion = 0;
    config.mana_parameters.decay = 0;
    let target_chunk = ChartChunkCoord::new(config.chart_id, ChunkCoord::new(0, 0, 0));
    let mut records = Vec::with_capacity(MAX_EXPERIMENT_RECIPE_MANA_SOURCES);
    for index in 0..MAX_EXPERIMENT_RECIPE_MANA_SOURCES {
        let index =
            u64::try_from(index).map_err(|_| MaterialSurfaceLoopBenchmarkError::MetricOverflow)?;
        records.push(ExperimentRecipeManaSource {
            source_record_id: index + 1,
            enabled: matches!(mode, ExperimentRecipeManaSourceBenchmarkMode::Enabled) && index == 0,
            scheduled_tick: EXPERIMENT_RECIPE_MANA_SOURCE_BENCHMARK_WARMUP_TICKS + 1 + index,
            target_chunk,
            cell_index: u16::try_from(index)
                .map_err(|_| MaterialSurfaceLoopBenchmarkError::MetricOverflow)?,
            amount: EXPERIMENT_RECIPE_MANA_SOURCE_BENCHMARK_AMOUNT,
            per_record_maximum: EXPERIMENT_RECIPE_MANA_SOURCE_BENCHMARK_AMOUNT,
            policy_schema_id: EXPERIMENT_RECIPE_MANA_SOURCE_POLICY_SCHEMA_V1,
        });
    }
    config.experiment_recipe_mana_sources = ExperimentRecipeManaSourceRecipe {
        records,
        recipe_budget: match mode {
            ExperimentRecipeManaSourceBenchmarkMode::Disabled => 0,
            ExperimentRecipeManaSourceBenchmarkMode::Enabled => {
                EXPERIMENT_RECIPE_MANA_SOURCE_BENCHMARK_AMOUNT
            }
        },
    };
    Ok(config)
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
    let mut last_snapshot = None;
    for _ in 0..config.measurement_ticks {
        let snapshot = runtime.tick()?;
        last_snapshot = Some(snapshot.clone());
        if matches!(mode, MaterialSurfaceLoopBenchmarkMode::WorldChunksQuery) {
            let response_bytes = bounded_world_chunks_query(&runtime, snapshot)?;
            observer_response_bytes = observer_response_bytes
                .checked_add(response_bytes)
                .ok_or(MaterialSurfaceLoopBenchmarkError::MetricOverflow)?;
        }
    }
    let tick_elapsed_ns = started.elapsed().as_nanos();
    let canonical_state = last_snapshot
        .ok_or(MaterialSurfaceLoopBenchmarkError::MissingMeasurement)?
        .canonical_state;
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
        canonical_state,
    };
    validate_benchmark_measurement(&measurement)?;
    Ok(measurement)
}

pub fn production_loop_config(seed: u64) -> RuntimeConfig {
    let mut config = RuntimeConfig::new(seed);
    config.active_chunk_radius = 0;
    config.actor_count = 1;
    config.sensor_count = 1;
    config.bootstrap_population = 8;
    config.mana_parameters.effect_threshold = 1;
    config.mana_parameters.effect_hysteresis = 0;
    config
}

pub fn measure_import_wall_time(
    data: &RuntimeSnapshotData,
    iterations: u32,
) -> Result<u128, RuntimeError> {
    if iterations == 0 {
        return Err(RuntimeError::InvalidSnapshot(
            "import benchmark iterations must be non-zero",
        ));
    }
    let started = Instant::now();
    for _ in 0..iterations {
        let state = RuntimeState::import_snapshot(std::hint::black_box(data.clone()))?;
        std::hint::black_box(state);
    }
    Ok(started.elapsed().as_nanos() / u128::from(iterations))
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

pub(crate) fn trace_count(
    snapshot: &crate::RuntimeSnapshotData,
) -> Result<u64, MaterialSurfaceLoopBenchmarkError> {
    u64::try_from(snapshot.traces.events.len())
        .map_err(|_| MaterialSurfaceLoopBenchmarkError::MetricOverflow)
}

pub(crate) fn source_event_count(
    snapshot: &crate::RuntimeSnapshotData,
) -> Result<u64, MaterialSurfaceLoopBenchmarkError> {
    u64::try_from(
        snapshot
            .traces
            .events
            .iter()
            .filter(|event| event.kind.raw() == EXPERIMENT_RECIPE_MANA_SOURCE_EVENT_KIND)
            .count(),
    )
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

pub const BOOTSTRAP_CLOSURE_BENCHMARK_VERSION: u32 = 1;

/// The bounded envelope the observer session already runs: nine active chunks,
/// bootstrap population 512, eight promoted actors, one sensor aperture each.
///
/// This is the workload the measurements below describe and the only workload
/// they describe. Nothing here supports a claim about a larger world.
pub fn bootstrap_closure_config(seed: u64) -> RuntimeConfig {
    let mut config = RuntimeConfig::new(seed);
    config.active_chunk_radius = 1;
    config.active_chunk_shape = crate::ActiveChunkShape::Area;
    config.bootstrap_population = 512;
    config.actor_count = 8;
    config.sensor_count = 1;
    config
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BootstrapClosureBenchmarkConfig {
    pub seed: u64,
    /// Unmeasured repetitions run before sampling starts, so a cold process,
    /// an unwarmed allocator, and first-touch page faults do not land in the
    /// reported distribution.
    pub warmup_iterations: u32,
    /// Measured bootstrap constructions. Each is one sample, not a mean.
    pub iterations: u32,
    pub import_iterations: u32,
    /// Measured observer polls, for the overhead control and its counterpart.
    pub observer_polls: u32,
}

impl Default for BootstrapClosureBenchmarkConfig {
    fn default() -> Self {
        Self {
            seed: 2_026,
            warmup_iterations: 4,
            iterations: 20,
            import_iterations: 20,
            observer_polls: 20,
        }
    }
}

/// One measured distribution, reported the way this repository's benchmark
/// methodology requires: every raw sample retained alongside the summary, so a
/// reader can see the spread rather than take a single number's word for it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BenchmarkSamples {
    pub samples_ns: Vec<u128>,
    pub mean_ns: u128,
    pub median_ns: u128,
    pub stddev_ns: u128,
    pub minimum_ns: u128,
    pub maximum_ns: u128,
}

impl BenchmarkSamples {
    pub fn from_samples(samples_ns: Vec<u128>) -> Result<Self, RuntimeError> {
        if samples_ns.is_empty() {
            return Err(RuntimeError::InvalidSnapshot(
                "a benchmark distribution needs at least one sample",
            ));
        }
        let count = samples_ns.len() as u128;
        let mean_ns = samples_ns.iter().sum::<u128>() / count;
        let mut sorted = samples_ns.clone();
        sorted.sort_unstable();
        let median_ns = if sorted.len().is_multiple_of(2) {
            (sorted[sorted.len() / 2 - 1] + sorted[sorted.len() / 2]) / 2
        } else {
            sorted[sorted.len() / 2]
        };
        // Population variance over the samples actually taken. Integer maths
        // throughout: a benchmark that reports in floating point invites a
        // precision it does not have.
        let variance = samples_ns
            .iter()
            .map(|sample| {
                let delta = sample.abs_diff(mean_ns);
                delta * delta
            })
            .sum::<u128>()
            / count;
        Ok(Self {
            mean_ns,
            median_ns,
            stddev_ns: variance.isqrt(),
            minimum_ns: sorted[0],
            maximum_ns: sorted[sorted.len() - 1],
            samples_ns,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BootstrapClosureBenchmarkReport {
    pub version: u32,
    pub config: BootstrapClosureBenchmarkConfig,
    /// The measured envelope, restated from the constructed runtime rather than
    /// from the configuration, so a report cannot describe a workload it did not
    /// actually run.
    pub active_chunk_count: u32,
    pub bootstrap_population: u64,
    pub promoted_actor_count: u64,
    pub sensor_count: u8,
    pub receipt_count: u32,
    /// Wall time of one complete `Runtime::new`, bootstrap included.
    pub bootstrap_wall_time: BenchmarkSamples,
    /// Wall time of one `RuntimeState::import_snapshot`, canonical record
    /// validation included.
    pub import_wall_time: BenchmarkSamples,
    /// The complete encoded snapshot envelope, header and section directory
    /// included — the same measurement the other benchmarks in this module
    /// report, so the figures are comparable.
    pub encoded_snapshot_bytes: u64,
    /// What the canonical record itself costs inside the population/bootstrap
    /// section, measured as the difference against an absent record.
    pub bootstrap_record_bytes: u64,
    /// Trace events the six stages committed, completions included.
    pub bootstrap_provenance_events: u64,
    /// Wall time of one authoritative summary read with no observer encoding —
    /// the control.
    pub control_poll_wall_time: BenchmarkSamples,
    /// The same read plus the bounded runtime-summary encoding.
    pub observer_poll_wall_time: BenchmarkSamples,
    pub observer_summary_bytes: u64,
    pub plan_id: u64,
    pub physical_state_digest: crate::PhysicalStateDigest,
    pub history_digest: crate::HistoryDigest,
}

/// Measure the bounded production bootstrap envelope.
///
/// Every figure is a measurement of the stated envelope on the machine that ran
/// it. None of them is a scale result, a throughput claim, or a regression
/// threshold; the repository has no reference hardware and this benchmark does
/// not establish one.
pub fn run_bootstrap_closure_benchmark(
    config: BootstrapClosureBenchmarkConfig,
) -> Result<BootstrapClosureBenchmarkReport, RuntimeError> {
    if config.iterations == 0 || config.import_iterations == 0 || config.observer_polls == 0 {
        return Err(RuntimeError::InvalidSnapshot(
            "bootstrap benchmark iterations must be non-zero",
        ));
    }
    let runtime_config = bootstrap_closure_config(config.seed);

    for _ in 0..config.warmup_iterations {
        let runtime = Runtime::new(std::hint::black_box(runtime_config.clone()))?;
        std::hint::black_box(&runtime);
    }

    let mut bootstrap_samples = Vec::with_capacity(config.iterations as usize);
    for _ in 0..config.iterations {
        let started = Instant::now();
        let runtime = Runtime::new(std::hint::black_box(runtime_config.clone()))?;
        bootstrap_samples.push(started.elapsed().as_nanos());
        std::hint::black_box(&runtime);
    }
    let bootstrap_wall_time = BenchmarkSamples::from_samples(bootstrap_samples)?;

    let runtime = Runtime::new(runtime_config)?;
    let data = runtime.export_snapshot()?;
    let summary = runtime.snapshot()?;

    // The whole encoded envelope, header and section directory included, so this
    // figure means the same thing as the one the material-surface and
    // experiment-recipe benchmarks already report rather than a sections-only
    // subtotal that quietly understates a file on disk.
    let encoded_snapshot_bytes = u64::try_from(
        assemble_envelope(&data)
            .and_then(|envelope| envelope.encode())
            .map_err(|_| RuntimeError::InvalidSnapshot("bootstrap benchmark envelope must encode"))?
            .len(),
    )
    .map_err(|_| RuntimeError::InvalidSnapshot("bootstrap benchmark envelope is too large"))?;
    let with_record =
        crate::encode_population_section(&data.population, &data.bootstrap).len() as u64;
    let without_record = crate::encode_population_section(
        &data.population,
        &crate::BootstrapReceiptSnapshot::absent(),
    )
    .len() as u64;
    let bootstrap_record_bytes = with_record.saturating_sub(without_record);
    let bootstrap_provenance_events = data.traces.events.len() as u64;

    for _ in 0..config.warmup_iterations {
        std::hint::black_box(RuntimeState::import_snapshot(std::hint::black_box(
            data.clone(),
        ))?);
    }
    let mut import_samples = Vec::with_capacity(config.import_iterations as usize);
    for _ in 0..config.import_iterations {
        let started = Instant::now();
        let state = RuntimeState::import_snapshot(std::hint::black_box(data.clone()))?;
        import_samples.push(started.elapsed().as_nanos());
        std::hint::black_box(state);
    }
    let import_wall_time = BenchmarkSamples::from_samples(import_samples)?;

    for _ in 0..config.warmup_iterations {
        std::hint::black_box(runtime.snapshot()?);
    }

    // The control and its counterpart are interleaved rather than run in two
    // blocks, so a drift in machine state over the run biases both alike instead
    // of landing entirely on whichever went second.
    let mut control_samples = Vec::with_capacity(config.observer_polls as usize);
    let mut observer_samples = Vec::with_capacity(config.observer_polls as usize);
    let mut observer_summary_bytes = 0_u64;
    for _ in 0..config.observer_polls {
        let started = Instant::now();
        std::hint::black_box(runtime.snapshot()?);
        control_samples.push(started.elapsed().as_nanos());

        let started = Instant::now();
        let polled = runtime.snapshot()?;
        let encoded =
            causafera_observer_wire::encode_observer_snapshot(&polled.observer_snapshot());
        observer_samples.push(started.elapsed().as_nanos());
        observer_summary_bytes = encoded.len() as u64;
        std::hint::black_box(encoded);
    }
    let control_poll_wall_time = BenchmarkSamples::from_samples(control_samples)?;
    let observer_poll_wall_time = BenchmarkSamples::from_samples(observer_samples)?;

    Ok(BootstrapClosureBenchmarkReport {
        version: BOOTSTRAP_CLOSURE_BENCHMARK_VERSION,
        config,
        active_chunk_count: summary.active_chunk_count,
        bootstrap_population: summary.population_total + u64::from(summary.actor_count),
        promoted_actor_count: u64::from(summary.actor_count),
        // Read back from a promoted actor, not from the configuration: the
        // point of this block is that a report cannot describe a workload it
        // did not run, and echoing the config here defeated it for this field.
        sensor_count: data
            .actors_objective
            .actors
            .first()
            .map_or(0, |(_, actor)| actor.sensors.len() as u8),
        receipt_count: data.bootstrap.receipts.len() as u32,
        bootstrap_wall_time,
        import_wall_time,
        encoded_snapshot_bytes,
        bootstrap_record_bytes,
        bootstrap_provenance_events,
        control_poll_wall_time,
        observer_poll_wall_time,
        observer_summary_bytes,
        plan_id: data.bootstrap.plan.id.raw(),
        physical_state_digest: summary.physical_state_digest,
        history_digest: summary.history_digest,
    })
}
