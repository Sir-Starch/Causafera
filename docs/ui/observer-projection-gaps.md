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

## 4. Per-cell terrain raster

**This is the highest value per unit of work in the list, and the map is the reason.**

**Needed:** the terrain carrier's existing per-cell arrays for a requested chunk —
`elevations_mm`, `surface_materials` and `roughness_mm` from `TerrainCarrierSnapshot`.

**Why:** the runtime already holds a complete raster. `TERRAIN_CELLS_PER_CHUNK` is
`CHUNK_SIZE²` = 1024, so every chunk carries 1024 elevations, 1024 surface materials and 1024
roughness values. `Runtime::observer_world_snapshot` collapses all of it to a minimum, a maximum
and a mean (`runtime.rs`, the terrain block). The demonstration session has roughly 70 metres of
relief inside a single chunk, and the observer sees two numbers.

That aggregation, not the simulation, is why the map draws flat squares. With the raster
projected, the same lens contract yields a real relief map: hypsometric tinting per cell,
hillshading from the elevation gradient, measured contour lines in place of the interpolated
preview, and a landcover map straight from `surface_materials`. No new frontend infrastructure is
needed — the renderer already resolves marks at real cell positions, and a raster is a cheaper
draw than the marks it already handles.

**Effort:** low to medium, and much smaller than it looks. 1024 values per chunk is a few
kilobytes before any encoding, and elevation delta-encodes well. The contract needs a per-chunk
request and a downsample level for far zoom, not a whole-chart dump.

**Related:** per-cell mana intensity and resolution relevance are the same shape of problem and
should follow the same contract once terrain proves it.

## 4b. Two-dimensional chunk activation

**Needed:** `active_chunk_keys` to generate an area rather than a line.

**Why:** it currently maps `(-radius..=radius)` over x only, with y and z pinned to zero, so the
world is a one-dimensional strip of at most nine chunks. The map draws a row because a row is what
exists. An area of chunks turns the same instrument into a map with shape, and nothing in the
renderer assumes otherwise — it culls by viewport and is written for far larger charts.

**Effort:** small in the runtime, but it is a simulation contract change and needs an ExecPlan.
Bootstrap, population attribution and resolution all iterate the active set.

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

Two further lenses would move from `preview` to `observed` on existing requests: the interpolated
isolines become real terrain contours with item 4 (per-cell fields), and the mana gradient becomes a
measured flux if a transport term between chunks is ever projected.

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
