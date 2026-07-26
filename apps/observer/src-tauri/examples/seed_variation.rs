//! Development-only evidence for `TODO-RUNTIME-002`: does the world seed reach
//! the running simulation?
//!
//! A seed that only reaches world generation produces one world with several
//! terrains. This tool measures the difference: it generates terrain directly
//! from each seed and reports how much it varies, then runs the same seeds
//! through the tick loop and reports how much the simulation varies. If the
//! first column moves and the second does not, the carrier is generated,
//! persisted and projected but never causally consumed.
//!
//! ```text
//! cargo run --release -p causafera-observer --example seed_variation
//! ```

use std::collections::BTreeSet;
use std::time::Instant;

use causafera_runtime::{
    Runtime, RuntimeConfig, RuntimeSnapshotData, TerrainCarrierAdapter, TerrainParticipation,
    deterministic_terrain_chunk, terrain_pattern,
};
use causafera_types::{ChartChunkCoord, ChunkCoord, SpatialChartId, TraceId};

const SEEDS: [u64; 6] = [7, 11, 23, 41, 59, 97];
const TICKS: u64 = 192;
/// Lattices to price the carrier's participation on. The projection emits one
/// sample per plan-view column, so its cost grows with `extent²` while the mana
/// volume it feeds grows with `extent³`.
const COST_EXTENTS: [u8; 4] = [3, 6, 8, 12];

struct Terrain {
    distinct_patterns: usize,
    mean_elevation_mm: i64,
    distinct_materials: usize,
}

struct Run {
    physical_digest: [u8; 32],
    history_digest: [u8; 32],
    total_mana: i128,
    mana_cells_nonzero: usize,
    gate_crossings: u64,
    gate_transitions: usize,
    surface_conditions: i128,
    actions_committed: u64,
    population: u64,
    /// Total mana at the sampled ticks, so a contribution that only exists at
    /// bootstrap is distinguishable from one that persists.
    mana_trajectory: Vec<(u64, i64)>,
    /// The ticks at which the local mana gate moved, which is where a change in
    /// the field turns into a change in behaviour.
    gate_ticks: Vec<u64>,
}

const TRAJECTORY_TICKS: [u64; 6] = [1, 2, 4, 8, 48, TICKS];

/// The terrain the seed produces, read straight from the generator without a
/// runtime, so a flat simulation column cannot be blamed on generation.
fn terrain(seed: u64) -> Terrain {
    let chunk = ChartChunkCoord::new(SpatialChartId::new(1), ChunkCoord::new(0, 0, 0));
    let chunk_terrain = deterministic_terrain_chunk(seed, chunk, TraceId::new(0));
    let adapter = TerrainCarrierAdapter::new(chunk, chunk_terrain.clone(), 3);
    Terrain {
        distinct_patterns: (0..chunk_terrain.elevations().len())
            .filter_map(|index| adapter.source_cell(index as u32))
            .map(terrain_pattern)
            .collect::<BTreeSet<_>>()
            .len(),
        mean_elevation_mm: chunk_terrain
            .elevations()
            .iter()
            .map(|value| i64::from(value.millimetres()))
            .sum::<i64>()
            / chunk_terrain.elevations().len().max(1) as i64,
        distinct_materials: chunk_terrain
            .surface_materials()
            .iter()
            .copied()
            .collect::<BTreeSet<_>>()
            .len(),
    }
}

/// The production-shaped loop: actors contact material surfaces, contacts feed
/// the mana field, and the local gate feeds back into surface condition.
///
/// `open_gate` reproduces the configuration `extent_decision` measures, whose
/// threshold of one opens every gate on the first contact. The default
/// parameters instead put the gate where terrain can move it.
fn config(seed: u64, open_gate: bool) -> RuntimeConfig {
    let mut config = RuntimeConfig::new(seed);
    config.actor_count = 8;
    config.sensor_count = 2;
    config.bootstrap_population = 512;
    if open_gate {
        config.mana_parameters.effect_threshold = 1;
        config.mana_parameters.effect_hysteresis = 0;
    }
    config
}

fn run(seed: u64, open_gate: bool) -> Run {
    let mut runtime = Runtime::new(config(seed, open_gate)).expect("runtime");
    let mut mana_trajectory = Vec::new();
    let mut summary = runtime.snapshot().expect("snapshot");
    for tick in 1..=TICKS {
        summary = runtime.tick().expect("tick");
        if TRAJECTORY_TICKS.contains(&tick) {
            mana_trajectory.push((tick, summary.mana_total));
        }
    }
    let data = runtime.export_snapshot().expect("snapshot");

    Run {
        physical_digest: summary.physical_state_digest.bytes(),
        history_digest: summary.history_digest.bytes(),
        total_mana: total_mana(&data),
        mana_cells_nonzero: data
            .mana
            .fields
            .iter()
            .flat_map(|field| field.intensity.iter())
            .filter(|value| **value != 0)
            .count(),
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
        mana_trajectory,
        gate_ticks: data
            .material_surfaces
            .gate_transitions
            .iter()
            .map(|transition| transition.occurred_at.raw())
            .collect(),
    }
}

