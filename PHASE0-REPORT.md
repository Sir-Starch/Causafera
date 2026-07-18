# Causafera Phase 0 Completion Report

> This document is a historical record from Phase 0 completion (2026-07-12), now superseded by the
> Detailed Development Program. Causafera is **Experimental pre-alpha** software.

**Date:** 2026-07-12
**Status:** Phase 0 Foundation Complete

## Verification Summary

- [x] Rust workspace builds
- [x] Tests pass
- [x] Clippy passes with -D warnings
- [x] Formatting passes
- [x] Non-CUDA build works
- [x] Frontend shell structure present
- [x] All ontology documentation exists
- [x] Domain Coverage Matrix exists
- [x] Causal Carrier documentation exists
- [x] Primitive vs emergent distinction documented
- [x] Language-independent simulation invariant exists
- [x] Project thesis exists
- [x] Isekai target outcomes documented
- [x] Geography architecture exists
- [x] Biology architecture exists
- [x] Cognition architecture exists
- [x] Language architecture exists
- [x] Language bootstrap requirements exist
- [x] Translation architecture exists
- [x] Epistemic architecture exists
- [x] Metrology requirements exist
- [x] Metaphysics research architecture exists
- [x] Explanation Engine architecture exists
- [x] Explanation IR documented
- [x] Observer analytical ontology documented
- [x] Deterministic renderer architecture exists
- [x] LLM surface limitations documented
- [x] Causal resolution documented
- [x] Observer architecture documented
- [x] Protocol boundary exists
- [x] Observer shell builds
- [x] Roadmap exists
- [x] Detailed TODO backlog exists
- [x] Required RFC backlog exists
- [x] Initial ADRs exist
- [x] AGENTS.md exists
- [x] PLANS.md exists
- [x] No fake simulation content was implemented
- [x] No English semantic labels were introduced into authoritative simulation state
- [x] No unverified performance or emergence claims are made

## Phase 0 Review Questions

### 1. Which files preserve the unique Causafera thesis?

- `docs/vision/project-thesis.md` — Core thesis: societies construct subjective causal models which alter behaviour, producing physical structures that mana responds to
- `docs/vision/uniqueness.md` — Key differentiators: causal reconstructability, no semantic shortcuts, language is physical, observation is not Ground Truth, geography is causal
- `AGENTS.md` — Enforces the thesis for all future developers and AI agents

### 2. How does the project prevent semantic enums from replacing emergent concepts?

- `docs/ontology/primitive-vs-emergent.md` explicitly documents the boundary
- `docs/vision/isekai-targets.md` shows the preferred historical process: social category → training → practices → mana coupling → classification
- Hard invariant INV-005: "Developer analytical labels are not agent concepts"
- `docs/ontology/domain-coverage-matrix.md` requires coverage analysis before any domain enters implementation
- AGENTS.md rule: "Never introduce semantic domain enums merely for convenience"

### 3. What objective primitives does the simulation currently assume?

From `docs/ontology/primitive-vs-emergent.md`:

- space, time, matter, position, orientation, motion
- energy-related state, temperature, material composition
- structural connection, biological structure, field state
- repetition, frequency, sequence, proximity, containment, transformation

### 4. Where is the primitive vs emergent boundary documented?

- Primary: `docs/ontology/primitive-vs-emergent.md`
- Cross-reference: `docs/ontology/domain-coverage-matrix.md`
- Enforcement: `docs/architecture/invariants.md` (INV-005, INV-024)
- Process: Every ExecPlan must include a "Primitive vs emergent review" section

### 5. Why do simulated inhabitants not know English?

- `docs/vision/project-thesis.md` and `docs/language/architecture.md` establish that English is not part of authoritative simulation semantics
- INV-LANG-001: "Simulation has no privileged human interface language"
- `docs/language/language-bootstrap.md` specifies that initial languages are generated, not manually authored as English dictionaries
- Test concept: identical inputs with different UI locales must produce identical canonical state hashes

### 6. Where do initial languages come from?

- `docs/language/language-bootstrap.md` documents the historical bootstrap approach
- Pre-simulation historical synthesis at lower resolution generates:
  - existing concept distributions
  - language communities, phonologies, lineages
  - established lexeme lineages, grammatical structures, writing systems
- The bootstrap uses constrained causal synthesis, not manual dictionary creation
- RFC-LANG-001 documents the full bootstrap requirements

### 7. How can a new word emerge?

- `docs/language/lexical-innovation.md` documents the process:
  1. communicative intent requires a concept reference
  2. no sufficiently established lexeme exists
  3. lexical pressure builds
  4. communication strategy selected (description, composition, derivation, metonymy, borrowing, novel root)
  5. phonotactic generator creates a novel form respecting the language's constraints
- `docs/language/communicative-pressure.md` explains when words are created

