use std::hint::black_box;
use std::time::Instant;

use causafera_observer_api::ObserverQuery;
use causafera_observer_wire::{ProtocolHandler, encode_query};
use causafera_runtime::Runtime;

const WARMUP_TICKS: u64 = 16;
const MEASURE_TICKS: u64 = 128;
const HEAVY_QUERIES_PER_TICK: u64 = 32;

fn main() {
    for _ in 0..2 {
        measure("headless", false, 0);
        measure("idle", true, 0);
        measure("normal", true, 1);
        measure("heavy", true, HEAVY_QUERIES_PER_TICK);
    }
}

fn measure(label: &str, connected: bool, queries_per_tick: u64) {
    let mut runtime = Runtime::from_seed(0x0b5e_7e12).expect("benchmark runtime");
    runtime.run_ticks(WARMUP_TICKS).expect("warmup");
    let started = Instant::now();
    let mut encoded_bytes = 0_u64;
    for tick in 0..MEASURE_TICKS {
        let snapshot = runtime.tick().expect("measured tick");
        if connected {
            let mut handler = ProtocolHandler::new(snapshot.time);
            handler.set_runtime_snapshot(&snapshot.observer_snapshot());
            for ordinal in 0..queries_per_tick {
                let query = ObserverQuery::runtime_summary((tick << 32) | ordinal);
                let response = handler
                    .handle_query(&encode_query(&query))
                    .expect("observer query");
                encoded_bytes += response.len() as u64;
                black_box(response);
            }
        }
    }
    let elapsed = started.elapsed();
    println!(
        "mode={label} ticks={MEASURE_TICKS} elapsed_ns={} encoded_bytes={encoded_bytes}",
        elapsed.as_nanos()
    );
}
