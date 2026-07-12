# Ontopolis TODO Backlog

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
**Status:** Pending
**Phase:** 7
**Priority:** Medium
**Dependencies:** TODO-BIO-001
**Goal:** Implement attention mechanism
**Acceptance Criteria:** Agents have limited attention, focus shifts based on salience
**Performance Requirements:** Sparse updates
**Determinism Requirements:** Attention deterministic given same inputs
**Ontology Implications:** Cognition is bounded
**Observer Implications:** May expose attention state
**Explanation Implications:** Explains why agents miss things
**Out of Scope:** Full cognitive architecture

## TODO-SCENE-001: Implement Minimal Subjective Scene Representation
**Status:** Pending
**Phase:** 9
**Priority:** High
**Dependencies:** RFC-COG-001 accepted
**Goal:** Implement the minimum viable subjective scene representation based on accepted RFC-COG-001
**Acceptance Criteria:** Rust types exist for bounded subjective scene; scene construction is deterministic; no authoritative entity identity leaks into cognitive state
**Performance Requirements:** Bounded active state per agent; deterministic reconstruction
**Determinism Requirements:** Scene construction deterministic given same inputs and history
**Ontology Implications:** Agents act on constructed scenes, not raw Ground Truth (INV-029)
**Observer Implications:** May expose scene analytics
**Explanation Implications:** Explains why different agents experience the same place differently
**Out of Scope:** Full implementation of all scene subsystems

## TODO-SCENE-002: Perceived Object Persistence
**Status:** Pending
**Phase:** 9
**Priority:** High
**Dependencies:** TODO-SCENE-001
**Goal:** Design and implement subjective object identity tracking
**Acceptance Criteria:** PerceivedObjectIdentity distinct from EntityId; agents can merge, split, lose, and misidentify objects; identity errors propagate
**Performance Requirements:** Sparse updates; bounded number of tracked objects per agent
**Determinism Requirements:** Identity updates deterministic given same perceptual history
**Ontology Implications:** Perceived identity is a subjective hypothesis (INV-028)
**Observer Implications:** Exposes identity tracking analytics
**Explanation Implications:** Explains misidentification and false recognition
**Out of Scope:** Perfect identity tracking; omniscient object knowledge

## TODO-SCENE-003: Subjective Body Schema
**Status:** Pending
**Phase:** 9
**Priority:** Medium
**Dependencies:** TODO-BIO-001, TODO-SCENE-001
**Goal:** Design the mapping from biological signals to experienced body model
**Acceptance Criteria:** Body schema is distinct from BodyStructure; updates via sensory and internal signals; may disagree with authoritative state
**Performance Requirements:** Incremental updates; bounded schema size
**Determinism Requirements:** Schema updates deterministic given same signal history
**Ontology Implications:** Objective body state and subjective body schema are distinct (INV-034)
**Observer Implications:** May expose schema vs structure divergence
**Explanation Implications:** Explains phantom sensations, capability misjudgments
**Out of Scope:** Full physiological simulation; neural body mapping

## TODO-SCENE-004: Self-Model Architecture
**Status:** Pending
**Phase:** 9
**Priority:** Medium
**Dependencies:** TODO-SCENE-001
**Goal:** Design the persistent but revisable self-model
**Acceptance Criteria:** Self-model contains believed abilities, autobiographical continuity, social identities; may disagree with Ground Truth
**Performance Requirements:** Persistent but not continuously active
**Determinism Requirements:** Self-model updates deterministic given same experiences
**Ontology Implications:** The self-model is subjective (INV-033)
**Observer Implications:** May expose self-model divergence
**Explanation Implications:** Explains overconfidence, identity confusion
**Out of Scope:** Full personality simulation; fixed trait enums

