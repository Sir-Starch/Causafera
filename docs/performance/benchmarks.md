# Benchmarks

This document defines benchmark methodology and requirements for Ontopolis.

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

Run `cargo run --release -p ontopolis-runtime --example observer_overhead`. The bounded harness
warms up for 16 ticks and measures 128 ticks in headless, idle, normal (one query/tick), and heavy
(32 queries/tick) modes. A 2026-07-13 local run measured 357–368 ms across the four modes and
encoded 0, 0, 18,198, and 582,336 bytes respectively. These are environment-specific diagnostics,
not throughput or scale claims; the statistical/RSS framework remains `TODO-PERF-001`.

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
