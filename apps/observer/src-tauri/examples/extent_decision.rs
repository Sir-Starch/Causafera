//! Development-only validation for the `chunk_extent` default decision
//! (`TODO-MANA-004`).
//!
//! This tool was rewritten after `TODO-RUNTIME-002`, because that work changed
//! what `chunk_extent` means. It used to be a pure discretisation parameter: one
//! world, sampled on a coarser or finer lattice, so a finer lattice was simply a
//! better approximation and "total mana converges from extent 12" was a
//! meaningful convergence claim. The standing terrain carrier presents itself at
//! the mana lattice's own resolution — one sample per plan-view column — so the
//! extent now also sets how finely the field reads the ground. A finer lattice
//! is not the same physics at higher resolution; it is more physical input.
//! Comparing a candidate field against a finer reference field therefore
//! conflates discretisation error with a different carrier reading, and the old
//! convergence table has no interpretation. It is gone.
//!
//! What replaces it are three questions that do have answers:
//!
//!   1. how much of the terrain that exists does the field at this extent get to
//!      respond to at all — measured against extent 32, which is one column per
//!      terrain cell and therefore the exact reading, with no runtime involved;
//!   2. can a world at this extent still be told apart from another world —
//!      after `TODO-RUNTIME-002` the seed reaches the simulation, so a lattice
//!      too coarse to carry the difference shows up as seeds collapsing back
//!      onto shared behaviour;
//!   3. what it costs, which is now dominated by one committed causal event per
//!      changed mana cell per tick rather than by the arithmetic.
//!
//! Build in release; debug timings and run lengths make this useless.
//!
//! ```text
//! cargo run --release -p causafera-observer --example extent_decision
//! ```

use std::collections::{BTreeMap, BTreeSet};
use std::time::Instant;

use causafera_runtime::{
    Runtime, RuntimeConfig, RuntimeSnapshotData, TerrainCarrierAdapter, deterministic_terrain_chunk,
};
use causafera_types::{CHUNK_SIZE, ChartChunkCoord, ChunkCoord, SpatialChartId, TraceId};

const SEEDS: [u64; 6] = [7, 11, 23, 41, 59, 97];
const CANDIDATES: [u8; 5] = [3, 4, 6, 8, 12];
/// One column per terrain cell, so the carrier reads every cell exactly. This is
/// the reference for the reading, not a candidate: the mana volume at this
/// extent is 32768 cells per chunk.
const EXACT: u8 = CHUNK_SIZE;
const TICKS: u64 = 192;
/// The production gate, restated here only so the report can print what the
/// intensities are being compared against. `RuntimeConfig::new` remains the
/// authority; a mismatch would show up as the ratio column disagreeing with the
/// gate crossings beside it.
const THRESHOLD: i64 = 6_144;
const HYSTERESIS: i64 = 1_536;

fn chunk() -> ChartChunkCoord {
    ChartChunkCoord::new(SpatialChartId::new(1), ChunkCoord::new(0, 0, 0))
}

/// What the carrier reads off one chunk of terrain at one lattice.
struct Reading {
    /// Per terrain cell, the structure of the column it was averaged into.
    /// Length is always `CHUNK_SIZE²`, whatever the extent, so two readings are
    /// directly comparable.
    per_cell: Vec<f64>,
    emitting_columns: usize,
    total_columns: usize,
    fingerprints: usize,
}

fn read_terrain(seed: u64, extent: u8) -> Reading {
    let terrain = deterministic_terrain_chunk(seed, chunk(), TraceId::new(1));
    let adapter = TerrainCarrierAdapter::new(chunk(), terrain, extent, &BTreeMap::new());
    let columns = adapter.columns();
    let side = usize::from(CHUNK_SIZE);
    let lattice = usize::from(extent);

    // The carrier's own cell-to-column mapping, so this measures the projection
    // that runs rather than an idealisation of it.
    let per_cell = (0..side * side)
        .map(|index| {
            let x = (index % side) * lattice / side;
            let y = (index / side) * lattice / side;
            f64::from(columns[y * lattice + x].structure)
        })
        .collect();

    Reading {
        per_cell,
        emitting_columns: columns.iter().filter(|column| column.structure > 0).count(),
        total_columns: columns.len(),
        fingerprints: columns
            .iter()
            .filter(|column| column.structure > 0)
            .map(|column| column.pattern())
            .collect::<BTreeSet<_>>()
            .len(),
    }
}

