# Causafera TODO Backlog

## TODO-LEGAL-001: CLA Legal Review and Acceptance Automation
**Status:** Pending
**Phase:** 0
**Priority:** Critical — before first external contribution
**Dependencies:** None
**Goal:** Have CLA version 1.0 and its acceptance workflow reviewed by a Netherlands-qualified IP/open-source lawyer, then configure a CLA service or equivalent electronic-signature process.
**Acceptance Criteria:** Counsel review recorded; acceptance cannot be inferred from pull-request submission; durable records include verified contributor identity, CLA version, timestamp, and associated pull request or commit; changed CLA versions require new acceptance.
**Performance Requirements:** N/A
**Determinism Requirements:** N/A
**Ontology Implications:** None
**Observer Implications:** None
**Explanation Implications:** None
**Out of Scope:** Accepting external contributions before review and automation are complete.

## TODO-ARCH-001: Workspace Structure
**Status:** Completed
**Phase:** 0
**Priority:** High
**Dependencies:** None
**Goal:** Establish Cargo workspace with domain crates
**Acceptance Criteria:** Workspace compiles, all crates have lib.rs
**Performance Requirements:** N/A
**Determinism Requirements:** N/A
**Ontology Implications:** Establishes domain boundaries
**Observer Implications:** N/A
**Explanation Implications:** N/A
**Out of Scope:** Implementation of domain logic

## TODO-CORE-001: Deterministic Scheduler
**Status:** Completed
**Phase:** 1
**Priority:** High
**Dependencies:** TODO-ARCH-001
**Goal:** Implement deterministic simulation scheduler with phase control
**Acceptance Criteria:** Scheduler ticks deterministically, phases execute in order
**Performance Requirements:** Minimal overhead per tick
**Determinism Requirements:** Strict mode: identical seeds produce identical states
**Ontology Implications:** Defines time and phase primitives
**Observer Implications:** Exposes simulation time
**Explanation Implications:** N/A
**Out of Scope:** Domain-specific phase logic

## TODO-CORE-002: Typed ID System
**Status:** Completed
**Phase:** 0
**Priority:** High
**Dependencies:** TODO-ARCH-001
**Goal:** All IDs are strongly typed to prevent mixing
**Acceptance Criteria:** AgentId, ConceptId, etc. are distinct types
**Performance Requirements:** Zero-cost abstraction
**Determinism Requirements:** N/A
**Ontology Implications:** Identifies entities without semantic meaning
**Observer Implications:** N/A
**Explanation Implications:** N/A
**Out of Scope:** ID generation algorithm

## TODO-COORD-001: Coordinate Primitives
**Status:** Completed
**Phase:** 1
**Priority:** High
**Dependencies:** TODO-CORE-001
**Goal:** Define spatial coordinate types and operations
**Acceptance Criteria:** Chunk coordinates, local coordinates, conversion functions
**Performance Requirements:** Integer math where possible
**Determinism Requirements:** All operations deterministic
**Ontology Implications:** Space is a primitive
**Observer Implications:** Exposes spatial data
**Explanation Implications:** N/A
**Out of Scope:** Terrain generation

## TODO-WORLD-001: Spatial Hierarchy
**Status:** Completed
**Phase:** 3
**Priority:** High
**Dependencies:** TODO-COORD-001
**Goal:** Implement world spatial hierarchy
**Acceptance Criteria:** World → Landmass → Basin → Region → Territory → Chunk → Parcel → Structure → Space
**Performance Requirements:** Efficient parent/child traversal
**Determinism Requirements:** Hierarchy deterministic from seed
**Ontology Implications:** Defines spatial containment primitives
**Observer Implications:** Exposes hierarchy for navigation
**Explanation Implications:** N/A
**Out of Scope:** Political regions

## TODO-GEO-001: Terrain Generation Contracts
**Status:** Completed
**Phase:** 4
**Priority:** High
**Dependencies:** TODO-WORLD-001
**Goal:** Define terrain generation interfaces
**Acceptance Criteria:** Elevation, material type, roughness per cell
**Performance Requirements:** Batch generation support
**Determinism Requirements:** Same seed → same terrain
**Ontology Implications:** Geography is causal state
**Observer Implications:** Exposes terrain for visualization
**Explanation Implications:** N/A
**Out of Scope:** Full terrain implementation

## TODO-BIO-001: Body Segment Primitives
**Status:** Completed
**Phase:** 5
**Priority:** High
**Dependencies:** TODO-CORE-001
**Goal:** Define biological structural primitives
**Acceptance Criteria:** BodySegmentId, parent_segment, joint, length, orientation
**Performance Requirements:** Compact representation
**Determinism Requirements:** N/A
**Ontology Implications:** Biology is causal state, no semantic labels
**Observer Implications:** Exposes structure for inspection
**Explanation Implications:** N/A
**Out of Scope:** Physiological simulation

## TODO-COG-001: Attention Primitives
**Status:** Completed
**Phase:** 7
**Priority:** Medium
**Dependencies:** TODO-BIO-001, TODO-SENSE-001
**Goal:** Implement a bounded subjective attention mechanism without authoritative identity
**Acceptance Criteria:** Fixed focus/candidate bounds; subjective AttentionTargetId distinct from Ground Truth IDs; threshold, salience, and continuity ranking; supporting subjective PerceptId references; no EntityId or TraceId in attention state
**Performance Requirements:** Fixed active-state arrays and sparse bounded updates; no scale claim before benchmarks
**Determinism Requirements:** Attention deterministic given same inputs
**Ontology Implications:** Cognition is bounded
**Observer Implications:** May expose attention state
**Explanation Implications:** Explains why agents miss things
**Out of Scope:** Full cognitive architecture, feature-to-subjective mapping, subjective scene construction, semantic target types

## TODO-SENSE-001: Physical Access and Sensory Acquisition
**Status:** Completed
**Phase:** 7
**Priority:** High
**Dependencies:** TODO-TRACE-001, TODO-BIO-001
**Goal:** Establish a structural boundary that admits only physically accessible signals to extraction
**Acceptance Criteria:** Property-based signals and sensor apertures; deterministic channel/time/range/threshold filtering; relative acquired samples; causal input traces
**Performance Requirements:** Contiguous canonical output batches; benchmark before scale claims
**Determinism Requirements:** Input ordering does not change acquired samples or acquisition IDs
**Ontology Implications:** Observation is incomplete physical access, not Ground Truth transfer
**Observer Implications:** Future read-only diagnostics only
**Explanation Implications:** Supplies causal support for why a signal was accessible
**Out of Scope:** Semantic modalities, realistic propagation/occlusion, physiology, sensor noise, cognition

## TODO-PERCEPT-001: Generic Feature Extraction
**Status:** Completed
**Phase:** 8
**Priority:** High
**Dependencies:** TODO-SENSE-001, TODO-ONTO-001
**Goal:** Extract generic structural features only from acquired sensory samples
**Acceptance Criteria:** Deterministic magnitude/change extraction; typed acquired-sample input; canonical FeatureId assignment; flattened supporting TraceId spans
**Performance Requirements:** Contiguous feature batches and flat provenance offsets; benchmark before scale claims
**Determinism Requirements:** Sample input ordering does not change features or provenance
**Ontology Implications:** Feature relations remain generic; authoritative target identity stops before Phase 9 cognition
**Observer Implications:** Future read-only feature diagnostics only
**Explanation Implications:** Feature claims retain physical causal support
**Out of Scope:** Subjective identity, scenes, concepts, semantic classifiers, exhaustive feature algorithms

## TODO-SCENE-001: Implement Minimal Subjective Scene Representation
**Status:** Completed
**Phase:** 9
**Priority:** High
**Dependencies:** RFC-COG-001 accepted
**Goal:** Implement the minimum viable subjective scene representation based on accepted RFC-COG-001
**Acceptance Criteria:** Fixed-capacity Rust scene and cue types; deterministic reconstruction; attention-gated contents; no authoritative identity or trace identifiers in cognitive state
**Performance Requirements:** Bounded active state per agent; deterministic reconstruction
**Determinism Requirements:** Scene construction deterministic given same inputs and history
**Ontology Implications:** Agents act on constructed scenes, not raw Ground Truth (INV-029)
**Observer Implications:** May expose scene analytics
**Explanation Implications:** Explains why different agents experience the same place differently
**Out of Scope:** Full implementation of all scene subsystems

## TODO-SCENE-002: Perceived Object Persistence
**Status:** Completed
**Phase:** 9
**Priority:** High
**Dependencies:** TODO-SCENE-001
**Goal:** Design and implement subjective object identity tracking
**Acceptance Criteria:** `PerceivedObjectId` distinct from `EntityId`; deterministic signature/location matching permits merge and split errors; bounded stale hypotheses may be lost or replaced
**Performance Requirements:** Sparse updates; bounded number of tracked objects per agent
**Determinism Requirements:** Identity updates deterministic given same perceptual history
**Ontology Implications:** Perceived identity is a subjective hypothesis (INV-028)
**Observer Implications:** Exposes identity tracking analytics
**Explanation Implications:** Explains misidentification and false recognition
**Out of Scope:** Perfect identity tracking; omniscient object knowledge

## TODO-SCENE-003: Subjective Body Schema
**Status:** Completed
**Phase:** 9
**Priority:** Medium
**Dependencies:** TODO-BIO-001, TODO-SCENE-001
**Goal:** Design the mapping from biological signals to experienced body model
**Acceptance Criteria:** Fixed-capacity subjective part identities and experienced numeric properties are structurally distinct from `BodyStructure`/`BodySegmentId` and update only through identity-free inputs
**Performance Requirements:** Incremental updates; bounded schema size
**Determinism Requirements:** Schema updates deterministic given same signal history
**Ontology Implications:** Objective body state and subjective body schema are distinct (INV-034)
**Observer Implications:** May expose schema vs structure divergence
**Explanation Implications:** Explains phantom sensations, capability misjudgments
**Out of Scope:** Full physiological simulation; neural body mapping

## TODO-SCENE-004: Self-Model Architecture
**Status:** Completed
**Phase:** 9
**Priority:** Medium
**Dependencies:** TODO-SCENE-001
**Goal:** Design the persistent but revisable self-model
**Acceptance Criteria:** Persistent bounded opaque self-associations are revisable, percept-supported, activated separately into scenes, and contain no Ground Truth identity or predefined trait taxonomy
**Performance Requirements:** Persistent but not continuously active
**Determinism Requirements:** Self-model updates deterministic given same experiences
**Ontology Implications:** The self-model is subjective (INV-033)
**Observer Implications:** May expose self-model divergence
**Explanation Implications:** Explains overconfidence, identity confusion
**Out of Scope:** Full personality simulation; fixed trait enums

## TODO-SCENE-005: Predictive World Model
**Status:** Completed
**Phase:** 10
**Priority:** High
**Dependencies:** TODO-SCENE-001
**Goal:** Design sparse, bounded prediction and prediction-error handling
**Acceptance Criteria:** Fixed-capacity generic near-future predictions resolve against accessible cues and emit bounded numeric prediction errors suitable for downstream attention, salience, memory, and concept revision
**Performance Requirements:** No global physics per agent; bounded prediction count
**Determinism Requirements:** Predictions deterministic given same beliefs and scene
**Ontology Implications:** Prediction error is a first-class cognitive driver (INV-035)
**Observer Implications:** May expose prediction accuracy analytics
**Explanation Implications:** Explains surprise and expectation-driven behavior
**Out of Scope:** Global physics simulator; perfect prediction

## TODO-SCENE-006: Working Memory and Active Context
**Status:** Completed
**Phase:** 10
**Priority:** High
**Dependencies:** TODO-SCENE-001
**Goal:** Design bounded working memory separate from persistent storage
**Acceptance Criteria:** Working context has a hard capacity, canonical activation ranking, deterministic decay/rehearsal, and is structurally separate from episodic storage
**Performance Requirements:** Bounded active items per agent; minimal overhead for inactive agents
**Determinism Requirements:** Working memory updates deterministic given same cues and state
**Ontology Implications:** Persistent memory is not continuously active context (INV-032)
**Observer Implications:** May expose active context size
**Explanation Implications:** Explains forgetting and inattention
**Out of Scope:** Unlimited working memory; perfect recall

