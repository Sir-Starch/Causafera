# Performance Baseline and Digest Cost ExecPlan

**Status:** Accepted and implemented; all five waves landed, each with its own verified checkpoint
(see Progress), including the differential oracle test Wave 3's INV-007/INV-038 sensitivity required.
`TODO-PERF-001` is closed; the two follow-ups this plan deliberately did not close continue as
`TODO-PERF-002` (`physical_state_digest`'s thermal-receipt growth) and `TODO-PERF-003` (regression
flagging and reference hardware).
**Revision 7**, incorporating three independent review passes that corrected several claims in
earlier drafts, plus what implementing all five waves established — see Decision log for what changed
and why across all revisions.

## Goal

Close `TODO-PERF-001` from a one-line intent ("Benchmark harness. Can measure ticks/second, memory,
active sets") into an evidence-backed baseline: identify what is currently representative enough to
benchmark, root-cause the concrete per-tick cost drivers actually measured in this investigation, fix
the validation gap that lets `RuntimeConfig::validate()` accept configurations the runtime cannot
execute, and lay out what to defer until richer workloads exist.

## Context

The Detailed Development rebaseline and the domain coverage matrix both name "representative
performance" as an explicit, still-open gap for the `Simulation runtime` capability
(`docs/ontology/domain-coverage-matrix.md`). `docs/simulation/long-run-experiments.md` already
flags that "provenance grows with accepted state transitions, so substantially larger workloads
require `TODO-PERF-001` benchmarks and later persistence/compaction work" — a known unknown, not yet
measured. This plan measures it.

The existing benchmark surface (`crates/causafera-runtime/src/benchmark.rs`, the
`material_surface_loop_benchmark` example, `observer_overhead` example, and the
`apps/observer/src-tauri/examples/*` diagnostic tools such as `extent_bench.rs` and
`mana_gate_calibration.rs`) already reports `tick_elapsed_ns`, RSS, provenance growth, and encoded
snapshot bytes for small, hand-picked configurations. What was missing before this plan: nobody had
swept the runtime's own validated configuration space, isolated which part of tick cost the digests
account for, or checked whether individually-valid `RuntimeConfig` fields compose into a runnable
tick.

### Investigation method

All numbers below come from throwaway example binaries built against this branch's `causafera-runtime`
(`cargo build --release -p causafera-runtime`), run locally, and then deleted — they are **not**
checked in and therefore not INV-018-reproducible yet. Wave 1 below checks in a harness that
reproduces them durably; until that lands, treat every number in this section as a provisional,
single-run local measurement, not a scale or throughput claim. Local hardware: 32 logical cores,
64 GB RAM, `rustc 1.97.1`, release profile — not the reference hardware in
`docs/performance/benchmarks.md` (Ryzen 9 7950X3D-class, RTX 4080 Super-class); no cross-hardware
comparison is claimed.

The probes called `RuntimeConfig::new`, varied one or more fields, ran `warmup_ticks` then measured
wall time across `Runtime::tick()` calls directly (not `export_snapshot`/`assemble_envelope`, which
were timed separately). One probe added temporary `AtomicU64` timers around the two digest calls
inside `RuntimeState::snapshot` (`crates/causafera-runtime/src/runtime.rs`), built, ran, and was
reverted immediately after.

An independent review pass re-ran the shipped `material_surface_loop_benchmark` separately and
reproduced a second physically-impossible negative observer-overhead delta (−187,351 ns, distinct
from this investigation's own −28,469 ns run) — two independent single-shot runs, two different
negative numbers, which is exactly the failure mode expected from a benchmark with no statistical
repetition (see the existing-infrastructure gaps below).

### What is already bounded (and holds up)

`RuntimeConfig::validate()` (`crates/causafera-runtime/src/config.rs`) hard-rejects `chunk_extent < 3`,
`active_chunk_radius > 4`, `actor_count > 128`, `sensor_count > 16`, and `bootstrap_population >
10_000`. This is a real, enforced instance of the "bounded active state" principle in
`docs/performance/philosophy.md` — it is not aspirational.

Measured: `bootstrap_population` (aggregate, non-promoted population) has no measurable per-tick cost
at the current maturity level. Sweeping 8 → 10,000 at fixed `actor_count=8` produced identical
`ms_per_tick` (~0.31 ms) and identical trace-event growth. This is expected at the historical-
bootstrap domain's current M1-with-a-narrow-M2-path maturity (`docs/ontology/domain-coverage-matrix.md`):
aggregate population is not iterated per tick.

`actor_count` (promoted/active actors) is cheap across the range this investigation could actually
execute (see Finding 1 for why that range is narrower than the validated bound): 1 → 48 actors at
`sensor_count=1` produced flat `ms_per_tick` (~0.30–0.32 ms). Perception and cognition are not the
bottleneck at this scale.

### Finding 1 — `RuntimeConfig::validate()` accepts actor/sensor/surface combinations the runtime cannot execute, and the exact cue-count formula is not yet nailed down

`crates/causafera-cognition/src/scene.rs` hard-caps `MAX_SCENE_CUES = 64` per actor per tick.
`actor_perception_step` (`crates/causafera-runtime/src/actors/perception.rs`) builds each actor's
perceptual signal batch from **two** sources, both re-sampled once per sensor aperture:

1. promoted actors' physical objects (`physical_signals(objects)`, scaling with `actor_count` —
   whether the perceiving actor's own object is excluded or included is not established by this
   investigation; the 65-cues-at-`actor_count=64` data point in the table below is consistent with it
   being included, which is one reason Finding 1 does not commit to a closed-form formula), and
2. every material surface with `contact_count > 0`
   (`material_surface_physical_signals`, `crates/causafera-runtime/src/material_surface.rs:1091`,
   called directly as `actor_perception_step`'s `material_signals` argument at
   `crates/causafera-runtime/src/actors/perception.rs:171`).

Source 2 is **not** a function of `actor_count` or `sensor_count` at all — it grows with how many
active chunks' material surfaces have registered actor contact over the run so far, bounded above by
`active_chunk_count` (a function of `active_chunk_radius`/`active_chunk_shape`, up to 81 chunks at
`radius=4`, `Area`), and it accumulates as the run progresses, not just at construction time.

Swept `actor_count`/`sensor_count` independently at the earliest point contact accumulation could
matter (`bootstrap_population=512`, `chunk_extent=3`, 8 warm-up ticks, `active_chunk_radius=0`):

| sensor_count | max actor_count tested that still ticks | first tested actor_count that fails | cue count at first failure |
|---|---|---|---|
| 1 | 48 | 64 | 65 |
| 2 | 24 | 32 | 66 |
| 4 | 8 (passes); next tested value (8) at sensor_count=8 fails | 8 (at sensor_count=8) | 72 |
| 16 | fails at every `actor_count ≥ 8` tested | 8 | 88 |

**An earlier revision of this plan asserted a specific closed-form formula (approximately
`sensor_count × (actor_count − 1 + contacted_surface_count)`) for the cue count.** Independent review
correctly pointed out this does not match the data: at `actor_count=64, sensor_count=1`, the observed
failure is at cue count 65, which is `actor_count + 1`, not `sensor_count × (actor_count − 1)` (which
would be 63). The sweep above only tested sparse, discrete `actor_count` values ([1, 2, 4, 8, 16, 24,
32, 48, 64, 96, 128], not every integer), so the exact boundary between the last known-passing value
(48) and the first known-failing value (64) was never located, and no arithmetic formula in this plan
should be trusted as exact — the actor's own object, warm-up-tick contact timing, and per-seed
variation in when a surface first registers `contact_count > 0` all plausibly contribute, and this
investigation did not isolate which. **This plan does not commit to a formula.** Wave 2 (below) is
scoped accordingly: derive the validation bound from the actual signal-construction code paths, not
from a hand-derived approximation, and verify it against an exhaustive (not sparse) boundary sweep.

At the field-level validated maxima together (`actor_count=128`, `sensor_count=16`, `chunk_extent=8`,
`active_chunk_radius` 0 through 4, `bootstrap_population=10_000`): every one of those runs failed
identically on the first post-warm-up tick with `actor cognition failed: scene cue count 1408 exceeds
64`, identically across every `active_chunk_radius` value tested (0 through 4) — evidence that at
tick 5 (one tick past a 4-tick warm-up), the actor/sensor term alone already exceeded the cap before
the surface-contact term (which needs time to accumulate contact history) had grown enough to differ
across radii. **A config that passes this plan's early-tick check is not proven safe for the rest of a
long run**: if `contacted_surface_count` keeps growing as more chunks register contact, a config that
ticks successfully at tick 5 could still fail at tick 5,000, even with no change to
`actor_count`/`sensor_count`.

This means `RuntimeConfig::validate()`'s independent per-field bounds do not describe the runtime's
actual runnable envelope, and no purely construction-time snapshot of "current" surface-contact state
can fully describe it either. A caller who reads `actor_count ≤ 128` and `sensor_count ≤ 16` as
independently safe and picks, say, `actor_count=64, sensor_count=2` gets a config that constructs
successfully via `Runtime::new` and then fails on tick 1 — a worse failure mode than a rejected
construction, because the cost of building the runtime and running warm-up ticks is already paid
before the error surfaces. The documented benchmark workload table in `docs/performance/benchmarks.md`
("small settlement: 100 residents") also assumes a promoted-actor scale this runtime cannot reach at
all today with more than one sensor per actor — not because it is slow, but because it does not run.

### Finding 2 — `RuntimeState::snapshot` recomputes two digests from scratch every tick, and both have components that grow without bound over a run's length — but the two are not equally fixable

`Runtime::tick()` (`crates/causafera-runtime/src/runtime.rs:180`) calls `state.snapshot(time)` after
every scheduler tick, and `Runtime::snapshot()` (the read-only observer-poll path) calls the same
`RuntimeState::snapshot` on every invocation. That function unconditionally computes two digests, both
built by re-walking their inputs from scratch, every call:

- **`history_digest()`** (`runtime.rs:1985`) — writes a fixed two-word header
  (`CURRENT_DIGEST_SCHEMA_VERSION`, `HISTORY_DIGEST_DOMAIN`), then walks the *entire* `CausalTraceStore`
  (`self.traces.iter()`, every event ever committed since tick 0 — append-only and unbounded by
  design, since bounding it would mean discarding provenance, which INV-014 forbids), then finally
  writes the (bounded, capped at `MAX_MATERIAL_SURFACE_TRANSITIONS` with oldest-entry eviction —
  `material_surface.rs:1046-1069` — **not** "never-truncated" as an earlier revision of this plan
  incorrectly claimed) `material_surfaces`/`material_surface_gate_transitions` state. **The unbounded
  section is written first, with only cheap bounded content after it** — a shape that is friendly to
  incremental computation (see Proposed architecture).

- **`physical_state_digest(time)`** (`runtime.rs:1755`) — writes a much longer sequence mixing bounded
  and unbounded terms, in this order: `executed_experiment_recipe_mana_sources`, `material_surfaces`,
  `pending_material_surface_changes`, `material_surface_transitions`, `material_surface_gate_transitions`
  (all bounded/capped), `pattern_history` (capped at `MAX_PATTERN_HISTORY_ENTRIES`/
  `MAX_PATTERN_HISTORY_PER_PATTERN`), mana field intensity arrays, thermal parameters, thermal field
  energy arrays — **then `self.thermal_receipts`/`self.thermal_conservation_receipts`
  (`runtime.rs:1877-1913`), two `BTreeMap<TraceId, ...>`s that gain one new entry every tick from
  `ThermalEvolutionSystem::execute` (`crates/causafera-runtime/src/thermal.rs:156-163`,
  `state.thermal_receipts.entry(conservation_trace).or_default().extend(receipts)` and
  `state.thermal_conservation_receipts.insert(...)`), with no eviction, pruning, or truncation of
  either map found anywhere in the crate — followed by `thermal_boundary_records`, `resolution`,
  `active_chunks`, `actors`, `actor_objects`, `population_aggregates`, and `aggregate_actor_pool`**
  (`runtime.rs:1914-1978`), all of which are bounded-but-arbitrarily-mutable current-tick state, not
  append-only.

**This ordering matters for what is fixable without changing the digest's byte output.**
`CanonicalDigest` (`digests.rs:196-215`) is a pure streaming accumulator with no composition operator:
each `write()` mutates its four-word state as a function of the current state and the new value, in
strict sequence, with a length prefix written immediately before several of the collections above
(including `self.thermal_receipts.len()` at `runtime.rs:1877` and
`self.thermal_conservation_receipts.len()` at `runtime.rs:1904`). Because `history_digest`'s only
unbounded input (the trace store) comes *first*, with nothing but cheap bounded content after it, an
incremental accumulator that resumes from where it left off and then writes the same small bounded
tail into a clone each tick reproduces the exact current output — this is achievable without a schema
change (see Proposed architecture). `physical_state_digest`'s unbounded inputs
(`thermal_receipts`/`thermal_conservation_receipts`) sit in the *middle* of its write sequence, with
more arbitrarily-mutable, non-append-only state (`thermal_boundary_records` through
`aggregate_actor_pool`) written *after* them every tick. There is no fixed point after which
`physical_state_digest`'s output only depends on append-only growth — the content written after the
thermal maps changes on every tick regardless of whether those maps grew, so a persistent accumulator
cannot be "resumed and then have a bounded tail appended" the way `history_digest`'s can. Reordering
the write sequence so the unbounded thermal maps come last would fix this, but reordering changes the
digest's byte output for every existing config, which is exactly the kind of change this plan is
trying to avoid without a deliberate, separately-designed schema migration. **This plan does not
propose that migration** (see Non-goals) — `physical_state_digest`'s unbounded thermal-receipt growth
is confirmed and real, but its fix requires a design this investigation did not do, not just an
implementation wave.

Isolated timing (temporary `AtomicU64` counters around each call, reverted after measurement; fixed
config `chunk_extent=3`, `active_chunk_radius=0`, `actor_count=8`, `sensor_count=2`,
`bootstrap_population=512`, the smallest workload this investigation used anywhere, and therefore also
the smallest possible `thermal_receipts`/`thermal_conservation_receipts` growth rate — one entry per
tick regardless of field size, since `ThermalEvolutionSystem` runs every tick independent of
`chunk_extent`/`active_chunk_radius`):

| tick range | trace events | total ms (64 ticks) | `physical_state_digest` | `history_digest` | rest of tick |
|---|---|---|---|---|---|
| [0, 64) | 2,878 | 21.2 | 4.8 ms (23%) | 9.7 ms (46%) | 6.7 ms |
| [64, 128) | 5,199 | 38.7 | 6.4 ms (17%) | 25.4 ms (66%) | 6.9 ms |
| [192, 256) | 9,669 | 71.8 | 6.9 ms (10%) | 56.9 ms (79%) | 7.9 ms |
| [320, 384) | 14,144 | 108.2 | 7.4 ms (7%) | 91.3 ms (84%) | 9.5 ms |
| [448, 512) | 18,700 | 144.6 | 7.7 ms (5%) | 125.7 ms (87%) | 11.2 ms |

`history_digest` is already the largest single share of tick time in the very first batch (46%, at
only 2,878 accumulated trace events) and grows to dominate (87%) as trace events accumulate. Over the
512 ticks measured, mean per-tick wall time grew from 0.33 ms to 2.26 ms — a 6.8x slowdown with zero
change in simulated workload size, solely from run length. `physical_state_digest`'s absolute cost
also grew over the same run (4.8 ms → 7.7 ms, 1.6x) — smaller than `history_digest`'s growth here, but
nonzero and attributable to the same class of defect (`thermal_receipts`/`thermal_conservation_receipts`
accumulating one entry per tick). **This plan's Wave 3 fixes the `history_digest` share of this table.
The `physical_state_digest` growth documented here is real, measured, and left unfixed by this plan**
— see Non-goals and the follow-up item this finding opens.

The same instrumentation on the `chunk_extent` and `active_chunk_radius` axes shows `history_digest`
remaining the largest single named cost even as field size grows, while an unlabeled "rest of tick"
bucket — dominated by legitimate per-cell mana/thermal field physics, not decomposed further in this
pass — grows fastest of all:

| chunk_extent | total ms (64 ticks) | `physical_state_digest` | `history_digest` | rest of tick |
|---|---|---|---|---|
| 3 | 20.1 | 4.7 ms (23%) | 9.6 ms (47%) | 5.9 ms |
| 8 | 150.4 | 9.3 ms (6%) | 68.6 ms (46%) | 72.4 ms |
| 16 | 719.4 | 19.0 ms (3%) | 285.9 ms (40%) | 414.5 ms |
| 32 | 4303.2 | 67.7 ms (2%) | 1218.9 ms (28%) | 3016.6 ms |

| `active_chunk_radius` (Area shape) | total ms (64 ticks) | `physical_state_digest` | `history_digest` | rest of tick |
|---|---|---|---|---|
| 0 | 20.4 | 4.7 ms (23%) | 9.7 ms (47%) | 6.0 ms |
| 1 | 109.5 | 7.8 ms (7%) | 55.3 ms (50%) | 46.4 ms |
| 2 | 298.2 | 10.2 ms (3%) | 140.3 ms (47%) | 147.7 ms |
| 4 | 924.4 | 17.3 ms (2%) | 426.3 ms (46%) | 480.8 ms |

Re-running the run-length sweep with `mana_parameters.effect_threshold = i64::MAX` (gate never opens,
`material_surface_gate_transitions` stays empty) produced trace-event counts and timing percentages
within noise of the churning-gate run — gate-transition growth specifically is not a material
confound for `history_digest`'s trace-store scan, consistent with that vector being capped and small.

### Existing benchmark-infrastructure gaps found

- No statistical repetition anywhere in the shipped harness (`benchmark.rs`, `material_surface_loop_benchmark`,
  `observer_overhead`, `extent_bench.rs`, `mana_gate_calibration.rs`): each reported number is a single
  timed run. `docs/performance/benchmarks.md`'s own requirement ("statistical reporting (mean, median,
  stddev)") is not met by any current tool. Confirmed independently twice, by two different negative
  `world_chunks_observer_overhead_ns` values on two separate single-shot runs (−28,469 ns and
  −187,351 ns) — both physically impossible for strictly additional work.
