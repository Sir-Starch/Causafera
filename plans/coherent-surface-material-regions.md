# Coherent Surface Material Regions ExecPlan

**Status:** Accepted and implemented.

## Goal

Generate surface materials as spatially coherent regions rather than per-cell independent
assignments, so a chart has real material structure at a scale larger than one cell (`TODO-GEO-004`).

## Context

`terrain_cells` (`crates/causafera-runtime/src/carrier.rs`) derived a cell's material from the same
well-mixed per-cell hash that drives elevation: `material_band = (base >> 17) & 0xF`. This makes
material independent noise, measured (this plan's own evidence, and the historical `terrain_probe.rs`
tool this backlog entry originally cited) at 6.5%–6.75% same-material neighbours against 6.25%
expected by chance over sixteen materials — no more coherent than chance.

This is not cosmetic. `terrain_structure`'s `material_delta` term (`carrier.rs`) is a `.max()` over up
to four neighbours of `(center ^ neighbor).count_ones()`. Independent noise makes this nonzero for
almost every cell (only exactly zero if all sampled neighbours happen to share the center's material,
improbable at 16 materials), so it acted as a de facto constant floor: measured at a mean contribution
of 50.0 of 204.8 (24.4%) to `terrain_structure` across three seeds, essentially flat across seeds
(50.1, 50.0, 50.0). `terrain_structure`'s own doc comment states this should not exist: "a floor under
every cell would make a flat plain drive the field as hard as a ridge does."

`TODO-MANA-004`'s own evidence reached the same finding from the mana side: projected onto the mana
lattice, the terrain's structural variation survives at only ~1.14x–1.27x what averaging pure noise
would retain (re-measured for this plan under identical conditions; see Benchmark plan), and the
ratio does not clearly improve as the lattice refines. There is little coherent structure for a finer
field to resolve — this plan is the work that would change that.

## Relevant invariants

- INV-018 — performance and behavioural claims are benchmarked; every number below comes from a real
  run, not an estimate.
- INV-038 — digests are equality/divergence anchors only; "changed" claims below are measured
  inequalities.
- Determinism (`docs/architecture/determinism.md`) — no RNG, floats, or non-canonical iteration in
  the authoritative generation path.

## Ontology domains affected

Geography only. No new domain state, no new carrier, no wire protocol change. This is a correction to
how one existing field (`TerrainCell::surface_material`) is generated.

## Causal carriers affected

The standing terrain carrier's structure computation (`terrain_structure`, `material_difference`)
reads whatever `surface_material` values `terrain_cells` produces; this plan changes what those
values are, not how they are read or projected.

## Relevant documents

- `docs/development/todo-backlog.md` — `TODO-GEO-004`; `TODO-MANA-004` (whose fidelity numbers move
  here, see Benchmark plan and Decision log — not reopened by this plan, flagged for its own review).
- `plans/terrain-chunk-boundary-continuity.md` (`TODO-GEO-005`) — established the chart-position-based
  (not chunk-position-based) generation idiom this plan reuses for material regions.
- `plans/terrain-structure-cross-chunk-neighbours.md` (`TODO-GEO-006`) — the cross-chunk neighbour
  resolution this plan's regions must remain continuous through.

## Current state

`crates/causafera-runtime/src/terrain_regions.rs` (new module) implements a bounded Worley (cellular)
partition: a coarse grid of feature points, each owning the region of space nearest to it, searched
over a 5x5 (Chebyshev-2) neighbourhood of coarse cells. `terrain_cells` (`carrier.rs`) calls
`terrain_regions::region_material(chart_seed, global_x, global_y)` in place of the old per-cell hash
band; `chart_seed` and `global_x`/`global_y` are the same chart-scoped position values `TODO-GEO-005`
established for elevation, so regions are continuous across chunk boundaries by construction, not by
a special case.

## Proposed architecture

A pure function of chart-global position:

```text
coarse_x = global_x.div_euclid(MATERIAL_REGION_SIZE)
coarse_y = global_y.div_euclid(MATERIAL_REGION_SIZE)
for (dx, dy) in the 5x5 neighbourhood of (coarse_x, coarse_y):
    feature = hash(chart_seed, coarse_x + dx, coarse_y + dy)  → jittered position + material
material at (global_x, global_y) = material of the nearest feature by squared distance
```