## TODO-SCENE-007: Episodic Memory Reactivation
**Status:** Completed
**Phase:** 10
**Priority:** Medium
**Dependencies:** TODO-SCENE-006
**Goal:** Design similarity-driven, relevance-weighted memory reactivation
**Acceptance Criteria:** Quantized perceptual signatures reactivate at most four ranked episodes through graded similarity and relevance; no semantic event triggers
**Performance Requirements:** Efficient similarity matching; bounded reactivation per tick
**Determinism Requirements:** Reactivation deterministic given same memory index and cues
**Ontology Implications:** Memories become active through similarity, not semantic lookup
**Observer Implications:** May expose reactivation traces
**Explanation Implications:** Explains involuntary memory and deja vu
**Out of Scope:** Perfect memory retrieval; keyword-based search

## TODO-SCENE-008: Agency Attribution
**Status:** Completed
**Phase:** 10
**Priority:** Medium
**Dependencies:** TODO-SCENE-005
**Goal:** Design learned agency attribution from action-outcome observation
**Acceptance Criteria:** Bounded opaque action/outcome associations learn deterministic proximity-weighted strength and can represent incorrect attribution without privileged causality
**Performance Requirements:** Incremental updates; bounded agency model size
**Determinism Requirements:** Agency updates deterministic given same action-outcome history
**Ontology Implications:** Agency is constructed, not innate
**Observer Implications:** May expose agency attribution analytics
**Explanation Implications:** Explains superstition and learned helplessness
**Out of Scope:** Perfect causal knowledge; innate agency

## TODO-SCENE-009: Subjective Temporal Continuity
**Status:** Completed
**Phase:** 10
**Priority:** Medium
**Dependencies:** TODO-SCENE-001
**Goal:** Design the bounded temporal envelope binding recent past, current scene, and expected near future
**Acceptance Criteria:** A fixed-capacity temporal envelope binds recent subjective object/percept frames and prediction error with deterministic oldest-frame eviction
**Performance Requirements:** Bounded temporal envelope size
**Determinism Requirements:** Temporal envelope updates deterministic given same scene sequence
**Ontology Implications:** Temporal continuity is constructed, not a simulation primitive
**Observer Implications:** May expose temporal envelope state
**Explanation Implications:** Explains anticipation and dread
**Out of Scope:** Full autobiographical narrative generation

## TODO-CONCEPT-001: Sparse Concept Formation
**Status:** Completed
**Phase:** 11
**Priority:** High
**Dependencies:** TODO-COG-001, TODO-SCENE-001, TODO-SCENE-006
**Goal:** RFC acceptance and implementation
**Acceptance Criteria:** Accepted RFC; bounded attended prototype formation, deterministic revision, activation decay, and subjective evidence support implemented
**Performance Requirements:** Attention-driven, not continuous clustering
**Determinism Requirements:** Concept formation deterministic
**Ontology Implications:** Concepts are subjective, not Ground Truth
**Observer Implications:** Exposes concept analytics
**Explanation Implications:** Core to Explanation IR
**Out of Scope:** Semantic concept enums

## TODO-COG-002: Bounded Cognition Model
**Status:** Completed
**Phase:** 12
**Priority:** Medium
**Dependencies:** TODO-COG-001, TODO-SCENE-006
**Goal:** Implement cognitive limits
**Acceptance Criteria:** Working-memory limits, fixed-point belief inertia, subjective source trust, bounded evidence batches, and fallible causal hypotheses implemented
**Performance Requirements:** Sparse updates
**Determinism Requirements:** Cognition deterministic given same state
**Ontology Implications:** Stable mistakes are essential
**Observer Implications:** Exposes cognitive state
**Explanation Implications:** Explains why agents are wrong
**Out of Scope:** Full psychological model

## TODO-LANG-001: Historical Language Bootstrap
**Status:** Completed
**Phase:** 13
**Priority:** High
**Dependencies:** TODO-CONCEPT-001
**Goal:** RFC acceptance and implementation
**Acceptance Criteria:** Accepted RFC; seed-deterministic bounded language and lexeme lineages; opaque phonological units and valid forms; no objective meaning or human-language strings
**Performance Requirements:** Lower-resolution than main simulation
**Determinism Requirements:** Bootstrap deterministic from seed
**Ontology Implications:** Languages are physical patterns, not English strings
**Observer Implications:** Exposes language trees
**Explanation Implications:** Explains word origins
**Out of Scope:** Manual dictionary creation

## TODO-LANG-002: Lexical Innovation
**Status:** Completed
**Phase:** 14
**Priority:** Medium
**Dependencies:** TODO-LANG-001
**Goal:** Implement novel word creation
**Acceptance Criteria:** Deterministic pressure-gated phonotactic coinage; percept-supported bounded adoption history; subjective fixed-point semantic revision
**Performance Requirements:** Deterministic in strict mode
**Determinism Requirements:** Form generation deterministic from inputs
**Ontology Implications:** Words are socially transmitted lineages
**Observer Implications:** Exposes lexeme histories
**Explanation Implications:** Explains neologisms
**Out of Scope:** Full semantic drift simulation

## TODO-PRACTICE-001: Evolvable Practice Representation
**Status:** Completed
**Phase:** 15
**Priority:** High
**Dependencies:** TODO-LANG-002
**Goal:** RFC acceptance and implementation
**Acceptance Criteria:** Accepted RFC; bounded validated operations, conditions, branches, repeats, proposal-only execution, and child lineage mutation
**Performance Requirements:** Efficient execution
**Determinism Requirements:** Practice execution deterministic
**Ontology Implications:** Practices are programs, explanations are separate
**Observer Implications:** Exposes practice lineages
**Explanation Implications:** Explains ritual origins
**Out of Scope:** Full practice diffusion, motor/resource validation, and institutional embedding

## TODO-EPI-001: Measurement and Metrology
**Status:** Completed
**Phase:** 16
**Priority:** Medium
**Dependencies:** TODO-PRACTICE-001
**Goal:** RFC acceptance and implementation
**Acceptance Criteria:** Accepted RFC; opaque socially constructed units, fixed-point precision/uncertainty, bounded calibration ancestry, and accessible-observation measurement
**Performance Requirements:** Minimal overhead
**Determinism Requirements:** Measurements deterministic given same conditions
**Ontology Implications:** Units are socially constructed
**Observer Implications:** Exposes measurement systems
**Explanation IR:** Explains standardization effects
**Out of Scope:** Full instrument, experiment, and science-institution simulation

## TODO-MANA-001: Information-Sensitive Field Model
**Status:** Completed
**Phase:** 17
**Priority:** High
**Dependencies:** TODO-GEO-001
**Goal:** RFC acceptance and implementation
**Acceptance Criteria:** Accepted RFC; bounded fixed-point local field responds to physical recurrence, periodicity, synchronization, and spatial repetition; deterministic diffusion/decay/saturation; proposal-only evolution with per-cell commit traces
**Performance Requirements:** Dense bounded CPU baseline; benchmark before sparse or GPU alternatives
**Determinism Requirements:** Field evolution deterministic
**Ontology Implications:** Mana does not understand meaning
**Observer Implications:** Future read-only numeric field visualization
**Explanation Implications:** Trace-backed cell changes support future causal explanations without semantic inference
**Out of Scope:** Full spell system, physical effects, carrier adapters, attractors, causal resolution, persistence, observer protocol, GPU implementation

## TODO-MANA-002: Mana Cross-Chunk Same-Chart Face Transfer
**Status:** Completed
**Phase:** Detailed Development
**Priority:** High
**Dependencies:** TODO-MANA-001, RFC-GEO-002
**Goal:** Make a same-chart chunk face conduct mana exactly as the interior lattice does, so the chunk grid is not physically visible (INV-037).
**Acceptance Criteria:** Mana evolution processes same-chart cross-chunk faces with compatible extents; each undirected face is handled exactly once; a cell hands the same share across a seam as it hands an in-chunk neighbour; diffusion conserves mana; zero-thermal/mana-only legacy configurations remain deterministic; existing system IDs and RNG streams are unchanged.
**Performance Requirements:** Comparable to current dense stencil; benchmark before claims
**Determinism Requirements:** Identical input configuration and seed produce identical traces
**Ontology Implications:** Chunk boundaries are containment/resolution, not physical barriers (INV-037)
**Observer Implications:** The map may now draw the active set as one continuous field rather than one field per chunk
**Explanation Implications:** Preserve trace-backed cell-change causality across chunk boundaries
**Out of Scope:** Cross-chart transport, new scheduler phase, material response, climate
**Evidence:** The premise this entry originally recorded — a reflecting chunk boundary — was false. `apply_boundary_exchange` has always moved mana across same-chart faces; `chart_boundaries_do_not_cross_by_implicit_integer_adjacency` asserts it. The defect measured was the opposite: the seam conducted 2.58x the interior rate, because it transferred `(left - right) * diffusion / 2` on top of an outgoing budget already distributed in full, while `neighbor_indices` clipped at the chunk edge so a face cell divided its share among five neighbours instead of six (1.20x over-feeding). Separately, the stencil destroyed up to `count - 1` units per cell per tick by subtracting an undivided outgoing budget against truncated incoming shares. Both are fixed; `a_seam_conducts_exactly_as_the_interior_does` and `diffusion_alone_conserves_mana_across_a_seam` hold the behaviour. Re-measured with `apps/observer/src-tauri/examples/extent_bench.rs` on seed 7 at 192 ticks, total mana now rises with the lattice and is flat from extent 12 (32320, 34667, 36950, 38017, 38397, 38397)

## TODO-THERMAL-001: Cross-Chart Thermal Transport
**Status:** Pending
**Phase:** Detailed Development
**Priority:** Medium
**Dependencies:** TODO-THERMAL-000 (completed same-chart slice)
**Goal:** Extend the conserved thermal carrier across registered chart seams using explicit world-geometry transforms.
**Acceptance Criteria:** Thermal energy flows across chart boundaries through registered transforms; each undirected cross-chart face is processed exactly once; conservation residual remains exactly zero.
**Performance Requirements:** Benchmark before scale claims
**Determinism Requirements:** Cross-chart ordering is canonical and seed-independent
**Ontology Implications:** Global geography remains chart-qualified (RFC-GEO-002)
**Observer Implications:** Observer summary aggregates across charts
**Explanation Implications:** Conservation claim continues to report zero residual
**Out of Scope:** Climate, biology, material response, economy

## TODO-THERMAL-002: Thermal-to-Material Coupling
**Status:** Pending
**Phase:** Detailed Development
**Priority:** Medium
**Dependencies:** TODO-THERMAL-000 (completed same-chart slice), TODO-MATER-??? (material response model)
**Goal:** Define and implement a physically meaningful material response to thermal exposure (e.g., retained heat, expansion, damage accumulation, or phase change) without semantic shortcuts.
**Acceptance Criteria:** Material surfaces track bounded thermal exposure state; transitions commit trace-backed events; no "condition +1" semantic toggle.
**Performance Requirements:** Benchmark before claims
**Determinism Requirements:** Material response deterministic given thermal state
**Ontology Implications:** Energy and matter exchange through conserved carriers
**Observer Implications:** Material thermal exposure deltas
**Explanation Implications:** Thermal exposure claims with trace support
**Out of Scope:** Full climate, biology, economy

## TODO-THERMAL-003: Thermal Influence on Mana Field
**Status:** Pending
**Phase:** Detailed Development
**Priority:** Low
**Dependencies:** TODO-THERMAL-000 (completed same-chart slice), TODO-MANA-002
**Goal:** Allow thermal state to feed into mana sample/response pathways as one physical carrier among others.
**Acceptance Criteria:** Mana systems can receive trace-backed thermal samples; no direct thermal-to-mana semantic mapping.
**Performance Requirements:** Benchmark before claims
**Determinism Requirements:** Thermal contribution deterministic
**Ontology Implications:** Mana responds to physical recurrence, including thermal patterns
**Observer Implications:** Optional combined field diagnostics
**Explanation Implications:** Trace support for thermal-mana correlations
**Out of Scope:** Climate, biology, material response

## TODO-THERMAL-004: Climate/Biology Thermal Integration
**Status:** Pending
**Phase:** Detailed Development
**Priority:** Low
**Dependencies:** TODO-THERMAL-001, TODO-THERMAL-002, TODO-BIO-??? (mature physiology)
**Goal:** Couple the thermal carrier to climate and biological thermoregulation through conserved physical carriers.
**Acceptance Criteria:** Climate/biology systems consume and emit thermal energy without direct access to `ThermalFieldSet`; body temperature is subjective/observer-derived, not Ground Truth.
**Performance Requirements:** Benchmark before claims
**Determinism Requirements:** Coupling deterministic
**Ontology Implications:** Biology and climate are causal state, not labels
**Observer Implications:** Body/climate thermal diagnostics
**Explanation Implications:** Thermal exposure and regulation claims
**Out of Scope:** Economy, cross-chart transport