- `MaterialSurfaceLoopBenchmarkMeasurement::peak_rss_kib` / `steady_rss_kib` read `/proc/self/status`
  for the *whole process*, and `run_material_surface_loop_benchmark` runs both measured modes
  sequentially in one process, so the second mode's reported peak RSS is polluted by the first mode's
  already-torn-down `Runtime` — confirmed locally (`world_chunks_query_peak_rss_kib` 7,680 ≥
  `observer_off_peak_rss_kib` 7,004 by construction). Per-case RSS needs process isolation to be
  trustworthy.
- `.github/workflows/benchmarks.yml` only runs `cargo test --workspace --all-features --release --
  --ignored`. It does not capture, store, or compare any benchmark number, despite the job's name
  ("Benchmarks and Long Runs") and despite `docs/performance/benchmarks.md`'s Reporting requirements.
- No documented reference-hardware run exists yet matching `docs/performance/benchmarks.md`'s stated
  reference machine.

## Relevant invariants

- INV-017 — performance is architectural; this plan treats `history_digest`'s cost as a now-relevant
  correction, not deferred polish, because it already dominates tick cost at the smallest possible
  workload and grows without bound over any long run.
- INV-018 — scale claims require reproducible benchmarks; every number in this plan is labeled
  provisional until Wave 1 checks in the harness that reproduces it.
