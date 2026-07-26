# Terrain Carrier Participation ExecPlan

**Status:** Accepted and implemented.

## Goal

Make the world seed reach the running simulation, so two seeds produce two different worlds rather
than one world with two terrains (`TODO-RUNTIME-002`).

## Context

`RuntimeConfig::new(seed)` sets `DeterministicConfig::world_seed` and
`CarrierAdapterConfig::terrain_seed`. The seed reaches terrain generation, which measurably varies:
over one chunk, seeds 7, 11, 23, 41, 59 and 97 give 815, 805, 808, 834, 809 and 816 distinct
terrain fingerprints and mean elevations from 1960 mm to 2094 mm.

It reached nothing else. Measured before this work with
`cargo run --release -p causafera-observer --example seed_variation`, all six seeds produced a
byte-identical physical state digest, a total mana of 32 266, three mana gate crossings, three
gate transitions, 251 total surface condition, 245 committed actions and a population of 493,
after 192 ticks of the same production-shaped configuration.

Two independent facts produced that:

1. `PhysicalPatternSystem::execute` filled `pending_samples` only from
   `MaterialSurfaceCarrierAdapter`. `TerrainCarrierAdapter::emit_samples` existed, was tested, and
   was never called by any system. `carrier_adapters` was read by bootstrap, by resolution
   relevance and by the snapshot export, and by nothing that evolves state.
2. No registered system consumes the `RandomStream` the scheduler threads through `System::run`.
   Every implementation binds it as `_stream`. The seed therefore had exactly one live path into
   the simulation — terrain — and that path terminated.

The seed also reaches the root event's `after` fingerprint, which is why the history digest already
varied. A varying history digest over an identical physical state is the signature of a seed that
labels a world without shaping it.

## Relevant invariants

- INV-013 — observation never drives simulation. The carrier's participation is a property of the
  world, not of who is watching.
- INV-021, INV-022 — a rendering is not simulation state; the observer projection of terrain is
  unchanged by this work.
- INV-036, INV-037 — chart-qualified addressing; resolution may change detail but not topology or
  geometry. The projection of terrain onto the mana lattice is a resolution decision.
- INV-038 — digests are equality anchors only. "Six distinct digests" here means six distinct
  worlds, never six measured distances between worlds.
- INV-039 — no fixture or demo constructor in the production path. Terrain reaches the loop through
  `HistoricalBootstrapPlan` and the registered `PhysicalPatternSystem` only.

## Ontology domains affected

Geography and local physical space, as the source of a standing structure; mana, as the field that
responds to it; causal resolution and the observer, only through values that already existed. No
cognitive, social, biological or linguistic domain is touched.

## Causal carriers affected

`TerrainCarrierAdapter` — no carrier is added, removed, or given a new schema. Its emission
contract changes from a per-cell raster to a projection onto the mana lattice, and it acquires a
stated participation contract in `RuntimeConfig`.

## Relevant documents

- `docs/rfc/RFC-MANA-001.md` — the field model and its structural response channels. Its deferred
  work lists "coupling to concrete acoustic, geometric, biological, material, glyph, and
  practice-emission producers"; this plan delivers the geometric producer.
- `docs/ontology/causal-carriers.md` — the carrier boundary record.
- `docs/rfc/RFC-GEO-002.md`, `docs/world/terrain.md` — terrain as authoritative world state.
- `plans/local-mana-material-surface-coupling.md` — the accepted loop this must not break.
- `plans/observer-field-raster-map.md` — the measured terrain and mana field properties used here.
- `docs/architecture/performance.md` — cost claims must be measured.

## Current state

Recorded above and in `TODO-RUNTIME-002`. In addition, two defects were found in the emission path
while reading it:

- `TerrainBootstrapStage::bootstrap` rebuilt every adapter with a hard-coded `field_extent` of 3,
  discarding the extent `runtime_carrier_adapters` had just been given. Any configuration with
  `chunk_extent != 3` therefore held carriers projecting onto the wrong lattice.
