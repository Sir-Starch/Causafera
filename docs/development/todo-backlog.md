# Causafera TODO Backlog

## TODO-LEGAL-001: CLA Assistant Configuration and Verification
**Status:** Completed
**Phase:** 0
**Priority:** Critical — before first external contribution is merged
**Dependencies:** None
**Goal:** Configure and end-to-end verify the hosted CLA Assistant acceptance workflow for CLA version 1.1.
**Acceptance Criteria:** Met. CLA 1.1 is published as a public Gist (`eb32d78ea648f989831f7aa0a3bac81c`, revision `7c6daa72020318c47d14bca27655097cce236d6b`), byte-identical to `CLA.md` at blob `3c89692912e7d645e376cded4d6547ca1f874fc7`; CLA Assistant is linked to `Sir-Starch/Causafera` against that Gist's `CLA.md`, with Shared Gist disabled and no minimum file-count or line-count exemption; acceptance cannot be inferred from pull-request submission, and the `license/cla` check moves from pending to passing on acceptance without a further push; `license/cla` is a required status check on `main` alongside `rust` and `ui`, so an unsigned contribution is blocked from merging; the maintainer and non-signing automation accounts are exempt through the service's own allowlist, the narrowest mechanism available, leaving enforcement intact for every other contributor.
**Follow-up:** Periodic private export and backup of acceptance records, and the CLA update procedure, are documented in `docs/legal/cla-service-setup.md` as ongoing maintainer operations rather than open work.
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
**Status:** Completed
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
**Resolution:** Retained heat is now a third conserved bucket, alongside thermal cells and reservoirs: every material surface exchanges energy with its co-located `ThermalField` cell inside the same atomic Physics-phase batch that already diffuses cell-to-cell energy, using the identical `signed_flux` formula as face diffusion (generalized to take an explicit `(fraction, scale)` pair so both paths share one implementation). The coefficient bound widened from `6 * transfer_fraction <= scale` to `6 * transfer_fraction + material_exchange_fraction <= scale`, so a cell can never go negative even when all seven simultaneous outflows (six faces plus the material sink) are realized in the same tick; headroom capping applies only to the heating direction, since the cooling direction can never remove more than the material actually retains. This landed as `ThermalMaterialSite`/`ThermalMaterialTransferRecord` in the domain layer, `MaterialSurfaceThermalState` and event kind `MATERIAL_SURFACE_THERMAL_EXCHANGE_EVENT_KIND` in the runtime, and a bounded transition history (`MAX_MATERIAL_SURFACE_TRANSITIONS`, oldest evicted first) mirroring the existing condition/gate transition recorders. Persistence bumped `CURRENT_DIGEST_SCHEMA_VERSION` 5→6 and `MATERIAL_SURFACE_SECTION_MAJOR`/`THERMAL_SECTION_MAJOR` accordingly; import extends the existing signed-flux receipt equation to include the material term (`pre_state - sum(face.signed_flux) - material.signed_flux == post_state`) so a coordinated forgery is rejected in any batch, not only the latest. No new scheduler `System` was registered and no RNG stream was added — the exchange is additive inside `ThermalEvolutionSystem`'s existing `propose_evolution` call, confirmed via `causafera-core/src/scheduler.rs` that RNG stream keys derive from registration order, a distinct namespace from the `*_SYSTEM_ID` constants used for `EventProposalKey` ordering, so adding a `System` would have been both unnecessary and RNG-destabilizing. A material site with no co-located thermal cell is treated as an internal invariant violation (`ThermalError::PositionOutsideField`), not tolerated defensively, since bootstrap guarantees the pairing today. Production defaults: `transfer_fraction = 128`, `material_exchange_fraction = 64`, `material_thermal_capacity = THERMAL_SCALE` (`1024`) — chosen well below the validated ceiling so the new coupling is observable rather than dominant; not a tuned physical constant. Observer/Explanation: `MaterialSurfaceThermalDelta` (bounded at 64, sharing `material_surface_delta_schema_version` with the existing condition/gate deltas since all three address the same `MaterialSurfaceId` family, bumped 3→4) and Explanation claim schema 17 (`MATERIAL_SURFACE_THERMAL_EXCHANGE_SCHEMA`), wired through a standalone `RuntimeState::material_surface_thermal_explanation` query rather than folded into the existing condition/mana loop explanation, because thermal exchange runs independent of contact and folding in would make a thermally-active-but-never-touched surface unexplainable. No UI panel, no temperature derivation, no heterogeneous per-material properties, no expansion/damage/phase-change response — see the follow-up TODOs this closure adds. Per-tick performance cost was not benchmarked in this tranche; the material record adds a small, bounded per-cell increment to the already-named unbounded thermal-receipt growth tracked by `TODO-PERF-002`/`TODO-PERF-003`, making that gap somewhat larger rather than introducing a new one. See `plans/thermal-material-surface-coupling.md` for the full design, Decision log, and V1-V15 verification mapping.

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
**Status:** Completed
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

**Resolution:** Implemented in `plans/thermal-conservation-aggregate-validation.md`. The validation enforces six identities on import: I1 (reservoir budget delta vs. accepted injection, already enforced), I2 (cell energy delta vs. per-receipt transition plus accepted injection), I3 (material retained delta vs. signed material flux), I3a (per-receipt material retained delta vs. signed flux), I4 (residual zero), I5 (chain continuity between consecutive batches), and I6 (terminal anchor against the fully materialized final field, reservoir, and material surface states). The terminal anchor plus downward induction determines all `6N` aggregate literals from final state and per-receipt data without reconstructing historical per-cell energies. The implementation is side-effect free, uses checked `i128` arithmetic, and is order-independent because the receipt fold is keyed by conservation trace. The same statistical harness on an AMD Ryzen 9 7950X3D measured a `production_loop_config(2026)` snapshot with `N=36`, `V=27`, `Σ|receipts|=668` before and after the validator: mean import time `1052697.1 ns` → `955733.4 ns` (`-9.21%`), median `942363 ns` → `949907 ns` (`+0.80%`), with sample standard deviations `177317.4 ns` and `24939.0 ns`; no performance pass/fail claim is made because no regression threshold was defined. A uniform aggregate offset alone is rejected by I6. The remaining limitation requires shifting the materialized final field and every batch's `total_cell_energy_before`/`total_cell_energy_after` by the same `+Δ`; because the bootstrap total `C_1^-` has no independent persisted anchor, that coordinated forgery remains inside the `SECURITY.md` pre-alpha untrusted-snapshot carve-out and was not chased because closing it requires a new persisted scalar, event, or effect encoding.

