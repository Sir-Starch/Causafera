# Terrain Structure Cross-Chunk Neighbours ExecPlan

**Status:** Accepted and implemented.

## Goal

Let the standing terrain carrier's mana-facing structure computation read real neighbour cells
across a chunk boundary, rather than only within the chunk that owns the cell being scored
(`TODO-GEO-006`).

## Context

`plans/terrain-chunk-boundary-continuity.md` (`TODO-GEO-005`) made elevation, roughness and
material a continuous function of a cell's position in its chart, closing the ~30 m seam at every
chunk boundary. It explicitly left one thing untouched: `elevation_contrast`, `material_difference`
and `neighbor_indices` in `crates/causafera-runtime/src/carrier.rs`, which feed
`terrain_structure` — the standing carrier's mana-facing magnitude — indexed only within the one
`TerrainChunk` passed to `TerrainCarrierAdapter::new`. `neighbor_indices` dropped a direction
entirely at `x == 0`, `x == CHUNK_SIZE - 1`, `y == 0` or `y == CHUNK_SIZE - 1`, so an edge cell's
structure was computed from 2–3 real neighbours where an interior cell has 4 — a chunk-boundary
artifact independent of, and left behind by, the elevation fix. This is `TODO-GEO-005`'s own
"Found while implementing it" note, and INV-043 territory: the engine must not rely on chunk
boundaries as barriers without an explicitly modelled physical process, and none was modelled here.

## Relevant invariants

- INV-009 — geography is causal state; the mana-facing reading of it should not depend on which
  chunk window happened to contain a cell.
- INV-043 — the world is one coherent spatial system; a missing cross-chunk mechanism is an
  unimplemented physical process to identify and close, not a permanent boundary.
- INV-017/INV-018 — performance is architectural and benchmarked, not patched afterward or claimed
  without measurement.
- INV-038 — digests are equality anchors only; every claim below about "changed" or "different" is
  a measured inequality, never a distance.

## Ontology domains affected

Geography and mana, exactly as `TODO-GEO-005`: no new domain state, no new carrier, no wire
protocol change. This is a correction to a derived carrier-magnitude computation.

## Causal carriers affected

`TerrainCarrierAdapter`. Its public shape gains one parameter (cross-chunk terrain context); its
schema, emission contract and persisted fields are unchanged.

## Relevant documents

- `plans/terrain-chunk-boundary-continuity.md` — the elevation fix this plan completes, and the
  origin of `TODO-GEO-006`.
- `docs/ontology/causal-carriers.md`, `docs/world/terrain.md` — terrain carrier documentation.
- `crates/causafera-domains/src/mana.rs` — `OpenNeighbors`, `open_neighbors_for` and
  `apply_boundary_exchange` are the established idiom this plan follows: a shared
  `BTreeMap<ChartChunkCoord, T>` of sibling state, looked up through
  `ChartChunkCoord::same_chart_neighbor`, rather than a bespoke neighbour-context type.

## Current state

Recorded above and in `TODO-GEO-006`. Three call sites construct `TerrainCarrierAdapter`:
`runtime_carrier_adapters` (`runtime.rs`), `TerrainBootstrapStage::bootstrap` (`bootstrap.rs`), and
`import_carrier_adapters` (`runtime.rs`, via `TerrainCarrierAdapter::import_snapshot`). Tracing
`RuntimeState::new`: `runtime_carrier_adapters` builds a placeholder `carrier_adapters` map keyed
correctly, and `HistoricalBootstrapPlan::bootstrap` — which always runs, unconditionally, inside
`RuntimeState::new` before it returns — overwrites every one of those entries before any tick or
query can observe them. Only `TerrainBootstrapStage::bootstrap`'s adapters ever reach a live,
ticking runtime; `import_carrier_adapters` reconstructs the equivalent set from a snapshot for
`Runtime::from_snapshot`, which restores `config` (and therefore the active chunk set) from
`data.recipe.config`, so the decoded snapshot's carrier set is always exactly the active chunk set
the exporting runtime had.

## Proposed architecture

### A shared map of sibling terrain, not a recomputed value

Two designs were considered. The first — recompute a missing neighbour's cell directly from the
deterministic per-cell generator function (`chart_seed`, chart-local position) introduced by
`TODO-GEO-005` — was rejected. `TerrainChunk` is data, not a cached formula evaluation:
`featureless_ground_is_not_a_physical_pattern` already constructs a hand-built uniform
`TerrainChunk` whose cell values do not match what the generator would produce for its stated seed,
and the project's own roadmap (`docs/world/terrain.md`) expects terrain to eventually carry
non-generated state (geomorphology, erosion). Recomputing from the formula would be silently wrong
for either, and would make the carrier layer duplicate generation logic it has no business owning.

