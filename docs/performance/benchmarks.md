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

The headless runtime and lab report wall-clock duration for bounded runs and numeric activity counts. Duration is excluded from canonical replay equality and is not a throughput or scale claim. Statistical repetition and RSS measurement were delivered by `plans/performance-baseline-and-digest-cost.md`, which closed `TODO-PERF-001`; the full benchmark matrix and reference-hardware reports were carried forward to `TODO-PERF-003`.

## Bounded Bootstrap Closure Benchmark

`run_bootstrap_closure_benchmark` measures the canonical production bootstrap at one bounded
envelope: nine active chunks (`Area`, radius 1), bootstrap population 512, eight promoted actors,
one sensor aperture each. It follows the Reporting requirements above as far as they can be met
without reference hardware — four unmeasured warm-up repetitions, twenty measured repetitions per
distribution, and mean, median, population standard deviation, minimum, maximum **and every raw
sample** retained on the report rather than a single collapsed number.

The observer control and its counterpart are interleaved rather than run as two blocks, so drift in
machine state over the run biases both alike instead of landing entirely on whichever went second.
That is what makes the encoding overhead resolvable at all: measured as two sequential blocks it sat
inside run-to-run noise, and interleaved at twenty samples the two distributions separate.

It defines no threshold and flags no regression, for the reason stated above: a threshold chosen
before a historical series exists is a guess. `TODO-PERF-003` still owns that and reference hardware.

## Observer Transport Diagnostic

Run `cargo run --release -p causafera-runtime --example observer_overhead`. The bounded harness
warms up for 16 ticks and measures 128 ticks in headless, idle, normal (one query/tick), and heavy
(32 queries/tick) modes. A 2026-07-13 local run measured 357–368 ms across the four modes and
encoded 0, 0, 18,198, and 582,336 bytes respectively. These are environment-specific diagnostics,
not throughput or scale claims; the statistical/RSS framework has since landed with `TODO-PERF-001`,
and this particular diagnostic predates it — it is a single run, not a distribution.

## Phase 26 UI Session Diagnostic

The desktop session uses a capacity-one latest-state stream and the client retains at most 96
timeline samples. On the 2026-07-13 local release build, the negotiation, initial runtime snapshot,
and four-tick delta test completed in 0.01 s, the bounded world projection test completed below the
test harness's 0.01 s display precision, and the replay-verified 192-tick populated
control/intervention Explanation run completed in 2.19 s. Build time, WebKit startup, and browser
paint are excluded. These are single environment-specific diagnostics, not statistical latency,
throughput, or population-scale claims. `TODO-PERF-001` is closed on the criteria it met; the
remainder it did not meet was carried to `TODO-PERF-003`.

## Conserved Thermal Carrier Benchmark

The thermal carrier workload is a fully populated `CHUNK_SIZE³` field plus one same-chart neighbor
chunk, with a center-cell reservoir injection. `TODO-THERMAL-002` added a bounded per-cell material
exchange term to the same `ThermalEvolutionSystem` batch (one material surface per active chunk,
exchanging every tick by production default); this workload should include it rather than measuring
face diffusion alone. Required measurements:

- wall time per tick for `ThermalEvolutionSystem`, with material exchange active;
- cells updated per tick;
- peak memory for thermal working buffers;
- snapshot bytes per chunk, including the material exchange term now added to every participating
  cell's transfer receipt;
- provenance event count growth per tick, including `MATERIAL_SURFACE_THERMAL_EXCHANGE_EVENT_KIND`;
- observer query payload bytes for thermal summary.

These are environment-specific baselines; no absolute latency claim is made without reference
hardware. `TODO-THERMAL-002`'s material exchange term was not benchmarked in that tranche — this
section names the requirement, not a measurement already taken. A benchmark harness with warm-up,
repeated measurements, and mean/median/stddev reporting over retained raw samples now exists for the
bootstrap closure workload; extending it to the remaining workloads is `TODO-PERF-003`.

## Conserved Hydrology Benchmark

`cargo run --release -p causafera-observer --example hydrology_bench` measures the
six workloads `plans/hydrology.md` defines. Every workload is a
production-bootstrapped runtime with an explicit grid metric, substrate, and
forcing schedule — no fixture constructors — and every measured run asserts that
each retained conservation receipt closed at exactly zero before its timing is
reported. A run that lost water is not a fast run.

### Environment

This is not a claim about reference hardware, but the CPU and memory happen to
match the reference specification below.

| | |
| --- | --- |
| Commit | `4f56b5d` |
| Toolchain | rustc 1.97.1 (8bab26f4f 2026-07-14) |
| Profile | `release`, `opt-level = 3`, `lto = "thin"` |
| OS | Linux 7.1.5-arch1-2 |
| CPU | AMD Ryzen 9 7950X3D, 32 logical cores |
| Memory | 64 GB |
| Repetitions | 4 warm-up ticks discarded, then 10 measured repetitions per workload |

### Results

Timings are whole-run milliseconds over the stated tick count. `vertical`,
`faces`, and `bounds` are the distinct vertical groups, interior faces, and
exterior faces the last measured batch actually evaluated, counted from its
transfer receipts rather than estimated. `levels` is the detail level the
engine's own resolution field chose, by chunk count.

