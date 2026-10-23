//! Development-only benchmark: what conserved hydrology costs, measured.
//!
//! Supplies the evidence the benchmark plan of `plans/hydrology.md` asks for.
//! Build in release; debug timings answer a different question.
//!
//! ```text
//! cargo run --release -p causafera-observer --example hydrology_bench
//! ```
//!
//! Two rules shape the harness. Every workload is a production-bootstrapped
//! runtime with an explicit metric, substrate, and forcing schedule — no fixture
//! constructors, because a benchmark of a world that could not exist measures
//! nothing. And every measured run asserts exact conservation before its timing
//! is reported: a run that lost water is not a fast run, it is a wrong one.
//!
//! Timing is deliberately not the oracle for the resolution workload. The
//! retained-fine/coarse design earns its keep only if a coarse chunk evaluates
//! fewer vertical groups and internal faces than a fine one, so the harness
//! counts both directly and reports them beside the milliseconds.

use std::collections::{BTreeMap, BTreeSet};
use std::io::Write;
use std::num::{NonZeroU32, NonZeroU64};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use causafera_domains::process;
use causafera_geography::{
    HydrologyBoundaryCondition, HydrologyCarrierKey, HydrologyCellKey, HydrologyGridMetric,
};
use causafera_runtime::{
    ActiveChunkShape, HYDROLOGY_BOOTSTRAP_PARAMETERS_SCHEMA_V1, HYDROLOGY_LIMITS_SCHEMA_V1,
    HydrologyBootstrapParameters, HydrologyConfig, HydrologyForcingSpec, HydrologyRuntimeState,
    Runtime, RuntimeConfig,
};
use causafera_types::{ChartChunkCoord, ChunkCoord, SpatialChartId, WaterVolume};

const WORLD_SEED: u64 = 20_260_731;
/// The plan's floor. Fewer repetitions is a number, not a measurement.
const REPETITIONS: usize = 10;
const WARM_UP_TICKS: u64 = 4;
const HISTORY_RUN_TICKS: u64 = 1_000;
const HISTORY_PROGRESS_INTERVAL: u64 = 10;

fn nz32(value: u32) -> NonZeroU32 {
    NonZeroU32::new(value).expect("the value is positive")
}

fn nz64(value: u64) -> NonZeroU64 {
    NonZeroU64::new(value).expect("the value is positive")
}

fn chart() -> SpatialChartId {
    SpatialChartId::new(1)
}

fn cell(ordinal: u16) -> HydrologyCellKey {
    HydrologyCellKey::new(
        ChartChunkCoord::new(chart(), ChunkCoord::new(0, 0, 0)),
        ordinal,
    )
    .expect("the ordinal is in range")
}

/// Substrate and initial storage that actually move water.
///
/// The transmissivities matter: `floor(transmissivity * millis / (1000 * edge))`
/// is zero for a small one, and a workload whose every conductance floors to
/// zero measures a solver that had nothing to do.
fn parameters(conductive: bool) -> HydrologyBootstrapParameters {
    HydrologyBootstrapParameters {
        schema_version: HYDROLOGY_BOOTSTRAP_PARAMETERS_SCHEMA_V1,
        default_surface_capacity: WaterVolume::new(4_000_000_000),
        default_soil_capacity: WaterVolume::new(4_000_000_000),
        default_groundwater_capacity: WaterVolume::new(4_000_000_000),
        initial_surface: WaterVolume::new(20_000_000),
        initial_soil: WaterVolume::new(100_000),
        initial_groundwater: WaterVolume::new(50_000),
        infiltration_rate_mm_per_second: 200,
        percolation_fraction_num: 1,
        percolation_fraction_den: nz32(4),
        specific_yield_num: 1,
        specific_yield_den: nz32(5),
        aquifer_base_offset_mm: -2_500,
        baseflow_threshold: WaterVolume::new(500),
        baseflow_fraction_num: 1,
        baseflow_fraction_den: nz32(8),
        // Zero transmissivity is the honest way to ask for a vertical-only
        // workload: no conductance means no lateral face is ever evaluated.
        base_surface_transmissivity_mm3_per_second: if conductive { 5_000_000 } else { 0 },
        base_groundwater_transmissivity_mm3_per_second: if conductive { 1_000_000 } else { 0 },
        roughness_reference_mm: nz64(50),
        conveyance_capacity: WaterVolume::new(100_000),
        conveyance_initial_storage: WaterVolume::new(1_000),
        conveyance_inlet_capacity_per_tick: WaterVolume::new(10_000),
        conveyance_release_fraction_num: 1,
        conveyance_release_fraction_den: nz32(4),
        default_boundary: HydrologyBoundaryCondition::CLOSED,
        chart_overrides: BTreeMap::new(),
        cell_overrides: BTreeMap::new(),
    }
}