## TODO-THERMAL-007: Material Expansion, Damage, and Phase-Change Response
**Status:** Pending
**Phase:** Detailed Development
**Priority:** Low
**Dependencies:** TODO-THERMAL-002 (completed retained-heat coupling)
**Goal:** Give a material surface's retained thermal energy a further physical consequence beyond storage — expansion, structural damage accumulation, or phase change — without a semantic toggle or a derived temperature figure standing in for the mechanism.
**Acceptance Criteria:** Whatever response is defined is a bounded, conserved, or explicitly-accounted physical quantity with its own trace-backed transitions, not a label; retained energy remains the authoritative unit driving it.
**Performance Requirements:** Benchmark before claims
**Determinism Requirements:** Response deterministic given retained energy and its history
**Ontology Implications:** Structural/phase state becomes causal state, not an English condition label
**Observer Implications:** New bounded delta(s) for whatever state is added
**Explanation Implications:** New claim schema(s) with trace support, scoped by `MaterialSurfaceId`
**Out of Scope:** Climate, biology, economy; heterogeneous per-material response curves (`TODO-THERMAL-008`)
**Context:** Named as an explicit Non-goal of `TODO-THERMAL-002` (`plans/thermal-material-surface-coupling.md`), which established the conserved retained-heat bucket this response would consume but deliberately did not define what retained energy causes beyond storage.

## TODO-THERMAL-008: Heterogeneous Per-Material Thermal Properties
**Status:** Pending
**Phase:** Detailed Development
**Priority:** Low
**Dependencies:** TODO-THERMAL-002 (completed retained-heat coupling)
**Goal:** Replace the homogeneous `material_exchange_fraction`/`material_thermal_capacity` parameters with per-material values drawn from `causafera_types::Material`'s existing `thermal_conductivity`/`specific_heat` fields, which the runtime does not yet read.
**Acceptance Criteria:** Distinct materials exchange and retain heat at physically distinct rates; the widened `6 * transfer_fraction + material_exchange_fraction <= scale` bound (or its per-material analogue) still provably prevents negative energy for every realized material; existing homogeneous-parameter tests either still pass under a uniform-material configuration or are superseded by an equivalent per-material test.
**Performance Requirements:** Benchmark before claims; per-material lookup must not turn the per-cell exchange step into a per-material-type branch explosion
**Determinism Requirements:** Per-material response deterministic given material assignment and thermal state
**Ontology Implications:** Matter's physical properties drive its thermal behavior directly, rather than a homogeneous stand-in
**Observer Implications:** None beyond what `TODO-THERMAL-002` already exposes, unless a per-material property becomes independently queryable
**Explanation Implications:** None beyond what `TODO-THERMAL-002` already exposes
**Out of Scope:** Defining what retained energy causes beyond storage (`TODO-THERMAL-007`); climate, biology, economy
**Context:** Named as Non-goal #2 of `TODO-THERMAL-002` (`plans/thermal-material-surface-coupling.md`): parameters stayed homogeneous "like `heat_capacity` today," and `causafera_types::Material`'s per-material `thermal_conductivity`/`specific_heat` (`f64`) were explicitly not pulled into that tranche.

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

## TODO-OBS-003: Observer Wire Decoder Parity Outside the Runtime Summary
**Status:** Pending
**Phase:** Detailed Development — Observer Surface
**Priority:** High
**Dependencies:** None
**Goal:** Make the Rust and TypeScript observer decoders accept and reject exactly the same byte strings, across every shared decoder, not only the runtime summary
**Acceptance Criteria:** For every shared codec — world chunk snapshot, query response, connect response, stream envelope, field raster, Explanation IR — a hand-built adversarial corpus fed to both decoders produces the same accept/reject verdict and the same decoded values; the corpus lives in version control and runs in CI on both sides; any deliberate asymmetry is documented as such rather than left to be rediscovered
**Performance Requirements:** None; these are decode-path guards on bounded payloads
**Determinism Requirements:** Both decoders are read-only and touch no authoritative state; the requirement is that they agree, not that either changes what it reports for valid input
**Ontology Implications:** None; transport validation only
**Observer Implications:** A payload one decoder accepts and the other refuses means a Rust client and a TypeScript client disagree about whether a session is speaking the protocol. Today they do disagree, on at least six classes of input
**Explanation Implications:** The Explanation IR is the sharpest case: the Rust decoder routes through `ExplanationReport::new`/`ExplanationFrame::new`/`ExplanationClaim::new` and enforces non-empty frames and claims, a derived assessment matching the declared one, confidence in `0.0..=1.0` and non-NaN, evidence on a Supported claim, `start <= end` on a range, a non-zero ratio denominator, and a cohort where the comparison kind requires one. The TypeScript decoder enforces none of them and additionally defaults a missing comparison kind to `None` where Rust returns `MissingField(1)`. Rust also normalises — sorting and deduplicating evidence traces, sorting frames and claims, zeroing a non-Supported claim's confidence — so the two produce different objects from identical accepted bytes
**Out of Scope:** Changing the wire format, adding fields, or altering what a valid payload means. This is about the two readers agreeing on the format that already exists
**Evidence:** Found by a systematic parity audit during `plans/production-bootstrap-receipt-closure.md` (round six), which compiled `packages/observer-protocol/src/index.ts` with the package's own `tsc` and fed identical bytes to both decoders: 156 hand-built vectors plus 6,300 mutation-fuzz vectors across all seven shared decoders. `decode_observer_snapshot`/`decodeRuntimeSummary` — the runtime summary including the bootstrap group and nested receipts — is **in parity**, 900 mutations with zero divergences, and the divergences that plan did introduce or reach were fixed there. Everything below is pre-existing on `main` and outside that plan's scope. Six root causes: (a) `packages/observer-protocol/src/index.ts:717-723` has no wrong-wire-type arm for world-snapshot field 4, which every neighbouring field has, so `080c2200` decodes there and is `DuplicateField(4)` in Rust (`crates/causafera-observer-wire/src/protocol.rs:679`); (b) no numeric narrowing anywhere in the TypeScript nested decoders — `decodeSpatialChunk`, `decodeMaterialSurfaceDelta`, `decodeMaterialSurfaceGateDelta`, `decodeMaterialSurfaceThermalDelta`, `decodeThermalFieldDelta`, `decodeFieldRaster`, `decodeQueryResponse`, `decodeConnectResponse`, `decodeStreamHeader` all use bare `Number(...)` against Rust's `to_i32`/`to_u32`/`u16::try_from`, and above 2^53 `Number()` also rounds, so the payload is misread rather than merely out of range; a `requireU32` helper already exists at `:1107` and is applied in the runtime summary only; (c) no enum validation in TypeScript — `Number(cursor.varint()) as SomeEnum` is a cast with zero runtime effect, at six sites, against Rust validating each, and Rust validates status inside the decode loop so a bad status followed by a good one still fails while TypeScript keeps the last; (d) no Explanation-IR semantic validation, sixteen distinct payloads Rust rejects and TypeScript accepts; (e) the nested-message helpers disagree on duplicates — Rust `decode_time`/`decode_digest`/`decode_chunk_scope` return on first match and never see the rest, TypeScript scans and keeps the last, which diverges in both directions including a 31-byte digest Rust rejects and TypeScript silently replaces with a valid second one, and is the only observed case where Rust accepts what TypeScript refuses; (f) `decodeDeltaBand` returns a `Float64Array` while the Rust encoder round-trips full `i64` and tests `i64::MIN`/`i64::MAX` explicitly, so any value past 2^53 is silently rounded on arrival — a contract gap rather than a decoder bug. `tools/audit/test-observer-bootstrap-decoder.mjs` is the pattern to follow: it compiles the real module and runs adversarial vectors through it under the existing `node --test` harness, and is confirmed to have teeth

