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

## Conserved Thermal Carrier Benchmark

The thermal carrier workload is a fully populated `CHUNK_SIZE³` field plus one same-chart neighbor
chunk, with a center-cell reservoir injection. Required measurements:

- wall time per tick for `ThermalEvolutionSystem`;
- cells updated per tick;
- peak memory for thermal working buffers;
- snapshot bytes per chunk;
- provenance event count growth per tick;
- observer query payload bytes for thermal summary.

These are environment-specific baselines; no absolute latency claim is made without reference
hardware. The benchmark harness, repeated measurements, and statistical reporting remain
`TODO-PERF-001`.

## Methodology Gaps Found by the Performance Baseline Investigation

`plans/performance-baseline-and-digest-cost.md` (Draft) swept the runtime's validated configuration
space and audited every existing benchmark/diagnostic tool against the requirements above. None of the
shipped tools (`benchmark.rs`, `material_surface_loop_benchmark`, `observer_overhead`, `extent_bench.rs`,
`mana_gate_calibration.rs`) take repeated measurements — each reports a single timed run, not the
mean/median/stddev this document already requires. This is not hypothetical: a single local run of
`material_surface_loop_benchmark` produced a negative observer-overhead delta, which is only possible
as single-run measurement noise. `MaterialSurfaceLoopBenchmarkMeasurement`'s `peak_rss_kib`/
`steady_rss_kib` also read `/proc/self/status` for the whole process while `run_material_surface_loop_benchmark`
runs both measured modes sequentially in that same process, so the second mode's reported peak RSS
includes the first mode's already-torn-down `Runtime` — confirmed locally, where the second mode's
peak RSS was measured strictly greater than the first's by construction, not by actual difference in
memory use. The CI job named "Benchmarks and Long Runs" (`.github/workflows/benchmarks.yml`) runs only
`cargo test --release -- --ignored`; it does not capture, store, or compare any benchmark number, so
the "stored in version control, compared across commits, flagged on regression" requirement above is
currently unmet by any CI job despite the job's name. See that plan's Implementation stages for the
proposed corrected harness (Wave 1) and CI capture (Wave 4, capture only — no regression gate yet).

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