fn total_mana(data: &RuntimeSnapshotData) -> i128 {
    data.mana
        .fields
        .iter()
        .flat_map(|field| field.intensity.iter())
        .map(|value| i128::from(*value))
        .sum()
}

fn short(bytes: [u8; 32]) -> String {
    bytes[..6]
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// What the standing carrier costs per tick, against the same world with the
/// carrier held inert.
fn report_cost() {
    println!("\n=== cost of the standing terrain carrier ===");
    println!(
        "a committed causal event per changed mana cell per tick is what a live \n\
         field costs; the carrier's own work is the sample projection alone"
    );
    println!(
        "{:>8}{:>13}{:>13}{:>13}{:>13}{:>13}{:>11}",
        "extent", "samples/t", "inert ms/t", "standing", "overhead", "inert cell/t", "standing"
    );
    for extent in COST_EXTENTS {
        let mut measured = Vec::new();
        for participation in [TerrainParticipation::Inert, TerrainParticipation::Standing] {
            let mut config = config(7, false);
            config.chunk_extent = extent;
            config.terrain_participation = participation;
            let mut runtime = Runtime::new(config).expect("runtime");
            // One warm-up run so allocation and first-touch costs do not land in
            // the measured window.
            let warm = runtime.run_ticks(16).expect("warm-up");
            let started = Instant::now();
            let done = runtime.run_ticks(TICKS).expect("measured ticks");
            measured.push((
                started.elapsed().as_secs_f64() * 1000.0 / TICKS as f64,
                (done.mana_cell_changes - warm.mana_cell_changes) / TICKS,
            ));
        }
        let ((inert, inert_cells), (standing, standing_cells)) = (measured[0], measured[1]);
        let chunks = 3;
        println!(
            "{extent:>8}{:>13}{inert:>13.3}{standing:>13.3}{:>12.1}%{inert_cells:>13}{standing_cells:>11}",
            usize::from(extent).pow(2) * chunks,
            100.0 * (standing - inert) / inert.max(f64::MIN_POSITIVE),
        );
    }
}

fn main() {
    println!("seed variation evidence: {TICKS} ticks, seeds {SEEDS:?}\n");

    println!("terrain generated from the seed, one chunk, without a runtime");
    println!(
        "{:>6}{:>12}{:>14}{:>12}",
        "seed", "patterns", "mean elev mm", "materials"
    );
    for seed in SEEDS {
        let terrain = terrain(seed);
        println!(
            "{seed:>6}{:>12}{:>14}{:>12}",
            terrain.distinct_patterns, terrain.mean_elevation_mm, terrain.distinct_materials
        );
    }

    for open_gate in [false, true] {
        let label = if open_gate {
            "gate threshold 1, hysteresis 0"
        } else {
            "default gate: threshold 4096, hysteresis 2000"
        };
        println!("\n=== {label} ===");

        println!("\nsimulation state after {TICKS} ticks");
        println!(
            "{:>6}{:>16}{:>16}{:>14}{:>10}",
            "seed", "physical digest", "history digest", "total mana", "cells"
        );
        let runs = SEEDS.map(|seed| (seed, run(seed, open_gate)));
        for (seed, run) in &runs {
            println!(
                "{seed:>6}{:>16}{:>16}{:>14}{:>10}",
                short(run.physical_digest),
                short(run.history_digest),
                run.total_mana,
                run.mana_cells_nonzero,
            );
        }

        println!("\nbehaviour after the same tick count");
        println!(
            "{:>6}{:>10}{:>14}{:>14}{:>12}{:>12}",
            "seed", "gate", "transitions", "conditions", "actions", "population"
        );
        for (seed, run) in &runs {
            println!(
                "{seed:>6}{:>10}{:>14}{:>14}{:>12}{:>12}",
                run.gate_crossings,
                run.gate_transitions,
                run.surface_conditions,
                run.actions_committed,
                run.population,
            );
        }

        println!("\ntotal mana over time, and the ticks the local mana gate moved");
        print!("{:>6}", "seed");
        for tick in TRAJECTORY_TICKS {
            print!("{:>12}", format!("t={tick}"));
        }
        println!("   gate ticks");
        for (seed, run) in &runs {
            print!("{seed:>6}");
            for (_, total) in &run.mana_trajectory {
                print!("{total:>12}");
            }
            println!("   {:?}", run.gate_ticks);
        }

        let distinct_physical = runs
            .iter()
            .map(|(_, run)| run.physical_digest)
            .collect::<BTreeSet<_>>()
            .len();
        let distinct_mana = runs
            .iter()
            .map(|(_, run)| run.total_mana)
            .collect::<BTreeSet<_>>()
            .len();
        let distinct_behaviour = runs
            .iter()
            .map(|(_, run)| {
                (
                    run.gate_crossings,
                    run.gate_transitions,
                    run.surface_conditions,
                    run.actions_committed,
                )
            })
            .collect::<BTreeSet<_>>()
            .len();
        println!(
            "\ndistinct across {} seeds: physical digests {distinct_physical}, \
             total mana {distinct_mana}, behaviour tuples {distinct_behaviour}",
            SEEDS.len()
        );
    }

    report_cost();
}