fn config(radius: u8, shape: ActiveChunkShape, conductive: bool, forcing: bool) -> RuntimeConfig {
    let mut config = RuntimeConfig::new(WORLD_SEED);
    config.active_chunk_radius = radius;
    config.active_chunk_shape = shape;
    config.hydrology = HydrologyConfig {
        enabled: true,
        grid_metrics: [(
            chart(),
            HydrologyGridMetric::new(nz64(1_000_000), nz64(1_000), nz64(1_000)),
        )]
        .into_iter()
        .collect(),
        bootstrap_parameters: Some(parameters(conductive)),
        forcing_schedule: if forcing {
            vec![HydrologyForcingSpec {
                forcing_id: 1,
                scheduled_tick: 3,
                targets: vec![(cell(0), nz64(1)), (cell(1), nz64(3))],
                precipitation_volume: WaterVolume::new(900_000),
                potential_et_volume: WaterVolume::new(120_000),
                external_inflow_volume: WaterVolume::ZERO,
            }]
        } else {
            Vec::new()
        },
        resolution_policy: causafera_domains::HydrologyResolutionPolicy::enabled(4)
            .expect("level four is the maximum"),
        limits_schema: HYDROLOGY_LIMITS_SCHEMA_V1,
    };
    config
}

/// What one measured run actually evaluated, counted rather than estimated.
#[derive(Clone, Copy, Debug, Default)]
struct Work {
    /// Distinct cells or coarse blocks a vertical process settled.
    vertical_groups: usize,
    /// Distinct interior faces a lateral process crossed.
    internal_faces: usize,
    /// Distinct exterior faces a boundary export crossed.
    boundary_faces: usize,
    /// Transfer receipts retained for the latest batch.
    receipts: usize,
    /// Resident hydrology cells.
    cells: usize,
    /// Conveyance edges.
    edges: usize,
}

fn measure_work(state: &HydrologyRuntimeState) -> Work {
    let mut work = Work {
        cells: state.fields.cell_count(),
        edges: state.conveyance.edges().len(),
        ..Work::default()
    };
    let Some(trace) = state.retained_batches.last() else {
        return work;
    };
    let Some(receipts) = state.receipts.get(trace) else {
        return work;
    };
    work.receipts = receipts.len();
    let mut vertical = BTreeSet::new();
    let mut internal = BTreeSet::new();
    let mut boundary = BTreeSet::new();
    for receipt in receipts {
        match receipt.process_kind() {
            process::INFILTRATION
            | process::PERCOLATION
            | process::EVAPOTRANSPIRATION_SURFACE
            | process::EVAPOTRANSPIRATION_SOIL
            | process::BASEFLOW => {
                vertical.insert(receipt.source().encode());
            }
            process::SURFACE_LATERAL | process::GROUNDWATER_LATERAL => {
                // One face, whichever side the receipt was written from.
                let mut pair = [receipt.source().encode(), receipt.target().encode()];
                pair.sort();
                internal.insert(pair);
            }
            process::SURFACE_BOUNDARY_EXPORT | process::GROUNDWATER_BOUNDARY_EXPORT => {
                boundary.insert(receipt.target().encode());
            }
            _ => {}
        }
    }
    work.vertical_groups = vertical.len();
    work.internal_faces = internal.len();
    work.boundary_faces = boundary.len();
    work
}

/// Every committed batch closed exactly, or the timing beside it means nothing.
fn assert_conserved(state: &HydrologyRuntimeState, workload: &str) {
    assert!(
        !state.retained_batches.is_empty(),
        "{workload}: a measured run must commit at least one hydrology batch"
    );
    for trace in &state.retained_batches {
        let receipt = state
            .conservation_receipts
            .get(trace)
            .unwrap_or_else(|| panic!("{workload}: a retained batch must keep its ledger"));
        assert_eq!(
            receipt.residual(),
            0,
            "{workload}: a committed batch must close exactly"
        );
    }
}