## TODO-PERSIST-004: Snapshot Import Re-derives What It Can Instead of Trusting It
**Status:** Pending
**Phase:** Detailed Development — Persistence
**Priority:** High
**Dependencies:** None
**Goal:** Make `RuntimeState::import_snapshot` reject any persisted value that the same snapshot lets it re-derive or check against committed provenance, rather than trusting it because a related trace merely exists
**Acceptance Criteria:** For each field named in the evidence below, a hand-built forgery is rejected with a specific message and covered by a test with teeth; a negative control sweeps the configuration space at bootstrap and after advancement and shows no legitimate state is refused; anything deliberately left trusted is documented with the reason
**Performance Requirements:** Import cost re-measured with the repository's benchmark methodology; the current figure is in the Bounded measurement section of `plans/production-bootstrap-receipt-closure.md`
**Determinism Requirements:** Every re-derivation must be a pure function of persisted configuration or committed provenance, never of wall-clock, iteration order, or process state
**Ontology Implications:** Geography, thermal state, and mana are authoritative causal domains (INV-036, INV-041). A snapshot that dictates them is authoring world state outside the scheduler's proposal/commit boundary, which is what INV-016 forbids
**Observer Implications:** Every forged value below survives into the observer read models, so the observer reports a world that no run produced
**Explanation Implications:** Deleting the material-surface transition histories removes the bounded evidence the material-surface claims read, so Explanation degrades to its insufficiency states over fabricated state rather than reporting the fabrication
**Out of Scope:** Authenticating snapshots. `SECURITY.md` already draws the boundary at a different, self-consistent history; this is about a snapshot that contradicts the history it carries
**Evidence:** Found by adversarial audit during `plans/production-bootstrap-receipt-closure.md` (round eight). Each was reproduced against a real exported snapshot, and each **launders**: import it, tick, re-export, and the result re-imports as a clean save. The pattern in every case is that a trace-existence or event-kind check stands in for a re-derivation.

- **Terrain is never re-derived** though it is byte-for-byte re-derivable. `import_carrier_adapters` checks only `field_extent`; bootstrap builds terrain with `deterministic_terrain_chunk(terrain_seed, chunk, trace)`, where `terrain_seed` comes from `config.carrier_adapter` and is persisted in `recipe.config`. Elevations and materials depend only on `terrain_cells(seed, chunk)` — the trace feeds provenance alone — so they are re-derivable from persisted configuration without it. Replacing all 1024 elevations with a flat value and every surface material with one id is accepted.
- **The living-population identity is one equation with both sides supplied by the snapshot, on any snapshot that has advanced.** `validate_persistent_domain_state` compares the population against `bootstrap_population + births − deaths`, and both counters arrive unvalidated. On an advanced snapshot, subtracting 100 residents and adding 100 to `population_deaths` is accepted, and the committed trace store is then byte-identical to an honest run that never lost them. At bootstrap time the same forgery is already rejected by `validate_bootstrap_population_conservation`, which compares aggregates and promoted actors against `config.bootstrap_population` with no counter term at all; only the advanced-time case is open. Closing it needs the aggregate anchored to committed provenance; an attempt during round eight was reverted because `fingerprint_population_aggregate` mixes count, births, deaths **and** material flow into one fingerprint while material flow is transitioned under a different property, so no single committed effect anchors the whole aggregate. A correct fix needs either a count-only fingerprint on the population-aggregate effect — a deliberate change to the effect payload and the digest — or a replay of the aggregate's effect chain.
- **Bootstrap-time thermal cell energy is anchored by nothing.** `validate_thermal_aggregate_conservation` returns early while `batch_sequence == 0` and boundary records must be empty, so before the first batch there is no anchor beyond an event-kind check. Adding 1e6 joules to a cell is accepted, and the first conservation receipt after resume adopts the forged total as its baseline. The advanced-time equivalents are correctly rejected.
- **Mana intensity, active-chunk relevance/level/mana, resolution entries, and reservoir schedules** are all accepted at forged values; only a `last_change` trace's *existence* is checked, never the value against the committed effect.
- **`actor_objects` values** are unchecked, though positions are derived at promotion from the aggregate's chunk. Round seven closed coverage — every actor has an object — not the contents. This is the object every other actor's perception reads.
- **Domain clocks roll back freely.** `mana.observed_through` and `resolution.evaluated_through` can both be set to zero on an advanced snapshot. Round seven closed `advanced_through`/`completed_time`; these per-field clocks are compared with neither.
- **Material-surface transition histories can be cleared**, and **subjective cognition is unbound to its actor** — two actors' subjective snapshots can be swapped, or every `subjective_scene` set to `None`.
- **Lower severity:** `next_actor_id` may be pushed arbitrarily high (round seven closed rollback only); aggregate `births`/`deaths`/`material_inflow` are forgeable; the observer counters are forgeable; `recipe.seed` may disagree with `config.deterministic.world_seed` because the field is never read; `RuntimeState::import_snapshot` does not check `recipe.system_registrations` while `Runtime::from_snapshot` does.

Round eight closed two of this class inside the bootstrap plan's scope — `actor_action_bounds` contradicting `config.action_bounds`, and `ResolutionPolicy` overriding the compiled constant — because both are one comparison against something the snapshot already carries. The rest is a persistence-wide programme spanning geography, thermal, mana, resolution and cognition, and is opened here rather than absorbed round by round into a bootstrap-receipt plan.

