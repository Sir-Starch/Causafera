# Ground Truth Events and Causal Provenance

Every accepted authoritative property transition is represented by a committed causal event. The Phase 6 implementation lives in `causafera-core::provenance` and follows:

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

The current implementation stores complete canonical causal and domain-evidence records in append-only bundle segments with a transactional catalog. A bounded hot overlay and generation-pinned cold reader support direct lookup, child traversal, and historical Explanation queries. Checkpoints and snapshots carry a history anchor instead of embedding the entire event stream; derived indexes can be rebuilt. See [RFC-PERSIST-002](../rfc/RFC-PERSIST-002.md) and the [storage plan](../../plans/causal-history-storage.md). Cold storage and catalog size grow with history, and the measured wet reopen cost remains open as `TODO-PERF-004`.

## Determinism

Commit order does not depend on producer scheduling or input vector order. The store uses no random source, system time, locale, pointer identity, semantic string, or hash-map traversal. Identical prior store state and proposal batches produce identical event and trace identities.

## Domain integration

Domain systems construct property-specific canonical fingerprints and proposals. Existing terrain generation and pathogen exposure `TraceId` references can now point into this graph when their producing systems are integrated. The existence of the graph does not itself implement biological mutation, terrain synthesis, or other domain state changes.

## Observer and explanation

The store exposes read-only numeric event views. Current Explanation queries can traverse committed cold history after hot eviction, with bounded pages and explicit partial or unknown results when evidence is unavailable. Further observer and Explanation adapters may attach non-authoritative localized glosses and report confidence. Neither adapter may mutate the graph or invent missing causes.

## Epistemic Provenance and Agent-Perspective Explanation

Causal provenance answers: “Why did this happen in the world?”
Epistemic provenance answers: “Why did this agent know, suspect, believe, or falsely believe this?”

Agent-perspective explanation requires explicit epistemic provenance. It conceptually answers: “Why did agent A, at tick T, believe or explain event E as hypothesis H?” The explanation must be time-specific; later knowledge must not silently rewrite what the agent believed earlier.

An agent-perspective explanation must be able to trace:
- what signals were physically accessible to the agent;
- what the agent actually perceived;
- the subjective scene formed from those perceptions;
- relevant memories activated at that time;
- testimony or information received from other agents;
- trust in those information sources;
- concepts and prior beliefs available to the agent;
- causal hypotheses considered;
- the selected interpretation;
- confidence or uncertainty;
- important unavailable or contradictory evidence;
- how the interpretation affected later belief or action.

Different agents must be able to form different explanations of the same event because they have different perceptions, memories, concepts, testimony, relationships, and prior beliefs. Beliefs learned through communication must retain their informational lineage (e.g., event → witness perception → witness interpretation → testimony → listener trust and prior beliefs → listener interpretation).

Subjective explanations are not authoritative Ground Truth. Agents must not have access to hidden runtime state, causal traces, operator intent, or other information they did not perceive or learn causally. The authoritative basis should remain structured subjective state and its history. Human-readable text, including future LLM-generated phrasing, is only a presentation layer and must not invent reasons absent from the agent state.

Full memory, belief formation, hypothesis comparison, social testimony, and agent-perspective Explanation remain proposed future work and are not currently implemented.

## Related documents

- `docs/rfc/RFC-TRACE-001.md`
- `docs/architecture/determinism.md`
- `docs/ontology/causal-carriers.md`
- `docs/explanation/causal-summaries.md`
- `docs/architecture/invariants.md` (INV-041)
