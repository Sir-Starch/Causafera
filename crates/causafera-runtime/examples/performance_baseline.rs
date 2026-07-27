//! Wave 1 harness for `TODO-PERF-001` / `plans/performance-baseline-and-digest-cost.md`.
//!
//! Replaces the deleted scratch probes that investigation used with a checked-in, reproducible
//! tool, per INV-018. Three modes, run individually or all together (default, no args):
//!
//! ```text
//! cargo run --release -p causafera-runtime --example performance_baseline -- boundary-sweep
//! cargo run --release -p causafera-runtime --example performance_baseline -- worst-case-contact
//! cargo run --release -p causafera-runtime --example performance_baseline -- digest-cost
//! ```
//!
//! `--worker <case_index> <pass>` is an internal mode `digest-cost` spawns as a subprocess per
//! (case, repetition) pair, for RSS process isolation; it is not meant to be invoked directly.

use std::process::{Command, ExitCode, Stdio};
use std::time::Instant;

use causafera_cognition::{MAX_SCENE_CUES, SceneUpdateError};
use causafera_runtime::{
    ActiveChunkShape, Runtime, RuntimeConfig, RuntimeError, measure_digest_cost,
};

/// Named, not ad hoc: `plans/performance-baseline-and-digest-cost.md` Wave 1 commits to this
/// exact repetition count.
const REPETITIONS: usize = 20;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    if args.len() >= 4 && args[1] == "--worker" {
        return run_worker(&args[2], &args[3]);
    }
    print_metadata();
    match args.get(1).map(String::as_str) {
        Some("boundary-sweep") => {
            run_boundary_sweep();
            ExitCode::SUCCESS
        }
        Some("worst-case-contact") => {
            run_worst_case_contact();
            ExitCode::SUCCESS
        }
        Some("digest-cost") => run_digest_cost_sweep(),
        Some(other) => {
            eprintln!("unknown mode: {other}");
            eprintln!("expected one of: boundary-sweep, worst-case-contact, digest-cost");
            ExitCode::FAILURE
        }
        None => {
            run_boundary_sweep();
            run_worst_case_contact();
            run_digest_cost_sweep()
        }
    }
}

fn print_metadata() {
    let cores = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(0);
    let rustc_version = Command::new("rustc")
        .arg("--version")
        .output()
        .ok()
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
        .unwrap_or_else(|| "unknown".to_string());
    let profile = if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    };
    println!("== hardware/toolchain metadata ==");
    println!("logical_cores={cores} rustc={rustc_version} profile={profile}");
    println!(
        "local-environment measurement only; not the reference hardware in docs/performance/benchmarks.md"
    );
}

// ---------------------------------------------------------------------------------------------
// Mode 1: exhaustive actor_count/sensor_count boundary sweep.
//
// Locates the exact `actor_count` at which each `sensor_count` first fails against
// `MAX_SCENE_CUES`, checking every integer in range rather than sparse samples (closes the
// off-by-one gap Finding 1 left open in the plan). Fixed at `active_chunk_radius=0` so
// `contacted_surface_count <= 1`, isolating the actor/sensor terms per the plan's Wave 1 scope.
// ---------------------------------------------------------------------------------------------

fn run_boundary_sweep() {
    println!("\n== exhaustive actor_count/sensor_count boundary sweep (active_chunk_radius=0) ==");
    println!(
        "{:>12} {:>10} {:>14} {:>10}",
        "sensor_count", "last_ok", "first_fail_at", "cues"
    );
    for sensor_count in [1u8, 2, 4, 8, 16] {
        let mut last_ok = 0u8;
        let mut outcome = None;
        for actor_count in 1u8..=128 {
            match probe_actor_sensor_boundary(actor_count, sensor_count) {
                BoundaryProbe::Ok => last_ok = actor_count,
                BoundaryProbe::CueCapExceeded(count) => {
                    outcome = Some((actor_count, count));
                    break;
                }
                BoundaryProbe::UnrelatedFailure(message) => {
                    println!(
                        "  sensor_count={sensor_count} actor_count={actor_count}: non-cue-cap failure ({message}), stopping this sensor_count's sweep"
                    );
                    break;
                }
            }
        }
        match outcome {
            Some((actor_count, count)) => {
                println!("{sensor_count:>12} {last_ok:>10} {actor_count:>14} {count:>10}")
            }
            None => println!(
                "{sensor_count:>12} {last_ok:>10} {:>14} {:>10}",
                "none<=128", "-"
            ),
        }
    }
    println!(
        "MAX_SCENE_CUES={MAX_SCENE_CUES} (causafera_cognition::scene). Boundary is per-actor cue \
count against this cap; the exact formula relating actor_count/sensor_count/surface-contact to the \
cue count is not committed by the plan (Finding 1) — this sweep reports the measured boundary \
directly rather than a derived formula."
    );
}