**Why Chebyshev-2 (5x5), not the more common Chebyshev-1 (3x3):** jitter is confined within its own
coarse cell (range `[0, MATERIAL_REGION_SIZE)`). The own cell's feature alone bounds the best
candidate at at most `sqrt(2) * MATERIAL_REGION_SIZE` (the cell diagonal), which is less than
`2 * MATERIAL_REGION_SIZE` — the minimum possible distance from any point in the query's own cell to
any point in a Chebyshev-3 cell. A 3x3 search is therefore not exact for full-cell jitter; Chebyshev-2
is the provably sufficient radius. The extra 16 hash evaluations per cell (25 against 9) are
bootstrap-time only and immaterial next to the mix64 calls already running per cell.

**Why `MATERIAL_REGION_SIZE = 16`, a power of two:** swept 4/8/16/32/64 against same-material
neighbour rate (interior and across a synthetic chunk boundary) and against the mana column footprint
(`CHUNK_SIZE / chunk_extent`, 10.7 cells at the production default extent 3). See Benchmark plan for
the full table. 16 sits close to that footprint (~1.5x), giving four regions per chunk on average:
coherent enough to give a finer mana lattice real region structure to resolve, without collapsing a
whole chunk to one material the way 32 or 64 tend to. A power of two lets jitter use a bitmask
(`key & (MATERIAL_REGION_SIZE - 1)`) rather than a modulo, which is bias-free only at a power of two.

## Primitive vs emergent review

Material identity and its spatial layout are primitive geography (`RFC-GEO-002`), same as elevation.
This plan changes the generation function, not what is primitive; no biome semantics, climate coupling
or named regions are introduced (explicitly out of scope, below).

## Non-goals