## TODO-SCENE-005: Predictive World Model
**Status:** Pending
**Phase:** 10
**Priority:** High
**Dependencies:** TODO-SCENE-001
**Goal:** Design sparse, bounded prediction and prediction-error handling
**Acceptance Criteria:** Predictions are sparse and context-limited; prediction error affects attention, salience, memory, and concept revision
**Performance Requirements:** No global physics per agent; bounded prediction count
**Determinism Requirements:** Predictions deterministic given same beliefs and scene
**Ontology Implications:** Prediction error is a first-class cognitive driver (INV-035)
**Observer Implications:** May expose prediction accuracy analytics
**Explanation Implications:** Explains surprise and expectation-driven behavior
**Out of Scope:** Global physics simulator; perfect prediction

## TODO-SCENE-006: Working Memory and Active Context
**Status:** Pending
**Phase:** 10
**Priority:** High
**Dependencies:** TODO-SCENE-001
**Goal:** Design bounded working memory separate from persistent storage
**Acceptance Criteria:** Working memory has capacity limit and decay; long-term memory is not continuously active
**Performance Requirements:** Bounded active items per agent; minimal overhead for inactive agents
**Determinism Requirements:** Working memory updates deterministic given same cues and state
**Ontology Implications:** Persistent memory is not continuously active context (INV-032)
**Observer Implications:** May expose active context size
**Explanation Implications:** Explains forgetting and inattention
**Out of Scope:** Unlimited working memory; perfect recall

## TODO-SCENE-007: Episodic Memory Reactivation
**Status:** Pending
**Phase:** 10
**Priority:** Medium
**Dependencies:** TODO-SCENE-006
**Goal:** Design similarity-driven, relevance-weighted memory reactivation
**Acceptance Criteria:** Current perceptual patterns can reactivate stored episodes; reactivation is partial and graded; no semantic event triggers
**Performance Requirements:** Efficient similarity matching; bounded reactivation per tick
**Determinism Requirements:** Reactivation deterministic given same memory index and cues
**Ontology Implications:** Memories become active through similarity, not semantic lookup
**Observer Implications:** May expose reactivation traces
**Explanation Implications:** Explains involuntary memory and deja vu
**Out of Scope:** Perfect memory retrieval; keyword-based search

## TODO-SCENE-008: Agency Attribution
**Status:** Pending
**Phase:** 10
**Priority:** Medium
**Dependencies:** TODO-SCENE-005
**Goal:** Design learned agency attribution from action-outcome observation
**Acceptance Criteria:** Agents can learn and mislearn that their actions cause outcomes; agency model feeds self-model and causal inference
**Performance Requirements:** Incremental updates; bounded agency model size
**Determinism Requirements:** Agency updates deterministic given same action-outcome history
**Ontology Implications:** Agency is constructed, not innate
**Observer Implications:** May expose agency attribution analytics
**Explanation Implications:** Explains superstition and learned helplessness
**Out of Scope:** Perfect causal knowledge; innate agency

## TODO-SCENE-009: Subjective Temporal Continuity
**Status:** Pending
**Phase:** 10
**Priority:** Medium
**Dependencies:** TODO-SCENE-001
**Goal:** Design the bounded temporal envelope binding recent past, current scene, and expected near future
**Acceptance Criteria:** Current scene is not an isolated tick; temporal anchors structure subjective time; bounded history buffer
**Performance Requirements:** Bounded temporal envelope size
**Determinism Requirements:** Temporal envelope updates deterministic given same scene sequence
**Ontology Implications:** Temporal continuity is constructed, not a simulation primitive
**Observer Implications:** May expose temporal envelope state
**Explanation Implications:** Explains anticipation and dread
**Out of Scope:** Full autobiographical narrative generation

## TODO-CONCEPT-001: Sparse Concept Formation
**Status:** Pending
**Phase:** 11
**Priority:** High
**Dependencies:** TODO-COG-001, TODO-SCENE-001, TODO-SCENE-006
**Goal:** RFC acceptance and implementation
**Acceptance Criteria:** RFC approved, prototype implemented
**Performance Requirements:** Attention-driven, not continuous clustering
**Determinism Requirements:** Concept formation deterministic
**Ontology Implications:** Concepts are subjective, not Ground Truth
**Observer Implications:** Exposes concept analytics
**Explanation Implications:** Core to Explanation IR
**Out of Scope:** Semantic concept enums

