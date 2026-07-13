# Snapshots and Deltas

Observer delivery uses scoped snapshots plus incremental deltas to keep the UI synchronized with simulation state.

## Delivery Model

```text
scoped snapshot
+
incremental deltas
```

## Stream Properties

Streams require:

- identity (stream identifier);
- schema version;
- sequence number;
- simulation time.
- physical and history digest anchors;
- an explicit initial-snapshot marker.

A stream rejects a delta until its initial scoped snapshot has been accepted. Sequence numbers
advance only for queued/replaced/coalesced messages; a sampled message dropped before enqueue does
not create an artificial sequence gap.

## Possible Streams

- `simulation_clock` - Time progression
- `world_chunks` - Spatial chunk updates
- `entity_changes` - Individual entity state changes
- `population_aggregates` - Aggregated population statistics
- `mana_field` - Mana field state
- `causal_activity` - Causal trace events
- `language_changes` - Lexical and semantic updates
- `concept_changes` - Concept formation and evolution
- `resolution_changes` - Causal resolution field updates
- `metrics` - Performance telemetry

## Scope Management

A closed UI panel must not require updates for its hidden data. The observer system must support scoped subscriptions that can be opened and closed dynamically.

## Related Documents

- `docs/observer/protocol.md` - Protocol defining stream messages
- `docs/observer/backpressure.md` - Managing stream flow
- `docs/observer/architecture.md` - Observer read model