- INV-038 — digests are equality/divergence anchors only; this plan never treats digest-byte distance
  as a physical or performance metric. Wave 3's design goal is that `history_digest`'s *value* does not
  change at all — a correction from this plan's first draft, which proposed a schema-version bump and
  is documented as rejected in the Decision log.
- INV-014 / INV-023 — provenance and world-generation provenance are first-class; Wave 3's incremental
  `history_digest` must still cover every committed event, not skip or sample any of them, and must
  produce bit-identical output to the current full-rescan implementation, verified by a differential
  test (see Verification).
- INV-016 — authoritative mutation is phase controlled; nothing in this plan adds mutation outside
  the existing scheduler phases.
- INV-042 — modular architecture; the digest-cost fix stays inside the existing `runtime.rs` /
  `provenance.rs` module boundaries rather than introducing a new cross-cutting module for a narrow
  fix.

## Ontology domains affected

None directly. This plan does not add, remove, or reinterpret domain state.

## Causal carriers affected

None.

## Relevant documents

- `docs/development/todo-backlog.md` — `TODO-PERF-001` (sharpened by this plan, still Pending).
- `docs/performance/philosophy.md`, `docs/performance/metrics.md`, `docs/performance/benchmarks.md`.
- `docs/architecture/detailed-development-rebaseline.md` — acceptance template question 9.
- `docs/ontology/domain-coverage-matrix.md` — `Simulation runtime` row.
- `docs/simulation/long-run-experiments.md`.
- `crates/causafera-runtime/src/benchmark.rs`, `crates/causafera-runtime/src/benchmark_validation.rs`,
  `crates/causafera-runtime/examples/material_surface_loop_benchmark.rs`,
  `crates/causafera-runtime/examples/observer_overhead.rs`,
  `apps/observer/src-tauri/examples/extent_bench.rs`,
  `apps/observer/src-tauri/examples/mana_gate_calibration.rs`.
- `.github/workflows/benchmarks.yml`.
- `crates/causafera-core/src/provenance.rs` (`CausalTraceStore`), `crates/causafera-runtime/src/digests.rs`
  (`CanonicalDigest`), `crates/causafera-runtime/src/runtime.rs` (`RuntimeState::snapshot`,
  `physical_state_digest`, `history_digest`, `RuntimeConfig::validate`), and
  `crates/causafera-runtime/src/thermal.rs` (`ThermalEvolutionSystem::execute`).
- `crates/causafera-cognition/src/scene.rs` (`MAX_SCENE_CUES`) — the source of truth Wave 2 must
  import, never duplicate.
- `crates/causafera-runtime/src/snapshot_sections.rs:2729-2730` — the fail-closed loader check this
  plan's no-schema-bump design keeps satisfied.

## Current state

Described in full in Context. Summarized: config validation is per-field, does not compose, and its
cue-count boundary is not precisely characterized by any formula this investigation derived; two
full-rescan digests dominate tick cost, one (`history_digest`) fixable now without a schema change
because its unbounded input is written first, and one (`physical_state_digest`) with a real but
harder-to-fix unbounded component this plan identifies but does not fix; and the benchmark
infrastructure cannot currently produce a trustworthy number because it lacks repetition and per-case
isolation.

## Proposed architecture

1. **Reject configurations that cannot execute, derived from the actual perception code, with a
   runtime backstop kept regardless.**
   `RuntimeConfig::validate()` gains a bound that rejects configurations whose worst-case per-actor cue
   count would exceed `causafera_cognition::MAX_SCENE_CUES` (imported, never duplicated as a literal).
   Given Finding 1's conclusion that this plan does not have a trustworthy closed-form formula, the
   bound must be **derived from the same construction the perception code actually performs** — either
   by factoring `actor_perception_step`'s signal-assembly logic into a function both it and `validate()`
   call, or by computing the bound as a worst-case count over the same inputs
   (`sensor_count` apertures × up to `active_chunk_count` worst-case contacted surfaces, plus every
   other actor's object, with the exact per-actor accounting matched to what
   `physical_signals`/`material_surface_physical_signals` actually produce, not approximated) — and
   verified against Wave 1's harness running an **exhaustive** boundary sweep (every integer
   `actor_count` near the transition, not sparse samples), closing the off-by-one risk Finding 1 left
   open. The cognition-layer `MAX_SCENE_CUES` check stays in place regardless as a defense-in-depth
   backstop; `validate()`'s bound is a usability improvement (fail at construction, not mid-run), not a
   replacement for it.

