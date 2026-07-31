# Observer Projection Gaps

Requests from the observer frontend to the observer layer, ordered by how much interface they
unblock per unit of backend work. Each entry names the read model or wire encoding required, so a
later agent can pick one up without re-deriving the need.

Protocol changes require an ExecPlan (`PLANS.md`). Nothing here is implemented in the frontend
against imagined data: the capability register in `src/observer/capability.ts` states each of these
as unavailable, and the interface renders that state rather than a placeholder.

## 1. Deterministic explanation text over the wire

**Needed:** transport for `RenderedExplanation` (the proto message already exists) produced by the
Rust `DeterministicExplanationRenderer`, as a query kind or as a field on the Explanation response.

**Why:** the renderer is authoritative and already emits Russian and English templates. The
frontend deliberately does not reproduce its sentences — a TypeScript port would be a second
implementation of authoritative wording that could silently diverge. Today the Assay area presents
claim structure with per-schema reading notes instead. With the rendered text transported, the area
gains the authoritative narrative beside the structure at no risk of divergence.

**Effort:** low. No new read model; the renderer output only needs encoding.

## 2. Resolution policy thresholds

**Needed:** the level thresholds and the relevance ceiling of the active `ResolutionPolicy` in the
runtime summary or the world snapshot.

**Why:** the observer receives `resolution_relevance` and `resolution_level` but not the scale they
live on, so relevance can only be shown as a raw integer. With the thresholds, relevance becomes a
ladder with the level boundaries marked — the difference between a number and a measurement.

**Effort:** low. Additive scalar fields on an existing message.

## 3. Trace ancestry query

**Needed:** a query kind over `CausalTraceStore` returning a bounded ancestry window for a trace
identifier: parents, depth, and the committed event kind.

**Why:** trace anchors are already threaded through the whole interface — transitions, gate
transitions, and Explanation claims all carry them, and selecting one filters the ledger. That is
the full extent of provenance navigation the protocol allows. An ancestry window turns the anchors
into a walkable chain, which is the single largest analytical capability the observer is missing.

**Effort:** medium. The store supports traversal; the query surface, wire encoding, and a decoder
are new. Bounding must be explicit — ancestry is unbounded in principle.

## 4. Per-cell terrain raster — resolved

**Delivered** by `TODO-OBS-001`. The `FieldRaster` query projects the terrain carrier's
`elevations_mm` and `roughness_mm` for one requested chunk, at the carrier's own 32 x 32 lattice or
a block-mean reduction of it, with the generation trace travelling alongside. The map assembles
every received lattice into one field over the surveyed extent and draws hypsometric tinting,
hillshading and measured contours from it; the interpolated contour lens survives only as the
fallback used where no raster has arrived.

`surface_materials` is deliberately still not projected. Measured at 6.5% same-material neighbours
against 6.2% expected from chance, drawing it as landcover would show the world as having regions it
does not have. `TODO-GEO-004` is the prerequisite.

**Found while implementing it, since resolved:** `terrain_cells` derived elevation from chunk-local
coordinates only, so every chunk repeated the same diagonal ridge and a two-dimensional chart showed
a thirty-metre scarp on every chunk boundary — recorded as `TODO-GEO-005` and closed in
`plans/terrain-chunk-boundary-continuity.md`. Terrain is now continuous across chunk boundaries; the
chart still does not open on terrain contours, but that is a UI decision rather than a consequence of
the generation gap.

## 4b. Two-dimensional chunk activation — resolved

**Delivered** by `TODO-OBS-001`. `RuntimeConfig::active_chunk_shape` selects `Line` or `Area`;
`Area` activates the square block of `(2 * radius + 1)²` chunks and the observer session opts into
it, so the demonstration chart is nine chunks with shape rather than a strip of three. The default
stays `Line`, so no recorded fixture or replay-verified experiment moved.

A Euclidean disc was considered and rejected: at radius 1 it is a five-chunk cross, and the
observer's own bounds were already written for the nine-chunk block.

**Found while implementing it:** `chart_chunk_hash` sign-extended each axis, so a coordinate of −1
became all ones on every axis and `(-1, -1, 0)` collided with `(0, 0, 0)`. Object identity is keyed
by that hash, so the first area-shaped runtime was rejected by the mana cell validator. The x term is
unchanged and the off-line terms are zero at zero, so every line-shaped chart keeps the identity it
was recorded with.

## 4c. Water — resolved

