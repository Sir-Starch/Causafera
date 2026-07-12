# Mana Topology

Mana is a local information-sensitive physical field. Phase 17 establishes its minimum deterministic topology without claiming final mana physics.

## Authoritative state

`ontopolis-domains::mana::ManaField` stores a chunk coordinate, a cubic extent of at most `CHUNK_SIZE`, row-major fixed-point intensity, the last committed causal trace per cell, and the latest incorporated simulation tick. Human names and classifications are absent.

## Physical inputs

The field accepts bounded `PhysicalPatternSample` batches. Each sample supplies an opaque fingerprint of canonical carrier structure, local coordinate, observation tick, magnitude, stable source ordinal, and causal trace. Integrations may derive fingerprints from acoustic wave structure, repeated geometry, motion, physical glyph arrangements, material state, or other objective carriers. They may not hash words, beliefs, concepts, practice meanings, document genres, or observer labels into a fingerprint.

## Minimal response

Canonical same-fingerprint samples increase response through repeated occurrence, regular temporal intervals, simultaneous occurrence, and repeated placement at distinct coordinates. Magnitude scales local injection. A fixed six-neighbour stencil then applies diffusion and decay, with non-negative saturation.

This permits two physically similar structures to couple even when societies interpret them differently, and physically different structures to couple differently even when agents believe they mean the same thing.

## Provenance and commits

Evolution produces replacement-state proposals and changed-cell records. It does not mutate the source field. Each changed cell carries traces supporting direct pattern injection and neighbouring prior field state. A caller must commit one new provenance trace per changed cell before constructing the next field.

## Geography

Fields are chunk-local causal state, so terrain, geology, hydrology, climate, ecology, and construction can later alter sample production or field parameters. Phase 17 does not invent those couplings. Cross-chunk boundary exchange is also deferred.

RFC-GEO-002 classifies the current cubic field as bounded local Euclidean 3D inside one surface chart. Bare `ChunkCoord` is not a global planetary position. Cross-chart diffusion requires curvature-aware registered transforms. Future density, phase, spectral, or persistence components would add field-state dimensions, not extra spatial dimensions.

## Determinism and performance

All hot arithmetic is fixed-point integer arithmetic; sample and cell traversal is canonical. Field volume and input batches have public hard bounds. The dense CPU implementation makes no scale claim. Sparse and accelerated alternatives require benchmarks and bit-identical validation.

## Deferred phenomena

Field-to-matter effects, interference phase state, hysteresis, long-lived attractors, artifacts, gods/spirits, semantic observer classifications, causal resolution, persistence, and visualization remain future work.

## Related documents

- `docs/rfc/RFC-MANA-001.md`
- `docs/vision/project-thesis.md`
- `docs/architecture/provenance.md`
- `docs/ontology/causal-carriers.md`
- `docs/rfc/RFC-GEO-002.md`
