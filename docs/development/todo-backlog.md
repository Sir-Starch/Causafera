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

## TODO-SIM-001: Durable Physical State and Cross-Domain Coupling
**Status:** Pending
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

## TODO-RUNTIME-001: Production Bootstrap and Fixture Elimination
**Status:** Pending
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

## TODO-OBSERVER-003: Bounded Causal and Domain Inspection
**Status:** Pending
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

## TODO-EXPLAIN-003: Domain-Aware Causal Explanation
**Status:** Pending
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
