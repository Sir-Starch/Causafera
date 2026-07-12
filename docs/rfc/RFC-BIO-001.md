# RFC-BIO-001: Minimal Biological Structural Model

**Status:** Accepted

## Summary

Define the Phase 5 authoritative representation of a biological body's structural topology: typed segment identities, parent connections, physical joint limits, fixed-point lengths, and relative orientations. The model is compact, deterministic, property-based, and contains no semantic anatomy.

## Motivation

Biology must be causal state before movement, development, injury, perception, health, or reproduction can depend on it. A semantic anatomy tree would pre-classify bodies according to developer concepts, while an unconstrained object graph would undermine validation, deterministic traversal, and data locality. Phase 5 therefore establishes only the smallest physical structural boundary.

## Authoritative primitives

### Segment identity

Every segment has a `BodySegmentId`. IDs distinguish structural entities but do not encode kind, function, location, lineage, or a human-readable name.

### Parent topology

A body has exactly one root segment. Every other segment references one parent segment, and parents precede children in canonical storage order. This produces a connected, acyclic topology with deterministic iteration.

### Length

`SegmentLengthMm` stores a strictly positive unsigned millimetre count. Explicit integer units avoid an implicit scale and eliminate floating-point rounding from authoritative length storage.

### Orientation

Each segment stores the existing Phase 2 `Orientation` primitive as its current orientation relative to its parent. Root orientation is relative to the body frame. Yaw, pitch, and roll must all be finite.

### Joint

Every non-root segment has one `Joint` describing inclusive lower and upper yaw, pitch, and roll bounds. Bounds must be finite and ordered, and the segment's current orientation must lie within them. A root has no parent joint.

Joint bounds are physical connection properties. There is no joint-type enum and no anatomical or functional classification.

## Data layout

`BodyStructure` owns private, equal-length vectors for:

- segment IDs;
- optional parent IDs;
- optional joints;
- lengths;
- orientations.

This structure-of-arrays layout supports stable sequential field iteration without per-segment allocation. `BodySegment` is a copied value view assembled at the API boundary. ID lookup is a deterministic linear scan in this phase; an indexed side store requires measurement before introduction.

## Construction and validation

All authoritative construction passes through validated constructors. They reject:

- empty structures or mismatched field lengths;
- duplicate segment IDs;
- zero or multiple roots;
- a root with a parent joint;
- a child without a parent joint;
- missing or non-preceding parents;
- zero length;
- non-finite orientation or joint values;
- inverted joint bounds;
- current orientations outside joint bounds.

Vectors remain private after construction so valid topology cannot be invalidated through ad hoc mutation.

## Determinism

The structural model consumes no randomness. The validated topological vector order is canonical traversal order. Validation uses no hash-map iteration, system time, hardware entropy, pointer identity, locale, or labels. Identical input fields produce identical structural state.

The existing physical orientation uses floating-point radians. Phase 5 accepts only finite values and performs comparisons but no platform-sensitive transcendental operations. Cross-platform canonical numeric encoding remains a persistence concern.

## Mutation and provenance

Phase 5 defines immutable construction state only. Later growth, injury, degeneration, or adaptation must use scheduler-controlled proposal/reduce/commit phases and retain causal event provenance. Those future processes may not bypass validation or permit the Explanation Engine, UI, or an LLM to mutate authoritative bodies.

## Primitive and emergent boundary

Ground Truth contains structural identities and measurable connection geometry. It does not contain labels such as finger, head, limb, wing, tail, hand, skeletal, species, race, disability, grasping organ, or locomotion organ. Agents may form concepts from observations; observer analytics may apply explicitly non-authoritative human glosses.

## Performance and memory

The five structural fields are contiguous vectors and joint state is inline and fixed-size. Construction uses a temporary ordered set for uniqueness and parent validation, then discards it. No scale, throughput, or memory-efficiency claim is made. Later benchmark work should measure construction, sequential field iteration, bytes per segment, and lookup strategies on representative bodies.

## Observer and explanation boundaries

This phase adds no observer protocol. A later read model may expose derived geometry without sharing mutable Ground Truth. Observer classifications cannot feed back into simulation.

This phase also adds no Explanation IR. Future structural changes must retain causal trace references so explanations can inspect rather than invent their causes.

## Cross-domain effects

Later systems may consume the structure for movement constraints, physical access, sensory geometry, development, heredity, injury, aging, physiology, or material interaction. Phase 5 derives no capabilities or semantic functions from topology.

## Non-goals

- Cellular, tissue, organ, or physiological simulation.
- Hunger, health, disease, metabolism, or sensory processing.
- Growth, reproduction, heredity, aging, injury, or regeneration.
- Semantic anatomy, species, social categories, or demo organisms.
- Movement solving, inverse kinematics, collisions, or animation.
- Observer schemas, persistence formats, explanation rendering, or GPU work.

## Decisions

- Use typed, label-free segment identity.
- Use one rooted tree with parent-before-child canonical order.
- Store length as fixed-point millimetres.
- Reuse physical `Orientation` and require finite components.
- Represent joints only as angular bounds, without named joint types.
- Store body state as private structure-of-arrays vectors with value-view access.

## Unresolved future questions

- How structural mutation proposals and causal provenance are encoded.
- Whether measured workloads justify a dense ID index or lineage template/delta representation.
- How material composition, mass, surface geometry, and attachment anchors integrate without semantic shortcuts.
- How multiresolution aggregation preserves causal effects for inactive organisms.
