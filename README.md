# Ontopolis

> **A city whose inhabitants do not receive reality in a form they can understand.**

Ontopolis is a high-performance experimental simulation of a geographically coherent fantasy / isekai world with extreme causal depth concentrated around a living city.

It is an attempt to simulate the bottom-up co-evolution of:

* physical reality;
* geography;
* biology;
* perception;
* subjective experience;
* memory and prediction;
* concepts;
* language;
* knowledge;
* practices;
* institutions;
* technology;
* information-sensitive magic;
* and historical path dependence.

The city is not generated as a collection of lore, modifiers, scripted events, or predefined fantasy systems.

It gradually becomes a physical, social, linguistic, and magical product of its own history.

The central goal of Ontopolis is simple to state and extremely inconvenient to implement:

> **Produce surprising situations which nobody authored, while preserving enough causal provenance to explain exactly how they happened.**

---

## The Core Thesis

Ontopolis is built around a specific simulation thesis.

### 1. Reality exists independently of its inhabitants

The simulation maintains authoritative physical and biological state.

Objects, organisms, materials, pathogens, terrain, and fields may continue to exist and produce consequences regardless of what any agent believes about them.

A stone does not become harmless because nobody understands geology.

A pathogen does not stop spreading because a society believes illness is caused by moral corruption.

Reality can disagree with its inhabitants.

That disagreement is one of the primary engines of history.

---

### 2. Agents never receive Ground Truth

An inhabitant does not inspect simulation state.

Agents cannot directly access:

* authoritative entity identities;
* exact material composition;
* true pathogen lineages;
* causal provenance graphs;
* objective biological state;
* the real intentions of other agents;
* developer-defined analytical labels.

Their access to reality is mediated through physical accessibility and bounded sensory processes.

Conceptually:

```text
GROUND TRUTH
    ↓
PHYSICAL ACCESS
    ↓
SENSORY ACQUISITION
    ↓
GENERIC FEATURE EXTRACTION
    ↓
SUBJECTIVE SCENE CONSTRUCTION
    ↓
CONCEPTS, MEMORY, PREDICTION
    ↓
BELIEFS AND CAUSAL HYPOTHESES
    ↓
DECISION AND ACTION
```

An agent does not live inside the authoritative world model.

It lives inside a continuously reconstructed, incomplete, and potentially incorrect model of its current situation.

---

### 3. Agents construct subjective situations

Ontopolis does not intend agents to behave as feature vectors connected directly to decision functions.

An agent constructs a **Subjective Scene**.

The scene may contain:

* a perceived self;
* a subjective body schema;
* believed object identities;
* believed locations;
* perceived people;
* active relationships;
* currently relevant goals;
* reactivated memories;
* active concepts;
* near-future predictions;
* attention;
* uncertainty.

Authoritative identity and perceived identity are different.

The world may contain:

```text
EntityId(718)
```

An agent may instead maintain a subjective hypothesis equivalent to:

```text
PerceivedObjectIdentity(19)
```

The agent may believe that two different objects are the same object.

It may believe that the same object seen twice is two different objects.

It may lose track of an object.

It may infer replacement, movement, destruction, or theft incorrectly.

The same principle applies to the body and the self.

Objective biological state is not identical to subjective body experience.

The authoritative agent is not identical to the agent's self-model.

---

### 4. Situations may be richer than explicit source-code categories

The engine does not need a semantic event such as:

```text
RememberDeadFatherMoment
```

A situation may emerge from:

```text
current perceived movement
    ↓
similarity to an episodic memory
    ↓
memory reactivation
    ↓
association with a known person
    ↓
attention shift
    ↓
change in the current subjective scene
```

The resulting situation may be understandable to a human as:

> An apprentice holds a tool exactly like the agent's dead father once did.

No developer authored that scene.

No semantic enum describes it.

Its content exists because runtime state and accumulated personal history interacted.

Emergence may reorganize and combine available information into structures much richer than explicit semantic categories in the source code.

It may not create information from complete absence.

If smell is not represented at any causally relevant level, an agent cannot suddenly smell bread.

Subjective detail must remain causally grounded.

---

### 5. Human concepts are not engine primitives

The authoritative simulation should not begin with a taxonomy of the human world.