/// The share of the terrain's own structural variation that survives projection
/// onto this lattice.
///
/// Averaging a block of terrain cells into one column keeps the variation
/// between blocks and destroys the variation inside them, which is the standard
/// variance decomposition. The ratio is what fraction of the ground's structure
/// the field can respond to: 1.0 at the exact reading, and it can only fall as
/// the lattice coarsens.
fn variation_retained(candidate: &Reading, exact: &Reading) -> f64 {
    let count = exact.per_cell.len() as f64;
    let mean = exact.per_cell.iter().sum::<f64>() / count;
    let total: f64 = exact
        .per_cell
        .iter()
        .map(|value| (value - mean).powi(2))
        .sum();
    let between: f64 = candidate
        .per_cell
        .iter()
        .map(|value| (value - mean).powi(2))
        .sum();
    if total <= 0.0 {
        return 1.0;
    }
    between / total
}

/// Everything a lattice change could move that is not a resolution detail.
#[derive(PartialEq, Eq, PartialOrd, Ord)]
struct Behaviour {
    gate_crossings: u64,
    gate_transitions: usize,
    surface_conditions: i128,
    actions_committed: u64,
    population: u64,
}

struct Run {
    total_mana: i128,
    behaviour: Behaviour,
    physical_digest: [u8; 32],
    milliseconds_per_tick: f64,
    changed_cells_per_tick: f64,
    causal_events: u64,
    /// Where the field sits relative to the gate it drives. A field far above the
    /// threshold has stopped being able to move the gate, whatever its
    /// resolution.
    mean_intensity: f64,
    cells_over_threshold: f64,
}

fn run(seed: u64, extent: u8) -> Run {
    let mut config = RuntimeConfig::new(seed);
    config.chunk_extent = extent;
    config.actor_count = 8;
    config.sensor_count = 2;
    config.bootstrap_population = 512;

    let threshold = config.mana_parameters.effect_threshold;
    let mut runtime = Runtime::new(config).expect("runtime");
    let started = Instant::now();
    let summary = runtime.run_ticks(TICKS).expect("ticks");
    let elapsed = started.elapsed();
    let data = runtime.export_snapshot().expect("snapshot");

    Run {
        total_mana: total_mana(&data),
        behaviour: Behaviour {
            gate_crossings: summary.mana_physical_effects,
            gate_transitions: data.material_surfaces.gate_transitions.len(),
            surface_conditions: data
                .material_surfaces
                .records
                .iter()
                .map(|record| i128::from(record.surface.condition))
                .sum(),
            actions_committed: summary.actor_actions_committed,
            population: summary.population_total,
        },
        physical_digest: summary.physical_state_digest.bytes(),
        milliseconds_per_tick: elapsed.as_secs_f64() * 1_000.0 / TICKS as f64,
        changed_cells_per_tick: summary.mana_cell_changes as f64 / TICKS as f64,
        causal_events: summary.causal_trace_count,
        mean_intensity: mean_intensity(&data),
        cells_over_threshold: over_threshold(&data, threshold),
    }
}

/// Mean intensity over the cells holding any mana at all, so a finer lattice is
/// not credited with dilution across cells the field never reached.
fn mean_intensity(data: &RuntimeSnapshotData) -> f64 {
    let live: Vec<i64> = data
        .mana
        .fields
        .iter()
        .flat_map(|field| field.intensity.iter().copied())
        .filter(|value| *value != 0)
        .collect();
    if live.is_empty() {
        return 0.0;
    }
    live.iter().map(|value| *value as f64).sum::<f64>() / live.len() as f64
}

/// The share of live cells sitting above the gate threshold.
fn over_threshold(data: &RuntimeSnapshotData, threshold: i64) -> f64 {
    let live: Vec<i64> = data
        .mana
        .fields
        .iter()
        .flat_map(|field| field.intensity.iter().copied())
        .filter(|value| *value != 0)
        .collect();
    if live.is_empty() {
        return 0.0;
    }
    live.iter().filter(|value| **value > threshold).count() as f64 / live.len() as f64
}

fn total_mana(data: &RuntimeSnapshotData) -> i128 {
    data.mana
        .fields
        .iter()
        .flat_map(|field| field.intensity.iter())
        .map(|value| i128::from(*value))
        .sum()
}

fn distinct<T: Ord>(values: impl IntoIterator<Item = T>) -> usize {
    values.into_iter().collect::<BTreeSet<_>>().len()
}

