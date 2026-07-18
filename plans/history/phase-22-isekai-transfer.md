# Phase 22 Isekai Transfer and Imported Priors

> **Historical record.** This completed ExecPlan describes a Foundation Era project stage. Its implementation status and terminology may be outdated; use [the documentation index](../../docs/index.md), [roadmap](../../docs/roadmap/roadmap.md), and [active plans](../../PLANS.md) for current guidance.

## Goal
Implement a bounded, deterministic, metaphysically neutral cross-world transfer contract and keep imported subjective priors structurally separate from locally evidenced capability.

## Context
Phase 21 can orchestrate causal history, while the isekai crate still contains a semantic transfer enum and no provenance, bounds, proposal/receipt boundary, or knowledge/capability separation.

## Relevant invariants
INV-001, INV-002, INV-006, INV-014 through INV-016, INV-019, INV-025, INV-027, and INV-030.

## Ontology domains affected
Isekai, metaphysics boundary, cognition, practices, materials, measurement, provenance, history, and place.

## Causal carriers affected
Opaque mechanism schemas, objective payload fingerprints, property correspondences, source/target locations, transfer receipts, subjective prior patterns, and independently evidenced capability prerequisites.

## Relevant documents
Isekai architecture and subsystem documents, cross-world continuity, identity, cognition rebaseline, ADR-002/003, RFC-COG-001, RFC-TRACE-001, and RFC-HIST-001.

## Current state
`TransferType` is a semantic candidate enum without execution or provenance semantics. Imported priors are documentation only.

## Proposed architecture
Replace the enum with an opaque mechanism-schema plan. Canonical objective payloads and property correspondences describe inputs without deciding identity metaphysics. A committed receipt must exactly cover payloads and continue declared traces. Imported priors contain only subjective patterns, weights, sources, and transfer provenance. Reproduction requirements are assessed exclusively against separate local practice/material/resource/measurement evidence.

## Primitive vs emergent review
IDs, fingerprints, time, place, correspondence, traces, weights, and prerequisite membership are bookkeeping. Reincarnation, soul, copy, hero, technology, truth, usefulness, social response, and historical significance remain hypotheses or emergent classifications.

## Non-goals
Final identity metaphysics, Soul objects, actual transport physics, body construction, cognition mutation, technology trees, translation, fake arrivals/history, observer protocol, persistence, Phase 23 experiments, or performance claims.

## Implementation stages
1. Replace semantic transfer candidates with bounded opaque plan/receipt carriers and deterministic seed derivation.
2. Add imported-prior and independent capability-evidence contracts.
3. Test canonicalization, exact causal continuation, and knowledge/capability separation.
4. Accept RFC-ISEKAI-001 and update subsystem, ontology, roadmap, TODO, changelog, and phase registry docs.

## Verification
Workspace tests, strict clippy, formatting, diff checks, semantic boundary searches, and knowledge-graph refresh.

## Benchmark plan
No performance claim; benchmark only with a concrete transfer adapter.

## Determinism impact
Canonical vectors, explicit seed inputs, integer-only mixing, no entropy/hash iteration/locale/floating point.

## Memory impact
Payload, property, cause, prior, and requirement vectors have hard caps.

## Observer impact
None; future read-only views may gloss mechanism schemas and traverse receipt traces.

## Explanation impact
Future explanations can distinguish subjective foreign influence from physical capability evidence without inventing missing causes.

## Persistence impact
Deferred; snapshots must preserve opaque schema versions, canonical order, fingerprints, and traces.

## Cross-domain effects
Adapters may map receipt traces to historical stages and imported patterns into normal subjective cognition paths. Neither contract mutates those systems.

## Risks
Opaque schemas could become hidden semantic enums; transfer plans could be mistaken for mutation authority; imported priors could be treated as facts; requirements could be mistaken for a technology tree.

## Documentation changes
Accept RFC-ISEKAI-001 and update isekai, metaphysics boundary, ontology, roadmap, TODO, changelog, rebaseline, index, and PLANS.

## TODO changes
Complete TODO-ISEKAI-001 and TODO-ISEKAI-002. Leave Phase 23 research pending.

## Decision log
- 2026-07-12: Mechanism identity is opaque and does not select a metaphysical theory.
- 2026-07-12: Transfer receipt ancestry must exactly continue the plan.
- 2026-07-12: Imported subjective patterns never satisfy capability prerequisites.

## Progress
- [x] Transfer plan/receipt contract implemented.
- [x] Imported-prior/capability separation implemented.
- [x] Documentation and phase tracking updated.
- [x] Full verification passes.