/// A workload's timings, reported whole rather than as one number.
struct Sample {
    values: Vec<f64>,
}

impl Sample {
    fn mean(&self) -> f64 {
        self.values.iter().sum::<f64>() / self.values.len() as f64
    }

    fn median(&self) -> f64 {
        let mut sorted = self.values.clone();
        sorted.sort_by(f64::total_cmp);
        let middle = sorted.len() / 2;
        if sorted.len().is_multiple_of(2) {
            (sorted[middle - 1] + sorted[middle]) / 2.0
        } else {
            sorted[middle]
        }
    }

    fn deviation(&self) -> f64 {
        let mean = self.mean();
        let variance = self
            .values
            .iter()
            .map(|value| (value - mean).powi(2))
            .sum::<f64>()
            / self.values.len() as f64;
        variance.sqrt()
    }

    fn minimum(&self) -> f64 {
        self.values.iter().copied().fold(f64::INFINITY, f64::min)
    }

    fn maximum(&self) -> f64 {
        self.values
            .iter()
            .copied()
            .fold(f64::NEG_INFINITY, f64::max)
    }
}

/// Advance until the target tick count, or until the snapshot cap refuses one.
///
/// The cap is not a failure to route around. `MAX_TOTAL_SIZE` is a real
/// operational ceiling that a hydrology-enabled session reaches by accumulating
/// receipts and traces, and the tick it engages at is the most useful number
/// this harness produces. A tick refused this way changed nothing — the staging
/// transaction restored the whole pre-image — so the state measured afterwards
/// is the last one that committed.
fn advance(runtime: &mut Runtime, target: u64) -> u64 {
    for reached in 0..target {
        if let Err(error) = runtime.tick() {
            let message = error.to_string();
            assert!(
                message.contains("snapshot") || message.contains("hydrology snapshot section"),
                "a tick may only stop on the export cap, not on {message}"
            );
            return reached;
        }
    }
    target
}

/// Run one workload `REPETITIONS` times after a warm-up, and report it.
fn workload(name: &str, ticks: u64, build: impl Fn() -> RuntimeConfig) {
    // Warm up on a throwaway runtime: the first run pays for lazy allocation and
    // cold caches, and charging that to the measurement makes the number a
    // report about the first repetition.
    {
        let mut runtime = Runtime::new(build()).expect("the runtime must bootstrap");
        advance(&mut runtime, WARM_UP_TICKS);
    }

    let mut values = Vec::with_capacity(REPETITIONS);
    let mut work = Work::default();
    let mut levels: BTreeMap<u8, usize> = BTreeMap::new();
    let mut reached = ticks;
    for _ in 0..REPETITIONS {
        let mut runtime = Runtime::new(build()).expect("the runtime must bootstrap");
        let started = Instant::now();
        reached = advance(&mut runtime, ticks);
        values.push(started.elapsed().as_secs_f64() * 1000.0);
        let state = runtime.hydrology_state();
        assert_conserved(&state, name);
        work = measure_work(&state);
        levels.clear();
        for entry in state.resolution.values() {
            *levels.entry(entry.level()).or_default() += 1;
        }
    }

    let sample = Sample { values };
    let level_summary = levels
        .iter()
        .map(|(level, chunks)| format!("L{level}x{chunks}"))
        .collect::<Vec<_>>()
        .join(" ");
    let ticks = if reached == ticks {
        format!("{ticks}")
    } else {
        // The cap engaged: report where, not the number that was asked for.
        format!("{reached}/{ticks}")
    };
    println!(
        "{name:<34} {ticks:>9} {:>7} {:>6} {:>8} {:>7} {:>7} {:>8} {:>9.3} {:>9.3} {:>8.3} {:>9.3} {:>9.3}  {level_summary}",
        work.cells,
        work.edges,
        work.vertical_groups,
        work.internal_faces,
        work.boundary_faces,
        work.receipts,
        sample.mean(),
        sample.median(),
        sample.deviation(),
        sample.minimum(),
        sample.maximum(),
    );
}

