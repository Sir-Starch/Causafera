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
use std::num::{NonZeroU32, NonZeroU64};
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

/// How long a session can run before the snapshot cap refuses a tick.
///
/// The most useful number this harness produces, and the reason the plan's
/// 1,000- and 10,000-tick sweeps are not reachable: what bounds a
/// hydrology-enabled session is not its per-tick cost but the causal trace and
/// receipt history it accumulates, which the 256 MiB export cap eventually
/// refuses to hold. The tick it engages at is exact and deterministic — the same
/// seed reaches the same tick every time — so this is measured once per
/// configuration rather than ten times. Repeating a value with no variance
/// spends minutes to report the same number.
fn ceiling_workload() {
    println!();
    println!("Ceiling: ticks committed before the 256 MiB export cap refuses one");
    println!(
        "{:<24} {:>7} {:>10} {:>12} {:>12}",
        "configuration", "cells", "ticks", "wall s", "ms/tick"
    );
    for (label, radius, shape) in [
        ("one chunk", 0_u8, ActiveChunkShape::Line),
        ("three line chunks", 1, ActiveChunkShape::Line),
        ("nine chunks", 1, ActiveChunkShape::Area),
    ] {
        let mut runtime =
            Runtime::new(config(radius, shape, true, true)).expect("the runtime must bootstrap");
        let started = Instant::now();
        // Far past any reachable ceiling: the loop stops when the cap does.
        let reached = advance(&mut runtime, 100_000);
        let seconds = started.elapsed().as_secs_f64();
        let state = runtime.hydrology_state();
        assert_conserved(&state, label);
        println!(
            "{label:<24} {:>7} {reached:>10} {seconds:>12.1} {:>12.3}",
            state.fields.cell_count(),
            seconds * 1000.0 / reached.max(1) as f64,
        );
    }
}

fn main() {
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
    // Workload 6 asks for 100, 1,000, and 10,000 ticks. Only the first is
    // reachable: the export cap refuses a tick well before the other two, and
    // the ceiling section below reports exactly where. Running the sweep on one
    // chunk is what gives 100 ticks any headroom at all.
    workload("6. run length 100", 100, || {
        config(0, ActiveChunkShape::Line, true, true)
    });
    resolution_workload();
    ceiling_workload();
}