enum BoundaryProbe {
    Ok,
    CueCapExceeded(usize),
    UnrelatedFailure(String),
}

fn probe_actor_sensor_boundary(actor_count: u8, sensor_count: u8) -> BoundaryProbe {
    let mut config = RuntimeConfig::new(7);
    config.chunk_extent = 3;
    config.active_chunk_radius = 0;
    config.actor_count = actor_count;
    config.sensor_count = sensor_count;
    config.bootstrap_population = 512;
    config.mana_parameters.effect_threshold = 1;
    config.mana_parameters.effect_hysteresis = 0;
    let mut runtime = match Runtime::new(config) {
        Ok(runtime) => runtime,
        Err(error) => return BoundaryProbe::UnrelatedFailure(error.to_string()),
    };
    match runtime.run_ticks(8) {
        Ok(_) => BoundaryProbe::Ok,
        Err(RuntimeError::ActorCognition(SceneUpdateError::TooManyCues { count })) => {
            BoundaryProbe::CueCapExceeded(count)
        }
        Err(error) => BoundaryProbe::UnrelatedFailure(error.to_string()),
    }
}

// ---------------------------------------------------------------------------------------------
// Mode 2: empirical worst-case surface-contact measurement.
//
// Not a formal proof that `contacted_surface_count` reaches `active_chunk_count` — an empirical
// measurement of how close a generously long run gets, per the plan's Non-goals/Risks: this
// investigation does not commit to a closed-form bound, so Wave 2 needs real data on how far
// contact spreads, not an assumption.
// ---------------------------------------------------------------------------------------------

fn run_worst_case_contact() {
    println!(
        "\n== empirical worst-case surface-contact measurement (768 ticks; not a formal proof) =="
    );
    println!(
        "{:>8} {:>8} {:>14} {:>20} {:>10}",
        "radius", "shape", "active_chunks", "contacted_surfaces", "coverage"
    );
    let cases: &[(u8, ActiveChunkShape, &str)] = &[
        (0, ActiveChunkShape::Line, "line"),
        (1, ActiveChunkShape::Area, "area"),
        (2, ActiveChunkShape::Area, "area"),
        (4, ActiveChunkShape::Area, "area"),
    ];
    for &(radius, shape, label) in cases {
        let mut config = RuntimeConfig::new(7);
        config.chunk_extent = 3;
        config.active_chunk_radius = radius;
        config.active_chunk_shape = shape;
        config.actor_count = 8;
        config.sensor_count = 2;
        config.bootstrap_population = 512;
        config.mana_parameters.effect_threshold = 1;
        config.mana_parameters.effect_hysteresis = 0;
        let mut runtime = match Runtime::new(config) {
            Ok(runtime) => runtime,
            Err(error) => {
                println!("  radius={radius} {label}: rejected at construction: {error}");
                continue;
            }
        };
        let snapshot = match runtime.run_ticks(768) {
            Ok(snapshot) => snapshot,
            Err(error) => {
                println!("  radius={radius} {label}: failed during run: {error}");
                continue;
            }
        };
        let exported = match runtime.export_snapshot() {
            Ok(exported) => exported,
            Err(error) => {
                println!("  radius={radius} {label}: snapshot export failed: {error}");
                continue;
            }
        };
        let contacted = exported
            .material_surfaces
            .records
            .iter()
            .filter(|record| record.surface.contact_count > 0)
            .count();
        let active_chunks = snapshot.active_chunk_count;
        let coverage = if active_chunks == 0 {
            0.0
        } else {
            100.0 * contacted as f64 / f64::from(active_chunks)
        };
        println!("{radius:>8} {label:>8} {active_chunks:>14} {contacted:>20} {coverage:>9.1}%");
    }
}

