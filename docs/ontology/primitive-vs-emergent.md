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
| Structural connection | To be implemented in Phase 5 (biology) | — |

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

### Future Primitives (Phases 3+)

| Primitive | Phase | Notes |
|-----------|-------|-------|
| Biological structure | 5 | Body segments, joints, lengths |
| Field state | 14 | Mana field properties |
| Repetition / Frequency / Sequence | 2 | Generic feature relations |
| Transformation | 2 | Change relation captures state transitions |

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
