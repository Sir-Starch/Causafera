# Observer Terrain Raster Map ExecPlan

**Status:** Draft, awaiting acceptance.

## Goal

Give the chart instrument a per-cell terrain projection so the map renders measured relief —
hypsometric tinting, hillshading and true contour lines — instead of one flat tint per chunk, and
widen the active chunk set from a line to an area so the map has shape. Both are bounded reads of
state the runtime already holds; neither invents world content.

## Context

The observer draws chunks as flat squares because `Runtime::observer_world_snapshot` reduces the
terrain carrier to a minimum, a maximum and a mean before the observer ever sees it. The carrier
itself holds a complete raster.

Measured on the demonstration session (seed 7, 48 ticks) with
`cargo run -p causafera-observer --example terrain_probe`:

| Property | Measurement |
|---|---|
| Cells per chunk | 1024 (`TERRAIN_CELLS_PER_CHUNK` = `CHUNK_SIZE²`) |
| Elevation range within one chunk | −33.3 m … +37.0 m |
| Elevation standard deviation | 14.3 m |
| Mean absolute neighbour delta | 1.6 m |
| Ratio of the two | **0.11** (white noise would be ≈ 1.13) |
| Roughness per cell | 0 … 158 mm |
| Distinct surface materials | 16 |
| Same-material neighbour pairs | **6.5 %** (uniform random over 16 ids would be 6.2 %) |

Two conclusions follow, and they point in opposite directions.

The elevation field is **coherent landform**, not noise: a neighbour delta at a ninth of the
standard deviation is a smooth surface with roughly seventy metres of relief across a chunk. It
will hillshade well. This is the single largest available improvement to the map, and it needs no
change to terrain generation.

The surface material field is **spatially random**: neighbours match at the rate chance predicts.
Rendering it as landcover would draw regions the world does not have. That is a geography-domain
gap, not an observer one, and this plan deliberately excludes a landcover lens rather than shipping
a misleading one.

`active_chunk_keys` maps `(-radius..=radius)` over x with y and z pinned to zero, so the active set
is a strip of at most nine chunks. The map draws a row because a row is what exists.

## Relevant invariants

INV-013, INV-021, INV-022 (a rendering is not simulation state), INV-036 (chart-qualified
coordinates; no seamless global surface), INV-037 (geometry, containment and resolution are
separate), INV-038, INV-039 (no substituted data), and the bounded-observer requirements in
`docs/observer/backpressure.md`.

## Ontology domains affected

Geography and local physical space; causal resolution, through the level a chunk has reached;
observer and protocol. No cognitive, social or linguistic domain is touched.

## Causal carriers affected

None are added or altered. Terrain generation provenance already exists on the carrier
(`generation_trace`, `generator`, `parameters`, `causal_inputs`) and is projected unchanged so a
rendered cell can be traced to the generation event that produced it.

## Relevant documents

- `docs/ui/map-lenses.md` — the lens contract this extends
- `docs/ui/observer-projection-gaps.md` §4, §4b — the requests this plan answers
- `docs/architecture/protocol.md`, `docs/observer/protocol.md` — wire boundaries and versioning
- `docs/observer/backpressure.md` — scoped subscriptions and bounded queues
- `docs/rfc/RFC-GEO-002.md`, `docs/world/spatial-hierarchy.md` — charted 2.5D surface
- `docs/architecture/performance.md` — observer overhead must stay bounded

## Current state

- `TerrainCarrierSnapshot` carries `elevations_mm: Vec<i32>`, `surface_materials: Vec<MaterialId>`
  and `roughness_mm: Vec<u32>`, each of length 1024, per active chunk.
- `Runtime::observer_world_snapshot` computes `minimum_elevation_mm`, `maximum_elevation_mm` and
  `mean_roughness_mm` from those arrays and discards the rest.
- `ObserverQuery` has three kinds: `RuntimeSummary`, `ExplanationIr`, `WorldChunks`.
- The frontend lens catalogue draws `relief`, `relief-range` and `roughness` as chunk fields, and
  marks its interpolated contour lens `preview` precisely because chunk aggregates are all it has.
- The renderer already resolves marks at real cell positions at cell zoom and culls by viewport.

## Proposed architecture

### A fourth query kind: `TerrainRaster`

A per-chunk request, not a chart dump. The payload is one chunk's field at a requested detail
level:

```text
TerrainRasterRequest { chart_id, chunk_x, chunk_y, chunk_z, detail_level }
TerrainRasterResponse {
  chunk scope, detail_level, edge,            // edge = 32 >> detail_level
  elevation_reference_mm, elevation_deltas,   // zigzag varint against the reference
  roughness_mm,                               // varint
  generation_trace, generator, parameters,    // provenance, unchanged
}
```