// ---------------------------------------------------------------------------------------------
// Mode 3: digest-cost sweep with N=20 repetitions, cyclically-rotated case order, one subprocess
// per (case, repetition) pair for RSS isolation.
// ---------------------------------------------------------------------------------------------

struct DigestCostCase {
    name: &'static str,
    chunk_extent: u8,
    active_chunk_radius: u8,
    active_chunk_shape: ActiveChunkShape,
    warmup_batches_of_64: u64,
    measured_ticks: u64,
}

const DIGEST_COST_CASES: &[DigestCostCase] = &[
    DigestCostCase {
        name: "baseline_batch0",
        chunk_extent: 3,
        active_chunk_radius: 0,
        active_chunk_shape: ActiveChunkShape::Line,
        warmup_batches_of_64: 0,
        measured_ticks: 64,
    },
    DigestCostCase {
        name: "baseline_batch7",
        chunk_extent: 3,
        active_chunk_radius: 0,
        active_chunk_shape: ActiveChunkShape::Line,
        warmup_batches_of_64: 7,
        measured_ticks: 64,
    },
    DigestCostCase {
        name: "chunk_extent_8",
        chunk_extent: 8,
        active_chunk_radius: 0,
        active_chunk_shape: ActiveChunkShape::Line,
        warmup_batches_of_64: 0,
        measured_ticks: 64,
    },
    DigestCostCase {
        name: "chunk_extent_16",
        chunk_extent: 16,
        active_chunk_radius: 0,
        active_chunk_shape: ActiveChunkShape::Line,
        warmup_batches_of_64: 0,
        measured_ticks: 64,
    },
    DigestCostCase {
        name: "radius_1_area",
        chunk_extent: 3,
        active_chunk_radius: 1,
        active_chunk_shape: ActiveChunkShape::Area,
        warmup_batches_of_64: 0,
        measured_ticks: 64,
    },
    DigestCostCase {
        name: "radius_4_area",
        chunk_extent: 3,
        active_chunk_radius: 4,
        active_chunk_shape: ActiveChunkShape::Area,
        warmup_batches_of_64: 0,
        measured_ticks: 64,
    },
];

