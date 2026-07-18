# Phase 18 Causal Resolution Field

> **Historical record.** This completed ExecPlan describes a Foundation Era project stage. Its implementation status and terminology may be outdated; use [the documentation index](../../docs/index.md), [roadmap](../../docs/roadmap/roadmap.md), and [active plans](../../PLANS.md) for current guidance.

## Goal

Implement a bounded deterministic field that assigns simulation detail from trace-backed causal relevance rather than physical distance alone.

## Context

Phase 17 completed the minimal mana field. `causafera-resolution` still contains only an unchecked chunk/byte pair, while Phase 19 depends on an explicit resolution contract.

## Relevant invariants

INV-006, INV-009, INV-010, INV-014, INV-016, INV-017, INV-018, INV-019, and INV-023.

## Ontology domains affected

Space, geography, provenance, mana, future society/economy, observer projection, and simulation scheduling.

## Causal carriers affected

Opaque trace-backed relevance signals between chunks and traced resolution-state transitions.

## Relevant documents

`docs/architecture/invariants.md`, `docs/architecture/determinism.md`, `docs/architecture/provenance.md`, `docs/world/spatial-hierarchy.md`, `docs/world/world-generation-provenance.md`, `docs/ontology/causal-carriers.md`, and `docs/rfc/RFC-RES-001.md`.

## Current state

`ResolutionConfig` is an unchecked `ChunkId` plus `u8`. There are no relevance inputs, bounds, deterministic reduction, transition policy, provenance, or proposal/commit boundary.

## Proposed architecture

Store a bounded, canonically ordered structure-of-arrays field indexed by authoritative `ChunkId`. Each entry contains fixed-point relevance, a bounded numeric detail level, and the trace of its last committed change. A validated policy supplies decay, saturation, level thresholds, and bounded weights for opaque relevance-channel IDs. Pure evaluation canonicalizes trace-backed directed signals, reduces them into target chunks, applies prior-state decay and threshold hysteresis, and returns replacement-state changes with ordered causes. Commit requires one new trace per changed entry.

## Primitive vs emergent review

Numeric relevance, opaque carrier channels, directed chunk linkage, thresholds, decay, hysteresis, bounds, and detail levels are primitives. Trade, migration, political influence, historical importance, observer interest, settlements, and institutions are not resolution-engine concepts; adapters may encode their physical causal effects into registered opaque channels.

## Non-goals

Full multi-resolution domain aggregation, entity promotion/demotion, historical bootstrap, scheduler integration, observer protocol changes, semantic carrier taxonomy, automatic distance calculation, persistence, GPU execution, or scale claims.

## Implementation stages

1. Replace the placeholder with validated policy, signal, field, and error contracts.
2. Implement canonical fixed-point proposal evaluation, hysteresis, causes, and traced commit.
3. Test canonical ordering, remote relevance, decay/transitions, bounds, and commit provenance.
4. Accept RFC-RES-001 and update roadmap, TODO, ontology, subsystem docs, changelog, and plan registry.

## Verification

Run workspace tests, strict clippy, formatting check, diff check, architectural searches for floats/strings/subjective IDs in the resolution engine, and refresh the code knowledge graph.

## Benchmark plan

No throughput claim is made. Benchmark proposal cost by field entries, signal count, and channel count before changing the bounded CPU structure-of-arrays baseline.

## Determinism impact

No RNG, floats, locale, system time, or unordered traversal. Policies, entries, signals, causes, and commits have canonical numeric order; arithmetic is checked or saturating integer arithmetic.

## Memory impact

Field entries, policy channels, signals, changed entries, and per-change causes are explicitly capped. Hot state uses parallel vectors and binary search.

## Observer impact

Future read-only projections may expose numeric relevance/detail and supporting traces. No observer API changes are included.

## Explanation impact

Transition records explain which physical/informational carrier traces supported a resolution decision without assigning semantic meaning to opaque channels.

## Persistence impact

No persistence format change. Serialization and migration remain future work.

## Cross-domain effects

Domain adapters may emit causal signals from physical proximity, flows, shared provenance, mana activity, or future social/material processes. Resolution selects computation detail but does not rewrite source-domain truth.

## Risks

- Integration code could treat channel IDs as a hidden semantic enum; registration remains opaque and authoritative labels are forbidden.
- Repeated score decay may generate transitions; unchanged replacement entries are omitted and future scheduler policy may batch evaluation cadence.
- Numeric levels do not themselves implement aggregation correctness; domain-specific promotion/demotion contracts remain future work.

## Documentation changes

Expand and accept RFC-RES-001; update spatial hierarchy, ontology coverage/carriers/primitive review/assumptions, roadmap, TODO, changelog, rebaseline report, and documentation index.

## TODO changes

Complete `TODO-RES-001` only after all verification passes.

## Decision log

- 2026-07-12: Use opaque weighted carrier channels instead of a semantic relevance-dimension enum.
- 2026-07-12: Keep physical distance outside the core reducer; it is one possible adapter signal, not the definition of relevance.
- 2026-07-12: Phase 18 establishes the decision field and provenance boundary, not full domain aggregation.

## Progress

- [x] Field, policy, and signal contracts implemented.
- [x] Deterministic proposal/commit and tests implemented.
- [x] Verification and architectural checks pass.
- [x] RFC, roadmap, TODO, ontology, and subsystem documentation updated.
