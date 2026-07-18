# RFC-MANA-001: Minimal Information-Sensitive Field Model

**Status:** Accepted

## Summary

Mana is a local fixed-point physical field which responds to measurable recurrence, regular timing, synchronization, and repeated spatial structure. It never receives words, concepts, beliefs, practice identities, social categories, or observer labels.

## Motivation

Causafera needs a causal substrate from which magic-like effects may eventually emerge without implementing “belief changes reality”. The previous `Vec<f32>` placeholder had no spatial contract, deterministic evolution rule, provenance boundary, or semantic exclusion.

## Accepted Phase 17 model

Each `ManaField` owns:

- opaque field identity and authoritative chunk coordinate;
- a validated cubic extent no larger than one chunk;
- row-major fixed-point scalar intensity;
- the last committed trace affecting each cell;
- the simulation time through which inputs have been incorporated.

`PhysicalPatternSample` contains only an opaque fingerprint of canonical physical structure, chunk-local position, observation tick, non-negative magnitude, source ordinal, and causal trace. A fingerprint must be derived from physical or informational carrier structure. It must not be derived from an English label, concept, belief, inferred meaning, spell name, practice identity, document genre, or observer classification.

## Structural response

Samples are canonically sorted and grouped by physical fingerprint. The model calculates integer response from:

- recurrence: additional occurrences of the same fingerprint;
- periodicity: equal non-zero intervals across at least three occupied ticks;
- synchronization: additional same-fingerprint samples at the same tick;
- spatial repetition: additional distinct coordinates occupied by the same fingerprint;
- local magnitude.

These quantities describe carrier structure, not meaning. The response is injected into sampled cells, then a deterministic six-neighbour stencil applies configured diffusion and decay. All values saturate at a configured maximum.

## Mutation and provenance boundary

Evolution is pure:

```text
READ FIELD + PHYSICAL SAMPLES
→ CANONICALIZE
→ ANALYZE STRUCTURE
→ PROPOSE CELL CHANGES
→ COMMIT TRACED REPLACEMENT FIELD
```

`propose_evolution` cannot mutate the source field. Every changed-cell record includes canonical input causes from directly relevant samples and prior neighbouring field changes. `commit` requires exactly one newly assigned `TraceId` per changed cell. Scheduler integration is responsible for committing corresponding Ground Truth events before accepting the replacement field.

## Determinism

The model uses fixed-point integer arithmetic, stable row-major traversal, ordered maps/sets, canonical sample sorting, and saturating operations. It uses no RNG, floats, hash iteration, locale, system clock, or pointer identity. Reordering an equivalent sample batch cannot change the proposal.

## Bounds and performance

Field volume is capped at `CHUNK_SIZE³` and an evolution batch at `MAX_MANA_SAMPLES`. The initial implementation is dense and CPU-only. It makes no scale or throughput claim. Sparse storage, multi-resolution fields, and GPU acceleration require representative benchmarks and must preserve bit-identical canonical results.

## Primitive vs emergent

Primitive in Phase 17: field intensity, coordinates, time, physical fingerprints, numeric structural counts, local diffusion, decay, saturation, and causal ancestry.

Not primitive: mana types, spells, schools, rituals, sacredness, enchantment, skills, levels, gods, spirits, artifacts, attractors, meanings, or observer-facing classifications.

## Deferred work

- coupling to concrete acoustic, geometric, biological, material, glyph, and practice-emission producers;
- field-to-physical-effect proposals;
- empirical parameter calibration;
- sparse, multi-resolution, or accelerated layouts;
- persistence and observer projections;
- stateful attractors and metaphysical hypotheses;
- the Phase 18 Causal Resolution Field.

## Decision log

- **Accepted:** fixed-point scalar field as the minimum representation.
- **Accepted:** physical structure is represented by opaque fingerprints plus explicit numeric space/time samples.
- **Accepted:** proposal-only evolution with mandatory commit traces.
- **Rejected:** semantic pattern categories or direct practice/document/belief coupling.
- **Deferred:** vector fields, interference phase state, hysteresis, attractors, effects, GPU execution, and causal resolution.
