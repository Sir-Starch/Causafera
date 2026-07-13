# Backpressure

A slow UI cannot indefinitely stall simulation. The observer layer implements backpressure policies to manage flow control.

## Delivery Policies

- **reliable ordered** - All messages delivered in order (control responses);
- **latest-state-wins** - Drop intermediate states, deliver only latest (mana visualization);
- **coalesced** - Merge adjacent updates into single delivery (entity position streams);
- **sampled** - Deliver periodic samples (performance telemetry);
- **request/response** - Only deliver when explicitly requested (entity inspector).

## Policy Assignment

Example assignments:

```text
control response → reliable ordered
mana visualization → latest-state-wins
performance telemetry → sampled
entity inspector → request/response
```

## Queue Limits

No unbounded observer queues. Every stream must have defined capacity limits and overflow behavior.

`ObserverStreamHub` enforces non-zero capacity per subscription. Unsubscribing removes the queue,
so a closed UI panel consumes no subsequent delivery work.

## Overflow Behavior

When queues overflow:

- drop oldest (for latest-state-wins);
- drop newest (for sampled);
- signal backpressure (for reliable ordered);
- return error (for request/response).

## Related Documents

- `docs/observer/protocol.md` - Protocol stream definitions
- `docs/observer/snapshots.md` - Snapshot and delta streaming
- `docs/performance/philosophy.md` - Performance goals including observer overhead