- `field_position` was shared by both carriers with a single body that decomposed its index as a
  `CHUNK_SIZE`-wide raster and then took `x % extent`. For terrain that is a stride comb across the
  whole chunk rather than a projection; for a material surface, whose index is a mana cell index in
  `extent³`, it is simply the wrong decomposition, giving the wrong position for any cell index
  above zero. Only cell zero exists today, so the second was latent.

## Proposed architecture

### Terrain participates as a standing structure, not as an event stream

A material surface emits when it changes; that is what `pending_material_surface_changes` records.
Terrain does not change in this milestone, so the natural reading — emit on change — would put its
whole causal contribution at bootstrap. That was measured and rejected: a one-shot emission
perturbs the transient and not the attractor. At 768 ticks the six seeds' gate transitions
reconverged to an identical sequence after tick 30, so the world was still one world reached by
slightly different routes.

Terrain is instead treated as what it is: a standing spatial structure that is continuously present.
RFC-MANA-001's response channels include spatial repetition precisely for this, and that channel had
no producer at all — a single material surface per chunk occupies one position, so `spatial` was
always zero.

### Standing structure earns no temporal credit

The decisive constraint, and the one that took measurement to find: **a standing structure must not
be scored for the rate at which it is read.**

`pattern_score` reads recurrence, periodicity and synchronisation from `pattern_history`. A carrier
re-emitting an unchanged raster every tick has, by construction, perfect periodicity and maximal
recurrence over any window. Measured with terrain retained in the history, total mana ran to
696 573 against a surface-driven baseline of 32 266 — a factor of 21 — the gate sat permanently
open, and gate transitions collapsed from 32 to 3 over 768 ticks. The field was scoring the
carrier's read cadence, not the world.

Terrain samples therefore reach `pending_samples` and never `pattern_history`, and are not counted
in `physical_events`. Both of those retain change. The response terrain does earn comes from
within-tick structure: columns of the same composition in the same chunk form a group, which scores
on synchronisation and spatial repetition. That is the channel the model provides for repeated
spatial structure, and it is the only one a static field is entitled to.

### The carrier presents its structure at the field's own lattice

Terrain is `CHUNK_SIZE²` = 1024 cells in plan view; the mana field is `extent³`. Emitting one
sample per terrain cell drives 1024 samples into `extent²` columns — a hundredfold over-sampling at
the default extent. That is a resolution mismatch rather than physics, and it also makes
`propose_evolution`'s per-group scan quadratic in a way the lattice never benefits from.

`TerrainCarrierAdapter` now emits one sample per plan-view column of the mana lattice:

- membership is a block projection, `x * extent / CHUNK_SIZE`, so a column summarises a contiguous
  patch of ground and the landform survives; the previous `x % extent` comb did not;
- `magnitude` is the column's mean per-cell **structure** — relief contrast, surface-material
  discontinuity and roughness. The former `128 +` floor is gone: featureless ground is not a
  physical pattern, and a floor would let a flat plain drive the field as hard as a ridge. A
  uniform chunk now emits nothing at all;
- `pattern` is a fingerprint of the column's dominant surface material and its roughness class.
  Roughness is quantised into 32 mm classes so that comparably rough ground shares a fingerprint and
  the spatial channel can see the repetition; a millimetre-exact mean would make every column unique
  and leave that channel silent. The fingerprint is derived from physical composition only, exactly
  as a single cell's is;
- `source_ordinal` is the column ordinal, and `source_column` resolves it back to the patch of
  ground it summarises. `source_cell` and the per-cell raster the observer projects are unchanged;
- `cause` is the terrain's own `generation_trace`, which is the truthful answer to why this
  structure is here at this tick. A mana cell change therefore carries terrain generation in its
  causal ancestry.

