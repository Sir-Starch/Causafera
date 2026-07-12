# RFC-SCENE-001: Bounded Subjective Scene and Cognitive Continuity

**Status:** Accepted

## Summary

Define the concrete Phase 9–10 implementation boundary for reconstructing an agent-local subjective scene and maintaining a small active temporal context without exposing authoritative identity.

## Decision

### Cognitive input boundary

Cognition accepts `PerceptualCue`, not `Feature` or `SensorySample`. A cue contains only:

- agent-local `PerceptId` and `AttentionTargetId`;
- a quantized generic appearance signature;
- relative position;
- fixed-point strength;
- simulation time.

The adapter that constructs cues may inspect extractor bookkeeping only to strip and regroup it. It must not copy `EntityId`, `FeatureId`, `TraceId`, `SensorId`, or an authoritative location into cognitive state.

### Subjective identity and scene

`SceneContinuityState` maintains at most 32 perceived-object hypotheses. Matching uses deterministic integer appearance and relative-position distance plus agent-local attention continuity. Similar cues may merge; discontinuous cues may split. Capacity pressure evicts the oldest/weakest hypothesis by an explicit numeric ordering.

Each reconstructed `SubjectiveScene` contains at most 16 attended objects, a bounded subjective body schema, and bounded activated self-associations. It is transient. Object hypotheses, body schema, and self-model are subjective and may be wrong.

### Active context and memory

`WorkingContext` holds at most eight active items, ranks them by fixed-point activation, and decays unrehearsed items. `EpisodicStore` is a minimal capped cold store. Reactivation uses deterministic partial signature similarity and relevance; at most four episodes become active.

The cold store is not claimed as the eventual long-term-memory persistence format. Its purpose is to enforce the active/cold separation before concept work.

### Prediction, agency, and time

At most eight near-future predictions are active. Due predictions compare a generic expected signature with accessible cues and emit bounded numeric `PredictionError` records. Errors retain only subjective evidence handles.

Agency attribution is a bounded learned numeric association between opaque action and outcome pattern IDs. It is updated by observed proximity; it is neither innate nor guaranteed correct.

`TemporalEnvelope` retains at most eight recent subjective frames. Frames contain subjective object IDs, percept support, and aggregate prediction error, not an autobiographical narrative or authoritative event history.

## Determinism

- All weights are integer parts-per-million.
- Inputs are sorted by typed numeric IDs.
- Every tie has an explicit numeric tie-breaker.
- All collections affecting active cognition have hard maxima.
- Hash iteration, randomness, wall time, locale, English labels, and floating-point thresholds are absent.

## Rejected alternatives

- Passing `Feature` directly to cognition: rejected because it carries authoritative target identity.
- Semantic object/situation enums: rejected because categories must emerge later.
- Keeping all autobiographical memory active: rejected by INV-032 and bounded cognition requirements.
- Full physics prediction per agent: rejected as omniscient and unbounded.
- `TraceId` inside cognition: rejected because causal provenance is external support, not agent knowledge.

## Non-goals

Concepts, beliefs, goals, language, realistic recognition, complete physiology, durable snapshot formats, observer wire schemas, and performance claims.

## Consequences

Phase 11 can consume situated patterns from scenes and working context instead of raw global features. Misidentification, forgetting, surprise, and incorrect agency become representable without semantic shortcuts. A future bridge must construct perceptual cues while preserving the one-way Ground Truth boundary.

## Related documents

- `docs/rfc/RFC-COG-001.md`
- `docs/architecture/cognition-rebaseline.md`
- `docs/architecture/invariants.md`
- `plans/phases-9-10-subjective-continuity.md`

## Decision log

- 2026-07-12: Accepted the fixed-capacity integer layout and identity-free cue boundary for Phases 9–10.
- 2026-07-12: Kept concept activation, semantic self traits, and named prediction kinds out of the implementation.
