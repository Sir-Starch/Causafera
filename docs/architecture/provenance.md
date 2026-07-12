# Ground Truth Events and Causal Provenance

Every accepted authoritative property transition is represented by a committed causal event. The Phase 6 implementation lives in `ontopolis-core::provenance` and follows:

```text
READ
→ PROPOSE
→ REDUCE BY STABLE NUMERIC KEY
→ COMMIT EVENT + TRACE
```

## Event boundary

A proposal contains:

- `EventProposalKey(system_id, subject_ordinal, operation_ordinal)`;
- opaque `EventKindId` schema identity;
- strictly ordered prior `TraceId` causes;
- one or more strictly ordered `CausalEffect` property transitions.

Each effect identifies a state object through opaque `StateObjectKindId`, numeric object identity, and `StatePropertyId`. It stores caller-supplied before/after fingerprints of canonical property representations. Fingerprints must differ. Human event names and property labels are not authoritative state.

## Commit and traversal

`CausalTraceStore` stably sorts a batch by proposal key, rejects duplicate keys and unknown causes, preflights capacities, then assigns monotonic `EventId` and `TraceId` values. Causes must have been committed before the current batch. This parent-before-child rule prevents causal cycles and makes forward ancestry slices canonical.

Hot event fields, causes, and effects use flat vectors with offsets. A deterministic cold side index supports direct child traversal. Full causal query planning, persistence, and observer projection remain future work.

## Determinism

Commit order does not depend on producer scheduling or input vector order. The store uses no random source, system time, locale, pointer identity, semantic string, or hash-map traversal. Identical prior store state and proposal batches produce identical event and trace identities.

## Domain integration

Domain systems construct property-specific canonical fingerprints and proposals. Existing terrain generation and pathogen exposure `TraceId` references can now point into this graph when their producing systems are integrated. The existence of the graph does not itself implement biological mutation, terrain synthesis, or other domain state changes.

## Observer and explanation

The store exposes read-only numeric event views. Future observer and Explanation Engine adapters may traverse these views, attach non-authoritative localized glosses, and report confidence. Neither adapter may mutate the graph or invent missing causes.

## Related documents

- `docs/rfc/RFC-TRACE-001.md`
- `docs/architecture/determinism.md`
- `docs/ontology/causal-carriers.md`
- `docs/explanation/causal-summaries.md`
