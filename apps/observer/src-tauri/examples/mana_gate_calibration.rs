//! Development-only evidence for `TODO-MANA-007`: calibrating the local
//! effect gate against the population it actually reads.
//!
//! `ManaEffectsSystem::execute` (`causafera-runtime/src/mana.rs`) evaluates
//! exactly one mana cell per material surface — `intensity[surface.id
//! .cell_index]` in the field of `surface.id.chunk` — and only for surfaces
//! with `contact_count > 0`. `MaterialSurfaceBootstrapStage` places every
//! surface at cell index 0, so the gate never sees any cell but that one per
//! contacted chunk. `extent_decision.rs` measures "share of live cells above
//! the gate" over every live cell in every field; that is a different,
//! larger population than the one the gate is actually evaluated against,
//! so it is the wrong yardstick for calibrating the threshold. This tool
//! measures the right one directly.
//!
//! The mana field's own evolution does not read `material_surfaces` (only
//! the gate reads the field; nothing feeds the surface's condition or gate
//! state back into mana evolution), so `threshold` and `hysteresis` cannot
//! change the intensity trace itself. One simulation run per seed/extent is
//! enough to capture every candidate operating point: record the intensity
//! each contacted surface actually saw, tick by tick, then replay the gate's
//! own hysteresis state machine against that recorded trace for each
//! candidate `(threshold, hysteresis)` pair instead of rerunning the
//! simulation per candidate.
//!
//! Build in release; per-tick snapshot export in a loop is expensive in
//! debug.
//!
//! ```text
//! cargo run --release -p causafera-observer --example mana_gate_calibration
//! ```

use std::collections::{BTreeMap, BTreeSet};

use causafera_runtime::{MaterialSurfaceId, Runtime, RuntimeConfig};
use causafera_types::ChartChunkCoord;

const SEEDS: [u64; 6] = [7, 11, 23, 41, 59, 97];
const EXTENTS: [u8; 5] = [3, 4, 6, 8, 12];
const TICKS: u64 = 192;
/// The production defaults, restated here only so the report can print what
/// they are being compared against. `RuntimeConfig::new` remains the
/// authority.
const CURRENT_THRESHOLD: i64 = 4_096;
const CURRENT_HYSTERESIS: i64 = 2_000;
/// Candidate thresholds to score against the recorded traces. Hysteresis is
/// scored at a fixed fraction of each candidate threshold rather than at a
/// fixed constant, so a low threshold does not get an oversized dead band
/// relative to its own scale.
const CANDIDATE_THRESHOLDS: [i64; 13] = [
    16, 32, 64, 128, 256, 512, 1_024, 2_048, 3_072, 4_096, 5_120, 6_144, 8_192,
];
const HYSTERESIS_FRACTION: i64 = 4; // threshold / 4
/// The only pair in the sweep above that discriminates the six seeds at
/// every candidate extent simultaneously, not just at the production
/// default.
const CHOSEN_THRESHOLD: i64 = 6_144;
const CHOSEN_HYSTERESIS: i64 = 1_536;

/// One contacted surface's cell-0 intensity, recorded only from the tick it
/// first became contacted — before that the real gate never evaluates it
/// either.
struct SurfaceTrace {
    intensity: Vec<i64>,
}