## TODO-THERMAL-005: Experiment-Recipe Thermal Source
**Status:** Pending
**Phase:** Detailed Development
**Priority:** Low
**Dependencies:** TODO-THERMAL-000 (completed same-chart slice)
**Goal:** Allow an experiment recipe to specify a thermal source input analogous to the existing experiment-recipe mana source.
**Acceptance Criteria:** Immutable recipe record commits a root Physics-phase thermal source event; chart-qualified cell transition follows existing thermal pathways.
**Performance Requirements:** Benchmark before claims
**Determinism Requirements:** Source contribution deterministic
**Ontology Implications:** External inputs enter through committed carrier events
**Observer Implications:** Source visible in thermal deltas
**Explanation Implications:** Source trace support in conservation claim
**Out of Scope:** Material response, climate, biology

## TODO-THERMAL-006: Aggregate Conservation-Total Cross-Validation on Snapshot Import
**Status:** Pending
**Phase:** Detailed Development
**Priority:** Medium
**Dependencies:** TODO-THERMAL-000 (completed same-chart slice)
**Goal:** Cross-validate every `ThermalConservationReceipt`'s `total_cell_energy_before`/`total_cell_energy_after` and `total_reservoir_budget_before`/`total_reservoir_budget_after` against checked sums of the actual imported field energies and reservoir budgets, not only the residual and per-cell latest-batch bindings currently enforced.
**Acceptance Criteria:** `RuntimeState::import_snapshot` rejects a snapshot whose reported aggregate totals diverge from the real summed field/reservoir state for any batch, including cells untouched by a transfer receipt; existing V1-V23 thermal contracts and CI remain green.
**Performance Requirements:** Benchmark the added summation cost against import time before acceptance; consider incremental or batch-scoped summation if full-field summation on every import is too costly at scale.
**Determinism Requirements:** Validation must be order-independent and side-effect free.
**Ontology Implications:** None; this is import-path integrity hardening, not a domain contract change.
**Observer Implications:** None.
**Explanation Implications:** None.
**Out of Scope:** Non-latest-batch per-cell bounds checking (closed in `3e46bc2`/follow-up commit) and the documented pre-alpha untrusted-snapshot threat-model carve-out (`SECURITY.md`).
**Context:** Identified during the independent review of `3e46bc2` ("fix(runtime): reconcile thermal receipts and reuse domain geometry"): the per-cell latest-batch binding added there proves touched cells match current field energy, but the conservation receipt's own aggregate summary fields are still trusted literal values from the snapshot with no cross-check against a real recomputed sum, so an untouched cell's energy is never bound to anything.

## TODO-RES-001: Causal Resolution Field
**Status:** Completed
**Phase:** 18
**Priority:** High
**Dependencies:** TODO-MANA-001
**Goal:** RFC acceptance and implementation
**Acceptance Criteria:** Accepted RFC; bounded fixed-point field, opaque trace-backed carrier signals, deterministic decay/weighting/threshold hysteresis, and proposal-only transitions with per-entry commit traces
**Performance Requirements:** Bounded structure-of-arrays CPU baseline; benchmark before alternative layouts
**Determinism Requirements:** Resolution decisions deterministic
**Ontology Implications:** Distance is not resolution
**Observer Implications:** Future read-only numeric resolution and provenance projection
**Explanation Implications:** Trace-backed transitions explain why areas receive detail without semantic inference
**Out of Scope:** Full multi-resolution domain aggregation, promotion/demotion, scheduler integration, persistence, observer protocol, and GPU implementation

## TODO-SOCIAL-001: Organization Primitives
**Status:** Completed
**Phase:** 19
**Priority:** Medium
**Dependencies:** TODO-RES-001
**Goal:** Define organization structures
**Acceptance Criteria:** Bounded trace-backed relations, members/roles, communication links, authority grants, document-backed records, property claims, rules, and practice associations with canonical validation
**Performance Requirements:** Distributed sorted-vector representation with hard capacities; benchmark before layout claims
**Determinism Requirements:** Canonical numeric ordering and input-order-independent validation
**Ontology Implications:** No organization brain
**Observer Implications:** Exposes organizational structure
**Explanation Implications:** Explains institutional beliefs
**Out of Scope:** Full governance, organization cognition, lifecycle mutation, enforcement, observer projection, persistence, and economy

## TODO-ECON-001: Material Flow Contracts
**Status:** Completed
**Phase:** 20
**Priority:** Medium
**Dependencies:** TODO-SOCIAL-001
**Goal:** Define physical economy carrier interfaces without a market shortcut
**Acceptance Criteria:** Bounded typed inventory lots, same-material transfers, input/output transformation ancestry, performed labour contributions, and optional references to contestable ownership claims
**Performance Requirements:** Canonical sorted vectors, bounded nested references, and binary-search validation; benchmark before scale claims
**Determinism Requirements:** Input-order-independent integer records with no RNG, floats, or unordered traversal
**Ontology Implications:** Material substitution preserves differences
**Observer Implications:** Exposes supply chains
**Explanation Implications:** Explains shortages and surpluses
**Out of Scope:** Markets, prices, currency, recipes, scheduling, automatic ownership, committed conservation batches, lifecycle mutation, observer projection, persistence, and benchmarks

## TODO-CITY-001: Infrastructure Networks
**Status:** Completed
**Phase:** 20
**Priority:** Medium
**Dependencies:** TODO-ECON-001
**Goal:** Define city infrastructure
**Acceptance Criteria:** Spatial parcel references, physical buildings with material provenance, and opaque-schema infrastructure nodes/links capable of representing roads, water, sewage, and other networks without semantic enums
**Performance Requirements:** Bounded canonical topology and deterministic outgoing-link traversal; benchmark before scale claims
**Determinism Requirements:** Input-order-independent topology validation; no layout generation or RNG
**Ontology Implications:** Infrastructure creates spatial patterns
**Observer Implications:** Exposes networks
**Explanation Implications:** Explains urban development
**Out of Scope:** Generated settlements, full urban growth, flow physics, traffic, interiors, lifecycle mutation, degradation, maintenance, fire, observer projection, persistence, and benchmarks

## TODO-HIST-001: Causal Historical Bootstrap Orchestration
**Status:** Completed
**Phase:** 21
**Priority:** High
**Dependencies:** TODO-TRACE-001, TODO-RES-001, TODO-LANG-001, TODO-ECON-001, TODO-CITY-001
**Goal:** Define bounded deterministic ordering and provenance contracts for low/high-resolution historical synthesis without generating semantic high-level history.
**Acceptance Criteria:** Canonical stage DAG with opaque process schemas, time spans, numeric detail, spatial targets, parameter fingerprints, stable seed contributions, and exact committed receipt ancestry
**Performance Requirements:** Hard stage/target/dependency/cause bounds; benchmark concrete adapters before scale claims
**Determinism Requirements:** Same seed and canonical plan produce identical stage seeds and validation results independent of input order
**Ontology Implications:** Historical causality is explicit trace structure; wars, plagues, migrations, settlements, and discoveries are not primitive event kinds
**Observer Implications:** Future read-only plan/receipt projection only
**Explanation Implications:** Future explanations traverse committed receipt traces and may not fill gaps narratively
**Out of Scope:** Fake history, residents, cities, event tables, domain synthesis algorithms, aggregation, mutation, scheduler integration, persistence, observer protocol, acceleration, Phase 22 metaphysics

## TODO-ISEKAI-001: Cross-World Transfer Model
**Status:** Completed
**Phase:** 22
**Priority:** Medium
**Dependencies:** TODO-CITY-001
**Goal:** RFC acceptance and implementation
**Acceptance Criteria:** RFC approved, transfer types defined
**Performance Requirements:** N/A
**Determinism Requirements:** Transfer events deterministic from seed
**Ontology Implications:** Arrivals are physical processes
**Observer Implications:** Exposes arrival history
**Explanation Implications:** Explains foreign influence
**Out of Scope:** Final metaphysical model

## TODO-META-001: Identity Persistence Research
**Status:** Completed
**Phase:** 23
**Priority:** Low
**Dependencies:** TODO-ISEKAI-001
**Goal:** RFC acceptance and neutral research contracts
**Acceptance Criteria:** RFC approved; bounded trace-backed observations; multiple opaque weighted criteria; no authoritative identity verdict
**Performance Requirements:** Bounded observation, channel, and criterion counts
**Determinism Requirements:** Canonical fixed-point evaluation
**Ontology Implications:** No primitive Soul object
**Observer Implications:** N/A
**Explanation Implications:** Explains identity concepts
**Out of Scope:** Final identity metaphysics, Soul objects, concrete death/transfer evidence adapters

## TODO-META-002: Stateful Mana Attractors
**Status:** Completed
**Phase:** 23
**Priority:** Low
**Dependencies:** TODO-META-001
**Goal:** RFC acceptance and read-only trajectory research contracts
**Acceptance Criteria:** RFC approved; bounded field observations; numeric stability and recovery evidence; no semantic attractor entity
**Performance Requirements:** Bounded checkpoint and observation counts
**Determinism Requirements:** Canonical integer probe evaluation; wall time excluded
**Ontology Implications:** Gods are emergent, not primitive
**Observer Implications:** N/A
**Explanation Implications:** Explains religious phenomena causally
**Out of Scope:** Gods, spirits, artifacts, agency, field-to-matter effects, final attractor criteria

## TODO-LAB-001: Executable Long-Run Causal Experiment
**Status:** Completed
**Phase:** 24
**Priority:** High
**Dependencies:** TODO-MANA-001, TODO-RES-001, TODO-TRACE-001, TODO-META-002
**Goal:** Run a real bounded headless simulation and strict control/intervention experiment
**Acceptance Criteria:** Runtime commits physical → mana → resolution transitions; errors return from tick; canonical state digest replays exactly; CLI run/lab commands execute; intervention changes trajectory
**Performance Requirements:** Bounded field and checkpoints; report measurement without scale claim
**Determinism Requirements:** Same seed and plan produce identical checkpoints and final state digest
**Ontology Implications:** Experiment labels are non-authoritative; trajectory evidence is not an emergence verdict
**Observer Implications:** CLI diagnostic only; observer protocol unchanged
**Explanation Implications:** Numeric evidence and causal traces enabled the minimal Foundation Phase 25 IR; domain-valid Detailed Development analytics remain required
**Out of Scope:** Fake populations/history, full phenomenon miner, persistence, observer streaming, scale claims

## TODO-EXPLAIN-001: Explanation IR
**Status:** Completed
**Phase:** 25
**Priority:** High
**Dependencies:** TODO-CONCEPT-001
**Goal:** RFC acceptance and implementation
**Acceptance Criteria:** RFC approved, IR supports typed claims, evidence, confidence
**Performance Requirements:** Query latency < 100ms
**Determinism Requirements:** IR generation deterministic from simulation state
**Ontology Implications:** Explanations are non-authoritative
**Observer Implications:** Core observer feature
**Explanation Implications:** Self-referential
**Out of Scope:** LLM surface

## TODO-OBSERVER-001: Protocol Buffer Schemas
**Status:** Completed
**Phase:** 0
**Priority:** High
**Dependencies:** TODO-ARCH-001
**Goal:** Initial protocol schemas defined
**Acceptance Criteria:** 10 proto files created, versioned under v1
**Performance Requirements:** Efficient serialization
**Determinism Requirements:** N/A
**Ontology Implications:** Observer data is derived
**Observer Implications:** Foundation of all observer communication
**Explanation Implications:** N/A
**Out of Scope:** Wire implementation

## TODO-PROTO-001: Wire Protocol Implementation
**Status:** Completed
**Phase:** 0
**Priority:** High
**Dependencies:** TODO-OBSERVER-001
**Goal:** Basic protocol handler
**Acceptance Criteria:** Query/response roundtrip works
**Performance Requirements:** Minimal overhead
**Determinism Requirements:** N/A
**Ontology Implications:** N/A
**Observer Implications:** Enables UI connection
**Explanation Implications:** N/A
**Out of Scope:** Streaming, backpressure

## TODO-UI-001: Tauri Observer Shell
**Status:** Completed
**Phase:** 0
**Priority:** High
**Dependencies:** None
**Goal:** Minimal desktop shell
**Acceptance Criteria:** Shell builds, shows connection status, no fake data
**Performance Requirements:** N/A
**Determinism Requirements:** N/A
**Ontology Implications:** UI is an observer
**Observer Implications:** Core UI infrastructure
**Explanation Implications:** N/A
**Out of Scope:** Simulation connection

