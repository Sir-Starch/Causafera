# Changelog

All notable changes to Causafera are documented in this file. No formal versioned release has
been published. This is a development changelog for an **Experimental pre-alpha** project.

## Format

This project follows a structured changelog format. Each entry includes:

- Phase reference (e.g., historical Phase 0, Phase 1);
- Category (ARCH, CORE, WORLD, BIO, etc.);
- Change description;
- Impact assessment;
- Related documents or RFCs.

## Unreleased

### Phase 0: Project Foundation

#### Documentation

- Created project vision documents (project-thesis, core-loop, isekai-targets, uniqueness);
- Created ontology documentation (world-ontology, domain-coverage-matrix, causal-carriers, lifecycle-audit, cross-domain-interactions, primitive-vs-emergent, unresolved-assumptions);
- Created architecture documentation (invariants, performance, determinism, data-oriented, observer, protocol);
- Created world documentation (geography-philosophy, spatial-hierarchy, coordinates, terrain, geology, hydrology, climate, ecology, settlements, mana-topology, world-generation-provenance, historical-bootstrap);
- Created biology documentation (architecture, morphology, physiology, development, heredity, reproduction, aging, death, pathogens, populations, demography);
- Created cognition documentation (attention, memory, salience, prediction, belief-inertia, goals, habits, trust, strategic-communication);
- Created language documentation (architecture, semantic-layer, lexicon, phonology, morphology, grammar, communication, lexical-innovation, semantic-drift, language-change, translation, writing-systems, language-bootstrap);
- Created epistemics documentation (architecture, knowledge-types, measurement, metrology, instruments, experiments, replication, science, writing, documents, document-lineage);
- Created isekai documentation (architecture, transfer-types, foreign-memory, imported-priors, translation-impact, historical-arrivals, causal-contamination);
- Created metaphysics documentation (identity, death-and-persistence, cross-world-continuity, attractors, gods-and-spirits, artifacts);
- Created simulation documentation (perceptual-features, emergent-concepts, technology-and-invention, maintenance);
- Created city documentation (parcels, buildings, streets, infrastructure-networks, maintenance, urban-growth, fire);
- Created society documentation (law, contracts, bureaucracy, records);
- Created explanation documentation (architecture, analytical-ontology, classification, explanation-ir, confidence, causal-summaries, glossing, deterministic-rendering, localization, optional-llm-surface);
- Created observer documentation (architecture, protocol, snapshots, backpressure);
- Created UI documentation (views, map-perspectives, language-inspection);
- Created analytics documentation (phenomenon-evaluation);
- Created performance documentation (philosophy, metrics, benchmarks);
- Created development documentation (codebase-memory, contributing, changelog);
- Created glossary and bibliography;
- Created root CONTRIBUTING.md and CHANGELOG.md.

#### Architecture

- Defined crate structure and dependency principles;
- Established hard invariants;
- Defined deterministic execution requirements;
- Specified data-oriented storage approach;
- Defined observer protocol boundaries.

#### RFCs

- Created placeholder RFCs for foundational systems.

#### ADRs

- Created initial Architecture Decision Records.

### Phase 1: Deterministic Simulation Kernel

#### CORE

- Implemented deterministic scheduler with phase-aware execution (`TODO-CORE-001`);
- Added [`Phase`] enum with fixed execution order and [`PhaseId`] identifier;
- Added [`System`] trait and per-phase registration API;
- Implemented [`RandomStream`] with ChaCha12 RNG keyed by `(world_seed, time, phase, system_id)`;
- Scheduler ticks deterministically in strict mode; identical config + systems → identical state;
- Updated [`Runtime`] to accept [`DeterministicConfig`].

#### WORLD

- Implemented coordinate primitives (`TODO-COORD-001`);
- Added [`WorldCoord`], [`ChunkCoord`], [`LocalCoord`] with integer math;
- Added bidirectional `WorldCoord` ↔ `(ChunkCoord, LocalCoord)` conversions;
- Added flat index for dense chunk array layout.

#### Testing

- Added 10 unit tests for scheduler determinism, phase ordering, and random stream independence;
- Added 6 integration tests for scheduler replay, seed divergence, and multi-system streams;
- Added 4 unit tests for coordinate roundtrips and chunk origins;
- Added 8 integration tests for coordinate conversions, distances, and edge cases;
- All tests pass under `--workspace --all-targets`.

#### Dependencies

- Added `rand` to workspace dependencies for deterministic RNG.

### Phase 2: Ontology Primitives and Generic Feature Representation

#### ONTO

- Completed primitive vs emergent inventory (`TODO-ONTO-001`);
- Updated `docs/ontology/primitive-vs-emergent.md` with structured Phase 1 and Phase 2 primitive tables;
- Updated `docs/ontology/domain-coverage-matrix.md` — Space, Time, Matter, Energy, Pattern marked Completed;
- Accepted `RFC-ONTO-001` with detailed primitive ontology and generic feature representation design.

#### CORE