2. **Make `history_digest`'s trace-store scan incremental, bit-identical, no schema change.**
   `CanonicalDigest` is a pure streaming accumulator: a sequence of `write()` calls produces an
   identical final state whether performed in one continuous pass or split across many calls that
   resume from previously-saved state. Because `history_digest` writes its only unbounded input (the
   trace-event sequence) *first*, with nothing but cheap, already-bounded content
   (`material_surfaces`/`material_surface_gate_transitions`) after it, `RuntimeState` can keep a
   persistent running accumulator that has already absorbed every trace event through the last tick;
   each tick, only the newly committed events are fed in, the accumulator is cloned (cheap — `[u64; 4]`,
   `Copy`), the small bounded tail is written into the clone, and `finish()` is called on the clone —
   reproducing the exact output the current full rescan produces, for the same state. This claim is not
   accepted on the strength of the argument alone (see Verification's differential oracle-test
   requirement).

3. **`physical_state_digest` stays a full rescan in this plan, in its entirety — not a partial
   incrementalization.** Finding 2 establishes why: its one genuinely unbounded input
   (`thermal_receipts`/`thermal_conservation_receipts`) is written in the *middle* of a long sequence
   that includes further non-append-only, arbitrarily-mutable current-tick state written *after* it
   (`thermal_boundary_records` through `aggregate_actor_pool`). There is no fixed resume point after
   which the remaining output depends only on append-only growth, so the same "clone and append a
   bounded tail" trick used for `history_digest` does not apply here without reordering the write
   sequence — and reordering changes the digest's byte output, which this plan is scoped to avoid. This
   is a narrower, more conservative position than this plan's second draft took (which proposed
   incrementalizing the thermal-receipt portion); independent review identified the interleaving
   problem that makes that proposal unworkable as stated. See Non-goals for what a real fix would
   require and why it is out of this plan's scope.

## Primitive vs emergent review

Not applicable — no primitive or emergent concept is added, removed, or reclassified. `MAX_SCENE_CUES`
remains the cognition layer's existing bounded-attention primitive; this plan makes runtime config
validation consistent with a cap that already exists and does not change what an actor perceives.

## Non-goals

- No SoA conversion, no scheduler parallelization, no multi-threaded phase execution, no CUDA/GPU
  work. Nothing measured shows sequential scheduler dispatch as a bottleneck; the measured cost is
  concentrated in digest computation and (separately) legitimate O(cells) field physics at large
  `chunk_extent`/`active_chunk_radius`.
- **No fix for `physical_state_digest`'s unbounded `thermal_receipts`/`thermal_conservation_receipts`
  growth.** Finding 2 and Proposed architecture point 3 establish that this is real, measured, and not
  fixable within this plan's "no schema change" constraint because of how the write sequence is
  ordered. A real fix needs one of: (a) a retention/compaction policy that bounds how many thermal
  receipt entries are retained (a domain decision about how much thermal history needs to stay
  reconstructable, not a pure performance change); (b) reordering `physical_state_digest`'s write
  sequence so the unbounded maps are written last, paired with a deliberate, explicitly-versioned
  schema migration and a persistence-compatibility plan for existing snapshots; or (c) a composable
  digest primitive (one that supports combining two independently-computed partial digests, which
  `CanonicalDigest` does not currently provide). This plan does not pick one — that is a follow-up
  design decision, not an implementation detail, and is recorded as an open item this investigation
  surfaced rather than closed.
- No change to `MAX_SCENE_CUES` itself, to how many objects/signals an actor perceives, or to what
  any accepted configuration computes. Proposed architecture point 1 is a validation-time rejection
  derived from existing behavior. It does change *which configurations construct*: a worst-case bound
  necessarily rejects configurations whose worst case is unreachable in some particular run, so some
  configurations that tick today stop being admitted. An earlier revision of this plan described
  point 1 as "not a capability or behavior change" without qualification, which was wrong — see the
  Decision log and Risks for the measured cost of that rejection and why the alternative is worse.
- No CI regression-gating. This plan proposes only that CI *capture and persist* benchmark output
  (Wave 4) as the prerequisite; this explicitly does **not** close `TODO-PERF-001`'s
  reproducible-and-compared-across-commits requirement on its own.
- No attempt to reach the `docs/performance/benchmarks.md` workload table's "10,000 residents" or
  "100,000 residents" tiers.
- No reference-hardware run.

## Implementation stages

Each wave that changes implementation, architecture, or a domain-facing contract updates its relevant
subsystem docs, `CHANGELOG.md`, `docs/development/todo-backlog.md`, `docs/roadmap/roadmap.md`, and
`docs/ontology/domain-coverage-matrix.md` as part of that wave's own checkpoint, per `AGENTS.md` —
**not** deferred to a single final documentation wave, which an earlier revision of this plan
incorrectly proposed.

**Wave 1 — checked-in, reproducible benchmark/diagnostic harness with one concrete executable surface.**
`physical_state_digest`/`history_digest` are `pub(crate)`, so a standalone example cannot call them
directly. Concrete design: add the instrumented measurement functions to
`crates/causafera-runtime/src/benchmark.rs` itself (same-crate `pub(crate)` access, matching the
existing pattern), returning new `pub` report types; a checked-in example calls the `pub` functions.
Required properties:
- **N = 20 repetitions per case** (a named constant, not ad hoc), reporting mean, median, and stddev,
  with all 20 raw per-repetition samples retained in the output;
- **deterministic cyclically-rotated case ordering, not a fixed repeated order**: for cases
  `[1, 2, ..., k]`, pass `p` (0-indexed) runs the cases starting at offset `p mod k` — pass 0 runs
  `1, 2, ..., k`; pass 1 runs `2, 3, ..., k, 1`; pass 2 runs `3, 4, ..., k, 1, 2`; and so on, cycling
  back once `p mod k` wraps. A plain repeated `1, 2, ..., k` pass order (what an earlier draft of this
  bullet described) still lets case 1 run first in every single pass, so any first-in-pass effect
  (cache warmth, thermal throttling) would bias case 1 identically across all 20 repetitions instead of
  being cancelled by them — the rotation is what actually distributes that bias evenly across every
  case. This needs no seed and is trivially reproducible, unlike a randomized order;
- per-case RSS measured via subprocess isolation (one process per measured case);
- hardware/toolchain metadata captured alongside every report (core count, `rustc --version`, profile
  flags);
- a canonical-result equality check between compared modes where applicable (e.g., `observer_off` vs.
  `world_chunks_query` should produce the same underlying simulation state);
- an **exhaustive** `actor_count`/`sensor_count` sweep near the `MAX_SCENE_CUES` boundary (every
  integer in the transition region, not sparse samples), closing the off-by-one gap Finding 1 left
  open and giving Wave 2 a precise boundary to validate against, **at `active_chunk_radius=0`
  specifically (so `contacted_surface_count ≤ 1`, isolating the actor/sensor terms)**;
- a separate **worst-case surface-contact check**, independent of the sweep above: a direct
  (non-simulation, or a simulation forced/seeded to actually reach full contact) verification that
  Wave 2's `validate()` bound stays correct when `contacted_surface_count` reaches its true worst case
  — `active_chunk_count` (at most one surface per chunk) — not just whatever contact level an arbitrary
  "long enough" test run happens to reach, since which chunks register contact within a bounded run
  depends on where actors move and act and is not otherwise controlled;
- a `chunk_extent`/`active_chunk_radius`/run-length sweep reporting total tick time,
  `physical_state_digest` time, and `history_digest` time per case, using the `pub` instrumented
  functions rather than the temporary hot-path `AtomicU64` counters this investigation used and
  reverted.
This wave is a prerequisite for Waves 2–3.

