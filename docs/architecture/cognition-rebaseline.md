# Cognition Rebaseline: Subjective Scene and Cognitive Continuity

**Status:** Accepted

**Scope:** Cognitive architecture for Phases 6–12 and beyond

**Previous baseline:** Phases 1–5 remain completed and unchanged. This document clarifies a missing architectural layer; it does not invalidate prior work.

---

## Context

Phases 1–5 established the deterministic kernel, ontology primitives, spatial world, causal geography, and biological structural model. These phases are completed and structurally sound.

A recent architectural analysis identified a gap: the existing documentation and planned roadmap described a direct progression from generic perceptual features to subjective concepts and beliefs, without an explicit intermediate layer for how an agent constructs a continuous subjective situation from scattered perceptual input.

The previous architecture was not invalid. It correctly described:

- Ground Truth;
- physical access and sensory acquisition;
- generic perceptual feature extraction;
- subjective concepts;
- beliefs;
- causal inference.

What it did not explicitly describe was the **subjective scene construction** layer that must sit between generic features and concepts/beliefs.

---

## The Gap

An agent does not receive a semantically labeled situation from the engine. It receives:

- incomplete sensory input;
- generic structural patterns (change, magnitude, periodicity, spatial relation);
- internal signals (proprioception, pain, balance);
- memories, when cued;
- predictions, when active.

From these lower-level accessible structures, the agent must construct a transient, coherent model of the currently experienced situation. This model is what the agent actually thinks and acts upon.

**Example:**

Ground Truth does not contain `Table`. It contains material and spatial structure. The agent perceives geometry, stability, spatial relations, and interaction outcomes. The agent links these to a stored concept prototype and subjectively experiences an object as an instance of a familiar category.

**Example:**

> An apprentice holds a tool like the agent's dead father.

This situation does not exist as an authored event or semantic engine label. It may emerge from:

```text
current perceived motion pattern
    → similarity to stored episodic percept
    → memory reactivation
    → identity association
    → attention shift
    → current subjective scene change.
```

Runtime state and accumulated history may therefore produce situations much richer than explicit semantic categories in source code.

However, subjective detail cannot invent information absent from the agent's physically accessible state, memories, concepts, or inference. If smell is not represented at any relevant level, an agent cannot subjectively smell bread. Emergence reorganizes available information. It does not create information from complete absence.

---

## Why an Agent Must Exist in a Subjective Situation

An agent must exist in a subjective situation rather than merely process a collection of independent features because:

1. **Features are not a world model.** A list of `Periodicity` on target A and `SpatialRelation` on target B does not tell the agent that A and B are part of the same situation, that A is a threat while B is an opportunity, or that the agent's own body state makes interaction with A urgent.

2. **Identity is constructed, not given.** The engine knows `EntityId(81)`. The agent may believe this is the same object it saw yesterday, or a different one, or a replacement. These hypotheses affect action. They must be represented somewhere between raw features and high-level concepts.

3. **The body is experienced, not merely measured.** Ground Truth contains segment lengths and orientations. The agent experiences reach, pain, fatigue, and balance. The experienced body model may disagree with objective state (e.g., phantom limb, misjudged capability).

4. **Memory is not continuously active.** Decades of autobiographical history exist in storage. Only a small active context affects the current step. The architecture must separate cold storage from hot context, or agents will have implausibly complete and instant access to their entire history.

5. **Prediction is a first-class driver.** Expectations about the near future affect attention, salience, and memory encoding. Prediction error is not an afterthought; it is a signal that reorganizes what the agent currently experiences.

6. **Temporal continuity binds ticks together.** The current subjective situation is not an isolated simulation tick. It contains relevant recent past, current scene, and expected near future. Anxiety, relief, and surprise all depend on this bounded temporal envelope.

Without an explicit subjective scene layer, future implementers will naturally build cognition that consumes `Feature` lists and `EntityId` values directly, producing agents that are structurally omniscient about authoritative identity and lack coherent situated experience. The subjective scene layer prevents that architectural failure.

---

## Required Cognitive Architecture Additions

### Subjective Scene Model

A transient, agent-specific model of the currently experienced situation.

Conceptually it may include:

- perceived self state;
- subjective body state;
- believed current location;
- persistent subjective identities of perceived objects;
- perceived people and believed identities;
- subjective spatial relations;
- currently activated concepts;
- active memories;
- current goals and relevant needs;
- expectations and near-future predictions;
- attention focus;
- uncertainty.

The precise representation must remain an RFC design problem. Do not prematurely freeze this into a final Rust structure.

### Perceived Object Persistence

Agents must not directly use authoritative entity identity as their own knowledge.

Ground Truth may contain `ObjectId(81)`. An agent may maintain a subjective hypothesis such as `PerceivedObjectIdentity(19)`.

The agent may:

- identify two distinct objects as the same object;
- identify one object as multiple objects at different times;
- lose track of an object;
- incorrectly infer movement, replacement, theft, or destruction.

Subjective identity and authoritative identity must remain distinct.