## TODO-GEO-004: Coherent Surface Material Regions
**Status:** Completed — see `plans/coherent-surface-material-regions.md`
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
**Resolution:** `terrain_cells` (`crates/causafera-runtime/src/carrier.rs`) derived material from the same well-mixed per-cell hash driving elevation, an independent-noise assignment. Replaced with `terrain_regions::region_material`, a bounded Worley (cellular) partition: chart-scoped coarse feature points, jittered within their own coarse cell, searched over a provably-sufficient 5x5 (Chebyshev-2) neighbourhood. A pure function of chart-global position, exactly like `TODO-GEO-005`'s elevation, so regions are continuous across chunk boundaries by construction. `MATERIAL_REGION_SIZE` (16, a power of two) chosen from a same-material-rate sweep weighed against the mana column footprint. `TERRAIN_GENERATOR` and `TERRAIN_PARAMETERS` both move to `0x2409_0001`. Three pre-existing tests needed attention (none from a defect in the generator): a recipe-source isolation test that never actually set terrain `Inert`, a below-threshold test whose pinned incidental intensity moved (182 → 154, its second re-pin), and a seed-discrimination test whose hand-picked seed pair collapsed a second time, rewritten to sweep eight fixed seeds against a "does not collapse across all of them" claim instead, with its independently-too-short 48-tick duration raised to 192. Full detail in the ExecPlan
**Original evidence:** `apps/observer/src-tauri/examples/terrain_probe.rs` (no longer in the repository) measured 6.5% same-material neighbours against 6.2% expected from chance over 16 materials. `TODO-MANA-004` reached the same finding from the mana side and put a cost on it: projected onto the mana lattice, the terrain's structural variation survives at only 1.32x to 2.75x what averaging pure noise would retain, and the ratio falls as the lattice refines. There is little coherent structure for a finer field to resolve, so this is the work that would make a finer mana lattice worth its cost rather than the other way round
**Evidence:** `crates/causafera-runtime/src/terrain_regions.rs` unit tests (coherence, same-material rate never below the true same-region rate, chunk-boundary continuity, determinism, cross-chart independence) and `apps/observer/src-tauri/examples/field_probe.rs`. Same-material rate rises from 6.5%–6.75% to 93.0%–94.1% (real production run, three chunks), with the boundary rate (90.6%) the same order as the interior rate (93.1%) — no continuity artifact. Same-material slightly overstates coherence, since two different regions can draw the same material by chance; measured directly against the true same-region rate at 92.1% material vs 91.7% region — a 0.4-point gap, not several. Re-measured `extent_decision.rs` under identical seeds/ticks/threshold immediately before and after: the mana lattice's fidelity-vs-noise ratio improves at every candidate extent (e.g. extent 6: 1.15x → 1.59x; extent 8: 1.23x → 1.52x), most sharply at extents 4–8. Full sweep and decision log in `plans/coherent-surface-material-regions.md`
**Follow-on:** `TODO-MANA-004`'s "extent stays 3" decision was made against weaker fidelity numbers than this change now measures; not reopened here (cost and discrimination are unaffected and outside this plan's scope), but flagged as due for its own re-review with current numbers

## TODO-GEO-005: Terrain Continuity Across Chunk Boundaries
**Status:** Completed — see `plans/terrain-chunk-boundary-continuity.md`
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
**Resolution:** `terrain_cells` now keys its per-cell hash on the cell's chart-local position — `chunk.chunk.world_origin()` plus its local index — instead of the flat local index alone, and a new `chart_seed` varies terrain by chart rather than by chunk. Both call sites (`runtime_carrier_adapters`, `TerrainBootstrapStage::bootstrap`) stopped XOR-ing `chart_chunk_hash(chunk)` into the generation seed, since a per-chunk seed term was the second half of the original defect: it would have reintroduced a boundary jump even with a continuous ridge term. `TerrainGeneratorFingerprint` moved to `0x2407_0001`; `TerrainParameterFingerprint` did not move, since no parameter value changed. No fixture or replay capture is checked into the repository, so none needed regeneration; two downstream tests that pinned incidental values from the old generator's output were re-pointed against measured evidence rather than relaxed. `elevation_contrast` and `material_difference`, which feed the standing carrier's mana-facing magnitude, still index only within one chunk and do not see across the now-continuous boundary — opened as `TODO-GEO-006`.
**Original evidence:** `terrain_cells` in `crates/causafera-runtime/src/carrier.rs` computed `ridge = (x - y) * 17` from chunk-local `x` and `y` and took the chunk only through the seed, so every chunk repeated the same diagonal ridge. Measured on the demonstration session at seed 7: the east edge of chunk (−1, 0) read +13.1 m … +19.5 m against −13.5 m … −13.7 m on the abutting west edge of chunk (0, 0), a step of about thirty metres where the mean neighbour step inside a chunk is 1.6 m. `TODO-OBS-001` made this visible by giving the chart two dimensions and a per-cell projection; before that the map drew one tint per chunk and the strip was one chunk deep, so nothing showed it
**Evidence:** `cargo run --release -p causafera-observer --example field_probe`, seed 7, three line-shaped chunks, 48 ticks: interior step mean 1617 mm (max 4992 mm, 2976 pairs) against boundary step mean 1806 mm (max 4608 mm, 64 pairs across 2 adjacent chunk pairs) — the boundary step is now the same order as the interior step, and is in fact slightly lower, against the ~30 m step recorded above. Chunk means now form a continuous ramp across the three chunks (−32.8 m, +2.0 m, +36.9 m), the direct and intended consequence of a ridge that no longer resets at chunk edges. `terrain_is_continuous_across_chunk_boundaries` in `crates/causafera-runtime/tests/terrain_carrier.rs` pins this as a test, including the negative chunk coordinate the original evidence measured

## TODO-GEO-006: Terrain Structure Carrier Does Not See Across Chunk Boundaries
**Status:** Completed — see `plans/terrain-structure-cross-chunk-neighbours.md`
**Phase:** Detailed Development — Geography
**Priority:** Low
**Dependencies:** TODO-GEO-005
**Goal:** Let the standing terrain carrier's mana-facing structure computation read real neighbour cells across a chunk boundary, rather than only within the chunk that owns the cell being scored
**Acceptance Criteria:** An edge cell's `elevation_contrast` and `material_difference` are computed against its true chart-local neighbours, including neighbours in an adjacent chunk when one is active; a cell with no active neighbour on one side still degrades gracefully rather than panicking or silently treating the missing side as flat
**Performance Requirements:** Measured against the current within-chunk-only computation; a cross-chunk lookup must not turn `project_columns` into a cost that scales with the number of active chunks touched per cell
**Determinism Requirements:** Unchanged — the same seed and the same active chunk set must still reproduce identically
**Ontology Implications:** None; this is a derived carrier-magnitude computation, not new domain state
**Observer Implications:** None to the wire protocol; may shift the standing carrier's mana contribution near chunk edges
**Explanation Implications:** None
**Out of Scope:** Any change to elevation, roughness or material generation itself; that closed with `TODO-GEO-005`
**Resolution:** `TerrainCarrierAdapter::new` and `project_columns` now take a `BTreeMap<ChartChunkCoord, TerrainChunk>` of sibling terrain; `neighbor_cells` resolves each of a cell's four axis-aligned neighbours from the chunk's own array when interior, or from `neighboring_terrain.get(&chunk.same_chart_neighbor(dx, dy, 0))`'s corresponding edge cell when not — a missing entry drops that direction, exactly as before, rather than inventing a value. This mirrors `causafera-domains::mana`'s existing `OpenNeighbors`/`same_chart_neighbor` idiom. Recomputing a missing neighbour from the deterministic generator formula was considered and rejected: `TerrainChunk` is data, not a cached formula evaluation, and `featureless_ground_is_not_a_physical_pattern`'s hand-built fixture already proves the two can diverge. `TerrainBootstrapStage::bootstrap` and `import_carrier_adapters` — the two call sites whose adapters ever reach a live runtime or a resumed one — now generate/decode every sibling chunk before building any one adapter; `runtime_carrier_adapters`, whose output is unconditionally overwritten by bootstrap before anything reads it, passes an empty map rather than being restructured for no observable effect.
**Evidence:** `elevation_contrast`, `material_difference` and `neighbor_indices` in `crates/causafera-runtime/src/carrier.rs` used to index only `terrain.elevations()`/`terrain.surface_materials()` of the one `TerrainChunk` passed in, dropping a direction entirely at `x == 0`, `x == CHUNK_SIZE - 1`, `y == 0` or `y == CHUNK_SIZE - 1` rather than looking into the neighbouring chunk — an edge cell's `structure` was drawn from at most 2-3 real neighbours where an interior cell has 4. Found while implementing `TODO-GEO-005`. Re-measured with `cargo run --release -p causafera-observer --example field_probe` (seed 7, three line-shaped chunks): 2 of 9 lattice columns in the west-most active chunk change `structure` once its real east-adjacent sibling becomes visible, and the mana field shifts only at the boundary-touching extremes (e.g. `166..2570` to `167..2570` in that chunk). Bootstrap wall-clock is unchanged within run-to-run noise at every `active_chunk_radius` from 1 to 4 (the validated ceiling), measured against the pre-fix source with the change stashed. No existing test needed re-pointing — `different_seeds_produce_different_worlds_not_one_world_with_two_terrains` and `below_threshold_source_changes_mana_without_material_consequence_or_supported_explanation` both stayed green as rerun explicitly after the fix