There is no requirement for primitive engine categories such as:

```text
Table
Disease
Race
Profession
Class
Skill
Monster
SacredStone
Criminal
```

The engine defines lower-level physical, structural, biological, and relational properties.

For example, Ground Truth may contain:

```text
material structures
attachments
geometry
orientation
movement
support relations
thermal state
biological segments
field state
```

An agent may repeatedly perceive a stable elevated surface supported from below and form a subjective concept around similar objects.

A local language may associate a lexeme with that concept.

The observer UI may display the English gloss:

> table

The simulation itself never required `Furniture::Table`.

Ontopolis distinguishes:

> **what exists**

from:

> **how agents divide existence into concepts**

and from:

> **how the observer explains those concepts to a human user**

---

## The Ontological Feedback Loop

The central historical loop of Ontopolis is:

```text
HIDDEN CAUSALITY
    ↓
INCOMPLETE OBSERVATION
    ↓
SUBJECTIVE INTERPRETATION
    ↓
CAUSAL HYPOTHESIS
    ↓
BEHAVIOUR
    ↓
PRACTICE
    ↓
REPETITION AND STANDARDIZATION
    ↓
PERSISTENT PHYSICAL / INFORMATIONAL PATTERN
    ↓
MANA RESPONSE
    ↓
CHANGED PHYSICAL EFFECT
    ↓
APPARENT CONFIRMATION
    ↓
REINFORCED BELIEF
```

A false explanation may therefore become practically predictive.

Not because belief directly changes reality.

Because belief changes behaviour.

Behaviour creates repeated real structures.

Mana reacts to those structures.

The resulting physical effect appears to confirm the original explanation.

A mistaken theory can slowly construct the conditions under which it begins to work.

---

## Magic Is Physical, Not Semantic

Mana is an information-sensitive physical substrate.

It does **not** understand human meaning.

Mana has no concept of:

* gods;
* prayer;
* marriage;
* ethnicity;
* guilt;
* law;
* professions;
* classes;
* skills;
* levels.

It cannot inspect an agent's belief state.

Mana may respond to real patterns such as:

* periodicity;
* frequency;
* phase relations;
* synchronization;
* spatial symmetry;
* recurring sequences;
* information density;
* persistent geometry;
* long-lived repeated structures.

Consider a ritual performed every morning.

The inhabitants may believe:

> an unmarried woman must pray beside a copper bell.

Mana does not understand unmarried women, prayer, or copper symbolism.

But the associated practice may consistently produce:

* a particular spatial arrangement;
* a repeated vocal sequence;
* synchronized movement;
* a stable acoustic frequency;
* the same timing every morning.

Those physical patterns may interact with the local mana field.

The local theory may be metaphysically wrong and operationally useful.

---

## Geography Is Causal

Geography is not decorative map data.

Ontopolis treats the world as a hierarchy of physical spatial systems:

```text
World
└── Landmass
    └── Geographic Basin
        └── Landscape Region
            └── Local Territory
                └── Spatial Chunk
                    └── Parcel / Site
                        └── Structure
                            └── Interior Space
```

Terrain, geology, hydrology, climate, material distribution, and mana topology participate in causal history.

A material may retain provenance through:

```text
geological formation
    ↓
deposit
    ↓
quarry
    ↓
extraction lot
    ↓
transport batch
    ↓
merchant inventory
    ↓
workshop
    ↓
building component
```

A phenomenon occurring in a bakery two centuries later may ultimately depend on a particular geological formation.

The simulation should be capable of reconstructing that chain.

Rivers, valleys, resources, roads, settlement locations, and city morphology should create conditions from which history develops.

A city is not placed on a map.

Its location should have consequences.

---

## Language Is a Simulated Historical Process

Simulated inhabitants do not know English.

The authoritative simulation contains no privileged human interface language.

Changing the observer UI from English to Russian must not change the canonical simulation state hash.

Language is separated into:

```text
SUBJECTIVE CONCEPT
    ↕
LEXICAL ASSOCIATION
    ↕
PHONOLOGICAL FORM
    ↕
MORPHOLOGY AND GRAMMAR
    ↕
PHYSICAL UTTERANCE
```

A word does not contain one objective meaning.

Different agents may associate the same lexeme with different concepts.

