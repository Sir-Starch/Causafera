use std::process::ExitCode;

use causafera_runtime::{Runtime, measure_import_wall_time, production_loop_config};

const SEED: u64 = 2026;
const WARMUP_TICKS: u64 = 4;
const MEASUREMENT_TICKS: u64 = 32;
const INNER_ITERATIONS: u32 = 100;
const REPETITIONS: usize = 10;

fn main() -> ExitCode {
    let mut runtime =
        Runtime::new(production_loop_config(SEED)).expect("runtime bootstrap must succeed");
    runtime
        .run_ticks(WARMUP_TICKS)
        .expect("warmup ticks must execute");
    runtime
        .run_ticks(MEASUREMENT_TICKS)
        .expect("measurement ticks must execute");
    let snapshot = runtime.export_snapshot().expect("snapshot must export");

    let mut samples: Vec<u128> = Vec::with_capacity(REPETITIONS);
    for _ in 0..REPETITIONS {
        let mean_ns = measure_import_wall_time(&snapshot, INNER_ITERATIONS)
            .expect("import benchmark must succeed");
        samples.push(mean_ns);
    }

    let batch_count = snapshot.thermal.field_set.batch_sequence;
    let total_cells: usize = snapshot
        .thermal
        .field_set
        .fields
        .iter()
        .map(|field| field.energy.len())
        .sum();
    let total_transfer_receipts = snapshot.thermal.transfer_receipts.len();
    let toolchain = json_escape(&rustc_version());
    let hardware = json_escape(&hardware_description());
    let stats = statistics(&samples);

    println!(
        r#"{{"mean_import_ns":{mean},"median_import_ns":{median},"stddev_import_ns":{stddev},"min_import_ns":{min},"max_import_ns":{max},"repetitions":{repetitions},"inner_iterations":{inner_iterations},"batch_count":{batch_count},"total_cells":{total_cells},"total_transfer_receipts":{total_transfer_receipts},"measurement_ticks":{MEASUREMENT_TICKS},"seed":{SEED},"toolchain":"{toolchain}","hardware":"{hardware}","deterministic_mode":true}}"#,
        mean = stats.mean,
        median = stats.median,
        stddev = stats.stddev,
        min = stats.min,
        max = stats.max,
        repetitions = REPETITIONS,
        inner_iterations = INNER_ITERATIONS,
    );
    ExitCode::SUCCESS
}

fn json_escape(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for c in value.chars() {
        match c {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\x08' => escaped.push_str("\\b"),
            '\x09' => escaped.push_str("\\t"),
            '\x0A' => escaped.push_str("\\n"),
            '\x0C' => escaped.push_str("\\f"),
            '\x0D' => escaped.push_str("\\r"),
            c if c as u32 <= 0x1F => escaped.push_str(&format!("\\u{:04x}", c as u32)),
            c => escaped.push(c),
        }
    }
    escaped
}

fn rustc_version() -> String {
    std::process::Command::new("rustc")
        .arg("--version")
        .output()
        .ok()
        .and_then(|output| {
            if output.status.success() {
                String::from_utf8(output.stdout)
                    .ok()
                    .map(|s| s.trim().to_string())
            } else {
                None
            }
        })
        .unwrap_or_else(|| "unknown".to_string())
}

fn hardware_description() -> String {
    std::fs::read_to_string("/proc/cpuinfo")
        .ok()
        .and_then(|content| {
            content
                .lines()
                .find(|line| line.starts_with("model name"))
                .and_then(|line| line.split(':').nth(1))
                .map(|s| s.trim().to_string())
        })
        .unwrap_or_else(|| {
            std::process::Command::new("uname")
                .arg("-m")
                .output()
                .ok()
                .and_then(|output| {
                    if output.status.success() {
                        String::from_utf8(output.stdout)
                            .ok()
                            .map(|s| s.trim().to_string())
                    } else {
                        None
                    }
                })
                .unwrap_or_else(|| "unknown".to_string())
        })
}

struct Statistics {
    mean: f64,
    median: u128,
    stddev: f64,
    min: u128,
    max: u128,
}

fn statistics(samples: &[u128]) -> Statistics {
    assert!(
        !samples.is_empty(),
        "benchmark requires at least one repetition"
    );
    let sorted = {
        let mut s = samples.to_vec();
        s.sort_unstable();
        s
    };
    let min = sorted[0];
    let max = sorted[sorted.len() - 1];
    let mean = sorted.iter().map(|&value| value as f64).sum::<f64>() / sorted.len() as f64;
    let median = if sorted.len() % 2 == 1 {
        sorted[sorted.len() / 2]
    } else {
        let mid = sorted.len() / 2;
        (sorted[mid - 1] + sorted[mid]) / 2
    };
    let variance = if sorted.len() > 1 {
        let sum_sq_diff: f64 = sorted
            .iter()
            .map(|v| {
                let diff = *v as f64 - mean;
                diff * diff
            })
            .sum();
        sum_sq_diff / (sorted.len() - 1) as f64
    } else {
        0.0
    };
    let stddev = variance.sqrt();
    Statistics {
        mean,
        median,
        stddev,
        min,
        max,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn statistics_reports_expected_summary() {
        let stats = statistics(&[1, 2, 3, 4]);
        assert_eq!(stats.mean, 2.5);
        assert_eq!(stats.median, 2);
        assert!((stats.stddev - (5.0_f64 / 3.0_f64).sqrt()).abs() < 1e-12);
        assert_eq!(stats.min, 1);
        assert_eq!(stats.max, 4);
    }

    #[test]
    fn json_escape_preserves_json_string_syntax() {
        assert_eq!(json_escape("cpu \\\"model"), "cpu \\\\\\\"model");
        let all_ctrl: String = (0..=0x1F).map(|c| c as u8 as char).collect();
        let escaped = json_escape(&all_ctrl);
        for c in escaped.chars() {
            assert!(c as u32 > 0x1F, "control character found: {:?}", c);
        }
        assert!(escaped.contains("\\u0000"));
        assert!(escaped.contains("\\u0001"));
        assert!(escaped.contains("\\u001f"));
        assert!(escaped.contains("\\b"));
        assert!(escaped.contains("\\t"));
        assert!(escaped.contains("\\n"));
        assert!(escaped.contains("\\f"));
        assert!(escaped.contains("\\r"));
        assert_eq!(json_escape("ASCII / Русский"), "ASCII / Русский");
    }
}