fn run_digest_cost_sweep() -> ExitCode {
    let case_count = DIGEST_COST_CASES.len();
    let mut tick_samples: Vec<Vec<u128>> = vec![Vec::new(); case_count];
    let mut physical_samples: Vec<Vec<u128>> = vec![Vec::new(); case_count];
    let mut history_samples: Vec<Vec<u128>> = vec![Vec::new(); case_count];
    let mut rss_samples: Vec<Vec<u128>> = vec![Vec::new(); case_count];
    let mut trace_event_samples: Vec<Vec<u128>> = vec![Vec::new(); case_count];

    let exe = match std::env::current_exe() {
        Ok(exe) => exe,
        Err(error) => {
            eprintln!("could not resolve current executable path: {error}");
            return ExitCode::FAILURE;
        }
    };

    println!(
        "\n== digest-cost sweep: N={REPETITIONS} repetitions per case, cyclically-rotated case \
order, one subprocess per (case, repetition) pair =="
    );
    for pass in 0..REPETITIONS {
        // Cyclic rotation, not a fixed repeated order: pass `p` starts at offset `p mod k`, so no
        // case runs first in every pass. A plain repeated order would still bias whichever case
        // ran first toward any first-in-pass effect (cache warmth, thermal throttling).
        let offset = pass % case_count;
        for step in 0..case_count {
            let case_index = (offset + step) % case_count;
            let output = Command::new(&exe)
                .arg("--worker")
                .arg(case_index.to_string())
                .arg(pass.to_string())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output();
            let output = match output {
                Ok(output) => output,
                Err(error) => {
                    eprintln!("failed to spawn worker subprocess: {error}");
                    return ExitCode::FAILURE;
                }
            };
            let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
            match parse_worker_result(&stdout) {
                Some(result) => {
                    tick_samples[case_index].push(result.total_tick_ns);
                    physical_samples[case_index].push(result.physical_state_digest_ns);
                    history_samples[case_index].push(result.history_digest_ns);
                    rss_samples[case_index].push(u128::from(result.peak_rss_kib));
                    trace_event_samples[case_index].push(u128::from(result.trace_events));
                }
                None => {
                    eprintln!(
                        "worker for case={case_index} pass={pass} produced no parseable RESULT \
line; stdout={stdout:?} stderr={:?}",
                        String::from_utf8_lossy(&output.stderr)
                    );
                    return ExitCode::FAILURE;
                }
            }
        }
    }

    println!(
        "{:<18} {:>12} {:>12} {:>12} {:>14} {:>14} {:>12} {:>10}",
        "case",
        "tick_mean_us",
        "tick_median_us",
        "tick_stddev_us",
        "phys_dig_mean_us",
        "hist_dig_mean_us",
        "rss_kib_mean",
        "trace_evts"
    );
    for (index, case) in DIGEST_COST_CASES.iter().enumerate() {
        let tick_stats = Stats::from_samples(&tick_samples[index]);
        let physical_stats = Stats::from_samples(&physical_samples[index]);
        let history_stats = Stats::from_samples(&history_samples[index]);
        let rss_stats = Stats::from_samples(&rss_samples[index]);
        let trace_stats = Stats::from_samples(&trace_event_samples[index]);
        println!(
            "{:<18} {:>12.1} {:>12.1} {:>12.1} {:>14.1} {:>14.1} {:>12.0} {:>10.0}",
            case.name,
            tick_stats.mean / 1000.0,
            tick_stats.median / 1000.0,
            tick_stats.stddev / 1000.0,
            physical_stats.mean / 1000.0,
            history_stats.mean / 1000.0,
            rss_stats.mean,
            trace_stats.mean,
        );
        println!(
            "  raw tick_ns samples ({REPETITIONS}): {:?}",
            tick_samples[index]
        );
    }
    ExitCode::SUCCESS
}

struct Stats {
    mean: f64,
    median: f64,
    stddev: f64,
}

impl Stats {
    fn from_samples(samples: &[u128]) -> Self {
        if samples.is_empty() {
            return Self {
                mean: 0.0,
                median: 0.0,
                stddev: 0.0,
            };
        }
        let count = samples.len() as f64;
        let mean = samples.iter().sum::<u128>() as f64 / count;
        let mut sorted = samples.to_vec();
        sorted.sort_unstable();
        let median = if sorted.len().is_multiple_of(2) {
            let mid = sorted.len() / 2;
            (sorted[mid - 1] as f64 + sorted[mid] as f64) / 2.0
        } else {
            sorted[sorted.len() / 2] as f64
        };
        let variance = samples
            .iter()
            .map(|&value| {
                let delta = value as f64 - mean;
                delta * delta
            })
            .sum::<f64>()
            / count;
        Self {
            mean,
            median,
            stddev: variance.sqrt(),
        }
    }
}

struct WorkerResult {
    total_tick_ns: u128,
    physical_state_digest_ns: u128,
    history_digest_ns: u128,
    peak_rss_kib: u64,
    trace_events: u64,
}

fn parse_worker_result(stdout: &str) -> Option<WorkerResult> {
    let line = stdout.lines().find(|line| line.starts_with("RESULT "))?;
    let mut total_tick_ns = None;
    let mut physical_state_digest_ns = None;
    let mut history_digest_ns = None;
    let mut peak_rss_kib = None;
    let mut trace_events = None;
    for token in line.split_whitespace().skip(1) {
        let (key, value) = token.split_once('=')?;
        match key {
            "total_tick_ns" => total_tick_ns = value.parse().ok(),
            "physical_state_digest_ns" => physical_state_digest_ns = value.parse().ok(),
            "history_digest_ns" => history_digest_ns = value.parse().ok(),
            "peak_rss_kib" => peak_rss_kib = value.parse().ok(),
            "trace_events" => trace_events = value.parse().ok(),
            _ => {}
        }
    }
    Some(WorkerResult {
        total_tick_ns: total_tick_ns?,
        physical_state_digest_ns: physical_state_digest_ns?,
        history_digest_ns: history_digest_ns?,
        peak_rss_kib: peak_rss_kib?,
        trace_events: trace_events?,
    })
}