New words may emerge because speakers repeatedly need to communicate a distinction for which no established lexeme exists.

Possible lexical strategies include:

* description;
* composition;
* derivation;
* metonymy;
* geographic naming;
* occupational naming;
* borrowing;
* novel root creation.

Listeners do not directly receive the speaker's concept.

They hear a physical utterance and reconstruct possible meaning from:

* known lexical associations;
* current context;
* perceived referents;
* previous uses;
* source trust.

Misunderstanding is normal.

Semantic drift is expected.

A term may begin as a label for a recurring physiological pattern, later become associated with a profession, then a district, and eventually an inherited social identity.

No developer needs to author that semantic history.

---

## Language Can Change Magic

Speech is physical.

It produces acoustic and temporal patterns.

Therefore language change may have physical magical consequences.

A historical vowel shift may alter the frequency profile of a traditional chant.

The changed pronunciation may modify mana coupling.

A spell lineage may gradually become unstable.

The inhabitants may explain the decline through:

* lost discipline;
* moral decay;
* corrupted teaching;
* divine anger.

The actual causal history may lead through phonological change.

Ontopolis is specifically designed to allow this kind of cross-domain causal chain.

---

## Knowledge Is Not Capability

Knowing that a technology is possible does not make it available.

Ontopolis distinguishes:

* declarative knowledge;
* procedural knowledge;
* perceptual expertise;
* motor skill.

An isekai arrival may know:

> microorganisms can cause disease.

That does not automatically provide:

* suitable lenses;
* pure glass;
* precision tools;
* sterile procedures;
* experimental institutions;
* social credibility.

Likewise, understanding the principle of an internal combustion engine does not produce:

* appropriate steel;
* precision machining;
* seals;
* lubricants;
* fuel infrastructure;
* ignition systems.

Technology emerges from combinations of:

```text
CONCEPT
+
MATERIAL
+
TOOL
+
MEASUREMENT
+
PROCEDURAL KNOWLEDGE
+
SKILL
+
SOCIAL TRANSMISSION
```

There is no technology tree.

---

## Isekai Is a Causal Contamination Process

Ontopolis treats cross-world arrival as more than a narrative excuse to give a protagonist modern knowledge.

Isekai arrivals introduce **foreign priors**.

Possible transfer phenomena may eventually include:

* complete physical transfer;
* identity-pattern transfer;
* reincarnation-like binding;
* partial autobiographical memory transfer;
* informational echoes;
* transferred artifacts;
* overlapping identity structures.

The exact metaphysics remain an open research problem.

An arrival from Earth may remember a concept which has no equivalent in the local conceptual system.

Translation therefore becomes approximate concept mapping.

For example:

```text
computer
≈ thinking mechanism
≈ calculation device
≈ memory machine
≈ thinking loom
```

A poor translation may redirect an entire technological history.

A society trying to understand the phrase "thinking loom" might discover computation through textile machinery rather than mathematics.

The isekai arrival did not unlock a technology.

They contaminated the world's epistemic trajectory.

---

## Familiar Fantasy Systems Are Target Emergent Outcomes

Ontopolis may eventually produce systems resembling:

* Status Windows;
* levels;
* skills;
* classes;
* magical schools;
* adventurer guilds;
* monster taxonomies;
* dungeons;
* magical contracts;
* artifacts;
* spirits;
* gods;
* sacred languages.

These are not normally primitive engine features.

For example, a class-like phenomenon may develop through:

```text
social category
    ↓
standardized training
    ↓
shared equipment
    ↓
repeated synchronized practices
    ↓
local mana coupling
    ↓
characteristic physical effects
    ↓
institutional classification
```

Centuries later, a historically evolved Status-like system may label the group as a `Class`.

The class became causally meaningful because society first created and standardized the social distinction.

The engine did not start with `enum Class`.

---

## Practices Evolve

A practice is an executable behavioural structure, not a prose description.

Future practice representations may contain:

* ordered operations;
* timing;
* repetitions;
* conditions;
* branches;
* materials;
* actor roles;
* locations;
* synchronization;
* tolerances.

Practice behaviour and practice explanation are separate.

A society may preserve an action while forgetting why it began.

It may preserve an explanation while gradually changing the action.

