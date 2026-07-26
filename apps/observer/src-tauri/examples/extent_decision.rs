//! Development-only validation for the `chunk_extent` default decision
//! (`TODO-MANA-004`).
//!
//! `extent_bench` measures one seed and reports totals. A total can agree while
//! the field sits in the wrong place, and a lattice change can move behaviour
//! across a gate without moving any aggregate much at all. This tool answers
//! the three questions a default change has to survive:
//!
//!   1. does the convergence hold across seeds, or is seed 7 lucky;
//!   2. is the candidate lattice right locally, not just in sum;
//!   3. does it cross the mana gate a different number of times, which is where
//!      a quiet lattice change would turn into a loud behavioural one.
//!
//! Build in release; debug timings and run lengths make this useless.
//!
//! ```text
//! cargo run --release -p causafera-observer --example extent_decision
//! ```

use causafera_runtime::{Runtime, RuntimeConfig, RuntimeSnapshotData};

const SEEDS: [u64; 6] = [7, 11, 23, 41, 59, 97];
/// Every compared extent must divide the reference so both fields reduce onto a
/// common grid exactly, with no interpolation inventing values.
const REFERENCE: u8 = 12;
const CANDIDATES: [u8; 3] = [3, 4, 6];
const TICKS: u64 = 192;

struct Run {
    total_mana: i128,
    gate_crossings: u64,
    gate_transitions: usize,
    actions_committed: u64,
    population: u64,
    /// Plan-view column sums per chunk, in the field set's canonical order.
    columns: Vec<(u8, Vec<i128>)>,
}

fn run(seed: u64, extent: u8) -> Run {
    let mut config = RuntimeConfig::new(seed);
    config.chunk_extent = extent;
    config.actor_count = 8;
    config.sensor_count = 2;
    config.bootstrap_population = 512;
    config.mana_parameters.effect_threshold = 1;
    config.mana_parameters.effect_hysteresis = 0;

    let mut runtime = Runtime::new(config).expect("runtime");
    let summary = runtime.run_ticks(TICKS).expect("ticks");
    let data = runtime.export_snapshot().expect("snapshot");

    Run {
        total_mana: total_mana(&data),
        gate_crossings: summary.mana_physical_effects,
        gate_transitions: data.material_surfaces.gate_transitions.len(),
        actions_committed: summary.actor_actions_committed,
        population: summary.population_total,
        columns: plan_view(&data),
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

/// The field the map actually draws: the volume reduced through z.
fn plan_view(data: &RuntimeSnapshotData) -> Vec<(u8, Vec<i128>)> {
    data.mana
        .fields
        .iter()
        .map(|field| {
            let edge = usize::from(field.extent);
            let mut columns = vec![0_i128; edge * edge];
            for (index, value) in field.intensity.iter().enumerate() {
                let plane = index % (edge * edge);
                columns[plane] += i128::from(*value);
            }
            (field.extent, columns)
        })
        .collect()
}

/// Sum a plan view down onto a coarser square grid. The reference lattice is
/// chosen so this is always an exact block sum.
fn reduce(columns: &[i128], from: usize, to: usize) -> Vec<i128> {
    let block = from / to;
    let mut reduced = vec![0_i128; to * to];
    for (index, value) in columns.iter().enumerate() {
        let x = (index % from) / block;
        let y = (index / from) / block;
        reduced[y * to + x] += value;
    }
    reduced
}

/// Two errors, because they answer different questions. The absolute error
/// includes the candidate simply holding less mana; the shape error normalises
/// that away and asks only whether the mana sits in the same places.
fn errors(candidate: &Run, reference: &Run, edge: u8) -> (f64, f64) {
    let to = usize::from(edge);
    let mut absolute = 0.0_f64;
    let mut shape = 0.0_f64;
    let mut scale = 0.0_f64;

    for ((_, candidate_columns), (reference_extent, reference_columns)) in
        candidate.columns.iter().zip(reference.columns.iter())
    {
        let reduced = reduce(reference_columns, usize::from(*reference_extent), to);
        let candidate_total: i128 = candidate_columns.iter().sum();
        let reduced_total: i128 = reduced.iter().sum();
        scale += reduced_total as f64;

        for (a, b) in candidate_columns.iter().zip(reduced.iter()) {
            absolute += (*a - *b).unsigned_abs() as f64;
            let share = |value: i128, total: i128| {
                if total == 0 {
                    0.0
                } else {
                    value as f64 / total as f64
                }
            };
            shape += (share(*a, candidate_total) - share(*b, reduced_total)).abs();
        }
    }

    // The shape term is a total-variation distance between two distributions,
    // which runs 0 to 2; halving puts it on 0 to 1 like the absolute error.
    (
        absolute / scale.max(1.0),
        shape / (2.0 * candidate.columns.len().max(1) as f64),
    )
}

fn main() {
    println!(
        "chunk_extent decision evidence: {TICKS} ticks, reference lattice {REFERENCE}, \
         seeds {SEEDS:?}\n"
    );

    println!("total mana against the reference lattice");
    print!("{:>6}", "seed");
    for extent in CANDIDATES {
        print!("{:>12}", format!("extent {extent}"));
    }
    println!("{:>14}", "reference");

    let mut runs = Vec::new();
    for seed in SEEDS {
        let reference = run(seed, REFERENCE);
        print!("{seed:>6}");
        let mut candidates = Vec::new();
        for extent in CANDIDATES {
            let candidate = run(seed, extent);
            print!(
                "{:>11.1}%",
                100.0 * candidate.total_mana as f64 / reference.total_mana.max(1) as f64
            );
            candidates.push((extent, candidate));
        }
        println!("{:>14}", reference.total_mana);
        runs.push((seed, candidates, reference));
    }

    println!("\nlocal error against the reference, on each candidate's own grid");
    println!("absolute includes holding less mana; shape asks only where the mana sits");
    print!("{:>6}", "seed");
    for extent in CANDIDATES {
        print!("{:>22}", format!("extent {extent} abs / shape"));
    }
    println!();
    for (seed, candidates, reference) in &runs {
        print!("{seed:>6}");
        for (extent, candidate) in candidates {
            let (absolute, shape) = errors(candidate, reference, *extent);
            print!("{:>13.1}% {:>7.1}%", 100.0 * absolute, 100.0 * shape);
        }
        println!();
    }

    println!("\nbehaviour: mana gate crossings, surface gate transitions, actions, population");
    println!("a lattice change that moves these has stopped being a resolution change");
    print!("{:>6}{:>12}", "seed", "extent");
    println!(
        "{:>10}{:>14}{:>12}{:>12}",
        "gate", "transitions", "actions", "population"
    );
    for (seed, candidates, reference) in &runs {
        for (extent, candidate) in candidates {
            println!(
                "{seed:>6}{extent:>12}{:>10}{:>14}{:>12}{:>12}",
                candidate.gate_crossings,
                candidate.gate_transitions,
                candidate.actions_committed,
                candidate.population,
            );
        }
        println!(
            "{seed:>6}{REFERENCE:>12}{:>10}{:>14}{:>12}{:>12}",
            reference.gate_crossings,
            reference.gate_transitions,
            reference.actions_committed,
            reference.population,
        );
    }
}
