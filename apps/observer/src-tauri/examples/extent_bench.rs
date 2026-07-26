//! Development-only benchmark: what a finer mana lattice costs and what it buys.
//!
//! Supplies the evidence stage 8 of `plans/observer-field-raster-map.md` asks for. Build in
//! release; debug timings are meaningless for this question.
//!
//! ```text
//! cargo run --release -p causafera-observer --example extent_bench
//! ```

use std::time::Instant;

use causafera_runtime::{Runtime, RuntimeConfig};

const TICKS: u64 = 48;

fn main() {
    println!(
        "{:>6} {:>10} {:>12} {:>10} {:>12} {:>10} {:>9} {:>9}",
        "extent",
        "cells/chunk",
        "mana cells",
        "build ms",
        "48 ticks ms",
        "ms/tick",
        "populated",
        "coherence",
    );

    for extent in [3u8, 4, 6, 8, 12, 16, 24, 32] {
        let mut config = RuntimeConfig::new(7);
        config.chunk_extent = extent;
        config.actor_count = 8;
        config.sensor_count = 2;
        config.bootstrap_population = 512;
        config.mana_parameters.effect_threshold = 1;
        config.mana_parameters.effect_hysteresis = 0;

        let started = Instant::now();
        let mut runtime = match Runtime::new(config) {
            Ok(runtime) => runtime,
            Err(error) => {
                println!("{extent:>6} rejected: {error}");
                continue;
            }
        };
        let build_ms = started.elapsed().as_secs_f64() * 1000.0;

        let started = Instant::now();
        if let Err(error) = runtime.run_ticks(TICKS) {
            println!("{extent:>6} failed after build: {error}");
            continue;
        }
        let run_ms = started.elapsed().as_secs_f64() * 1000.0;

        let data = runtime.export_snapshot().expect("snapshot");
        let volume = usize::from(extent).pow(3);
        let mut total_cells = 0usize;
        let mut populated = 0usize;
        let mut deltas = 0.0f64;
        let mut pairs = 0.0f64;
        let mut variance_sum = 0.0f64;
        let mut variance_count = 0.0f64;

        for field in &data.mana.fields {
            let edge = usize::from(field.extent);
            let cells = &field.intensity;
            total_cells += cells.len();
            populated += cells.iter().filter(|value| **value != 0).count();

            let mean = cells.iter().map(|v| *v as f64).sum::<f64>() / cells.len() as f64;
            for value in cells {
                variance_sum += (*value as f64 - mean).powi(2);
                variance_count += 1.0;
            }
            let at = |x: usize, y: usize, z: usize| cells[z * edge * edge + y * edge + x] as f64;
            for z in 0..edge {
                for y in 0..edge {
                    for x in 1..edge {
                        deltas += (at(x, y, z) - at(x - 1, y, z)).abs();
                        pairs += 1.0;
                    }
                }
            }
        }

        let sigma = (variance_sum / variance_count).sqrt().max(1.0);
        println!(
            "{:>6} {:>10} {:>12} {:>10.1} {:>12.1} {:>10.2} {:>8.1}% {:>9.2}",
            extent,
            volume,
            total_cells,
            build_ms,
            run_ms,
            run_ms / TICKS as f64,
            100.0 * populated as f64 / total_cells.max(1) as f64,
            (deltas / pairs.max(1.0)) / sigma,
        );
    }

    println!("\ncoherence: 1.13 would be white noise, under 0.3 is a smooth field");

    // Does a finer lattice merely need longer to fill, or does it never fill? If mana
    // propagates a fixed number of cells per tick rather than a fixed distance, the lattice
    // silently changes how fast mana moves through the world.
    println!("\nfill over time");
    println!(
        "{:>6} {:>8} {:>11} {:>10}",
        "extent", "ticks", "populated", "ms/tick"
    );
    for extent in [3u8, 8, 16] {
        for ticks in [48u64, 192, 768] {
            let mut config = RuntimeConfig::new(7);
            config.chunk_extent = extent;
            config.actor_count = 8;
            config.sensor_count = 2;
            config.bootstrap_population = 512;
            config.mana_parameters.effect_threshold = 1;
            config.mana_parameters.effect_hysteresis = 0;
            let mut runtime = Runtime::new(config).expect("runtime");
            let started = Instant::now();
            runtime.run_ticks(ticks).expect("ticks");
            let elapsed = started.elapsed().as_secs_f64() * 1000.0;
            let data = runtime.export_snapshot().expect("snapshot");
            let mut total = 0usize;
            let mut populated = 0usize;
            for field in &data.mana.fields {
                total += field.intensity.len();
                populated += field.intensity.iter().filter(|v| **v != 0).count();
            }
            println!(
                "{:>6} {:>8} {:>10.1}% {:>10.2}",
                extent,
                ticks,
                100.0 * populated as f64 / total.max(1) as f64,
                elapsed / ticks as f64,
            );
        }
    }

    // Where does the mana actually sit? A blob would be contiguous; a plane would be one
    // layer; source-local would be a handful of scattered points.
    // Correctness before appearance: is the field conserved as the lattice refines? If the
    // same total mana is divided into more integer cells, some floor to zero and the total
    // falls. That would make the physics lattice-dependent, which is a simulation defect and
    // not a rendering one.
    println!("\nconservation across the lattice, 192 ticks");
    println!(
        "{:>6} {:>14} {:>12} {:>12} {:>10}",
        "extent", "total mana", "vs extent 3", "max cell", "min non-zero",
    );
    let mut baseline = 0i128;
    for extent in [3u8, 4, 6, 8, 12, 16] {
        let mut config = RuntimeConfig::new(7);
        config.chunk_extent = extent;
        config.actor_count = 8;
        config.sensor_count = 2;
        config.bootstrap_population = 512;
        config.mana_parameters.effect_threshold = 1;
        config.mana_parameters.effect_hysteresis = 0;
        let mut runtime = Runtime::new(config).expect("runtime");
        runtime.run_ticks(192).expect("ticks");
        let data = runtime.export_snapshot().expect("snapshot");
        let mut total = 0i128;
        let mut max_cell = 0i64;
        let mut min_non_zero = i64::MAX;
        for field in &data.mana.fields {
            for value in &field.intensity {
                total += i128::from(*value);
                max_cell = max_cell.max(*value);
                if *value != 0 {
                    min_non_zero = min_non_zero.min(value.abs());
                }
            }
        }
        if extent == 3 {
            baseline = total;
        }
        println!(
            "{:>6} {:>14} {:>11.1}% {:>12} {:>10}",
            extent,
            total,
            100.0 * total as f64 / baseline.max(1) as f64,
            max_cell,
            if min_non_zero == i64::MAX {
                0
            } else {
                min_non_zero
            },
        );
    }

    // The map reduces the volume to a plan view, so the honest metric is the column field:
    // how many columns carry mana, and whether that field is smooth enough to draw.
    println!("\nplan view after 192 ticks: the field the map would actually draw");
    println!(
        "{:>6} {:>10} {:>13} {:>12} {:>11}",
        "extent", "columns", "non-zero", "coherence", "ms/tick",
    );
    for extent in [3u8, 4, 6, 8, 12, 16] {
        let mut config = RuntimeConfig::new(7);
        config.chunk_extent = extent;
        config.actor_count = 8;
        config.sensor_count = 2;
        config.bootstrap_population = 512;
        config.mana_parameters.effect_threshold = 1;
        config.mana_parameters.effect_hysteresis = 0;
        let mut runtime = Runtime::new(config).expect("runtime");
        let started = Instant::now();
        runtime.run_ticks(192).expect("ticks");
        let ms = started.elapsed().as_secs_f64() * 1000.0 / 192.0;
        let data = runtime.export_snapshot().expect("snapshot");
        let edge = usize::from(extent);
        let mut columns_total = 0usize;
        let mut columns_non_zero = 0usize;
        let mut deltas = 0.0f64;
        let mut pairs = 0.0f64;
        let mut variance = 0.0f64;
        let mut count = 0.0f64;
        for field in &data.mana.fields {
            let mut columns = vec![0i64; edge * edge];
            for z in 0..edge {
                for y in 0..edge {
                    for x in 0..edge {
                        columns[y * edge + x] += field.intensity[z * edge * edge + y * edge + x];
                    }
                }
            }
            columns_total += columns.len();
            columns_non_zero += columns.iter().filter(|v| **v != 0).count();
            let mean = columns.iter().map(|v| *v as f64).sum::<f64>() / columns.len() as f64;
            for value in &columns {
                variance += (*value as f64 - mean).powi(2);
                count += 1.0;
            }
            for y in 0..edge {
                for x in 1..edge {
                    deltas +=
                        (columns[y * edge + x] as f64 - columns[y * edge + x - 1] as f64).abs();
                    pairs += 1.0;
                }
            }
        }
        let sigma = (variance / count).sqrt().max(1.0);
        println!(
            "{:>6} {:>10} {:>11.1}% {:>12.2} {:>11.2}",
            extent,
            columns_total,
            100.0 * columns_non_zero as f64 / columns_total.max(1) as f64,
            (deltas / pairs.max(1.0)) / sigma,
            ms,
        );
    }

    println!("\nwhere populated cells sit, extent 8 after 192 ticks");
    let mut config = RuntimeConfig::new(7);
    config.chunk_extent = 8;
    config.actor_count = 8;
    config.sensor_count = 2;
    config.bootstrap_population = 512;
    config.mana_parameters.effect_threshold = 1;
    config.mana_parameters.effect_hysteresis = 0;
    let mut runtime = Runtime::new(config).expect("runtime");
    runtime.run_ticks(192).expect("ticks");
    let data = runtime.export_snapshot().expect("snapshot");
    for field in &data.mana.fields {
        let edge = usize::from(field.extent);
        let mut per_layer = vec![0usize; edge];
        let mut bounds = [usize::MAX, 0usize, usize::MAX, 0usize, usize::MAX, 0usize];
        for (z, layer) in per_layer.iter_mut().enumerate() {
            for y in 0..edge {
                for x in 0..edge {
                    if field.intensity[z * edge * edge + y * edge + x] != 0 {
                        *layer += 1;
                        bounds[0] = bounds[0].min(x);
                        bounds[1] = bounds[1].max(x);
                        bounds[2] = bounds[2].min(y);
                        bounds[3] = bounds[3].max(y);
                        bounds[4] = bounds[4].min(z);
                        bounds[5] = bounds[5].max(z);
                    }
                }
            }
        }
        println!(
            "  chunk {:?} per-z {:?} bbox x{}..{} y{}..{} z{}..{}",
            field.chunk.chunk,
            per_layer,
            bounds[0],
            bounds[1],
            bounds[2],
            bounds[3],
            bounds[4],
            bounds[5],
        );
    }
}
