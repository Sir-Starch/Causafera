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

### Phase 6: Ground Truth Events and Causal Provenance

#### CORE / TRACE

- Implemented deterministic Ground Truth event and causal provenance tracking (`TODO-TRACE-001`);
- Added strongly-typed `TraceId` and `EventId` identifiers;
- Implemented structure-of-arrays causal trace store (`CausalTraceStore`) with direct parent/child traversal and proposal keys (`RFC-TRACE-001`).

### Phase 7: Physical Access, Sensory Acquisition, and Bounded Attention Primitives

#### SENSE / COG

- Established physical sensory acquisition and accessibility filtering boundary (`TODO-SENSE-001`, `RFC-PERCEPT-001`);
- Implemented bounded attention mechanisms and `AttentionTargetId` (`TODO-COG-001`).

### Phase 8: Generic Perceptual Feature Extraction

#### PERCEPT

- Implemented generic perceptual feature extraction from sensory signals (`TODO-PERCEPT-001`);
- Extracted generic magnitude/change features while preventing Ground Truth identity exposure.

### Phase 9: Subjective Scene Construction

#### COG

- Implemented identity-free perceptual cue boundary and `SubjectiveScene` reconstruction (`TODO-SCENE-001`, `RFC-SCENE-001`, `RFC-COG-001`);
- Added subjective perceived object tracking (`PerceivedObjectId`) and subjective body schema / self-model architecture (`TODO-SCENE-002`, `TODO-SCENE-003`, `TODO-SCENE-004`).

### Phase 10: Working Context, Prediction, and Cognitive Continuity

#### COG

- Implemented fixed-capacity working memory context separate from episodic storage (`TODO-SCENE-006`);
- Added predictive world model, sparse prediction errors (`TODO-SCENE-005`), similarity-weighted episodic memory reactivation (`TODO-SCENE-007`), agency attribution (`TODO-SCENE-008`), and subjective temporal envelope (`TODO-SCENE-009`).

### Phase 11: Sparse Subjective Concept Formation

#### COG

- Implemented sparse prototype-based concept formation (`ConceptId`) and deterministic prototype revision (`TODO-CONCEPT-001`, `RFC-CONCEPT-001`).

### Phase 12: Beliefs and Subjective Causal Inference

#### COG

- Implemented fixed-capacity belief structures (`BeliefId`), evidence records (`EvidenceId`), belief inertia, subjective source trust, and fallible causal hypotheses (`TODO-COG-002`).

### Phase 13: Language Bootstrap and Communication Architecture

#### LANG

- Implemented historical language bootstrap with seed-deterministic form lineages and abstract phoneme inventory (`TODO-LANG-001`, `RFC-LANG-001`);
- Added physical form communication boundary separating speaker intent, utterance, and listener interpretation (`TODO-LANG-003`).

### Phase 14: Lexical Innovation and Semantic Inference

#### LANG

- Implemented communicative pressure accumulator, deterministic phonotactic coinage, and adoption/semantic drift tracking (`TODO-LANG-002`, `RFC-LANG-002`).

### Phase 15: Practice Representation and Evolution

#### PRACTICES

- Implemented evolvable practice programs (`PracticeId`) with deterministic proposal-only execution and lineage mutation (`TODO-PRACTICE-001`, `RFC-PRACTICE-001`).

### Phase 16: Measurement, Documents, and Epistemic Infrastructure

#### EPI / LANG

- Implemented socially constructed measurement units, fixed-point precision/uncertainty, and calibration ancestry (`TODO-EPI-001`, `RFC-EPI-001`);
- Implemented physical document copying with opaque glyph mark sequences and document lineages (`TODO-LANG-004`).

### Phase 17: Minimal Information-Sensitive Mana

#### MANA

- Implemented chunk-local fixed-point mana field responding to physical recurrence, timing, geometry, and spatial patterns (`TODO-MANA-001`, `RFC-MANA-001`);
- Added 6-neighbor stencil diffusion, decay, saturation, and trace-backed evolution.

### Phase 18: Causal Resolution Field

#### RES

- Implemented dynamic causal resolution field assigning simulation detail based on trace-backed causal relevance (`TODO-RES-001`, `RFC-RES-001`).

