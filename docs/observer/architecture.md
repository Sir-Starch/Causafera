# Observer Architecture

The simulation remains headless and authoritative. The observer layer provides read-only access to derived simulation state for UI and analytics.

## Architecture Stack

```text
AUTHORITATIVE SIMULATION
↓
OBSERVER READ MODEL
↓
EXPLANATION ENGINE
↓
VERSIONED OBSERVER API
↓
DESKTOP APPLICATION
```

## Key Principles

- The UI never directly reads simulation internal storage.
- The UI never owns authoritative state.
- Observer data is derived, not primary.
- All observer access goes through defined protocol boundaries.

## Observer Read Model

The read model:

- extracts relevant simulation state;
- applies analytical classifications;
- maintains versioning for consistency;
- supports scoped queries and subscriptions.

## Explanation Engine Integration

The Explanation Engine sits between the read model and the UI. It converts structured state into human-understandable explanations without modifying state.

## Related Documents

- `docs/observer/protocol.md` - Protocol Buffers schema
- `docs/observer/snapshots.md` - Snapshot and delta streaming
- `docs/observer/backpressure.md` - Flow control
- `docs/explanation/architecture.md` - Explanation pipeline
- `docs/architecture/invariants.md` - INV-021: UI is an observer
