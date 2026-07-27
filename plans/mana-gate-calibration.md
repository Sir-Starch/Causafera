# Mana Gate Calibration ExecPlan

**Status:** Accepted and implemented.

## Goal

Recalibrate the local mana effect gate's `effect_threshold` and `effect_hysteresis` against the
population it actually reads, so the gate discriminates between worlds instead of latching
(`TODO-MANA-007`).

## Context

`TODO-MANA-004`'s evidence tool, `extent_decision.rs`, measured "share of live cells above the gate"
over every live cell in every mana field and found it climbing from 0% to 24% as the lattice
refines, with distinct behaviour tuples across six seeds collapsing from 3 to 1. That evidence
framed `TODO-MANA-007` as a field-wide saturation problem: the response and gate constants were
calibrated before any carrier populated the field, and a finer lattice now drives the whole field
past the threshold.

`ManaEffectsSystem::execute` (`crates/causafera-runtime/src/mana.rs`) does not read the whole field.
It iterates `state.material_surfaces`, filters to `contact_count > 0`, and for each survivor reads
exactly `field.intensity()[surface_id.cell_index]` — one cell per contacted chunk.
`MaterialSurfaceBootstrapStage` (`bootstrap.rs`) places every surface at `cell_index` 0. So the gate's
actual input is cell 0 of each contacted chunk's field, a narrow, fixed-position sample — not the
field-wide distribution `extent_decision.rs` reports. The two are not the same population, and the
field-wide statistic is the wrong yardstick for calibrating a threshold the gate never evaluates
against it.

Measuring the correct population changes the diagnosis: at the production default (`chunk_extent`
3) and at every candidate lattice except the coarsest (12), the current constants (`4096`/`2000`)
already produce more than one distinct behaviour tuple across the six seeds — see Decision log. The
one place the acceptance criterion was not met is extent 12, where the population's mean (7163) sits
well above the threshold (4096) and the gate latches open early in the run.

## Relevant invariants

- INV-018 — performance and behavioural claims are benchmarked, not asserted; every number below
  comes from a run of `apps/observer/src-tauri/examples/mana_gate_calibration.rs`.
- INV-038 — digests are equality/divergence anchors only; every "changed" or "discriminates" claim
  below is a measured inequality (a distinct behaviour-tuple count), never a digest-distance claim.
- The mana field model (`docs/rfc/RFC-MANA-001.md`) states the local effect gate as a bounded
  physical threshold; a threshold the field permanently exceeds is not a threshold, which is the
  defect this plan closes.

## Ontology domains affected

Mana only. No new domain state, no new carrier, no wire protocol change. This is a constant
recalibration of an existing bounded mechanism (`ManaParameters::effect_threshold`,
`ManaParameters::effect_hysteresis`).

## Causal carriers affected

None. `ManaEffectsSystem` reads `material_surfaces` and the mana field; neither carrier's projection
changes. The response channels (`base_response`, `recurrence_response`, `periodicity_response`,
`synchrony_response`, `spatial_response`), diffusion, decay, the stencil, and which carriers
participate are all untouched, per the TODO's Out of Scope.

## Relevant documents