Practices may:

* mutate;
* combine;
* simplify;
* accumulate copying errors;
* become standardized by institutions.

A copying mistake in a manuscript may change a ritual from three repetitions to eight.

The modified practice may initially be objectively worse.

If it becomes socially prestigious and widely repeated, it may eventually create a different stable mana pattern.

The error may become physically effective.

---

## Biology Does Not Define Social Categories

Ontopolis models objective biological structure and population lineages.

It does not require Ground Truth enums such as:

```text
Human
Elf
HalfElf
Demon
```

Biological populations may differ statistically in:

* morphology;
* lifespan;
* fertility;
* development;
* sensory ranges;
* metabolism;
* mana coupling.

Societies may construct categories such as "elf" or "demon".

Their social boundaries may not correspond cleanly to biological population structure.

The same objective biological continuum may be classified differently by different societies.

Long lifespan is also not merely a stat bonus.

A population living for centuries should affect:

* property ownership;
* inheritance;
* institutional memory;
* political turnover;
* professional access;
* innovation;
* intergenerational conflict.

Biology participates in history.

---

## Disease Is a Causal Process, Not an Event

Ontopolis does not intend to trigger:

```text
SpawnPlagueEvent
```

Ground Truth may contain pathogen lineages and host interactions.

Agents perceive physiological patterns.

Societies construct illness concepts.

Different medical traditions may classify the same pathogen differently.

Disease may interact with:

* water systems;
* geography;
* migration;
* demography;
* social practices;
* medical theories;
* institutions.

An epidemic may reduce one generation of children.

Decades later the small cohort may cause:

```text
labour shortage
    ↓
wage growth
    ↓
military recruitment failure
    ↓
inheritance concentration
    ↓
political conflict
```

History should contain delayed consequences.

---

## Memory, Prediction, and the Self

Persistent history is not continuously active thought.

An agent may have decades of autobiographical experience while only a small active context participates in the current cognitive step.

Ontopolis distinguishes:

* persistent memory;
* working context;
* active memories;
* active concepts;
* current predictions.

Perception may reactivate old episodes through partial similarity.

A familiar movement, sound, object, or place may acquire enormous subjective significance because of prior experience.

Agents also construct subjective self-models.

A self-model may include beliefs equivalent to:

* I can do this;
* I usually fail at this;
* this is my body;
* I was here before;
* I trust this person;
* people see me this way;
* I caused that outcome.

The self-model may be wrong.

An agent may sincerely believe they are an excellent physician while historical outcomes suggest otherwise.

Ground Truth does not automatically correct self-understanding.

Prediction error is a first-class cognitive driver.

Agents form bounded expectations about active situations.

Unexpected outcomes may change:

* attention;
* salience;
* memory encoding;
* concepts;
* causal hypotheses.

A large portion of Ontopolis history should begin with some agent noticing:

> something is wrong with what I expected.

---

## Causal Resolution, Not Distance-Based LOD

Ontopolis does not simulate the entire world at identical detail.

Simulation resolution depends on causal relevance.

Candidate relevance dimensions include:

* physical proximity;
* trade connectivity;
* migration;
* information flow;
* social connectivity;
* political influence;
* material dependency;
* mana coupling;
* historical importance.

A village five kilometres from the city may remain aggregated.

A monastery six hundred kilometres away may require greater detail because half the city copies its rituals.

A researcher on another continent may become causally important because their book is being imported.

Distance is only one component of resolution.

Promotion and demotion between simulation resolutions must preserve historically significant identity and provenance.

Detailed people cannot simply disappear into:

```text
population += 541
```

without controlled aggregation semantics.

---

## Causal Provenance Is First-Class

Ontopolis should be able to explain its own history.

A phenomenon might have a causal lineage such as:

```text
geological formation
    ↓
stone extraction
    ↓
bakery oven construction
    ↓
fermentation anomaly
    ↓
incorrect prayer hypothesis
    ↓
ritual standardization
    ↓
copper bell adoption
    ↓
stable rhythmic pattern
    ↓
mana response
    ↓
medical diagnostic practice
    ↓
guild authority
    ↓
district regulation
```

A user should eventually be able to ask:

> Why does this district prohibit bells after sunset?

