# Observer Protocol

The observer protocol uses Protocol Buffers for structured communication between the simulation and UI.

## Schema Versioning

Version schemas under:

```text
proto/ontopolis/observer/v1/
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
relevance/level, population aggregate, activity count, and trace anchor. It does not contain a city,
biome, species, occupation, spell, or other semantic classification.

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