fn record(seed: u64, extent: u8) -> Vec<SurfaceTrace> {
    let mut config = RuntimeConfig::new(seed);
    config.chunk_extent = extent;
    config.actor_count = 8;
    config.sensor_count = 2;
    config.bootstrap_population = 512;
    let mut runtime = Runtime::new(config).expect("runtime");

    let mut traces: BTreeMap<MaterialSurfaceId, Vec<i64>> = BTreeMap::new();
    for _ in 0..TICKS {
        runtime.run_ticks(1).expect("tick");
        let data = runtime.export_snapshot().expect("snapshot");
        let field_by_chunk: BTreeMap<ChartChunkCoord, &Vec<i64>> = data
            .mana
            .fields
            .iter()
            .map(|field| (field.chunk, &field.intensity))
            .collect();
        for surface_record in &data.material_surfaces.records {
            if surface_record.surface.contact_count == 0 {
                continue;
            }
            let intensity = field_by_chunk
                .get(&surface_record.id.chunk)
                .and_then(|values| values.get(usize::from(surface_record.id.cell_index)))
                .copied()
                .unwrap_or(0);
            traces.entry(surface_record.id).or_default().push(intensity);
        }
    }

    traces
        .into_values()
        .map(|intensity| SurfaceTrace { intensity })
        .collect()
}

/// Replays the gate's own hysteresis state machine
/// (`ManaEffectsSystem::execute`) against a recorded trace. Returns the
/// number of transitions (both directions) and whether the gate ever opened.
fn simulate(trace: &[i64], threshold: i64, hysteresis: i64) -> (u64, bool) {
    let mut active = false;
    let mut transitions = 0u64;
    let mut ever_active = false;
    for &intensity in trace {
        let fires = if active {
            intensity < threshold.saturating_sub(hysteresis)
        } else {
            intensity > threshold
        };
        if fires {
            active = !active;
            transitions += 1;
            ever_active |= active;
        }
    }
    (transitions, ever_active)
}

fn distinct<T: Ord>(values: impl IntoIterator<Item = T>) -> usize {
    values.into_iter().collect::<BTreeSet<_>>().len()
}

/// The same five-field tuple `extent_decision.rs` uses to judge whether a
/// lattice can still tell worlds apart, so a candidate operating point is
/// checked against the exact claim the TODO's original evidence made, not a
/// narrower proxy.
#[derive(PartialEq, Eq, PartialOrd, Ord)]
struct Behaviour {
    gate_crossings: u64,
    gate_transitions: usize,
    surface_conditions: i128,
    actions_committed: u64,
    population: u64,
}

fn run_behaviour(seed: u64, extent: u8, threshold: i64, hysteresis: i64) -> Behaviour {
    let mut config = RuntimeConfig::new(seed);
    config.chunk_extent = extent;
    config.actor_count = 8;
    config.sensor_count = 2;
    config.bootstrap_population = 512;
    config.mana_parameters.effect_threshold = threshold;
    config.mana_parameters.effect_hysteresis = hysteresis;
    let mut runtime = Runtime::new(config).expect("runtime");
    let summary = runtime.run_ticks(TICKS).expect("ticks");
    let data = runtime.export_snapshot().expect("snapshot");
    Behaviour {
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
    }
}