and inspect the actual historical chain.

Surprise without provenance is random noise.

Ontopolis is interested in **causal surprise**.

---

## The Explanation Engine

The authoritative simulation is not shaped for human readability.

Internally, the world may contain:

```text
ConceptId(8172)
FeaturePattern(...)
LexemeId(4412)
BodySegmentId(...)
TraceId(...)
```

Users should not be forced to stare at this and develop Stockholm syndrome toward debug output.

Ontopolis therefore contains a separate, non-authoritative **Explanation Engine**.

```text
AUTHORITATIVE SIMULATION
    ↓
CAUSAL / ANALYTICAL QUERY
    ↓
OBSERVER ANALYTICAL CLASSIFICATION
    ↓
EXPLANATION IR
    ↓
DETERMINISTIC LOCALIZED RENDERING
    ↓
OPTIONAL LLM SURFACE REALIZATION
    ↓
UI
```

The observer analytical layer may contain human-designed classifications such as:

* finger-like structure;
* tremor-like motion;
* disease-like cluster;
* occupational category.

These are human-facing interpretations.

They never become agent knowledge or authoritative simulation state.

Explanations preserve:

* evidence;
* causal provenance;
* confidence;
* alternative interpretations;
* perspective.

The UI may separately show:

### Objective analytical view

What observer analytics infer from Ground Truth.

### Local understanding

What a community currently believes.

### Historical view

How a concept or practice developed.

### Selected agent view

What one particular inhabitant believes.

### Plain explanation

A compressed human-readable explanation of the known causal structure.

These perspectives must not be silently mixed.

---

## LLMs Are Non-Authoritative

Ontopolis is not an LLM-driven simulation.

LLMs do not:

* control inhabitants;
* decide world state;
* invent history;
* discover authoritative causal relationships;
* modify beliefs;
* generate simulation events.

An optional LLM may eventually receive a validated structured fact packet from the Explanation Engine and improve the wording of a paragraph.

For example:

```text
Explanation IR
    ↓
validated fact packet
    ↓
LLM
    ↓
readable prose
```

The LLM is allowed to make an explanation less unpleasant to read.

It is not allowed to decide what happened.

Ontopolis must remain fully understandable and operational without an LLM.

---

## What Ontopolis Is Not

### Not a generic civilization simulator

There are no high-level history buttons such as:

```text
spawn war
spawn plague
create religion
advance technology
```

High-level phenomena must arise from lower-level processes.

### Not a Dwarf Fortress clone

Ontopolis is not primarily a fortress-management simulation.

Its focus is causal, epistemic, cognitive, linguistic, geographic, and magical co-evolution.

### Not an LLM agent town

Agent minds do not run as chatbots.

Natural-language fluency is not a substitute for persistent structured cognition.

### Not a procedural story generator

Stories are downstream interpretations of actually simulated history.

The simulation does not choose a narrative arc and then manufacture supporting events.

### Not a collection of semantic enums

Convenient developer categories must not silently replace emergence.

A feature called `FingerTremor`, a primitive `Disease`, or an enum of fantasy `Classes` would undermine the central architecture unless explicitly justified as observer-side analytics.

### Not a consciousness claim

Ontopolis does not claim to create conscious beings.

It does aim to construct agents with increasingly rich forms of functional subjectivity:

* subjective scenes;
* self-models;
* body schemas;
* autobiographical continuity;
* prediction;
* bounded attention;
* private perceptual states.

Whether such systems possess phenomenal experience is not assumed or claimed.

---

## Project Invariants

The full invariant set is documented in `docs/architecture/invariants.md`.

Core invariants include:

* **No omniscient agents.**
* **Observation is not Ground Truth.**
* **Belief is not magic.**
* **Mana cannot inspect semantic concepts.**
* **Developer analytical labels are not agent concepts.**
* **The simulation has no privileged human UI language.**
* **Language decoding does not directly transfer concepts.**
* **Geography is causal state.**
* **Distance is not simulation resolution.**
* **LLMs are non-authoritative.**
* **Explanation systems are non-authoritative.**
* **Provenance is first-class.**
* **Authoritative mutation is phase controlled.**
* **Emergence must be inspectable.**
* **Narrative is downstream.**
* **The UI is an observer.**
* **Agents do not directly perceive authoritative entity identity.**
* **Perceived object identity is a subjective hypothesis.**
* **Agents act on a constructed subjective scene.**
* **Subjective detail cannot introduce inaccessible information.**
* **Persistent autobiographical memory is not continuously active context.**
* **The self-model is subjective.**
* **Objective body state and subjective body schema are distinct.**
* **Prediction error is a first-class cognitive driver.**