## TODO-UI-002: Protocol-Connected Rich Observer
**Status:** Completed
**Phase:** 26
**Priority:** High
**Dependencies:** TODO-UI-001, TODO-PROTO-001, TODO-OBSERVER-002, TODO-EXPLAIN-001
**Goal:** Connect the desktop observer to real bounded simulation data
**Acceptance Criteria:** Tauri 2 shell negotiates observer v1, receives runtime stream snapshots/deltas, queries chart-qualified chunks, renders typed Explanation IR, and contains no fabricated simulation data
**Performance Requirements:** Capacity-one latest-state runtime stream, bounded 96-sample client timeline, hidden world view performs no world refresh queries
**Determinism Requirements:** Same seed/ticks produce identical payloads and locale changes preserve physical/history digests
**Ontology Implications:** Human labels, colors, selected views, and locale remain non-authoritative presentation
**Observer Implications:** First complete external consumer of observer v1
**Explanation Implications:** Preserves schema, numeric value, evidence state, confidence, comparison, checkpoint, and trace count
**Out of Scope:** Entity-per-DOM views, global planetary map, large-dataset WebGPU renderer, agent-known maps, LLM narrative, authoritative mutation

## TODO-UI-003: Observer Instrument Frontend
**Status:** Completed
**Phase:** Detailed Development — Observer Surface
**Priority:** High
**Dependencies:** TODO-UI-002
**Goal:** Replace the bounded first observer shell with a durable analytical instrument: a design system, a component library, a capability-aware application shell, and complete workflows over every capability the current protocol delivers
**Acceptance Criteria:** Five independent areas over runtime summary, chart-qualified chunks, material surface and gate transitions, and Explanation IR; every unavailable observable stated in a capability register rather than omitted; evidence state, confidence, and trace anchors always presented together; no fabricated simulation data in any build
**Performance Requirements:** Canvas rendering for chart surfaces, bounded observer-side buffers (256 summary frames, 120 exchanges), feed demand registry so closed panels issue no queries
**Determinism Requirements:** Locale changes preserve physical/history digests; presentation carries no state
**Ontology Implications:** Signal hues, area names, claim reading notes, and capability labels are observer classifications and never simulation meaning
**Observer Implications:** First consumer of material surface gate deltas; documents required projections in `docs/ui/observer-projection-gaps.md`
**Explanation Implications:** Claim schemas render by schema ID with a generic fallback, so new schemas appear without frontend work; the authoritative Rust renderer is not reimplemented in TypeScript
**Out of Scope:** Trace ancestry navigation, per-cell field plates, entity inspection, historical comparison, streaming subscriptions, WebGPU

## TODO-UI-005: Chart Instrument and Analytical Lenses
**Status:** Completed
**Phase:** Detailed Development — Observer Surface
**Priority:** High
**Dependencies:** TODO-UI-003
**Goal:** Make spatial observation a first-class instrument: an interactive map of the chunk lattice inspected through an extensible lens system that distinguishes measured, partial, constructed and unavailable information
**Acceptance Criteria:** Pan, zoom and spatial selection to cell resolution; multiple meaningfully different lenses over real observer data; combined overlays; scale-aware representation; a lens contract the renderer does not know the domains of; unsurveyed ground drawn rather than blank
**Performance Requirements:** Viewport culling independent of chart size; cached palette and hatch tiles; level of detail bound to legibility
**Determinism Requirements:** Preview constructions are arithmetic over received values only and are clipped to the charted extent
**Ontology Implications:** Lens names and availability labels are observer classifications; preview geometry is not a measurement and is marked wherever it appears
**Observer Implications:** Every unavailable lens states the read model it needs (`docs/ui/observer-projection-gaps.md` §7)
**Explanation Implications:** None; the map is a read surface
**Out of Scope:** Joining charts into a global surface, agent-known perspectives, historical comparison, 3D terrain

## TODO-UI-006: Five-Locale Observer Presentation
**Status:** Completed
**Phase:** Detailed Development — Observer Surface
**Priority:** Medium
**Dependencies:** TODO-UI-003, TODO-UI-005
**Goal:** Present the whole observer surface in five languages without letting presentation reach authoritative state, and without leaving a layer half-translated
**Acceptance Criteria:** `en-US`, `ru-RU`, `zh-Hans`, `de-DE` and `es-ES` cover UI chrome, locale-keyed observer metadata (claim descriptors, coverage register, lens catalogue) and the authoritative Rust Explanation renderer; the English dictionary is the derived baseline so a missing key fails compilation; an explicit choice persists across restarts and a first run follows browser preferences with an English fallback; unsupported tags including traditional Chinese fall back rather than being answered with the wrong script
**Performance Requirements:** Static dictionaries with no runtime fetch; no effect on tick execution or wire encoding
**Determinism Requirements:** INV-007 covered across the full locale set, comparing digests per tick and session payload bytes, not one locale pair
**Ontology Implications:** Human languages remain non-authoritative presentation resources; a locale tag is presentation identity and never simulation identity
**Observer Implications:** Locale travels to the protocol handler on connect and reaches nothing authoritative; the switcher is one meridian cell plus a command-palette entry per language
**Explanation Implications:** Schema labels, evidence states, assessments and comparison contexts are localized in the authoritative Rust renderer; unregistered schemas keep their numeric identity in every locale
**Out of Scope:** Traditional Chinese, right-to-left layout, locale-specific number and date formats beyond `Intl` grouping, translating protocol nouns, logs or domain identifiers

## TODO-OBS-001: Field Raster Projection and Chart Shape
**Status:** Completed
**Phase:** Detailed Development — Observer Surface
**Priority:** High
**Dependencies:** TODO-UI-005
**Goal:** Project the terrain carrier's and the mana field's per-cell lattices to the observer and let the active chunk set form an area, so the map renders measured relief and a measured mana field rather than one aggregate per chunk
**Acceptance Criteria:** A bounded per-chunk `FieldRaster` query covering terrain elevation, terrain roughness and mana intensity, with runtime-computed detail levels for terrain; measured hypsometric tinting, hillshading and contours; a mana field lens whose availability derives from the received lattice edge; a config-gated two-dimensional active chunk shape that leaves every existing fixture digest unchanged
**Performance Requirements:** Per-chunk requests only, delta-encoded elevation, viewport culling and a cache keyed by generation trace; encoded size and paint time measured before any scale claim
**Determinism Requirements:** Read-only for stages 1-5; the active chunk shape defaults to the existing line so no replay fixture moves
**Ontology Implications:** Downsampling and hillshading are presentation reductions and never re-enter the runtime (INV-022); surface materials are excluded until geography generates coherent regions
**Observer Implications:** One additive query kind, one additive Tauri command, protocol stays v1
**Explanation Implications:** None; generation provenance travels with the raster for future claims
**Out of Scope:** Landcover from surface materials, per-cell causal resolution, raising `chunk_extent`, joined charts, terrain generation changes
**Plan:** `plans/observer-field-raster-map.md`
**Evidence:** A `FieldRaster` query kind, its wire codec, a session bound and a TypeScript decoder;
`ActiveChunkShape::Area` behind a config field with `Line` still the default. Measured on the
demonstration session (seed 7, 48 ticks, nine chunks): terrain elevation with its roughness band
encodes to 3 369 bytes per chunk against 8 192 bytes of raw arrays, the mana volume to 181 bytes per
chunk, and the world-chunk snapshot the map already fetched is 1 874 bytes. The map assembles the
received lattices into one field over the surveyed extent and draws hypsometric tinting, hillshading
and measured contours from it; mana availability is derived from the received lattice edge and
therefore reads `preview` at `chunk_extent` 3 and `observed` above it with no frontend change.

Two defects surfaced only once the chart had two dimensions. `chart_chunk_hash` sign-extended each
axis, so `(-1, -1, 0)` collided with `(0, 0, 0)` and the mana cell object identity validator rejected
the first area-shaped runtime; the fix leaves every line-shaped chart with the identity it was
recorded with. `terrain_cells` derives elevation from chunk-local coordinates only, which is recorded
separately as `TODO-GEO-005`.

## TODO-OBS-002: Batched Per-Frame Field Raster Query
**Status:** Pending
**Phase:** Detailed Development — Observer Surface
**Priority:** Medium
**Dependencies:** TODO-OBS-001
**Goal:** Replace the nine-to-eighteen separate `FieldRaster` calls a world frame issues today (one per active chunk per field) with a single query carrying a list of chunks, so a frame pays the bridge's per-call overhead once instead of once per chunk
**Acceptance Criteria:** One additive query variant answering a bounded list of `(chunk, field, detailLevel)` requests in one response, still bounded to the active chunk set (never a chart dump); `refreshRasters` in `apps/observer/src/observer/session.ts` issues one call per frame; every existing fixture, replay capture and digest is unchanged, since this only changes how already-computed per-chunk rasters are transported, not what they contain
**Performance Requirements:** Before-and-after wall-clock span of a full frame, measured with the corrected Instrument log (see below), not a byte-count estimate
**Determinism Requirements:** Read-only; no runtime state or digest is touched
**Ontology Implications:** None; this is a transport shape, not a domain contract
**Observer Implications:** One additive query kind; existing per-chunk `FieldRaster` stays for callers that want a single chunk
**Explanation Implications:** None
**Out of Scope:** Raising `chunk_extent`, joined charts, a landcover lens
**Evidence:** The Instrument log folded concurrent per-frame `observer_field_raster` exchanges by summing their individual `durationMs`, and each individual duration already included the wait for every call ahead of it on the session's Tauri mutex — the fold therefore reported a batch's cost roughly as the square of its size rather than its real span (fixed in `apps/observer/src/observer/session.ts`, `recordExchange`). Re-measured against a real desktop session (seed 0, debug build) with the fold bypassed: consecutive per-call completions land 3-5 ms apart regardless of payload — a 113-byte mana raster and a ~3.4 KB terrain raster cost about the same marginal time. Fixed per-call overhead (mutex acquisition, command dispatch) dominates in this range; payload size barely registers. Switching every observer command's response from a `Vec<u8>` return (serialised by Tauri as a JSON array of numbers, four times the wire bytes for the elevation raster) to `tauri::ipc::Response` (raw bytes, confirmed to arrive as an `ArrayBuffer` on the JS side) did not move this measurement at these payload sizes, which is consistent with fixed overhead rather than serialisation cost dominating today — but it removes a real inefficiency that will matter once payloads grow, including under this TODO's own batched response. Batching removes that fixed overhead (n-1) times per frame; it does not remove per-byte cost, which the same measurement shows is not the bottleneck at current sizes

## TODO-GEO-004: Coherent Surface Material Regions
**Status:** Pending
**Phase:** Detailed Development — Geography
**Priority:** Medium
**Dependencies:** TODO-OBS-001
**Goal:** Generate surface materials as spatially coherent regions rather than per-cell independent assignments
**Acceptance Criteria:** Measured same-material neighbour rate substantially above the chance rate for the material count; regions are deterministic and reproducible from the world seed
**Performance Requirements:** Generation cost measured against the current per-cell assignment
**Determinism Requirements:** Same seed produces identical material fields
**Ontology Implications:** Material regions are authoritative geography, not an observer classification
**Observer Implications:** Unblocks a landcover lens, which `plans/observer-field-raster-map.md` deliberately excludes today
**Explanation Implications:** Material regions become available as causal context for surface claims
**Out of Scope:** Biome semantics, climate coupling, named regions
**Evidence:** `apps/observer/src-tauri/examples/terrain_probe.rs` measures 6.5% same-material neighbours against 6.2% expected from chance over 16 materials. `TODO-MANA-004` reaches the same finding from the mana side and puts a cost on it: projected onto the mana lattice, the terrain's structural variation survives at only 1.32x to 2.75x what averaging pure noise would retain, and the ratio falls as the lattice refines. There is little coherent structure for a finer field to resolve, so this is the work that would make a finer mana lattice worth its cost rather than the other way round

