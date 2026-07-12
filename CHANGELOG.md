# Changelog

All notable changes to Ontopolis are documented in this file.

## Format

This project follows a structured changelog format. Each entry includes:

- Phase reference (e.g., Phase 0, Phase 1);
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