**Wave 2 — reject unrunnable actor/sensor/surface-contact configurations at construction.**
Implement Proposed architecture point 1. Direct tests: a config at the exhaustively-located boundary
(Wave 1) is rejected by `Runtime::new` with a descriptive error instead of succeeding and failing on
`tick()`; a config just inside the boundary still constructs and ticks successfully, verified two ways
— an ordinary run (useful but not sufficient on its own, since which chunks register contact within a
bounded run depends on where actors happen to move and act) and Wave 1's dedicated worst-case
surface-contact check forcing `contacted_surface_count` to `active_chunk_count`. Confirm the
cognition-layer `MAX_SCENE_CUES` check still exists and still fails closed after this wave. Update
`docs/development/todo-backlog.md`, `docs/ontology/domain-coverage-matrix.md`, `CHANGELOG.md` as part
of this wave's checkpoint.

**Wave 3 — incremental `history_digest`, with a differential oracle test as the primary verification.
Scoped to the trace-event portion only — `physical_state_digest` is untouched by this wave (see
Non-goals).**
Implement Proposed architecture point 2. No digest schema version change. The load-bearing
verification is a **differential oracle test**: for a range of ticks, multiple commit batches per
tick, snapshot export/import/resume boundaries, and observer polls interleaved with ticks, assert the
incremental digest equals a digest computed by the current full-rescan implementation kept alongside
it as a test-only retained reference — because the existing replay/locale-independence suite compares
two runs produced by the *same* implementation and cannot, on its own, distinguish a correct
incremental accumulator from one with a systematic absorption bug that affects both compared runs
identically. Existing replay/locale tests remain useful secondary regression coverage once the oracle
test establishes correctness. Re-run Wave 1's harness before/after to confirm the measured
`history_digest` share drops materially at the same fixed workloads used in Finding 2's tables (the
`physical_state_digest` share is expected to be unchanged by this wave, consistent with it being out of
scope). Update `docs/simulation/long-run-experiments.md`, `docs/ontology/domain-coverage-matrix.md`,
`CHANGELOG.md`, `docs/development/todo-backlog.md` as part of this wave's checkpoint.

**Wave 4 — CI capture, not gating.**
Extend `.github/workflows/benchmarks.yml` to run Wave 1's harness and persist its output as a build
artifact keyed by commit SHA. No pass/fail threshold. This wave alone does not close `TODO-PERF-001`
— reflected explicitly in the TODO's acceptance criteria, updated as part of this wave's checkpoint
alongside `docs/performance/benchmarks.md`.

**Wave 5 — close-out.**
Once Waves 1–4 land and each has updated its own relevant docs per the checkpoint discipline above:
re-evaluate `TODO-PERF-001` against its full sharpened acceptance criteria (it may remain Pending if
the `physical_state_digest`/thermal-receipt follow-up from Non-goals is judged part of its scope, or be
narrowed/split — a decision for whoever accepts this wave, not predetermined here), move this plan from
Draft to Active/completed in `PLANS.md`, and do a final pass confirming no cross-document claim is left
inconsistent with what actually landed.

## Verification

- `cargo test --release --workspace` after each wave, zero failures.
- `cargo test --release --workspace -- --ignored` after any wave that changes what a configuration
  admits or how a tick is computed. The default suite skips these, but `.github/workflows/benchmarks.yml`
  runs exactly this command, and the long-run tests it covers are the ones most likely to use a wide
  or long-running configuration a change like Wave 2's would newly reject.
- `cargo fmt --all -- --check` and `cargo clippy --release --workspace --all-targets -- -D warnings`
  after each wave.