| Workload | ticks | cells | edges | vertical | faces | receipts | mean ms | median | stddev | min | max | levels |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| 1. one chunk, vertical only | 24 | 1,024 | 889 | 1,024 | 0 | 2,932 | 2,489.401 | 2,480.969 | 32.446 | 2,445.802 | 2,538.367 | L3 x1 |
| 2. three line chunks, seams | 24 | 3,072 | 2,670 | 3,072 | 639 | 10,068 | 8,848.186 | 8,815.303 | 212.797 | 8,460.306 | 9,218.548 | L3 x3 |
| 3. nine chunks, full routing | **15 of 24** | 9,216 | 8,048 | 9,216 | 2,111 | 30,625 | 15,690.245 | 15,672.230 | 292.701 | 15,368.794 | 16,448.757 | L3 x9 |
| 5. snapshot export | 6 (3+ batches) | — | — | — | — | — | 8.521 | 8.560 | 0.514 | 7.452 | 9.248 | — |
| 5. snapshot import | 6 (3+ batches) | — | — | — | — | — | 154.245 | — | 2.277 | — | — | — |
| 6. run length 100 | 100 | 1,024 | 889 | 858 | 192 | 2,227 | 31,699.029 | 31,642.794 | 252.006 | 31,377.843 | 32,156.760 | L3 x1 |

Workload 1 sets both transmissivities to zero, which is the honest way to ask for
a vertical-only workload: no conductance means no lateral face is evaluated, and
the measured zero faces confirm it rather than assume it. Workload 2's 639
interior faces include both active seam faces.

### The export cap is the binding limit, not per-tick cost

Workload 3 could not reach 24 ticks. At tick 15 the whole-tick staging
transaction refused a tick whose result would have outgrown the 256 MiB snapshot
export cap, rolled it back entire, and left the session at its last committed
state. That is the designed behaviour — a state that cannot be exported is not
accepted — and it is also the most useful number this harness produces:

| Configuration | cells | ticks committed before the cap refused one | wall s | ms/tick |
| --- | ---: | ---: | ---: | ---: |
| one chunk | 1,024 | 221 | 160.4 | 725.788 |
| three line chunks | 3,072 | 59 | 45.7 | 774.687 |
| nine chunks | 9,216 | 15 | 15.1 | 1,009.558 |

These ceilings are exact and deterministic — the same seed reaches the same tick
every time — so they are measured once per configuration rather than ten times.
Repeating a value with no variance spends minutes to report the same number.

**The plan's 1,000- and 10,000-tick sweeps are therefore not reachable and were
not run.** Nothing bounds them but this ceiling: even one chunk stops at 221. The
100-tick point was measured, on one chunk, because that is the only configuration
with headroom for it.

Per-tick cost also grows with accumulated history rather than staying flat. The
same one-chunk configuration costs about 104 ms/tick over 24 ticks, 317 ms/tick
over 100, and 726 ms/tick over 221 — the causal trace store, not the solver, is
what the session is paying for. No threshold is declared and no regression is
flagged: this is the first hydrology series and a threshold chosen before one
exists would be a guess.

### Re-measured after the import fail-closed additions

The snapshot import path gained agreement checks against the persisted
configuration and a trace-existence pass over every bucket, edge, resolution
entry and retained receipt (`plans/hydrology.md` §11, V25). The whole series was
re-run on a quiet machine at commit `4a99c26`+ to see what it cost:

| Workload | `4f56b5d` mean ms | after mean ms | change |
| --- | ---: | ---: | ---: |
| 1. one chunk | 2,489.401 | 2,375.303 | −4.6% |
| 2. three line chunks | 8,848.186 | 8,944.305 | +1.1% |
| 3. nine chunks | 15,690.245 | 15,110.326 | −3.7% |
| 5. snapshot export | 8.521 | 7.776 | −8.7% |
| 5. snapshot import | 154.245 | 153.108 | −0.7% |
| 6. run length 100 | 31,699.029 | 31,002.075 | −2.2% |

Every export-cap ceiling is identical — 221, 59 and 15 ticks — which is expected
of a deterministic bound and is the check that the additions changed no accepted
state. Import, the figure the additions actually touch, moved by less than its
own stddev. The scope of that claim is what was measured: workload 5 imports a
one-chunk session, so the anchor pass is 4,096 lookups against 153 ms of import.
The pass is linear in cells and retained receipts, and no nine-chunk import was
measured.

### The fine/coarse comparison was not obtained

`plans/hydrology.md` accepts the retained-fine/coarse design as a performance
architecture only if a coarse workload evaluates strictly fewer vertical groups
and interior faces than a fine one. **That comparison could not be made, and the
design is not accepted on this evidence.** Two independent reasons:

1. **The level is not configurable.** `ResolutionPolicy` is a compiled constant
   rather than a setting, deliberately, and a hydrology policy with a lower
   maximum refuses the tick rather than coarsening it. A benchmark cannot ask for
   a coarse world; it can only report what the engine chose. The engine placed
   every resident chunk at level 3 in all six workloads, so there is no fine
   chunk to compare a coarse one against.

2. **Coarse grouping did not engage even at level 3.** A block groups only cells
   whose constitutive identity matches exactly, and that identity includes the
   substrate's derived conductances, which are derived from per-cell terrain
   roughness. On production terrain the roughness varies cell to cell, so the
   nine-chunk workload at level 3, block edge 8, still evaluated **9,216 vertical
   groups over 9,216 cells** — one per cell, no reduction at all.

The second point is the design working as specified, not a defect:
`HydrologyConstitutiveKey` requires exact equality because aggregating cells that
differ in any parameter would invent an averaged cell that none of them is. The
consequence is that on heterogeneous ground the coarse path costs the same as the
fine one plus the cost of discovering that it cannot group. Whether that path
pays for itself needs either homogeneous substrate or a tolerance the current
contract deliberately refuses; measuring it is future work and this section states
so rather than reporting the level number as if it were a saving.

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
regression" and "reproducible on reference hardware" therefore remain unmet. `TODO-PERF-001` was
closed on the criteria it did meet and those two were carried forward as `TODO-PERF-003`, which is
where they stay open. The job does fail if the harness's boundary sweep finds a configuration
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