- Biome semantics, climate coupling, or named regions (`TODO-GEO-004`'s stated Out of Scope).
- Changing elevation or roughness generation.
- Changing the mana lattice or `chunk_extent` (that is `TODO-MANA-004`'s decision, flagged not
  reopened; see Decision log).
- Changing the material count (still sixteen, `MaterialId::new(1..=16)`).

## Implementation stages

**Wave 1 — region generator, wiring, and test fallout.**
Added `crates/causafera-runtime/src/terrain_regions.rs` with `region_material` and four unit tests
(coherence, chunk-boundary continuity, determinism, cross-chart independence). Wired `terrain_cells`
to call it in place of the old per-cell material hash; bumped `TERRAIN_GENERATOR` and
`TERRAIN_PARAMETERS` fingerprints (both changed: a new algorithm and a new tunable parameter,
`MATERIAL_REGION_SIZE`).

Three pre-existing tests broke, none from a defect in the region generator itself:

- `enabled_recipe_source_commits_once_and_drives_production_loop`
  (`material_surface_loop.rs`) asserted a recipe source's target cell started at intensity 0, but its
  config leaves `terrain_participation` at the default `Standing` — it was never actually isolated
  from terrain, and the old generator's noise happened to read zero at that exact cell. Fixed by
  setting `TerrainParticipation::Inert` explicitly, matching the test's own stated intent and its
  sibling isolation tests in the same file, rather than re-pinning a value a future generator change
  would move again.
- `below_threshold_source_changes_mana_without_material_consequence_or_supported_explanation`
  (`material_surface_observer.rs`) deliberately leaves terrain `Standing` to test source + terrain
  interaction, and pins the resulting intensity (182, itself a re-pin from `TODO-GEO-005`/`006`).
  Re-measured and re-pinned to 154, with a comment explaining the cause: coherent regions activate the
  field's spatial-repetition channel on columns that previously read independent noise.
- `different_seeds_produce_different_worlds_not_one_world_with_two_terrains` (`terrain_carrier.rs`)
  asserted two hand-picked seeds (7 and 5, itself a re-pin from `TODO-MANA-007`) produce different
  behaviour tuples; this pair collapsed a second time. Rather than pin a third pair, rewrote the test
  to sweep eight fixed seeds and assert the coarse behaviour tuple does not collapse across all of
  them — the actual claim, independent of which specific seeds a future change leaves discriminating.
  Also found and fixed an unrelated latent issue while re-sweeping: the test's original 48-tick
  duration is too short for this claim regardless of generator version (59 of 60 sampled seeds share
  one tuple at 48 ticks; five distinct tuples appear by 192), so ticks moved to 192, matching the
  duration `extent_decision.rs` and `mana_gate_calibration.rs` already use for the same reason.

**Wave 2 — evidence.**
Added a `TODO-GEO-004` block to `apps/observer/src-tauri/examples/field_probe.rs` measuring
same-material rate across a real chunk boundary against the interior rate, mirroring the block
`TODO-GEO-005` added for elevation. `field_probe.rs` already printed per-chunk same-material
neighbour rate (added generically during earlier terrain work), so no addition was needed there.

**Wave 3 — documentation.**
Updated `docs/development/todo-backlog.md` (close `TODO-GEO-004`; flag `TODO-MANA-004` for its own
re-review), `docs/ontology/domain-coverage-matrix.md` (Geography row), `CHANGELOG.md`, `PLANS.md`.

## Verification

- `cargo test --release --workspace` — full workspace, zero failures (see Implementation stages for
  the three fixes required).
- `cargo fmt --all -- --check` — clean.
- `cargo clippy --release -p causafera-runtime --all-targets -- -D warnings` — clean.
- Four new unit tests in `terrain_regions.rs`: coherence rate above 50% (well above the 6.25% chance
  floor and the measured 93%+ production rate), chunk-boundary continuity within 25 points of the
  interior rate, same-position determinism, cross-chart independence.

## Benchmark plan / measured evidence

**Region-size decision sweep** (three seeds, same-material rate; "regions/chunk" is `(32/region_size)²`):

| region_size | interior rate | boundary rate | regions/chunk |
|---|---|---|---|
| 4 | 72.53% | 64.58% | 64.0 |
| 8 | 85.27% | 77.08% | 16.0 |
| **16** | **93.15%** | **93.75%** | **4.0** |
| 32 | 96.22% | 94.79% | 1.0 |
| 64 | 98.08% | 100.00% | 0.2 |

All candidates trivially clear "substantially above the 6.25% chance floor" — that criterion alone
does not discriminate between them. 16 was chosen because its boundary rate is closest to its
interior rate (no sign of a boundary artifact even at this small sample) and it sits closest to the
mana column footprint (10.7 cells at the production default `chunk_extent` 3), giving multiple
regions per chunk rather than collapsing a chunk to near-uniform material.

**Structure decomposition, old generator vs new** (mean `terrain_structure` = elevation contribution +
material contribution + roughness contribution, three seeds):

| | elevation | material | roughness | total | same-material rate |
|---|---|---|---|---|---|
| Old (per-cell noise) | 90.5 | 50.0 | 63.8 | 204.4 | 6.5%–6.75% |
| New (region_size 16, real production run) | — | — | — | — | 93.0%–94.1% |

The old material term (50.0 of 204.4, 24.5%) was the constant floor described in Context. The new
generator's real production same-material rate (93.0%–94.1%, from `field_probe.rs`, three chunks)
matches the region-size-16 sweep prediction (93.15%/93.75%) closely.

**Chunk-boundary continuity, real production run** (`field_probe.rs`, seed 7, three adjacent
line-shaped chunks):

```text
interior same-material rate 93.1% (2976 pairs) | boundary same-material rate 90.6% (64 pairs)
```

Same order as the interior rate — no sign of the ~30-point systematic gap `TODO-GEO-005` found and
fixed for elevation continuity.

**Mana-lattice fidelity, before vs after** (`extent_decision.rs`, identical seeds/ticks/threshold,
measured immediately before and after this change on the same commit range):

| extent | variation retained, before | vs noise, before | variation retained, after | vs noise, after |
|---|---|---|---|---|
| 3 | 1.0% | 1.14x | 1.0% | 1.19x |
| 4 | 2.0% | 1.27x | 2.6% | 1.66x |
| 6 | 4.1% | 1.15x | 5.6% | 1.59x |
| 8 | 7.7% | 1.23x | 9.5% | 1.52x |
| 12 | 16.1% | 1.15x | 18.7% | 1.33x |

Every candidate extent retains more structure relative to noise after this change, most at extents
4–8 (vs-noise ratio up 31%–38%). Extent 3 (the production default) barely moves. Discrimination
(distinct behaviour tuples across six seeds) and cost (ms/tick, cells/tick) are both unaffected within
measurement noise — full tables in the tool's own output; not reproduced here since neither is what
this plan changes.

**Bootstrap cost**: unaffected. `region_material` runs at generation time only (once per chunk, not
per tick), and its 25-candidate search is a fixed, extent-independent cost already dominated by the
generation-time `mix64` calls `terrain_cells` was already making per cell.

## Determinism impact

`region_material` is a pure function of `chart_seed` and chart-global position: no RNG, no floats, no
hash-iteration order dependence. Same seed reproduces identically (covered by
`the_same_chart_position_always_yields_the_same_material`); different charts can disagree at the same
position (covered by `different_charts_can_disagree_at_the_same_position`). `TERRAIN_GENERATOR` and
`TERRAIN_PARAMETERS` both moved to `0x2409_0001`, so every world's terrain (and therefore physical
digest) changes by construction, exactly as `TODO-GEO-005` established as the precedent for this class
of change. No checked-in fixture or replay capture exists in the repository, so none needed
regeneration.