The projection is derived once, in `TerrainCarrierAdapter::new`, because recomputing a static
summary per tick is exactly what a standing carrier must not cost.

### The participation is a stated contract

`RuntimeConfig::terrain_participation` is a validated `TerrainParticipation`:

- `Standing` (default) — the carrier presents its structure on every tick the physical pattern
  schedule emits, so an experiment's suppression window still suppresses the whole physical source;
- `Inert` — the carrier never reaches the tick loop. This is the behaviour `TODO-RUNTIME-002`
  records, retained so a configuration can isolate the rest of the loop from terrain, which the
  experiment-recipe and single-contact tests now do explicitly.

It is persisted in the runtime recipe section, whose major version rises from 4 to 5. A resumed
world that silently began sensing its terrain would be a different world, so this cannot be
defaulted on read.

## Primitive vs emergent

Primitive: the terrain raster, its per-cell structure, the lattice projection, the column
fingerprint, and the participation contract.

Emergent: where mana stands in a world, when a local gate crosses, how many times a surface
condition advances, and how far two seeds' worlds diverge. None of these is written down anywhere;
all of them now vary with the seed.

Not introduced: mana types, terrain semantics, landcover regions, biomes, named places, or any
classification of ground. A column's fingerprint is an opaque composition hash, not a terrain type.

## Non-goals

- Changing terrain generation. `deterministic_terrain_chunk` and `terrain_cells` are untouched.
- Adding a carrier, a schema, or a wire projection.
- Cross-chart transport.
- Recalibrating `ManaParameters`. The field is now larger because the world is no longer empty;
  whether the response constants should move is a separate, measured question.
- Making terrain change over time. Geomorphology remains future work, and when it arrives the
  carrier will additionally emit on change.

## Implementation stages

1. Split the shared `field_position` into `terrain_field_position` (a `CHUNK_SIZE²` block
   projection onto the lattice's plan view) and `field_position` (a correct `extent³`
   decomposition), and fix the hard-coded extent in `TerrainBootstrapStage`.
2. Add `TerrainColumn`, `project_columns`, `terrain_structure` and `TerrainCarrierAdapter::columns`
   / `source_column`, and rewrite `PhysicalCarrierAdapter::emit_samples` as the lattice projection.
3. Add `TerrainParticipation` to `RuntimeConfig`, persist it in the runtime recipe section, and
   raise `RUNTIME_RECIPE_SECTION_MAJOR` to 5.
4. Emit terrain from `PhysicalPatternSystem::execute` into `pending_samples` only, gated on the
   pattern schedule and the participation contract.
5. Tests and evidence: carrier unit tests, runtime acceptance tests, and the `seed_variation`
   evidence tool.

## Verification

`cargo run -p xtask -- ci` is green: 369 tests pass, against 358 before this work.

Direct acceptance coverage, in `crates/causafera-runtime/tests/terrain_carrier.rs`:

- `different_seeds_produce_different_worlds_not_one_world_with_two_terrains` — seeds 7 and 59 over
  48 ticks differ in physical state digest, total mana, mana gate crossings, gate transition count
  and total surface condition.
- `the_same_seed_still_reproduces_itself_exactly` — the same seed still gives an identical summary
  and an identical exported snapshot.
- `terrain_reaches_the_field_and_the_seed_reaches_terrain` — with no actor and no contact, two
  seeds both produce a non-empty and unequal field.
- `an_inert_terrain_carrier_leaves_an_empty_world_empty` — the recorded prior behaviour.
- `terrain_participation_survives_a_snapshot_round_trip` — an inert world resumes inert, and a
  standing control confirms the flag is what makes the difference.

Carrier coverage, in `crates/causafera-runtime/src/carrier.rs`:
one sample per lattice column at extents 3, 4, 6 and 8; every terrain cell reaching exactly one
column; a column covering a contiguous patch rather than a stride comb; equal composition giving an
equal fingerprint; an ordinal resolving back to its ground; and featureless ground emitting nothing.

