# RFC-TRACE-001: Deterministic Ground Truth Event Provenance

**Status:** Accepted

## Summary

Define the Phase 6 authoritative event format and append-only causal graph without semantic event strings, cyclic ancestry, or scheduling-dependent identity allocation.

## Motivation

INV-014 and INV-019 require significant state changes and surprising phenomena to be reconstructable from stored provenance. Prior phases introduced `EventId` and `TraceId`, and terrain/pathogen contracts retain trace references, but no graph validated or resolved those references. Phase-controlled mutation also requires a deterministic proposal/reduce/commit boundary.

## Authoritative records

`CausalEventProposal` contains a stable numeric proposal key, opaque `EventKindId`, ordered prior causes, and ordered effects. An effect identifies an objective property through `CausalTarget(StateObjectKindId, object_id, StatePropertyId)` and stores distinct 32-byte before/after fingerprints.

IDs identify registered binary schemas and state slots. They do not encode English labels or developer taxonomies. A future registry may associate observer metadata with those IDs outside authoritative semantics.

## Proposal, reduction, and commit

Producers read authoritative state and emit validated proposals. `CausalTraceStore::commit_batch`:

1. stably orders proposals by `(system_id, subject_ordinal, operation_ordinal)`;
2. rejects duplicate proposal keys;
3. rejects causes absent from the prior committed graph;
4. preflights ID and flat-offset capacity;
5. assigns monotonic event and trace IDs;
6. appends event fields, cause edges, effects, and reverse child edges.

Causes cannot refer to another proposal in the same batch. Later scheduler phases or ticks may refer to the committed trace. This establishes parent-before-child ancestry and excludes cycles by construction.

## Storage

Event IDs, trace IDs, times, phases, kind IDs, causes, and effects use structure-of-arrays/flat-offset storage. Direct causes are contiguous. Direct children use a deterministic cold `BTreeMap<TraceId, Vec<TraceId>>` side index. The latter is an explicit starting point, not an unbenchmarked scale claim.

## Determinism

The same prior graph and logical proposal set produce the same store regardless of proposal input order. No RNG, floating computation, locale, system clock, pointer order, or hash traversal participates.

## Primitive and emergent boundary

Objective property transitions, schema identity, canonical fingerprints, simulation time, scheduler phase, and causal edges are primitive. Named events such as plague, discovery, battle, or miracle are not event kinds in core. Observer analytics may later classify causal subgraphs without feeding those labels back into simulation.

## Observer and explanation impact

No wire protocol changes are made. Numeric borrowed event views are sufficient for a future read-only adapter. Explanation systems may use the graph as evidence but cannot mutate it or fill gaps narratively.

## Persistence impact

No snapshot representation is selected. Persistence remains deferred to `TODO-PERSIST-001`; a future format must preserve IDs, canonical order, schema registry versions, fingerprints, and edges exactly.

## Non-goals

- Domain-specific event taxonomies or mutation systems.
- Full graph query language, transitive index, pruning, compaction, or distributed merge.
- Observer protobuf changes or Explanation IR.
- Defining domain canonical-state serialization.
- Performance or scale claims.

## Decision log

- **Accepted:** Event kinds and target schemas are opaque typed IDs, not strings or semantic enums.
- **Accepted:** Proposal keys, not producer execution order, determine commit order.
- **Accepted:** Causes must pre-exist the batch, guaranteeing acyclic parent-before-child ancestry.
- **Accepted:** Effects retain before/after canonical fingerprints rather than copies of arbitrary domain values.
- **Accepted:** Reverse children are a deterministic cold side index pending benchmark evidence.
