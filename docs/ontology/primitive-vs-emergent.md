# Primitive vs Emergent

Ontopolis must explicitly distinguish between what the engine assumes as physical reality and what simulated agents construct as subjective categories.

## Engine Primitives

Primitives are properties required to define the physical or computational universe. They exist in Ground Truth regardless of whether any agent recognizes them.

Examples of engine primitives:

- space
- time
- matter
- position
- orientation
- motion
- energy-related state
- temperature
- material composition
- structural connection
- biological structure
- field state
- repetition
- frequency
- sequence
- proximity
- containment
- transformation

The engine may contain a structural biological model with segments, joints, lengths, and movement samples. It must not assume that agents automatically possess developer-defined biological labels.

## Emergent Human Concepts

Emergent concepts are categories created by agents or societies through perception, learning, and social transmission. They may or may not correspond to Ground Truth structure.

Examples of emergent human concepts:

- finger
- tremor
- disease
- race
- class
- skill
- monster
- sacred stone
- criminal
- profession
- religion
- scientist

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
