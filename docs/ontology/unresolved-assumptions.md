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

The exact field model for information-sensitive mana remains open. Candidate measurable properties include frequency, phase, periodicity, synchronization, spatial symmetry, sequence recurrence, pattern density, and persistence. Planned mechanisms include local field state, resonance, interference, decay, saturation, hysteresis, and diffusion-like behaviour.

No serious mana implementation should proceed before `RFC-MANA-001: Minimal Information-Sensitive Field Model` is accepted.

## Causal Resolution

The precise aggregation and resolution rules for the Causal Resolution Field remain under research. Candidate relevance dimensions include physical proximity, trade connectivity, migration flow, social connectivity, information flow, political influence, material dependency, mana coupling, historical relevance, and observer research focus.

See `RFC-RES-001: Causal Resolution and Aggregation`.

## Language Bootstrap

The exact mechanism for generating initial languages from pre-simulation historical bootstrap remains unspecified. The historical bootstrap may use lower-resolution cultural simulation and constrained causal synthesis. The resulting lexicon contains internal IDs and generated forms, not manually authored dictionaries.

See `RFC-LANG-001: Historical Language Bootstrap`.

## Concept Formation

The sparse subjective concept formation algorithm remains a research area. Concepts must be attention-driven and sparse. Do not continuously cluster all world features for every agent.

See `RFC-CONCEPT-001: Sparse Subjective Concept Formation`.

## Practice Representation

The evolvable practice representation format remains under design. A practice must support ordered operations, conditions, branches, timing, repetition, materials, objects, actor roles, locations, synchronization, and tolerances.

See `RFC-PRACTICE-001: Evolvable Practice Representation`.

## Documented Uncertainty

When a system depends on an unresolved assumption, the code and documentation must explicitly state:

- what assumption is being made
- what RFC or research task addresses it
- what would need to change if the assumption is rejected
- why the current placeholder is sufficient for the current phase

Never silently hardcode a research hypothesis as settled architecture.
