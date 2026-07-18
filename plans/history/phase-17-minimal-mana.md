# Phase 17 Minimal Information-Sensitive Mana

> **Historical record.** This completed ExecPlan describes a Foundation Era project stage. Its implementation status and terminology may be outdated; use [the documentation index](../../docs/index.md), [roadmap](../../docs/roadmap/roadmap.md), and [active plans](../../PLANS.md) for current guidance.

## Goal

Implement a bounded deterministic local mana field which responds to physically grounded recurrence, periodicity, synchronization, and spatial repetition without inspecting semantic meaning.

## Context

Phase 15 practices and Phase 16 documents now emit physical/informational structures, but the mana domain is still an unbounded floating-point placeholder. RFC-MANA-001 is proposed and leaves the field contract unresolved.

## Relevant invariants

INV-003, INV-004, INV-006, INV-014, INV-016, INV-017, INV-018, and INV-019.

## Ontology domains affected

Mana, space, time, geography, physical patterns, provenance, future practices, and future causal resolution.

## Causal carriers affected

Local fixed-point field state, opaque physical pattern fingerprints, sampled activity, and trace-backed cell changes.

## Relevant documents

`docs/vision/project-thesis.md`, `docs/architecture/invariants.md`, `docs/architecture/determinism.md`, `docs/architecture/provenance.md`, `docs/world/mana-topology.md`, `docs/ontology/causal-carriers.md`, and `docs/rfc/RFC-MANA-001.md`.

## Current state

`causafera-domains::mana` contains only an unbounded `Vec<f32>` initialized to zero. It has no spatial identity, bounds, pattern inputs, deterministic evolution, diffusion, saturation, or provenance output.

## Proposed architecture

Represent a chunk-local dense fixed-point scalar field with a validated cubic extent. Evolution is a pure read/propose operation: canonically ordered `PhysicalPatternSample` inputs are grouped by opaque physical fingerprint, and structural scores are derived from repeated samples, regular intervals, simultaneous occurrences, and repeated coordinates. Scores inject local field energy. A deterministic six-neighbour stencil then applies diffusion, decay, and saturation. The result is a replacement field plus trace-backed changed-cell records; scheduler integration will commit it in an authoritative phase.

## Primitive vs emergent review

Numeric field intensity, coordinates, sample time, physical fingerprints, recurrence counts, interval regularity, and diffusion are primitives. Spells, rituals, sacredness, words, beliefs, skills, attractors, and magical effect taxonomies remain emergent or future research.

## Non-goals

Semantic pattern recognition, spell systems, belief coupling, practice interpretation, stateful attractors, metaphysics, GPU acceleration, causal resolution, scheduler integration, observer protocol changes, or claimed real-world field physics.

## Implementation stages

1. Replace the floating-point placeholder with validated fixed-point field and parameter contracts.
2. Add canonical physical sample analysis and pure deterministic field evolution.
3. Add boundary, determinism, semantic-separation, diffusion, and saturation tests.
4. Accept RFC-MANA-001 and update roadmap, TODO, ontology, subsystem documentation, changelog, and plan registry.

## Verification

Run workspace tests, strict clippy, formatting check, diff check, architectural searches for floats/strings/subjective IDs in mana, and refresh the knowledge graph.

## Benchmark plan

No throughput claim is made. The implementation uses bounded samples and field extents. Before optimization, benchmark evolution cost by active sample count and field volume; only then consider sparse or accelerated layouts.

## Determinism impact

No RNG, floats, hash iteration, locale, or system time. Samples are canonicalized, arithmetic is saturating fixed-point integer arithmetic, and stencil traversal is stable row-major order.

## Memory impact

Field volume is capped at one chunk, sample batches are capped, and changed cells and cause lists cannot exceed those input/state bounds.

## Observer impact

Future read-only projections may visualize numeric field intensity and trace-backed changes. No observer API changes are included.

## Explanation impact

Changed-cell records identify physical sample traces that supported local injection. They do not infer semantic causes or magical narratives.

## Persistence impact

No persistence format change. The new field is deterministic plain Rust state, but serialization remains future work.

## Cross-domain effects

Geography supplies spatial context; physical activity, acoustics, practice emissions, and glyph geometry may later produce samples. Phase 18 causal resolution may consume field activity, but mana cannot consume resolution policy or semantic state.

## Risks

- Opaque fingerprints could be populated from semantic labels by integration code; the API and RFC require fingerprints of canonical physical structure only.
- A dense stencil may be expensive; volume is bounded and no scale claim is made.
- Integer saturation may conceal excess energy; changed-cell records expose saturated results, and richer diagnostics remain future work.

## Documentation changes

Expand and accept RFC-MANA-001; update mana topology, ontology coverage/carriers/primitive review, assumptions, roadmap, TODO, changelog, and documentation index if new documents are added.

## TODO changes

Complete `TODO-MANA-001` only after all verification passes.

## Decision log

- 2026-07-12: Keep Phase 17 separate from Phase 18; causal resolution is foundational work with its own RFC and ExecPlan.
- 2026-07-12: Use pure replacement-state proposals so authoritative commits remain scheduler controlled.
- 2026-07-12: Treat the model as a minimal simulation contract, not a claim about final mana physics.

## Progress

- [x] Fixed-point spatial field and parameters implemented.
- [x] Physical pattern analysis and proposal-only evolution implemented.
- [x] Tests and architectural checks pass.
- [x] RFC, roadmap, TODO, ontology, and subsystem documentation updated.
