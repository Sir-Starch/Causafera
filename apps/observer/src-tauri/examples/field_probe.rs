//! Development-only probe: what the spatial fields actually hold per chunk.
//!
//! Sizes the observer projection proposed in `plans/observer-field-raster-map.md`, and answers
//! the question that decides how each field can be drawn: is it a coherent field or noise?
//! Prints nothing that is not read straight from a real runtime snapshot.

use std::collections::BTreeMap;
use std::time::Instant;

use causafera_runtime::{Runtime, RuntimeConfig, TerrainCarrierAdapter, decode_terrain_chunk};

fn main() {
    let mut config = RuntimeConfig::new(7);
    config.actor_count = 8;
    config.sensor_count = 2;
    config.bootstrap_population = 512;
    let mut runtime = Runtime::new(config).expect("runtime");
    runtime.run_ticks(48).expect("ticks");
    let data = runtime.export_snapshot().expect("snapshot");

    println!("terrain chunks: {}", data.spatial.carrier_adapters.len());
    for carrier in &data.spatial.carrier_adapters {
        let elevations = &carrier.elevations_mm;
        let roughness = &carrier.roughness_mm;
        let mut materials: BTreeMap<u64, usize> = BTreeMap::new();
        for material in &carrier.surface_materials {
            *materials.entry(material.raw()).or_default() += 1;
        }
        let min = elevations.iter().copied().min().unwrap_or(0);
        let max = elevations.iter().copied().max().unwrap_or(0);
        let mean = elevations.iter().map(|v| i64::from(*v)).sum::<i64>() / elevations.len() as i64;
        // Neighbour delta along a row: how much of the range is local texture.
        let mut steepest = 0;
        for row in 0..32 {
            for column in 1..32 {
                let a = elevations[row * 32 + column];
                let b = elevations[row * 32 + column - 1];
                steepest = steepest.max((a - b).abs());
            }
        }
        // Is the field a coherent landform or high-frequency noise? For white noise the mean
        // absolute neighbour delta approaches sigma * 2/sqrt(pi) ~= 1.13 * sigma.
        let n = elevations.len() as f64;
        let mean_f = elevations.iter().map(|v| f64::from(*v)).sum::<f64>() / n;
        let sigma = (elevations
            .iter()
            .map(|v| (f64::from(*v) - mean_f).powi(2))
            .sum::<f64>()
            / n)
            .sqrt();
        let mut deltas = 0.0;
        let mut count = 0.0;
        let mut same_material = 0.0;
        let mut pairs = 0.0;
        for row in 0..32usize {
            for column in 0..32usize {
                for (dr, dc) in [(0usize, 1usize), (1, 0)] {
                    let (r2, c2) = (row + dr, column + dc);
                    if r2 >= 32 || c2 >= 32 {
                        continue;
                    }
                    let a = elevations[row * 32 + column];
                    let b = elevations[r2 * 32 + c2];
                    deltas += f64::from((a - b).abs());
                    count += 1.0;
                    if carrier.surface_materials[row * 32 + column]
                        == carrier.surface_materials[r2 * 32 + c2]
                    {
                        same_material += 1.0;
                    }
                    pairs += 1.0;
                }
            }
        }
        println!(
            "   sigma {:.0}mm  mean|delta| {:.0}mm  ratio {:.2} (1.13 = white noise, <0.3 = smooth)  same-material neighbours {:.1}% (6.2% = random)",
            sigma,
            deltas / count,
            (deltas / count) / sigma,
            100.0 * same_material / pairs,
        );
        println!(
            "chunk {:?} cells={} elev {}..{} mean {} steepest-step {}mm | roughness {}..{} | materials {:?}",
            carrier.chunk.chunk,
            elevations.len(),
            min,
            max,
            mean,
            steepest,
            roughness.iter().copied().min().unwrap_or(0),
            roughness.iter().copied().max().unwrap_or(0),
            materials,
        );
    }

    // TODO-GEO-005 evidence: is the elevation step across a chunk boundary the
    // same order of magnitude as the step between two neighbouring cells inside
    // one chunk, or does the ridge reset at every chunk edge?
    let mut by_x: BTreeMap<i32, &_> = BTreeMap::new();
    for carrier in &data.spatial.carrier_adapters {
        if carrier.chunk.chunk.y == 0 && carrier.chunk.chunk.z == 0 {
            by_x.insert(carrier.chunk.chunk.x, carrier);
        }
    }
    let interior_steps = by_x
        .values()
        .flat_map(|carrier| {
            (0..32).flat_map(move |row| {
                (1..32).map(move |column| {
                    (carrier.elevations_mm[row * 32 + column]
                        - carrier.elevations_mm[row * 32 + column - 1])
                        .unsigned_abs()
                })
            })
        })
        .collect::<Vec<_>>();
    let boundary_steps = by_x
        .keys()
        .copied()
        .collect::<Vec<_>>()
        .windows(2)
        .flat_map(|pair| {
            let west = by_x[&pair[0]];
            let east = by_x[&pair[1]];
            (0..32).map(move |row| {
                (east.elevations_mm[row * 32] - west.elevations_mm[row * 32 + 31]).unsigned_abs()
            })
        })
        .collect::<Vec<_>>();
    let mean =
        |values: &[u32]| values.iter().map(|v| f64::from(*v)).sum::<f64>() / values.len() as f64;
    println!(
        "\nTODO-GEO-005: interior step mean {:.0}mm max {}mm ({} pairs) | boundary step mean {:.0}mm max {}mm ({} pairs, {} adjacent chunk pairs)",
        mean(&interior_steps),
        interior_steps.iter().copied().max().unwrap_or(0),
        interior_steps.len(),
        mean(&boundary_steps),
        boundary_steps.iter().copied().max().unwrap_or(0),
        boundary_steps.len(),
        by_x.len().saturating_sub(1),
    );

    // TODO-GEO-004 evidence: is the same-material neighbour rate across a
    // chunk boundary the same order as the interior rate, or does the region
    // partition notice the boundary the way the old per-cell noise never
    // could (having no spatial structure to notice a boundary with at all)?
    let interior_same_material = by_x
        .values()
        .flat_map(|carrier| {
            (0..32).flat_map(move |row| {
                (1..32).map(move |column| {
                    carrier.surface_materials[row * 32 + column]
                        == carrier.surface_materials[row * 32 + column - 1]
                })
            })
        })
        .collect::<Vec<_>>();
    let boundary_same_material = by_x
        .keys()
        .copied()
        .collect::<Vec<_>>()
        .windows(2)
        .flat_map(|pair| {
            let west = by_x[&pair[0]];
            let east = by_x[&pair[1]];
            (0..32).map(move |row| {
                east.surface_materials[row * 32] == west.surface_materials[row * 32 + 31]
            })
        })
        .collect::<Vec<_>>();
    let rate = |values: &[bool]| {
        100.0 * values.iter().filter(|v| **v).count() as f64 / values.len() as f64
    };
    println!(
        "TODO-GEO-004: interior same-material rate {:.1}% ({} pairs) | boundary same-material rate {:.1}% ({} pairs)",
        rate(&interior_same_material),
        interior_same_material.len(),
        rate(&boundary_same_material),
        boundary_same_material.len(),
    );

    // TODO-GEO-006 evidence: does an edge column's structure actually change
    // once its real neighbouring chunk is visible, against the same chunk
    // built with no cross-chunk context at all (the pre-fix behaviour)?
    let decoded: BTreeMap<_, _> = data
        .spatial
        .carrier_adapters
        .iter()
        .map(|snapshot| {
            (
                snapshot.chunk,
                decode_terrain_chunk(snapshot.clone()).unwrap(),
            )
        })
        .collect();
    if let Some((chunk, terrain)) = decoded.iter().next() {
        let aware = TerrainCarrierAdapter::new(*chunk, terrain.clone(), 3, &decoded);
        let isolated = TerrainCarrierAdapter::new(*chunk, terrain.clone(), 3, &BTreeMap::new());
        let changed = aware
            .columns()
            .iter()
            .zip(isolated.columns())
            .filter(|(a, b)| a.structure != b.structure)
            .count();
        println!(
            "TODO-GEO-006: chunk {:?}, {changed}/{} columns changed structure once real neighbouring terrain was visible",
            chunk.chunk,
            aware.columns().len(),
        );
    }

    // TODO-GEO-006 performance evidence: bootstrap wall-clock at increasing
    // active chunk radius, since the fix generates every chunk's terrain
    // before deriving any adapter's columns, and clones each TerrainChunk once
    // into the neighbour map.
    println!("\nTODO-GEO-006: bootstrap wall-clock by active_chunk_radius (Area shape)");
    for radius in [1u8, 2, 3, 4] {
        let mut config = RuntimeConfig::new(7);
        config.active_chunk_radius = radius;
        config.active_chunk_shape = causafera_runtime::ActiveChunkShape::Area;
        let started = Instant::now();
        let _runtime = Runtime::new(config).expect("runtime bootstraps");
        let elapsed = started.elapsed();
        let chunks = (usize::from(radius) * 2 + 1).pow(2);
        println!(
            "  radius {radius} ({chunks:>3} chunks): {:.3} ms ({:.3} us/chunk)",
            elapsed.as_secs_f64() * 1000.0,
            elapsed.as_micros() as f64 / chunks as f64,
        );
    }

    println!("\nmana fields: {}", data.mana.fields.len());
    for field in &data.mana.fields {
        let extent = usize::from(field.extent);
        let volume = extent * extent * extent;
        let cells = &field.intensity;
        let non_zero = cells.iter().filter(|v| **v != 0).count();
        let traced = field.last_change.iter().filter(|t| t.is_some()).count();
        let min = cells.iter().copied().min().unwrap_or(0);
        let max = cells.iter().copied().max().unwrap_or(0);
        let n = cells.len() as f64;
        let mean = cells.iter().map(|v| *v as f64).sum::<f64>() / n;
        let sigma = (cells
            .iter()
            .map(|v| (*v as f64 - mean).powi(2))
            .sum::<f64>()
            / n)
            .sqrt();

        // Neighbour coherence along x, over the 3D lattice.
        let at = |x: usize, y: usize, z: usize| cells[z * extent * extent + y * extent + x] as f64;
        let mut deltas = 0.0;
        let mut count = 0.0;
        for z in 0..extent {
            for y in 0..extent {
                for x in 1..extent {
                    deltas += (at(x, y, z) - at(x - 1, y, z)).abs();
                    count += 1.0;
                }
            }
        }
        // Column sums through z: the natural plan-view reduction for a 2.5D map.
        let mut columns = vec![0i64; extent * extent];
        for z in 0..extent {
            for y in 0..extent {
                for x in 0..extent {
                    columns[y * extent + x] += cells[z * extent * extent + y * extent + x];
                }
            }
        }
        let column_non_zero = columns.iter().filter(|v| **v != 0).count();
        println!(
            "  chunk {:?} extent {} volume {} | intensity {}..{} sigma {:.0} | non-zero {}/{} ({:.1}%) | traced cells {} | column non-zero {}/{}",
            field.chunk.chunk,
            extent,
            volume,
            min,
            max,
            sigma,
            non_zero,
            cells.len(),
            100.0 * non_zero as f64 / n,
            traced,
            column_non_zero,
            columns.len(),
        );
        println!(
            "     mean|delta| {:.0}  ratio {:.2} (1.13 = white noise, <0.3 = smooth field)",
            deltas / count,
            (deltas / count) / sigma.max(1.0),
        );
    }
}

// Appended: what the mana field holds, and whether it is a coherent field or noise.
