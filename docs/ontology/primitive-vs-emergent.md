# Primitive vs Emergent

Ontopolis must explicitly distinguish between what the engine assumes as physical reality and what simulated agents construct as subjective categories.

## Engine Primitives

Primitives are properties required to define the physical or computational universe. They exist in Ground Truth regardless of whether any agent recognizes them.

### Spatial and Temporal Primitives (Phase 1)

| Primitive | Representation | Crate |
|-----------|---------------|-------|
| Space | `WorldCoord`, `ChunkCoord`, `LocalCoord` | `ontopolis-types` |
| Time | `SimulationTime` | `ontopolis-types` |
| Position | `WorldCoord` | `ontopolis-types` |
| Proximity | Integer distance on coordinate grid | `ontopolis-types` |
| Containment | Chunk/local coordinate conversion | `ontopolis-types` |

### Physical Primitives (Phase 2)

| Primitive | Representation | Crate |
|-----------|---------------|-------|
| Matter | `Material` (density, conductivity, hardness, porosity, specific heat) | `ontopolis-types` |
| Temperature | `Temperature` (Kelvin) | `ontopolis-types` |
| Orientation | `Orientation` (yaw, pitch, roll) | `ontopolis-types` |
| Motion | `Motion` (linear velocity, angular velocity) | `ontopolis-types` |
| Energy-related state | Stored as temperature and material properties | `ontopolis-types` |
| Material composition | `Material` with component proportions | `ontopolis-types` |
| Structural connection | `BodyStructure` parent topology and `Joint` angular limits | `ontopolis-biology` |

### Pattern Primitives (Phase 2)

Generic perceptual features that extract structural patterns from raw state. These are not semantic labels.

| Primitive | Representation | Crate |
|-----------|---------------|-------|
| Change | `FeatureRelation::Change` | `ontopolis-types` |
| Magnitude | `FeatureRelation::Magnitude` | `ontopolis-types` |
| Direction | `FeatureRelation::Direction` | `ontopolis-types` |
| Variance | `FeatureRelation::Variance` | `ontopolis-types` |
| Periodicity | `FeatureRelation::Periodicity` | `ontopolis-types` |
| Synchrony | `FeatureRelation::Synchrony` | `ontopolis-types` |
| Recurrence | `FeatureRelation::Recurrence` | `ontopolis-types` |
| Duration | `FeatureRelation::Duration` | `ontopolis-types` |
| Spatial relation | `FeatureRelation::SpatialRelation` | `ontopolis-types` |
| Temporal relation | `FeatureRelation::TemporalRelation` | `ontopolis-types` |
| Co-occurrence | `FeatureRelation::CoOccurrence` | `ontopolis-types` |
| Structural similarity | `FeatureRelation::StructuralSimilarity` | `ontopolis-types` |
| Relative difference | `FeatureRelation::RelativeDifference` | `ontopolis-types` |
| Sequence similarity | `FeatureRelation::SequenceSimilarity` | `ontopolis-types` |

### Additional Primitives (Phases 3+)

| Primitive | Phase | Notes |
|-----------|-------|-------|
| Field state | 17 | Implemented fixed-point local mana intensity and physical pattern coupling |
| Causal relevance / detail ordinal | 18 | Implemented fixed-point relevance and numeric resolution levels over opaque traced carrier channels |
| Social carrier record | 19 | Implemented traced directed links, assignments, claims, and document/practice associations with opaque schemas; their social meaning remains emergent |
| Material lot / physical transfer ancestry | 20 | Implemented typed integer quantity, custody/location, transformation inputs/outputs, performed labour, and traces; commodity and ownership meaning remain emergent |
| Urban physical topology | 20 | Implemented parcel/building references and opaque-schema infrastructure connectivity; city and network-purpose categories remain emergent |
| Repetition / Frequency / Sequence | 2 | Generic feature relations |
| Transformation | 2 | Change relation captures state transitions |

### Spatial Containment Primitives (Phase 3)

| Primitive | Representation | Crate |
|-----------|----------------|-------|
| Nested spatial containment | `SpatialHierarchy`, `SpatialNode` | `ontopolis-world` |
| Structural containment level | `SpatialLevel` | `ontopolis-world` |
| Deterministic hierarchy construction | `SpatialHierarchyBuilder` | `ontopolis-world` |
| Chunk/place identity boundary | Validated `PlaceId` ↔ `ChunkId` conversion | `ontopolis-world` |