## TODO-COG-002: Bounded Cognition Model
**Status:** Pending
**Phase:** 12
**Priority:** Medium
**Dependencies:** TODO-COG-001, TODO-SCENE-006
**Goal:** Implement cognitive limits
**Acceptance Criteria:** Working memory limits, belief inertia, source trust
**Performance Requirements:** Sparse updates
**Determinism Requirements:** Cognition deterministic given same state
**Ontology Implications:** Stable mistakes are essential
**Observer Implications:** Exposes cognitive state
**Explanation Implications:** Explains why agents are wrong
**Out of Scope:** Full psychological model

## TODO-LANG-001: Historical Language Bootstrap
**Status:** Pending
**Phase:** 13
**Priority:** High
**Dependencies:** TODO-CONCEPT-001
**Goal:** RFC acceptance and implementation
**Acceptance Criteria:** RFC approved, bootstrap generates language lineages
**Performance Requirements:** Lower-resolution than main simulation
**Determinism Requirements:** Bootstrap deterministic from seed
**Ontology Implications:** Languages are physical patterns, not English strings
**Observer Implications:** Exposes language trees
**Explanation Implications:** Explains word origins
**Out of Scope:** Manual dictionary creation

## TODO-LANG-002: Lexical Innovation
**Status:** Pending
**Phase:** 14
**Priority:** Medium
**Dependencies:** TODO-LANG-001
**Goal:** Implement novel word creation
**Acceptance Criteria:** Phonotactic generation, community adoption model
**Performance Requirements:** Deterministic in strict mode
**Determinism Requirements:** Form generation deterministic from inputs
**Ontology Implications:** Words are socially transmitted lineages
**Observer Implications:** Exposes lexeme histories
**Explanation Implications:** Explains neologisms
**Out of Scope:** Full semantic drift simulation

## TODO-PRACTICE-001: Evolvable Practice Representation
**Status:** Pending
**Phase:** 15
**Priority:** High
**Dependencies:** TODO-LANG-002
**Goal:** RFC acceptance and implementation
**Acceptance Criteria:** RFC approved, practice format supports operations, conditions, branches
**Performance Requirements:** Efficient execution
**Determinism Requirements:** Practice execution deterministic
**Ontology Implications:** Practices are programs, explanations are separate
**Observer Implications:** Exposes practice lineages
**Explanation Implications:** Explains ritual origins
**Out of Scope:** Full practice evolution

## TODO-EPI-001: Measurement and Metrology
**Status:** Pending
**Phase:** 16
**Priority:** Medium
**Dependencies:** TODO-PRACTICE-001
**Goal:** RFC acceptance and implementation
**Acceptance Criteria:** RFC approved, measurement types with precision and error
**Performance Requirements:** Minimal overhead
**Determinism Requirements:** Measurements deterministic given same conditions
**Ontology Implications:** Units are socially constructed
**Observer Implications:** Exposes measurement systems
**Explanation IR:** Explains standardization effects
**Out of Scope:** Full instrument simulation

## TODO-MANA-001: Information-Sensitive Field Model
**Status:** Pending
**Phase:** 17
**Priority:** High
**Dependencies:** TODO-GEO-001
**Goal:** RFC acceptance and implementation
**Acceptance Criteria:** RFC approved, minimal field model responds to patterns
**Performance Requirements:** GPU candidate
**Determinism Requirements:** Field evolution deterministic
**Ontology Implications:** Mana does not understand meaning
**Observer Implications:** Exposes field visualization
**Explanation Implications:** Explains magical effects causally
**Out of Scope:** Full spell system

## TODO-RES-001: Causal Resolution Field
**Status:** Pending
**Phase:** 18
**Priority:** High
**Dependencies:** TODO-MANA-001
**Goal:** RFC acceptance and implementation
**Acceptance Criteria:** RFC approved, resolution varies by causal relevance
**Performance Requirements:** Efficient aggregation
**Determinism Requirements:** Resolution decisions deterministic
**Ontology Implications:** Distance is not resolution
**Observer Implications:** Exposes resolution state
**Explanation Implications:** Explains why some areas are detailed
**Out of Scope:** Full multi-resolution simulation