- `docs/development/todo-backlog.md` — `TODO-MANA-007`, `TODO-MANA-004` (evidence tool origin),
  `TODO-MANA-005` (provenance boundary the gate's causes still route through, untouched here).
- `docs/rfc/RFC-MANA-001.md` — the field model's canonical spec; its deferred-work list is narrowed
  to note the gate is now calibrated against a populated field, while the response channel weights
  remain deferred (untouched, per Non-goals).
- `plans/local-mana-material-surface-coupling.md` — the local gate/hysteresis architecture this plan
  recalibrates, including the four-case boundary semantics `ManaEffectsSystem::execute` implements.
- `apps/observer/src-tauri/examples/extent_decision.rs` — the `TODO-MANA-004` tool whose field-wide
  statistic motivated `TODO-MANA-007`'s original framing; still authoritative for lattice-fidelity
  and cost, superseded here only on what population calibrates the gate.

## Current state

`RuntimeConfig::new` (`crates/causafera-runtime/src/config.rs`) set `effect_threshold: 4_096`,
`effect_hysteresis: 2_000`. These predate `TODO-RUNTIME-002` (the terrain carrier reaching the tick
loop) and were never revisited against a populated field.

## Proposed architecture

No architectural change. `ManaEffectsSystem::execute`'s two-way hysteresis state machine is unchanged;
only the two constants it reads move. The mechanism justifying the new values:

1. The gate's actual input, at every candidate lattice, is cell 0 of each contacted chunk's mana
   field — 18 surfaces (3 per extent × 6 seeds) traced tick-by-tick over 192 ticks.
2. Because `material_surfaces` is read-only input to the gate and does not feed back into mana field
   evolution (`ManaEffectsSystem::execute` never mutates `state.mana`), one simulation run per
   seed/extent captures the whole intensity trace; every candidate `(threshold, hysteresis)` pair is
   then scored by replaying the gate's own hysteresis logic against the recorded trace, not by
   rerunning the simulation per candidate.
3. A candidate is scored by how many distinct `(transitions, ever-active-surface-count)` tuples the
   six seeds produce at a given lattice — a purified, gate-only proxy for "distinguishes worlds",
   cheap enough to sweep widely.
4. The chosen candidate is then re-verified end-to-end with real production runs, against the exact
   five-field `Behaviour` tuple (`gate_crossings`, `gate_transitions`, `surface_conditions`,
   `actions_committed`, `population`) that `extent_decision.rs` uses, so the claim is checked against
   what the original evidence measured, not only the narrower proxy.

Point 2 is only approximately true. `ManaEffectsSystem::execute` does not write `state.mana`
directly, but a gate transition changes `surface.condition`, which can reach a later actor's
signal/scene and action, and a different action can change which cells get contacted and sampled —
a loop closing back into the field the trace was recorded from. A direct feedback check (real
production runs at both the pre-calibration and chosen constants, same seed, same extent) measures
this: `mana_total` shifts a few percent at the finer lattices (extent 3: 73 470 → 78 054 on seed 7,
a 6.2% shift) and becomes negligible at the coarser ones (extent 12: 7 886 000 → 7 886 009, 0.0001%),
while `actions_committed` is unmoved (245 at every seed and extent tested, both constants). The
recorded-trace replay (points 3 sweep, neighbourhood check, hysteresis-axis check) is therefore an
approximate screening tool for narrowing the search space, not an exact measurement, and `record()`
pins a fixed reference config (the pre-calibration constants) so its output stays reproducible
regardless of what `RuntimeConfig::new`'s current default is. The end-to-end check in point 4 uses
fully independent real production runs per candidate and is not subject to this artifact; it is the
authoritative evidence for the accepted operating point.

## Primitive vs emergent review

`effect_threshold`/`effect_hysteresis` are stated bounded constants of the field model (RFC-MANA-001),
not an emergent property; recalibrating them is parameter tuning within an already-accepted
mechanism, not a new primitive.

## Non-goals

- Changing the response channel weights (`base_response`, `recurrence_response`,
  `periodicity_response`, `synchrony_response`, `spatial_response`).
- Changing diffusion, decay, the stencil, or which carriers participate.
- Changing the lattice (`chunk_extent`) or its default.
- Making every candidate lattice discriminate identically well; extent 12 discriminates more weakly
  in the low-threshold region than the mid-range, and that is reported rather than hidden (see
  Decision log).

## Implementation stages

**Wave 1 — measurement tool and evidence.**
Add `apps/observer/src-tauri/examples/mana_gate_calibration.rs`: records the cell-0 intensity trace
of every contacted surface across 192 ticks, 6 seeds, 5 candidate extents; reports the population's
spread; scores a threshold sweep, a neighbourhood-robustness check around the candidate, and a
hysteresis-axis check at both the current and candidate threshold; and re-verifies the chosen point
against real production runs using the exact five-field `Behaviour` tuple `extent_decision.rs` uses.

**Wave 2 — constant change and test fallout.**
Change `RuntimeConfig::new`'s `effect_threshold`/`effect_hysteresis` defaults
(`crates/causafera-runtime/src/config.rs`). Re-run the full workspace test suite; the only failure
was `different_seeds_produce_different_worlds_not_one_world_with_two_terrains`
(`crates/causafera-runtime/tests/terrain_carrier.rs`), whose hand-picked seed pair (7, 30) collapsed
onto the same behaviour tuple under the new gate — the exact class of defect this plan fixes. Fixed
by re-sweeping seeds 1–39 plus 59, 97, 101, 137, 211 under the new constants and re-pointing the test
at a pair (7, 5) that still discriminates on every metric, with a comment recording why.

**Wave 3 — documentation.**
Update `docs/development/todo-backlog.md` (close `TODO-MANA-007`), `docs/ontology/domain-coverage-matrix.md`
(Mana row), `CHANGELOG.md`, `PLANS.md`.

**Wave 4 — feedback verification and stale-constant sync.**
Added the feedback check to `mana_gate_calibration.rs` (real runs at both constants, same seed,
comparing `mana_total` and `actions_committed`) to verify rather than assume the replay's decoupling
claim; found it real but small (see Benchmark plan), reworded the plan's claims about the replay
accordingly, and pinned `record()`'s reference config so it no longer implicitly depends on
`RuntimeConfig::new`'s current default. Synced `extent_decision.rs`'s `THRESHOLD`/`HYSTERESIS`
constants to the new production defaults and narrowed `RFC-MANA-001.md`'s deferred-work line.

## Verification

- `cargo test --release --workspace` — full workspace, zero failures (see Decision log for the one
  fix required).
- `cargo fmt --all -- --check` — clean.
- `cargo clippy --release -p causafera-runtime --all-targets -- -D warnings` and
  `cargo clippy --release -p causafera-observer --example mana_gate_calibration -- -D warnings` —
  clean.
- Replay determinism: `same_seed_replay_is_preserved_with_mana_effects_active`,
  `strict_replay_has_identical_canonical_state`, and the snapshot round-trip tests all pass against
  the new defaults, which is the "regenerated replay evidence" the TODO's Determinism Requirements
  call for — replay identity is a property of the mechanism, not of a specific constant value, and
  it is re-demonstrated here rather than assumed.

## Benchmark plan / measured evidence

All figures from `cargo run --release -p causafera-observer --example mana_gate_calibration`, 6 seeds
(7, 11, 23, 41, 59, 97), 192 ticks, `actor_count` 8, `sensor_count` 2, `bootstrap_population` 512.

**The gate's actual population** (cell 0 of every contacted surface, pooled across all recorded
ticks and all six seeds):

