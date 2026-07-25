# Observer Field Raster Map ExecPlan

**Status:** Draft, awaiting acceptance.

## Goal

Give the chart instrument per-cell projections of the two spatial fields the runtime already
maintains — terrain and mana — so the map renders measured relief and a measured mana field
instead of one flat tint per chunk, and widen the active chunk set from a line to an area so the
map has shape. All three are bounded reads of state that already exists; none invents world
content.

## Context

The observer draws chunks as flat squares because `Runtime::observer_world_snapshot` reduces the
terrain carrier to a minimum, a maximum and a mean before the observer ever sees it. The carrier
itself holds a complete raster.

Mana is a different case. It is fully implemented and fully live — every cell carries an intensity
and the trace that last changed it — but the demonstration configuration runs it at a much coarser
lattice than terrain, and that, not the observer, is what limits how smooth a mana map can be.

Measured on the demonstration session (seed 7, 48 ticks) with
`cargo run -p causafera-observer --example field_probe`:

### Terrain carrier, per chunk

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

### Mana field, per chunk

| Property | Measurement |
|---|---|
| Lattice | `chunk_extent³` = **3 × 3 × 3 = 27 cells** |
| Populated cells | 27 / 27, 27 / 27, 23 / 27 |
| Cells carrying a `last_change` trace | every populated cell |
| Intensity range | 0 … 2113, standard deviation 151 … 418 |
| Neighbour coherence ratio | 0.41 … 0.48 (white noise ≈ 1.13) |
| Plan-view columns per chunk | **9** |

Three conclusions follow, and they point in different directions.

The elevation field is **coherent landform**, not noise: a neighbour delta at a ninth of the
standard deviation is a smooth surface with roughly seventy metres of relief across a chunk. It
will hillshade well. This is the single largest available improvement to the map, and it needs no
change to terrain generation.

The mana field is **real, dense and traced, but coarse**. It is not sparse or stubbed: every cell
holds an intensity, and every populated cell records the causal trace that last changed it, which
is a per-cell provenance no other field offers. Its coherence ratio of about 0.45 is well below
noise, so it is a field rather than static. But `chunk_extent` defaults to 3, giving nine columns
per chunk in plan view. A smooth mana blob at that lattice requires interpolation, and
`chunk_extent` is a validated configuration field accepting 3 … 32 — so smoother mana is a
simulation cost decision, not an observer limitation.

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

Geography and local physical space; mana; causal resolution, through the level a chunk has
reached; observer and protocol. No cognitive, social or linguistic domain is touched.

## Causal carriers affected

None are added or altered. Terrain generation provenance already exists on the carrier
(`generation_trace`, `generator`, `parameters`, `causal_inputs`) and the mana field already records
`last_change` per cell; both are projected unchanged, so a rendered cell can be traced to the event
that produced or last altered it.

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
- `ManaFieldSnapshot` carries `intensity: Vec<i64>`, `last_change: Vec<Option<TraceId>>` and
  `last_change_before: Vec<i64>`, each of length `extent³`, per active chunk.
- `RuntimeConfig::chunk_extent` defaults to 3 and is validated into 3 … 32. It sizes the mana
  volume and the material-surface cell index space; terrain is always 32 × 32 regardless.
- `Runtime::observer_world_snapshot` computes `minimum_elevation_mm`, `maximum_elevation_mm` and
  `mean_roughness_mm` from those arrays and discards the rest.
- `ObserverQuery` has three kinds: `RuntimeSummary`, `ExplanationIr`, `WorldChunks`.
- The frontend lens catalogue draws `relief`, `relief-range` and `roughness` as chunk fields, and
  marks its interpolated contour lens `preview` precisely because chunk aggregates are all it has.
- The renderer already resolves marks at real cell positions at cell zoom and culls by viewport.

## Proposed architecture

### A fourth query kind: `FieldRaster`

One query shape serves both fields, because both are lattices over a chunk and both want the same
bounding. A per-chunk request, not a chart dump:

```text
FieldRasterRequest { chart_id, chunk_x, chunk_y, chunk_z, field, detail_level }
FieldRasterResponse {
  chunk scope, field, detail_level, edge,
  reference_value, value_deltas,    // zigzag varint against the reference
  auxiliary,                        // roughness for terrain; the change trace for mana
  provenance,                       // generation trace for terrain; latest trace for mana
}
```

`field` selects `TerrainElevation`, `TerrainRoughness` or `ManaIntensity`. Adding a field later is
an additive enum variant, not a new query.

**Terrain.** `detail_level` 0 gives the full 32 × 32 field; levels 1 and 2 give 16 × 16 and 8 × 8
by block mean, computed in the runtime so the observer never re-derives geometry. Block mean is
the only reduction: it is order-independent and cannot introduce values the field does not
contain. Elevation is delta-encoded against a per-chunk reference; the measured mean neighbour
delta is 1.6 m against a 70 m range, so the varint payload is a fraction of the 4 KB raw array.

**Mana.** The field is volumetric at `chunk_extent³`. The observer receives it whole — at the
default extent that is 27 values, far below any bounding concern — together with the per-cell
`last_change` trace. Reduction to plan view happens in the presentation layer, not the runtime,
because the choice of reduction is a reading of the field rather than a property of it: a column
sum answers "how much mana stands over this ground", a column maximum answers "how intense does it
get anywhere in this column", and the map offers both, each labelled. Neither is invented — both
are stated reductions of received values.

The per-cell change trace is the distinctive part. It lets the map show which cells moved and when,
and lets a trace selected anywhere in the interface light up the cells it touched — spatial
provenance, which no other field currently offers.

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

### Frontend: raster lenses

Two new catalogue entries with `cellProjection: "full"` — `terrain` and `mana-field` — plus a
measured contour lens over each. The renderer gains one new layer kind, `raster`, alongside the
five it already draws:

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
labelled as such.

Terrain contours become measured: marching squares runs over 1024 real samples instead of three
chunk aggregates, and the interpolated preview lens is demoted to a fallback used only where no
raster is available.

Mana is drawn as a field rather than as a relief: a single-hue blob over the column reduction, with
isolines at stated intensity levels. At `chunk_extent` 3 the nine columns per chunk are upsampled
for display, and the lens is marked `preview` for exactly that reason, with the caveat naming the
lattice it came from. The moment `chunk_extent` rises the same lens becomes `observed` with no code
change — the availability follows the received edge length, not a constant.

## Primitive vs emergent review

Nothing here promotes an observer construction into simulation state. Elevation, roughness and the
generation provenance are authoritative and are transported unchanged; block-mean downsampling and
hillshading are presentation reductions that never re-enter the runtime. The active chunk shape is
a configuration of an existing mechanism, not a new spatial primitive.

## Non-goals

- A landcover or material map. The field is spatially random; see the decision log.
- Raising `chunk_extent`. The plan projects the mana field at whatever lattice the runtime is
  configured for, and measures what a larger one would cost; choosing to pay that cost is a
  separate decision with its own evidence.
- Per-cell causal resolution. Same contract shape, and an additive `field` variant when a read
  model exists.
- Joining charts into a global surface (INV-036).
- Streaming terrain. Terrain is static per generation; request and response is the correct shape.
- 3D rendering, textured terrain, or any game-object representation.
- Changing terrain generation.

## Implementation stages

1. **Read model.** Add `ObserverFieldRaster` to `causafera-observer-api`, and runtime methods
   projecting one chunk of terrain elevation, terrain roughness or mana intensity at one detail
   level. Unit tests: terrain level 0 round-trips the carrier field exactly; levels 1 and 2 are
   block means; mana round-trips intensity and the per-cell change trace; an inactive chunk or an
   unknown field yields `NotAvailable`.
2. **Wire.** Proto message, encoder and decoder in `causafera-observer-wire`, round-trip tests
   including the delta encoding at the extremes of `i32` and `i64`. Additive `QueryKind` variant;
   existing queries unaffected.
3. **Session and command.** `ObserverSession::field_raster`, a Tauri command, and the request
   bounds. Session tests: locale invariance, digest invariance, refusal outside the active set.