fn main() {
    println!(
        "mana gate calibration evidence (TODO-MANA-007): {TICKS} ticks, seeds {SEEDS:?}\n\
         population: cell 0 of every contacted material surface, which is the only\n\
         cell the local effect gate ever reads\n"
    );

    let mut by_extent: BTreeMap<u8, Vec<Vec<SurfaceTrace>>> = BTreeMap::new();
    for extent in EXTENTS {
        let mut runs = Vec::new();
        for seed in SEEDS {
            runs.push(record(seed, extent));
        }
        by_extent.insert(extent, runs);
    }

    println!("spread of the gate's own population, pooling every contacted surface's");
    println!("every recorded tick across all six seeds");
    println!(
        "{:>7}{:>10}{:>12}{:>12}{:>12}{:>12}",
        "extent", "surfaces", "mean", "stdev", "min", "max"
    );
    for (extent, runs) in &by_extent {
        let pooled: Vec<i64> = runs
            .iter()
            .flat_map(|traces| traces.iter().flat_map(|t| t.intensity.iter().copied()))
            .collect();
        let surfaces: usize = runs.iter().map(|traces| traces.len()).sum();
        let n = pooled.len().max(1) as f64;
        let mean = pooled.iter().map(|v| *v as f64).sum::<f64>() / n;
        let variance = pooled
            .iter()
            .map(|v| (*v as f64 - mean).powi(2))
            .sum::<f64>()
            / n;
        let min = pooled.iter().copied().min().unwrap_or(0);
        let max = pooled.iter().copied().max().unwrap_or(0);
        println!(
            "{extent:>7}{surfaces:>10}{mean:>12.0}{:>12.0}{min:>12}{max:>12}",
            variance.sqrt(),
        );
    }

    println!(
        "\nfor reference, the production gate: threshold {CURRENT_THRESHOLD}, hysteresis {CURRENT_HYSTERESIS}"
    );
    println!(
        "{:>7}{:>12}{:>14}{:>12}",
        "extent", "transitions", "ever-active", "distinct"
    );
    for (extent, runs) in &by_extent {
        let per_seed: Vec<(u64, usize)> = runs
            .iter()
            .map(|traces| {
                let mut total = 0u64;
                let mut active_surfaces = 0usize;
                for trace in traces {
                    let (t, ever) =
                        simulate(&trace.intensity, CURRENT_THRESHOLD, CURRENT_HYSTERESIS);
                    total += t;
                    active_surfaces += usize::from(ever);
                }
                (total, active_surfaces)
            })
            .collect();
        let total_range = (
            per_seed.iter().map(|(t, _)| *t).min().unwrap_or(0),
            per_seed.iter().map(|(t, _)| *t).max().unwrap_or(0),
        );
        println!(
            "{extent:>7}{:>7}-{:<5}{:>14}{:>12}",
            total_range.0,
            total_range.1,
            "n/a",
            distinct(per_seed.iter().copied()),
        );
    }

    println!("\ncandidate thresholds against the recorded traces");
    println!(
        "hysteresis = threshold / {HYSTERESIS_FRACTION}; distinct = distinct (transitions, ever-active-count) tuples across the 6 seeds"
    );
    println!(
        "{:>10}{:>12}{}",
        "threshold",
        "hysteresis",
        EXTENTS
            .iter()
            .map(|e| format!("{:>10}", format!("ext{e}")))
            .collect::<String>()
    );
    for threshold in CANDIDATE_THRESHOLDS {
        let hysteresis = threshold / HYSTERESIS_FRACTION;
        let mut row = format!("{threshold:>10}{hysteresis:>12}");
        for extent in EXTENTS {
            let runs = &by_extent[&extent];
            let per_seed: Vec<(u64, usize)> = runs
                .iter()
                .map(|traces| {
                    let mut total = 0u64;
                    let mut active_surfaces = 0usize;
                    for trace in traces {
                        let (t, ever) = simulate(&trace.intensity, threshold, hysteresis);
                        total += t;
                        active_surfaces += usize::from(ever);
                    }
                    (total, active_surfaces)
                })
                .collect();
            row.push_str(&format!("{:>10}", distinct(per_seed.iter().copied())));
        }
        println!("{row}");
    }

    println!("\nneighbourhood check around 6144 (hysteresis = t/4): is 6144 a plateau or a spike?");
    println!(
        "{:>10}{:>12}{}",
        "threshold",
        "hysteresis",
        EXTENTS
            .iter()
            .map(|e| format!("{:>10}", format!("ext{e}")))
            .collect::<String>()
    );
    for threshold in [5_632i64, 5_888, 6_144, 6_400, 6_656] {
        let hysteresis = threshold / HYSTERESIS_FRACTION;
        let mut row = format!("{threshold:>10}{hysteresis:>12}");
        for extent in EXTENTS {
            let runs = &by_extent[&extent];
            let per_seed: Vec<(u64, usize)> = runs
                .iter()
                .map(|traces| {
                    let mut total = 0u64;
                    let mut active_surfaces = 0usize;
                    for trace in traces {
                        let (t, ever) = simulate(&trace.intensity, threshold, hysteresis);
                        total += t;
                        active_surfaces += usize::from(ever);
                    }
                    (total, active_surfaces)
                })
                .collect();
            row.push_str(&format!("{:>10}", distinct(per_seed.iter().copied())));
        }
        println!("{row}");
    }

    println!(
        "\nhysteresis axis at threshold 4096 and 6144: is the effect from the threshold or the hysteresis?"
    );
    println!(
        "{:>10}{:>12}{}",
        "threshold",
        "hysteresis",
        EXTENTS
            .iter()
            .map(|e| format!("{:>10}", format!("ext{e}")))
            .collect::<String>()
    );
    for threshold in [4_096i64, 6_144] {
        for fraction in [2i64, 3, 4, 6] {
            let hysteresis = threshold / fraction;
            let mut row = format!("{threshold:>10}{hysteresis:>12}");
            for extent in EXTENTS {
                let runs = &by_extent[&extent];
                let per_seed: Vec<(u64, usize)> = runs
                    .iter()
                    .map(|traces| {
                        let mut total = 0u64;
                        let mut active_surfaces = 0usize;
                        for trace in traces {
                            let (t, ever) = simulate(&trace.intensity, threshold, hysteresis);
                            total += t;
                            active_surfaces += usize::from(ever);
                        }
                        (total, active_surfaces)
                    })
                    .collect();
                row.push_str(&format!("{:>10}", distinct(per_seed.iter().copied())));
            }
            println!("{row}");
        }
    }

    println!(
        "\nchosen operating point: threshold {CHOSEN_THRESHOLD}, hysteresis {CHOSEN_HYSTERESIS}"
    );
    println!("per seed, per extent, against the recorded traces");
    println!(
        "{:>7}{:>6}{:>14}{:>16}{:>18}",
        "extent", "seed", "transitions", "ever-active", "mean/threshold"
    );
    for extent in EXTENTS {
        let runs = &by_extent[&extent];
        for (seed, traces) in SEEDS.iter().zip(runs.iter()) {
            let mut total = 0u64;
            let mut active_surfaces = 0usize;
            let mut sum = 0i64;
            let mut count = 0usize;
            for trace in traces {
                let (t, ever) = simulate(&trace.intensity, CHOSEN_THRESHOLD, CHOSEN_HYSTERESIS);
                total += t;
                active_surfaces += usize::from(ever);
                sum += trace.intensity.iter().sum::<i64>();
                count += trace.intensity.len();
            }
            let mean = sum as f64 / count.max(1) as f64;
            println!(
                "{extent:>7}{seed:>6}{total:>14}{active_surfaces:>16}{:>17.2}x",
                mean / CHOSEN_THRESHOLD as f64,
            );
        }
    }

    println!(
        "\nend-to-end check against the same five-field Behaviour tuple extent_decision.rs uses\n\
         (gate crossings, gate transitions, surface conditions, actions, population) — real\n\
         production runs, not the recorded-trace replay above"
    );
    println!(
        "{:>7}{:>24}{:>24}",
        "extent",
        format!("current {CURRENT_THRESHOLD}/{CURRENT_HYSTERESIS}"),
        format!("chosen {CHOSEN_THRESHOLD}/{CHOSEN_HYSTERESIS}")
    );
    for extent in EXTENTS {
        let current: Vec<Behaviour> = SEEDS
            .iter()
            .map(|seed| run_behaviour(*seed, extent, CURRENT_THRESHOLD, CURRENT_HYSTERESIS))
            .collect();
        let chosen: Vec<Behaviour> = SEEDS
            .iter()
            .map(|seed| run_behaviour(*seed, extent, CHOSEN_THRESHOLD, CHOSEN_HYSTERESIS))
            .collect();
        println!(
            "{extent:>7}{:>24}{:>24}",
            format!("{} distinct", distinct(current)),
            format!("{} distinct", distinct(chosen)),
        );
    }
}