These structural levels describe objective containment only. They do not encode political regions, ownership, place names, land use, observer classifications, or causal resolution. Phase 18 resolution is a separate overlay keyed by validated chunk identity.

### Terrain Primitives (Phase 4)

| Primitive | Representation | Crate |
|-----------|----------------|-------|
| Elevation | `ElevationMm` | `ontopolis-geography` |
| Surface roughness | `RoughnessMm` | `ontopolis-geography` |
| Surface material identity | `MaterialId` linked to physical `Material` properties | `ontopolis-geography` / `ontopolis-types` |
| Dense terrain surface | `TerrainChunk` structure-of-arrays fields | `ontopolis-geography` |
| Generation provenance | `TerrainGenerationProvenance` with causal trace references | `ontopolis-geography` |

These are physical quantities and identities, not landscape or material classifications. Biomes, terrain names, settlement suitability, political geography, and resource categories remain derived, observer, or agent concepts.

## Emergent Human Concepts

Emergent concepts are categories created by agents or societies through perception, learning, and social transmission. They may or may not correspond to Ground Truth structure.

### Biological Emergents

- finger
- tremor
- disease
- symptom
- anatomy

### Social Emergents

- race
- class
- skill
- profession
- criminal
- scientist
- religion
- law
- contract
- organization

### Cognitive Emergents

- concept
- belief
- goal
- habit
- trust
- monster
- sacred stone

### Linguistic Emergents

- word
- meaning
- grammar
- dialect
- writing system

### Geographic Emergents

- kingdom
- border
- trade route
- settlement
- city

## The Boundary in Practice

Allowed primitive representation:

```text
BodySegmentId
parent_segment
joint
length
orientation
movement samples
```

Forbidden as a primitive cognitive feature:

```text
FeatureKind::FingerTremor
```

A developer-facing observer classifier may later describe an observed motion as "tremor-like". That classification is not authoritative simulation state. It is an analytical gloss produced by the Explanation Engine for human observers.

## Why the Boundary Matters

If the engine treats `Class`, `Skill`, `Monster`, or `Disease` as primitive systems, then:

- emergence becomes decoration around predefined categories
- historical causality becomes irrelevant because outcomes are hardcoded
- agent subjectivity becomes meaningless when the engine already knows the "correct" category
- translation and misunderstanding become impossible when meanings are fixed

The engine starts with physics. Agents build concepts. The observer layer may classify. None of these three layers should confuse their categories with each other.

## Examples

### Biological structure

Ground Truth contains body segments with physical properties. A physician agent may construct a concept grouping certain segments by their role in grasping. A tax official may group the same segments by district and occupation. Neither concept is more correct in simulation terms. Both are subjective.

### Disease

Ground Truth may contain pathogen lineages and physiological state transitions. Agents observe symptom patterns and construct illness concepts. Different societies may classify the same pathogen differently. The engine does not contain a `Disease` enum with entries like `Plague` or `CoughingSickness`.

### Social categories

Ground Truth contains biological population lineages with distributions of lifespan, fertility, morphology, and metabolism. Agents and societies may construct categories such as "elf" or "demon". Their boundaries may not match objective biological population structure. Mixed ancestry and incorrect taxonomies must be possible.

## Phase 2 Primitive Inventory Status

All Phase 2 primitives are now represented as Rust types in `ontopolis-types`:

- `features.rs` — generic perceptual feature relations and values
- `physics.rs` — temperature, orientation, motion, material properties

No semantic shortcuts were introduced. All types are property-based, not taxonomy-based.

## Phase 3 Spatial Skeleton Status

The documented world-to-interior containment chain is implemented as validated, label-free structural state in `ontopolis-world`. The implementation retains explicit seed provenance but performs no random geographic generation.

## Phase 4 Terrain Contract Status

The causal terrain boundary is implemented in `ontopolis-geography` with fixed-point elevation and roughness, property-linked surface material identity, dense deterministic storage, batch generation, and per-chunk causal provenance. No terrain synthesis algorithm or semantic geographic taxonomy was introduced.

## Phase 5 Biological Structure Status

The causal body-structure boundary is implemented in `ontopolis-biology` as a validated, canonically ordered structure-of-arrays model. Ground Truth stores only typed segment identity, rooted parent connections, property-based joint limits, fixed-point length, and physical orientation. Named anatomy, species, physiology, capability, and social taxonomy remain emergent or future concerns.

The completed pathogen extension stores typed lineage ancestry, fixed-point transmission and progression properties, canonically ordered host-lineage interactions, and causally referenced physical exposure dose. It contains no disease, symptom, pathogen-type, or transmission-route enum. Infection mutation, evolution, immunity, and epidemic processes remain future provenance-aware work.