## TODO-SOCIAL-001: Organization Primitives
**Status:** Pending
**Phase:** 19
**Priority:** Medium
**Dependencies:** TODO-RES-001
**Goal:** Define organization structures
**Acceptance Criteria:** Members, roles, communication, authority, records, property, rules, practices
**Performance Requirements:** Distributed representation
**Determinism Requirements:** N/A
**Ontology Implications:** No organization brain
**Observer Implications:** Exposes organizational structure
**Explanation Implications:** Explains institutional beliefs
**Out of Scope:** Full governance simulation

## TODO-ECON-001: Material Flow Contracts
**Status:** Pending
**Phase:** 20
**Priority:** Medium
**Dependencies:** TODO-SOCIAL-001
**Goal:** Define economy interfaces
**Acceptance Criteria:** Inventory, production, transformation, labour, ownership
**Performance Requirements:** Efficient batch updates
**Determinism Requirements:** Flows deterministic
**Ontology Implications:** Material substitution preserves differences
**Observer Implications:** Exposes supply chains
**Explanation Implications:** Explains shortages and surpluses
**Out of Scope:** Full market simulation

## TODO-CITY-001: Infrastructure Networks
**Status:** Pending
**Phase:** 20
**Priority:** Medium
**Dependencies:** TODO-ECON-001
**Goal:** Define city infrastructure
**Acceptance Criteria:** Parcels, roads, water, sewage, buildings
**Performance Requirements:** Network traversal efficient
**Determinism Requirements:** Layout deterministic from history
**Ontology Implications:** Infrastructure creates spatial patterns
**Observer Implications:** Exposes networks
**Explanation Implications:** Explains urban development
**Out of Scope:** Full city growth simulation

## TODO-ISEKAI-001: Cross-World Transfer Model
**Status:** Pending
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
**Status:** Pending
**Phase:** 23
**Priority:** Low
**Dependencies:** TODO-ISEKAI-001
**Goal:** RFC acceptance
**Acceptance Criteria:** RFC approved, research architecture defined
**Performance Requirements:** N/A
**Determinism Requirements:** N/A
**Ontology Implications:** No primitive Soul object
**Observer Implications:** N/A
**Explanation Implications:** Explains identity concepts
**Out of Scope:** Full implementation

## TODO-META-002: Stateful Mana Attractors
**Status:** Pending
**Phase:** 23
**Priority:** Low
**Dependencies:** TODO-META-001
**Goal:** RFC acceptance
**Acceptance Criteria:** RFC approved, attractor hypothesis documented
**Performance Requirements:** N/A
**Determinism Requirements:** N/A
**Ontology Implications:** Gods are emergent, not primitive
**Observer Implications:** N/A
**Explanation Implications:** Explains religious phenomena causally
**Out of Scope:** Full implementation

## TODO-EXPLAIN-001: Explanation IR
**Status:** Pending
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
**Status:** Pending
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
**Status:** Pending
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

## TODO-PERF-001: Benchmark Framework
**Status:** Pending
**Phase:** 0
**Priority:** Medium
**Dependencies:** TODO-ARCH-001
**Goal:** Benchmark harness
**Acceptance Criteria:** Can measure ticks/second, memory, active sets
**Performance Requirements:** Minimal measurement overhead
**Determinism Requirements:** Benchmarks reproducible
**Ontology Implications:** N/A
**Observer Implications:** Exposes performance metrics
**Explanation Implications:** N/A
**Out of Scope:** Full performance suite

## TODO-PERSIST-001: Snapshot Format
**Status:** Pending
**Phase:** 0
**Priority:** Medium
**Dependencies:** TODO-ARCH-001
**Goal:** Define snapshot structure
**Acceptance Criteria:** Snapshot can be serialized and deserialized
**Performance Requirements:** Efficient serialization
**Determinism Requirements:** Roundtrip preserves state exactly
**Ontology Implications:** N/A
**Observer Implications:** N/A
**Explanation Implications:** N/A
**Out of Scope:** Full persistence system