## TODO-GEO-005: Terrain Continuity Across Chunk Boundaries
**Status:** Pending
**Phase:** Detailed Development — Geography
**Priority:** Medium
**Dependencies:** None
**Goal:** Generate elevation as a function of a cell's position in its chart rather than of its position in its chunk, so adjacent chunks meet
**Acceptance Criteria:** Measured elevation step across a chunk boundary is of the same order as the step between neighbouring cells inside a chunk; the field stays deterministic from the world seed
**Performance Requirements:** Generation cost measured against the current per-chunk assignment
**Determinism Requirements:** Changing terrain changes state hashes by construction, so it ships with regenerated fixtures and replay evidence
**Ontology Implications:** Terrain is authoritative geography; continuity between chunks is a property of the world, not of the drawing
**Observer Implications:** The relief lens already draws the discontinuity and states that the step is world state rather than a seam. With this closed, terrain contours become a defensible default overlay, which they are not today
**Explanation Implications:** None
**Out of Scope:** Landforms, erosion, hydrology, biomes, raising the terrain lattice
**Evidence:** `terrain_cells` in `crates/causafera-runtime/src/carrier.rs` computes `ridge = (x - y) * 17` from chunk-local `x` and `y` and takes the chunk only through the seed, so every chunk repeats the same diagonal ridge. Measured on the demonstration session at seed 7: the east edge of chunk (−1, 0) reads +13.1 m … +19.5 m against −13.5 m … −13.7 m on the abutting west edge of chunk (0, 0), a step of about thirty metres where the mean neighbour step inside a chunk is 1.6 m. `TODO-OBS-001` made this visible by giving the chart two dimensions and a per-cell projection; before that the map drew one tint per chunk and the strip was one chunk deep, so nothing showed it

## TODO-MANA-004: Mana Field Lattice Cost Decision
**Status:** Completed — `chunk_extent` stays 3
**Phase:** Detailed Development — Mana
**Priority:** Medium
**Dependencies:** TODO-OBS-001, TODO-RUNTIME-002
**Goal:** Decide whether the mana field should run at a finer lattice than `chunk_extent` 3, on measured evidence. The accuracy argument was stated to lead, on the reading that the default lattice underestimated total mana by 15.8% against a value the finer lattices converged on. That criterion did not survive `TODO-RUNTIME-002` — see the decision below
**Acceptance Criteria:** A decision recorded either way with its reasoning, against the measurements already taken in `plans/observer-field-raster-map.md`; if the extent rises, regenerated fixtures and replay evidence ship with it
**Performance Requirements:** The mana volume grows with the cube of the extent — 3 to 8 is a factor of nineteen, 3 to 32 a factor of 1214 — so no extent change is accepted without measurements
**Determinism Requirements:** Changing the extent changes state hashes by construction; any change ships with regenerated fixtures and replay evidence
**Ontology Implications:** The field is already fully implemented and every cell is live with per-cell provenance; only its spatial lattice is coarse
**Observer Implications:** The map's mana lens reports `preview` while the lattice needs upsampling and `observed` when it does not, so a finer lattice improves the map with no frontend change. The decision below leaves the lens on `preview`
**Explanation Implications:** A finer lattice gives mana claims finer spatial evidence
**Out of Scope:** Changing mana propagation rules, response parameters, or the gate model
**Decision:** `chunk_extent` stays 3. Nothing is applied, so no fixture or replay evidence needs regenerating.

The question this TODO was written to answer stopped existing when `TODO-RUNTIME-002` landed. `chunk_extent` used to be a pure discretisation parameter — one world read on a coarser or finer lattice — so "total mana converges from extent 12" was a convergence claim and extent 3 being 15.8% below it was an error estimate. The standing terrain carrier presents itself at the mana lattice's own resolution, one sample per plan-view column, so the extent now also sets how finely the field reads the ground. A finer lattice is not the same physics at higher resolution, it is more physical input: total mana runs 82 679, 154 901, 926 121, 2 158 238 and 8 117 028 at extents 3, 4, 6, 8 and 12 on seed 7. Comparing a candidate field against a finer reference field therefore measures two things at once and neither separately, and the old convergence table has no interpretation. `extent_decision.rs` was rewritten around three questions that do have answers, and all three came back against a finer lattice.

*Fidelity.* Measured against extent 32, which is one column per terrain cell and so the exact reading, the share of the terrain's own structural variation that survives projection is 1.9%, 4.3%, 6.6%, 9.8% and 18.6% at extents 3, 4, 6, 8 and 12. Those look like an argument for refining until the ratio against chance is taken: a spatially incoherent terrain retains 1/(cells per column) by averaging alone, and the measured retention is 2.20x, 2.75x, 1.87x, 1.56x and 1.32x that floor. It falls toward 1.0. Whatever coherent structure this terrain has is already captured at extent 3 and 4; what a finer lattice resolves is cell-scale noise. That is the same finding `TODO-GEO-004` records from the other side — surface materials are assigned per cell with a same-material neighbour rate of 6.5% against 6.2% by chance — and it is why the lattice is not the parameter to change. Distinct column fingerprints do rise, 7.5, 11.5, 17.8, 26.5 and 39.0 against 80.0 at the exact reading, averaged over six seeds.

*Discrimination.* After `TODO-RUNTIME-002` the seed reaches the simulation, so a lattice can be judged on whether a world at that lattice can still be told apart from another. It gets worse with refinement, not better. Distinct behaviour tuples across the six seeds are 3, 3, 1, 2 and 1 at extents 3, 4, 6, 8 and 12: at extent 6 and 12 all six worlds produce identical gate crossings, gate transitions, surface conditions, actions and population. Physical digests and total mana stay six-distinct throughout, so the worlds do differ in state — what stops discriminating is the gate. The mechanism is measured rather than inferred: the mean live cell holds 1021, 807, 1429, 1588 and 2349 against a gate threshold of 4 096, and the share of live cells above the gate runs 0%, 0%, 8%, 11% and 24%. A finer lattice drives more of the field past the threshold, so gates latch open in the first ticks and never fall back, gate crossings drop to exactly 3 from extent 6 upward and transitions from 4–8 to 3. The response and gate constants were calibrated against a field no carrier populated, which is now recorded as `TODO-MANA-007`; until that is settled a finer lattice buys a less responsive simulation.

*Cost.* On seed 7 at 192 ticks: 1.346, 2.677, 7.981, 15.149 and 39.037 ms per tick, or 1.0x, 2.0x, 5.9x, 11.3x and 29.0x. The dominant term is provenance, not arithmetic — one committed causal event per changed mana cell per tick, with 74, 170, 554, 1043 and 2620 cells changing per tick and 17 064 to 506 193 events committed over the run.

The prior decision of extent 6 is void, along with its stated 98.1% convergence and 94% cost. Both were measured on a field that no carrier populated, against a convergence criterion that no longer means anything, and every number in them has been superseded above. Recorded below as it stood.

**Superseded decision:** `chunk_extent` 6 as the production default, 8 as a reference/high-fidelity profile, 3 and 4 for fast tests only. Extent 6 reaches 98.1% of the value the lattice converges to, against 89.9% at the current default, and cuts neighbour variation from 0.73 to 0.29, for 94% more tick cost; the remaining 1.9 points cost a further 31% of tick time and 2.37x the cells. Not yet applied: the change was conditioned on validating across several seeds and confirming no behavioural threshold moves, and `TODO-RUNTIME-002` shows the seed does not vary the simulation at all, so that validation cannot be performed today. `extent_decision.rs` does show the local error falling with the lattice — against the extent 12 reference, the plan-view absolute error is 126.7%, 113.2% and 77.5% at extents 3, 4 and 6, and the shape error 66.2%, 58.7% and 39.6% — and that gate crossings, surface transitions, actions and population are identical at every extent, so no behavioural threshold moves on the one world that currently exists
**Decision status after `TODO-RUNTIME-002`:** Both halves of this decision now rest on measurements taken against a nearly empty field, and the field is no longer nearly empty. The blocking condition is gone — seeds now produce different worlds, so multi-seed validation is possible — but the cost side has moved sharply against a finer lattice. Measured in `plans/terrain-carrier-participation.md` with terrain standing, extent 6 costs 9.050 ms per tick against 2.563 ms with terrain inert, and extent 12 costs 45.386 ms against 6.788 ms. The cause is not the carrier, whose own overhead is single-digit percent at the default lattice: terrain populates cells that were previously dead, and the runtime commits one causal event per changed mana cell per tick, so a finer lattice now prices a genuinely live volume rather than a mostly empty one. The convergence and local-error measurements below were also taken on the old, contact-only field and should be re-taken before the extent is changed
**Evidence:** `apps/observer/src-tauri/examples/extent_decision.rs`, rewritten for the questions the decision above turns on, and `apps/observer/src-tauri/examples/seed_variation.rs` for the standing-carrier cost. Six seeds, 192 ticks, the production gate, extents 3, 4, 6, 8 and 12 against an exact reading at extent 32. Every figure quoted in the decision comes from one run of that tool; the terrain-fidelity half needs no runtime at all, since it reads the carrier's own projection directly.

**Superseded evidence:** `apps/observer/src-tauri/examples/extent_bench.rs`, re-measured on seed 7 at 192 ticks after `TODO-MANA-002`, on a field no carrier populated. Total mana rises with the lattice and is flat from extent 12 (34 667, 36 950, 38 017, 38 397, 38 397) while extent 3 gives 32 320 — 15.8% low. Separately, the same tool measures the plan-view column field the map draws. Extent 4 gives 78% more columns at full coverage for 57% more tick cost; extent 6 quadruples detail at 88.9% coverage; extent 16 leaves 17.4% coverage at ten times the tick cost. Open question: the smallest non-zero cell falls from 66 at extent 3 to 1 from extent 6 upward, so at fine lattices part of the field sits at the quantisation floor and integer flooring does start to bite

## TODO-MANA-005: Cross-Chunk Seam Mana Changes Without Causal Ancestry
**Status:** Completed
**Phase:** Detailed Development — Mana
**Priority:** High
**Dependencies:** TODO-MANA-002
**Goal:** Give every committed mana cell change a cause, so no authoritative mana state can appear without causal ancestry
**Acceptance Criteria:** No committed mana event has an empty cause list in any configuration; the seam test asserts provenance and not only intensity
**Performance Requirements:** None beyond the existing per-tick mana cost; this adds anchors, not work
**Determinism Requirements:** Closing the gap changes the committed events, so it changes the history digest of every world. It was stated here that the physical digest would change too; measured, it does not. Causes enter `history_digest` and not `physical_state_digest`, which covers state and not ancestry, and no intensity moves. Seed 7 at 192 ticks: history digest `40d180cdd41a` becomes `3678584751b8`, while the physical digest stays `1d4f6c4194ee`, total mana stays 82 679, and gate crossings, gate transitions, surface conditions, actions and population are unmoved at 4, 6, 252, 245 and 493. All six seeds behave the same way
**Ontology Implications:** A mana cell that changed for no recorded reason is authoritative state without provenance, which is the one thing the field model's proposal/commit boundary exists to prevent
**Observer Implications:** None; the affected cells are already projected, only their anchors are missing
**Explanation Implications:** Blocks any general claim that a mana change is attributable to the carrier that drove it. `plans/terrain-carrier-participation.md` had to narrow its Explanation section for exactly this reason
**Out of Scope:** Changing diffusion, decay, the stencil, response parameters, or the gate model
**Resolution:** The exchange now carries what actually produced the value that crossed — this tick's injection causes as well as the source cell's previous change — and the receiving side contributes its own previous change and injection. `ManaEvolutionProposal` retains the injection causes sparsely for that purpose, since only cells a sample landed on have any. `propose_evolution` then fails closed with `ManaError::UnattributedChange` on any change that still has no cause, at both the field and the field-set boundary, so an unattributed transition is reported instead of committed. That guarantee rests on a cell holding mana only because some commit put it there: `ManaField::new` starts every cell at zero and untraced, and every commit writes `last_change` for every cell it moves. A field imported in violation of that is now rejected rather than silently spreading what it cannot attribute. Covered by `crates/causafera-runtime/tests/mana_provenance.rs`, which asserts no Mana-phase event other than the experiment-recipe root claims no cause, under standing and inert terrain alike, and by `a_cell_fed_across_a_seam_records_what_crossed_it` and `an_untraced_cell_holding_mana_cannot_be_evolved` in `causafera-domains`. Both tests were checked against the defect: reverting the injection causes reproduces exactly the four causeless events on seed 7 recorded below
**Evidence:** `apply_boundary_exchange` passed `[Option<TraceId>; 2]` built only from the two participants' `last_change`, and `apply_exchange_delta` flattened it, so a seam cell whose participants both lacked a `last_change` yielded a nonzero `ManaCellChange` with an empty `causes`, and `ManaRuntimeSystem::execute` committed it as an ordinary mana event. The participants of a freshly injected cell always lack one: `last_change` records the previous tick's change and injection happens in this one. Measured over 24 ticks with four actors and a bootstrap population of 64: zero causeless mana events at `active_chunk_radius` 0 (one chunk, no seams), three at radius 1 with the terrain carrier inert and four with it standing — so the defective mechanism is the cross-chunk seam and terrain is not its source. The counts do not establish a bound on how often the standing path reaches it — four against three is one measurement, not a proven ceiling. The seam test `a_seam_conducts_exactly_as_the_interior_does` (`crates/causafera-domains/src/mana.rs`) exercises the state but asserts intensity only. `causafera-domains` is untouched by `TODO-RUNTIME-002`; this predates it