## Phase 6 Causal Provenance Status

`ontopolis-core::provenance` implements opaque event and state-schema IDs, canonical property fingerprints, stable proposal keys, prior-cause validation, and append-only parent/child trace traversal. Event names and domain classifications are not core state. The graph records objective changes; observer/explanation systems may later classify supported subgraphs without feedback.

## Phase 7–8 Access and Perception Status

`ontopolis-perception` implements property-based physical signals, sensor apertures, deterministic accessibility filtering, relative acquired samples, and generic magnitude/change extraction with causal input spans. Signal channels are opaque schema identities, not named modality enums.

`ontopolis-cognition::attention` implements bounded fixed-point ranking over agent-local `AttentionTargetId`. It cannot store authoritative entity or feature identity. The Phase 9 boundary maps extractor bookkeeping to identity-free cues before cognition consumes it. Object categories, situations, threats, opportunities, and modality names remain emergent or observer-level.

## Phase 9–10 Subjective Continuity Status

`ontopolis-cognition` accepts identity-free, quantized `PerceptualCue` values and constructs bounded transient scenes with fallible `PerceivedObjectId` hypotheses. Subjective body parts, self-associations, working items, episodes, predictions, actions, and outcomes use opaque agent-local IDs and numeric properties only.

Working context, episodic reactivation, prediction error, agency attribution, and temporal continuity are implemented as fixed-capacity deterministic mechanisms. Sparse concepts add only agent-local prototypes over quantized signatures; beliefs add only opaque subjects, signed evidence direction, inertia, subjective trust, and directed pattern associations. They do not introduce object categories, situation enums, emotions, traits, abilities, semantic events, truth access, or authoritative identity guesses.

## Phase 13–14 Language Foundation Status

`ontopolis-language` represents abstract phonological units, ordered forms, language/lexeme ancestry, physical-form utterances, transmission records, and bounded numeric learning state. These are structural and historical carriers, not human vocabulary.

A lexeme lineage never stores a meaning. `ConceptId` associations exist only in individual subjective lexicon entries and may disagree. Words, meanings, languages as communities, polysemy, synonymy, register, grammar, and dialect remain emergent distributions rather than authoritative semantic enums.

## Phase 15–16 Practice and Epistemic Status

`ontopolis-domains` stores only bounded control flow, opaque learned action and condition identities, integer timing/tolerance, proposal records, and parent lineage. Named rituals, techniques, jobs, skills, and procedural explanations are not primitive state.

`ontopolis-epistemics` stores opaque quantity/unit/calibration identities, rational scale, fixed-point observations and uncertainty, physical glyph sequences, and explicit document-copy ancestry. Unit names, quantity categories such as “length”, document genres, textual meaning, correctness, and science remain social, subjective, or observer-level constructions.

## Phase 17 Mana Status

`ontopolis-domains::mana` stores bounded chunk-local fixed-point intensity, opaque fingerprints of canonical physical structure, numeric space/time samples, deterministic structural response, diffusion, decay, saturation, and traced replacement proposals. These are Ground Truth field mechanisms.

Spells, rituals, mana schools or types, sacredness, enchantment, skills, levels, attractors, artifacts, gods, spirits, and interpretations of a pattern are not primitive state. `PhysicalPatternId` identifies carrier structure only and must never be assigned from a semantic label or subjective category.

## Phase 19–20 Social, Economy, and City Status

`ontopolis-domains::social` keeps ownership as trace-backed contestable claims. `ontopolis-domains::economy` separately stores physical custody, positive integer quantity, material-lot ancestry, transformations, and performed agent labour. Possession does not establish ownership, and labour does not establish a job or profession.

`ontopolis-domains::city` stores physical parcel references, building entities, and generic directed infrastructure topology tied to material lots. Roads, water systems, sewers, utilities, buildings by use, districts, settlements, and cities remain agent/social/observer concepts rather than authoritative enums.

## Phase 21 Historical Bootstrap Status

`ontopolis-world::historical` stores bounded synthesis stages with typed identity, time spans, numeric detail, target chunks, canonical fingerprints, explicit dependencies, deterministic seed inputs, and committed receipts. It contains no event names, historical eras, peoples, settlements, wars, plagues, discoveries, lore, or narrative.

Receipts demonstrate orchestration-level causal continuity but do not authorize mutation or make endpoint plausibility primitive. Concrete domain adapters and their committed event traces remain responsible for objective history.
