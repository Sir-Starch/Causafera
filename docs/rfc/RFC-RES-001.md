# RFC-RES-001: Causal Resolution and State Aggregation

**Status:** Accepted

## Summary

Simulation detail is selected from bounded trace-backed causal relevance, not inferred from physical distance alone. Phase 18 establishes the resolution decision field; domain-specific aggregation remains later work.

## Motivation

A distant site whose practices, documents, material flows, or mana patterns affect many regions may require more detail than a nearby causally isolated chunk. Distance-only level-of-detail would erase long-range causal structures and violate INV-010.

## Authoritative state

`ontopolis-resolution::ResolutionField` is a bounded structure-of-arrays store ordered by authoritative `ChunkId`. Each entry contains fixed-point relevance, a numeric detail level, and the `TraceId` of its last committed change. Levels are computational ordinals, not semantic geographic classifications. Chunk identity remains Ground Truth bookkeeping and is never agent cognition.

## Relevance carriers

`CausalRelevanceSignal` is directed from source chunk to target chunk and contains an opaque `ResolutionChannelId`, fixed-point strength, causal trace, and producer ordinal. The reducer has no enum for trade, migration, politics, religion, observer interest, or mana importance. Domain adapters may register opaque channels and emit signals only from real causal state.

Physical distance can affect an adapter's signal strength, but is neither mandatory nor privileged. A remote strong signal can produce more detail than a nearby weak signal.

## Policy and transitions

A validated `ResolutionPolicy` contains bounded channel weights, relevance saturation, prior-score retention, strictly increasing thresholds, and hysteresis. Evaluation canonicalizes signals, rejects duplicates and unknown identities, reduces fixed-point contributions by target, decays prior relevance, saturates, selects a numeric level, and emits changed entries with sorted direct and prior-state traces.

Evaluation is pure and proposal-only. Commit requires exactly one new trace for every changed entry and installs a replacement field. Scheduler/event-store integration remains future work.

## Aggregation boundary

The field says how much causal detail a chunk should receive; it does not define how terrain, biology, populations, language, mana, or future organizations aggregate. Each domain needs conservation, provenance, promotion, and demotion contracts before consuming levels. Phase 18 does not destroy detailed state or synthesize individuals from aggregates.

RFC-GEO-002 further separates resolution from geometry: a level may trigger future promotion from a 2.5D surface/layer representation to bounded local volumetric 3D, but it cannot change topology, curvature, physical distance, or chart adjacency. Existing bare chunk keys are local-chart identities pending a chart-qualified migration.

## Determinism and performance

The reducer uses no randomness, floats, strings, locale, system time, hash iteration, or pointer ordering. Entries and policies are canonically sorted; arithmetic is integer, saturating, and bounded. Entries, channels, and signal batches have hard caps. This is a CPU reference baseline with no scale claim.

## Provenance, observer, and explanation

Every changed entry retains direct supporting traces plus the prior entry trace, and commit supplies a new transition trace. A future read-only projection may expose numeric scores, levels, and provenance. Explanation may gloss registered channels outside authoritative state, but cannot invent meaning or mutate resolution.

## Decisions

- **Accepted:** bounded fixed-point directed relevance field with opaque channels.
- **Accepted:** numeric detail ordinals with deterministic threshold hysteresis.
- **Accepted:** pure proposal and traced replacement commit.
- **Deferred:** domain aggregation, promotion/demotion, scheduler cadence, persistence, observer protocol, carrier adapters, and acceleration.

## Unresolved Questions

- Appropriate domain adapter formulas and evaluation cadence.
- Conservation rules for each aggregated domain.
- Whether large worlds need hierarchical or sparse relevance propagation.
- How observer research focus is admitted without UI locale or rendering state affecting canonical history.