## TODO-HYDRO-001: Conserved Multi-Resolution Hydrology
**Status:** Completed — see `plans/hydrology.md`
**Phase:** Detailed Development — Geography / Hydrology
**Priority:** High
**Dependencies:** Accepted `docs/rfc/RFC-HYDRO-001.md`; the terrain continuity and cross-chunk neighbour contracts closed by `TODO-GEO-005` and `TODO-GEO-006`; the provenance, persistence, and causal-resolution foundations (`TODO-TRACE-001`, `TODO-PERSIST-001`, `TODO-RES-001`)
**Goal:** Replace the inert `HydrologyCell { water_table: f32 }` placeholder with deterministic, conserved, causally inspectable hydrologic state and processes: surface, unsaturated-soil, groundwater, and unlabeled conveyance storage; explicit tick-indexed precipitation and evapotranspiration forcing; infiltration, percolation, runoff, groundwater flow, baseflow, and storage-discharge routing that crosses same-chart chunk seams; conservative hydrology-specific resolution changes; exact persistence and replay; and bounded Explanation and observer read models
**Acceptance Criteria:** Every carrier, process, conservation, provenance, resolution, persistence, observer, and Explanation requirement in `plans/hydrology.md`, verified by its V1–V34 gates. In particular: `storage_before + sources == storage_after + sinks` exactly for every tick and for the aggregate run; demotion and promotion preserve every water bucket, topology, and retained fine provenance; export/import/export bytes are identical; and a `2N`-tick run matches `N` ticks, save, import, then `N` ticks
**Performance Requirements:** Reproducible measurement of the six workloads in the plan's benchmark section, including the fine-versus-coarse resolution comparison. The retained-fine/coarse design counts as a performance architecture only if the coarse workload evaluates strictly fewer vertical process groups and internal faces than the fine workload while every conservation, replay, topology, and ancestry check stays green; timing alone is not the oracle. No absolute threshold or scale claim is declared before baseline measurement
**Determinism Requirements:** Checked integer arithmetic and ordered maps/edges throughout; every substage reads a frozen state; rounding remainders stay in donor storage or use canonical largest-remainder allocation; forcing is explicit persisted state with no wall clock and no RNG; hydrology registration is appended so legacy system IDs and RNG stream keys are unchanged; digest schema 8 deliberately distinguishes hydrology-bearing state
**Ontology Implications:** Hydrology moves from a documentation-only M0 placeholder to an executable conserved physical domain. Geography, Space/Resolution, Time, and Matter gain hydrology-facing inputs. Climate stays M0 and gains only a future-compatible output boundary — persisted hydrology forcing records. Final maturity wording is evidence-driven and is not pre-claimed
**Observer Implications:** Bounded additive protocol-V1 fields: per-chunk surface/soil/groundwater rasters with lossless unsigned bands, conveyance storage and flow summaries, latest forcing and conservation summary, bounded transfer receipts by chart-qualified scope, and trace anchors. The frozen six-receipt bootstrap field is preserved and the seventh bootstrap receipt is projected separately, so a pre-hydrology V1 decoder accepts the new payload unchanged
**Explanation Implications:** Typed claims for storage and water-table range, forcing ancestry and accepted/unmet forcing, transfer path and limiter evidence, exact conservation residual, and boundary export, with explicit insufficiency instead of narrowing or fabricated classification
**Out of Scope:** The plan's Non-goals — full climate or atmospheric generation; geological formation, strata, deformation, or aquifer classification; snow/ice accumulation and phase change; sediment, erosion, solutes, salinity, pollutants, or water quality; full Saint-Venant hydraulics, backwater, flow reversal, pressurization, coastal tides, or cross-chart ocean routing; dams, pumps, weirs, canals, irrigation, or municipal networks; ecology, agriculture, health, settlement, economy, history, biology, or mana coupling implementation; semantic water-body or hazard labels in authoritative state; observer UI work; CUDA/GPU work; and migration shims that default absent hydrology into an old production snapshot
**Outcome:** Implemented across eight stages on `feat/conserved-hydrology`. Every V1–V34 gate is covered by tests in `crates/causafera-geography`, `causafera-domains`, `causafera-runtime`, `causafera-explanation`, `causafera-observer-wire`, and the `tools/audit` decoder and boundary audits. Digest schema 7→8, runtime recipe section major 6→7, snapshot section `0x000F` v1. Measured evidence, including the ceiling the 256 MiB export cap places on session length, is in `docs/performance/benchmarks.md`; the fine-versus-coarse comparison the Performance Requirements ask for could not be driven from configuration, because the engine's resolution policy is a compiled constant rather than a setting — what was measured instead is the work at each level the engine itself chose, and that limitation is recorded rather than worked around.