---

## Roadmap and Current Status

Ontopolis is developed in dependency-ordered phases.

Completed phases preserve their original historical numbering.

**Current status: Phase 8 completed. Entering Phase 9, Subjective Scene Construction.**

### Completed

#### Phase 1: Deterministic simulation kernel

* phase-aware scheduler;
* deterministic execution contracts;
* keyed random streams;
* runtime configuration.

#### Phase 2: Ontology primitives and generic features

* coordinate primitives;
* fixed-point and physical property foundations;
* typed identifiers;
* generic feature relations such as periodicity, recurrence, structural similarity, and relative difference.

#### Phase 3: Spatial world skeleton

* authoritative world hierarchy;
* spatial containment;
* stable spatial ownership contracts.

#### Phase 4: Minimal causal geography

* terrain;
* geological, hydrological, and climate boundaries;
* terrain generation;
* generation provenance.

#### Phase 5: Biological structural model

* body segments;
* joints;
* orientation and structural state;
* pathogen lineage and host-interaction contracts.

#### Phase 6: Ground Truth events and causal provenance

* causal event infrastructure;
* traceable authoritative transitions.

#### Phase 7: Physical access and sensory acquisition

* physical signal accessibility;
* sensor apertures;
* bounded acquisition.

#### Phase 8: Generic perceptual feature extraction

* non-semantic feature extraction;
* bounded attention foundations.

### Next

#### Phase 9: Subjective Scene Construction

A sparse, agent-specific model of the currently experienced situation.

See `RFC-COG-001`.

#### Phase 10: Working context, prediction, and cognitive continuity

* active context;
* bounded working state;
* prediction;
* prediction error;
* episodic reactivation;
* subjective temporal continuity.

#### Phase 11: Sparse subjective concept formation

Agents begin constructing useful categories from their own experience.

#### Phase 12: Beliefs and subjective causal inference

Agents build and revise causal hypotheses.

#### Later phases

* language bootstrap;
* lexical innovation and semantic drift;
* evolvable practices;
* measurement and epistemic infrastructure;
* information-sensitive mana;
* Causal Resolution Field;
* social networks and organizations;
* material economy and city infrastructure;
* historical bootstrap;
* isekai transfer;
* metaphysical experiments;
* long-run emergence studies;
* Explanation Engine expansion;
* rich observer application;
* optional narrative surface realization.

See `docs/roadmap/roadmap.md` for the authoritative roadmap.

---

## Project Structure

Ontopolis is organized as a Rust workspace with strict domain boundaries, a separate observer application, and versioned protocol definitions.

Important components include:

* **`ontopolis-core`**
  Deterministic scheduling, execution phases, random streams, and simulation kernel contracts.

* **`ontopolis-types`**
  Primitive IDs, coordinates, units, physical structures, and generic feature representations.

* **`ontopolis-world`**
  Authoritative world hierarchy and spatial ownership.

* **`ontopolis-geography`**
  Terrain, geology, hydrology, climate boundaries, and geographic provenance.

* **`ontopolis-biology`**
  Structural biology, body state, biological lineages, and pathogen contracts.

* **`ontopolis-perception`**
  Physical accessibility, sensory acquisition, and generic feature extraction.

* **`ontopolis-cognition`**
  Attention, active cognitive state, and future subjective scene and continuity systems.

* **`ontopolis-language`**
  Future lexicon, phonology, morphology, grammar, communication, and language change.

* **`ontopolis-epistemics`**
  Future measurement, metrology, documents, experiments, and knowledge infrastructure.

* **`ontopolis-isekai`**
  Cross-world transfer and imported-prior architecture.

* **`ontopolis-metaphysics`**
  Identity persistence and mana-attractor research boundaries.

* **`ontopolis-resolution`**
  Causal Resolution Field and state aggregation architecture.