## TODO-MANA-006: Seam Delivery Ignores the Saturation Ceiling
**Status:** Pending
**Phase:** Detailed Development — Mana
**Priority:** Medium
**Dependencies:** TODO-MANA-002
**Goal:** Make `maximum_intensity` bound a cell that is fed across a chunk seam exactly as it bounds a cell fed from inside its own chunk
**Acceptance Criteria:** No proposed cell intensity exceeds `maximum_intensity` in any configuration, and whatever the ceiling refuses is accounted for rather than silently created or destroyed
**Performance Requirements:** None beyond the existing per-tick mana cost
**Determinism Requirements:** Any fix changes intensities and therefore the physical digest of a saturated world; it ships with regenerated replay evidence
**Ontology Implications:** `maximum_intensity` is a stated property of the field, not of the interior of a chunk. A ceiling that a cell escapes by sitting next to a seam makes the chunk grid physically visible, which is what INV-037 forbids
**Observer Implications:** The map's mana lens can currently show a cell above the field's own stated maximum, so a normalised colour ramp has no reliable upper bound
**Explanation Implications:** A saturation claim cannot be made about a seam cell
**Out of Scope:** Changing the stencil, the response channels, decay, or the gate model
**Evidence:** `diffuse_cell` clamps to `0..=maximum_intensity`, but `apply_exchange_delta` adds its delivered share to the already-clamped value with no ceiling of its own. Measured on two adjacent extent-3 chunks with every cell seeded at a `maximum_intensity` of 1 000: ten of the 54 cells finish above it, worst 1 034, or 3.4% over. The straightforward fix is not a clamp: the giving cell has already parted with the share inside its own stencil, so discarding the excess destroys mana and reopens the conservation defect `TODO-MANA-002` closed. Refusing the delivery and leaving the amount with the giver needs an ordering-independent rule for which of several givers is refused, which is a design decision rather than a one-line bound. Unreached in the worlds measured so far: at the configured ceiling of 1 000 000, seed 7's 82 679 total mana across 81 cells would be 8.3% of the cap even if every unit sat in one cell

## TODO-MANA-007: Response and Gate Constants Calibrated Against an Unpopulated Field
**Status:** Pending
**Phase:** Detailed Development — Mana
**Priority:** High
**Dependencies:** TODO-RUNTIME-002
**Goal:** Recalibrate the mana response channels and the local effect gate against the field the simulation actually has, so the gate discriminates between worlds instead of latching
**Acceptance Criteria:** Across six seeds at the production configuration, gate crossings and gate transitions distinguish worlds rather than collapsing onto one tuple, and they still do so at more than one lattice; a stated operating point for where the field sits relative to the threshold, with the measurement that establishes it
**Performance Requirements:** None beyond the existing per-tick mana cost; this changes constants, not work
**Determinism Requirements:** Changing the constants changes every world by construction; the change ships with regenerated replay evidence
**Ontology Implications:** The gate is a bounded physical threshold, so its calibration point is a claim about the field's scale. A threshold the field permanently exceeds is not a threshold
**Observer Implications:** The map's mana lens normalises against a field whose scale moved by two orders of magnitude between lattices, so any fixed colour ramp is currently arbitrary
**Explanation Implications:** A gate transition is only evidence of something happening if the gate can also not fire
**Out of Scope:** Changing the stencil, the diffusion or decay rules, the lattice, or which carriers participate
**Evidence:** `apps/observer/src-tauri/examples/extent_decision.rs`, six seeds at 192 ticks. The constants predate any carrier that populates the field: with contact alone, seed 7 held 32 266 total mana; with standing terrain at the same lattice it holds 82 679, and at extent 12 it holds 8 117 028. Against a fixed threshold of 4 096 the share of live cells above the gate runs 0%, 0%, 8%, 11% and 24% at extents 3, 4, 6, 8 and 12, and distinct behaviour tuples across the six seeds run 3, 3, 1, 2 and 1 — the finer the lattice, the more completely the gate latches open in the first ticks and the less it can tell one world from another. Gate crossings sit at exactly 3 from extent 6 upward, which is the number of contacted surfaces rather than a response to anything. The gate is therefore the binding constraint on `TODO-MANA-004`, which was closed by keeping the lattice where the current calibration still works

## TODO-MANA-003: Isotropic Diffusion Kernel
**Status:** Completed
**Phase:** Detailed Development — Mana
**Priority:** Medium
**Dependencies:** TODO-MANA-002
**Goal:** Make mana diffusion approximately isotropic, so a point source spreads as a sphere rather than along the lattice axes
**Acceptance Criteria:** A single source in an empty field produces a distribution whose iso-surfaces are measurably closer to spherical than to the current octahedron; measured with an axis-versus-diagonal spread ratio
**Performance Requirements:** The larger stencil's cost measured against the current six-neighbour form
**Determinism Requirements:** Weights are exact rationals or fixed-point; no floating point in the authoritative path
**Ontology Implications:** The lattice is a discretisation of continuous space; a field that spreads faster along axes than diagonals makes the discretisation physically visible
**Observer Implications:** Isolines on the map become round rather than diamond-shaped
**Explanation Implications:** Spatial mana claims stop depending on lattice orientation
**Out of Scope:** Changing the lattice geometry itself; see the decision recorded in `plans/observer-field-raster-map.md`
**Evidence:** The six-neighbour stencil propagated on the L1 ball. Replaced by an eighteen-neighbour stencil weighting the six faces 2 and the twelve edges 1, which is the smallest exact-integer solution of the fourth-order isotropy condition `f == 2e + 8c`; corners are dropped so the stencil never reaches a diagonally opposite chunk. Measured by the fourth-moment ratio `<x^4> / 3<x^2 y^2>`, which is 1.0 for a round distribution, the old stencil reads 1.28 and the new one 1.00 (`a_point_source_spreads_round_rather_than_along_the_axes`). Tick cost at `chunk_extent` 3 rises from 0.99 ms to 1.29 ms

## TODO-WORLD-002: Chunk Activation Beyond Bootstrap
**Status:** Pending
**Phase:** Detailed Development — World
**Priority:** Medium
**Dependencies:** None
**Goal:** Allow the active chunk set to change during a run, so the world is not permanently exactly the chunks chosen at bootstrap
**Acceptance Criteria:** A chunk can be activated and deactivated during a run with canonical ordering; deactivation preserves conserved quantities; replay-identical
**Performance Requirements:** Activation cost measured; the active set stays bounded
**Determinism Requirements:** Activation is driven by simulation state, never by observer attention (INV-013)
**Ontology Implications:** Activation is a resolution decision and may change detail, never topology or geometry (INV-037)
**Observer Implications:** The chart extent becomes something that changes during a run, which the map already handles — it culls by viewport and draws unsurveyed ground beyond the received extent
**Explanation Implications:** None
**Out of Scope:** Observer-driven activation, cross-chart activation
**Evidence:** `RuntimeConfig::validate` builds the active set once from `active_chunk_keys`; no activation or deactivation path exists in the runtime

## TODO-UI-004: Observer Projection Requests
**Status:** Pending
**Phase:** Detailed Development — Observer Surface
**Priority:** Medium
**Dependencies:** TODO-UI-003
**Goal:** Deliver the observer projections the frontend is waiting on, in the order recorded in `docs/ui/observer-projection-gaps.md`
**Acceptance Criteria:** Rendered explanation text transported rather than reimplemented; resolution policy thresholds projected; bounded trace ancestry query available; per-cell mana and resolution projection with an explicit bounding contract; `PerformanceMetrics` encoded
**Performance Requirements:** Every new projection is explicitly bounded; no unbounded observer queue
**Determinism Requirements:** New payloads are locale-invariant and reproduce identically for the same state
**Ontology Implications:** Entity projection must not expose Ground Truth identity to agents or to observer classification feedback (INV-013, INV-027)
**Observer Implications:** Each item requires an ExecPlan for the protocol change
**Explanation Implications:** Rendered text remains non-authoritative and carries evidence state alongside it
**Out of Scope:** Entity summaries pending a contract decision, historical queries pending persistence maturity

## TODO-DEPTH-001: Detailed Domain Maturity Audit and Sequencing
**Status:** Pending
**Phase:** Detailed Development — Program
**Priority:** Critical
**Dependencies:** TODO-UI-002, TODO-PERSIST-001, TODO-LAB-001
**Goal:** Split every broad domain into evidence-bearing capabilities, assign provisional M0–M5 maturity, identify production placeholders and missing couplings, and propose the first bounded Detailed Development ExecPlans
**Acceptance Criteria:** Every domain in the coverage matrix has capability-level authoritative state, mutation owner, incoming/outgoing carriers, resolution, persistence, provenance, observer, Explanation, performance, and negative-control gaps; dependencies are canonical; no final phase number is reserved
**Performance Requirements:** Audit records representative workload requirements for every capability targeting M5
**Determinism Requirements:** Every planned mutation path identifies RNG streams, canonical ordering, replay, and save/resume gates
**Ontology Implications:** Deepening mechanisms must not promote observer or human semantic categories into authoritative primitives
**Observer Implications:** Identifies inspection contracts needed for validation; does not require a UI panel per capability
**Explanation Implications:** No capability may target M4/M5 without typed metrics, uncertainty, traces, and counterfactual/negative-control strategy
**Out of Scope:** Implementing every domain in one batch, fixed final phase count, optional LLM integration

**Current note:** The frozen `26026fb3862e` audit is paused after completed Todos 1–4. Its
unfinished deep audit is not a prerequisite for the bounded actor/material/mana implementation
slice; future maturity claims require current-HEAD evidence.

## TODO-SIM-001: Durable Physical State and Cross-Domain Coupling
**Status:** In Progress
**Phase:** Detailed Development — Simulation Core
**Priority:** Critical
**Dependencies:** TODO-DEPTH-001, TODO-TRACE-001, TODO-MANA-001, TODO-PERSIST-001
**Goal:** Replace counter/sample-only physical feedback with durable authoritative physical changes owned by explicit domain systems
**Acceptance Criteria:** At least one accepted vertical slice commits durable material, terrain, geometry, body, or other physical property changes; mana and actor actions affect that state through causal proposals; later perception/carrier samples read the changed state; negative controls and persistence/replay equivalence pass
**Performance Requirements:** Representative active-set workload with time, memory, provenance growth, and observer-off overhead measurements
**Determinism Requirements:** Canonical proposal reduction and same-seed replay across uninterrupted and resumed runs
**Ontology Implications:** Effects target structural physical properties, never semantic event labels or meanings
**Observer Implications:** Bounded property deltas and supporting traces are inspectable
**Explanation Implications:** Typed before/after values and causal attribution replace digest-byte proximity
**Out of Scope:** Fake weather, fake settlements, gameplay mechanics, or unbounded global voxel physics

**Current evidence:** The completed actor/material/mana slice adds chart-qualified authoritative
material surfaces, Action- and Mana-phase provenance commits (including the persisted mana gate),
canonical repeated samples, and an accessible physical signal → subjective-scene → later-action
production test. It has snapshot/replay/save-resume coverage, material codec/version-rejection
coverage, parent-before-child provenance validation, no-mana/no-repetition controls, retained
newest-mana observation evidence, and a bounded release diagnostic. Final repository validation,
five independent review lanes, and a focused runtime failure-mode audit passed; this completes the
accepted actor/material/mana slice but not the broader TODO.

The completed local mana-material-surface coupling slice replaces the global mana-total gate with
per-surface local hysteresis gates that retain cell transition priors, emit bounded observer deltas,
and produce typed explanation claims with trace-backed evidence. Fail-closed validation, eviction-safe
gate history, and contact-aware condition history are verified.

