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
- Projected the terrain and mana lattices to the observer and gave the chart a second dimension (`TODO-OBS-001`, `plans/observer-field-raster-map.md`). `Runtime::observer_world_snapshot` reduced the terrain carrier's 1 024 elevations, materials and roughness values to a minimum, a maximum and a mean before the observer saw any of it, so one flat tint per chunk was the honest drawing of everything the map had. An additive `FieldRaster` query kind now projects one chunk of one field at a time: terrain elevation with roughness as a second band, at the carrier's own 32 x 32 lattice or a block-mean reduction of it, and the mana volume whole at its configured extent with the trace that last changed each cell. Values travel as packed ZigZag varints of successive differences, measured at 3 369 bytes per chunk for elevation and roughness against 8 192 raw and 181 bytes for the mana volume. The runtime performs one reduction, the block mean, and deliberately not the plan-view reduction of the mana volume: a column sum and a column maximum answer different questions, so the choice stays a reading of the field and the map offers both, each labelled. `RuntimeConfig::active_chunk_shape` adds `Area`, the square block of `(2 * radius + 1)²` chunks, which the observer session opts into; the default stays `Line`, so no recorded fixture or replay-verified experiment moved. A Euclidean disc was rejected because at radius 1 it is a five-chunk cross. Two dimensions exposed a latent defect in `chart_chunk_hash`: a sign-extended `-1` is all ones on every axis, so `(-1, -1, 0)` collided with `(0, 0, 0)` and the mana cell object identity validator rejected the first area-shaped runtime — the x term is unchanged and the off-line terms are zero at zero, so every line-shaped chart keeps the identity it was recorded with;
- Bumped the pinned Rust toolchain from 1.85.0 to 1.97.1 (`rust-toolchain.toml`) to track the newest stable release. The repository's `rust` CI job installs `dtolnay/rust-toolchain` with no explicit `toolchain` input, so it always resolves to the current stable release regardless of `rust-toolchain.toml`; this bump brings local/`rust-toolchain.toml` reproducibility back in line with what CI already runs, rather than pinning CI backward. Fixed the resulting new lints: two `clippy::manual_unwrap_or` occurrences (`causafera-analytics`, `causafera-lab`), one `clippy::useless_conversion`-style explicit `.into_iter()` on a `zip` argument (`causafera-cognition`), and four `clippy::collapsible_if` occurrences rewritten using stable 2024-edition let-chains (`causafera-runtime`).
- Made terrain elevation, roughness and material continuous across chunk boundaries (`TODO-GEO-005`, `plans/terrain-chunk-boundary-continuity.md`). `terrain_cells` computed its ridge term from a cell's chunk-local `x`/`y`, always reset to `0..32`, and reached the chunk only through a seed XOR — `terrain_seed ^ chart_chunk_hash(chunk)`, computed identically at both `runtime_carrier_adapters` and `TerrainBootstrapStage::bootstrap` — so every chunk repeated the same diagonal ridge shifted by an unrelated offset. Measured at seed 7: the east edge of chunk (−1, 0) read +13.1 m … +19.5 m against −13.5 m … −13.7 m on the abutting west edge of chunk (0, 0), a ~30 m step where the mean interior neighbour step is 1.6 m. The generator now keys its per-cell hash on each cell's chart-local position — `chunk.chunk.world_origin()` plus its local index — and a `chart_seed` that varies by chart, not by chunk; both call sites stopped XOR-ing `chart_chunk_hash` into the generation seed, since a per-chunk seed term would have reintroduced the same boundary jump even with a continuous ridge. Re-measured: boundary step mean 1806 mm against interior step mean 1617 mm — the same order of magnitude, and in fact lower. `TerrainGeneratorFingerprint` moved to `0x2407_0001`; no checked-in fixture or replay capture exists in the repository, so none needed regeneration, but two downstream tests that had pinned incidental values from the old generator's output were re-pointed against measured evidence. `TerrainBootstrapStage`'s bootstrap-event fingerprint still uses `chart_chunk_hash(chunk)`, kept deliberately separate from the pure `terrain_seed` now passed to generation: reusing the plain seed for both, tried first, made the default observer session's `terrain_seed = 0` collide its bootstrap event's before/after fingerprints and panic on every default-seed launch, caught by running `desktop:dev` and now covered by `a_world_bootstraps_at_the_zero_seed`. Opened `TODO-GEO-006`: the standing carrier's `terrain_structure` magnitude still computes an edge cell's contrast from neighbours within its own chunk only, so it cannot yet see the continuity this change gives the field itself.
- Gave the terrain structure carrier real cross-chunk neighbours (`TODO-GEO-006`, `plans/terrain-structure-cross-chunk-neighbours.md`). `elevation_contrast` and `material_difference` indexed only the one `TerrainChunk` passed to `TerrainCarrierAdapter::new`, dropping a direction entirely at a chunk edge, so an edge cell's `structure` — the standing carrier's mana-facing magnitude — was drawn from at most 2-3 real neighbours where an interior cell has 4, even after `TODO-GEO-005` made the underlying elevation continuous. `TerrainCarrierAdapter::new` and `project_columns` now take a `BTreeMap<ChartChunkCoord, TerrainChunk>` of sibling terrain; `neighbor_cells` resolves each of a cell's four axis-aligned neighbours from that map via `ChartChunkCoord::same_chart_neighbor`, mirroring `causafera-domains::mana`'s existing `OpenNeighbors` idiom, with a missing entry dropping that direction exactly as before rather than inventing a value. Recomputing a missing neighbour from the deterministic generator formula was considered and rejected: `TerrainChunk` is data, not a cached formula evaluation, and a hand-built test fixture already proves the two can diverge. `TerrainBootstrapStage::bootstrap` and `import_carrier_adapters` — the two call sites whose adapters ever reach a live or resumed runtime — now generate or decode every sibling chunk before building any one adapter; `runtime_carrier_adapters`, whose output is unconditionally overwritten by bootstrap before anything reads it, passes an empty map rather than being restructured for no observable effect. Measured: 2 of 9 lattice columns in a boundary chunk change `structure` once its real neighbour becomes visible, the mana field shifts only at the boundary-touching extremes, and bootstrap wall-clock is unchanged within run-to-run noise at every `active_chunk_radius` from 1 to 4. No existing test needed re-pointing.
- Recalibrated the local mana effect gate against a populated field (`TODO-MANA-007`, `plans/mana-gate-calibration.md`). `TODO-MANA-004`'s evidence framed this as field-wide saturation — share of live cells above the gate climbing from 0% to 24% as the lattice refines — but `ManaEffectsSystem::execute` reads only cell 0 of each contacted material surface's field, never the field-wide distribution. Measured against that actual population across six seeds and five candidate lattices, the constants inherited from before `TODO-RUNTIME-002` (4096/2000) already discriminated worlds at four of five lattices, including the production default; the one real failure was the coarsest lattice tested, where the population's mean sat above the threshold. `effect_threshold`/`effect_hysteresis` moved to 6144/1536 — the only point in a sweep that discriminates at all five lattices simultaneously, confirmed as a neighbourhood plateau rather than a spike, and re-verified end-to-end against real production runs on the exact five-field behaviour tuple the original evidence used: distinct tuples across the six seeds move from 2, 2, 3, 2, 1 to 4, 3, 3, 4, 4 at extents 3, 4, 6, 8 and 12. Response channel weights, diffusion, decay, the stencil and the lattice are untouched. `different_seeds_produce_different_worlds_not_one_world_with_two_terrains` needed re-pointing: its hand-picked seed pair (7, 30) collapsed onto one behaviour tuple under the recalibrated gate — the exact defect this change fixes — and was moved to (7, 5).
- Investigated `TODO-PERF-001` and drafted `plans/performance-baseline-and-digest-cost.md` (Draft, not yet implemented). Swept the validated `RuntimeConfig` space and found two concrete issues rather than a general "add more benchmarks" gap: `RuntimeConfig::validate()`'s independent `actor_count ≤ 128` / `sensor_count ≤ 16` bounds admit combinations `causafera-cognition`'s `MAX_SCENE_CUES = 64` cap cannot execute (e.g. `actor_count=64, sensor_count=1` constructs successfully and then fails on the first tick), and `RuntimeState::snapshot` — called by every `Runtime::tick()` and every observer-poll `Runtime::snapshot()` — unconditionally recomputes both `physical_state_digest` and `history_digest` from scratch each call; `history_digest` alone already accounts for 46% of tick time in the very first 64-tick batch of the smallest possible workload and grows to 87% by tick 512 from trace-store accumulation, with per-tick wall time rising 6.8x over that span at a fixed, unchanging config — driven mainly by that trace-store growth, with a smaller additional contribution from `physical_state_digest`'s own unbounded (and, per the plan, not-yet-fixed) growth via unpruned `thermal_receipts`/`thermal_conservation_receipts` maps. Also found the shipped `material_surface_loop_benchmark`/`benchmark.rs` harness takes no repeated measurements (reproduced a physically-impossible negative observer-overhead delta from single-run noise) and shares one process across its two measured RSS cases, so its second-mode peak-RSS figure is polluted by the first mode's already-torn-down `Runtime`; and that `.github/workflows/benchmarks.yml` runs the ignored test suite but persists no benchmark output for comparison across commits, despite `docs/performance/benchmarks.md`'s existing Reporting requirements. No `crates/`/`apps/` source file changed in this investigation; all instrumentation used to gather these numbers was written, run, and reverted before this and the accompanying documentation updates were made — the working tree as a whole is not clean, only the source tree is;
- Generated surface material as spatially coherent regions instead of per-cell noise (`TODO-GEO-004`, `plans/coherent-surface-material-regions.md`). `terrain_cells` derived material from the same well-mixed per-cell hash driving elevation, measured at 6.5%–6.75% same-material neighbours against 6.25% expected by chance over sixteen materials — no more coherent than noise, and a de facto constant floor in `terrain_structure`'s `material_delta` term (50.0 of a mean 204.4, essentially flat across seeds) that the function's own doc comment says should not exist. Replaced with a bounded Worley (cellular) partition — chart-scoped coarse feature points, jittered within their own coarse cell, searched over a provably-sufficient 5x5 (Chebyshev-2) neighbourhood, not the more common but unproven 3x3 — continuous across chunk boundaries by construction, the same chart-position idiom `TODO-GEO-005` established for elevation. `MATERIAL_REGION_SIZE` (16) chosen from a sweep against same-material rate and the mana column footprint. Real production same-material rate rises to 93.0%–94.1%, with the chunk-boundary rate (90.6%) the same order as the interior rate (93.1%) — no continuity artifact; same-material slightly overstates true region coherence (two different regions can share a material by chance), measured directly at 92.1% material against 91.7% region — a 0.4-point gap. Re-measured `extent_decision.rs` immediately before and after under identical conditions: the mana lattice's fidelity-vs-noise ratio improves at every candidate extent (extent 6: 1.15x → 1.59x; extent 8: 1.23x → 1.52x), which flags `TODO-MANA-004`'s "extent stays 3" decision for its own re-review without reopening it here. Three pre-existing tests needed attention: a recipe-source isolation test that never actually set terrain `Inert` (fixed by setting it, matching its own stated intent), a below-threshold test's pinned incidental intensity moving a second time (182 → 154), and a seed-discrimination test whose hand-picked pair collapsed a second time, rewritten to sweep eight seeds against a non-collapse claim with its independently-too-short 48-tick duration raised to 192, matching `extent_decision.rs` and `mana_gate_calibration.rs`.
- Landed the first two waves of `plans/performance-baseline-and-digest-cost.md` (`TODO-PERF-001`): a checked-in benchmark harness, and a construction-time rejection for configurations the runtime cannot execute. Wave 1 replaced the investigation's deleted scratch probes with `crates/causafera-runtime/examples/performance_baseline.rs`, plus two `pub` measurement surfaces in `benchmark.rs` — `measure_digest_cost`, and a `canonical_state` on each measurement that `benchmark_validation.rs` now checks for equality across the two observer modes, so the harness cannot report an overhead delta between two runs that did not compute the same thing. It reports mean/median/stddev over N=20 repetitions with the case order cyclically rotated per pass, and one subprocess per (case, repetition) pair, because the shipped harness's shared-process `/proc/self/status` reads made the second mode's peak RSS a reading of the first mode's already torn-down `Runtime`. Wave 2 then closed the validation gap: a per-actor cue batch is exactly the acquired sample count, since `GenericFeatureExtractor::extract` adds a `Change` feature only for a sample pair at strictly increasing times and `acquire_signals` discards any signal whose time is not the acquisition's own, so no single batch can contain two — which makes the worst case `sensor_count * (actor_count + active_chunk_count)`, one aperture per sensor over every promoted actor's object plus, since `MaterialSurfaceBootstrapStage` creates one surface per active chunk, every surface that has registered contact. `RuntimeConfig::validate` rejects anything past `MAX_RUNNABLE_SCENE_CUES` (the smaller of `MAX_SCENE_CUES` and `MAX_ATTENTION_CANDIDATES`, both 64) with `RuntimeError::SceneCueBudgetExceeded`, naming both terms and the knobs that relieve them; the cognition-layer cap stays as the backstop, covered by its own test. The bound is exact where it can be attained — the validation boundary reproduces the harness's exhaustively measured first failure at 64, 32, 16 and 8 actors for 1, 2, 4 and 8 sensors — and deliberately conservative past roughly eight sensors, where apertures stop seeing every signal and it rejects 4 actors on 16 sensors against a measured failure at 5. It has to be: surface contact does not spread to fill a chart (3 contacted surfaces against 49 active chunks after 768 ticks), so a run cannot demonstrate the worst case and a check calibrated on observed contact would pass configurations a differently-moving population would break. The proof is instead a unit test that forces every active chunk's surface into contact and drives the real perception and cognition steps at it. The cost is real and stated: `Area` at radius 2 or more no longer admits 8 actors on 2 sensors, and the harness's own `radius_4_area` case now runs with `material_surface_signals_enabled = false`, re-measured at 942 ms against the 916 ms recorded with surface signals on. `plans/observer-field-raster-map.md`, which proposes config-gated `Area` charts, will need fewer sensors or its own decision.
- Made `history_digest` incremental (`TODO-PERF-001`, `plans/performance-baseline-and-digest-cost.md` Wave 3). `RuntimeState::snapshot` runs on every tick and every observer poll, and it re-walked the entire causal trace store from tick 0 each time, so a run paid for its own history on every call — 46% of tick time in the first 64-tick batch of the smallest workload, 87% by tick 512. `CanonicalDigest` is a pure streaming accumulator, so the same sequence of writes reaches the same state whether performed in one pass or resumed across many, and `history_digest` writes the trace store first with only bounded, capped state after it. `RuntimeState` now carries a `HistoryDigestPrefix` holding the accumulator and how many events it has folded in; each call absorbs only what was committed since, copies the prefix, and writes the bounded tail into the copy. Three properties of `CausalTraceStore` make that sound, verified rather than assumed: `commit_batch` is its only `&mut self` method, it only pushes (no truncate, clear, pop, remove, drain or retain exists anywhere in the crate), and it never rewrites an earlier event's cause or effect offsets. The one field it does back-patch is `children`, appending to an already-committed parent — `history_digest` never reads it, and both the prefix type and the shared `write_trace_event` say so explicitly, because adding it would silently break the resume. No digest schema version change: `CURRENT_DIGEST_SCHEMA_VERSION` stays at 5 and the value is required to be bit-identical, which is why the load-bearing check is a differential oracle against a retained full-rescan implementation rather than the existing replay and locale suites — those compare two runs from the *same* implementation and would agree with each other even if the accumulator absorbed wrongly in a way that moved both. The oracle asserts equality at every one of 48 consecutive ticks (also asserting each tick committed events, so a degenerate run cannot pass vacuously), across repeated observer polls interleaved with ticks, and across snapshot export, import and resume — including the digest `assemble_envelope` writes into the snapshot header, since a wrong prefix would otherwise be recorded into every exported snapshot. Confirmed the oracle has teeth by injecting an off-by-one into the absorbed count: all three tests fail, and reverting restores them. No pinned digest value in any existing test needed re-pointing, which is itself evidence of bit-identity. Measured at the plan's own fixed workloads, 64 ticks: 147 ms to 22 ms after seven warm-up batches, 736 ms to 468 ms at `chunk_extent` 16, 942 ms to 550 ms at radius 4, and the run-length penalty between the zero-warm-up and seven-batch cases falls from 6.7x to 1.7x. The residual is `physical_state_digest`, whose own unbounded `thermal_receipts`/`thermal_conservation_receipts` growth this wave deliberately does not touch and which remains a named open follow-up.
- Made the "Benchmarks and Long Runs" CI job capture a benchmark number (`TODO-PERF-001`, `plans/performance-baseline-and-digest-cost.md` Wave 4). The job ran only `cargo test --release -- --ignored` and stored nothing, so the "stored, compared across commits, flagged on regression" requirements in `docs/performance/benchmarks.md` were unmet by any job despite its name. It now also runs the Wave 1 harness and uploads the output as an artifact named for the commit SHA. Capture only, deliberately: no threshold and no regression flag, because one chosen before any historical series exists would be a guess rather than a measurement — this makes cross-commit comparison possible without claiming to perform it, and `benchmarks.md` now states which of its four Reporting requirements that does and does not satisfy. The job can still fail on the harness's boundary sweep finding a configuration `RuntimeConfig::validate` accepted that then exceeded the cue cap at a tick, which is a soundness bug in the Wave 2 bound rather than a performance threshold; the artifact uploads regardless so the evidence survives the failure. Full three-mode run measured at 39 s locally after Wave 3, against the job's default 360-minute timeout.
- Closed `TODO-PERF-001` and carried its excluded work forward (`plans/performance-baseline-and-digest-cost.md` Wave 5). Every in-scope acceptance criterion is met — a checked-in statistical harness, both findings reproducible without throwaway instrumentation, the configuration-validation gap fixed at construction time, `history_digest` incremental and bit-identical, and per-commit CI capture — while reference-hardware runs, CI regression gating and any treatment of `physical_state_digest` had each been named Out of Scope before the work began, so no remaining wave could ever have cleared a Pending status. Rather than let that excluded work vanish into a closed entry, it becomes `TODO-PERF-002` (`physical_state_digest`'s unbounded `thermal_receipts`/`thermal_conservation_receipts` growth, now the largest single named per-tick cost at roughly 6.3 ms of a 13.0 ms case and 8.0 ms of a 22.4 ms case per 64 ticks, carrying the three candidate approaches and the constraint that any change to the digest's bytes needs an explicit schema bump) and `TODO-PERF-003` (regression flagging against an actual historical series, and a reference-hardware run). Consistency pass over every cross-document reference: `docs/roadmap/roadmap.md` still described the plan as "Draft, not-yet-accepted" with "no implementation has landed", `docs/simulation/long-run-experiments.md` and `docs/ontology/domain-coverage-matrix.md` carried stale status markers, and long-run-experiments had become self-contradictory — one paragraph calling both digests full-rescan, the next describing `history_digest` as incremental. Noted but deliberately not fixed as unrelated: both workflows pin `dtolnay/rust-toolchain` with a trailing `# 1.85.0` comment while passing no `toolchain` input, so the comment names a version CI does not actually use.
- Fixed the misleading `dtolnay/rust-toolchain` pin comment left open above, and corrected the diagnosis that named it — both the diagnosis in the entry above and the identical one in `plans/performance-baseline-and-digest-cost.md` were wrong about the mechanism. `dtolnay/rust-toolchain` does not read a `toolchain` input at all at the pinned commit: each version of that action lives on its own branch whose `action.yml` hardcodes the toolchain string directly (confirmed by diffing the pinned commit `8641a17e25bf5b40c118d48fe0f81e8655731839` against the action's upstream history), so omitting the input does not make it "resolve to current stable" as both prior notes claimed — it installs exactly Rust 1.85.0, the version that commit hardcodes, and always would regardless of `rust-toolchain.toml`. Checked what that meant for the actual build rather than assuming (`gh run view` on a recent green `main` run of the `rust` job): `rustup default 1.85.0` runs and takes effect, but every subsequent `cargo` invocation happens inside the checked-out repo, so rustup's own toolchain-file override reads `rust-toolchain.toml`, auto-installs 1.97.1 on the first `cargo fmt --check`, and every real compile and clippy check in the job already ran under 1.97.1 — there was no CI/local version skew to begin with. The actual defect was narrower: the pinned action installed and set default to a 1.85.0 toolchain that nothing in the job then used, wasting a toolchain download every run, while its trailing comment named the version actually discarded rather than the one actually compiling. Repointed both `.github/workflows/ci.yml` and `.github/workflows/benchmarks.yml` to commit `46511b1c83438f0dd37c02d843619ece5a4abb5b` (`dtolnay/rust-toolchain`'s `1.97.1` branch head), so the action installs the same toolchain `rust-toolchain.toml` was already forcing cargo to fetch, and its comment now names the version that runs.

#### UI

- Completed the **Observer Locale Coverage** slice (`plans/observer-locale-coverage.md`, `TODO-UI-006`): the observer now presents itself in `en-US`, `ru-RU`, `zh-Hans`, `de-DE` and `es-ES` across all three layers that carry human language, not only the dictionary. English became the derived baseline — `Copy` is `typeof en`, so a key added to English is a compile error in the other four until translated — and the dictionary split into one module per locale. Widening `ObserverLocale` turned all 139 `Record<ObserverLocale, string>` literals in the claim descriptors, coverage register and lens catalogue into compile errors until complete, which is how the map legend and the Explanation schema labels were kept in step with the chrome. The authoritative Rust renderer widened from a two-variant enum to five, with format strings moved into `[&str; 5]` tables and named placeholders, and `ObserverLocale::parse` now resolving by primary subtag with the script deciding for Chinese. Traditional-script tags (`zh-Hant`, `zh-TW`, `zh-HK`, `zh-MO`) deliberately fall back to English rather than being answered with simplified text, on the same principle that forbids an invented reading. An explicit choice persists in `localStorage`, a first run follows `navigator.languages`, and anything unrecognised resolves to English; previously the session opened in Russian for every reader on every run. The language switcher became a single meridian cell whose options name themselves in their own language, plus one command-palette entry per locale. Fixed a live defect found on the way: the relief, elevation-range, roughness and contour lenses hard-coded `м` and `мм`, so an English session read metres labelled in Cyrillic. INV-007 coverage widened from one locale pair to the whole set, comparing per-tick digests across one runtime per locale and, in the Tauri session, the emitted payload bytes as well;
- Rebuilt the chart as a map rather than a grid of squares (`TODO-OBS-001`). Every received lattice is assembled into one field over the whole surveyed extent, and the tint, the relief shading, the contours and the hover readout are all readings of that one surface, so a gradient crossing a chunk boundary is drawn as one gradient. Interpolation between samples is Catmull-Rom, which passes through every measurement exactly and is clamped to the bracketing pair; contours refine a coarse lattice through the same interpolant before marching squares runs, so the line and the tint underneath it cannot disagree. Hillshading is Lambertian, computed in the presentation layer and applied only to fields whose gradient is a slope — mana intensity is not a height. The graticule stops ruling the sheet: over a drawn field the chunk lattice is stated by ticks at the intersections and only resolves into rules once a chunk is large enough that its boundary is a reading rather than a frame. The chart opens on the mana field, the one field the runtime maintains that is continuous across the extent; the terrain contour lens is promoted from `preview` to a measured trace of the elevation lattice and keeps the interpolation only as its fallback. Mana availability is derived from the received lattice edge rather than declared, so refining `chunk_extent` promotes those lenses with no change to the catalogue. Terrain contours are deliberately not a default overlay: `terrain_cells` derives elevation from chunk-local coordinates only, so every chunk repeats the same diagonal ridge and the chart carries a thirty-metre scarp on every boundary — measured at +13.1 m … +19.5 m against −13.5 m on the abutting edge, where the mean neighbour step inside a chunk is 1.6 m. The relief lens draws it and states that the step is world state rather than a seam in the drawing; the generation gap is recorded as `TODO-GEO-005`;
- Rewrote `tools/audit/validate-i18n.mjs` for that architecture. Key parity is now a compile error, so the tool checks what the compiler cannot see: placeholder parity, untranslated leakage against a per-key allowlist that states a reason, empty values, agreement between the four places that enumerate locales independently, and placeholder parity inside the Rust template tables — where `rustc` checks that an array holds five strings but nothing checks that `{name}` survives into all five. Verified against seeded regressions in both the TypeScript and Rust halves;
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
- Bounded repaint to the scrolling regions with `contain: paint`, and stopped republishing map hover readings that had not changed;
- Fixed the Instrument exchange log inflating the reported cost of a raster frame. `refreshRasters` issues a frame's per-chunk `observer_field_raster` calls together (`Promise.all`), but they still serialise on the Tauri session mutex, so each call's own `durationMs` already carries the queueing wait for every call ahead of it. `recordExchange`'s fold summed those durations across the frame, counting the shared wait once per call and inflating the logged cost roughly with the square of the batch size instead of reporting it. The fold now reports the batch's wall-clock span — first call's start to the folding call's finish — recovered from the already-folded entry's `at` and `durationMs` rather than re-accumulated, so the instrument reads the wire's real behaviour before any batching of the nine per-frame calls is considered;
- Stopped every observer command answering across the Tauri bridge as a `Vec<u8>` return, which Tauri serialises as a JSON array of numbers — four times the wire bytes for the 3.3 KB terrain raster, encoded and decoded as JSON on every call in both directions. Commands now answer with `tauri::ipc::Response`, which carries the same bytes raw; confirmed against a real desktop session that the JS side receives an `ArrayBuffer` rather than a number array, and `apps/observer/src/observer/transport.ts` decodes it directly instead of falling through to `Uint8Array.from`. Measured against the corrected exchange log above (`TODO-OBS-002`): at today's payload sizes (113 B to ~3.4 KB) this does not move per-call duration, which is dominated by fixed per-call overhead rather than serialisation cost — the fix removes a real inefficiency without yet being the bottleneck, and matters more once payloads grow, including under a future batched `FieldRaster` query (`TODO-OBS-002`, not implemented here — spans the wire protocol, the runtime and both frontend transports, so it is recorded rather than done in this change).

#### DOCS

- Added `GOVERNANCE.md` as the authoritative governance document: Causafera is an independent, author-led, maintainer-governed FOSS project whose canonical repository is directed by the natural person who controlled GitHub user ID `281476371` on 27 July 2026, then using the login `Sir-Starch`, not by community consensus, contributor vote, or influence proportional to contribution size. The account is an evidentiary anchor to a fixed date rather than a transferable token, matching `CLA.md`: a username change, a released login, a repository move, or later control of the account by sale, transfer, or compromise conveys no authority. Contributing does not automatically create governance rights; authority exists only where the maintainer has explicitly delegated it, and such delegation is scoped, revocable, and never accumulated through continued contribution. Technically valid work may be declined for vision fit, and design disagreements are legitimately explored through forks. The document also records why the engine must stay FOSS — independent inspection, reproducibility, verification of causal traces and assumptions, adaptation, and the avoidance of unverifiable "trust the author" claims — while separating source availability from validation, which comes from deterministic replay, tests, provenance, documented assumptions, and reproducible experiments. Six concepts are held apart deliberately: public FOSS licensing, governance authority, copyright ownership, CLA-granted rights, commercial licensing, and scientific or technical validation;
- Released CLA version 1.1. The document deliberately carries **no operational status** and uses **absolute links**: it is the text contributors sign, it must read identically before and after the acceptance service is configured, and it is published to a Gist where relative links would be dead. A status banner would have forced an edit the moment the service went live, breaking the Gist's byte-identity and requiring everyone to re-accept a CLA they had just accepted; contribution status therefore lives only in `CONTRIBUTING.md`, `README.md`, `docs/legal/cla-service-setup.md`, and the pull request template. Anchored the Project Maintainer to the natural person who controlled GitHub user ID `281476371` on the effective date, so that renaming the account, releasing the `Sir-Starch` login, moving the repository, or acquiring the account by sale, transfer, or compromise conveys no rights, and transfer requires a separate written assignment; the legal name remains unpublished. Scoped the copyright grant to Contributions submitted through the authenticated identity covered by an acceptance, until that acceptance is terminated or a later revision is required for new Contributions — replacing a blanket grant over "all present and future Contributions" that contradicted both the termination clause and versioned re-acceptance; grants over already-submitted Contributions remain perpetual and irrevocable. Narrowed "Contribution" to material intentionally submitted for inclusion through a pull request, a commit or patch, or an explicitly designated channel, so issue comments, bug reports, feature requests, and design discussion are no longer swept into the licence grant. Directed the patent grant to the Project Maintainer and recipients of the Project rather than to "the Project", which is defined as a codebase and cannot hold rights or perform legal acts, keeping the contribution-scoped grant and defensive termination intact; applied the same correction wherever the Project was made an actor — retaining acceptance records, publishing updated versions, maintaining attribution, receiving termination notice, designating contribution channels, and being party to the agreement. Aligned the Public Project License definition with the repository's functional/non-functional split. Replaced the availability condition with a per-release licensing commitment: every public release containing an accepted contribution licenses it under the applicable public licence and that licence cannot later be revoked, while explicitly creating no obligation to host files, retain contributions in future versions, or maintain the project indefinitely. Aligned the acceptance-record fields with what a hosted CLA service actually exports — authenticated GitHub identity with numeric user ID where available, repository, accepted document revision, timestamp — with pull-request association established through the service comment, GitHub history, or a private register. Stopped the section 6 disclaimer from negating section 5: the "AS IS" disclaimer and the liability limitation are now expressly subject to the representations the contributor makes about originality and third-party rights, `NON-INFRINGEMENT` is no longer disclaimed since section 5 promises it, and the limitation does not reach fraud, wilful misconduct, or breach of those representations — a document that both warrants provenance and disclaims all warranties warrants nothing. Added section 13 confirming that the CLA grants no governance rights and transfers no copyright;
- Replaced the blanket prohibition on external code and documentation contributions with a staged policy. Until the CLA acceptance service is configured and tested, external pull requests may be prepared, submitted, and discussed but cannot be merged; no repository text implies the CLA workflow is already operational. `CONTRIBUTING.md` now documents the intended eight-step flow from open TODO through CLA acceptance to merge;
- Documented that AI coding agents are explicitly permitted, including handing an open TODO to an agent, while the human contributor remains accountable for understanding the change, reviewing the complete diff, running and honestly reporting validation, holding the right to submit the work, correcting hallucinated or unrelated changes, and complying with the architecture, determinism, provenance, and documentation requirements. Unreviewed generated diffs are closed rather than iterated on;
- Added `docs/legal/cla-service-setup.md`, a maintainer checklist for the manual work required outside the repository, written against the mechanism CLA Assistant actually uses: publishing the CLA as a public GitHub Gist and recording its specific revision hash for the maintainer's audit trail, verifying the Gist and repository copies are identical, linking `Sir-Starch/Causafera` to that Gist — the service follows the Gist rather than a hand-picked revision, so there is no per-revision configuration to maintain — testing with a real pull request from a second account, requiring the status check on `main`, handling automation accounts such as Dependabot, exporting and privately retaining acceptance records with a private PR-association register, publishing a material CLA change as a new Gist revision and confirming the service detects it and re-asks earlier accepters, and updating repository status text only after the service is confirmed operational. Nothing in it has been performed; no external GitHub or CLA Assistant setting was configured by this change. Professional legal review is not a prerequisite for any of it — the checklist notes only that it is worth considering before material commercial licensing agreements, substantial corporate contributions, rights assignment, or relying on the CLA in a dispute;
- Added contributor confirmations to the pull request template covering diff review including AI-assisted changes, the right to submit, honest validation reporting, and CLA acceptance as a merge precondition, without duplicating the CLA text;
- Described Causafera consistently as an author-led free and open-source simulation engine across `README.md`, `CONTRIBUTING.md`, `SUPPORT.md`, and `docs/development/contributing.md`, and removed the stale "personal hobby" and "contributions not accepted" wording; added a terminology note to `CODE_OF_CONDUCT.md` clarifying that the Contributor Covenant's "community leaders" describes moderation rather than project governance;
- Retargeted `TODO-LEGAL-001` from mandatory professional legal review to the configuration and end-to-end verification of the CLA Assistant workflow for CLA 1.1.

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
