# Observer Protocol

The observer protocol defines the communication boundary between the headless authoritative simulation and the desktop observer application.

## Protocol Technology

The protocol uses Protocol Buffers for schema definition and serialization.

Schema versions are maintained under:

```text
proto/causafera/observer/v1/
```

## Initial Protocol Categories

```text
common.proto      - shared types and primitives
control.proto     - simulation control commands and responses
query.proto       - request/response queries for entities and state
stream.proto      - subscription and delta streaming
spatial.proto     - chunks, coordinates, terrain summaries
entity.proto      - agent, object, and organization snapshots
causal.proto      - causal traces and provenance summaries
language.proto    - lexeme, language, and semantic drift data
explanation.proto - Explanation IR and rendered explanations
metrics.proto     - performance and telemetry data
```

## Bindings

Generate Rust bindings for the simulation side and TypeScript bindings for the UI side. Do not manually duplicate schemas. The single source of truth is the `.proto` definition.

The Rust and TypeScript v1 codecs cover negotiation, runtime-summary and chart-qualified
world-chunk query/response, per-chunk field rasters, typed Explanation IR payloads, and stream
envelopes. The Tauri bridge transports only these bytes.

`FieldRaster` is the one request whose answer depends on what was asked rather than on when, so its
parameters ride the query payload and the session answers it directly instead of from the payload
cache the other kinds share. It stays additive: the version remains v1, capability negotiation
advertises the kind so a client can tell whether a runtime supports it, and existing decoders skip
what they do not know. Schema validity is checked with `protoc`; expanding
either codec must begin with the v1 `.proto` files.

## What the Protocol Is Not

- Not simulation persistence format
- Not internal domain representation
- Not causal trace storage
- Not authoritative state

The protocol is a derived read-only view of simulation state, optimized for observer consumption.

## Snapshot and Delta Streaming

Observer delivery uses scoped snapshots plus incremental deltas:

```text
scoped snapshot
+
incremental deltas
```

Streams require:

- identity
- schema version
- sequence number
- simulation time
- physical and history digest anchors
- an explicit snapshot/delta flag

Possible streams:

```text
simulation_clock
world_chunks
entity_changes
population_aggregates
mana_field
causal_activity
language_changes
concept_changes
resolution_changes
metrics
```

A closed UI panel must not require updates for its hidden data. The simulation must not waste resources streaming data to uninterested observers.

## Backpressure

A slow UI cannot indefinitely stall simulation. Delivery policies include:

- **reliable ordered** - control responses, critical state
- **latest-state-wins** - visualizations, field data
- **coalesced** - high-frequency updates merged
- **sampled** - telemetry, metrics
- **request/response** - entity inspection, specific queries

Examples:

```text
control response        → reliable ordered
mana visualization      → latest-state-wins
performance telemetry   → sampled
entity inspector        → request/response
```

No unbounded observer queues. The protocol must drop or coalesce data rather than accumulate infinite backlog.

## Versioning

Protocol schemas are versioned. Breaking changes require a new version directory:

```text
proto/causafera/observer/v2/
```

The simulation and UI negotiate supported versions at connection time.

An *additive* change stays inside the current version, and the bar for calling
one additive is that a decoder built before the change reads a new payload and
gets exactly what it always got. Hydrology is the worked example: it appended
runtime-summary fields 36 to 48, world-snapshot fields 9 to 14, and two raster
fields, while every field a pre-hydrology consumer reads — including the
six-receipt bootstrap bound, the projected stage count, and the completion flag —
kept its frozen meaning, with the seventh bootstrap receipt projected into its own
optional field rather than into the existing list. The claim is not asserted, it
is tested: a frozen copy of the pre-hydrology decoder is kept as an oracle and
must decode a new payload to exactly what it decodes without the additions.
Editing the oracle is a test failure, not a re-freeze.

## Security Model

The observer protocol is read-only. There is no mutation path from the UI back to authoritative simulation state through the observer protocol. Control commands such as pause, resume, or speed change affect simulation execution parameters, not simulation content.