- Wave 3: the differential oracle test is required and is the primary verification, not optional.
  Replay-determinism tests (`same_seed_replay_is_preserved_with_mana_effects_active`,
  `strict_replay_has_identical_canonical_state`, the full snapshot round-trip suite, and
  `observer_boundary`'s locale-independence tests) must also pass, as secondary regression coverage.
- Wave 1's harness output for the exact fixed workloads in this plan's Finding 2 tables becomes the
  durable replacement for the provisional numbers above.

## Benchmark plan

Covered inline in Context (Findings 1 and 2) and Implementation stages (Wave 1 is the benchmark-plan
deliverable).

## Determinism impact

Waves 1, 2, 4, 5: none.

Wave 3: changes `history_digest`'s *computation path* only, scoped to its trace-event portion. Design
goal, enforced by the differential oracle test: the digest *value* does not change — no schema version
bump, no persistence break. If the oracle test cannot be made to pass, this wave is blocked and
reconsidered rather than falling back to a version bump, which is exactly what this plan's first draft
proposed and what independent review correctly rejected.

`physical_state_digest` is untouched by any wave in this plan; its unbounded thermal-receipt growth
(Finding 2) remains present after this plan's waves land. This is a known, explicitly-not-fixed
limitation, not an oversight — see Non-goals.

## Memory impact

Wave 3 adds a small fixed amount of accumulator state (last-absorbed trace index, four-word hash
state) to `RuntimeState`, not proportional to trace count.

No other wave changes persisted or resident memory shape.

## Observer impact

None of the fixes in this plan change any wire-visible field, schema, or projection. `Runtime::snapshot()`'s
observer-poll path pays the same `history_digest` cost on every poll as `tick()` does today, and Wave 3
improves both call sites identically (it fixes the function, not a specific caller).

## Explanation impact

None.

## Persistence impact

None. Wave 3 is specifically scoped to avoid a digest schema version change —
`CURRENT_DIGEST_SCHEMA_VERSION` stays at 5, and the fail-closed loader check at
`crates/causafera-runtime/src/snapshot_sections.rs:2729-2730` continues to accept snapshots produced
before and after Wave 3 lands, because the bytes those snapshots carry are required to be identical to
what the current implementation produces.

The in-memory running accumulator Wave 3 adds does not need to be part of the persisted snapshot
envelope — on resume, it can be rebuilt by replaying the restored `CausalTraceStore` once (a one-time
O(n) cost at resume, not a per-tick one).

## Cross-domain effects

None.

## Risks

- Wave 3 is the highest-risk wave: it touches code every replay-determinism and locale-independence
  test depends on. The differential oracle test is the load-bearing check specifically because the
  existing replay suite cannot distinguish a correct incremental accumulator from a systematically
  buggy one that affects compared runs identically.
- Wave 2's bound is only as good as how faithfully it mirrors the actual perception construction
  (Proposed architecture point 1); Finding 1's unresolved formula means this needs care and an
  exhaustive boundary sweep, not a quick arithmetic patch. The cognition-layer backstop is kept
  specifically because this bound might still be imperfect.
- **Wave 2's bound rejects configurations that run today.** `Area` charts at `active_chunk_radius`
  2 or more no longer admit 8 actors on 2 sensors, and radius 4 admits no sensors at all while
  material-surface signals are enabled, because 81 surfaces alone exceed the 64-cue cap. This is the
  price of a sound worst-case bound and it is charged to real work: this plan's own Wave 1
  `radius_4_area` measurement case had to move to `material_surface_signals_enabled = false`.
  Checked, not merely flagged: no shipped caller varies `active_chunk_radius`. `session_config` in
  `apps/observer/src-tauri/src/session.rs` takes only a seed and sets `active_chunk_shape = Area` at
  the default radius 1 (nine chunks, worst case 34), and no Tauri command or frontend path reaches
  the field at all, so the observer app is unaffected. The one affected consumer is
  `plans/observer-field-raster-map.md`, a Draft proposing config-gated `Area` charts, which will need
  fewer sensors, fewer actors, or its own decision about the surface-signal term before it widens the
  radius.
- Wave 2's bound depends on two facts in `causafera-perception` that no test in that crate is
  currently written to protect: `acquire_signals` discarding signals whose `time` differs from the
  acquisition's, and `is_later_sample` requiring a strictly increasing time. Together they make the
  extractor's `Change` term always zero on this call path, which is why a cue batch is exactly the
  sample count. `worst_case_scene_cue_count`'s doc comment names both call sites; if either changes,
  the bound silently becomes an undercount.
- **`physical_state_digest`'s unbounded thermal-receipt growth is left unfixed by this plan.** A long
  enough run will still see this cost grow, even after Wave 3 lands. This is Finding 2's second half,
  explicitly not addressed here (see Non-goals) — flagging it prominently so it is not mistaken for a
  closed issue once Wave 3's `history_digest` fix ships.
- **Wave 3 made the observer-poll path mutate shared state.** `Runtime::snapshot` still takes
  `&self`, but it now advances the history-digest prefix through the state mutex, so a path that was
  logically read-only no longer is. Checked, and not currently reachable concurrently: the workspace
  contains no `thread::spawn`, `tokio::spawn`, `rayon` or `par_iter`; every `Arc<Mutex<RuntimeState>>`
  clone belongs to a scheduler system that runs sequentially inside `tick`; and the one path that
  does cross threads — `observer_analyze`'s `spawn_blocking` in `apps/observer/src-tauri/src/main.rs`
  — takes the outer `Mutex<ObserverSession>` that every other command also takes, so runtime access
  is serialized before the state mutex is ever reached. A future observer that polls from a second
  thread while another ticks would newly contend on that mutex, and would want a concurrent
  differential test; today there is nothing to write one against.
- Finding 2's "rest of tick" bucket (field physics at large `chunk_extent`/`active_chunk_radius`) is
  large and was not decomposed further in this investigation.
- Wave 1's harness, once checked in, becomes the new source of truth this plan's own tables are
  measured against; if its numbers disagree materially with this plan's provisional numbers, Wave 1's
  numbers govern.
- This plan's evidence is single-environment (32-core/64GB local machine), not the documented
  reference hardware.

## Documentation changes

Per-wave, per the Implementation stages header note (AGENTS.md requires this, not a deferred final
wave). Already touched while authoring this Draft plan (recording the investigation and the plan's
existence, not yet any implemented fix): `CHANGELOG.md`, `PLANS.md`, `docs/development/todo-backlog.md`,
`docs/performance/benchmarks.md`, `docs/simulation/long-run-experiments.md`,
`docs/ontology/domain-coverage-matrix.md`, `docs/roadmap/roadmap.md`.

## TODO changes

`TODO-PERF-001`: **Completed.** Its acceptance criteria named Wave 1's harness output, Finding 1's
exhaustively-located config-boundary regression test, and Finding 2's `history_digest` before/after
measurement (not `physical_state_digest`, which this plan does not fix) as the concrete acceptance
evidence, and each is now in place. Its own Out of Scope list had already excluded reference-hardware
runs, CI regression gating and any treatment of `physical_state_digest`, so nothing in scope remains;
leaving it Pending would have meant a status no remaining wave could ever clear.

Two successors were opened rather than letting the excluded work stay implicit in a closed TODO:

- `TODO-PERF-002` — `physical_state_digest`'s unbounded `thermal_receipts`/
  `thermal_conservation_receipts` growth, carrying this plan's three named candidate approaches
  (retention/compaction, reorder-plus-schema-migration, or a composable digest primitive) and the
  measured cost that motivates it. A design decision, not an implementation task.
- `TODO-PERF-003` — the two `docs/performance/benchmarks.md` Reporting requirements Wave 4's
  capture-only step does not satisfy: regression flagging, which needs a historical series before a
  threshold means anything, and a reference-hardware run, which does not exist.

## Decision log

- Rejected implementing any wave without a Draft/acceptance gate, given INV-007/INV-038 sensitivity.
- **Revision 2 — rejected the schema-version bump proposed in the first draft.** `CanonicalDigest` is a
  pure streaming accumulator; splitting the write sequence across calls does not change the final
  state, and the loader fail-closed-rejects any schema mismatch, making a bump an avoidable persistence
  break.
- **Revision 2 — corrected the "never-truncated" claim about `material_surface_gate_transitions`**; it
  is capped with oldest-entry eviction.
- **Revision 2 — (superseded by Revision 3) briefly expanded `physical_state_digest`'s scope to include
  incrementalizing `thermal_receipts`/`thermal_conservation_receipts`.** Revision 3 walks this back:
  independent review identified that these maps are written in the *middle* of `physical_state_digest`'s
  write sequence, with further arbitrarily-mutable state written after them, so the same
  "clone-and-append" trick that works for `history_digest` does not apply without reordering the write
  sequence (which would itself change the digest's output). This is now an explicit, unfixed Non-goal
  with three named possible approaches for a future plan, not a Wave 3 deliverable.
- **Revision 2 — expanded Finding 1 from two axes to three**, adding the material-surface-contact term.
  **Revision 3 — retracted the specific closed-form formula** Revision 2 proposed for that three-axis
  relationship (`sensor_count × (actor_count − 1 + contacted_surface_count)`), after independent review
  showed it does not match the measured data (65 cues at `actor_count=64, sensor_count=1` is
  `actor_count + 1`, not the formula's prediction of 63). Wave 2 now derives its bound from the actual
  signal-construction code rather than a formula, verified by an exhaustive boundary sweep (Wave 1).
- **Revision 2 — made Wave 1's executable surface concrete** (functions live in `benchmark.rs`, same-crate
  `pub(crate)` access; a checked-in example consumes `pub` report types).
- **Revision 3 — restructured Implementation stages so each wave updates its own relevant
  documentation as part of its checkpoint**, per `AGENTS.md`, instead of deferring all documentation to
  a single final wave, which the second revision incorrectly proposed.
- **Revision 3 — fixed Wave 1's benchmark-methodology gaps**: committed to N=20 repetitions (named,
  not left open) and deterministic round-robin case ordering (not "counterbalanced or randomized",
  which left the choice unresolved).
- **Revision 3 — corrected `CHANGELOG.md`'s stale "working tree clean" claim** (see that file's entry
  for this investigation) to state precisely that `crates/`/`apps/` were left unmodified, not that the
  whole working tree was clean — it was not, once this plan's own documentation updates are counted.
- **Revision 4 — corrected Revision 3's "round-robin" description**, which as written (`1, 2, ..., k`
  repeated 20 times) still ran the same case first in every pass, leaving the positional bias it was
  meant to cancel. Replaced with cyclic rotation of the starting offset per pass (Implementation
  stages, Wave 1).
- **Revision 4 — added a worst-case, helper-level check to Wave 1/Wave 2's boundary verification.**
  A "run long enough to exercise real surface-contact growth" test (as Wave 2 described) does not
  guarantee every active chunk actually registers contact within that run — it depends on where actors
  happen to move and act, which is not controlled. Added a direct check, independent of any specific
  run's actual contact pattern, that Wave 2's `validate()` bound holds against `contacted_surface_count
  == active_chunk_count` (the true worst case, since at most one surface is bootstrapped per chunk),
  not only against whatever contact level a particular test run happens to reach.
- **Revision 4 — corrected `docs/development/todo-backlog.md`'s "both dominate" phrasing.** Finding
  2's own tables show `history_digest` as the dominant share at every measured point (46-87% of tick
  time); `physical_state_digest` grows in absolute terms but stays a 2-23% minority share in the same
  tables. Restated precisely rather than treating the two as equally dominant.
- **Revision 4 — softened two overstated phrasings.** "Every other promoted actor's physical object"
  (Finding 1) is stated more cautiously as likely including the perceiving actor's own object, since
  Finding 1's own retracted-formula discussion shows the exact accounting is unresolved. "Per-tick wall
  time rising 6.8x ... purely from trace-store accumulation" (`docs/simulation/long-run-experiments.md`,
  `CHANGELOG.md`) is restated as driven mainly by trace-store accumulation, with a smaller contribution
  from `physical_state_digest`'s own unbounded thermal-receipt growth over the same span (per Finding
  2's own numbers: 4.8 ms → 7.7 ms across the batches where `history_digest` went 9.7 ms → 125.7 ms) —
  "purely" overstated a single-cause claim this plan's own data does not support.
- **Wave 2 — derived the cue count exactly, which retired Finding 1's open formula question.** A
  per-actor cue batch is exactly the acquired sample count: `GenericFeatureExtractor::extract`
  appends a `Change` feature only for an adjacent sample pair at strictly increasing times, and
  `acquire_signals` discards any signal whose `time` is not the acquisition's own, so one batch can
  never hold two times and the change term is always zero. The worst case is therefore
  `sensor_count * (actor_count + active_chunk_count)` — one aperture per sensor, over one signal per
  promoted actor's object (held at `actor_count` by `population.rs`'s promotion guard) plus one per
  contacted material surface (`MaterialSurfaceBootstrapStage` creates exactly one per active chunk).
  This is why Finding 1's retracted formula did not fit: it subtracted the perceiving actor's own
  object, which perception does not exclude, and treated the surface term as a run property rather
  than a chart property.