The accepted design instead threads real, already-materialized sibling `TerrainChunk`s through:
`TerrainCarrierAdapter::new` and `project_columns` take
`neighboring_terrain: &BTreeMap<ChartChunkCoord, TerrainChunk>`. `neighbor_cells` resolves each of
a cell's four axis-aligned neighbours by reading the chunk's own array when the neighbour is
interior, or `neighboring_terrain.get(&chunk.same_chart_neighbor(dx, dy, 0))`'s corresponding edge
cell when it is not. A direction whose neighbouring chunk is absent from the map drops out — the
same graceful degradation an interior-chart edge always had — rather than inventing a flat or zero
value. This mirrors `causafera-domains::mana`'s established idiom (`OpenNeighbors`,
`open_neighbors_for`, `apply_boundary_exchange`) exactly: a shared map of sibling state, looked up
through `same_chart_neighbor`, rather than a bespoke per-adapter neighbour-context type.

### Two of the three call sites needed a two-pass restructure

`elevation_contrast`/`material_difference` need every sibling chunk's terrain generated (or
decoded) before any one adapter derives its columns, so both real production paths generate/decode
the whole active set first, then build every adapter against that shared map:

- `TerrainBootstrapStage::bootstrap` generates every chunk's `TerrainChunk` (and commits its trace)
  in a first pass into a `BTreeMap`, then builds every `TerrainCarrierAdapter` against that map in a
  second pass.
- `import_carrier_adapters` decodes every snapshot's `TerrainChunk` via the new `decode_terrain_chunk`
  free function in a first pass, then builds every adapter the same way.

`runtime_carrier_adapters` is the third call site and needed no such restructure: its adapters are
the placeholder set `TerrainBootstrapStage::bootstrap` unconditionally overwrites before
`RuntimeState::new` returns, so nothing ever observes their columns. It passes an empty map with a
comment recording why, rather than generating a batch of terrain that is guaranteed to be
discarded.

### `decode_terrain_chunk` replaces half of `import_snapshot`

The snapshot-to-`TerrainChunk` decode step was extracted from `TerrainCarrierAdapter::import_snapshot`
into a standalone `pub fn decode_terrain_chunk(snapshot: TerrainCarrierSnapshot) -> Result<TerrainChunk, TerrainChunkError>`.
`import_snapshot` itself now also takes `neighboring_terrain` and calls it, so it remains a correct,
if isolated, single-adapter convenience path (used by `terrain_patterns` in `terrain_carrier.rs`,
where an empty map is exact: `TerrainColumn::pattern()` reads only `dominant_material` and
`roughness_class`, never `structure`, so cross-chunk context cannot change which fingerprints that
helper collects).

## Primitive vs emergent

Primitive: the four-neighbour resolution, the shared sibling map, the decode/build split. Emergent:
which columns' `structure` actually differ once real neighbours are visible — measured, not
predicted, below.

## Non-goals

- Recomputing a neighbour's value from the generator formula (rejected above).
- Diagonal neighbours. `neighbor_cells` resolves the same four axis-aligned directions
  `neighbor_indices` always did; no isotropy or stencil-shape claim is made.
- Changing elevation, roughness or material generation. `terrain_cells` and
  `deterministic_terrain_chunk` are untouched.
- Fixing `runtime_carrier_adapters`'s redundant terrain generation (its output is provably
  discarded before any tick). Noted as a pre-existing inefficiency, independent of this TODO, and
  not fixed here.
- Any wire protocol or observer projection change.

## Implementation stages

1. `TerrainCarrierAdapter::new`, `project_columns`, `terrain_structure` take
   `neighboring_terrain: &BTreeMap<ChartChunkCoord, TerrainChunk>`. `neighbor_indices` is replaced by
   `neighbor_cells`, which returns resolved `TerrainCell` values instead of local indices;
   `elevation_contrast`/`material_difference` become plain comparisons over a cell and its resolved
   neighbours.
2. `decode_terrain_chunk` extracted as a standalone function; `TerrainCarrierAdapter::import_snapshot`
   gains the same neighbour parameter and calls it.
3. `TerrainBootstrapStage::bootstrap` (`bootstrap.rs`) and `import_carrier_adapters` (`runtime.rs`)
   restructured to a generate/decode-then-build two-pass shape. `runtime_carrier_adapters`
   (`runtime.rs`) passes an empty map with a comment explaining why its output is never read.