Two existing tests changed meaning rather than failing incidentally, and both were re-pointed
explicitly rather than relaxed:

- `material_surface_loop_without_repetition_has_no_mana_material_consequence` now sets
  `TerrainParticipation::Inert`. It measures the mana model's requirement for repetition, which is
  unchanged; a world with standing terrain is simply no longer empty when the first contact lands.
  `standing_terrain_sustains_the_field_a_contact_lands_in` was added to state the new fact.
- The experiment-recipe observer and explanation tests set `Inert` alongside the diffusion and decay
  they already switch off, so the cell the recipe targets holds the recipe's contribution and
  nothing else.

## Benchmark plan

Measured with `cargo run --release -p causafera-observer --example seed_variation`, 192 ticks,
three active chunks, against the same world with the carrier inert.

| `chunk_extent` | samples/tick | inert ms/tick | standing ms/tick | overhead | inert changed cells/tick | standing |
|---|---|---|---|---|---|---|
| 3 (default) | 27 | 1.351 | 1.445 | **+7.0 %** (see below) | 72 | 74 |
| 6 | 108 | 2.563 | 9.050 | +253 % | 134 | 577 |
| 8 | 192 | 3.326 | 17.667 | +431 % | 134 | 1088 |
| 12 | 432 | 6.788 | 45.386 | +569 % | 134 | 2731 |

At the default lattice the carrier costs single-digit percent, and the field was already fully live,
so the two columns of changed cells agree. The overhead figure is timing noise sensitive and the
table's single run overstates its own precision: re-running the same measurement six times gave
7.0 %, 7.5 %, 7.6 %, 8.7 %, 9.4 % and 17.3 %. Read it as "under ten percent on a quiet machine, with
outliers", not as a constant. The structural counts beside it are exact and reproduced identically
on every run: 27 samples per tick, and 72 changed cells per tick inert against 74 standing. Above it the growth is not the carrier: it is that terrain
populates cells which were previously dead, and the runtime commits one causal event per changed
mana cell per tick. Per changed cell the cost does not rise — at extent 12, 0.052 ms per changed
cell standing against 0.076 ms inert. The carrier's own work is the sample projection, which is
`extent²` samples read from a precomputed summary.

This is direct evidence for `TODO-MANA-004`: a finer lattice is now materially more expensive than
its earlier measurement suggested, because the field it prices is genuinely populated.

## Determinism impact

Unchanged in kind, changed in value. The same seed still reproduces identically, verified by
`the_same_seed_still_reproduces_itself_exactly`, `terrain_carrier_determinism` and the existing
replay tests. The projection uses integer arithmetic, a `BTreeMap` for the dominant-material tally
with a lowest-identifier tie-break, and canonical row-major column order, so no iteration order,
float, or hash seed can reach it.

Every physical digest changes by construction. This is the intended result, not a regression: the
digest is what records that the world is now the seed's world.

## Memory impact

`TerrainCarrierAdapter` gains a derived `Vec<TerrainColumn>` of `extent²` entries — 9 columns per
chunk at the default extent, 1024 at the maximum. It is not persisted and not compared, being a
pure function of the terrain and the extent already stored beside it.

## Observer impact

None to the wire protocol or to any projection. The per-cell terrain raster the observer reads is
unchanged, and `source_cell` still resolves it. The mana field the observer already projects is now
populated and seed-dependent, which is a change in what there is to see rather than in how it is
seen.

## Explanation impact

A mana cell change now carries the terrain's `generation_trace` among its causes, so a claim about
where mana stands can be traced to the generation of the ground under it. No claim schema changes.

## Persistence impact

`RUNTIME_RECIPE_SECTION_MAJOR` rises from 4 to 5 for the added `terrain_participation` byte.
Sections fail closed on an unknown major, so a v4 snapshot is rejected rather than silently
defaulted. There are no recorded snapshot fixtures in the repository; round-trip coverage is
in-memory and passes.