- **Wave 2 — chose a worst-case bound over an exact one, accepting that it over-rejects.** The bound
  reproduces the harness's exhaustively measured first failure exactly at 1, 2, 4 and 8 sensors
  (rejecting first at 64, 32, 16 and 8 actors), because at those sensor counts every signal still
  clears every aperture. At 16 sensors it rejects 4 actors where the measured failure is at 5:
  apertures stop seeing every signal once sensor geometry spreads, so the worst case is no longer
  attained. **This gap is deliberate, not an off-by-one.** The acceptance criterion is containment —
  every configuration the sweep found failing must be rejected — plus tightness where the worst case
  is attainable, and both are asserted by tests. An exact bound was rejected because it would have to
  model sensor range and position, and because a configuration's *worst* case is what a long run can
  reach: contact spreading further later is the failure being prevented.
- **Wave 2 — replaced the "run long enough" worst-case check with a forced one.** Wave 1's
  `worst-case-contact` mode measured contact spread and found it flat at 3 surfaces against up to 49
  active chunks over 768 ticks, which is useful evidence but is not the check Wave 2 needs: no run
  demonstrates the worst case, because contact does not spread. The load-bearing verification is
  instead a unit test in `actors/perception.rs` that forces every active chunk's surface into contact
  and drives the real `actor_perception_step`/`actor_cognition_step` at it. Wave 1's mode was kept,
  re-scoped to report the measured spread against the admitted worst case, since that gap is the
  evidence for why the bound must be conservative.
- **Wave 2 — bounded against both cognition caps, not only `MAX_SCENE_CUES`.**
  `MAX_ATTENTION_CANDIDATES` is checked first, inside `Attention::update`, and both are 64;
  `MAX_RUNNABLE_SCENE_CUES` takes the smaller so the bound stays correct if they ever diverge.
- **Wave 3 — proved the resume sound from the trace store's mutation surface, not from INV-014's
  intent.** `commit_batch` is `CausalTraceStore`'s only `&mut self` method; it only pushes, and no
  truncate/clear/pop/remove/drain/retain exists in the crate, so an absorbed event's bytes can never
  change. The one back-patched field is `children` (`commit_batch` appends to an already-committed
  parent's entry), which `history_digest` does not read — recorded as an explicit precondition on
  `HistoryDigestPrefix` and `write_trace_event`, since adding it later would silently break resume
  with no test failing.
- **Wave 3 — rejected adding an `iter_from` accessor to `causafera-core`.** An earlier sketch added
  one to avoid `Iterator::skip` walking already-absorbed indices. The walk only constructs
  `CausalEventRef`s (a few slice reads each) against the `mix64` rounds per word it replaces, and
  widening a second crate's public API on the wave that touches INV-007 buys risk for an unmeasured
  gain. `skip` is used instead; if it ever shows up in the harness, `iter_from` can be added with a
  number behind it.
- **Wave 3 — kept the full-rescan oracle compiled into normal builds** (`#[doc(hidden)] pub`) rather
  than behind `#[cfg(test)]`, so integration tests can reach it too, not only in-crate unit tests.
- **Wave 3 — verified the oracle can fail.** An off-by-one injected into the absorbed count makes all
  three differential tests fail; reverting restores them. A differential test that has never been
  observed failing is not evidence.
- Rejected proposing scheduler parallelization, SoA conversion, or CUDA work: no measurement shows
  phase-dispatch overhead as a bottleneck at any config this runtime can currently execute.
- Rejected proposing CI regression gating in this plan: a threshold set before any historical data
  exists would be arbitrary.
- Rejected trusting the deleted scratch-probe numbers as this plan's final evidence: Wave 1 exists
  specifically because INV-018 requires reproducible benchmarks.

## Progress

Investigation and Draft-plan authoring (Revision 4) on `perf/detailed-dev-baseline-investigation`
(branched from `main` at `1557c9e`), checkpoint `8475af0`: no `crates/`/`apps/` source file is
modified by the investigation or plan-authoring itself (all instrumentation and probe examples used
during the investigation were written, run, and then reverted/deleted). Documentation touched across
all four revisions (distinct from the per-wave documentation updates specified in Implementation
stages, which record completed implementation, not the existence of this Draft): `CHANGELOG.md`,
`PLANS.md`, `docs/development/todo-backlog.md`, `docs/performance/benchmarks.md`,
`docs/simulation/long-run-experiments.md`, `docs/ontology/domain-coverage-matrix.md`,
`docs/roadmap/roadmap.md`.

- **Wave 1 (checked-in benchmark/diagnostic harness), checkpoint `c873f31`:**
  `crates/causafera-runtime/src/benchmark.rs` (`DigestCostSample`, `measure_digest_cost`,
  `canonical_state` added to `MaterialSurfaceLoopBenchmarkMeasurement`),
  `crates/causafera-runtime/src/benchmark_validation.rs` (`validate_benchmark_report`, new
  `CanonicalStateDivergedAcrossObserverModes` error variant, two new unit tests),
  `crates/causafera-runtime/examples/performance_baseline.rs` (new: `boundary-sweep`,
  `worst-case-contact`, `digest-cost` modes, `N=20`, cyclic case rotation, one subprocess per
  case/repetition for RSS isolation, hardware/toolchain metadata).
  Verified: `cargo test --release --workspace` (zero failures), `cargo fmt --all -- --check`,
  `cargo clippy --release --workspace --all-targets -- -D warnings` (all clean).
  Ran all three modes locally: `boundary-sweep` locates the exact boundary per `sensor_count`
  (e.g. `sensor_count=1` fails first at `actor_count=64`, cue count 65 — matching Finding 1's
  provisional number, now reproducible and exhaustive rather than sparse); `worst-case-contact`
  found `contacted_surface_count` stays flat at 3 regardless of `active_chunk_count` (1, 9, 25, 81
  across the tested radii) for an `actor_count=8` workload — real data superseding the "worst case
  assumed" framing in Finding 1's prose, informing Wave 2's design; `digest-cost` reproduced the
  plan's provisional scratch-probe numbers closely (e.g. `radius_4_area` 916 ms here vs. 924 ms in
  the deleted probe; `chunk_extent_16` 719 ms both times), with `history_digest` again the dominant
  named cost and low relative stddev across the 20 repetitions per case.
  This plan's provisional Context tables are superseded by this harness's output per Verification;
  they are retained as the historical record of what motivated this plan, not as ongoing evidence.

