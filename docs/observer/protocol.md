# Observer Protocol

The observer protocol uses Protocol Buffers for structured communication between the simulation and UI.

## Schema Versioning

Version schemas under:

```text
proto/causafera/observer/v1/
```

## Initial Protocol Categories

- `common.proto` - Shared types and primitives
- `control.proto` - Simulation control messages
- `query.proto` - Query requests and responses
- `stream.proto` - Streaming subscriptions
- `spatial.proto` - Spatial data and chunks
- `entity.proto` - Entity state and changes
- `causal.proto` - Causal traces and provenance
- `language.proto` - Language and lexeme data
- `explanation.proto` - Explanation IR and rendering
- `metrics.proto` - Performance and telemetry

## Bindings

Generate Rust and TypeScript bindings from the proto definitions. Do not manually duplicate schemas.

The implemented v1 surface supports negotiation and four request kinds:

- runtime summary;
- typed Explanation IR;
- a bounded chart-qualified world-chunk snapshot;
- a bounded per-chunk field raster.

`FieldRaster` is the one request that carries parameters, in the query payload rather than in a
second envelope shape: one chunk, one field, one detail level. `TerrainElevation` and
`TerrainRoughness` project the carrier's own 32 x 32 lattice, or a block-mean reduction of it at
detail level 1 or 2; `ManaIntensity` projects the mana volume whole at its configured extent,
together with the trace that last changed each cell. Values travel as packed ZigZag varints of
successive differences along the scan order, which is why nine chunks of elevation and roughness
encode to about 3.4 KB each against 8 KB of raw arrays.

The runtime performs exactly one reduction on a raster, the block mean, and it is order-independent
and introduces no value the field does not contain. It never flattens the mana volume to plan view:
a column sum and a column maximum answer different questions, so choosing between them is a reading
of the field rather than a property of it and the map states which reading it draws. It never shades
a surface either, because lighting is presentation (INV-022).

A request for a chunk outside the active set is answered `NotAvailable` and a malformed one
`InvalidRequest`; neither is answered with substituted data.

The world projection contains numeric terrain bounds, roughness, local mana total, causal-resolution
relevance/level, population aggregate, activity count, and trace anchor. Its additive
`MaterialSurfaceDelta` V3 projection is bounded and includes a chart-qualified chunk, cell ordinal,
typed before/after condition, mana total, transition tick, and optional contact, mana-effect, and
mana-transition trace anchors. Optional in-world mana before/after values are present only when a
bounded source receipt supports them. V3 adds optional matching-cell local mana before/after values
and the local mana transition trace. A separate bounded `MaterialSurfaceGateDelta` exposes
gate-only falling transitions without inventing a condition change. Neither projection contains a city, biome, species, occupation,
spell, or other semantic classification.

`MaterialSurfaceDelta` is an inspection projection, not the material state or a mutation API. Its
schema version and capacity remain explicit in the existing `world_chunks` response; the projection
retains at most 64 material deltas and preserves the newest mana-mediated transition when the
bounded window is full. A query, locale, or UI rendering cannot alter authoritative state.

The trace anchors and optional mana values use protobuf field presence, not a numeric sentinel: an
omitted value is different from an explicitly encoded `TraceId(0)` or zero-valued field. Consumers
must preserve that distinction when round-tripping V2 or V3 data, so bootstrap-only surface state cannot
be misreported as actor contact. Observer output explicitly redacts external origin, source record
ID, recipe identity or hash, policy schema, operator intent, and semantic labels such as divine,
reward, punishment, or worship.

## Protocol Boundaries

Observer protocol is not:

- simulation persistence format;
- internal domain representation;
- causal trace storage.

It is a derived read-only interface for external consumers.

## Related Documents

- `docs/observer/architecture.md` - Overall observer design
- `docs/observer/snapshots.md` - Streaming over the protocol
- `docs/observer/backpressure.md` - Flow control policies
