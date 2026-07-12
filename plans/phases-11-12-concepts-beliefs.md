# Phases 11–12 Sparse Concepts and Subjective Beliefs

## Goal

Implement adjacent Phases 11–12 as a bounded deterministic cognitive learning layer over subjective scenes: sparse prototype formation, concept activation and revision, belief inertia, subjective source trust, and fallible causal hypotheses.

## Context

Phases 9–10 now provide identity-free scenes, bounded working context, episodic reactivation, prediction errors, agency associations, and temporal continuity. The remaining cognition placeholders do not yet retain subjective categories or stable mistakes.

## Relevant invariants

- INV-001, INV-002, INV-027 through INV-031: no Ground Truth identity or inaccessible information enters cognition.
- INV-032 and INV-035: learning is sparse and driven through active context and explicit prediction error.
- INV-005, INV-006: no observer classifications or English labels become simulation meaning.
- INV-017, INV-018: layouts are bounded and no scale claim is made without benchmarks.

## Ontology domains affected

Cognition only: subjective concepts, beliefs, evidence evaluation, trust, and causal hypotheses.

## Causal carriers affected

`PerceptId` remains the immediate subjective evidence handle. `ConceptId`, `BeliefId`, `EvidenceId`, `SubjectiveSourceId`, and pattern IDs are agent-local opaque references, not authoritative identities.

## Relevant documents

- `docs/architecture/invariants.md`
- `docs/architecture/cognition-rebaseline.md`
- `docs/simulation/emergent-concepts.md`
- `docs/cognition/belief-inertia.md`
- `docs/cognition/trust.md`
- `docs/cognition/prediction.md`
- `docs/rfc/RFC-COG-001.md`
- `docs/rfc/RFC-CONCEPT-001.md`

## Current state

Concept formation has no code. `Belief` is only a `ConceptId` and unconstrained `f32`, with no bounds, evidence, inertia, trust, update process, or causal inference.

## Proposed architecture

1. Accept RFC-CONCEPT-001 with a fixed-capacity prototype store fed only by explicit attended observations.
2. Canonicalize observations and update matching prototypes by deterministic integer running means; allocate monotonic subjective concept IDs for unmatched patterns.
3. Retain bounded activation, exemplar counts, predictive utility, and percept support without semantic category labels.
4. Replace the belief placeholder with fixed-capacity belief records and canonical evidence-batch updates.
5. Weight evidence by salience and subjective source trust, then apply explicit inertia so contradictory evidence can fail to reverse a belief.
6. Learn bounded source trust and bounded directed causal hypotheses from opaque subjective pattern IDs.

## Primitive vs emergent review

Opaque IDs, quantized signatures, signed evidence direction, confidence, similarity, counts, time, and bounded associations are computational primitives. Object kinds, propositions in English, emotions, social roles, truth labels, and named causes remain emergent or external.

## Non-goals

- Semantic concept enums, lexical labels, language, propositions encoded as strings, or objective truth access.
- Automatic global clustering, complete psychology, social reputation networks, goals, habits, or decision policy.
- Observer wire formats, persistence schemas, or scale claims.

## Implementation stages

1. Accept the concept RFC and add opaque subjective identifiers.
2. Implement sparse concept formation and activation.
3. Implement bounded beliefs, evidence/inertia, and source trust.
4. Implement bounded subjective causal hypotheses.
5. Update docs, roadmap, TODOs, coverage, changelog, and plan state.
6. Format, test, lint, check the diff, and refresh the code graph.

## Verification

Tests cover canonical input order, sparse allocation, prototype revision, fixed capacity, stable contradictory beliefs, trust-weighted evidence, trust revision, causal association bounds, time monotonicity, and absence of authoritative IDs. Run workspace fmt, tests, clippy with warnings denied, `git diff --check`, and refresh the code graph.

## Benchmark plan

No performance claim. Future benchmarks must measure update cost and bytes per agent at declared concept, belief, evidence, trust, and hypothesis bounds.

## Determinism impact

All arithmetic is integer/fixed-point. Inputs are sorted by opaque numeric IDs; all rankings have stable ID tie-breakers. No hash iteration, floating point, locale, wall clock, or global randomness participates.

## Memory impact

All stores and update batches have compile-time maxima. Updates scan only bounded active stores.

## Observer impact

No wire change. Numeric read-only analytics can be added later without exposing them back to simulation.

## Explanation impact

Beliefs retain bounded subjective evidence summaries and source hypotheses, allowing later explanations of stable errors. No narrative or authoritative classification is added.

## Persistence impact

In-memory contracts only. Versioned snapshot integration remains future work.

## Cross-domain effects

Phase 13 language may associate lexemes with subjective `ConceptId` values. Later practices and institutions may generate evidence and repetition, but cannot directly author belief truth.

## Risks

- Compact appearance prototypes are deliberately minimal and not a realistic category-learning claim.
- A signed evidence direction is a generic update relation, not an encoded proposition meaning.
- Source IDs are subjective hypotheses; adapters must never substitute authoritative `AgentId` values as hidden cognition.

## Documentation changes

Accept and expand RFC-CONCEPT-001; update concept, belief, trust, cognition architecture, ontology coverage, roadmap, TODO backlog, index, changelog, and rebaseline report.

## TODO changes

Complete TODO-CONCEPT-001 and TODO-COG-002. Leave TODO-LANG-001 pending.

## Decision log

- 2026-07-12: Batch Phases 11–12 because evidence-driven belief revision consumes the concept and prediction-error contracts directly and shares the same bounded subjective-state constraints.
- 2026-07-12: Defer Phase 13 because language bootstrap requires separate lineage, physical signal, and communication architecture work.

## Progress

- [x] Required architecture, cognition, ontology, ADR, RFC, roadmap, and current code reviewed.
- [x] RFC-CONCEPT-001 accepted and identifiers introduced.
- [x] Sparse concepts implemented.
- [x] Beliefs, trust, and causal hypotheses implemented.
- [x] Documentation and TODO state updated.
- [x] Workspace verification and graph refresh completed.
