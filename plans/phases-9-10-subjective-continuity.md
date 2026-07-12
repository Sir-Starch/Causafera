# Phases 9–10 Subjective Scene and Cognitive Continuity

## Goal

Implement the adjacent Phase 9–10 cognitive foundation as one bounded deterministic pipeline: identity-free perceptual cues, subjective object persistence and scene reconstruction, subjective body/self state, working context, sparse prediction/error, episodic reactivation, agency attribution, and a short temporal envelope.

## Context

Phases 7–8 stop authoritative identity at the extraction boundary and provide bounded attention over subjective targets. Concept formation cannot safely begin until cognition has a coherent transient scene and an explicitly bounded active context. RFC-COG-001 establishes the architectural direction but deliberately leaves concrete layouts unresolved.

## Relevant invariants

- INV-001, INV-002, INV-027 through INV-031: cognition receives neither Ground Truth identity nor inaccessible detail.
- INV-028: perceived identity is a fallible hypothesis.
- INV-032 through INV-034: active context, self-model, and body schema remain structurally distinct from authoritative and cold state.
- INV-035: prediction error is an explicit bounded signal.
- INV-006, INV-017, INV-018: no English simulation meaning; bounded layouts and no unbenchmarked scale claims.

## Ontology domains affected

- Cognition: subjective identity, scenes, active context, prediction, memory activation, agency, and temporal continuity.
- Perception boundary: generic extracted features are converted into identity-free perceptual cues before cognition.

## Causal carriers affected

`PerceptId` is the subjective evidence handle. Cognitive structures retain percept or subjective-memory support, never `TraceId`, `FeatureId`, `EntityId`, `BodySegmentId`, or `PlaceId`.

## Relevant documents

- `docs/architecture/invariants.md`
- `docs/architecture/cognition-rebaseline.md`
- `docs/cognition/attention.md`
- `docs/cognition/memory.md`
- `docs/cognition/prediction.md`
- `docs/simulation/perceptual-features.md`
- `docs/rfc/RFC-COG-001.md`

## Current state

Attention is bounded and identity-safe, but no scene exists. The memory module is an unbounded `HashMap<ConceptId, f32>` placeholder. Phase 8 `Feature` records still contain authoritative extractor bookkeeping and therefore cannot enter cognition directly.

## Proposed architecture

1. Define a compact `PerceptualCue` boundary containing only subjective IDs, quantized appearance/location, time, and numeric strengths.
2. Maintain a bounded `SceneContinuityState` that matches cues to agent-local perceived-object hypotheses using deterministic appearance/location similarity. Unmatched cues allocate monotonic subjective IDs; stale objects remain fallible persistence hypotheses.
3. Reconstruct a fixed-capacity `SubjectiveScene` each update from attended cues plus bounded body-schema and self-model activations.
4. Replace the memory placeholder with a fixed-capacity working context and bounded episodic store/reactivation contract.
5. Add fixed-capacity predictions, numeric prediction errors, agency associations, and recent-scene temporal frames.
6. Keep all algorithms integer/fixed-point, canonical-order, and free of semantic enums.

## Primitive vs emergent review

Opaque subjective IDs, quantized signatures, relative coordinates, confidence/activation weights, time, bounded histories, and generic association strengths are computational primitives. Object kinds, situations, emotions, abilities, social identities, event names, and meanings are not introduced.

## Non-goals

- Concept formation, beliefs, goals, personality, emotion taxonomies, language, or decision policy.
- Perfect object tracking, complete physiology, autobiographical narrative, or global per-agent physics.
- Observer protocol or persistence schemas.
- Performance/scale claims.

## Implementation stages

1. Accept a concrete implementation RFC and introduce required subjective IDs.
2. Implement cue boundary, persistence tracker, body/self state, and scene reconstruction.
3. Implement working context and episodic reactivation.
4. Implement sparse prediction/error, agency attribution, and temporal continuity.
5. Update subsystem docs, TODOs, roadmap, coverage matrix, changelog, and plan state.
6. Format, test, lint, validate the diff, and refresh the code graph.

## Verification

Unit tests cover input-order independence, absence of authoritative IDs, continuity, misidentification/splitting, capacity bounds, decay, partial episodic reactivation, prediction error, agency updates, and temporal eviction. Run workspace fmt, tests, clippy with warnings denied, and `git diff --check`.

## Benchmark plan

No performance claim. Future benchmarks must measure scene update cost at declared cue/object bounds, memory matching cost at declared episode bounds, bytes per active agent, and prediction/continuity update cost.

## Determinism impact

All ranking uses integer scores and explicit numeric tie-breakers. Inputs are canonicalized. No hash iteration, random source, locale, wall clock, pointer identity, or floating-point threshold participates.

## Memory impact

All active per-agent collections have compile-time maxima and use fixed arrays. Episodic storage is also capped for this minimal implementation. No global scan or unbounded cognitive map is introduced.

## Observer impact

No wire change. Read-only scene analytics remain future work.

## Explanation impact

Subjective evidence handles and numeric confidence/errors can later explain attention, surprise, forgetting, and misidentification. No narrative or analytical classifier is added.

## Persistence impact

In-memory contracts only. Snapshot/version integration remains future work.

## Cross-domain effects

Phase 11 concepts may consume scene/working-context patterns rather than raw features. Biology may later feed internal cues through the same identity-free boundary. Attention may consume prediction-error salience without learning Ground Truth ancestry.

## Risks

- Simple signature matching is intentionally fallible and is not a claim of realistic object recognition.
- Bounded linear episodic matching needs benchmarks before larger capacities or indexing are selected.
- The perception-to-cue adapter must remain external bookkeeping; cognition must never recover extractor target IDs.

## Documentation changes

Add an accepted concrete scene/continuity RFC and update cognition architecture, memory, prediction, perceptual feature, ontology coverage, roadmap, TODO backlog, index, and changelog.

## TODO changes

Complete TODO-SCENE-001 through TODO-SCENE-009. Do not start TODO-CONCEPT-001.

## Decision log

- 2026-07-12: Batch Phases 9–10 because continuity, working context, and prediction are required inputs to a useful scene and share the same bounded active-state constraints.
- 2026-07-12: Defer Phase 11 because learnable concepts require a separate accepted RFC and persistent prototype lifecycle.

## Progress

- [x] Required architecture, ontology, cognition, ADR, RFC, roadmap, and current implementation context reviewed.
- [x] Concrete implementation RFC accepted.
- [x] Phase 9 scene and subjective persistence implemented.
- [x] Phase 10 active context and cognitive continuity implemented.
- [x] Documentation and TODO state updated.
- [x] Workspace verification and graph refresh completed.