- **Wave 2 (reject unrunnable configurations at construction), checkpoint `e6d5ebb`:**
  `crates/causafera-runtime/src/actors/perception.rs` (`MAX_RUNNABLE_SCENE_CUES`,
  `worst_case_contacted_surface_count`, `worst_case_scene_cue_count`, three unit tests),
  `crates/causafera-runtime/src/runtime.rs` (`RuntimeError::SceneCueBudgetExceeded`),
  `crates/causafera-runtime/src/config.rs` (the `validate()` check, six unit tests),
  `crates/causafera-runtime/examples/performance_baseline.rs` (re-scoped modes 1 and 2, and one
  digest-cost case adjusted), `CHANGELOG.md`, `docs/development/todo-backlog.md`,
  `docs/ontology/domain-coverage-matrix.md`, and this plan.
  Verified: `cargo test --release --workspace` (66 result blocks, zero failures),
  `cargo test --release --workspace -- --ignored` (the command CI runs; four long-run tests,
  including `runtime_executes_a_long_causal_run_without_errors`, all pass — none used a
  configuration the new bound rejects), `cargo fmt --all -- --check`, and
  `cargo clippy --release --workspace --all-targets -- -D warnings` (all clean). No pre-existing
  test needed re-pointing.
  The bound is `sensor_count * (actor_count + active_chunk_count)`, derived in the Decision log.
  Re-ran all three harness modes against it. `boundary-sweep` now reports the validation boundary and
  additionally asserts that every accepted configuration ticks: it first rejects at `actor_count`
  64, 32, 16, 8 and 4 for `sensor_count` 1, 2, 4, 8 and 16, which reproduces Wave 1's measured first
  failures exactly for the first four and is conservative by one step at 16 sensors (measured failure
  at 5), with no configuration accepted and then failing. `worst-case-contact`, re-scoped to report
  measured spread against the admitted worst case at `sensor_count=1`, shows contact flat at 3
  surfaces while active chunks grow 1 → 9 → 25 → 49 (coverage 100% → 6.1%), and radius 4 `Area`
  no longer admitted at all — the evidence for why the bound cannot be calibrated on observed
  contact. `digest-cost` is unchanged for five of six cases; `radius_4_area` moved to
  `material_surface_signals_enabled = false` to stay constructible and re-measured at 942 ms mean
  against the 916 ms Wave 1 recorded with surface signals enabled, so that row is no longer directly
  comparable to Wave 1's and is labelled in the harness's own table. The 26 ms is not a finding and
  must not be read as one: turning surface signals off removes perception work, so a slower mean is
  the opposite of what the change would cause, and the delta is ordinary run-to-run and
  changed-workload variation rather than a regression signal.
  The `MAX_SCENE_CUES` backstop is unchanged and still fails closed, covered by
  `cognition_still_rejects_a_batch_over_the_cue_cap`.

- **Wave 3 (incremental `history_digest`), checkpoint `8020008`:**
  `crates/causafera-runtime/src/digests.rs` (`HistoryDigestPrefix`, `write_trace_event`, `Clone`/
  `Copy` on `CanonicalDigest`), `crates/causafera-runtime/src/runtime.rs` (the resumable
  `history_digest`, the retained `history_digest_full_rescan` oracle, the extracted
  `write_history_digest_tail`, the `history_digest_prefix` field, and three differential tests),
  `crates/causafera-runtime/src/benchmark.rs` and `snapshot_sections.rs` (`&mut` receivers),
  `CHANGELOG.md`, `docs/development/todo-backlog.md`, `docs/ontology/domain-coverage-matrix.md`,
  `docs/simulation/long-run-experiments.md`, and this plan.
  Verified: `cargo test --release --workspace` (zero failures),
  `cargo test --release --workspace -- --ignored` (zero failures), `cargo fmt --all -- --check`,
  `cargo clippy --release --workspace --all-targets -- -D warnings` (all clean). **No pinned digest
  value in any existing test needed re-pointing**, which is corroborating evidence for bit-identity
  beyond the oracle itself.
  The oracle covers 48 consecutive ticks (asserting each committed events, so a degenerate run
  cannot pass vacuously), repeated observer polls interleaved with ticks, and export/import/resume
  including the digest `assemble_envelope` writes into the snapshot header — that last one because a
  wrong prefix would otherwise be recorded into every exported snapshot rather than merely failing a
  test. Its teeth were confirmed by injecting an off-by-one into the absorbed count: all three tests
  fail, and reverting restores them.
  Re-ran `digest-cost` against Wave 2's numbers, 64 measured ticks per case (mean, N=20):
  `baseline_batch0` 21.9 ms → 13.0 ms, `baseline_batch7` 147.2 ms → 22.4 ms, `chunk_extent_8`
  151.6 ms → 90.8 ms, `chunk_extent_16` 736.0 ms → 468.2 ms, `radius_1_area` 114.2 ms → 61.4 ms,
  `radius_4_area` 942.1 ms → 549.9 ms. The load-bearing figure is `total_tick_ns`, quoted above: the
  harness's separate `history_digest_ns` column drops to roughly 0.2 µs, which is **expected and not
  the claim** — every tick has already absorbed, so a post-loop call finds nothing to do and is
  measuring an up-to-date accumulator rather than measuring nothing. The clearest single reading is
  the run-length penalty, `baseline_batch7` against `baseline_batch0`: 6.7x before, 1.7x after.
  `physical_state_digest` is unchanged by this wave, as intended — 97.4 µs and 125.9 µs per call
  before, 98.2 µs and 124.9 µs after, on those two cases. Those are per-call figures against
  per-64-tick totals, so stating the residual checkably: across the 64 calls in a case it is about
  6.3 ms of `baseline_batch0`'s 13.0 ms and about 8.0 ms of `baseline_batch7`'s 22.4 ms, which makes
  it the largest single named cost now that the trace scan is gone. Its growth between the two cases
  is about 1.7 ms, so it explains part but not all of the 9.4 ms that still separates them; the rest
  is other run-length-dependent state this investigation did not decompose.

- **Wave 4 (CI capture, not gating), checkpoint `84f743f`:** `.github/workflows/benchmarks.yml` (runs
  the harness, uploads its output as an artifact named for the commit SHA),
  `docs/performance/benchmarks.md` (the methodology-gaps section rewritten to separate what is now
  fixed from what is still open), `CHANGELOG.md`, `docs/development/todo-backlog.md`.
  No threshold and no regression flag, per this plan's Non-goals. The upload step carries
  `if: always()` so a failing capture still preserves its evidence, and `if-no-files-found: error` so
  a silently-empty artifact cannot pass for a recorded measurement. The capture step is allowed to
  fail the job on one condition only — the boundary sweep finding a configuration `validate()`
  accepted that then exceeded the cue cap at a tick — which is a Wave 2 soundness bug, not a
  performance threshold, and is documented as such in both the workflow and `benchmarks.md`.
  `actions/upload-artifact` is pinned by commit SHA to match every other action in the repository;
  the pin was resolved against the upstream tag rather than assumed.
  Sizing: the full three-mode run measures 39 s locally after Wave 3 (it was ~54 s before), against
  the job's default 360-minute timeout, so no timeout change was needed. Artifacts fall under the
  repository's default retention, which means "stored in version control" in `benchmarks.md`'s
  Reporting list is satisfied in spirit — durably keyed to a commit — but not literally.

- **Wave 5 (close-out), checkpoint `ec711ed`:** `TODO-PERF-001` re-evaluated against its full criteria and closed, with the
  reasoning recorded in the TODO itself rather than only here; `TODO-PERF-002` and `TODO-PERF-003`
  opened for the work its Out of Scope list had excluded, so closing it drops nothing.
  `PLANS.md` moved this plan to accepted-and-implemented. Cross-document consistency pass: every
  reference to this plan outside itself was re-read and corrected —
  `docs/roadmap/roadmap.md` still called it "Draft, not-yet-accepted" with "no implementation has
  landed", `docs/simulation/long-run-experiments.md` and `docs/ontology/domain-coverage-matrix.md`
  still carried "(Draft)"/"(Accepted)" status markers, and `long-run-experiments.md` had become
  internally contradictory, describing both digests as full-rescan in one paragraph and
  `history_digest` as incremental in the next.
  Problems found during this plan's work but outside its scope, recorded rather than fixed:
  `.github/workflows/benchmarks.yml` and `ci.yml` both pin `dtolnay/rust-toolchain` with a trailing
  `# 1.85.0` comment, but that action is invoked with no `toolchain` input, so it resolves to current
  stable — which `CHANGELOG.md` already notes when recording the 1.97.1 bump. The comment therefore
  names a version CI does not use. Left alone: it is a pre-existing annotation defect on a line this
  plan had no reason to touch.
  Correction (see `CHANGELOG.md`, "Fixed the misleading `dtolnay/rust-toolchain` pin comment..."):
  the "resolves to current stable" mechanism above is wrong. The pinned action commit hardcodes
  exactly 1.85.0 with no live resolution at all; checking a real CI run showed `rust-toolchain.toml`'s
  own toolchain-file override then silently superseded it on the first `cargo` invocation, so the job
  had already been compiling and linting under 1.97.1 throughout — the pin only wasted a download on
  an installed-but-unused 1.85.0 toolchain. Fixed by repointing both workflows to the action's
  `1.97.1` branch commit.
