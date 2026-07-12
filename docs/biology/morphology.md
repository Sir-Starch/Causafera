# Morphology

Morphology is the study of biological form and structure. In Ontopolis, it describes the physical configuration of organisms.

## Morphological Representation

Organisms are represented as structured bodies:

```text
MorphologyState:
    segments: [BodySegment]
    joints: [Joint]
    attachment_graph: Graph
    overall_proportions: ProportionSet
```

### Body Segment

```text
BodySegment:
    segment_id: BodySegmentId
    parent_segment: Option<BodySegmentId>
    segment_type: SegmentType
    dimensions: Dimensions
    mass: float
    tissue_composition: {TissueType → proportion}
    surface_properties: SurfaceProperties
    articulation_points: [JointId]
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
    joint_id: JointId
    connected_segments: (BodySegmentId, BodySegmentId)
    joint_type: JointType
    degrees_of_freedom: int
    range_of_motion: Range
    current_position: Position
```

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
