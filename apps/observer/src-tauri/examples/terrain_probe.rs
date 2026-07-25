//! Development-only probe: what the terrain carrier actually holds per chunk.
//!
//! Used to size the observer projection proposed in `plans/observer-terrain-raster-map.md`.
//! Prints nothing that is not read straight from a real runtime snapshot.

use std::collections::BTreeMap;

use causafera_runtime::{Runtime, RuntimeConfig};

fn main() {
    let mut config = RuntimeConfig::new(7);
    config.actor_count = 8;
    config.sensor_count = 2;
    config.bootstrap_population = 512;
    let mut runtime = Runtime::new(config).expect("runtime");
    runtime.run_ticks(48).expect("ticks");
    let data = runtime.export_snapshot().expect("snapshot");

    println!("chunks: {}", data.spatial.carrier_adapters.len());
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
}