## TODO-MANA-004: Mana Field Lattice Cost Decision
**Status:** Completed — `chunk_extent` stays 3. **Flagged for re-review:** `TODO-GEO-004` measured
the fidelity-vs-noise ratio this decision rests on improving at every candidate extent once surface
material became spatially coherent (e.g. extent 6: 1.15x → 1.59x); cost and discrimination are
unaffected. Not reopened by that change, but the fidelity half of this decision now rests on
superseded numbers
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
**Status:** Completed
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
**Resolution:** The Evidence's "not a clamp" reasoning was a deferral rationale, not a design mandate, and did not hold up against the code it was describing. Two direct checks settle it: the interior path (`diffuse_cell`) itself subtracts `outgoing` unconditionally and adds `incoming` unconditionally, then clamps the sum once at the end — when that clamp engages, the contributing neighbour is not refunded either. And `diffusion_alone_conserves_mana_across_a_seam`, the test the Evidence worried about reopening, sets `maximum_intensity = i64::MAX / 4` with `decay = 0`, so the clamp never engages in it; its own comment already names "the only sanctioned losses" as "decay and the clamp." `TODO-MANA-002`'s conservation defect was truncation loss (an undivided outgoing budget subtracted against truncated incoming shares, every cell, every tick) — a different mechanism from ceiling loss, which only engages when a field is actually saturated and was already a sanctioned loss on the interior path. A refund-to-giver design would in fact have made a seam cell behave *differently* from an interior cell under saturation, which is the seam-vs-interior asymmetry INV-037 forbids, just with the sign flipped. The fix instead gives seam delivery the same single end-of-computation clamp the interior path already has: `apply_exchange_delta` now clamps the running total to `0..=maximum_intensity` after every delivery. Addition is commutative, so summing every seam delta into a cell before the clamp engages is order-independent by construction — the "which of several givers is refused" problem the original Evidence raised dissolves rather than needing a rule, because no individual giver is ever singled out. Verified inert at production scale: physical digest, history digest, total mana, and every behaviour metric are byte-identical before and after the fix across all six standard seeds and both gate configurations at the production `chunk_extent` of 3 — the ceiling structurally does not engage there, exactly as the Evidence's own headroom measurement (82 679 / 1 000 000) predicted. Covered by `seam_delivery_never_exceeds_the_saturation_ceiling` in `crates/causafera-domains/src/mana.rs`, which reproduces the Evidence's exact scenario (two adjacent extent-3 chunks, every cell seeded at the ceiling) and asserts no proposed cell exceeds it. This TODO's scope is seam delivery specifically: `ManaFieldSet::propose_experiment_recipe_mana_source` still saturates at `i64::MAX`, not `maximum_intensity` (`experiment_recipe_mana_source_saturates_at_i64_maximum` pins this), because it is a separate root-event source with its own per-record maximum and recipe-wide budget (`docs/world/mana-topology.md`'s "Bounded immutable experiment-recipe source" section), not the diffusion path this TODO's Evidence measured — left as-is, not an oversight. See `plans/mana-seam-saturation-ceiling.md`

## TODO-MANA-007: Response and Gate Constants Calibrated Against an Unpopulated Field
**Status:** Completed
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
**Resolution:** The original evidence measured the wrong population: `ManaEffectsSystem::execute` reads only cell 0 of each contacted material surface's field, never the field-wide distribution `extent_decision.rs` reports. Measured against the correct population, the current constants already discriminated worlds at four of five candidate lattices, including the production default; the one real failure was the coarsest lattice tested (extent 12), where the population's mean sat above the threshold. `effect_threshold`/`effect_hysteresis` moved from 4 096/2 000 to 6 144/1 536 — the only point in a sweep that discriminates at all five candidate lattices simultaneously, confirmed as a neighbourhood plateau rather than a one-point spike, and re-verified end-to-end against real production runs on the exact five-field behaviour tuple this evidence originally used. The response channel weights, diffusion, decay, the stencil and the lattice are untouched, per Out of Scope. See `plans/mana-gate-calibration.md`
**Original evidence:** `apps/observer/src-tauri/examples/extent_decision.rs`, six seeds at 192 ticks. The constants predate any carrier that populates the field: with contact alone, seed 7 held 32 266 total mana; with standing terrain at the same lattice it holds 82 679, and at extent 12 it holds 8 117 028. Against a fixed threshold of 4 096 the share of live cells above the gate runs 0%, 0%, 8%, 11% and 24% at extents 3, 4, 6, 8 and 12, and distinct behaviour tuples across the six seeds run 3, 3, 1, 2 and 1 — the finer the lattice, the more completely the gate latches open in the first ticks and the less it can tell one world from another. Gate crossings sit at exactly 3 from extent 6 upward, which is the number of contacted surfaces rather than a response to anything. The gate is therefore the binding constraint on `TODO-MANA-004`, which was closed by keeping the lattice where the current calibration still works
**Evidence:** `apps/observer/src-tauri/examples/mana_gate_calibration.rs`, six seeds at 192 ticks, five candidate lattices. The gate's actual population (cell 0 of every contacted surface) has a mean of 2165, 2038, 3174, 4214 and 7163 at extents 3, 4, 6, 8 and 12 — only ~3.3x range, against the ~98x range of field-wide total mana across the same lattices. At the current constants (4096/2000) this population already yields 2, 2, 3, 2 and 1 distinct behaviour tuples across the six seeds; at 6144/1536 it yields 4, 3, 3, 4 and 4 — discriminating at every lattice and never worse than the current constants on the exact tuple `extent_decision.rs` uses. Full sweep, neighbourhood-robustness and hysteresis-axis tables in `plans/mana-gate-calibration.md`

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

## TODO-WORLD-003: Minimal Local Detail Vertical Slice (Parcel/Structure/Interior)
**Status:** Pending
**Phase:** Detailed Development — World
**Priority:** Low
**Dependencies:** TODO-RES-001, TODO-CITY-001, TODO-MANA-002, TODO-THERMAL-000 (completed same-chart slice)
**Goal:** Validate the Chunk → Parcel/Site → Structure → Interior Space local-detail path (`spatial-hierarchy.md`) end to end on one minimal bounded scenario, proving the promotion contract rather than building city content
**Acceptance Criteria:** One chunk, one parcel/site, one structure, several interior spaces; each interior addressed through its own bounded `LocalMetricFrame` with an explicit chart transform, not derived from `CHUNK_SIZE` or `chunk_extent`; interior physical materials exist as chart-qualified surfaces; at least one interaction with the existing thermal carrier and one with the existing mana carrier crossing the parcel/structure/interior boundary; a persistence round-trip; causal provenance for every promoted element; a bounded observer projection exposing the promoted structure with no fabricated content (INV-039)
**Performance Requirements:** Cost of one promoted structure measured against an unpromoted chunk of the same size before any claim about scaling to many structures
**Determinism Requirements:** Same seed and promotion inputs reproduce identical local geometry, material state, and provenance; demotion is lossless for every conserved quantity it retains
**Ontology Implications:** Local detail is a domain-owned promotion contract (INV-037, INV-043); no specific resident, object, or building content may be synthesized from an aggregate outside this validated contract (INV-039)
**Observer Implications:** One additive bounded projection for the promoted structure and its interiors; no protocol-wide interior/parcel query surface
**Explanation Implications:** Promotion and demotion events are trace-backed and inspectable like any other committed state change
**Out of Scope:** City/road/settlement generation, mass building construction, full indoor physics (light, acoustics, airflow), a general promotion/demotion system for arbitrary chunks, persistence schema changes beyond this slice's own section, observer-protocol changes beyond this slice's own additive query, UI work

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