## TODO-RUNTIME-001: Production Bootstrap and Fixture Elimination
**Status:** In Progress
**Phase:** Detailed Development — Runtime
**Priority:** Critical
**Dependencies:** TODO-DEPTH-001, TODO-HIST-001, TODO-PERSIST-001
**Goal:** Ensure all production residents and authoritative initial state originate from accepted causal generation/bootstrap paths
**Acceptance Criteria:** Production runtime/session code contains no fixture/demo constructors; aggregate population conservation includes promoted actors; actor/body/object state has bootstrap ancestry; reset, experiment, save/resume, and observer sessions use the same production recipe; tests prove fixtures remain test-only
**Performance Requirements:** Bootstrap time and memory measured for the accepted bounded envelope
**Determinism Requirements:** Same seed and bootstrap plan produce identical state/history digests and ancestry
**Ontology Implications:** No invented residents, histories, languages, settlements, or semantic roles
**Observer Implications:** Bootstrap receipts and promotion/demotion summaries are inspectable without exposing Ground Truth to agents
**Explanation Implications:** Initial conditions and later outcomes can be traced to real bootstrap receipts
**Out of Scope:** Authored lore, demo worlds, or random high-level history tables

**Current evidence:** The completed slice constructs material surfaces through `HistoricalBootstrapPlan`
and its integration tests exercise the production runtime path rather than fixture construction.
This does not yet establish fixture elimination or production-bootstrap coverage for every runtime
capability.

## TODO-RUNTIME-002: The World Seed Does Not Vary the Simulation
**Status:** Completed — see `plans/terrain-carrier-participation.md`
**Phase:** Detailed Development — Runtime
**Priority:** Critical
**Dependencies:** TODO-RUNTIME-001
**Goal:** Make the world seed reach the running simulation, so two seeds produce two different worlds rather than one world with two terrains
**Acceptance Criteria:** Different `RuntimeConfig::new(seed)` values produce different physical state digests, mana fields and behavioural counts after the same tick count; the terrain carrier's samples participate in the tick loop or the reason they do not is recorded as a deliberate contract
**Performance Requirements:** Measured against the current tick cost. Stated when the carrier's only emission shape was one sample per cell per active chunk, which is 1024 per chunk; the delivered contract projects onto the mana lattice instead and emits `chunk_extent²` per chunk, which is 9 at the default. That is a change of emission shape, not a relaxation of the measurement requirement — see the measured cost under Evidence
**Determinism Requirements:** Unchanged — the same seed must still reproduce identically; this is about different seeds differing, not about the same seed varying
**Ontology Implications:** Terrain is authoritative world state. A carrier that is generated, persisted and projected but never causally consumed is world content that exists without participating
**Observer Implications:** Every measurement taken on "seed 7" is currently a measurement of the only world there is
**Explanation Implications:** No terrain fact can appear in a causal explanation while terrain feeds nothing
**Out of Scope:** Changing terrain generation, adding new carriers, cross-chart transport
**Resolution:** The terrain carrier now participates in the tick loop as a standing spatial structure. It emits one sample per plan-view column of the mana lattice on every tick the physical pattern schedule emits, with a magnitude equal to the column's mean terrain structure — relief contrast, material discontinuity and roughness — and a fingerprint of the column's dominant material and roughness class. Those samples reach `pending_samples` only: they are deliberately kept out of `pattern_history` and out of `physical_events`, because both retain change and a structure that is merely still there has not happened again. Participation is a persisted `RuntimeConfig::terrain_participation` contract with `Standing` as the default and `Inert` retaining the prior behaviour; the runtime recipe snapshot section major rises from 4 to 5. Two defects were fixed on the way: `TerrainBootstrapStage` had hard-coded `field_extent` 3 over the configured `chunk_extent`, and the position function shared by both carriers decomposed a mana cell index as a `CHUNK_SIZE`-wide raster
**Original evidence:** `apps/observer/src-tauri/examples/extent_decision.rs` reports byte-identical totals, gate crossings, action counts and populations across seeds 7, 11, 23, 41, 59 and 97, and the physical state digest is identical across seeds after 48 ticks. Terrain itself does vary — `deterministic_terrain_chunk` gives different elevations and 815 against 805 distinct patterns for seeds 7 and 11 — but `PhysicalPatternSystem::execute` fills `pending_samples` only from `MaterialSurfaceCarrierAdapter`, so `TerrainCarrierAdapter::emit_samples` is never called in the tick loop. `carrier_adapters` is read only by bootstrap, resolution relevance and the snapshot export. The seed reaches terrain and terrain reaches nothing
**Evidence:** `apps/observer/src-tauri/examples/seed_variation.rs`, 192 ticks, six seeds, production-shaped configuration. Before: one physical digest, one total mana of 32 266, one behaviour tuple. After: six distinct physical digests, six distinct total manas from 50 416 to 116 189, and three distinct behaviour tuples — mana gate crossings of 3, 4 and 5, gate transition counts of 4, 6 and 8, and total surface conditions of 251, 252 and 253. The same seed still reproduces identically, and an inert world resumes inert across a snapshot. Cost at the default `chunk_extent` 3 is **single-digit percent** per tick — one run gives 1.351 ms to 1.445 ms, and six runs of the same measurement give 7.0%, 7.5%, 7.6%, 8.7%, 9.4% and 17.3%, so the figure is timing-noise sensitive and should not be quoted to one decimal. The structural counts are exact and identical on every run: 27 samples per tick, and a changed-cell count unmoved at 72 against 74. Above the default the growth is not the carrier but the field it populates: at extent 12 the standing world changes 2731 cells per tick against 134 inert, and the runtime commits one causal event per changed cell, yet the cost per changed cell falls from 0.076 ms to 0.052 ms. Two designs were measured and rejected — one sample per terrain cell per tick over-samples the lattice a hundredfold, and retaining terrain in `pattern_history` let the recurrence and periodicity channels score the carrier's read cadence, running total mana to 696 573 and collapsing gate transitions from 32 to 3 over 768 ticks. A one-shot emission at bootstrap was also rejected: gate transitions reconverged to an identical sequence after tick 30 of 768, so the seed shaped the transient and not the world

## TODO-OBSERVER-003: Bounded Causal and Domain Inspection
**Status:** In Progress
**Phase:** Detailed Development — Observer
**Priority:** High
**Dependencies:** TODO-DEPTH-001, TODO-OBSERVER-002, TODO-TRACE-001
**Goal:** Expose the evidence required to understand and validate detailed simulation behavior without coupling UI to runtime internals
**Acceptance Criteria:** Versioned bounded queries provide causal event slices, typed domain state series/deltas, resolution transitions, bootstrap receipts, and explicitly separated objective/subjective projections; pagination/capacity and provenance anchors are mandatory
**Performance Requirements:** Observer-off, idle, normal inspection, and heavy bounded query overhead measured
**Determinism Requirements:** Query cadence, locale, selection, and rendering cannot alter authoritative digests or event ordering
**Ontology Implications:** Schema metadata and human labels remain observer-only
**Observer Implications:** Becomes the primary validation surface for M4 capabilities
**Explanation Implications:** Explanation claims can link to inspectable typed evidence rather than trace counts alone
**Out of Scope:** Direct storage access, unbounded graph dumps, a panel for every field, or LLM prose

**Current evidence:** `world_chunks` now has an additive, versioned, bounded
`MaterialSurfaceDelta` V1 with chart/cell, typed condition delta, mana context, and optional trace
anchors whose field presence distinguishes absence from `TraceId(0)`. The local mana-material-surface
coupling slice adds local gate deltas to the observer wire protocol with V2 protocol guards. These
cover the accepted vertical slices only; broader query, pagination, and overhead requirements remain
open.

## TODO-EXPLAIN-003: Domain-Aware Causal Explanation
**Status:** In Progress
**Phase:** Detailed Development — Explanation
**Priority:** High
**Dependencies:** TODO-ANALYTICS-001, TODO-OBSERVER-003, TODO-EXPLAIN-001
**Goal:** Maintain an explanation layer that can answer what changed, why, relative to what baseline, with what uncertainty, and which alternatives remain viable
**Acceptance Criteria:** Stable typed schemas cover accepted Detailed Development vertical slices; reports retain all comparison frames; claims include domain units/scales, observation windows, causal/counterfactual context, alternatives or insufficiency, and inspectable trace-backed evidence
**Performance Requirements:** Bounded interactive queries benchmarked separately from offline experiment analysis
**Determinism Requirements:** Same state, query, and schema registry produce identical IR independent of locale/UI
**Ontology Implications:** Human classifications remain analytical and cannot become simulation or agent semantics
**Observer Implications:** Read models carry all structured claim inputs and source references
**Explanation Implications:** Deterministic rendering remains complete without an LLM
**Out of Scope:** Narrative invention, uncertainty resolution by prose, or terminal optional LLM integration

**Current evidence:** `MaterialSurfaceLoopClaim` supplies live typed values, window/control context,
insufficiency behavior, and trace-backed evidence for the actor/material/mana slice. Local coupling
claims added in the mana-material-surface coupling slice provide additional typed values and causal
attribution. These do not complete the domain-aware Explanation requirement for all future vertical
slices.

## TODO-PERF-001: Benchmark Framework
**Status:** Pending
**Phase:** Detailed Development — Cross-cutting
**Priority:** High
**Dependencies:** TODO-ARCH-001, TODO-DEPTH-001
**Goal:** Benchmark harness
**Acceptance Criteria:** Can measure ticks/second, memory, active sets
**Performance Requirements:** Minimal measurement overhead
**Determinism Requirements:** Benchmarks reproducible
**Ontology Implications:** N/A
**Observer Implications:** Exposes performance metrics
**Explanation Implications:** N/A
**Out of Scope:** Full performance suite

## TODO-PERSIST-001: Snapshot Format
**Status:** Completed
**Phase:** 0
**Priority:** Medium
**Dependencies:** TODO-ARCH-001
**Goal:** Define snapshot structure
**Acceptance Criteria:** Canonical bounded envelope with sectioned binary format; validated domain-state reconstruction; Runtime/scheduler resume at completed tick; uninterrupted versus save/reload/resume equivalence proven; CLI save/resume workflow; digest equality after reconstruction
**Performance Requirements:** Efficient serialization
**Determinism Requirements:** Roundtrip preserves state exactly
**Ontology Implications:** N/A
**Observer Implications:** N/A
**Explanation Implications:** N/A
**Out of Scope:** Full persistence system (incremental saves, compression, distributed storage)

## TODO-CUDA-001: Accelerator Abstraction
**Status:** Pending
**Phase:** Detailed Development — Performance (deferred until measured crossover)
**Priority:** Low
**Dependencies:** TODO-ARCH-001
**Goal:** Define accelerator trait
**Acceptance Criteria:** CPU reference + CUDA backend structure
**Performance Requirements:** Workload crossover measurement
**Determinism Requirements:** GPU results match CPU reference
**Ontology Implications:** N/A
**Observer Implications:** N/A
**Explanation Implications:** N/A
**Out of Scope:** Full CUDA implementation

## TODO-DET-001: Locale Independence Test
**Status:** Completed
**Phase:** 0
**Priority:** High
**Dependencies:** TODO-ARCH-001
**Goal:** Verify simulation state hash invariant
**Acceptance Criteria:** Different UI locales produce identical state hashes
**Performance Requirements:** N/A
**Determinism Requirements:** Test is deterministic
**Ontology Implications:** INV-LANG-001
**Observer Implications:** N/A
**Explanation Implications:** N/A
**Out of Scope:** Full localization

## TODO-LANG-003: Communicative Pressure Model
**Status:** Completed
**Phase:** 13
**Priority:** Medium
**Dependencies:** TODO-LANG-001
**Goal:** Model when agents create new words
**Acceptance Criteria:** Repeated unmet subjective concept-reference need accumulates bounded fixed-point pressure and gates deterministic coinage
**Performance Requirements:** Sparse computation
**Determinism Requirements:** Pressure calculation deterministic
**Ontology Implications:** Words emerge from social need
**Observer Implications:** Exposes lexical pressure analytics
**Explanation Implications:** Explains word origins
**Out of Scope:** Full language evolution

## TODO-ONTO-001: Primitive Inventory
**Status:** Completed
**Phase:** 2
**Priority:** High
**Dependencies:** TODO-CORE-001
**Goal:** Complete primitive vs emergent inventory
**Acceptance Criteria:** All engine primitives listed, all emergent concepts identified
**Performance Requirements:** N/A
**Determinism Requirements:** N/A
**Ontology Implications:** Foundation of all ontology
**Observer Implications:** Guides analytical classification
**Explanation Implications:** Distinguishes objective from subjective claims
**Out of Scope:** Full ontology implementation