`detail_level` 0 gives the full 32×32 field; levels 1 and 2 give 16×16 and 8×8 by block mean,
computed in the runtime so the observer never re-derives geometry. Block mean is the only
reduction: it is order-independent and cannot introduce values the field does not contain.

Elevation is delta-encoded against a per-chunk reference. On the measured field the mean neighbour
delta is 1.6 m against a 70 m range, so deltas are small and the varint payload is a fraction of
the 4 KB raw array.

Surface materials are **not** projected by this plan. See the decision log.

### Bounding

- One chunk per request, at one detail level.
- The session refuses a request for a chunk outside the active set, with `NotAvailable`.
- The frontend requests only chunks intersecting the viewport, at the detail level its zoom
  warrants, and holds a bounded LRU of decoded rasters keyed by chunk and level.
- A raster is fetched once per chunk per generation trace. Terrain is generated once and does not
  change per tick, so the cache is invalidated by `generation_trace`, not by time.

### Two-dimensional activation, behind a config field

`RuntimeConfig` gains `active_chunk_shape: ActiveChunkShape { Line, Disc }`, defaulting to `Line`.
`active_chunk_keys` honours it. Existing configurations, their digests and every replay fixture are
therefore untouched; the observer session opts into `Disc`.

### Frontend: a raster lens

One new catalogue entry, `terrain`, with `cellProjection: "full"`. The renderer gains one new layer
kind, `raster`, alongside the five it already draws:

```text
LensRaster {
  chunkKey, chunkX, chunkY, edge,
  values: Float32Array,          // normalised into the lens's own extent
  shade?: Float32Array,          // precomputed hillshade, 0..1
}
```

Drawn through an `ImageData` blit per chunk, built once per raster and cached — not a per-cell
fill. Hypsometric tinting uses the single-hue sequential ramp already in the tokens; hillshading is
a standard Lambertian term over the elevation gradient, computed in the presentation layer and
labelled as such. Contours become a measured lens over the raster, and the interpolated preview
lens is demoted to a fallback used only when no raster is available.

## Primitive vs emergent review

Nothing here promotes an observer construction into simulation state. Elevation, roughness and the
generation provenance are authoritative and are transported unchanged; block-mean downsampling and
hillshading are presentation reductions that never re-enter the runtime. The active chunk shape is
a configuration of an existing mechanism, not a new spatial primitive.

## Non-goals

- A landcover or material map. The field is spatially random; see the decision log.
- Per-cell mana or resolution. Same contract shape, deliberately deferred until terrain proves it.
- Joining charts into a global surface (INV-036).
- Streaming terrain. Terrain is static per generation; request and response is the correct shape.
- 3D rendering, textured terrain, or any game-object representation.
- Changing terrain generation.

## Implementation stages

1. **Read model.** Add `ObserverTerrainRaster` to `causafera-observer-api`, and a runtime method
   projecting one chunk at one detail level from the carrier. Unit tests: level 0 round-trips the
   carrier field exactly; levels 1 and 2 are block means; an inactive chunk yields `NotAvailable`.
2. **Wire.** Proto message, encoder and decoder in `causafera-observer-wire`, round-trip tests
   including the delta encoding at the extremes of `i32`. Additive `QueryKind` variant; existing
   queries unaffected.
3. **Session and command.** `ObserverSession::terrain_raster`, a Tauri command, and the request
   bounds. Session tests: locale invariance, digest invariance, refusal outside the active set.
4. **TypeScript codec.** Decoder in `@causafera/observer-protocol`, with a capture-driven test.
5. **Frontend raster layer.** `LensRaster` in the lens contract, `ImageData` rendering with a
   bounded cache, and the `terrain` lens with hypsometric tinting and hillshading. The measured
   contour lens replaces the preview when a raster is present.
6. **Active chunk shape.** `ActiveChunkShape` in `RuntimeConfig`, `Disc` in the observer session,
   and the determinism evidence in the verification section.

Stages 1–5 are independent of stage 6 and deliver the visual improvement on the current line of
three chunks.

## Verification

- Replay: the same seed and tick count produce byte-identical raster payloads.
- Locale invariance: raster bytes and session digests are unchanged across `ru-RU` and `en-US`.
- Digest invariance: adding the query kind changes no physical or history digest, verified against
  the existing session tests.
- Bounds: a request for an inactive chunk returns `NotAvailable`; a malformed detail level returns
  `InvalidRequest`.
