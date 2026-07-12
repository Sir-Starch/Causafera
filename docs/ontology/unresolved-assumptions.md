# Unresolved Assumptions

Ontopolis contains deliberate research hypotheses and explicitly unresolved questions. This document records them so they are not accidentally hardcoded as decided architecture.

## Metaphysics

### Identity and persistence

The physical basis of identity persistence, death, reincarnation, ghosts, and cross-world memory remains unresolved. Do not use a primitive `Soul` object without an accepted RFC.

See `RFC-META-001: Identity and Post-Biological Pattern Persistence`.

### Gods and spirits

Gods and spirits are target emergent phenomena. The hypothesis is that persistent distributed information and mana structures may form stable stateful attractors. A religious system produces repeated names, symbols, synchronized practices, architecture, calendars, and recurring behavioural structures. These patterns may create a stable mana attractor which eventually exhibits state persistence and responsive behaviour.

This is a research hypothesis. Do not implement it as decided architecture.

See `RFC-META-002: Stateful Mana Attractors`.

### Artifacts

Artifact formation is a research target. The candidate process involves material objects, persistent repeated use, stable physical patterns, local mana coupling, and historical persistence. A bell used at the same time for centuries may develop persistent coupling. A currency token repeatedly exchanged through millions of social transactions may develop unusual effects.

No `EnchantItem` action exists in the engine.

## Isekai Transfer

Cross-world transfer must be a physical or metaphysical process. Possible transfer types include full physical transfer, identity-pattern transfer, partial memory transfer, reincarnation-like binding, informational echo, artifact transfer, and overlapping identity patterns.

Do not select a final metaphysical model during Phase 0.

See `RFC-ISEKAI-001: Cross-World Transfer Model`.

## Mana

RFC-MANA-001 now settles the Phase 17 minimum: a bounded fixed-point scalar field responds to opaque physical fingerprints through recurrence, regular intervals, synchronization, repeated coordinates, magnitude, diffusion, decay, and saturation. Evolution is proposal-only and every committed changed cell requires causal provenance.

The final field physics remain open. Vector state, explicit phase/interference, cross-chunk exchange, hysteresis, field-to-matter effects, concrete carrier adapters, sparse/multi-resolution layouts, acceleration, and empirical parameter selection are deferred. Stateful attractors remain a separate metaphysical research hypothesis.

See `RFC-MANA-001: Minimal Information-Sensitive Field Model`.

## Causal Resolution

RFC-RES-001 now settles the Phase 18 decision-field minimum: bounded fixed-point relevance is reduced from directed, trace-backed signals on opaque weighted channels; deterministic decay, saturation, thresholds, and hysteresis select numeric detail ordinals through proposal/commit transitions. Distance is only one possible adapter input and is not privileged by the reducer.

Domain-specific aggregation remains under research. Terrain, biology, populations, language, mana, society, and economy still need explicit conservation, promotion, demotion, and provenance rules. Adapter formulas, evaluation cadence, hierarchical propagation, persistence, and carefully isolated observer-focus inputs are also deferred.

See `RFC-RES-001: Causal Resolution and Aggregation`.

## Language Bootstrap

The exact mechanism for generating initial languages from pre-simulation historical bootstrap remains unspecified. The historical bootstrap may use lower-resolution cultural simulation and constrained causal synthesis. The resulting lexicon contains internal IDs and generated forms, not manually authored dictionaries.

See `RFC-LANG-001: Historical Language Bootstrap`.

## Concept Formation

The sparse subjective concept formation algorithm remains a research area. Concepts must be attention-driven and sparse. Do not continuously cluster all world features for every agent.

See `RFC-CONCEPT-001: Sparse Subjective Concept Formation`.

## Practice Representation

RFC-PRACTICE-001 now settles the Phase 15 structural core: bounded ordered operations, subjective numeric conditions, branches, timing, repetition, tolerances, proposal-only execution, and lineage mutation. Physical material/object bindings, actor roles, locations, synchronization across agents, and transmission fidelity remain unresolved future extensions.

See `RFC-PRACTICE-001: Evolvable Practice Representation`.

## Documented Uncertainty

When a system depends on an unresolved assumption, the code and documentation must explicitly state:

- what assumption is being made
- what RFC or research task addresses it
- what would need to change if the assumption is rejected
- why the current placeholder is sufficient for the current phase

Never silently hardcode a research hypothesis as settled architecture.