4. **TypeScript codec.** Decoder in `@causafera/observer-protocol`, with a capture-driven test.
5. **Frontend raster layer.** `LensRaster` in the lens contract, `ImageData` rendering with a
   bounded cache keyed by chunk, field and provenance.
6. **Terrain lenses.** Hypsometric tinting, hillshading, and measured contours replacing the
   interpolated preview where a raster is present.
7. **Mana lenses.** The column reduction with both readings offered, the blob rendering, isolines
   at stated intensity levels, and availability derived from the received edge length rather than
   hard-coded. Selecting a trace anywhere in the interface highlights the cells it last changed.
8. **`chunk_extent` cost evidence.** Measured; see the results below. No default is changed by
   this plan, and the measurements now inform `TODO-MANA-001` rather than await it.
9. **Active chunk shape.** `ActiveChunkShape` in `RuntimeConfig`, `Disc` in the observer session,
   and the determinism evidence in the verification section.

Stages 1–7 are independent of 8 and 9 and deliver the visual improvement on the current line of
three chunks.

## Verification

- Replay: the same seed and tick count produce byte-identical raster payloads.
- Locale invariance: raster bytes and session digests are unchanged across `ru-RU` and `en-US`.
- Digest invariance: adding the query kind changes no physical or history digest, verified against
  the existing session tests.
- Bounds: a request for an inactive chunk returns `NotAvailable`; a malformed detail level returns
  `InvalidRequest`.
- Downsample correctness: level 1 and 2 values equal the block means of level 0.
- Mana fidelity: the projected intensity and change traces equal the field snapshot cell for cell.
- Mana availability: a lens fed an edge of 3 reports `preview`; the same lens fed a larger edge
  reports `observed`, with no code change.
- Stage 6: with `active_chunk_shape: Line` every existing fixture digest is unchanged; with `Disc`,
  replay determinism holds and the population, resolution and bootstrap paths that iterate the
  active set are exercised.
- Frontend: the render smoke check covers the raster lens with and without a cached raster, and
  the unattached and awaiting states are unaffected.

## Benchmark plan

Measure and record, rather than assert:

- Encoded raster size per chunk at each detail level and per field, against the raw arrays.
- Observer overhead per raster request, against the existing `WorldChunks` query.
- Encoded mana raster size per extent, once the wire format exists.

### Measured: `chunk_extent` against the map

Taken with `cargo run --release -p causafera-observer --example extent_bench` on the
demonstration session (seed 7). The honest metric is the **plan-view column field** — what the map
actually draws — not the cell count, because the map reduces the volume through z.

After 192 ticks:

| `chunk_extent` | Columns drawn | Columns carrying mana | Neighbour coherence | ms per tick |
|---|---|---|---|---|
| 3 (default) | 27 | 100 % | 0.71 | 0.66 |
| 4 | 48 | 95.8 % | 0.46 | 0.75 |
| 6 | 108 | 57.4 % | 0.26 | 0.79 |
| 8 | 192 | 36.5 % | 0.18 | 0.90 |
| 12 | 432 | 16.7 % | 0.11 | 1.25 |
| 16 | 768 | 9.4 % | 0.08 | 1.97 |
| 32 | 3072 | — | — | 11.33 (48 ticks) |

Three things follow.

**The useful range is 4 to 6, not 16 or 32.** At extent 4 the map draws 78 % more columns, still
covered almost everywhere, with neighbour variation cut from 0.71 to 0.46 — visibly smoother — for
14 % more tick cost. At extent 6 the detail quadruples and the field stops saturating the chart,
which may well read better on a map: mana that gathers somewhere is more informative than mana
that is uniformly everywhere. Beyond 6 the field becomes a minority of the chart and the cost
climbs steeply.

**Coverage does not recover with time.** Sixteen times the ticks moves extent 8 from 9.0 % to
13.6 % of cells and extent 16 from 1.1 % to 1.7 %. The field plateaus; a finer lattice is not
merely slower to fill.

**The mana does form a real gradient, not scattered points.** At extent 8 the populated cells span
the full width and decay upward through z — layer counts 28, 24, 15, 8, 3, 0, 0, 0 — which is what
a field anchored near the ground should look like. The coverage figures are about how much of a
finer lattice that gradient reaches, not about whether the field has structure.

