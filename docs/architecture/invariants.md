# Hard Invariants

These invariants are non-negotiable architectural constraints. Violating any invariant means the system is no longer Ontopolis.

## INV-001: No omniscient agents

Agents do not have direct access to Ground Truth. They observe through physical sensory acquisition, which is incomplete and lossy.

## INV-002: Observation is not Ground Truth

What an agent perceives may differ from what objectively exists. The engine must maintain this separation structurally, not merely as a comment.

## INV-003: Belief is not magic

Mana cannot inspect belief state. Beliefs are subjective representations inside agents. They do not directly alter the physical world. Only the physical and informational structures produced by behaviour may affect mana.

## INV-004: Mana cannot inspect semantic concepts

Mana responds to real patterns such as repetition, frequency, geometry, and synchronization. It does not understand words, categories, or meanings.

## INV-005: Developer analytical labels are not agent concepts

An observer classifier may label a body structure as "finger-like". That label belongs to the Explanation Engine. It is not stored inside the agent's conceptual system and it does not affect simulation state.

## INV-006: Simulation has no privileged human UI language

English, Russian, Ukrainian, or any other UI language is not part of authoritative simulation semantics. The simulation must never depend on English strings such as "finger", "disease", or "warrior".

## INV-007: Changing observer locale cannot change simulation state hash

Identical simulation inputs executed with different observer UI locales must produce identical canonical simulation state hashes. Human-readable glosses belong exclusively to observer and explanation systems.

## INV-008: Language decoding does not directly transfer speaker concepts

The listener does not receive communicative intent directly. Meaning must be reconstructed through phonological decoding, lexical candidate recognition, and contextual semantic hypotheses. Misunderstanding is a normal process.

## INV-009: Geography is causal state

Geography modifies material availability, pathogen ecology, trade routes, mana topology, language contact, and political boundaries. It is not decorative background.

## INV-010: Distance is not simulation resolution

Causal relevance determines resolution, not only physical distance. A village 5 km away may remain aggregated while a monastery 600 km away requires detail because its practices are widely copied.

## INV-011: LLMs are non-authoritative

LLMs may improve sentence flow, tone, and narrative readability in the explanation pipeline. They may not inspect raw authoritative state independently, discover causal relationships, resolve uncertainty, invent missing events, or modify history.

## INV-012: Explanation systems are non-authoritative

The Explanation Engine converts structured simulation state into human explanations. It never modifies, invents, or becomes authoritative over simulation state.

## INV-013: Observer classifications cannot feed back into simulation

Analytical classifications produced for human observers cannot alter simulation state without an explicit physical or experimental intervention API. The observer layer is read-only with respect to authoritative state.

## INV-014: Provenance is first-class

Every significant state change must retain traceable causal history. Historical phenomena must be reconstructable from stored provenance, not reconstructed from narrative.

## INV-015: No random high-level history

Historical outcomes such as wars, plagues, or discoveries must not be generated through high-level random event tables. They must emerge from lower-level causal processes.

## INV-016: Authoritative mutation is phase controlled

All modifications to authoritative simulation state occur through defined scheduler phases with explicit proposal and commit semantics. No ad hoc mutation.

## INV-017: Performance is architectural

Performance characteristics must be considered during design, not patched afterward. Data layout, cache locality, and deterministic batch execution are architectural concerns.

## INV-018: Scale claims require reproducible benchmarks

Any claim about simulation scale, throughput, or emergence must be backed by reproducible benchmarks. Admiring random output is not sufficient.

## INV-019: Emergence must be inspectable

A surprising phenomenon must be reconstructable as a causal history. The explanation system must be able to trace how it emerged.

## INV-020: Narrative is downstream

Human-readable narrative is produced by the Explanation Engine from causal structure. It is not generated as lore and then backfilled with justification.

## INV-021: UI is an observer

The desktop application is an observer of the simulation, not a controller of it. It receives derived data through the observer protocol. It never directly reads simulation internal storage.

## INV-022: Rendering representation is not simulation state

Visual representations, map icons, and UI labels are rendering artifacts. They do not alter what exists in the simulation.

## INV-023: World generation has provenance

