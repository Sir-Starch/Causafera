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

`plans/performance-baseline-and-digest-cost.md` swept the runtime's validated configuration space and
audited every existing benchmark/diagnostic tool against the requirements above. Two methodology
defects were found and are now fixed; the rest of the Reporting requirements are still open, and this
section states which is which so the job's name is not mistaken for the guarantee.

**Fixed.** None of the previously shipped tools (`benchmark.rs`, `material_surface_loop_benchmark`,
`observer_overhead`, `extent_bench.rs`, `mana_gate_calibration.rs`) took repeated measurements — each
reported a single timed run, not the mean/median/stddev this document already requires. That was not
hypothetical: two independent single runs of `material_surface_loop_benchmark` each produced a
*negative* observer-overhead delta, which is only possible as single-run noise, since the compared
mode does strictly more work. Separately, `MaterialSurfaceLoopBenchmarkMeasurement`'s `peak_rss_kib`/
`steady_rss_kib` read `/proc/self/status` for the whole process while both measured modes ran
sequentially in that one process, so the second mode's reported peak RSS included the first mode's
already-torn-down `Runtime` — its figure was greater than the first's by construction rather than by
any real difference in memory use. `crates/causafera-runtime/examples/performance_baseline.rs`
replaces both: N=20 repetitions with the case order cyclically rotated per pass, mean/median/stddev
plus every raw sample, and one subprocess per (case, repetition) pair so each RSS reading covers one
`Runtime` and nothing else. It also records logical core count, `rustc --version` and profile
alongside every report, and states in its own output that the environment is not the reference
hardware below.

**Still open.** The CI job named "Benchmarks and Long Runs"
(`.github/workflows/benchmarks.yml`) now runs that harness and stores its output as an artifact named
for the commit SHA, which satisfies "stored" — as a build artifact under the run's retention, not in
version control — and makes "compared across commits" *possible* for the first time. It does not do
the comparing: there is deliberately no threshold and no regression flag, because a threshold chosen
before any historical series exists would be a guess rather than a measurement. "Flagged on
regression" and "reproducible on reference hardware" therefore remain unmet, and `TODO-PERF-001`
stays open on exactly those. The job does fail if the harness's boundary sweep finds a configuration
`RuntimeConfig::validate` accepted that then exceeded the cognition cue cap at a tick, which is a
soundness bug rather than a performance threshold.

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