### 8. How does a listener infer an unknown word's meaning?

- `docs/language/communication.md` and `docs/language/semantic-layer.md` document the decoding process:
  1. speaker concept state → semantic message construction → language encoding → physical utterance
  2. listener sensory acquisition → phonological decoding → lexical candidate recognition
  3. contextual semantic hypotheses with probability distributions
  4. listener interpretation
- The listener does not receive speaker intent directly
- Meaning must be reconstructed from context and prior associations
- Later uses update these associations (learning)

### 9. Why does a lexeme not contain a single objective `meaning` field?

- `docs/language/lexicon.md` and `docs/language/semantic-layer.md` explain:
  - Meaning differs between agents
  - Each agent has `AgentLexiconEntry` with weighted semantic associations
  - Community-level analytics show usage distributions (e.g., Concept A: 48%, Concept B: 29%)
  - There may be no single correct meaning
  - Supports polysemy, homonymy, synonym competition, semantic drift

### 10. How can semantic drift occur?

- `docs/language/semantic-drift.md` documents mechanisms:
  - polysemy resolution: one sense becomes dominant
  - contextual specialization
  - metonymy and metaphor
  - social category reanalysis
  - euphemism treadmill
- Community usage distributions shift over time
- Different speakers may weight associations differently

### 11. How can language change affect magic?

- `docs/language/language-change.md` and `docs/vision/project-thesis.md` explain:
  - Language is physically relevant because utterances create acoustic and temporal patterns
  - Mana does not understand words but responds to: frequency, timing, repetition, synchronization, stable sequences
  - Example causal chain: historical vowel shift → changed acoustic signature → changed mana coupling → gradual spell instability
  - Agents may interpret this as declining discipline, divine punishment, or corrupted teaching

### 12. How are speaker intent and listener interpretation separated?

- `docs/language/communication.md` documents the full pipeline with four separate levels:
  1. belief
  2. communicative intent
  3. utterance
  4. listener interpretation
- INV-008: "Language decoding does not directly transfer speaker concepts"
- The listener cannot access speaker intent directly
- Misunderstanding is a normal process

### 13. How is a familiar UI word such as "finger" produced without entering simulation state?

- `docs/explanation/analytical-ontology.md` and `docs/simulation/perceptual-features.md` explain:
  - Authoritative data: substructure 41, attached to structure 18, distal depth 3, articulated, participates in grasp actions
  - Generic extractor produces: PERIODIC_CHANGE on perceived_substructure_19, frequency_band: 71
  - Observer classifier: "finger-like body structure" confidence: 0.96
  - The UI displays: "Finger"
  - The simulation contains no English `Finger` concept
  - The gloss is produced by the non-authoritative Explanation Engine

### 14. What is the observer analytical ontology?

- `docs/explanation/analytical-ontology.md` documents:
  - Observer MAY contain human-designed analytical categories: body-part-like structure, periodic motion, disease-like pattern, social category, occupational category
  - These are observer classifications, not Ground Truth domain labels
  - They are used for Explanation IR generation and UI glossing
  - They must never be fed back into simulation state

### 15. What is Explanation IR?

- `docs/explanation/explanation-ir.md` documents:
  - Structured intermediate representation: PhenomenonExplanation
  - Contains: subject, classification, display_label, origin, key_associations, historical_transitions, confidence
  - Supports: typed claims, evidence references, causal trace references, alternative interpretations, temporal ranges, perspectives
  - No human prose belongs in Explanation IR
  - Used by deterministic renderer to produce human text

### 16. How are uncertain human glosses represented?

- `docs/explanation/confidence.md` documents:
  - Every classification includes confidence value
  - High confidence: "finger"
  - Moderate confidence: "finger-like body structure"
  - Low confidence: "distal articulated body structure"
  - Rendering respects uncertainty, never turns uncertain analytics into confident statements

### 17. How is understandable text generated without an LLM?

- `docs/explanation/deterministic-rendering.md` documents:
  - Template-based rendering from Explanation IR
  - Uses localization resources and analytical glosses
  - Example template: "[ConceptLabel] originally referred to [origin_feature_summary]. Over [duration], its use became associated with [later_association]."
  - Deterministic: same IR + same locale → same text
  - Preserves uncertainty, avoids inventing claims

### 18. What is an LLM allowed to do in the explanation pipeline?

- `docs/explanation/optional-llm-surface.md` documents:
  - Allowed: improve sentence flow, tone, narrative readability
  - Not allowed: inspect raw authoritative state, discover causal relationships, resolve uncertainty, invent missing events, modify history
  - Pipeline: Explanation IR → validated fact packet → LLM wording → UI prose
  - Every LLM output remains associated with its source fact packet
  - UI can expose structured source data
  - LLM use is optional; Causafera must work without it

### 19. Which invariant prevents LLM explanations from becoming history?