### Subjective Body Schema

Objective biological body state and the agent's experienced body model are different representations.

Cognition receives information through appropriate sensory and internal-access mechanisms. The agent constructs a body schema from proprioception, pain, balance, internal sensory signals, and learned body boundaries.

Do not give cognition direct omniscient access to all biological state.

### Self-Model

Agents require a persistent but subjective model of themselves.

Possible future contents include:

- believed abilities;
- autobiographical continuity;
- social identity associations;
- believed personal traits;
- expected behavioural tendencies;
- beliefs about how others perceive them.

The self-model may disagree with Ground Truth and observed outcomes.

### Predictive World Model

Agents should maintain lightweight expectations about the current scene and near future.

Examples:

- object released above stable surface → expected support;
- known person approaches → expected social interaction;
- door receives familiar action → expected state transition.

Prediction error should affect attention, salience, memory encoding, possible concept revision, and causal inference.

Prediction is not a global physics simulator inside every agent. It must be sparse, bounded, subjective, and focused on active context.

### Working Memory / Active Context

Persistent knowledge must not be continuously active.

An agent may have decades of autobiographical history while only a small active context affects the current cognitive step.

Separate:

- persistent memory/state;
- active working context;
- currently activated memories and concepts.

### Episodic Memory Reactivation

Current perceptual patterns may reactivate episodic memories through partial similarity and current relevance.

Example:

```text
current movement pattern
    → similarity with stored perceptual pattern
    → old episode becomes active
    → associated person identity becomes salient
    → current scene acquires additional subjective meaning.
```

Do not create semantic events such as `RememberFatherMoment`.

### Agency Attribution

Agents need a mechanism for learning that their actions may cause outcomes.

Conceptually:

```text
attempted action
    → observed transition
    → temporal and contextual association
    → strengthened subjective agency model.
```

This contributes to learned capabilities, self-model, action expectations, and causal inference.

### Subjective Temporal Continuity

The current subjective situation is not an isolated simulation tick.

Agents need a bounded temporal envelope containing:

- relevant recent past;
- current scene;
- expected near future.

Example:

> "I expected this person earlier" + "the person is absent" + "it is becoming dark" + "an attack occurred here recently"

may produce a highly salient subjective situation without a primitive `AnxietySituation` enum.

---

## New Invariants

The following invariants have been added to `docs/architecture/invariants.md`:

- **INV-027:** Agents do not directly perceive authoritative entity identity.
- **INV-028:** Perceived object identity is a subjective hypothesis.
- **INV-029:** Agents act on a constructed subjective scene.
- **INV-030:** Subjective scene content must be causally grounded.
- **INV-031:** Subjective detail cannot introduce inaccessible information.
- **INV-032:** Persistent autobiographical memory is not continuously active context.
- **INV-033:** The self-model is subjective.
- **INV-034:** Objective body state and subjective body schema are distinct.
- **INV-035:** Prediction error is a first-class cognitive driver.

See `docs/architecture/invariants.md` for full text and rationale.

---

## Compatibility with Phases 1–5

### Phase 1: Deterministic Kernel

The kernel supports transient per-agent active state through the existing `System` trait and scheduler phase model. No kernel changes are required. Per-agent systems can be registered in the Cognition phase just as global systems are registered in the Physics phase.

### Phase 2: Ontology Primitives and Generic Features

`FeatureRelation` remains genuinely generic. No semantic shortcuts were found in the generic feature layer.

`Feature.target_id` references `EntityId`. This is acceptable at the extractor level because `ontopolis-perception` operates on Ground Truth samples admitted through physical access. The subjective scene construction layer must map these authoritative references to perceived identities before cognition consumes them. The implemented attention primitive accepts only agent-local `AttentionTargetId` and therefore does not cross this boundary.

### Phase 3: Spatial World

`SpatialHierarchy` uses authoritative `PlaceId` and `ChunkId`. This is correct for Ground Truth. Future cognition must not consume these IDs directly as subjective location knowledge. The architecture permits this separation because the spatial types are clearly authoritative and the cognition crate does not currently import them.

RFC-GEO-002 later clarifies that the hierarchy is containment, not geometry, and that bare chunks are local-chart addresses. Future spatial cues must derive through physical access from bounded local 3D relations and then be mapped into agent-relative subjective signatures. `SpatialChartId`, `LocalFrameId`, global coordinates, exact distances, and authoritative poses must not enter a subjective scene as hidden knowledge.

### Phase 4: Causal Geography

Geography exposes physical state through property-based contracts (`ElevationMm`, `MaterialId`, `RoughnessMm`). No subjective geographic knowledge is embedded in authoritative geography. No changes required.

### Phase 5: Biological Structural Model

`BodyStructure` contains authoritative `BodySegmentId`, length, orientation, and joint limits. Objective body state remains separate from future subjective body schema. The cognition crate does not currently import `ontopolis-biology`. No direct omniscient cognitive access exists. No changes required.

---

## Roadmap Rebaseline