## TODO-CUDA-001: Accelerator Abstraction
**Status:** Pending
**Phase:** 0
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
**Status:** Pending
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
**Status:** Pending
**Phase:** 13
**Priority:** Medium
**Dependencies:** TODO-LANG-001
**Goal:** Model when agents create new words
**Acceptance Criteria:** Repeated communication need → lexical pressure
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
**Status:** Pending
**Phase:** 6
**Priority:** High
**Dependencies:** TODO-CORE-001
**Goal:** Define causal trace format
**Acceptance Criteria:** Events have causes, effects, and event type
**Performance Requirements:** Efficient graph traversal
**Determinism Requirements:** Traces deterministic
**Ontology Implications:** INV-014: Provenance is first-class
**Observer Implications:** Exposes causal graphs
**Explanation Implications:** Core to causal explanations
**Out of Scope:** Full causal query engine

## TODO-ANALYTICS-001: Phenomenon Evaluation
**Status:** Pending
**Phase:** 24
**Priority:** Low
**Dependencies:** TODO-EXPLAIN-001
**Goal:** Implement phenomenon metrics
**Acceptance Criteria:** Causal depth, domain coupling, path dependence measurable
**Performance Requirements:** Offline analysis acceptable
**Determinism Requirements:** Metrics deterministic from state
**Ontology Implications:** Emergence must be inspectable (INV-019)
**Observer Implications:** Exposes phenomenon analytics
**Explanation Implications:** Supports emergence claims
**Out of Scope:** Real-time mining

## TODO-GEO-002: Material Provenance
**Status:** Pending
**Phase:** 4
**Priority:** Medium
**Dependencies:** TODO-GEO-001
**Goal:** Track material origins
**Acceptance Criteria:** Material has geological formation → deposit → quarry → ... chain
**Performance Requirements:** Provenance chain compact
**Determinism Requirements:** Chain deterministic from generation
**Ontology Implications:** Materials have causal history
**Observer Implications:** Exposes material origins
**Explanation Implications:** Explains artifact composition
**Out of Scope:** Full trade chain simulation

## TODO-BIO-002: Pathogen Model
**Status:** Pending
**Phase:** 5
**Priority:** Medium
**Dependencies:** TODO-BIO-001
**Goal:** Define pathogen primitives
**Acceptance Criteria:** Pathogen lineages, transmission, host interaction
**Performance Requirements:** Epidemic simulation efficient
**Determinism Requirements:** Pathogen evolution deterministic
**Ontology Implications:** Disease is socially constructed, pathogens are real
**Observer Implications:** Exposes pathogen analytics
**Explanation Implications:** Explains disease concepts
**Out of Scope:** Molecular biology

## TODO-LANG-004: Writing System Model
**Status:** Pending
**Phase:** 16
**Priority:** Low
**Dependencies:** TODO-LANG-003
**Goal:** Define writing system primitives
**Acceptance Criteria:** Glyphs, documents, copying errors
**Performance Requirements:** Document storage efficient
**Determinism Requirements:** Copying errors deterministic
**Ontology Implications:** Documents are physical carriers
**Observer Implications:** Exposes document lineage
**Explanation Implications:** Explains textual traditions
**Out of Scope:** Full paleography

## TODO-SOCIAL-002: Law and Contracts
**Status:** Pending
**Phase:** 19
**Priority:** Low
**Dependencies:** TODO-SOCIAL-001
**Goal:** Define legal primitives
**Acceptance Criteria:** Texts, interpretations, precedent, authority
**Performance Requirements:** N/A
**Determinism Requirements:** N/A
**Ontology Implications:** Law is not `active: bool`
**Observer Implications:** Exposes legal structure
**Explanation Implications:** Explains legal evolution
**Out of Scope:** Full legal simulation

## TODO-ISEKAI-002: Imported Knowledge Separation
**Status:** Pending
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
**Status:** Pending
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
**Status:** Pending
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
