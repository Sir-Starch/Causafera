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

There is no generated-binding pipeline. The Rust codec
(`crates/causafera-observer-wire/src/protocol.rs`) and the TypeScript codec
(`packages/observer-protocol/src/index.ts`) are written by hand, and
`proto/causafera/observer/v1/query.proto` is a declaration of what they do rather
than their source.

That is a real hazard — a field the schema names at 37 and a codec writes at 38
would compile, pass every Rust test, and be wrong for anyone generating bindings
from the schema — so it is pinned by an audit instead of by a build step.
[`tools/audit/test-observer-proto-schema.mjs`](../../tools/audit/test-observer-proto-schema.mjs)
reads the schema and both codecs as text and asserts that every field number,
wire shape, enum discriminant, carrier length, and declared bound agrees across
all three, and that no message declares a number twice.

The two codecs are independent implementations of one specification, which is
what makes the second one worth having: the failure the protocol cannot afford is
the two disagreeing about whether a payload is *valid*, not about what it means.
[`tools/audit/test-observer-hydrology-decoder.mjs`](../../tools/audit/test-observer-hydrology-decoder.mjs)
drives the TypeScript decoder against payloads built byte by byte — never from an
encoder — plus a payload captured from a running engine.

The implemented v1 surface supports negotiation and four request kinds:

- runtime summary;
- typed Explanation IR;
- a bounded chart-qualified world-chunk snapshot;
- a bounded per-chunk field raster.

`FieldRaster` is the one request that carries parameters, in the query payload rather than in a
second envelope shape: one chunk, one field, one detail level. `TerrainElevation` and
`TerrainRoughness` project the carrier's own 32 x 32 lattice, or a block-mean reduction of it at
detail level 1 or 2; `ManaIntensity` projects the mana volume whole at its configured extent,
together with the trace that last changed each cell. The three hydrology kinds —
surface water, soil water, and groundwater — project the chunk's own 32 x 32
lattice whole, like mana: a block mean of volumes would report a quantity no cell
holds, and changing hydrology's detail is a conservative resolution transition
inside the simulation rather than a reduction an observer may ask for. Values travel as packed ZigZag varints of
successive differences along the scan order, which is why nine chunks of elevation and roughness
encode to about 3.4 KB each against 8 KB of raw arrays.

The runtime performs exactly one reduction on a raster, the block mean, and it is order-independent
and introduces no value the field does not contain. It never flattens the mana volume to plan view:
a column sum and a column maximum answer different questions, so choosing between them is a reading
of the field rather than a property of it and the map states which reading it draws. It never shades
a surface either, because lighting is presentation (INV-022).

A request for a chunk outside the active set is answered `NotAvailable` and a malformed one
`InvalidRequest`; neither is answered with substituted data.

`MAX_QUERY_PAYLOAD_BYTES` bounds a request and a distinct
`MAX_QUERY_RESPONSE_PAYLOAD_BYTES` bounds a response. They happen to be the same
size and are two constants because they protect different parties: one limits
what a peer may ask this runtime to parse, the other what a client must be
willing to allocate. The response cap is enforced before the bytes are emitted
and again before a decoder copies them.

The world projection contains numeric terrain bounds, roughness, local mana total, causal-resolution
relevance/level, population aggregate, activity count, and trace anchor. Its additive
`MaterialSurfaceDelta` V3 projection is bounded and includes a chart-qualified chunk, cell ordinal,
typed before/after condition, mana total, transition tick, and optional contact, mana-effect, and
mana-transition trace anchors. Optional in-world mana before/after values are present only when a
bounded source receipt supports them. V3 adds optional matching-cell local mana before/after values
and the local mana transition trace. A separate bounded `MaterialSurfaceGateDelta` exposes
gate-only falling transitions without inventing a condition change. A third and independent bounded
projection, `MaterialSurfaceThermalDelta` (V4), exposes a surface's retained-heat exchange with its
co-located thermal cell — before/after retained energy, the cell's frozen pre-state, the signed
flux, and the exchange trace — without a temperature figure, expansion, damage, or phase-change
label; that response is not modeled yet (`TODO-THERMAL-002`). It shares its schema-version field with
the other two, since all three address the same `MaterialSurfaceId`, unlike the cell-addressed
`ThermalFieldDelta`, which keeps its own separate version field. Neither this nor the other two
projections contain a city, biome, species, occupation, spell, or other semantic classification.

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

## Hydrology

Hydrology is additive to V1. `RuntimeSummary` fields 36 to 42 are one atomic
group — a schema version, four `u128` storage totals, the signed `i128` residual
of the latest committed batch, and the active chunk count — written even when the
domain is disabled, because "this build has no hydrology" and "this world holds
no water" are different facts. Fields 43 to 47 describe the greatest applied
forcing record and are either all present or all absent; they disappear once
retention evicts the batch that evidenced them, since an identity beside
fabricated zeroes would be worse than absence.

`WorldChunkSnapshot` fields 9 to 14 carry three bounded lists — per-cell storage
deltas, transfer summaries, and conveyance summaries — each capped at 64 with its
own schema marker. Unlike the older material-surface and thermal lists, which
silently drop the excess, these reject entry 65: a bound a peer can exceed
without being told is not a bound. Duplicate cell, transfer, or conveyance keys
within a tick reject decoding, because two rows for one cell in one tick disagree
about what that cell did.

Hydrology rasters travel in `FieldRaster` fields 13 and 14, a packed band of
shortest-form `u64` varints. The band is mutually exclusive with the signed
`values` and `auxiliary` bands: a water volume is a `u64` whose upper half has no
signed image, and an elevation is signed. Rust exposes `Vec<u64>` and TypeScript
`BigUint64Array`; neither converts through `i64` or `Float64Array`.

Carrier keys travel as opaque fixed-length bytes with a leading variant code, and
each decoder validates them against the declared encoding rather than by
importing the simulation. Unknown variants, wrong lengths, unknown face
directions, reversed edge endpoints, and a non-cell carrier named as both ends of
one transfer are all rejected. A *cell* may legitimately be both ends:
infiltration, percolation, and evapotranspiration move water between buckets
inside one cell.

Every hydrology byte integer must be in shortest canonical form. A value that
admits two encodings admits two byte strings for one payload, and the digest of a
payload is an identity.

The canonical runtime bootstrap has seven stages, and field 35 keeps its frozen
six-receipt bound: the seventh, hydrology, receipt is projected separately in
optional field 48. Fields 31 and 32 keep their frozen meanings — a projected
six-stage count and six-stage completion — so a pre-hydrology decoder reads
exactly what it always read. New clients define complete hydrology bootstrap as
that legacy predicate plus a valid field-48 stage-seven receipt.

A frozen copy of the pre-hydrology TypeScript decoder is kept as an oracle and
driven by
[`tools/audit/test-observer-hydrology-legacy-decoder.mjs`](../../tools/audit/test-observer-hydrology-legacy-decoder.mjs):
it decodes a payload carrying fields 36 to 48 and the world sections 9 to 14 to
exactly what it decodes without them. Editing the oracle is a test failure, not a
re-freeze.

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