- INV-011: "LLMs are non-authoritative"
- INV-012: "Explanation systems are non-authoritative"
- INV-013: "Observer classifications cannot feed back into simulation without an explicit physical or experimental intervention API"
- INV-020: "Narrative is downstream"

### 20. Why must UI locale changes preserve simulation state hashes?

- INV-LANG-001: "Simulation has no privileged human interface language"
- INV-007: "Changing observer locale cannot change simulation state hash"
- English, Russian, Ukrainian, or any UI language is not part of authoritative simulation semantics
- Human-readable glosses belong exclusively to observer and explanation systems
- Identical simulation inputs with different observer UI locales must produce identical canonical simulation state hashes

### 21. How is geography causal?

- `docs/world/geography-philosophy.md` documents:
  - Geography modifies: material availability, pathogen ecology, trade routes, mana topology, language contact, political boundaries
  - INV-009: "Geography is causal state"
  - Materials must maintain geographic provenance through the full chain: geological formation → deposit → quarry → extraction lot → transport batch → merchant inventory → workshop → building component
  - Historical phenomena can be traced back to a deposit

### 22. What is the Causal Resolution Field?

- `docs/architecture/determinism.md` and `docs/ontology/cross-domain-interactions.md` reference it
- `RFC-RES-001` documents the full model
- Simulation resolution depends on causal relevance, not only physical distance
- Relevance dimensions: physical proximity, trade connectivity, migration flow, social connectivity, information flow, political influence, material dependency, mana coupling, historical relevance, observer research focus
- Example: a village 5 km away may remain aggregated; a monastery 600 km away may require detail if its practices are widely copied

### 23. How are biological populations separated from social fantasy-race concepts?

- `docs/biology/populations.md` documents:
  - Ground Truth: distributions (lifespan tendencies, fertility, development timing, sensory ranges, morphology, metabolism, mana coupling)
  - Agent/social concepts: elf, human, half-elf, demon
  - Boundaries may not match objective biological population structure
  - Supports: biological continua, mixed ancestry, incorrect taxonomies, social classification conflicts
  - INV-024: "Human social categories are not assumed to match biological Ground Truth"

### 24. How is Earth knowledge separated from reproduction capability?

- `docs/isekai/imported-priors.md` documents:
  - Knowledge types: declarative, procedural, perceptual, motor
  - Example: an arrival may know "microorganisms can cause disease" but lack microscopes, glass purity, sterile tools, experimental infrastructure, scientific credibility
  - Technology requires: concept + materials + tools + measurement + procedural knowledge + social transmission
  - An arrival primarily introduces foreign priors and search directions
  - This can alter causal inference without immediately producing technology

### 25. How can familiar isekai systems such as classes or Status emerge without primitive engine support?

- `docs/vision/isekai-targets.md` documents the preferred historical process:
  - social category → standardized training → shared equipment → synchronized practices → local mana coupling → stable characteristic effects → institutional classification → later Status-system representation
- The simulation may eventually produce a concept functionally similar to a class
- The engine does not start with `Class`
- All target phenomena (Status, levels, skills, classes, dungeons, artifacts, magical schools) must emerge from lower-level causal processes

### 26. Where are unresolved metaphysical assumptions documented?

- `docs/metaphysics/identity.md` — Identity persistence
- `docs/metaphysics/death-and-persistence.md` — Death and post-biological patterns
- `docs/metaphysics/cross-world-continuity.md` — Cross-world continuity
- `docs/metaphysics/attractors.md` — Mana attractors
- `docs/metaphysics/gods-and-spirits.md` — Gods and spirits as research hypothesis
- `docs/metaphysics/artifacts.md` — Artifact formation
- `docs/ontology/unresolved-assumptions.md` — Summary of all unresolved assumptions
- RFC-META-001 and RFC-META-002 document specific research hypotheses

### 27. How does the Domain Coverage Matrix prevent forgotten foundational systems?

- `docs/ontology/domain-coverage-matrix.md` requires every fundamental domain to answer 14 questions:
  1. What objectively exists?
  2. Where is authoritative state stored?
  3. What processes modify it?
  4. What moves or propagates?
  5. How can the state be physically observed?
  6. How may agents conceptualize it?
  7. How can information about it spread?
  8. How is it represented under causal resolution?
  9. How is provenance preserved?
  10. How is it exposed to the observer layer?
  11. What are its performance risks?
  12. What are its deterministic requirements?
  13. Which other domains can it affect?
  14. Which other domains can affect it?
- A foundational domain cannot enter implementation without coverage analysis
- The matrix explicitly lists all domains with their implementation phases

### 28. Which RFCs block foundational implementation?

The following RFCs must be accepted before their respective domains can be implemented:

- RFC-ONTO-001: Primitive Simulation Ontology → Blocks Phase 2
- RFC-CONCEPT-001: Sparse Subjective Concept Formation → Blocks Phase 8
- RFC-LANG-001: Historical Language Bootstrap → Blocks Phase 10
- RFC-LANG-002: Lexical Innovation and Semantic Mapping → Blocks Phase 11
- RFC-PRACTICE-001: Evolvable Practice Representation → Blocks Phase 12
- RFC-MANA-001: Minimal Information-Sensitive Field Model → Blocks Phase 14
- RFC-RES-001: Causal Resolution and State Aggregation → Blocks Phase 15
- RFC-GEO-001: Minimal Causal Geological World Model → Blocks Phase 4
- RFC-HYDRO-001: Multi-Resolution Hydrology → Blocks Phase 4
- RFC-BIO-001: Minimal Biological Structural Model → Blocks Phase 5
- RFC-EPI-001: Measurement and Metrology → Blocks Phase 13
- RFC-ISEKAI-001: Cross-World Transfer Model → Blocks Phase 19
- RFC-META-001: Identity and Post-Biological Persistence → Blocks Phase 20
- RFC-META-002: Stateful Mana Attractors → Blocks Phase 20
- RFC-EXPLAIN-001: Observer Analytical Ontology and Explanation IR → Blocks Phase 22

### 29. What code currently exists?

- **22 Rust crates** with Cargo.toml and lib.rs:
  - `causafera-types`: Typed IDs (AgentId, ConceptId, etc.), SimulationTime
  - `causafera-core`: Scheduler, Phase enum, DeterministicConfig
  - `causafera-world`: World hierarchy stub
  - `causafera-geography`: TerrainCell, GeologyLayer, HydrologyCell, ClimateCell stubs
  - `causafera-biology`: BodySegment, PhysiologyState, PopulationLineage stubs
  - `causafera-cognition`: AttentionState, MemoryStore, Belief stubs
  - `causafera-language`: AgentLexiconEntry, PhonemeInventory, GrammarFrame stubs
  - `causafera-epistemics`: Measurement, Document stubs
  - `causafera-isekai`: TransferConfig, TransferType
  - `causafera-metaphysics`: IdentityState stub
  - `causafera-resolution`: ResolutionConfig stub
  - `causafera-domains`: EconomyNode, Practice, ManaField stubs
  - `causafera-accelerate`: Accelerator trait
  - `causafera-accelerate-cuda`: CudaAccelerator stub
  - `causafera-persistence`: Snapshot serde type
  - `causafera-analytics`: SimulationMetrics stub
  - `causafera-explanation`: PhenomenonExplanation, AnalyticalClassification stubs
  - `causafera-observer-api`: ObserverQuery, ObserverStream stubs
  - `causafera-observer-wire`: ProtocolHandler stub
  - `causafera-runtime`: Runtime composition root
  - `causafera-lab`: ExperimentConfig stub
  - `causafera-cli`: CLI with doctor, lab, run commands
- **10 `.proto` files** for observer protocol v1
- **Tauri observer shell** with React components (ConnectionStatus, SimulationControls, WorldViewport, InspectorPanel, TimelinePanel, ExplanationPanel)
- **xtask** CI runner
- **Integration tests** for typed IDs, determinism, explanation IR

### 30. What was deliberately not implemented?

Per specification section 81 and 85, the following were explicitly excluded from Phase 0:

- Simulated residents
- Concept clustering
- Language generation
- Words / fake language dictionaries
- Mana field simulation
- Biology simulation
- Terrain generation
- Economy simulation
- Historical events
- Isekai arrivals
- Fake explanation examples as live data
- Fake map
- Fake people
- Fake history
- Hunger systems
- Random jobs
- Fake cities
- Placeholder magical schools, classes, races, diseases, religions, social categories

### 31. Does Causafera currently make any unverified performance or emergence claims?

No. The project contains only documented target emergent phenomena (not claims that they currently occur) and explicit performance metrics to be measured once the relevant systems exist. No benchmarks have been run that would produce claims. The performance philosophy states that "a million inert agents are not success" but does not claim any specific current performance.

### 32. What is the first READY Phase 1 TODO according to the actual dependency graph?

Per `docs/development/todo-backlog.md`, the first ready Phase 1 TODO with all dependencies satisfied is:

**TODO-CORE-001: Deterministic Scheduler**
- Phase: 1
- Priority: High
- Dependencies: TODO-ARCH-001 (completed)
- Goal: Implement deterministic simulation scheduler with phase control
- Status: Ready to start

Other ready Phase 1 TODOs:
- TODO-CORE-002: Typed ID System (completed)
- TODO-COORD-001: Coordinate Primitives

## Next Steps

Do not begin Phase 1. Select the next task from the actual TODO backlog.

The first recommended task is **TODO-CORE-001: Deterministic Scheduler** as it unblocks all subsequent simulation work.
