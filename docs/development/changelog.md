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

### Architecture

- **COG** — Added architecture rebaseline `docs/architecture/cognition-rebaseline.md` clarifying the subjective scene construction layer between generic perceptual features and subjective concepts/beliefs.
- **COG** — Added RFC-COG-001 proposing the minimum viable representation for the subjective scene and cognitive continuity model.
- **ARCH** — Added invariants INV-027 through INV-035 enforcing the subjective scene boundary.
- **ARCH** — Resequenced roadmap Phases 8–27 to insert Subjective Scene Construction (Phase 9), Working Context / Prediction / Cognitive Continuity (Phase 10), and shift subsequent cognitive phases accordingly.
- **COG** — Added nine new implementation TODO items (TODO-SCENE-001 through TODO-SCENE-009) for subjective scene subsystems, with dependencies that block concept formation and belief implementation until scene dependencies are resolved. TODO-SCENE-001 depends on `RFC-COG-001: Accepted`; it is not itself an RFC.
- **DOCS** — Added specification provenance note to `docs/index.md` documenting how post-initialization architectural discoveries are recorded.

### Phase 0: Project Foundation

#### Documentation

- Revised CLA to support commercial and proprietary outbound licensing while preserving public AGPL-3.0-only/CC BY-SA 4.0 availability, require separately recorded electronic acceptance, and select Netherlands law;
- Documented that external code contributions remain closed until the CLA acceptance workflow is configured;
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

### Phase 5: Biological Foundations

#### Pathogen Contracts

- **BIO** — Accepted RFC-BIO-002 and completed `TODO-BIO-002` with validated fixed-point pathogen properties, canonical lineage ancestry, objective host-interaction profiles, and traced physical exposure records.
- **ONTO** — Kept pathogen types, transmission routes, diseases, and symptoms out of authoritative enums; infection mutation and evolution remain deferred to provenance-aware scheduler phases.
- **PERF** — Added deterministic structure-of-arrays lineage storage without making unbenchmarked epidemic-scale claims.

### Phases 6–8: Causal Perception Foundation

#### Ground Truth provenance

- **CORE** — Accepted RFC-TRACE-001 and completed `TODO-TRACE-001` with stable proposal reduction, opaque event/state schema IDs, property before/after fingerprints, monotonic event/trace allocation, and direct parent/child traversal.
- **ONTO** — Kept semantic event names and domain taxonomies outside authoritative provenance.

#### Physical access and generic extraction

- **ONTO** — Added `ontopolis-perception` and accepted RFC-PERCEPT-001, completing `TODO-SENSE-001` and `TODO-PERCEPT-001` with property-based signal apertures, deterministic accessibility filtering, relative samples, generic magnitude/change features, and flattened causal inputs.
- **COG** — Completed `TODO-COG-001` with fixed-capacity attention over agent-local `AttentionTargetId`; authoritative entity and feature identities cannot enter attention state.
- **ARCH** — Marked Phases 6–8 complete and Phase 9 Subjective Scene Construction next. Feature-to-subjective identity mapping remains mandatory before broader cognition.
- **PERF** — Used flat event/edge/feature batches and fixed attention arrays without making unbenchmarked throughput or scale claims.

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