## TODO-TRACE-001: Causal Provenance System
**Status:** Completed
**Phase:** 6
**Priority:** High
**Dependencies:** TODO-CORE-001
**Goal:** Define deterministic Ground Truth event and causal trace format
**Acceptance Criteria:** Stable proposal keys; opaque event-kind IDs; ordered prior causes; property-level before/after effects; monotonic event/trace IDs; direct parent/child traversal
**Performance Requirements:** Flat event/cause/effect storage with deterministic cold reverse-edge index; benchmark before scale claims
**Determinism Requirements:** Traces deterministic
**Ontology Implications:** INV-014: Provenance is first-class
**Observer Implications:** Exposes causal graphs
**Explanation Implications:** Core to causal explanations
**Out of Scope:** Semantic event strings, domain mutation systems, full causal query engine, persistence and observer wire projection

## TODO-ANALYTICS-001: Phenomenon Evaluation
**Status:** Pending
**Phase:** Detailed Development — Explanation
**Priority:** Critical
**Dependencies:** TODO-EXPLAIN-001
**Goal:** Implement domain-valid phenomenon, divergence, persistence, and recovery metrics
**Acceptance Criteria:** Digests are used only for equality/divergence; typed domain state vectors define scales and tolerances; causal depth is not reduced to trace count; domain coupling, path dependence, persistence, and recovery have baselines, observation windows, negative controls, supporting traces, and explicit unknown/unsupported outcomes
**Performance Requirements:** Offline analysis acceptable
**Determinism Requirements:** Metrics deterministic from state
**Ontology Implications:** Emergence must be inspectable (INV-019)
**Observer Implications:** Exposes phenomenon analytics
**Explanation Implications:** Supports emergence claims
**Out of Scope:** Digest-byte distance as physical similarity, arbitrary recovery tolerance, real-time mining, semantic verdicts without evidence

## TODO-GEO-002: Material Provenance
**Status:** Pending
**Phase:** Detailed Development — Geography / Matter
**Priority:** High
**Dependencies:** TODO-GEO-001
**Goal:** Track material origins
**Acceptance Criteria:** Material has geological formation → deposit → quarry → ... chain
**Performance Requirements:** Provenance chain compact
**Determinism Requirements:** Chain deterministic from generation
**Ontology Implications:** Materials have causal history
**Observer Implications:** Exposes material origins
**Explanation Implications:** Explains artifact composition
**Out of Scope:** Full trade chain simulation

## TODO-GEO-003: Multiscale World Spatial Geometry
**Status:** Completed
**Phase:** Foundational correction completed with Phase 24
**Priority:** High
**Dependencies:** TODO-COORD-001, TODO-WORLD-001, TODO-GEO-001
**Goal:** Separate global topology, local geometry, geographic surface, containment, and causal resolution
**Acceptance Criteria:** RFC-GEO-002 accepted; finite closed charted default surface; fixed-point 2.5D geography; bounded full local 3D; explicit chart/frame types and tested transforms
**Performance Requirements:** No planet-wide millimetre voxel grid; local volume allocated selectively
**Determinism Requirements:** Integer coordinates, explicit chart/frame identities, bounded exact local transforms
**Ontology Implications:** Geometry is physical state; containment and jurisdiction remain separate
**Observer Implications:** Rendering coordinates remain derived and non-authoritative
**Explanation Implications:** Future spatial explanations must cite geometry and containment separately
**Out of Scope:** Concrete planet metric, atlas generation, cross-chart geodesics, migration of every bare chunk, volumetric promotion/demotion

## TODO-BIO-002: Pathogen Model
**Status:** Completed
**Phase:** 5
**Priority:** Medium
**Dependencies:** TODO-BIO-001
**Goal:** Define pathogen primitives
**Acceptance Criteria:** Property-based pathogen lineages, traced physical exposure, objective host-interaction profiles
**Performance Requirements:** Canonical dense lineage traversal; epidemic performance claims deferred to benchmarks
**Determinism Requirements:** Construction and ancestry deterministic; future evolution uses scheduler-provided streams
**Ontology Implications:** Disease is socially constructed, pathogens are real
**Observer Implications:** Future read models may expose traced pathogen analytics
**Explanation Implications:** Exposure carries causal trace references for future explanations
**Out of Scope:** Molecular biology, live infection mutation, pathogen evolution algorithms, semantic disease/type/route enums

## TODO-BIO-003: Biological Mana Coupling and Emergent Practitioners
**Status:** Proposed
**Phase:** Detailed Development — Biology/Mana
**Priority:** High
**Dependencies:** TODO-DEPTH-001, TODO-SIM-001, TODO-BIO-001, TODO-MANA-001, TODO-PERCEPT-001, TODO-PRACTICE-001, TODO-PERSIST-001
**Goal:** Implement external physical biological mana coupling, including rare active modulation, dominant ritual/history coupling, and causally formed acquired or congenital retention without intrinsic MP or authoritative mage/spell categories
**Acceptance Criteria:** RFC-BIO-003 implementation gate is satisfied; ordinary weak coupling and negative controls exist; retained state has traced inflow or conversion, capacity, leakage, release, birth/death behavior, and environmental accounting; inherited structures and developmental history may produce congenital reserves without assigning a genetic MP value; intention reaches mana only through physical action; learned procedures remain subjective practices; ritual and active-coupling counterfactuals replay exactly
**Performance Requirements:** Representative organism, ritual, retained-carrier, and resolution workloads benchmark time, memory, provenance growth, and observer-off cost before scale claims
**Determinism Requirements:** Fixed-point authoritative state, canonical proposal reduction, named RNG streams, same-seed replay, save/resume equivalence, and batch-order invariance
**Ontology Implications:** Mana remains physical; mage, spell, affinity, personal mana, school, and magical taxonomy are emergent classifications rather than authoritative state
**Observer Implications:** Bounded evidence may expose field/carrier transfers, physiological costs, and uncertainty without asserting subjective classifications as Ground Truth
**Explanation Implications:** Claims distinguish coupling, developmental origin, retained field, execution, ritual history, learned belief, and social classification with trace support
**Out of Scope:** Semantic spell dispatch, elemental enums, guaranteed practitioners, free mana regeneration, wish fulfillment, direct belief/intention coupling, final field physics, and implementation before detailed biology contracts

## TODO-LANG-004: Writing System Model
**Status:** Completed
**Phase:** 16
**Priority:** Low
**Dependencies:** TODO-LANG-003
**Goal:** Define writing system primitives
**Acceptance Criteria:** Opaque physical glyph sequences, bounded documents, explicit deterministic copying edits, and document ancestry
**Performance Requirements:** Document storage efficient
**Determinism Requirements:** Copying errors deterministic
**Ontology Implications:** Documents are physical carriers
**Observer Implications:** Exposes document lineage
**Explanation Implications:** Explains textual traditions
**Out of Scope:** Full paleography

## TODO-SOCIAL-002: Law and Contracts
**Status:** Completed
**Phase:** 19
**Priority:** Low
**Dependencies:** TODO-SOCIAL-001
**Goal:** Define legal primitives
**Acceptance Criteria:** Contestable rule records separate source text, interpretations, precedent, authority, and trace; attested agreements separate physical text, opaque parties/witnesses, authority, time, and trace
**Performance Requirements:** N/A
**Determinism Requirements:** Canonical reference ordering and deterministic validation
**Ontology Implications:** Law is not `active: bool`
**Observer Implications:** Exposes legal structure
**Explanation Implications:** Explains legal evolution
**Out of Scope:** Full legal simulation, adjudication, universal validity, automatic enforcement, semantic legal taxonomy, and contract magic

## TODO-ISEKAI-002: Imported Knowledge Separation
**Status:** Completed
**Phase:** 22
**Priority:** Medium
**Dependencies:** TODO-ISEKAI-001
**Goal:** Model knowledge vs capability
**Acceptance Criteria:** Declarative knowledge separate from procedural, tools, materials
**Performance Requirements:** N/A
**Determinism Requirements:** N/A
**Ontology Implications:** Knowing != being able to do
**Observer Implications:** Exposes isekai agent state
**Explanation Implications:** Explains technology gaps
**Out of Scope:** Full technology tree

## TODO-EXPLAIN-002: Deterministic Renderer
**Status:** Completed
**Phase:** 25
**Priority:** High
**Dependencies:** TODO-EXPLAIN-001
**Goal:** Implement template-based text generation
**Acceptance Criteria:** IR → localized text without LLM
**Performance Requirements:** < 10ms per explanation
**Determinism Requirements:** Same IR → same text
**Ontology Implications:** Text is non-authoritative
**Observer Implications:** Core UI feature
**Explanation Implications:** Self-referential
**Out of Scope:** LLM surface

## TODO-OBSERVER-002: Snapshot and Delta Streaming
**Status:** Completed
**Phase:** 0
**Priority:** Medium
**Dependencies:** TODO-PROTO-001
**Goal:** Implement observer data delivery
**Acceptance Criteria:** Scoped snapshots + incremental deltas
**Performance Requirements:** Efficient delta encoding
**Determinism Requirements:** Delta order deterministic
**Ontology Implications:** Derived data
**Observer Implications:** Enables real-time UI
**Explanation Implications:** N/A
**Out of Scope:** Full streaming backpressure

## TODO-DEV-001: Codebase Knowledge Graph
**Status:** Completed
**Phase:** 0
**Priority:** High
**Dependencies:** None
**Goal:** Document codebase-memory-mcp usage
**Acceptance Criteria:** AGENTS.md references graph tools
**Performance Requirements:** N/A
**Determinism Requirements:** N/A
**Ontology Implications:** N/A
**Observer Implications:** N/A
**Explanation Implications:** N/A
**Out of Scope:** Graph implementation

## TODO-ADR-001: Rust Edition 2024
**Status:** Accepted
**Phase:** 0
**Priority:** High
**Dependencies:** None
**Goal:** Use modern Rust
**Acceptance Criteria:** Edition 2024 in all crates
**Performance Requirements:** N/A
**Determinism Requirements:** N/A
**Ontology Implications:** N/A
**Observer Implications:** N/A
**Explanation Implications:** N/A
**Out of Scope:** N/A

## TODO-ADR-002: Workspace with Domain Crates
**Status:** Accepted
**Phase:** 0
**Priority:** High
**Dependencies:** TODO-ADR-001
**Goal:** Clear domain separation
**Acceptance Criteria:** ~20 crates with directed dependencies
**Performance Requirements:** N/A
**Determinism Requirements:** N/A
**Ontology Implications:** Enforces domain boundaries
**Observer Implications:** N/A
**Explanation Implications:** N/A
**Out of Scope:** N/A

## TODO-ADR-003: Deterministic Random Streams
**Status:** Accepted
**Phase:** 0
**Priority:** High
**Dependencies:** TODO-ADR-001
**Goal:** Reproducible simulation
**Acceptance Criteria:** Strict and fast modes defined
**Performance Requirements:** N/A
**Determinism Requirements:** Strict mode fully deterministic
**Ontology Implications:** INV-003
**Observer Implications:** N/A
**Explanation Implications:** N/A
**Out of Scope:** N/A

## TODO-ADR-004: Headless Simulation
**Status:** Accepted
**Phase:** 0
**Priority:** High
**Dependencies:** TODO-ADR-001
**Goal:** UI cannot access simulation state
**Acceptance Criteria:** Observer API is only interface
**Performance Requirements:** N/A
**Determinism Requirements:** N/A
**Ontology Implications:** INV-021: UI is an observer
**Observer Implications:** All data derived
**Explanation Implications:** N/A
**Out of Scope:** N/A

## TODO-ADR-005: Tauri + React + WebGPU
**Status:** Accepted
**Phase:** 0
**Priority:** High
**Dependencies:** TODO-ADR-004
**Goal:** Modern desktop UI
**Acceptance Criteria:** Tauri shell with React panels and WebGPU map
**Performance Requirements:** WebGPU for large datasets
**Determinism Requirements:** N/A
**Ontology Implications:** N/A
**Observer Implications:** Rich UI views
**Explanation Implications:** N/A
**Out of Scope:** N/A

## TODO-ADR-006: Protocol Buffers
**Status:** Accepted
**Phase:** 0
**Priority:** High
**Dependencies:** TODO-ADR-004
**Goal:** Efficient observer protocol
**Acceptance Criteria:** Versioned schemas, multi-language support
**Performance Requirements:** Binary serialization
**Determinism Requirements:** N/A
**Ontology Implications:** N/A
**Observer Implications:** Protocol foundation
**Explanation Implications:** N/A
**Out of Scope:** N/A