- Implemented generic perceptual feature types in `causafera-types/src/features.rs`;
- Added `FeatureRelation` (Change, Magnitude, Direction, Variance, Periodicity, Synchrony, Recurrence, Duration, SpatialRelation, TemporalRelation, CoOccurrence, StructuralSimilarity, RelativeDifference, SequenceSimilarity);
- Added `FeatureValue` (Scalar, Direction, FrequencyBand, MagnitudeBand);
- Added `Persistence` (Fleeting, Brief, Moderate, Persistent, High);
- Added `Feature` struct tying relation, value, persistence, target entity, and ID.

#### WORLD

- Implemented primitive physical state types in `causafera-types/src/physics.rs`;
- Added `Temperature` (Kelvin, with Celsius conversion);
- Added `Orientation` (yaw, pitch, roll);
- Added `Velocity`, `AngularVelocity`, and `Motion`;
- Added `Material` (density, thermal conductivity, specific heat, hardness, porosity, composition by SubstanceId);
- No semantic material names introduced; all properties are measurable physical quantities.

#### Types

- Added `EntityId` and `SubstanceId` typed IDs to `causafera-types/src/ids.rs`;
- Added `serde_json` dev-dependency for serde roundtrip tests.

#### Testing

- Added 5 unit tests for feature type serde roundtrips and Direction3D math;
- Added 5 unit tests for physics type creation, conversion, serde roundtrips, and Material composition;
- All 39 workspace tests pass (10 core unit + 6 core integration + 15 types unit + 8 types integration).

### Phase 3: Spatial World Skeleton

#### WORLD

- Completed the authoritative spatial containment skeleton (`TODO-WORLD-001`);
- Added `SpatialLevel` for the exact World → Landmass → Basin → Region → Territory → Chunk → Parcel → Structure → Interior Space chain;
- Added a validated `SpatialHierarchyBuilder` that rejects skipped or invalid containment transitions;
- Added immutable `SpatialHierarchy` storage with dense nodes and contiguous child adjacency;
- Added constant-index parent lookup and direct child-slice traversal without per-node allocation;
- Added level-checked `PlaceId` ↔ `ChunkId` conversion;
- Replaced the placeholder world root with a seed-explicit `World` wrapper over the finalized hierarchy;
- Retained the world seed as minimal construction provenance without generating fake geography or history.

#### Testing

- Added 8 unit tests covering the complete hierarchy, parent/child traversal, interleaved construction order, transition validation, unknown parents, deterministic replay, chunk identity conversion, and world wrapping;
- All 47 workspace tests pass.

### Phase 4: Minimal Causal Geography

#### GEO

- Completed deterministic terrain generation contracts (`TODO-GEO-001`);
- Added fixed-point `ElevationMm` and `RoughnessMm` physical values plus property-linked `MaterialId` surface identity;
- Added dense `TerrainChunk` structure-of-arrays storage with exact surface-cell count validation and row-major access;
- Added compact per-chunk `TerrainGenerationProvenance` containing the world seed, generation trace, generator and parameter fingerprints, and ordered causal input traces;
- Added a batch-first `TerrainGenerator` trait and validated generation boundary that rejects output count, order, chunk identity, and provenance mismatches;
- Accepted `RFC-GEO-001` with the minimal terrain, determinism, provenance, and primitive/emergent boundaries;
- Did not add a terrain synthesis algorithm, geology, hydrology, climate, ecology, semantic material categories, or fictional geography.

#### Testing

- Added 7 unit tests covering dense field validation, row-major indexing, provenance, batch order and identity, deterministic replay, reordered output, and changed provenance.
- All 54 workspace tests pass.

### Phase 5: Biological Structural Model

#### BIO

- Completed body-segment structural primitives (`TODO-BIO-001`);
- Added fixed-point `SegmentLengthMm`, property-based `Joint` angular bounds, and complete `BodySegment` value views;
- Added validated `BodyStructure` structure-of-arrays storage with canonical parent-before-child topology;
- Enforced unique typed IDs, exactly one root, parent/joint consistency, positive length, finite orientation, ordered joint limits, and current orientation within limits;
- Accepted `RFC-BIO-001` with the Phase 5 causal, deterministic, performance, observer, and primitive/emergent boundaries;
- Did not add named anatomy, semantic segment or joint types, physiology, growth, injury, movement, species, social categories, or demo organisms.

#### Testing

- Added 10 unit tests covering canonical access, field validation, typed identity, topology, root/joint consistency, physical numeric constraints, joint ranges, and deterministic reconstruction.
- All 64 workspace tests pass.

## Categories

- **ARCH** - Architecture
- **ONTO** - Ontology
- **CORE** - Core simulation
- **WORLD** - World generation
- **BIO** - Biology
- **COG** - Cognition
- **LANG** - Language
- **EPI** - Epistemics
- **MANA** - Mana
- **ISEKAI** - Isekai
- **META** - Metaphysics
- **EXPLAIN** - Explanation Engine
- **OBSERVER** - Observer layer
- **UI** - User interface
- **PERF** - Performance
- **FIX** - Bug fixes
- **DOCS** - Documentation