/// Workload 5: export and import a session holding at least three batches.
fn snapshot_workload() {
    let name = "5. snapshot export/import";
    {
        let mut runtime = Runtime::new(config(1, ActiveChunkShape::Line, true, true))
            .expect("the runtime must bootstrap");
        advance(&mut runtime, WARM_UP_TICKS);
        let _ = runtime.export_snapshot().expect("the export must succeed");
    }

    let mut export = Vec::with_capacity(REPETITIONS);
    let mut import = Vec::with_capacity(REPETITIONS);
    let mut batches = 0;
    for _ in 0..REPETITIONS {
        let mut runtime = Runtime::new(config(1, ActiveChunkShape::Line, true, true))
            .expect("the runtime must bootstrap");
        assert_eq!(advance(&mut runtime, 6), 6, "{name}: six ticks must commit");
        let state = runtime.hydrology_state();
        assert_conserved(&state, name);
        batches = state.retained_batches.len();
        assert!(batches >= 3, "{name}: the plan asks for three batches");

        let started = Instant::now();
        let data = runtime.export_snapshot().expect("the export must succeed");
        export.push(started.elapsed().as_secs_f64() * 1000.0);

        let started = Instant::now();
        let restored = Runtime::from_snapshot(data).expect("the import must succeed");
        import.push(started.elapsed().as_secs_f64() * 1000.0);
        // A round trip that lost a batch is not a faster round trip.
        assert_conserved(&restored.hydrology_state(), name);
        assert_eq!(restored.hydrology_state().retained_batches.len(), batches);
    }

    let export = Sample { values: export };
    let import = Sample { values: import };
    println!(
        "{name:<34} {batches:>9} {:>7} {:>6} {:>8} {:>7} {:>7} {:>8} {:>9.3} {:>9.3} {:>8.3} {:>9.3} {:>9.3}  import {:.3}/{:.3}",
        "-",
        "-",
        "-",
        "-",
        "-",
        "-",
        export.mean(),
        export.median(),
        export.deviation(),
        export.minimum(),
        export.maximum(),
        import.mean(),
        import.deviation(),
    );
}

