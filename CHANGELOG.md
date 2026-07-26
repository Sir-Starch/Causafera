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
- Completed the **Bounded Conserved Thermal Storage and Same-Chart Transfer** vertical slice (`plans/conserved-thermal-energy-carrier.md`): added fixed-point `ThermalEnergy` carrier, finite historical-bootstrap `ThermalReservoir`, conservative six-face intra-chunk and same-chart cross-chunk diffusion, exact conservation accounting with per-tick zero-residual verification, real causal-batch rollback coverage, authoritative current-batch outside-region boundary records, atomic batch commit via `CausalTraceStore`, snapshot section `0x000E` v1, observer summary/delta projection, and `THERMAL_CARRIER_CONSERVATION_SCHEMA` Explanation claim; digest schema bumped to v5; thermal systems registered in `Phase::Physics` as global IDs 9 and 10.
- Fixed four thermal snapshot-import integrity gaps found by independent review: unchecked receipt signed-flux transitions, an unbound latest-batch receipt/current-field-energy binding, unchecked reservoir-budget subtraction, and duplicated boundary-neighbor geometry in `causafera-runtime` now reusing `causafera-domains`' authoritative `ThermalFieldSet::boundary_neighbor_keys`; also closed a follow-up gap where non-latest-batch transfer receipts did not bounds-check `cell_index` against the field. See `TODO-THERMAL-006` for the remaining, out-of-scope aggregate conservation-total cross-validation gap.
- Corrected the mana chunk seam and the diffusion budget (`TODO-MANA-002`). The seam was not the reflecting wall the backlog recorded — `apply_boundary_exchange` has always moved mana across same-chart faces — but it conducted 2.58x the interior rate, transferring a half-difference term on top of an outgoing budget the interior stencil had already distributed in full, while `neighbor_indices` clipped at the chunk edge so a face cell divided its share among five neighbours instead of six. An open face is now counted in the cell's neighbour count and the exchange delivers exactly that one share, read from the pre-diffusion values, which also removes a dependence on face visit order. Separately, diffusion had been destroying up to `count - 1` units per cell per tick by subtracting an undivided outgoing budget against truncated incoming shares. Re-measurement inverts the recorded `chunk_extent` evidence: total mana now rises with the lattice and is flat from extent 12, so the default extent 3 underestimates by 15.8% rather than overestimating by 11%;
- Replaced the mana diffusion stencil with an isotropic one (`TODO-MANA-003`). The six axis neighbours propagated on the L1 ball, so a point source spread as an octahedron and every mana isoline would have been diamond-shaped. The stencil is now eighteen neighbours — six faces at weight 2 and twelve edges at weight 1 — the smallest exact-integer solution of the fourth-order isotropy condition `f == 2e + 8c`, keeping floating point out of the authoritative path and the stencil out of diagonally opposite chunks. Measured by the fourth-moment ratio `<x^4> / 3<x^2 y^2>`, the old stencil reads 1.28 against the new one's 1.00. The boundary exchange generalises with it, walking every directed step that leaves a chunk exactly once instead of pairing three positive faces;
- Made the world seed reach the running simulation (`TODO-RUNTIME-002`, `plans/terrain-carrier-participation.md`). The seed reached terrain generation and terminated there: `PhysicalPatternSystem::execute` filled `pending_samples` only from `MaterialSurfaceCarrierAdapter`, no registered system consumes the scheduler's `RandomStream`, and six seeds produced a byte-identical physical digest, total mana and behaviour after 192 ticks. The terrain carrier now participates as a **standing** structure, presented at the mana field's own lattice: one sample per plan-view column, magnitude equal to the column's mean relief contrast, material discontinuity and roughness, fingerprint derived from its dominant surface material and quantised roughness class. Featureless ground emits nothing — the former constant magnitude floor is removed. Terrain samples reach `pending_samples` but deliberately not `PhysicalPatternHistory` or `physical_events`, both of which retain change: a structure that is merely still there has not happened again, and retaining it was measured to let the recurrence and periodicity channels score the carrier's read cadence, running total mana to twenty-one times the contact-driven baseline and collapsing gate transitions from 32 to 3. Participation is the persisted `RuntimeConfig::terrain_participation` contract (`Standing` by default, `Inert` retaining the recorded prior behaviour); the runtime recipe snapshot section major rises from 4 to 5. Six seeds now give six distinct physical digests, six distinct total manas from 50 416 to 116 189, and three distinct behaviour tuples, while the same seed still reproduces identically. Cost at the default `chunk_extent` 3 is single-digit percent per tick, measured over six runs at 7.0% to 9.4% with one 17.3% outlier; above it the growth is the price of a genuinely populated field — one causal event per changed mana cell — not of the carrier, whose projection is derived once. Two defects were fixed on the way: `TerrainBootstrapStage` hard-coded `field_extent` 3 over the configured `chunk_extent`, and the position function shared by both carriers decomposed a mana cell index as a `CHUNK_SIZE`-wide raster, which was a stride comb for terrain and simply wrong for any material-surface cell index above zero;
- Gave every mana cell change causal ancestry (`TODO-MANA-005`). The cross-chunk boundary exchange attributed a delivered share to the two participants' `last_change` alone, and a cell injected for the first time has no previous change to point at, so the share it handed across a seam arrived with an empty cause list and the runtime committed it as an ordinary mana event with no parents — four such events on seed 7 over 24 ticks, with the terrain carrier standing and inert alike, and none with a single active chunk. The exchange now carries what produced the value that crossed, this tick's injection causes included, and the receiving side contributes its own; `propose_evolution` fails closed with `ManaError::UnattributedChange` on any change that still has no cause, at both the field and the field-set boundary. That guarantee rests on a cell holding mana only because some commit put it there, so a field imported with a non-zero intensity and no trace is now rejected rather than spreading what it cannot attribute. The stated determinism consequence was wrong in one respect: causes enter the history digest and not the physical one, so every world's history digest changes while the physical digest, total mana and every behavioural count are unmoved. Opened `TODO-MANA-006` for an adjacent defect found while measuring — seam delivery does not apply `maximum_intensity`, so a saturated field can finish 3.4% above its own ceiling;
- Decided the mana lattice and kept `chunk_extent` at 3 (`TODO-MANA-004`). The question the decision was written to answer stopped existing when the standing terrain carrier landed: the carrier presents itself at the mana lattice's own resolution, so the extent no longer only samples the field, it sets how finely the field reads the ground. Total mana then runs 82 679, 154 901, 926 121, 2 158 238 and 8 117 028 at extents 3, 4, 6, 8 and 12, and comparing a coarse field against a fine reference measures two things at once. `extent_decision.rs` was rewritten around three questions that do have answers, and all three came back against refining. Fidelity: against the exact reading at extent 32, the terrain's structural variation survives at 1.9% to 18.6%, which is only 2.20x to 1.32x what averaging pure noise would retain, and the ratio *falls* as the lattice refines — the coherent structure is already captured at extent 3 and a finer lattice resolves cell-scale noise, the same finding `TODO-GEO-004` reports from the terrain side. Discrimination: distinct behaviour tuples across six seeds run 3, 3, 1, 2 and 1, so at extent 6 and above every world produces identical gate crossings, transitions, conditions, actions and population, because a finer lattice pushes 8% to 24% of live cells past a gate threshold of 4 096 and the gate latches open in the first ticks. Cost: 5.9x at extent 6 and 29.0x at extent 12, dominated by one committed causal event per changed mana cell per tick. The prior decision of extent 6, with its 98.1% convergence and 94% cost, is void — measured on a field no carrier populated, against a convergence criterion that no longer means anything. Nothing is applied, so no fixtures or replay evidence change. Opened `TODO-MANA-007` for what the measurement identified as the binding constraint: the response and gate constants were calibrated when nothing populated the field;
- Bumped the pinned Rust toolchain from 1.85.0 to 1.97.1 (`rust-toolchain.toml`) to track the newest stable release. The repository's `rust` CI job installs `dtolnay/rust-toolchain` with no explicit `toolchain` input, so it always resolves to the current stable release regardless of `rust-toolchain.toml`; this bump brings local/`rust-toolchain.toml` reproducibility back in line with what CI already runs, rather than pinning CI backward. Fixed the resulting new lints: two `clippy::manual_unwrap_or` occurrences (`causafera-analytics`, `causafera-lab`), one `clippy::useless_conversion`-style explicit `.into_iter()` on a `zip` argument (`causafera-cognition`), and four `clippy::collapsible_if` occurrences rewritten using stable 2024-edition let-chains (`causafera-runtime`).