**Delivered by `plans/production-bootstrap-receipt-closure.md` (2026-07-28):** All five
acceptance criteria are backed by evidence for the six stages the runtime executes today. The status
stays In Progress because the criteria are worded for *every* runtime capability, and this slice
covers six stages.

- *Production runtime/session code contains no fixture/demo constructors* — met. `fixture_actors`
  and `fixture_sensors` are removed; no test needed them. A source audit
  (`production_source_reaches_no_fixture_constructor`) scans every production `.rs` under `crates/`
  and `apps/` with comments stripped, asserts it actually walked the tree, and was confirmed to fail
  on an injected `fn demo_`. Its test-only allowlist is explicit and holds one entry, the name of a
  `#[cfg(test)]` test in `runtime.rs`.
- *Aggregate population conservation includes promoted actors* and *actor/body/object state has
  bootstrap ancestry* — met at bootstrap. Import requires aggregates plus promoted actors to equal
  the configured bootstrap population, and every promoted actor's ancestry to be a trace the
  canonical actor-promotion receipt named. Both are scoped to a store that ends at the last stage
  completion — decided by the store's shape, not by the `advanced_through` counter the snapshot
  supplies — because from the first tick the population lifecycle legitimately moves those numbers,
  so an equality afterwards would be false rather than protective. The weaker identity that does
  survive advancement is asserted at every simulation time: living population equals the configured
  bootstrap population plus births minus deaths.
- *Reset, experiment, save/resume, and observer sessions use the same production recipe* — met.
  Headless `Runtime::new`, a bare seed, a resumed snapshot, the observer session and its reset, and
  the lab experiment runner all produce the same canonical record for the same configuration, and
  resuming imports the record rather than re-bootstrapping on top of it.
- *Tests prove fixtures remain test-only* — met vacuously, since no fixture constructor exists.
- **Scope caveat, not an unmet criterion:** the criteria are worded for *every* runtime capability.
  This slice covers the six stages the runtime executes today — terrain, material surface, population, actor promotion,
  material activity, thermal — and nothing else. `TODO-DEPTH-001` and `TODO-HIST-001` are untouched
  by this slice; `TODO-HIST-001` has been Completed since Phase 21 and nothing here reopens or
  extends it.

**Bounded measurement (envelope only, AMD Ryzen 9 7950X3D, `rustc 1.97.1`, release profile):** nine
active chunks, bootstrap population 512, eight promoted actors, one sensor aperture each. Four
warm-up repetitions, twenty measured repetitions per distribution, observer control and counterpart
interleaved, every raw sample retained; medians and population standard deviations below are ranges
across three consecutive runs. The four deterministic figures are stable: complete encoded snapshot envelope 177 071 bytes of
which the bootstrap record is 1 676; 53 provenance events committed by bootstrap; observer
runtime-summary payload 436 bytes. The wall-time figures are **not restated here** — they have gone
stale in this file twice, once for the timings and once for the overhead, because each audit round
re-measures and the copy is not propagated. They live in one place, the Bounded measurement section
of `plans/production-bootstrap-receipt-closure.md`. The overhead is resolvable and reported there; earlier
passes called it unresolvable, which was a property of sequential-block measurement rather than of
the system. These are measurements of one machine at
one envelope, not a scale result and not a regression threshold, and this machine is not reference
hardware. See `docs/performance/benchmarks.md`.

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

**Delivered by `plans/production-bootstrap-receipt-closure.md` (2026-07-28):** the *bootstrap
receipts* clause of the acceptance criteria only. The runtime summary now carries a bounded
projection of the canonical bootstrap record — plan identity, world seed, stage count, validation
status, configured population/promotion bounds, and at most six receipts with completion time,
result fingerprint, completion trace, and dependency anchors. The wire fields are additive on the
existing summary, so `OBSERVER_PROTOCOL_V1` is unchanged and a payload written before them still
decodes, reporting schema version 0 for "no evidence in this payload". Both decoders bound the lists
before growing them and reject non-canonical order. Causal event slices, typed domain time series,
resolution transitions, pagination/capacity, and the observer-overhead measurement across all four
load levels remain open. The one overhead figure taken here — the bootstrap-summary encoding cost, a few hundred nanoseconds
per poll, measured in the Bounded measurement section of
`plans/production-bootstrap-receipt-closure.md` — is reported, and covers one load level, not four.

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

**Delivered by `plans/production-bootstrap-receipt-closure.md` (2026-07-28):** typed claim schemas 18
and 19 for the canonical bootstrap record — stage completeness anchored to the receipts' completion
traces, and the bounded canonical window they span. An incomplete or unevidenced record answers with
the existing unknown state at zero confidence rather than erroring, which is the insufficiency
behaviour the criteria ask for. Neither claim renders a process name, infers a purpose, or reports a
fingerprint as a numeric value. Comparison frames, counterfactual context, and alternatives are not
part of this slice and remain open.