fn main() {
    println!(
        "chunk_extent decision evidence: {TICKS} ticks, seeds {SEEDS:?}\n\
         re-measured on a populated field after TODO-RUNTIME-002\n"
    );

    println!("what the field gets to respond to, averaged over the seeds");
    println!("extent {EXACT} is one column per terrain cell, so it is the exact reading");
    println!(
        "a spatially incoherent terrain would retain 1/(cells per column) by chance alone, so the\n\
         last column is how much better than noise the ground actually is at this lattice"
    );
    println!(
        "{:>7}{:>10}{:>12}{:>14}{:>16}{:>12}{:>12}",
        "extent", "columns", "cells/col", "variation", "fingerprints", "emitting", "vs noise"
    );
    let exact: Vec<Reading> = SEEDS
        .iter()
        .map(|seed| read_terrain(*seed, EXACT))
        .collect();
    for extent in CANDIDATES.iter().copied().chain([EXACT]) {
        let readings: Vec<Reading> = SEEDS
            .iter()
            .map(|seed| read_terrain(*seed, extent))
            .collect();
        let seeds = SEEDS.len() as f64;
        let variation = readings
            .iter()
            .zip(exact.iter())
            .map(|(candidate, reference)| variation_retained(candidate, reference))
            .sum::<f64>()
            / seeds;
        let fingerprints = readings.iter().map(|r| r.fingerprints).sum::<usize>() as f64 / seeds;
        let emitting = readings
            .iter()
            .map(|r| r.emitting_columns as f64 / r.total_columns.max(1) as f64)
            .sum::<f64>()
            / seeds;
        let cells_per_column =
            usize::from(CHUNK_SIZE).pow(2) as f64 / readings[0].total_columns as f64;
        println!(
            "{extent:>7}{:>10}{:>12.1}{:>13.1}%{:>16.1}{:>11.0}%{:>11.2}x",
            readings[0].total_columns,
            cells_per_column,
            100.0 * variation,
            fingerprints,
            100.0 * emitting,
            variation * cells_per_column,
        );
    }

    println!("\ncan the lattice still tell one world from another");
    println!("after TODO-RUNTIME-002 the seed reaches the simulation, so a lattice too coarse");
    println!("to carry the difference shows up as seeds sharing a behaviour tuple");
    println!(
        "{:>7}{:>12}{:>12}{:>14}{:>16}",
        "extent", "digests", "total mana", "behaviours", "gate crossings"
    );
    let mut runs = Vec::new();
    for extent in CANDIDATES {
        let extent_runs: Vec<Run> = SEEDS.iter().map(|seed| run(*seed, extent)).collect();
        let gates: Vec<u64> = extent_runs
            .iter()
            .map(|r| r.behaviour.gate_crossings)
            .collect();
        println!(
            "{extent:>7}{:>12}{:>12}{:>14}{:>16}",
            distinct(extent_runs.iter().map(|r| r.physical_digest)),
            distinct(extent_runs.iter().map(|r| r.total_mana)),
            distinct(extent_runs.iter().map(|r| &r.behaviour)),
            format!(
                "{}-{}",
                gates.iter().min().copied().unwrap_or(0),
                gates.iter().max().copied().unwrap_or(0)
            ),
        );
        runs.push((extent, extent_runs));
    }

    println!("\nwhat it costs, on seed 7");
    println!("a committed causal event per changed mana cell per tick is the dominant term");
    println!(
        "{:>7}{:>12}{:>16}{:>16}{:>14}",
        "extent", "ms/tick", "vs extent 3", "cells/tick", "events"
    );
    let baseline = runs[0].1[0].milliseconds_per_tick;
    for (extent, extent_runs) in &runs {
        let first = &extent_runs[0];
        println!(
            "{extent:>7}{:>12.3}{:>15.1}x{:>16.0}{:>14}",
            first.milliseconds_per_tick,
            first.milliseconds_per_tick / baseline.max(f64::MIN_POSITIVE),
            first.changed_cells_per_tick,
            first.causal_events,
        );
    }

    println!("\nwhere the field sits relative to the gate it drives, on seed 7");
    println!("gate threshold {}, hysteresis {}", THRESHOLD, HYSTERESIS);
    println!(
        "{:>7}{:>14}{:>16}{:>18}{:>16}",
        "extent", "total mana", "mean live cell", "x threshold", "live over gate"
    );
    for (extent, extent_runs) in &runs {
        let first = &extent_runs[0];
        println!(
            "{extent:>7}{:>14}{:>16.0}{:>17.1}x{:>15.0}%",
            first.total_mana,
            first.mean_intensity,
            first.mean_intensity / f64::from(THRESHOLD as i32),
            100.0 * first.cells_over_threshold,
        );
    }

    println!("\nbehaviour per seed, to show which seeds a coarse lattice merges");
    println!(
        "{:>7}{:>6}{:>10}{:>14}{:>13}{:>10}{:>12}",
        "extent", "seed", "gate", "transitions", "conditions", "actions", "population"
    );
    for (extent, extent_runs) in &runs {
        for (seed, run) in SEEDS.iter().zip(extent_runs.iter()) {
            println!(
                "{extent:>7}{seed:>6}{:>10}{:>14}{:>13}{:>10}{:>12}",
                run.behaviour.gate_crossings,
                run.behaviour.gate_transitions,
                run.behaviour.surface_conditions,
                run.behaviour.actions_committed,
                run.behaviour.population,
            );
        }
    }
}