#### UI

- Rebuilt the observer frontend as a five-area analytical instrument (Observatory, Survey, Flux, Assay, Instrument), replacing the Phase 26 three-view shell and its vanilla stylesheet;
- Introduced a design token system and component library implementing the midnight-cartography direction: chart-surface treatment, engraved typographic hierarchy, coordinate-lock selection, and hatched unsurveyed states;
- Introduced a six-hue signal palette in which each hue is reserved for one simulation quantity, generated in OKLCH and validated for lightness band, chroma floor, protanopia and deuteranopia separation, normal-vision separation, and contrast against the chart surface;
- Replaced the session hook with a session controller over a selector store, adding a feed demand registry so a closed panel produces no observer traffic (`docs/observer/backpressure.md`);
- Added canvas visualisation: a single-axis chart recorder with crosshair probing, a stacked chart profile of active chunks, and the material surface condition ladder with provenance markers;
- Added a capability register presenting every defined observable with its state and maturity, replacing silent omission of unavailable data;
- Added the Instrument exchange log reporting real transport byte counts, durations, and outcomes;
- Added a command palette and keyboard navigation across areas, transport, and the inspector dock;
- Rendered material surface gate deltas, which the protocol previously delivered without a consumer;
- Added a development-only replay channel and a render smoke check driven by captured real protocol bytes (`cargo run -p causafera-observer --example capture_replay`), giving the frontend its first automated verification; both are excluded from production builds and marked in the interface (`INV-039`);
- Recorded the observer projections the frontend is waiting on (`docs/ui/observer-projection-gaps.md`);
- Reworked the visual system into a black outline atlas: an SVG terra incognita chart sheet beneath the application (coastlines, water lining, contour rings, graticule, rhumb lines, soundings, compass rose, and survey-dashed coasts where the survey was never closed), generated from fixed sums of sinusoids so it is identical in every session and cannot be mistaken for data;
- Removed glow, glass, gradient and rounded-corner treatments from the interface chrome; plates are hairline-framed regions of the sheet with registration ticks, charts use dotted grids and engraved hatching instead of gradient washes;
- Reserved hue entirely for measured quantities: the chrome is monochrome ink, so a coloured mark always denotes a simulation quantity. Selection, activity and focus are drawn rather than lit;
- Added the chart instrument: a pannable, zoomable canvas map of one chart's chunk lattice with viewport culling, three levels of detail, spatial selection down to the 32³ cell lattice, and unsurveyed hatching beyond the received extent;
- Added an extensible analytical lens architecture (`apps/observer/src/map`) in which a lens supplies fields, proportional symbols, cell marks, vectors and isolines, and the renderer knows nothing about any domain — connecting a future domain is a catalogue entry, not a renderer change;
- Declared lens availability as part of the contract: observed, partial, preview and awaiting are drawn differently, and an awaiting lens renders the chart as unsurveyed while naming the read model it needs;
- Isolated observer-side constructions in `src/map/preview.ts` (inverse-distance interpolation with marching-squares isolines, neighbour-difference vectors), clipped to the charted extent so a construction never paints knowledge over unsurveyed ground;
- Replaced the Survey area with the Chart area, folding the chart profile and chunk register beneath the map as supporting reads of the same selection;
- Documented the lens contract and its promotion recipe (`docs/ui/map-lenses.md`);
- Rebuilt the meridian bar as one strip of equal-height cells divided by hairlines, with a single standard control shape throughout the chrome, a one-line digest run, and no connection word while the link is simply working;
- Gave both side panels identical behaviour: one standard collapse control that stays visible when collapsed, and a drag handle for width;
- Moved hover explanations into a portal so a panel with clipping can no longer cut one in half;
- Stopped the desktop launcher from disabling compositing and forcing software GL by default, which made the shell markedly slower than the same build in a browser; the narrow XWayland and DMABUF workaround remains, and the heavy profile is available through `CAUSAFERA_SOFTWARE_RENDER=1`;
- Painted the chart sheet once into a bitmap instead of keeping roughly a hundred masked vector paths and a filtered noise layer live beneath a translucent scrolling workspace;
- Bounded repaint to the scrolling regions with `contain: paint`, and stopped republishing map hover readings that had not changed.

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