/// Workload 4: what the engine's own resolution choice costs, per chunk.
///
/// The resolution policy is a compiled constant rather than configuration, so a
/// benchmark cannot ask for a coarse world. It can only report what the engine
/// chose and count the work at each level it produced — which is the honest
/// measurement, and the only one that describes a session anybody can run.
fn resolution_workload() {
    let mut runtime = Runtime::new(config(1, ActiveChunkShape::Area, true, true))
        .expect("the runtime must bootstrap");
    let reached = advance(&mut runtime, 24);
    let state = runtime.hydrology_state();
    assert_conserved(&state, "4. resolution");

    let mut per_level: BTreeMap<u8, (usize, usize, usize)> = BTreeMap::new();
    let trace = state
        .retained_batches
        .last()
        .expect("a committed batch exists");
    let receipts = state.receipts.get(trace).expect("its receipts survive");
    let chunk_of = |key: HydrologyCarrierKey| match key {
        HydrologyCarrierKey::Cell(cell) => Some(cell.chunk()),
        HydrologyCarrierKey::Edge(edge) => Some(edge.low().chunk()),
        HydrologyCarrierKey::ExteriorFace(face) => Some(face.cell().chunk()),
        _ => None,
    };
    for (chunk, entry) in &state.resolution {
        let slot = per_level.entry(entry.level()).or_default();
        slot.0 += 1;
        let _ = chunk;
    }
    for receipt in receipts {
        let Some(chunk) = chunk_of(receipt.source()) else {
            continue;
        };
        let Some(entry) = state.resolution.get(&chunk) else {
            continue;
        };
        let slot = per_level.entry(entry.level()).or_default();
        match receipt.process_kind() {
            process::INFILTRATION
            | process::PERCOLATION
            | process::EVAPOTRANSPIRATION_SURFACE
            | process::EVAPOTRANSPIRATION_SOIL
            | process::BASEFLOW => slot.1 += 1,
            process::SURFACE_LATERAL | process::GROUNDWATER_LATERAL => slot.2 += 1,
            _ => {}
        }
    }
    println!();
    println!(
        "Workload 4: work by the level the engine chose, over nine chunks after {reached} ticks"
    );
    println!(
        "{:>6} {:>8} {:>12} {:>18} {:>18}",
        "level", "chunks", "block edge", "vertical receipts", "lateral receipts"
    );
    for (level, (chunks, vertical, lateral)) in per_level {
        println!(
            "{level:>6} {chunks:>8} {:>12} {vertical:>18} {lateral:>18}",
            causafera_domains::block_edge(level)
        );
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<Vec<_>>()
        .join("")
}

/// Read the bounded envelope's section directory and test whether it contains
/// the legacy complete-trace section. `open_bundle` performs the full envelope
/// integrity check after this structural inspection.
fn bounded_snapshot_has_section(bytes: &[u8], target: u16) -> Result<bool, &'static str> {
    const HEADER_SIZE: usize = 170;
    const SECTION_COUNT_OFFSET: usize = 128;
    const DIRECTORY_OFFSET_OFFSET: usize = 130;
    const DIRECTORY_ENTRY_SIZE: usize = 72;

    if bytes.len() < HEADER_SIZE || bytes.get(..4) != Some(b"OTPS") {
        return Err("snapshot header is truncated or has invalid magic");
    }
    let section_count = u16::from_le_bytes(
        bytes
            .get(SECTION_COUNT_OFFSET..SECTION_COUNT_OFFSET + 2)
            .ok_or("snapshot section count is truncated")?
            .try_into()
            .map_err(|_| "snapshot section count is malformed")?,
    );
    let directory_offset = usize::try_from(u64::from_le_bytes(
        bytes
            .get(DIRECTORY_OFFSET_OFFSET..DIRECTORY_OFFSET_OFFSET + 8)
            .ok_or("snapshot directory offset is truncated")?
            .try_into()
            .map_err(|_| "snapshot directory offset is malformed")?,
    ))
    .map_err(|_| "snapshot directory offset does not fit this platform")?;
    let directory_size = usize::from(section_count)
        .checked_mul(DIRECTORY_ENTRY_SIZE)
        .ok_or("snapshot directory size overflowed")?;
    let directory_end = directory_offset
        .checked_add(directory_size)
        .ok_or("snapshot directory end overflowed")?;
    if directory_offset < HEADER_SIZE || directory_end > bytes.len() {
        return Err("snapshot section directory is outside the envelope");
    }

    for entry in bytes[directory_offset..directory_end].chunks_exact(DIRECTORY_ENTRY_SIZE) {
        let schema_id = u64::from_le_bytes(
            entry
                .get(..8)
                .ok_or("snapshot section ID is truncated")?
                .try_into()
                .map_err(|_| "snapshot section ID is malformed")?,
        );
        if schema_id == u64::from(target) {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Run the wet nine-chunk scenario against a durable group-commit bundle.
///
/// The bundle uses the repository target filesystem; the report names it for
/// inspection and the caller removes that owned directory after capturing evidence.
fn history_bundle_workload(
    ticks_requested: u64,
    scenario: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    static NEXT_BUNDLE: AtomicU64 = AtomicU64::new(0);
    let bundle_path = std::env::current_dir()?.join("target").join(format!(
        "causafera-hydrology-history-1000-{}-{}",
        std::process::id(),
        NEXT_BUNDLE.fetch_add(1, Ordering::Relaxed)
    ));
    let mut stdout = std::io::stdout().lock();
    writeln!(
        stdout,
        "{{\"event\":\"start\",\"scenario\":\"{scenario}\",\"ticks_requested\":{ticks_requested},\"active_chunks\":9,\"bundle_path\":{:?}}}",
        bundle_path.display().to_string(),
    )?;
    stdout.flush()?;

    let mut runtime = Runtime::create_bundle(
        config(1, ActiveChunkShape::Area, true, true),
        &bundle_path,
        causafera_runtime::BundleOptions::default(),
    )?;
    let started = Instant::now();
    let mut conservation_checks = 0_u64;
    let mut last_progress = Instant::now();
    for tick in 1..=ticks_requested {
        let snapshot = runtime.tick()?;
        if snapshot.hydrology.latest_residual != 0 {
            return Err(format!(
                "tick {tick} has nonzero hydrology residual {}",
                snapshot.hydrology.latest_residual
            )
            .into());
        }
        conservation_checks += 1;

        if tick.is_multiple_of(HISTORY_PROGRESS_INTERVAL) || tick == ticks_requested {
            let status = runtime.history_status()?;
            if status.hot_bytes > status.hot_budget {
                return Err(format!(
                    "tick {tick} exceeded the hot history budget: {} > {} bytes",
                    status.hot_bytes, status.hot_budget
                )
                .into());
            }
            writeln!(
                stdout,
                "{{\"event\":\"progress\",\"tick\":{tick},\"ticks_requested\":{ticks_requested},\"interval_ms\":{:.3},\"hot_bytes\":{},\"hot_budget_bytes\":{}}}",
                last_progress.elapsed().as_secs_f64() * 1000.0,
                status.hot_bytes,
                status.hot_budget,
            )?;
            stdout.flush()?;
            last_progress = Instant::now();
        }
    }
    let elapsed_ms = started.elapsed().as_secs_f64() * 1000.0;
    if runtime.current_time().raw() != ticks_requested {
        return Err(format!(
            "run ended at tick {}, expected {ticks_requested}",
            runtime.current_time().raw()
        )
        .into());
    }

    let final_summary = runtime.snapshot()?;
    if final_summary.active_chunk_count != 9 || final_summary.hydrology.active_chunk_count != 9 {
        return Err("the final state is not the configured wet nine-chunk scenario".into());
    }
    let water_total = final_summary.hydrology.total_surface
        + final_summary.hydrology.total_soil
        + final_summary.hydrology.total_groundwater
        + final_summary.hydrology.total_conveyance;
    if water_total == 0 {
        return Err("the wet scenario ended with no water in its reservoirs".into());
    }

    let hydrology = runtime.hydrology_state();
    if !hydrology.enabled
        || !hydrology.forcing.iter().any(|record| {
            record.forcing_id() == 1
                && record.scheduled_tick() == 3
                && record.applied_at().is_some()
        })
    {
        return Err("the configured wet forcing was not applied".into());
    }
    assert_conserved(&hydrology, scenario);
    let retained_conservation_batches = hydrology.retained_batches.len();
    let before = runtime.snapshot()?;
    let before_status = runtime.history_status()?;
    if before_status.hot_bytes > before_status.hot_budget {
        return Err("final hot history exceeds its configured budget".into());
    }

    let save_started = Instant::now();
    let saved = runtime.save_bundle()?;
    writeln!(
        stdout,
        "{{\"event\":\"saved\",\"elapsed_ms\":{:.3}}}",
        save_started.elapsed().as_secs_f64() * 1000.0
    )?;
    stdout.flush()?;
    let saved_status = runtime.history_status()?;
    if saved_status.logical_generation != saved_status.durable_generation {
        return Err("save_bundle did not force the final logical generation".into());
    }
    let bounded_snapshot = std::fs::read(&saved.snapshot_path)?;
    let snapshot_bytes = bounded_snapshot.len();
    let contains_legacy_trace_section =
        bounded_snapshot_has_section(&bounded_snapshot, causafera_runtime::SECTION_CAUSAL_TRACES)?;
    if contains_legacy_trace_section {
        return Err("the bounded snapshot contains section 0x000A".into());
    }
    runtime.shutdown()?;

    let reopen_started = Instant::now();
    let (resumed, recovery) = Runtime::open_bundle(&bundle_path)?;
    writeln!(
        stdout,
        "{{\"event\":\"reopened\",\"elapsed_ms\":{:.3}}}",
        reopen_started.elapsed().as_secs_f64() * 1000.0
    )?;
    stdout.flush()?;
    let after = resumed.snapshot()?;
    let after_status = resumed.history_status()?;
    if after_status.hot_bytes > after_status.hot_budget {
        return Err("reopened hot history exceeds its configured budget".into());
    }
    if recovery.lost.is_some()
        || recovery.completed_time.raw() != ticks_requested
        || recovery.replayed_ticks != 0
        || before.physical_state_digest != after.physical_state_digest
        || before.history_digest != after.history_digest
        || before.canonical_state != after.canonical_state
    {
        return Err("save/reopen did not preserve the final tick and all runtime digests".into());
    }
    resumed.shutdown()?;

    let path_json = format!("{:?}", bundle_path.display().to_string());
    let scenario_json = format!("{:?}", scenario);
    writeln!(
        stdout,
        "{{\"event\":\"report\",\"scenario\":{scenario_json},\"world_seed\":{WORLD_SEED},\"active_chunk_radius\":1,\"active_chunk_shape\":\"area\",\"active_chunks\":9,\"wet_forcing_enabled\":true,\"durability_mode\":\"group_commit\",\"ticks_requested\":{ticks_requested},\"ticks_committed\":{},\"elapsed_ms\":{elapsed_ms:.3},\"conservation_checks\":{conservation_checks},\"retained_conservation_batches\":{retained_conservation_batches},\"final_latest_residual\":{},\"water_total\":{water_total},\"logical_generation\":{},\"durable_generation\":{},\"hot_history_bytes\":{},\"hot_history_budget_bytes\":{},\"reopened_hot_history_bytes\":{},\"reopened_hot_history_budget_bytes\":{},\"bounded_snapshot_bytes\":{snapshot_bytes},\"bounded_snapshot_has_0x000A\":false,\"recovery_replayed_ticks\":{},\"recovery_lost_ticks\":0,\"save_reopen_digests_equal\":true,\"physical_digest\":\"{}\",\"history_digest\":\"{}\",\"experiment_digest\":\"{}\",\"bundle_path\":{path_json}}}",
        after.time.raw(),
        after.hydrology.latest_residual,
        saved_status.logical_generation,
        saved_status.durable_generation,
        saved_status.hot_bytes,
        saved_status.hot_budget,
        after_status.hot_bytes,
        after_status.hot_budget,
        recovery.replayed_ticks,
        hex(&after.physical_state_digest.bytes()),
        hex(&after.history_digest.bytes()),
        hex(&after.canonical_state.bytes()),
    )?;
    stdout.flush()?;
    Ok(())
}

fn history_1000_workload() -> Result<(), Box<dyn std::error::Error>> {
    history_bundle_workload(HISTORY_RUN_TICKS, "history-1000")
}

fn history_smoke_workload() -> Result<(), Box<dyn std::error::Error>> {
    let ticks = std::env::var("CAUSAFERA_HISTORY_SMOKE_TICKS")
        .ok()
        .map(|value| value.parse::<u64>())
        .transpose()?
        .unwrap_or(3);
    if !(1..=100).contains(&ticks) {
        return Err("CAUSAFERA_HISTORY_SMOKE_TICKS must be in 1..=100".into());
    }
    history_bundle_workload(ticks, "history-smoke")
}

fn default_workloads() {
    println!(
        "{:<34} {:>9} {:>7} {:>6} {:>8} {:>7} {:>7} {:>8} {:>9} {:>9} {:>8} {:>9} {:>9}  levels",
        "workload",
        "ticks",
        "cells",
        "edges",
        "vertical",
        "faces",
        "bounds",
        "receipts",
        "mean ms",
        "median",
        "stddev",
        "min",
        "max",
    );

    workload("1. one chunk, vertical only", 24, || {
        config(0, ActiveChunkShape::Line, false, true)
    });
    workload("2. three line chunks, seams", 24, || {
        config(1, ActiveChunkShape::Line, true, true)
    });
    workload("3. nine chunks, full routing", 24, || {
        config(1, ActiveChunkShape::Area, true, true)
    });
    snapshot_workload();
    // Preserve the historical one-chunk 100-tick timing point. The separate
    // history-1000 mode exercises the current durable nine-chunk path.
    workload("6. run length 100", 100, || {
        config(0, ActiveChunkShape::Line, true, true)
    });
    resolution_workload();
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    match args.as_slice() {
        [] => default_workloads(),
        [mode] if mode == "history-1000" => history_1000_workload()?,
        [mode] if mode == "history-smoke" => history_smoke_workload()?,
        [mode] if mode == "--help" || mode == "-h" => {
            println!(
                "Usage: hydrology_bench [history-1000|history-smoke]\n\
                 Default runs the short hydrology workloads.\n\
                 history-1000 runs the wet nine-chunk durable-bundle scenario.\n\
                 history-smoke runs the same bundle and validation flow for 3 ticks.\n\
                 Set CAUSAFERA_HISTORY_SMOKE_TICKS=1..100 to change its length."
            );
        }
        [mode] => return Err(format!("unknown benchmark mode {mode:?}; use --help").into()),
        _ => return Err("expected at most one benchmark mode; use --help".into()),
    }
    Ok(())
}