**Delivered** by `TODO-UI-007`, over the read models `TODO-HYDRO-001` shipped. Nothing had to be
added to the protocol for the chart to draw water: the hydrology slice shipped a whole-session
summary, a bounded per-cell delta window, transfer and conveyance summaries, and three `FieldRaster`
kinds for the storages. What was missing was a frontend that read any of it.

The `hydrology` lens group now draws surface water, soil water and groundwater from those lattices.
The Observatory reports the four storages, the exact conservation residual, and the latest applied
forcing record with its origin; Flux reports the movement and cell-delta windows as tables, each
movement carrying all three of its volumes so a limiter that engaged is visible as such.

Three things stayed unprojected and are not gaps in the frontend:

- **The grid metric** (`TODO-OBS-004`). `HydrologyGridMetric` is declared per chart and is not in any observer
  payload, so a volume cannot honestly be turned into a depth on this side of the wire. Every
  surface therefore states volumes. Projecting the metric would let the chart read depths, which is
  the more natural quantity for a reader — worth doing, and a protocol change rather than a
  frontend one.
- **Conveyance edge geometry** (`TODO-OBS-005`). Conveyance summaries arrive with storage, capacity and the tick's
  accepted exchange, but the edge key is the canonical carrier encoding and is opaque at this
  boundary. The observer can report an edge and cannot place it on the chart, so no lens draws the
  drainage graph. Projecting the two endpoint addresses alongside the key would resolve it.
- **A spatially representative delta window** (`TODO-OBS-006`). `MAX_HYDROLOGY_DELTAS` and
  `MAX_HYDROLOGY_TRANSFER_SUMMARIES` are 64 each, and the runtime sorts by canonical address or
  carrier key before truncating. On a nine-chunk chart where thousands of cells change per tick the
  window is therefore the lowest-addressed sixty-four, which is deterministic and reproducible but
  is not a sample. It reads correctly as a table and would read as a lie as a map overlay, so there
  is no delta lens. Either a stratified selection or a cap large enough to cover a tick would make
  one honest.

## 5. Performance telemetry

**Needed:** wire encoding for the existing `PerformanceMetrics` message.

**Why:** the Instrument area measures the client side of every exchange — bytes, duration, outcome
— from real transport activity. The runtime side of the same picture is missing, so the area can
report what the observer costs but not what the simulation costs.

**Effort:** low. Proto defined, metrics partially collected.

## 6. Entity summaries

**Needed:** an `EntitySummary` read model and wire encoding.

**Why:** actors and population appear only as counts. Entity summaries would give the observer its
first per-agent surface. This is deliberately ranked last: it is the largest new surface, it
depends on decisions about which entity attributes are observable at all, and it must not become a
back door to Ground Truth identity (INV-027).

**Effort:** high, and it needs a contract decision before an implementation.

## 7. Lenses awaiting a read model

The chart instrument lists every one of these as an `awaiting` lens: selectable, described, and
stating what it needs. Connecting one is a small frontend change once its projection exists — see
the promotion recipe in `docs/ui/map-lenses.md`.

| Lens | Needs |
|------|-------|
| Agents | `EntitySummary` read model with a spatial address; item 6 above |
| Knowledge and belief | Subjective scene and belief read models; cognition is at contract level |
| Language | Lexeme projection; the language domain is not coupled to the runtime |
| Social structure | Agent-inferred structure and a read model |
| Practices | Embodied practice execution and transmission |
| Economy | City and material read models |
| Ecology | The domain itself; documented but not implemented |
| Provenance graph | Item 3 above — a bounded ancestry query |

The terrain contour lens has since been promoted: item 4 landed, so it traces the measured elevation
lattice and keeps the interpolation only as a fallback. The mana gradient would become a measured
flux if a transport term between chunks were ever projected.

The mana field lenses report their availability from the received lattice edge rather than from a
constant, so they read `preview` while the field has to be upsampled to be drawn and `observed` when
it does not. Raising `chunk_extent` promotes them with no frontend change; `TODO-MANA-004` decided
to leave it at 3, so today they read `preview` and name the lattice in their caveat.

## Not requested

- **Streaming subscriptions.** The stream hub already supports multiple streams, but request and
  response over the Tauri bridge is adequate at the current tick rates, and the demand registry
  already stops traffic for closed panels. Revisit when a second stream kind has real data.
- **Historical state queries.** Blocked on persistence maturity, not on the observer layer. The
  frontend labels its own series as observer-side rather than implying stored history.

## Related Documents

- `docs/ui/observer-application.md` — frontend architecture
- `docs/observer/architecture.md` — observer pipeline
- `docs/architecture/protocol.md` — protocol boundaries and versioning
- `docs/observer/capability-maturity-map.md` — capability readiness assessment