The cognitive sequence has been rebaselined. Phases 1–5 remain completed with their original numbers. The following phases are updated:

| Phase | Previous | New |
|-------|----------|-----|
| 6 | Ground Truth events and causal provenance | Ground Truth events and causal provenance |
| 7 | Physical access, sensory acquisition, generic feature extraction | Physical access and sensory acquisition |
| 8 | Subjective percepts and sparse concept formation | Generic perceptual feature extraction |
| 9 | Minimal cognition and belief hypotheses | **Subjective Scene Construction** |
| 10 | Language bootstrap and communication architecture | **Working context, prediction, and cognitive continuity** |
| 11 | Lexical innovation, semantic inference, and language change | **Sparse subjective concept formation** |
| 12 | Practice representation and evolution | **Beliefs and subjective causal inference** |
| 13 | Measurement, documents, and epistemic infrastructure | Language bootstrap and communication architecture |
| 14 | Minimal information-sensitive mana | Lexical innovation, semantic inference, and language change |
| 15 | Causal Resolution Field | Practice representation and evolution |
| 16 | Social networks and organizations | Measurement, documents, and epistemic infrastructure |
| 17 | Material economy and city infrastructure | Minimal information-sensitive mana |
| 18 | Historical bootstrap | Causal Resolution Field |
| 19 | Isekai transfer and imported priors | Social networks and organizations |
| 20 | Metaphysical experiments and attractors | Material economy and city infrastructure |
| 21 | Long-run emergence experiments | Historical bootstrap |
| 22 | Explanation Engine expansion | Isekai transfer and imported priors |
| 23 | Rich observer UI | Metaphysical experiments and attractors |
| 24 | Optional narrative surface realization | Long-run emergence experiments |
| 25 | — | Explanation Engine expansion |
| 26 | — | Rich observer UI |

See `docs/roadmap/roadmap.md` for the authoritative updated roadmap.

> Subsequent supersession: the Detailed Development rebaseline closes the preallocated roadmap at
> Phase 26. Optional narrative/LLM realization is now an unnumbered terminal gate after an unknown
> amount of simulation and Explanation depth work. The table above remains the historical record of
> the cognition resequencing through the completed Foundation Era.

---

## Specification Provenance

The repository was initialized from a project initialization specification. That specification remains the historical baseline.

Post-initialization architectural discoveries, refinements, and course corrections are recorded through:

- **ADRs** — individual architectural decisions;
- **RFCs** — design investigations for complex subsystems;
- **Architecture rebaseline documents** — mid-course corrections like this one;
- **Subsystem documentation** — detailed domain documentation;
- **Roadmap revisions** — phase resequencing and scope updates.

Later documents supersede conflicting initialization-spec sections **only when they explicitly say so**. This rebaseline explicitly supersedes the previous Phase 8–12 cognitive sequence in the initialization specification and roadmap.

---

## Related Documents

- `docs/architecture/invariants.md` — hard invariants including INV-027 through INV-037
- `docs/rfc/RFC-GEO-002.md` — multiscale Ground Truth geometry feeding future subjective spatial cues
- `docs/rfc/RFC-COG-001.md` — accepted architecture for the subjective scene and cognitive continuity model
- `docs/rfc/RFC-SCENE-001.md` — accepted concrete Phase 9–10 layout
- `docs/roadmap/roadmap.md` — updated phase sequence
- `docs/development/todo-backlog.md` — updated TODO dependencies
- `docs/simulation/perceptual-features.md` — generic feature layer (predecessor to subjective scene)
- `docs/cognition/attention.md` — attention primitives
- `docs/cognition/memory.md` — memory structures
- `docs/cognition/prediction.md` — prediction mechanisms

---

## Decision Log

- **Accepted:** The subjective scene construction layer is a required architectural boundary between generic perceptual features and subjective concepts/beliefs.
- **Accepted:** Phases 1–5 remain completed and unchanged.
- **Accepted:** Phases 8–12 are resequenced to insert subjective scene construction and cognitive continuity before concept formation and belief.
- **Accepted:** No existing Phase 1–5 code requires modification; the contracts are compatible but require the new layer to prevent future violations.
- **Accepted:** RFC-SCENE-001 resolves the minimal Rust representation with fixed-capacity identity-free cognitive state.
- **Accepted:** New invariants INV-027 through INV-035 are added to enforce the boundary.

## Phase 9–10 Implementation Status

`ontopolis-cognition` now implements the accepted boundary. `PerceptualCue` is the only generic scene input and cannot carry authoritative entity, feature, place, body-segment, sensor, or trace identity. `SceneContinuityState` maintains fallible agent-local object hypotheses and reconstructs an attention-gated transient scene.

Active cognition is explicitly bounded: working context decays independently of capped episodic storage; sparse predictions emit numeric errors; agency is an opaque learned association; and a short temporal envelope retains only recent subjective frames. Phases 11–12 now add attention-fed subjective prototypes, fixed-point belief inertia, subjective source trust, and fallible causal pattern hypotheses without crossing the Ground Truth identity boundary.
