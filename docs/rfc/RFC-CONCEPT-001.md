# RFC-CONCEPT-001: Sparse Subjective Concept Formation

**Status:** Accepted

## Summary

Agents construct bounded subjective prototypes from explicitly attended, identity-free observations. Concepts are agent-local hypotheses that may disagree across agents and need not match Ground Truth categories.

## Motivation

Continuous clustering of all world features for every agent is both architecturally omniscient and computationally unsuitable. Phase 11 needs a minimal learning contract that consumes the subjective scene boundary established in Phases 9–10 and preserves sparse, fallible cognition.

## Decision

### Input boundary

`ConceptObservation` contains only:

- a quantized generic appearance signature;
- numeric salience;
- numeric predictive utility;
- a supporting `PerceptId`.

The caller is responsible for selecting attended observations. The concept store does not scan Ground Truth, extractor features, entities, places, bodies, or global history.

### Representation

A `SubjectiveConcept` has an opaque agent-local `ConceptId`, prototype signature, confidence, predictive utility, recent activation, exemplar count, supporting percept, and activation time. It contains no category name, English label, semantic enum, or authoritative identifier.

### Sparse formation and revision

At most 32 observations enter one update and at most 32 concepts are retained in the minimal store. Observations are canonicalized by `PerceptId`. A matching prototype is revised through integer running means. An unmatched observation forms a concept only when its salience crosses the configured formation threshold.

At most eight currently active concepts are exposed, ranked by activation and then `ConceptId`. Activation decays by simulation time. Larger stores, splitting, merging, consolidation, social transmission, and durable indexing remain future work.

### Belief boundary

Phase 12 may reference `ConceptId`, but a concept is not itself a truth claim. Beliefs retain separate confidence, inertia, evidence summaries, and subjective source trust. A belief cannot query whether its subject matches Ground Truth.

## Determinism

All values use integer/fixed-point arithmetic. Inputs are canonicalized, matching has explicit ID tie-breakers, identifiers are monotonic and agent-local, and no hash iteration or random source participates.

## Capacity and performance

All active collections have fixed maxima. The implementation performs bounded linear scans. No scale or throughput claim is made; larger limits require reproducible benchmarks.

## Provenance and explanation

Concepts retain the most recent supporting `PerceptId`. This is subjective evidence ancestry, not direct Ground Truth provenance. Observer/explanation systems may later expose numeric analytics but cannot feed classifications back into cognition.

## Rejected alternatives

- Developer-defined category enums: rejected as semantic shortcuts.
- English labels or embeddings as authoritative meaning: rejected by locale and ontology invariants.
- Continuous clustering over all features: rejected because formation must be attention-driven and sparse.
- `EntityId`-keyed prototypes: rejected because agents do not know authoritative identity.
- Floating-point similarity thresholds: rejected for canonical deterministic state.

## Consequences

Phase 13 language can associate learned signal patterns with subjective concepts without making lexemes authoritative categories. The minimal representation deliberately cannot claim realistic human concept learning.

## Resolved Questions

- Attention mechanism: explicit attended observations supplied after subjective scene construction.
- Concept representation: bounded quantized prototype plus numeric activation, utility, confidence, and exemplar count.
- Similarity metric: deterministic Manhattan distance over the generic four-component signature with a configured tolerance.
