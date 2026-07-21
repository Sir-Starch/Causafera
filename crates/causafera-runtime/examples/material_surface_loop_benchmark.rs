use std::process::ExitCode;

use causafera_runtime::{
    MaterialSurfaceLoopBenchmarkMeasurement, run_material_surface_loop_benchmark,
};

fn main() -> ExitCode {
    match run_material_surface_loop_benchmark(Default::default()) {
        Ok(report) => {
            println!("benchmark_version={}", report.version);
            println!("seed={}", report.config.seed);
            println!("warmup_ticks={}", report.config.warmup_ticks);
            println!("measurement_ticks={}", report.config.measurement_ticks);
            print_measurement(&report.observer_off);
            print_measurement(&report.world_chunks_query);
            println!(
                "world_chunks_observer_overhead_ns={}",
                report.world_chunks_observer_overhead_ns
            );
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("material_surface_loop_benchmark_error={error}");
            ExitCode::FAILURE
        }
    }
}

fn print_measurement(measurement: &MaterialSurfaceLoopBenchmarkMeasurement) {
    let mode = measurement.mode.label();
    println!("{mode}_tick_elapsed_ns={}", measurement.tick_elapsed_ns);
    println!(
        "{mode}_mean_tick_elapsed_ns={}",
        measurement.mean_tick_elapsed_ns
    );
    print_memory_kib(mode, "peak_rss_kib", measurement.peak_rss_kib);
    print_memory_kib(mode, "steady_rss_kib", measurement.steady_rss_kib);
    println!(
        "{mode}_provenance_event_growth={}",
        measurement.provenance_event_growth
    );
    println!(
        "{mode}_encoded_snapshot_bytes={}",
        measurement.encoded_snapshot_bytes
    );
    println!(
        "{mode}_observer_response_bytes={}",
        measurement.observer_response_bytes
    );
    println!(
        "{mode}_promoted_actor_count={}",
        measurement.promoted_actor_count
    );
    println!(
        "{mode}_material_surface_site_count={}",
        measurement.material_surface_site_count
    );
    println!(
        "{mode}_material_contact_count={}",
        measurement.material_contact_count
    );
    println!(
        "{mode}_mana_material_transition_count={}",
        measurement.mana_material_transition_count
    );
}

fn print_memory_kib(mode: &str, metric: &str, value: Option<u64>) {
    match value {
        Some(value) => println!("{mode}_{metric}={value}"),
        None => println!("{mode}_{metric}=unavailable"),
    }
}