- Downsample correctness: level 1 and 2 values equal the block means of level 0.
- Stage 6: with `active_chunk_shape: Line` every existing fixture digest is unchanged; with `Disc`,
  replay determinism holds and the population, resolution and bootstrap paths that iterate the
  active set are exercised.
- Frontend: the render smoke check covers the raster lens with and without a cached raster, and
  the unattached and awaiting states are unaffected.

## Benchmark plan

Measure and record, rather than assert:

- Encoded raster size per chunk at each detail level, against the 4 KB raw array.
- Observer overhead per raster request, against the existing `WorldChunks` query.
- Frontend: time to decode and blit one chunk raster, and steady-state paint time for a viewport of
  nine chunks at cell zoom, on the software-rendering profile as the pessimistic case.

No scale claim is accepted without these numbers.

## Determinism impact

Stages 1–5 add a read path only; no RNG stream, ordering or mutation is touched. Stage 6 changes
which chunks exist for sessions that opt in, which changes their state hashes by construction —
that is why it is behind a config field with `Line` as the default, so no existing fixture or
replay-verified experiment moves.

## Memory impact

Runtime: none; the projection reads the existing carrier. Observer: one raster per request, freed
after encoding. Frontend: a bounded LRU of decoded rasters — nine chunks at level 0 is roughly
36 KB of `Float32Array` plus the same again for the shade term.

## Observer impact

One additive query kind and one additive Tauri command. Protocol version stays at v1; the change is
additive and existing decoders skip unknown fields. Capability negotiation advertises the new kind
so a client can tell whether a runtime supports it.

## Explanation impact

None. The raster is a read surface and carries no claims. The generation provenance travels with it
so a future explanation about terrain has an anchor.

## Persistence impact

None. The carrier is already persisted; nothing new is stored.

## Cross-domain effects

Stage 6 widens the active set, which the population, resolution and bootstrap paths iterate. Their
cost scales with the number of active chunks, and the `Disc` shape must therefore be introduced
with the benchmark numbers above rather than as a default.

## Risks

- **The map looks worse, not better, if hillshading is overdone.** Mitigation: the shade term is a
  presentation parameter with a conservative default, and hypsometric tinting stays on the
  documented sequential ramp rather than a rainbow.
- **Payload growth at larger active sets.** Mitigation: per-chunk requests, detail levels, viewport
  culling, and a cache keyed by generation trace.
- **Stage 6 cost.** Mitigation: config-gated, benchmarked before the default changes.
- **Temptation to project surface materials anyway.** Mitigation: recorded as a decision, with the
  measurement that justifies it.

## Documentation changes

`docs/ui/map-lenses.md` (the raster layer and the promotion of the contour lens),
`docs/ui/observer-projection-gaps.md` (§4 and §4b resolved), `docs/ui/observer-application.md`,
`docs/observer/protocol.md`, `docs/architecture/protocol.md`, `CHANGELOG.md`,
`docs/development/todo-backlog.md`, `docs/roadmap/roadmap.md`, and
`docs/ontology/domain-coverage-matrix.md` for the geography observer row.

## TODO changes

Resolves `TODO-UI-004` items 4 and 4b. Adds a geography-domain item for coherent surface material
generation, which is the prerequisite for a landcover lens.

## Decision log

- **Per-chunk requests, not a chart dump.** A dump does not survive a larger chart and would
  violate the bounded-observer requirement.
- **Block mean for downsampling.** Order-independent, introduces no value the field does not
  contain, and is computed in the runtime so the observer never re-derives geometry.
- **Delta-encoded elevation.** The measured field has a mean neighbour delta of 1.6 m against a
  70 m range; deltas are the natural encoding.
- **Surface materials excluded.** Measured at 6.5 % same-material neighbours against 6.2 %
  expected from chance. Drawing that as landcover would show the world as having regions it does
  not have, which is the same failure as substituting data. Revisit when geography generates
  coherent material regions.
- **Contours become measured, the interpolation becomes a fallback.** The preview lens exists only
  because chunk aggregates were all the observer had; it should not outlive that.
- **Active chunk shape behind a config field.** Widening the active set changes state hashes by
  construction; defaulting to `Line` keeps every existing fixture and replay-verified experiment
  intact.
- **No hillshade in the runtime.** It is a lighting choice, and a lighting choice is presentation
  (INV-022).

## Progress

Not started. The measurements in the context section were taken with
`apps/observer/src-tauri/examples/terrain_probe.rs` against seed 7 at 48 ticks and should be
re-taken if terrain generation changes.