4. Tests: `crates/causafera-runtime/src/carrier.rs` gains
   `structure_near_a_chunk_edge_uses_the_real_neighbouring_terrain` (proves real neighbour data
   changes `columns()` against an empty-map build — the test that would fail if a production call
   site silently reverted to `default`),
   `neighbor_cells_read_the_true_edge_of_each_adjacent_chunk` (all four directions resolve the exact
   cell the neighbouring chunk's own generation holds), and
   `a_missing_neighbor_chunk_drops_its_direction_rather_than_inventing_one` (a corner cell with no
   siblings still gets exactly its two real in-chunk neighbours, never four).
5. Evidence tool: `apps/observer/src-tauri/examples/field_probe.rs` gains a structure-change count
   (real neighbours vs an empty map, same chunk) and a bootstrap wall-clock sweep by
   `active_chunk_radius`.

## Verification

`just ci` is green: full workspace build, fmt, clippy, tests, doctests.

Direct acceptance coverage, in `crates/causafera-runtime/src/carrier.rs`:

- `structure_near_a_chunk_edge_uses_the_real_neighbouring_terrain` — `columns()` built with a real
  west neighbour differs from the same chunk built with an empty map.
- `neighbor_cells_read_the_true_edge_of_each_adjacent_chunk` — west, east, south and north each
  resolve to the exact cell `deterministic_terrain_chunk` generated for the corresponding
  neighbouring chunk (exact equality is possible only because `TODO-GEO-005` made that a pure
  function of chart position everywhere).
- `a_missing_neighbor_chunk_drops_its_direction_rather_than_inventing_one` — a corner cell with no
  sibling chunks resolves exactly its two in-chunk neighbours, never a manufactured four.

Production-path coverage, in `crates/causafera-runtime/tests/terrain_carrier.rs`:
`a_world_bootstraps_at_the_zero_seed` and the full existing suite (19 tests) stayed green with no
re-pointing needed this time — `different_seeds_produce_different_worlds_not_one_world_with_two_terrains`
(seeds 7/30) and `below_threshold_source_changes_mana_without_material_consequence_or_supported_explanation`
(intensity 182) both still hold under the new structure computation. This was checked, not assumed:
both were rerun explicitly after the fix landed.

Evidence, from `cargo run --release -p causafera-observer --example field_probe` (seed 7, three
line-shaped chunks, 48 ticks):

```
TODO-GEO-006: chunk ChunkCoord(-1, 0, 0), 2/9 columns changed structure once real neighbouring terrain was visible
```

Two of nine lattice columns in the west-most active chunk changed once its real east-adjacent
sibling (chunk `(0, 0, 0)`) became visible — the columns whose block of ground actually touches that
shared edge. The mana field shifted correspondingly and only there: `intensity` at chunk
`(-1, 0, 0)` moved from `166..2570` to `167..2570`, at `(0, 0, 0)` from `133..942` to `135..943`, and
at `(1, 0, 0)` from `99..3560` to `100..3566` — single-digit-unit shifts at the extremes of a
field in the hundreds to thousands, consistent with a boundary-local correction rather than a
global one.

## Benchmark plan

`cargo run --release -p causafera-observer --example field_probe`'s bootstrap sweep,
`RuntimeConfig::new(7)` with `active_chunk_shape: Area`:

| `active_chunk_radius` | chunks | bootstrap (before) | bootstrap (after) | per chunk (after) |
|---|---|---|---|---|
| 1 | 9  | 0.775 ms | 0.778 ms | 86.4 µs |
| 2 | 25 | 1.933 ms | 1.893 ms | 75.7 µs |
| 3 | 49 | 3.716 ms | 3.693 ms | 75.3 µs |
| 4 | 81 | 6.259 ms | 6.076 ms | 75.0 µs |

"Before" is the same standalone measurement taken against the pre-`TODO-GEO-006` source with the
change stashed, "after" against this plan's code — both release builds, same machine, same seed.
The two are within run-to-run noise of each other at every radius; per-chunk cost does not rise with
the active set, which is what the TODO's performance requirement asks for. The two-pass restructure
generates the same terrain the single-pass version did (no new generation work) and adds one
`TerrainChunk` clone per chunk into the shared map — `active_chunk_radius` is validated `<= 4`
(`RuntimeConfig::validate`), so the largest possible active set is 81 chunks and the largest possible
one-time clone cost is on the order of a megabyte, matching the measured noise-level difference.
`neighbor_cells`' `Vec<TerrainCell>` allocation per cell is the same shape `neighbor_indices`'
`Vec<usize>` already had — no new per-cell allocation class was introduced.

## Determinism impact

Unchanged in kind, and only chunk pairs that are both in the active set can differ in value —
verified by `a_world_bootstraps_at_the_zero_seed` and the full existing determinism/replay suite
staying green. `neighbor_cells` resolves through a `BTreeMap`, which iterates and looks up in a
fixed key order regardless of construction order, so no hash-iteration or insertion-order dependency
is introduced. Physical digests for any world with more than one active chunk change by
construction, since structure — and therefore mana input — moves at real boundaries; this is the
intended result of closing `TODO-GEO-006`, not a regression.

## Memory impact

One extra `TerrainChunk` clone per active chunk during construction (bootstrap or snapshot import
only, never per tick): at most 81 chunks × ~16 KB ≈ 1.3 MB at the validated
`active_chunk_radius` ceiling of 4. The temporary `generated`/`decoded` map is dropped once
construction finishes; `TerrainCarrierAdapter` itself stores only its own chunk's `TerrainChunk`, as
before.

## Observer impact

None to the wire protocol or to any projection. The per-cell terrain raster the observer reads is
unchanged; `TerrainColumn.structure`, which only ever reached the mana carrier's `magnitude` and was
never itself projected to the observer, is what changed value near active boundaries.

## Explanation impact

None. No causal chain, claim schema, or evidence path changes; a terrain-driven mana change's
`cause` is still the terrain's own `generation_trace`, unaffected by which cells contributed to its
magnitude.

## Persistence impact

None. `TerrainCarrierSnapshot` is unchanged; `structure` and `columns` are derived and were never
persisted, before or after this plan. `import_carrier_adapters`'s restructure changes *when* decode
happens (batched) but not *what* is decoded or in what format.

## Cross-domain effects

Same as `TODO-GEO-005`: the mana field's exact intensity near active-chunk boundaries shifts by a
small, measured amount (see Evidence). No domain outside geography and mana is touched.

## Risks

- **Only chunk pairs both in the active set benefit.** A chunk at the perimeter of the active region
  still has no data beyond it and keeps the truncated neighbour count on that side — a true absence,
  not an artifact, and expected to remain until the active set grows or a future promotion mechanism
  introduces neighbour data there. Not a partial fix; a correctly bounded one.
- **`runtime_carrier_adapters`'s terrain generation remains fully redundant** (generated, then
  immediately overwritten by bootstrap). Pre-existing, independent of this TODO, not fixed here to
  avoid opportunistic scope growth; flagged for whoever next touches that function.
