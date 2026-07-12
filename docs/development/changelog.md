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
- **COG** — Accepted RFC-COG-001 and RFC-SCENE-001, fixing the minimum architecture and concrete bounded layout for subjective scenes and cognitive continuity.
- **ARCH** — Added invariants INV-027 through INV-035 enforcing the subjective scene boundary.
- **ARCH** — Resequenced roadmap Phases 8–27 to insert Subjective Scene Construction (Phase 9), Working Context / Prediction / Cognitive Continuity (Phase 10), and shift subsequent cognitive phases accordingly.
- **COG** — Added and completed nine implementation TODO items (TODO-SCENE-001 through TODO-SCENE-009); concept formation now has its required scene and active-context dependencies.
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

### Phases 9–10: Subjective Scene and Cognitive Continuity

- **COG** — Added identity-free `PerceptualCue`, deterministic subjective object persistence, fixed-capacity scene reconstruction, subjective body schema, and revisable opaque self-associations.
- **COG** — Replaced the unbounded semantic memory placeholder with a bounded decaying working context and capped similarity/relevance-driven episodic reactivation.
- **COG** — Added sparse generic predictions, explicit numeric prediction errors, learned opaque agency associations, and an eight-frame subjective temporal envelope.
- **ARCH** — Completed TODO-SCENE-001 through TODO-SCENE-009 and marked Phases 9–10 complete; Phase 11 sparse concept formation is next.
- **ONTO** — Kept authoritative IDs, traces, semantic object/situation kinds, emotions, traits, and English labels out of cognitive state.
- **PERF** — Used fixed maxima and integer ranking throughout active cognition without making scale or throughput claims.
- **COG** — Completed Phases 11–12 with bounded sparse concept prototypes, fixed-point belief inertia, subjective source trust, canonical evidence batches, and fallible directed causal hypotheses.
- **RFC** — Accepted and expanded RFC-CONCEPT-001; concepts consume only attended identity-free observations and retain subjective percept support.
- **ARCH** — Completed TODO-CONCEPT-001 and TODO-COG-002; Phase 13 language bootstrap is next.

### Phases 13–14: Language Bootstrap and Lexical Change

- **LANG** — Accepted RFC-LANG-001/002 and replaced string/float placeholders with bounded opaque phonology, deterministic language/lexeme ancestry, and subjective fixed-point lexicons.
- **LANG** — Added a structural intent → physical form → listener interpretation boundary, percept-supported exposure/adoption, repeated lexical pressure, deterministic coinage, and minimal semantic revision.
- **ONTO** — Kept human strings, objective meanings, semantic speech-act enums, and authoritative identities out of language state.
- **ARCH** — Completed TODO-LANG-001 through TODO-LANG-003 and Phases 13–14; Phase 15 practice representation is next.
- **PERF** — Added hard capacities and canonical integer ordering without making unbenchmarked language-scale claims.

### Phases 15–16: Practices and Epistemic Carriers

- **ARCH** — Accepted RFC-PRACTICE-001 and RFC-EPI-001; completed TODO-PRACTICE-001, TODO-EPI-001, TODO-LANG-004, and Phases 15–16. Phase 17 mana is next.
- **CORE** — Replaced string-based practice placeholders with bounded validated control flow, deterministic proposal-only execution, and structural child lineage mutation.
- **EPI** — Replaced floating-point/string measurement placeholders with opaque fixed-point units, bounded calibration ancestry, uncertainty-preserving measurement, and explicit practice provenance.
- **LANG** — Replaced string document media with bounded physical glyph sequences, opaque writing/medium identities, deterministic edit scripts, and document-copy ancestry.
- **ONTO** — Kept named actions, quantity names, unit names, genres, textual meaning, authoritative identities, and hidden true values out of the new state.
- **PERF** — Added hard program, execution, calibration, glyph, and edit budgets without making throughput claims.

### Phase 17: Minimal Information-Sensitive Mana

- **MANA** — Accepted RFC-MANA-001 and completed TODO-MANA-001 with a bounded chunk-local fixed-point scalar field.
- **MANA** — Added canonical physical samples and numeric response to recurrence, regular intervals, synchronization, repeated coordinates, and magnitude, followed by deterministic diffusion, decay, and saturation.
- **CORE** — Kept evolution proposal-only and required one newly committed provenance trace for every accepted changed cell; proposals retain direct sample and neighbouring prior-field causes.
- **ONTO** — Kept words, beliefs, concepts, practice meanings, spell categories, sacredness, attractors, and observer labels outside mana state; opaque fingerprints identify canonical physical structure only.
- **PERF** — Established a bounded dense CPU baseline without scale claims; sparse, multi-resolution, and GPU variants require benchmarks and bit-identical validation.
- **ARCH** — Completed Phase 17; Phase 18 Causal Resolution Field is next and remains a separate foundational plan.

### Phase 18: Causal Resolution Field

- **RES** — Accepted RFC-RES-001 and completed TODO-RES-001 with a bounded fixed-point structure-of-arrays resolution field.
- **RES** — Added directed trace-backed relevance signals on opaque weighted channels, deterministic prior-score decay, saturation, numeric thresholds, and hysteresis.
- **CORE** — Kept evaluation proposal-only; changed entries retain canonical signal/prior-state causes and require exactly one new commit trace.
- **ONTO** — Kept distance non-privileged and excluded trade, migration, politics, religion, observer labels, and other semantic dimensions from authoritative resolution state.
- **PERF** — Established bounded CPU reference contracts without scale claims; full domain aggregation and alternative layouts require later contracts and benchmarks.
- **ARCH** — Completed Phase 18; Phase 19 social networks and organizations is next.

### Phase 19: Distributed Social Foundation

- **SOCIAL** — Accepted RFC-SOCIAL-001 and completed TODO-SOCIAL-001/002 with bounded trace-backed relations, roles, communication links, authority grants, property claims, rule records, practice associations, and attested agreements.
- **ONTO** — Kept organization cognition, semantic relation/role/legal taxonomies, universal rule validity, shared interpretation, and automatic enforcement out of authoritative state.
- **LANG** — Made institutional rules and agreements reference physical documents while preserving separate interpretation, precedent, party, witness, and authority records.
- **PERF** — Added hard capacities, canonical ordering, and binary-search reference validation without making scale claims.
- **ARCH** — Completed Phase 19; Phase 20 material economy and city infrastructure is next.

### Phase 20: Material Economy and City Infrastructure

- **ECON** — Accepted RFC-ECON-001 and completed TODO-ECON-001 with bounded material lots, same-material transfers, transformation ancestry, performed labour, and optional contestable ownership-claim support.
- **CITY** — Accepted RFC-CITY-001 and completed TODO-CITY-001 with spatial parcel references, physical buildings, and generic directed infrastructure networks tied to material provenance.
- **ONTO** — Kept prices, markets, jobs, ownership validity, roads, water/sewage categories, districts, settlements, and cities out of authoritative semantic enums.
- **PERF** — Added hard capacities, canonical typed-ID ordering, binary-search validation, and deterministic topology traversal without making scale claims.
- **ARCH** — Completed Phase 20; Phase 21 historical bootstrap is next.

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
- **RES** - Causal resolution
- **SOCIAL** - Social carriers and institutions
- **ECON** - Material economy carriers
- **CITY** - Urban physical infrastructure
- **ISEKAI** - Isekai
- **META** - Metaphysics
- **EXPLAIN** - Explanation Engine
- **OBSERVER** - Observer layer
- **UI** - User interface
- **PERF** - Performance
- **FIX** - Bug fixes
- **DOCS** - Documentation