Generated world state must retain traceable provenance connecting it to generation parameters and causal synthesis steps. Do not invent lore prose as authoritative history.

## INV-024: Human social categories are not assumed to match biological Ground Truth

The engine may represent biological population lineages with distributions. Agents and societies may construct categories such as "elf" or "demon". Their boundaries may not match objective biological population structure.

## INV-025: Knowledge of a technology does not imply capability to reproduce it

An agent may know that microorganisms cause disease without having access to microscopes, glass purity, sterile tools, experimental infrastructure, or scientific credibility. Technology requires concept, materials, tools, measurement, procedural knowledge, and social transmission.

## INV-026: An explanation must expose confidence and supporting provenance

Every analytical classification and explanation claim must include confidence levels and references to supporting causal traces. Do not turn uncertain analytics into confident human statements.

## INV-027: Agents do not directly perceive authoritative entity identity

Authoritative identifiers such as `EntityId`, `BodySegmentId`, or `PlaceId` are Ground Truth bookkeeping. They are not subjective knowledge. Agent cognition must not consume these IDs directly as perceived object identities.

Authoritative identity identifiers must not be stored as semantic or inferential content in agent cognitive state. Any Ground Truth to perceived-identity correspondence is external bookkeeping and is inaccessible to the agent. The agent may believe an object is the same one it saw yesterday, but that belief is expressed through continuity confidence, appearance signatures, and relationship associations — not through a hidden guess about an authoritative ID.

## INV-028: Perceived object identity is a subjective hypothesis

An agent may maintain a subjective hypothesis that two distinct authoritative entities are the same object, or that one object is multiple objects, or that an object has been replaced, stolen, or destroyed. Subjective identity tracking may be wrong. It must remain structurally distinct from authoritative identity.

## INV-029: Agents act on a constructed subjective scene

Agent cognition and decisions must not directly consume raw Ground Truth or an unstructured global list of features as a complete world model. There must be an explicit intermediate layer in which the agent constructs a transient, agent-specific model of the currently experienced situation.

## INV-030: Subjective scene content must be causally grounded

Every element of a subjective scene must derive from physically accessible input, memory, concepts, prediction, self-state, or explicit inference. Subjective construction may reorganize and interpret information, but it must not fabricate content without causal ancestry.

## INV-031: Subjective detail cannot introduce inaccessible information

The cognitive system cannot invent sensory or factual information absent from the agent's physically accessible state. If smell is not represented at any relevant level, an agent cannot subjectively smell bread. Emergence reorganizes available information; it does not create information from complete absence.

## INV-032: Persistent autobiographical memory is not continuously active context

Cold historical memory and active cognition are distinct. An agent may possess decades of stored experience while only a small active context affects the current cognitive step. The architecture must not equate stored memory with currently active working context.

## INV-033: The self-model is subjective

An agent's persistent model of itself may disagree with authoritative state and historical outcomes. Believed abilities, autobiographical continuity, social identity associations, and expected behavioural tendencies are constructed, not read from Ground Truth.

## INV-034: Objective body state and subjective body schema are distinct

Biological Ground Truth contains structural and physiological state. Cognition receives information through appropriate sensory and internal-access mechanisms and constructs a body schema from proprioception, pain, balance, and learned boundaries. Do not give cognition direct omniscient access to complete biological state as complete self-knowledge.

## INV-035: Prediction error is a first-class cognitive driver

Prediction error may affect attention, salience, memory encoding, concept revision, and causal inference. The architecture must treat prediction-surprise as a signal that propagates through cognitive systems, not as a cosmetic annotation.

## INV-036: Spatial coordinate scope is explicit

Local physical space is three-dimensional. Global geography is a charted planetary surface with elevation and selective local volumetric detail. Bare local Cartesian or chunk coordinates must never be treated as a unique global planetary embedding. Chart seams, frame bounds, curvature, and coordinate transforms are explicit physical contracts.

## INV-037: Geometry is not containment or resolution

Containment, physical geometry, ownership/jurisdiction, rendering, and causal resolution are structurally separate. A hierarchy edge does not define shape or distance. A resolution transition may change representation detail but cannot alter topology, metric distance, geometric adjacency, or physical extent without a committed physical process.