## Memory impact

None; no new persisted fields. `MATERIAL_REGION_SIZE` and the region algorithm are generation-time
constants, not stored state.

## Observer impact

Unblocks a landcover lens (`plans/observer-field-raster-map.md` deliberately excluded one, citing this
TODO), which this plan does not implement — the lens is a batched UI milestone per the project's
stated priority order and is left for its own work.

## Explanation impact

None beyond what `TODO-GEO-005`/`006` already established: material is already projected to the
observer and Explanation layers; this plan changes its spatial distribution, not its provenance or
exposure.

## Persistence impact

None; the persisted terrain snapshot format is unchanged, only the generator's output.

## Cross-domain effects

Mana: the standing terrain carrier's structure magnitude changes (see Benchmark plan's fidelity
table), which is the intended effect. `TODO-MANA-004`'s "extent stays 3" decision was made against
weaker fidelity numbers (vs-noise ratios of 1.14x–1.27x); post-this-plan numbers are stronger
(1.19x–1.66x) at every candidate extent. This plan does not reopen or redecide `TODO-MANA-004` — that
decision also depends on cost (unaffected here, still 31x at extent 12) and discrimination (unaffected
here) — but flags it in the backlog as due for its own re-review with current numbers.

## Risks

- `MATERIAL_REGION_SIZE = 16` is a single chosen point, not swept against the full mana-fidelity
  pipeline for every candidate size (only against same-material rate, a cheaper proxy). If a future
  change to the mana lattice or gate makes region scale matter more precisely, this constant should be
  re-swept against the real fidelity metric, not assumed.
- The structure-decomposition "after" figures in the Benchmark plan report the real production
  same-material rate but not a full elevation/material/roughness re-decomposition matching the "before"
  table's three-way split; the real production same-material rate is the figure that matters for this
  plan's acceptance criterion and is reported precisely.

## Documentation changes

- `docs/development/todo-backlog.md` — `TODO-GEO-004` marked Completed; `TODO-MANA-004` flagged for
  re-review with current fidelity numbers.
- `docs/ontology/domain-coverage-matrix.md` — Geography row.
- `CHANGELOG.md`, `PLANS.md`.

## TODO changes

`TODO-GEO-004`: Pending → Completed.

## Decision log

- Rejected 3x3 (Chebyshev-1) neighbourhood search as used in many Worley-noise references: proved it
  is not exact for jitter confined to a full coarse cell (own-cell worst case `sqrt(2)*R` exceeds a
  Chebyshev-2 cell's minimum possible distance in the adversarial case), so widened to 5x5
  (Chebyshev-2), which the same bound proves sufficient. Chose exactness over the smaller, more
  common but unproven radius, since the cost difference (16 extra bootstrap-time hashes per cell) is
  immaterial.
- Rejected halving jitter range to guarantee 3x3 sufficiency instead: would have worked, but 5x5 with
  full-cell jitter gives the same guarantee with simpler, more legible code (no separate "half-cell"
  invariant to maintain alongside the coarse-cell arithmetic).
- Rejected `MATERIAL_REGION_SIZE` values 32 and 64: both drop to ~1 or fewer regions per chunk on
  average, collapsing most chunks toward a single material and losing the intra-chunk variety a
  chosen 16 keeps (four regions per chunk on average).
- Did not reopen `TODO-MANA-004`'s lattice-extent decision, despite its fidelity numbers moving
  favourably here: that decision also weighs cost (unaffected) and discrimination (unaffected), and a
  change this narrow in scope (Out of Scope explicitly excludes touching the lattice) is not the place
  to redecide it. Flagged in the backlog instead.

## Progress

- Wave 1 (region generator, wiring, test fallout): pending checkpoint.
- Wave 2 (evidence): pending checkpoint.
- Wave 3 (documentation): pending checkpoint.