One caveat the mana domain should check before acting on this: intensity is an integer, and the
same total mana spread over more cells will floor to zero in more of them. Part of the coverage
loss at high extent may be quantisation rather than physics, in which case a finer lattice needs a
finer intensity unit, not just more cells.
- Frontend: time to decode and blit one chunk raster, and steady-state paint time for a viewport of
  nine chunks at cell zoom, on the software-rendering profile as the pessimistic case.

No scale claim is accepted without these numbers.

## Determinism impact

Stages 1–5 add a read path only; no RNG stream, ordering or mutation is touched. Stage 6 changes
which chunks exist for sessions that opt in, which changes their state hashes by construction —
that is why it is behind a config field with `Line` as the default, so no existing fixture or
replay-verified experiment moves.

## Memory impact

Runtime: none; the projection reads the existing carrier and field. Observer: one raster per
request, freed after encoding. Frontend: a bounded LRU of decoded rasters — nine chunks of terrain
at level 0 is roughly 36 KB of `Float32Array` plus the same again for the shade term; nine chunks
of mana at the default extent is negligible beside it.

Raising `chunk_extent` grows the runtime mana field with the cube of the extent: 3 → 8 is a factor
of nineteen, 3 → 16 a factor of 152, 3 → 32 a factor of 1214. Stage 8 measures it; nothing in this
plan assumes it.

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

- **A mana blob drawn from nine columns per chunk reads as more resolution than exists.**
  Mitigation: availability is derived from the received edge, the lens is marked and captioned with
  the lattice it came from, and the upsampling is stated rather than implied.
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
`docs/observer/protocol.md`, `docs/architecture/protocol.md`, `docs/world/mana-topology.md`,
`CHANGELOG.md`,
`docs/development/todo-backlog.md`, `docs/roadmap/roadmap.md`, and
`docs/ontology/domain-coverage-matrix.md` for the geography observer row.

## TODO changes

Resolves `TODO-UI-004` items 4 and 4b. Adds a geography-domain item for coherent surface material
generation, which is the prerequisite for a landcover lens, and a mana-domain item for the
`chunk_extent` cost decision that stage 8 supplies evidence for.

## Decision log

- **Per-chunk requests, not a chart dump.** A dump does not survive a larger chart and would
  violate the bounded-observer requirement.
- **Block mean for downsampling.** Order-independent, introduces no value the field does not
  contain, and is computed in the runtime so the observer never re-derives geometry.
- **Delta-encoded elevation.** The measured field has a mean neighbour delta of 1.6 m against a
  70 m range; deltas are the natural encoding.
- **One query for both fields.** Terrain and mana are both chunk lattices wanting identical
  bounding; a shared `field` selector makes a third field an enum variant rather than a third
  query.
- **Mana is projected whole, terrain by detail level.** At the configured extent the mana volume
  is 27 values. Adding a downsample contract to that would be ceremony without a payload to
  justify it; the detail level exists for terrain, where a chunk is 1024 values.
- **The plan-view reduction of mana lives in the frontend.** Column sum and column maximum answer
  different questions, and choosing between them is a reading of the field rather than a property
  of it. Both are offered and both are labelled; the runtime projects the volume unreduced.
- **Mana availability is derived, not declared.** The lens reports `preview` while the received
  edge is too coarse to draw without upsampling, and `observed` when it is not. That way raising
  `chunk_extent` improves the map without a frontend change, and lowering it cannot quietly
  overstate what is drawn.
- **`chunk_extent` is measured, not raised.** The field is fully implemented and every cell is
  live; only its lattice is coarse. The measurements above put the useful range at 4 to 6 and rule
  out 16 and 32 on both cost and coverage. Acting on that is a mana-domain decision with its own
  determinism and fixture consequences, so this plan supplies the evidence and stops there.
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
`apps/observer/src-tauri/examples/field_probe.rs` against seed 7 at 48 ticks and should be re-taken
if terrain generation, mana propagation or `chunk_extent` changes.