## TODO-PERF-001: Benchmark Framework
**Status:** Completed
**Phase:** Detailed Development — Cross-cutting
**Priority:** High
**Dependencies:** TODO-ARCH-001, TODO-DEPTH-001
**Goal:** Benchmark harness
**Acceptance Criteria:** A checked-in harness reports mean/median/stddev over `N=20` repeated runs
whose case order is cyclically rotated per pass (not single-shot timing, and not a fixed order
repeated, which would leave the first case first in every pass) and per-case RSS isolated across
process boundaries (not shared-process `/proc/self/status` reads across sequential cases); it
reproduces, without the throwaway instrumentation used to find them, the two concrete findings
recorded in `plans/performance-baseline-and-digest-cost.md`: (1) **done, Waves 1-2** —
`RuntimeConfig::validate()` accepted `actor_count`/`sensor_count`/surface-contact combinations that
failed at the first tick against `causafera-cognition::MAX_SCENE_CUES`; the harness's exhaustive
sweep located that boundary, and `validate()` now rejects past it at construction with
`RuntimeError::SceneCueBudgetExceeded`, using a bound derived from the perception code rather than an
approximate formula, and (2)
`RuntimeState::snapshot`'s unconditional full-rescan `history_digest` dominates tick cost (46-87% of
tick time across the plan's measured run-length sweep) and grows unboundedly with run length by
design (the causal trace store, per INV-014); `physical_state_digest` is not the dominant share at any
measured point (2-23% in the same sweep) but has its own real, unbounded, run-length-dependent growth
term — the unpruned `thermal_receipts`/`thermal_conservation_receipts` maps (one new entry per tick, no
eviction found in the crate) — that also compounds over a long enough run even though it is currently
the smaller cost. Only `history_digest`'s growth is fixed by this plan's Wave 3.
`physical_state_digest`'s growth is written in the *middle* of its digest-write sequence, with further
arbitrarily-mutable state written after it, so it cannot use the same incremental technique without
reordering the write sequence (which would itself change the digest's output); it is left as a named,
explicitly open follow-up requiring its own design (see the plan's Non-goals). See that plan for the
measured evidence, proposed fix waves, and non-goals. Finding (2)'s `history_digest` half is **done,
Wave 3**: the trace-event scan is incremental, its value bit-identical and asserted so against a
retained full-rescan oracle, with 64 ticks after seven warm-up batches falling from 147 ms to 22 ms
and the run-length penalty from 6.7x to 1.7x. `physical_state_digest`'s unbounded thermal-receipt
growth is the residual and stays open. CI capture is **done, Wave 4**: `benchmarks.yml` runs the
harness and stores its output as an artifact named for the commit SHA, with no threshold and no
regression flag, which makes cross-commit comparison possible without performing it. What keeps this
TODO open is therefore no longer any wave of that plan — all five are complete — but the two
Reporting requirements in `docs/performance/benchmarks.md` that the plan explicitly placed out of
scope: "flagged on regression" (which needs a historical series before a threshold can be anything
but a guess) and "reproducible on reference hardware" (no such run exists). Both need their own
decision, as does `physical_state_digest`'s thermal-receipt growth.
The cue-budget bound landed in Wave 2 is
worst-case rather than exact, so it rejects some configurations that run today — `Area` charts at
radius 2 or more no longer admit 8 actors on 2 sensors — which is deliberate, since surface contact
spreading further in a longer or differently-moving run is exactly the failure the bound exists to
prevent and no configuration property rules it out.
**Performance Requirements:** Minimal measurement overhead; the harness itself must not distort the
measurements it takes (see the plan's finding on the shipped `benchmark.rs` harness's own RSS-sharing
defect).
**Determinism Requirements:** Benchmarks reproducible (INV-018); every reported number must trace to
a checked-in, re-runnable tool, not a deleted scratch probe. `history_digest`'s incremental rewrite
must produce bit-identical output to the current implementation, verified by a differential oracle
test, with no digest schema version change.
**Ontology Implications:** N/A
**Observer Implications:** Exposes performance metrics
**Explanation Implications:** N/A
**Out of Scope:** Full performance suite; reference-hardware runs; CI regression gating (capture-only
is in scope, per the plan's Wave 4); SoA conversion, scheduler parallelization, CUDA work, or any
incremental treatment of `physical_state_digest` (all of it, not only its current-state-bounded terms
— its unbounded thermal-receipt terms are a real, measured gap this TODO's plan identifies but does
not fix, pending a separate design decision; see the plan's Non-goals and Decision log)
**Closed because:** every in-scope criterion is met — the harness exists and is checked in, both
findings are reproducible without throwaway instrumentation, finding (1) is fixed at construction
time, finding (2)'s `history_digest` half is fixed bit-identically, and CI captures the output per
commit. The three things still open were each named Out of Scope above before the work started, and
are carried forward as `TODO-PERF-002` and `TODO-PERF-003` rather than left implicit in a Pending
status that no remaining wave would ever clear.

## TODO-PERF-002: Unbounded Thermal Receipt Growth in `physical_state_digest`
**Status:** Pending
**Phase:** Detailed Development — Cross-cutting
**Priority:** Medium
**Dependencies:** TODO-PERF-001
**Goal:** Bound `physical_state_digest`'s per-tick cost over a run's length
**Acceptance Criteria:** `RuntimeState::physical_state_digest` no longer grows without bound with run
length. `thermal_receipts` and `thermal_conservation_receipts` gain one entry per tick from
`ThermalEvolutionSystem::execute` with no eviction, pruning or truncation anywhere in the crate, and
the digest re-walks both on every tick and every observer poll. Measured once, at `TODO-PERF-001`'s
Wave 3 checkpoint: about 6.3 ms of `baseline_batch0`'s 13.0 ms and 8.0 ms of `baseline_batch7`'s
22.4 ms per 64 ticks, making it the largest single named cost now that the trace scan is incremental.
Those are one run's figures derived from a per-call mean, and the harness moves between runs by more
than its own stddev on some cases, so re-run `digest-cost` for a current baseline before working
against them rather than treating them as a fixed reference. The technique that
fixed `history_digest` does **not** apply: these maps are written in the *middle* of the digest's
write sequence, with further arbitrarily-mutable current-tick state written after them, so there is no
resume point past which the output depends only on append-only growth. A fix requires one of three
approaches, and picking one is the substance of this TODO rather than an implementation detail: (a) a
retention or compaction policy bounding how many receipts stay retained, which is a domain decision
about how much thermal history must remain reconstructable rather than a performance change; (b)
reordering the write sequence so the unbounded maps come last, paired with a deliberate digest schema
migration and a persistence-compatibility plan for existing snapshots; or (c) a composable digest
primitive able to combine independently-computed partial digests, which `CanonicalDigest` does not
provide. Evidence for the chosen approach must come from the checked-in harness, not a scratch probe.
**Performance Requirements:** Per-tick digest cost must not grow with the number of ticks already run
**Determinism Requirements:** Any change to the digest's byte output requires an explicit
`CURRENT_DIGEST_SCHEMA_VERSION` bump and a stated persistence-compatibility position (INV-007,
INV-038); an approach that keeps the output identical must be verified by a differential oracle test
against a retained full-rescan reference, as `TODO-PERF-001`'s Wave 3 was
**Ontology Implications:** Approach (a) decides what thermal history remains reconstructable, which is
a domain question, not a caching one
**Observer Implications:** Approach (b) changes the digest every observer poll reports
**Explanation Implications:** N/A
**Out of Scope:** `history_digest` (already incremental); any change to thermal physics itself

## TODO-PERF-003: Benchmark Regression Flagging and Reference Hardware
**Status:** Pending
**Phase:** Detailed Development — Cross-cutting
**Priority:** Medium
**Dependencies:** TODO-PERF-001
**Goal:** Close the two `docs/performance/benchmarks.md` Reporting requirements CI capture does not
**Acceptance Criteria:** `benchmarks.md` requires benchmark results to be stored, compared across
commits, flagged on regression, and reproducible on reference hardware. `TODO-PERF-001`'s Wave 4
stores them as a per-commit artifact, which makes comparison possible but performs neither the
comparison nor the flagging, and no reference-hardware run exists. This TODO adds a regression signal
derived from an actual historical series rather than a guessed threshold — the harness already reports
stddev per case, so the threshold should be expressed against observed run-to-run spread — and a
documented run on the reference machine named in `benchmarks.md`. Deliberately not started earlier: a
threshold chosen before any series exists measures nothing.
**Performance Requirements:** The regression check must not itself distort the measurement it gates
**Determinism Requirements:** Every flagged number must trace to the checked-in harness (INV-018)
**Ontology Implications:** N/A
**Observer Implications:** N/A
**Explanation Implications:** N/A
**Out of Scope:** Full performance suite; a CI gate that blocks merges before the series justifies its
threshold

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