| extent | surfaces | mean | stdev | min | max |
|---|---|---|---|---|---|
| 3 | 18 | 2165 | 1387 | 0 | 7845 |
| 4 | 18 | 2038 | 1245 | 0 | 7349 |
| 6 | 18 | 3174 | 1442 | 7 | 8923 |
| 8 | 18 | 4214 | 1543 | 95 | 10645 |
| 12 | 18 | 7163 | 2462 | 197 | 15331 |

The mean grows only ~3.3x from extent 3 to extent 12 — nowhere near the ~98x range `total_mana`
spans across the same lattices (`TODO-MANA-004`), because cell 0 is one fixed-position sample, not a
sum over the field. A single scalar threshold spanning this narrower range is plausible, which the
sweep confirms.

**Current constants (4096/2000) against the correct population**, replayed per surface:

| extent | distinct (transitions, ever-active) tuples across 6 seeds |
|---|---|
| 3 | 2 |
| 4 | 2 |
| 6 | 3 |
| 8 | 2 |
| 12 | 1 |

Already discriminates at 4 of 5 lattices, including the production default (extent 3) — the
field-wide statistic in the original `TODO-MANA-007` evidence overstated the defect by measuring a
population the gate does not read. The one real failure is extent 12, where the population's mean
(7163) sits above the threshold and the gate latches open.

