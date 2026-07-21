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

The implemented v1 surface supports negotiation and three request kinds:

- runtime summary;
- typed Explanation IR;
- a bounded chart-qualified world-chunk snapshot.

The world projection contains numeric terrain bounds, roughness, local mana total, causal-resolution
relevance/level, population aggregate, activity count, and trace anchor. Its additive
`MaterialSurfaceDelta` V2 projection is bounded and includes a chart-qualified chunk, cell ordinal,
typed before/after condition, mana total, transition tick, and optional contact, mana-effect, and
mana-transition trace anchors. Optional in-world mana before/after values are present only when a
bounded source receipt supports them. It does not contain a city, biome, species, occupation,
spell, or other semantic classification.

`MaterialSurfaceDelta` is an inspection projection, not the material state or a mutation API. Its
schema version and capacity remain explicit in the existing `world_chunks` response; the projection
retains at most 64 material deltas and preserves the newest mana-mediated transition when the
bounded window is full. A query, locale, or UI rendering cannot alter authoritative state.

The trace anchors and optional mana values use protobuf field presence, not a numeric sentinel: an
omitted value is different from an explicitly encoded `TraceId(0)` or zero-valued field. Consumers
must preserve that distinction when round-tripping V2 data, so bootstrap-only surface state cannot
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
