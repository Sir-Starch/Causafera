# Phase 5 Biological Structural Model ExecPlan

## Goal

Define a compact, validated, label-free representation of biological body-segment structure containing typed identity, parent connection, physical joint constraints, length, and relative orientation, without implementing physiology or semantic anatomy.

## Context

Phase 5 implements `TODO-BIO-001` after the spatial and terrain foundations. The existing biology crate contains only a placeholder `BodySegment` with an unvalidated floating-point length. This phase establishes authoritative structural state that later movement, development, injury, perception, and physiology may consume.

## Relevant invariants

- INV-001 and INV-002: body structure contains no semantic anatomy enum or authoritative English label.
- INV-004: agents may observe structure later but cannot directly access biological Ground Truth.
- INV-009: biological structure is causal state.
- INV-014 and INV-016: later structural mutations require causal provenance and phase-controlled proposal/commit; this phase defines immutable construction contracts only.
- INV-017 and INV-018: hot structural fields use deterministic dense storage; no performance claim is made without a benchmark.
- INV-021 and INV-022: observer classifications and renderings remain outside authoritative state.
- INV-024: physical morphology must not encode social or taxonomic categories.

## Ontology domains affected

- Biology: body-segment topology and articulation become authoritative causal structure.
- Matter and space: lengths, orientations, and constrained physical connections reuse property-based physical primitives.

## Causal carriers affected

- Structural connection: parent and joint state connect body segments.
- Position and motion: segment orientation and joint limits can later constrain movement.
- Material contact, energy transfer, heredity, and sensory acquisition remain downstream work.

## Relevant documents

- `docs/biology/architecture.md`
- `docs/biology/morphology.md`
- `docs/biology/physiology.md`
- `docs/ontology/domain-coverage-matrix.md`
- `docs/ontology/primitive-vs-emergent.md`
- `docs/ontology/causal-carriers.md`
- `docs/ontology/cross-domain-interactions.md`
- `docs/architecture/invariants.md`
- `docs/architecture/data-oriented.md`
- `docs/architecture/determinism.md`
- `docs/adr/ADR-001.md`
- `docs/adr/ADR-002.md`
- `docs/adr/ADR-003.md`
- `docs/adr/ADR-004.md`
- `docs/rfc/RFC-BIO-001.md`

## Current state

`causafera-types` already supplies `BodySegmentId` and the generic physical `Orientation` primitive. `causafera-biology::morphology` contains a public three-field placeholder with no orientation, joint, compact body store, validation, deterministic order, or tests. `RFC-BIO-001` leaves structural representation unresolved.

## Proposed architecture

Represent length as an unsigned fixed-point millimetre value. Represent each joint as lower and upper yaw/pitch/roll bounds using the existing physical `Orientation`; the representation describes permitted relative rotations and carries no named joint type. Keep `BodySegment` as a value view with ID, optional parent, optional joint, length, and relative orientation.

Store complete morphology in a `BodyStructure` structure-of-arrays container. Construction validates equal field lengths, non-empty structure, unique IDs, exactly one root, root-without-joint and child-with-joint consistency, parent-before-child topological order, positive lengths, finite orientations, and finite ordered joint limits. Stable vector order is canonical iteration order.

## Primitive vs emergent review

Typed identity, structural parentage, angular constraints, length, and orientation are physical primitives. Names such as finger, head, limb, wing, joint type, species, race, disability, and anatomical function are observer or agent classifications and are not stored.

## Non-goals

- Physiology, hunger, health, disease, tissue simulation, metabolism, or sensory function.
- Growth, heredity, reproduction, aging, injury, regeneration, or morphology mutation.
- Named segment/joint types, anatomy labels, species, social categories, or demo organisms.
- Movement solvers, collision geometry, inverse kinematics, or animation.
- Observer protocol, explanation rendering, persistence schemas, or GPU code.

## Implementation stages

1. Define fixed-point segment length, physical joint constraints, and a complete segment value view.
2. Define dense `BodyStructure` storage, validation errors, canonical accessors, and ID lookup.
3. Test valid construction, layout access, and every topology/physical validation boundary.
4. Accept and develop RFC-BIO-001, then update biology/ontology documentation, TODO, roadmap, changelog, and plan registration.

## Verification

- `cargo fmt --all -- --check`
- `cargo test --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `git diff --check`

## Benchmark plan

No scale or throughput claim is introduced. A later benchmark should measure construction validation, sequential field iteration, ID lookup strategy, and bytes per segment over representative morphology sizes before optimization claims are made.

## Determinism impact

Structure contains no RNG. Integer length values avoid unit and rounding ambiguity. Floating-point orientations are accepted only when finite, matching the existing Phase 2 physical primitive. Canonical topology and iteration order are explicit and independent of hash-map iteration.

## Memory impact

Hot segment attributes are stored in five contiguous vectors with no per-segment heap allocation. Optional fixed-size joint values remain inline. Construction currently uses a temporary ordered set for uniqueness and parent validation; it is discarded after construction.

## Observer impact

None in this phase. Future observer read models may expose derived structural geometry and human-facing classifications without sharing mutable Ground Truth or feeding classifications back into simulation.

## Explanation impact

None in this phase. Future causal changes to morphology must reference event traces; the Explanation Engine may inspect those traces but cannot mutate structure.

## Persistence impact

No stable persistence schema is defined. Typed IDs, explicit units, and canonical topological order make a future encoding possible, but current Rust layout is not a snapshot format.

## Cross-domain effects

Later movement may consume lengths, orientations, and joint limits. Development, heredity, injury, aging, physiology, perception, and material interactions may propose causal changes or derive capabilities. No such behavior is implemented here.

## Risks

- Named anatomy could leak into Ground Truth; the contract stores only physical properties and typed identities.
- Arbitrary graph order could weaken deterministic iteration; construction requires parent-before-child topological order.
- Invalid floats could poison later calculations; construction rejects non-finite orientation and limit values.
- A structure-of-arrays API could permit mismatched fields; all vectors are private and validated together.
- Joint data could imply physiology or function; it records only angular constraints on a structural connection.

## Documentation changes

Develop RFC-BIO-001 and update morphology, biological architecture, ontology status, roadmap, root changelog, and completed-plan registration.

## TODO changes

Mark `TODO-BIO-001` completed only after all verification passes.

## Decision log

- 2026-07-12: Use millimetre fixed-point segment lengths and reuse the Phase 2 `Orientation` physical primitive.
- 2026-07-12: Model joints only as property-based relative angular bounds, never named joint types.
- 2026-07-12: Require one root and parent-before-child order to give bodies canonical deterministic topology.
- 2026-07-12: Use private structure-of-arrays vectors with value-view access.

## Progress

- [x] Required vision, ontology, architecture, biology, ADR, and RFC context reviewed.
- [x] Biological structural primitives and validation implemented.
- [x] Focused and workspace verification passes (`cargo test`, Clippy with warnings denied, formatting check, and `git diff --check`).
- [x] RFC, documentation, TODO, roadmap, and changelog updated.