**Threshold sweep** (hysteresis = threshold / 4), distinct tuples per extent:

| threshold | hyst. | ext3 | ext4 | ext6 | ext8 | ext12 |
|---|---|---|---|---|---|---|
| 16–128 | — | 1 | 1 | 1 | 1 | 1 |
| 2048 | 512 | 3 | 5 | 2 | 1 | 1 |
| 3072 | 768 | 5 | 4 | 5 | 2 | 1 |
| 4096 | 1024 | 4 | 3 | 6 | 4 | 1 |
| 5120 | 1280 | 2 | 1 | 4 | 2 | 1 |
| **6144** | **1536** | **4** | **4** | **3** | **4** | **4** |
| 8192 | 2048 | 1 | 1 | 4 | 4 | 5 |

6144/1536 is the only point in this sweep that discriminates at all 5 candidate lattices
simultaneously. This sweep is the approximate screening tool described in Proposed architecture —
it narrows the search space, and the end-to-end check further below re-verifies the chosen point
against independent real production runs, which is the authoritative evidence.

**Neighbourhood check** (is 6144 a plateau or a one-point spike?), hysteresis = t/4:

| threshold | ext3 | ext4 | ext6 | ext8 | ext12 |
|---|---|---|---|---|---|
| 5632 | 1 | 1 | 3 | 4 | 3 |
| 5888 | 2 | 2 | 3 | 3 | 4 |
| 6144 | 4 | 4 | 3 | 4 | 4 |
| 6400 | 4 | 5 | 4 | 5 | 4 |
| 6656 | 5 | 3 | 4 | 4 | 4 |

ext6, ext8 and ext12 hold at ≥3 across the whole 5632–6656 band in this screen — a plateau, not a
knife-edge. ext3 and ext4 are noisier in this band (1→2→4→4→5, not monotone): their means
(2038–2165) sit far below this threshold range, so the screen is scoring the tail of their
distribution rather than its bulk, and — per the feedback check below — this screen's recorded
trace is itself only an approximation of what production would produce at each candidate. This is
stated rather than hidden: the deciding evidence for whether 6144/1536 actually holds at every
extent is the end-to-end check further below, using independent real production runs per candidate,
not this screen.

**Hysteresis axis** (is the fix "narrower hysteresis at 4096" or does the threshold itself have to
move?):

| threshold | hyst. | ext3 | ext4 | ext6 | ext8 | ext12 |
|---|---|---|---|---|---|---|
| 4096 | 2048 | 2 | 3 | 3 | 2 | 1 |
| 4096 | 1365 | 4 | 3 | 3 | 2 | 1 |
| 4096 | 1024 | 4 | 3 | 6 | 4 | 1 |
| 4096 | 682 | 5 | 3 | 6 | 4 | 1 |
| 6144 | 3072 | 5 | 5 | 3 | 4 | 1 |
| 6144 | 2048 | 5 | 4 | 3 | 4 | 3 |
| 6144 | 1536 | 4 | 4 | 3 | 4 | 4 |
| 6144 | 1024 | 4 | 4 | 2 | 5 | 4 |

At threshold 4096, no hysteresis value reaches distinct > 1 at extent 12 — the population's mean
there (7163) sits far enough above 4096 that narrowing the dead band cannot help; the threshold
itself has to move. This rules out "hysteresis alone" as a sufficient fix and justifies moving the
threshold rather than only the hysteresis.

**Chosen operating point: threshold 6144, hysteresis 1536.** Per-seed detail, replayed against the
recorded trace:

| extent | mean/threshold ratio range across the 6 seeds | transitions range |
|---|---|---|
| 3 | 0.28x–0.43x | 4–10 |
| 4 | 0.28x–0.40x | 6–10 |
| 6 | 0.45x–0.57x | 9–11 |
| 8 | 0.55x–0.85x | 6–11 |
| 12 | 1.03x–1.61x | 3–8 |

The field sits well below the threshold at the production default and stays below it through extent
8; at extent 12 it sits just above, which is exactly the regime in which the gate can still both open
and close — consistent with distinct = 4 there.