### Phase 19: Social Networks and Organizations

#### SOCIAL

- Implemented distributed, trace-backed social state tracking agent relations, roles, communication links, authority grants, property claims, institutional rules, and attested agreements (`TODO-SOCIAL-001`, `TODO-SOCIAL-002`, `RFC-SOCIAL-001`).

### Phase 20: Material Economy and City Infrastructure

#### ECON / CITY

- Implemented physical material flow contracts tracking inventory lots, same-material transfers, transformation ancestry, and labour contributions (`TODO-ECON-001`, `RFC-ECON-001`);
- Implemented physical city infrastructure topology tying parcels, buildings, and network nodes to material provenance (`TODO-CITY-001`, `RFC-CITY-001`).

### Phase 21: Historical Bootstrap

#### HIST

- Implemented deterministic historical bootstrap orchestration with canonical stage DAGs and committed stage receipts (`TODO-HIST-001`, `RFC-HIST-001`).

### Phase 22: Isekai Transfer and Imported Priors

#### ISEKAI

- Implemented cross-world transfer model with transfer plans/receipts and imported priors, strictly keeping subjective knowledge separate from local capability (`TODO-ISEKAI-001`, `TODO-ISEKAI-002`, `RFC-ISEKAI-001`).

### Phase 23: Metaphysical Experiments and Attractors

#### META

- Implemented identity persistence research contracts and stateful mana attractor research contracts (`TODO-META-001`, `TODO-META-002`, `RFC-META-001`, `RFC-META-002`).

### Phase 24: Long-Run Emergence Experiments

#### SIM / GEO

- Completed multiscale world spatial geometry (`TODO-GEO-003`, `RFC-GEO-002`);
- Implemented deterministic headless laboratory harness for long-run control/intervention experiments (`TODO-LAB-001`).

### Phase 25: Explanation Engine Expansion

#### EXPLAIN

- Implemented minimal typed Explanation IR, evidence states, confidence metrics, and localized deterministic rendering (`TODO-EXPLAIN-001`, `TODO-EXPLAIN-002`, `RFC-EXPLAIN-001`).

### Phase 26: Rich Observer UI

#### OBSERVER / UI

- Implemented versioned observer protocol, Protocol Buffer v1 transport (`TODO-OBSERVER-001`, `TODO-PROTO-001`, `TODO-OBSERVER-002`);
- Built Tauri 2 + React desktop observer application providing research-console inspection views (`TODO-UI-001`, `TODO-UI-002`).

### Detailed Development Program

#### CORE / RUNTIME

- Completed the **Actor Material Mana Loop** vertical slice (`plans/actor-material-mana-loop.md`, `TODO-SIM-001`), linking chart-qualified material surfaces, contact actions, physical signals, perception, and mana effects;
- Completed the **Local Mana-Material-Surface Coupling** vertical slice (`plans/local-mana-material-surface-coupling.md`), replacing global mana gates with per-surface local hysteresis gates, transition priors, observer deltas, and explanation claims;
- Refactored `causafera-runtime` into a modular architecture (`INV-042`), extracting actor subsystems, material surface, mana, pattern history, digests, resolution, and bootstrap into dedicated modules;
- Tightened workspace internal visibility by removing unused `pub(crate)` modifiers;
- Implemented deterministic binary persistence snapshot format v1 (`TODO-PERSIST-001`, `RFC-PERSIST-001`).
- Completed the **Bounded Conserved Thermal Storage and Same-Chart Transfer** vertical slice (`plans/conserved-thermal-energy-carrier.md`): added fixed-point `ThermalEnergy` carrier, finite historical-bootstrap `ThermalReservoir`, conservative six-face intra-chunk and same-chart cross-chunk diffusion, exact conservation accounting with per-tick zero-residual verification, atomic batch commit via `CausalTraceStore`, snapshot section `0x000E` v1, observer summary/delta projection, and `THERMAL_CARRIER_CONSERVATION_SCHEMA` Explanation claim; digest schema bumped to v5; thermal systems registered in `Phase::Physics` as global IDs 9 and 10.

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