## Cross-domain effects

The mana field is no longer near-empty in a world with no actors, which changes the baseline every
mana-dependent measurement sits on. `plans/observer-field-raster-map.md` recorded 27/27, 27/27 and
23/27 populated cells; those numbers were measured on a field driven only by contact and should be
re-taken. Resolution relevance reads `pattern_event_counts_by_chunk`, which is computed from
`pattern_history` and therefore unaffected by terrain by construction.

## Risks

- **The field is larger.** Total mana at 192 ticks runs 50 416 to 116 189 across the six seeds
  against a prior 32 266. Configurations tuned against an almost-empty field may cross their gates
  differently. Mitigated by `Inert`, by the surviving gate cycle measured below, and by leaving
  `ManaParameters` untouched so the change is attributable to the carrier alone.
- **Column fingerprints use dominant surface material**, and
  `plans/observer-field-raster-map.md` measured the material field as spatially random. The
  fingerprint is therefore a weak spatial signal. It is not a claim that regions exist — no
  landcover is asserted anywhere — and the structure magnitude, which carries relief, does the
  larger part of the work.
- **Divergence is not proportional.** Two seeds differ; how much they differ is not a metric, per
  INV-038. The acceptance criterion is inequality, and the plan claims nothing more.

## Decision log

- **Accepted:** terrain participates continuously as a standing structure.
- **Accepted:** terrain earns spatial and within-tick response only, never temporal recurrence
  credit; it does not enter `pattern_history` or `physical_events`.
- **Accepted:** the carrier presents its structure at the mana lattice's resolution, one sample per
  plan-view column.
- **Accepted:** featureless ground emits nothing; the constant magnitude floor is removed.
- **Accepted:** participation is a persisted configuration contract with an explicit `Inert`
  variant, not an implicit consequence of which system reads `carrier_adapters`.
- **Rejected:** one sample per terrain cell per tick. Measured: it over-samples the lattice a
  hundredfold, floods a 512-entry history, and makes the per-group scan quadratic in a count the
  field cannot represent.
- **Rejected:** emitting once at bootstrap. Measured: gate transitions reconverged to an identical
  sequence after tick 30 of 768, so the seed shaped the transient and not the world.
- **Rejected:** terrain retained in `pattern_history`. Measured: total mana 696 573 against a
  32 266 baseline, gate permanently open, gate transitions collapsing from 32 to 3. The model was
  scoring the read cadence.
- **Deferred:** recalibrating `ManaParameters` against a populated field.
- **Deferred:** emitting terrain on change, which needs terrain that changes.

## Documentation changes

`CHANGELOG.md`, `docs/development/todo-backlog.md` (`TODO-RUNTIME-002` completed,
`TODO-MANA-004` unblocked and re-costed), `docs/ontology/causal-carriers.md`,
`docs/ontology/domain-coverage-matrix.md`, `docs/roadmap/roadmap.md`, `docs/world/terrain.md`,
`docs/world/mana-topology.md`, and this plan.

## TODO changes

- `TODO-RUNTIME-002` — completed.
- `TODO-MANA-004` — no longer blocked. Multi-seed validation is now possible, and the extent cost
  it must weigh has been re-measured on a populated field.

## Progress

- Wave 1 — the whole slice, integrated and verified together: carrier projection, participation
  contract, persistence, emission, tests and evidence tool. Checkpoint `2e7c4a0`. Verified with
  `cargo run -p xtask -- ci` (green) and `cargo test --workspace` (369 passed, 0 failed, against a
  358-passing baseline before the change).
- One unrelated pre-existing `clippy::needless_range_loop` error in
  `apps/observer/src-tauri/examples/extent_bench.rs` was blocking `just ci` on this branch before
  any change here; it is fixed in place so the gate could run.
