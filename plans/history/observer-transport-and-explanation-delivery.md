# Observer Transport and Explanation Delivery ExecPlan

> **Historical record.** This completed ExecPlan describes a Foundation Era project stage. Its implementation status and terminology may be outdated; use [the documentation index](../../docs/index.md), [roadmap](../../docs/roadmap/roadmap.md), and [active plans](../../PLANS.md) for current guidance.

## Goal

Deliver a bounded, versioned, read-only observer transport from runtime snapshots and Explanation IR through deterministic Protocol Buffer bytes, with scoped snapshot/delta streams, explicit overflow policies, locale-independent authoritative digests, and end-to-end verification.

## Context

Phase 24 and the post-phase-24 depth plan provide an executable simulation, separated physical/history digests, Explanation IR, and exact persistence. The observer crates and v1 schemas remain placeholders. This plan closes the boundary required before a rich UI can inspect real runs.

## Relevant invariants

INV-006, INV-007, INV-012, INV-013, INV-014, INV-017, INV-018, INV-019, INV-021, INV-022, INV-026, INV-036, and INV-037.

## Ontology domains affected

Simulation runtime, causal provenance, analytics, explanation, and observer projections. Observer types are derived and introduce no authoritative ontology.

## Causal carriers affected

No authoritative carrier changes. Digest anchors and trace references are copied into read-only projections.

## Relevant documents

`docs/architecture/{observer,protocol,determinism,performance}.md`, `docs/observer/*.md`, `docs/explanation/{architecture,explanation-ir,deterministic-rendering,localization}.md`, `docs/rfc/RFC-EXPLAIN-001.md`, ADR-004, and ADR-006.

## Current state

`RuntimeSnapshot` exposes bounded numeric summaries and independent digests. Explanation IR is typed and deterministic. The observer API contains only skeletal query/stream types, `ProtocolHandler::handle_query` returns empty bytes, and the proto schemas contain stringly typed placeholders.

## Proposed architecture

```text
RuntimeSnapshot / ExplanationReport
  -> ObserverSnapshot / ObserverExplanation (read-only projection)
  -> ProtocolHandler (canonical protobuf wire)
  -> bounded scoped StreamHub
  -> external client
```

The runtime constructs observer API values; observer-wire never imports runtime internals. Text rendering is downstream from IR and locale is never passed into runtime execution.

## Primitive vs emergent review

Protocol kind IDs and claim schema IDs are transport discriminants, not simulated semantic categories. Human labels and templates exist only in deterministic rendering resources.

## Non-goals

Network sockets, Tauri integration, simulation mutation/control, full entity/world projections, an LLM surface, and scale claims.

## Implementation stages

1. Define bounded observer read-model projections and stable query/result contracts.
2. Replace v1 string placeholders with numeric/versioned protobuf schemas.
3. Implement deterministic query/response protobuf roundtrips and negotiation.
4. Implement snapshot/delta envelopes with sequence and digest anchors.
5. Implement bounded scoped queues with reliable, latest-wins, coalesced, sampled, and request/response behavior.
6. Add deterministic localized Explanation IR rendering with honest fallback behavior.
7. Prove observer locale and activity cannot change authoritative physical/history digests.
8. Add end-to-end and overhead measurements; reconcile documentation and TODO status.

## Verification

Unit roundtrips, malformed-input rejection, deterministic byte equality, queue overflow tests, snapshot-before-delta enforcement, locale digest equality, runtime-to-wire decode, `cargo fmt --check`, clippy, workspace tests, and `git diff --check`.

## Benchmark plan

Measure projection plus encoding for observer-off, idle, normal snapshot, heavy repeated query, and explanation rendering workloads. Record measurements without scale claims or hard CI timing thresholds.

## Determinism impact

Canonical field order, monotonically increasing per-stream sequence numbers, stable scope ordering, and digest anchors. Locale, wall time, queue state, rendered text, and observer presence remain outside authoritative state and digests.

## Memory impact

Every stream has an explicit non-zero capacity. Overflow either rejects, drops, replaces, or coalesces according to policy. No observer queue is unbounded.

## Observer impact

Establishes the first usable external read-only boundary. Closed scopes produce no updates.

## Explanation impact

Typed IR is transported without prose. Deterministic localized prose is an optional downstream payload and preserves evidence state and confidence.

## Persistence impact

None. Observer protocol is explicitly distinct from snapshot persistence and is not a resume format.

## Cross-domain effects

Runtime summary metrics, mana/resolution activity, biological actor counts, population flow, and causal trace anchors become inspectable but cannot feed back.

## Risks

String semantics leaking into protocol, queues affecting runtime scheduling, delta gaps, locale leakage, accidental persistence reuse, and overclaiming performance. Mitigate with numeric schemas, ownership separation, bounded queues, resnapshot signals, digest tests, and measured-only reporting.

## Documentation changes

Update observer, explanation rendering, protocol, benchmark, roadmap/domain coverage, changelog, RFC status, and documentation index as needed.

## TODO changes

Complete TODO-EXPLAIN-001, TODO-EXPLAIN-002, TODO-OBSERVER-001, TODO-PROTO-001, TODO-OBSERVER-002, and TODO-DET-001 when their acceptance gates pass. TODO-PERF-001 remains pending unless the general benchmark framework is delivered.

## Decision log

- 2026-07-13: Transport consumes explicit derived values rather than importing runtime storage.
- 2026-07-13: v1 supports bounded in-process delivery first; sockets are a later adapter.
- 2026-07-13: Observer protocol and persistence remain unrelated formats.

## Progress

- [x] 1. Observer read model.
- [x] 2. Protocol v1 schemas.
- [x] 3. Query roundtrip and negotiation.
- [x] 4. Snapshot/delta envelopes.
- [x] 5. Backpressure policies.
- [x] 6. Deterministic renderer.
- [x] 7. Locale independence.
- [x] 8. End-to-end verification, measurements, and documentation.

All stages completed on 2026-07-13. `protoc` schema validation, strict workspace
clippy, and `cargo test --workspace --all-features` passed. The TypeScript client adapter was
updated, but its local typecheck could not provision existing frontend dependencies because npm
DNS was unavailable in the execution environment.