**End-to-end check, real production runs, exact `extent_decision.rs` five-field `Behaviour` tuple**
(`gate_crossings`, `gate_transitions`, `surface_conditions`, `actions_committed`, `population`):

| extent | current 4096/2000 | chosen 6144/1536 |
|---|---|---|
| 3 | 2 distinct | 4 distinct |
| 4 | 2 distinct | 3 distinct |
| 6 | 3 distinct | 3 distinct |
| 8 | 2 distinct | 4 distinct |
| 12 | 1 distinct | 4 distinct |

The chosen constants never discriminate worse than the current ones on the exact metric the
original `TODO-MANA-007` evidence used, and strictly better at extents 3, 4, 8 and 12.

**Feedback check**: does the constant choice change `mana_total` or `actions_committed` at the same
seed (real production runs, pre-calibration vs. chosen constants)?

| extent | seed | mana_total (4096/2000) | mana_total (6144/1536) | actions_committed (both) |
|---|---|---|---|---|
| 3 | 7 | 73 470 | 78 054 | 245 |
| 3 | 11 | 79 044 | 81 270 | 245 |
| 3 | 23 | 93 591 | 97 666 | 245 |
| 3 | 41 | 40 856 | 43 087 | 245 |
| 3 | 59 | 57 613 | 59 930 | 245 |
| 3 | 97 | 89 398 | 89 877 | 245 |
| 12 | 7 | 7 886 000 | 7 886 009 | 245 |
| 12 | 11 | 8 425 561 | 8 425 508 | 245 |
| 12 | 23 | 8 728 655 | 8 728 598 | 245 |
| 12 | 41 | 8 831 824 | 8 831 858 | 245 |
| 12 | 59 | 8 526 746 | 8 526 762 | 245 |
| 12 | 97 | 7 653 519 | 7 653 533 | 245 |

The gate is not fully decoupled from the field it reads: a transition changes `surface.condition`,
which can reach a later actor's action and change which cells get contacted and sampled. `mana_total`
shifts up to ~6% at extent 3 and falls to ~0.0001% at extent 12; `actions_committed` is unmoved at
245 in every case tested. This confirms the recorded-trace replay above (threshold sweep,
neighbourhood check, hysteresis axis) is an approximate screen, not an exact measurement — `record()`
pins a fixed reference config precisely because of this, so its output does not additionally depend
on whichever constants happen to be compiled into `RuntimeConfig::new` at build time. It does not
undermine the chosen operating point: the end-to-end check above measures the real, feedback-inclusive
production behaviour directly, at both constants, and that is what the acceptance criterion is
checked against.

## Determinism impact

Changes every world by construction (a different gate threshold changes committed
`MaterialSurfaceGateTransition` events and downstream surface conditions), exactly as the TODO's
Determinism Requirements state. Replay identity is unaffected: `same_seed_replay_is_preserved_with_mana_effects_active`,
`strict_replay_has_identical_canonical_state`, and the full snapshot round-trip suite all pass
against the new constants.

## Memory impact

None; no new fields, no new persisted state.

## Observer impact

None in this plan. The TODO's Observer Implications note that the map's mana lens normalises
against a field whose scale differs by orders of magnitude between lattices — unaffected by this
constant change and out of scope here.

## Explanation impact

A gate transition is now evidence of something happening across every candidate lattice tested,
including the production default, per the acceptance criterion — closing the concern in the TODO's
Explanation Implications that the gate's silence was never informative because it could not also
not fire.

## Persistence impact

None; `effect_threshold`/`effect_hysteresis` are already persisted fields
(`snapshot_sections.rs`), only their default values change.

## Cross-domain effects

None beyond mana/material-surface, which are already coupled per
`plans/local-mana-material-surface-coupling.md`.

## Risks

- The calibration is measured against the current `actor_count`/`sensor_count`/`bootstrap_population`
  production-loop shape (8/2/512); a materially different population shape could shift the
  contacted-surface population's scale and require re-measurement.
