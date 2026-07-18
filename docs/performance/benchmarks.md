# Benchmarks

This document defines benchmark methodology and requirements for Causafera.

## Benchmark Requirements

Every performance-sensitive system must have:

- defined workload;
- reproducible setup;
- warm-up phase;
- measurement duration;
- statistical reporting (mean, median, stddev);
- hardware specification;
- deterministic mode verification.

## Workload Definitions

### Simulation Throughput Benchmark

Measure simulated days per wall-clock second under:

- minimal world (empty geography);
- small settlement (100 residents);
- medium city (10,000 residents);
- large region (100,000 residents).

### Observer Overhead Benchmark

Measure performance with:

- no observer;
- idle observer;
- normal UI load;
- heavy inspection load.

### Explanation Query Benchmark

Measure Explanation Engine latency for:

- simple entity lookup;
- causal trace reconstruction (10 hops);
- causal trace reconstruction (100 hops);
- phenomenon classification.

## GPU Benchmarks

Every GPU kernel requires:

1. CPU reference implementation;
2. Defined inputs;
3. Defined outputs;
4. Correctness tests;
5. Transfer-inclusive benchmark;
6. Workload crossover measurement.

## Reporting

Benchmark results must be:

- stored in version control;
- compared across commits;
- flagged on regression;
- reproducible on reference hardware.

## Phase 24 Executable Baseline

The headless runtime and lab report wall-clock duration for bounded runs and numeric activity counts. Duration is excluded from canonical replay equality and is not a throughput or scale claim. The full benchmark matrix, RSS measurement, statistical repetition, and reference-hardware reports remain `TODO-PERF-001`.

## Observer Transport Diagnostic

Run `cargo run --release -p causafera-runtime --example observer_overhead`. The bounded harness
warms up for 16 ticks and measures 128 ticks in headless, idle, normal (one query/tick), and heavy
(32 queries/tick) modes. A 2026-07-13 local run measured 357–368 ms across the four modes and
encoded 0, 0, 18,198, and 582,336 bytes respectively. These are environment-specific diagnostics,
not throughput or scale claims; the statistical/RSS framework remains `TODO-PERF-001`.

## Phase 26 UI Session Diagnostic

The desktop session uses a capacity-one latest-state stream and the client retains at most 96
timeline samples. On the 2026-07-13 local release build, the negotiation, initial runtime snapshot,
and four-tick delta test completed in 0.01 s, the bounded world projection test completed below the
test harness's 0.01 s display precision, and the replay-verified 192-tick populated
control/intervention Explanation run completed in 2.19 s. Build time, WebKit startup, and browser
paint are excluded. These are single environment-specific diagnostics, not statistical latency,
throughput, or population-scale claims; `TODO-PERF-001` remains pending.

## Reference Hardware

- Linux;
- x86-64;
- Ryzen 9 7950X3D-class CPU;
- 64 GB RAM;
- RTX 4080 Super-class GPU;
- NVIDIA CUDA.

## Related Documents

- `docs/performance/philosophy.md` - Performance philosophy
- `docs/performance/metrics.md` - Metric definitions