* **`ontopolis-explanation`**
  Non-authoritative analytical classification and Explanation IR.

* **`ontopolis-observer-api`**
  Read-only observer contracts.

* **`ontopolis-observer-wire`**
  Versioned observer protocol and transport boundaries.

* **`ontopolis-runtime`**
  Runtime composition root.

* **`ontopolis-lab`**
  Experiments, causal inspection, and emergence analysis.

* **`ontopolis-cli`**
  Developer command-line tooling.

* **`apps/observer`**
  Tauri, React, TypeScript, and future WebGPU simulation observer.

The documentation entry point is `docs/index.md`.

---

## Development Philosophy

Ontopolis optimizes:

> **simulated causal complexity per wall-clock second**

Raw entity count is not the goal.

One million inert agents are less interesting than a smaller population capable of producing deep, reconstructable, cross-domain history.

The architecture therefore emphasizes:

* data-oriented storage;
* sparse active state;
* active sets;
* multi-rate simulation;
* deterministic parallel execution;
* causal-resolution transitions;
* bounded observer overhead.

Persistent identity does not imply full-resolution cognition on every simulation tick.

A person may have sixty years of history without loading sixty years of autobiographical memory into active state every update.

---

## Getting Started

### Prerequisites

* stable Rust toolchain configured by `rust-toolchain.toml`;
* `just` command runner, recommended;
* pnpm for observer development.

### Build and test

```bash
just test
```

Equivalent Rust command:

```bash
cargo test --workspace --all-features
```

Run the canonical CI workflow:

```bash
just ci
```

or:

```bash
cargo xtask ci
```

### Diagnostics

```bash
just doctor
```

or:

```bash
cargo run --bin ontopolis -- doctor
```

---

## Development Rules

All contributors and AI coding agents must follow `AGENTS.md`.

In particular:

* preserve deterministic execution;
* preserve Ground Truth / perception / subjective scene separation;
* never expose authoritative identity as agent knowledge;
* never use human-language labels as authoritative simulation meaning;
* never introduce semantic situation enums as shortcuts for subjective scene construction;
* keep objective body state separate from subjective body schema;
* keep persistent memory separate from active working context;
* treat geography and biology as causal state;
* preserve causal provenance;
* do not let the Explanation Engine or observer mutate authoritative state;
* do not place LLMs in the simulation loop;
* benchmark performance claims;
* use the repository's RFC and architecture-rebaseline process for foundational changes.

Use the codebase memory graph tools documented by the project for code discovery and navigation where available.

---

## License

**Software code** is licensed under the GNU Affero General Public License v3.0 only:

`AGPL-3.0-only`

See `LICENSE`.

**Documentation and architectural texts** are licensed under the Creative Commons Attribution-ShareAlike 4.0 International license:

`CC BY-SA 4.0`

See `LICENSE-CC-BY-SA-4.0`.

Contribution policy and CLA requirements are documented in `CONTRIBUTING.md`.

---

## The Intended Result

A successful Ontopolis run should not merely report:

> A religious conflict occurred.

It should be capable of producing and reconstructing something closer to:

> 184 years earlier, a cross-world arrival introduced a concept roughly translated as "level." A monastery adopted numerical evaluation of novices. Repeated standardized measurement created a stable social practice and eventually a local mana pattern. Mercenary companies copied the system. The state later used the measurements for professional taxation. A persistent measurement bias against one population produced economic exclusion. The excluded population developed a competing scale. Both systems now produce locally measurable magical effects and disagree about the same person's status.

Or:

> A copying error changed a healing practice from three repetitions to eight. The slower variant became associated with wealthy physicians and gained prestige. It spread widely enough to create a stable repeated mana pattern. Ninety years later, the originally incorrect version became physically more effective than the text it was copied from.

Or:

> A translated Earth concept for "computer" was rendered as "thinking loom." Textile workshops began experimenting with encoded pattern storage. Local mana was unusually sensitive to recurring binary spatial structures. The world's first computational architecture emerged from carpet manufacturing.

Nobody writes these histories in advance.

Ontopolis exists to discover whether a sufficiently coherent system of physical constraints, subjective minds, cultural transmission, language, geography, and strange local magic can produce them on its own.

And then explain why.