- ext3/4's neighbourhood-robustness result is noisier than ext6/8/12's (see Benchmark plan); the
  end-to-end check is the stronger evidence there, and future work changing the response weights
  should re-run this tool rather than assume the operating point still holds.

## Documentation changes

- `docs/development/todo-backlog.md` — `TODO-MANA-007` marked Completed.
- `docs/ontology/domain-coverage-matrix.md` — Mana row.
- `docs/rfc/RFC-MANA-001.md` — deferred-work line narrowed: gate calibration done, response-channel
  calibration remains deferred.
- `apps/observer/src-tauri/examples/extent_decision.rs` — `THRESHOLD`/`HYSTERESIS` constants synced
  to the new production defaults, so its printed ratio column keeps comparing against what
  `RuntimeConfig::new` actually uses.
- `CHANGELOG.md`, `PLANS.md`.

## TODO changes

`TODO-MANA-007`: Pending → Completed.

## Decision log

- Rejected reusing `extent_decision.rs`'s field-wide "share of live cells above the gate" statistic
  as the calibration target: the gate never evaluates that population (see Context). Measuring the
  correct population first, before touching any constant, was the decisive step — it showed the
  problem was narrower than originally framed (one lattice failing, not five).
- Rejected changing hysteresis alone at the current threshold: the hysteresis-axis sweep shows
  extent 12 never discriminates at threshold 4096 regardless of hysteresis, because the field's mean
  there sits structurally above 4096.
- Rejected the first candidate found (6144/1536 at a single sweep point) without a neighbourhood
  check: re-ran at 5632/5888/6144/6400/6656 to confirm it is a plateau, not a one-point spike, before
  accepting it.
- Rejected trusting the purified `(transitions, ever-active)` proxy alone as sufficient evidence:
  re-verified the chosen point with real production runs against the exact five-field tuple the
  original TODO evidence used, so the claim is checked against what was actually measured before,
  not a narrower substitute.
- `different_seeds_produce_different_worlds_not_one_world_with_two_terrains` broke because its
  hand-picked seed pair (7, 30) — chosen after the `TODO-GEO-005` fix specifically because it
  discriminated then — collapsed onto the same behaviour tuple under the recalibrated gate. Re-swept
  the same seed range under the new constants and re-pointed the test at (7, 5).
- Did not assume the recorded-trace replay's decoupling claim; checked it directly with a feedback
  test (real runs at both constants, same seed). It found a real but small effect (see Benchmark
  plan), which changes the replay's epistemic status from "exact" to "approximate screen" and is why
  the end-to-end check, not the replay, is cited as the accepting evidence throughout this plan.
  `record()` was changed to pin a fixed reference config rather than inherit `RuntimeConfig::new`'s
  current default, for reproducibility independent of this finding.

## Progress

- Wave 1 (measurement tool), checkpoint `a03856b`:
  `apps/observer/src-tauri/examples/mana_gate_calibration.rs` added.
  Verified: `cargo build --release -p causafera-observer --example mana_gate_calibration`,
  `cargo clippy --release -p causafera-observer --example mana_gate_calibration -- -D warnings`.
- Wave 2 (constant change + test fix), checkpoint `7979936`:
  `crates/causafera-runtime/src/config.rs` (`effect_threshold` 4096 → 6144, `effect_hysteresis` 2000
  → 1536), `crates/causafera-runtime/tests/terrain_carrier.rs` (seed pair 7/30 → 7/5 in
  `different_seeds_produce_different_worlds_not_one_world_with_two_terrains`).
  Verified: `cargo test --release --workspace` (zero failures), `cargo fmt --all -- --check`,
  `cargo clippy --release -p causafera-runtime --all-targets -- -D warnings`.
- Wave 3 (documentation), checkpoint `02401c0`: `docs/development/todo-backlog.md`,
  `docs/ontology/domain-coverage-matrix.md`, `CHANGELOG.md`, `PLANS.md`.
- Checkpoint hashes recorded, checkpoint `833732b`.
- Wave 4 (feedback verification + stale-constant sync): pending checkpoint.
