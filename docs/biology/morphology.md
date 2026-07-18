# Morphology

Morphology is the study of biological form and structure. In Causafera, it describes the physical configuration of organisms.

## Morphological Representation

Organisms are represented as structured bodies:

```text
MorphologyState:
    segment_ids: [BodySegmentId]
    parent_segments: [Option<BodySegmentId>]
    joints: [Option<Joint>]
    lengths_mm: [SegmentLengthMm]
    relative_orientations: [Orientation]
```

Phase 5 implements this state as the validated `BodyStructure` structure-of-arrays container. It requires a single root and canonical parent-before-child order.

### Body Segment

```text
BodySegment:
    segment_id: BodySegmentId
    parent_segment: Option<BodySegmentId>
    joint: Option<Joint>
    length: SegmentLengthMm
    orientation: Orientation
```

Segment types (observer classifications):
- head-like
- torso-like
- limb-like
- appendage-like
- tail-like

Ground Truth stores:
- parent_segment
- joint connections
- length
- orientation
- movement samples

### Joint

```text
Joint:
    lower_orientation: Orientation
    upper_orientation: Orientation
```

The bounds are inclusive physical yaw/pitch/roll constraints. Root segments have no parent joint; every connected segment has one. Named joint types, degrees-of-freedom categories, and anatomical functions are not authoritative state.

## Phase 5 Validation

Authoritative construction rejects mismatched field lengths, duplicate IDs, disconnected or cyclic topology, invalid root/joint relationships, zero lengths, non-finite values, inverted joint bounds, and orientations outside those bounds. Vectors remain private after validation.

The Phase 5 model does not yet represent mass, material/tissue composition, surface geometry, attachment anchors, movement samples, growth, injury, or function.

## Morphological Variation

Morphology varies between individuals and lineages:

- **Size**: stature, mass, segment proportions
- **Shape**: segment dimensions, attachment angles
- **Number**: segment count (e.g., fingers, vertebrae)
- **Proportions**: relative segment lengths

## Morphology and Function

Morphology determines:

- **Movement**: joint configuration determines gait and dexterity
- **Perception**: head configuration determines sensory field
- **Manipulation**: appendage configuration determines grasping ability
- **Metabolism**: body size determines metabolic requirements
- **Defense**: morphology may provide armor, camouflage, or escape ability

## Morphology and Society

Societies may construct categories based on morphology:

- **Occupational**: certain morphologies may be suited to specific tasks
- **Social**: morphological features may become status markers
- **Taxonomic**: societies may classify organisms by morphology

These categories are emergent, not primitive.

## Morphological Development

Morphology changes over the lifespan:

- **Growth**: segments increase in size
- **Proportion change**: relative segment lengths shift
- **Degeneration**: joints wear; segments atrophy
- **Injury**: segments may be damaged or lost
- **Adaptation**: repeated use may alter segment development

## Determinism

Morphological development must be deterministic given:

- genetic parameters
- developmental conditions
- age
- injury history

## Performance

Morphological data may be detailed. Strategies:

- Standard morphologies for common lineages
- Delta representation for individual variation
- Sparse representation for distant or inactive organisms

## Related Documents

- `architecture.md` — biological system overview
- `physiology.md` — functional processes
- `development.md` — morphological development
- `heredity.md` — genetic determination of morphology
- `aging.md` — morphological degeneration

## TODO Categories

- `BIO` — biology