- **A future call site could pass an empty map by mistake** and silently regress to the pre-fix
  truncation with no compiler error, since the parameter degrades gracefully rather than failing
  closed. Mitigated by `structure_near_a_chunk_edge_uses_the_real_neighbouring_terrain`, which is
  specifically shaped to catch exactly that regression at the unit level, and by the doc comment on
  `TerrainCarrierAdapter::new` stating the contract.

## Decision log

- **Accepted:** real sibling `TerrainChunk` data, threaded through a shared
  `BTreeMap<ChartChunkCoord, TerrainChunk>`, mirroring `causafera-domains::mana`'s existing
  `OpenNeighbors`/`same_chart_neighbor` idiom.
- **Rejected:** recomputing a missing neighbour's value from the deterministic generator formula.
  `featureless_ground_is_not_a_physical_pattern`'s hand-built fixture is a concrete, already-existing
  case where a `TerrainChunk`'s content does not match what the generator would produce for its
  stated seed; recomputing would have been silently wrong for it and for any future non-generated
  terrain state.
- **Accepted:** a missing neighbour direction drops out rather than inventing a flat or zero value —
  the same degradation an interior-chart edge always had.
- **Accepted:** `runtime_carrier_adapters` passes an empty map rather than being restructured, since
  its adapters are unconditionally overwritten by bootstrap before anything can read them; verified
  by tracing `RuntimeState::new`, not assumed.
- **Deferred:** fixing `runtime_carrier_adapters`'s redundant generation work.

## Documentation changes

`CHANGELOG.md`, `docs/development/todo-backlog.md` (`TODO-GEO-006` completed),
`docs/ontology/domain-coverage-matrix.md`, `PLANS.md`.

## TODO changes

- `TODO-GEO-006` — completed. No new TODO opened; the one identified risk (perimeter chunks staying
  truncated) is expected, bounded behaviour rather than an open gap, and `runtime_carrier_adapters`'s
  redundant generation is a pre-existing, independent inefficiency rather than a new finding.

## Progress

- Wave 1 — the whole slice, integrated and verified together: carrier signature change, the two
  restructured call sites, `decode_terrain_chunk`, tests, and the evidence tool. Checkpoint
  `b9553c8`. Verified with `just ci` (green: full workspace build, fmt, clippy, tests, doctests),
  `cargo test -p causafera-runtime --lib carrier` (9 passed) and
  `cargo test -p causafera-runtime --test terrain_carrier` (19 passed, no re-pointing needed).
- Wave 2 — documentation: `CHANGELOG.md`, `docs/development/todo-backlog.md` (`TODO-GEO-006`
  completed), `docs/ontology/domain-coverage-matrix.md`, `PLANS.md`.