/// Runs exactly one `(case, repetition)` pair in this process and exits. Spawned by
/// `run_digest_cost_sweep` for RSS process isolation — each worker's peak RSS reflects only its
/// own case, unlike the shared-process RSS Finding "existing benchmark-infrastructure gaps"
/// documented in the plan.
fn run_worker(case_index: &str, pass: &str) -> ExitCode {
    let case_index: usize = match case_index.parse() {
        Ok(value) => value,
        Err(_) => {
            eprintln!("invalid case index: {case_index}");
            return ExitCode::FAILURE;
        }
    };
    let Some(case) = DIGEST_COST_CASES.get(case_index) else {
        eprintln!("case index out of range: {case_index}");
        return ExitCode::FAILURE;
    };
    let _pass = pass; // logged by the parent; not otherwise needed by the worker.

    let mut config = RuntimeConfig::new(7);
    config.chunk_extent = case.chunk_extent;
    config.active_chunk_radius = case.active_chunk_radius;
    config.active_chunk_shape = case.active_chunk_shape;
    config.actor_count = 8;
    config.sensor_count = 2;
    config.bootstrap_population = 512;
    config.mana_parameters.effect_threshold = 1;
    config.mana_parameters.effect_hysteresis = 0;

    let mut runtime = match Runtime::new(config) {
        Ok(runtime) => runtime,
        Err(error) => {
            println!("ERROR runtime_new {error}");
            return ExitCode::FAILURE;
        }
    };

    let warmup_ticks = case.warmup_batches_of_64 * 64;
    if warmup_ticks > 0 && runtime.run_ticks(warmup_ticks).is_err() {
        println!("ERROR warmup_failed");
        return ExitCode::FAILURE;
    }

    let started = Instant::now();
    for _ in 0..case.measured_ticks {
        if runtime.tick().is_err() {
            println!("ERROR tick_failed");
            return ExitCode::FAILURE;
        }
    }
    let total_tick_ns = started.elapsed().as_nanos();

    // A single post-loop digest-cost sample: `physical_state_digest`/`history_digest` are
    // read-only and side-effect-free, so this measures "cost of one call at the batch's final
    // state" without inflating `total_tick_ns` above (matches the plan's Finding 2 methodology,
    // which measured tick time and digest time independently rather than accumulating digest
    // calls inside the timed tick loop).
    let digest_sample = match measure_digest_cost(&runtime) {
        Ok(sample) => sample,
        Err(error) => {
            println!("ERROR digest_cost {error}");
            return ExitCode::FAILURE;
        }
    };

    let trace_events = match runtime.export_snapshot() {
        Ok(exported) => exported.traces.events.len() as u64,
        Err(error) => {
            println!("ERROR export_snapshot {error}");
            return ExitCode::FAILURE;
        }
    };

    let peak_rss_kib = read_rss_kib().unwrap_or(0);

    println!(
        "RESULT case={} total_tick_ns={} physical_state_digest_ns={} history_digest_ns={} peak_rss_kib={} trace_events={}",
        case.name,
        total_tick_ns,
        digest_sample.physical_state_digest_ns,
        digest_sample.history_digest_ns,
        peak_rss_kib,
        trace_events,
    );
    ExitCode::SUCCESS
}

fn read_rss_kib() -> Option<u64> {
    #[cfg(target_os = "linux")]
    {
        std::fs::read_to_string("/proc/self/status")
            .ok()?
            .lines()
            .find_map(|line| {
                line.strip_prefix("VmHWM:")
                    .and_then(|value| value.split_whitespace().next())
                    .and_then(|value| value.parse::<u64>().ok())
            })
    }
    #[cfg(not(target_os = "linux"))]
    {
        None
    }
}
