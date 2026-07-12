# RFC-ONTO-001: Primitive Simulation Ontology

**Status:** Accepted

## Summary

Define the primitive simulation ontology: what objectively exists in the engine versus what emerges from agent cognition. Phase 2 completes the primitive inventory and implements generic feature representation types.

## Motivation

Prevent semantic shortcuts from entering the engine as primitive systems. Every foundational domain must answer fourteen coverage questions before implementation. The primitive/emergent boundary is the first and most important coverage question.

## Details

### Phase 1 Primitives (Implemented)

- **Space**: `WorldCoord`, `ChunkCoord`, `LocalCoord` — absolute integer spatial addressing
- **Time**: `SimulationTime` — discrete tick counter with phase-aware scheduling
- **Position**: derived from `WorldCoord`
- **Proximity**: integer distance on coordinate grid
- **Containment**: chunk/local coordinate conversion

### Phase 2 Primitives (Implemented)

#### Physical Primitives

- **Matter**: `Material` struct with physical properties (density, thermal_conductivity, hardness, porosity, specific_heat). No semantic material names.
- **Temperature**: `Temperature` struct wrapping Kelvin as `f64`.
- **Orientation**: `Orientation` struct with yaw, pitch, roll.
- **Motion**: `Motion` struct with linear velocity and angular velocity.
- **Energy-related state**: represented through temperature and material thermal properties.
- **Material composition**: `Material` stores component proportions by `SubstanceId` and ratio.

#### Generic Perceptual Feature Primitives

Generic feature relations extract structural patterns from raw Ground Truth state. They carry no semantic labels.

- `FeatureRelation` enum: Change, Magnitude, Direction, Variance, Periodicity, Synchrony, Recurrence, Duration, SpatialRelation, TemporalRelation, CoOccurrence, StructuralSimilarity, RelativeDifference, SequenceSimilarity.
- `FeatureValue` enum: Scalar(f64), Direction(Direction3D), FrequencyBand(u8), MagnitudeBand(u8).
- `Persistence` enum: Fleeting, Brief, Moderate, Persistent, High.
- `Feature` struct: ties relation, value, persistence, target entity, and feature ID.

### Future Primitives (Pending)

| Primitive | Phase | Representation |
|-----------|-------|---------------|
| Biological structure | 5 | Body segments, joints, lengths |
| Field state | 14 | Mana field properties |
| Structural connection | 5 | Parent/child segment links |

### Emergent Concepts (Never Primitive)

The following are always emergent, never engine primitives:

- Biological: finger, tremor, disease, symptom, anatomy
- Social: race, class, skill, profession, criminal, scientist, religion, law, contract, organization
- Cognitive: concept, belief, goal, habit, trust, monster, sacred stone
- Linguistic: word, meaning, grammar, dialect, writing system
- Geographic: kingdom, border, trade route, settlement, city

## Primitive vs Emergent Review

- `FeatureRelation` and `FeatureValue` are generic, pattern-level relations. They carry no semantic labels like "Tremor" or "Disease".
- `Temperature`, `Orientation`, `Motion` are physical primitives that exist in Ground Truth regardless of agent recognition.
- `Material` stores physical properties, not material names like "wood" or "stone". Agents may later construct concepts grouping materials by properties.
- No semantic enums (`Class`, `Skill`, `Monster`, `Disease`) enter the engine as primitive systems.

## Determinism

All primitive types are passive data structures. Determinism is preserved by construction: no randomness, no locale dependence, no semantic shortcuts.

## Performance

Feature and physics types are compact and stack-allocated. No heap allocation in the hot path. `Feature` is 48 bytes or less.

## Unresolved Questions

- Emergence detection criteria: how do we programmatically detect that an agent has formed a concept? (Deferred to Phase 11 concept formation.)
- Feature extraction algorithms: how are generic features actually extracted from state? (Deferred to Phase 7-12 cognition.)

## Decision Log

- **Accepted**: Use `f64` for physical scalars. Rationale: sufficient precision for simulation; fixed-point can be introduced later if determinism issues arise.
- **Accepted**: Keep `Material` property-based, not taxonomy-based. Rationale: prevents semantic shortcut (INV-005, INV-006).
