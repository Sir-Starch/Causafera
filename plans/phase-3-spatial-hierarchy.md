# Phase 3 Spatial Hierarchy ExecPlan

## Goal

Implement the authoritative spatial containment skeleton from world through interior space, with deterministic construction and efficient parent/child traversal, without generating terrain, settlements, political regions, or fictional content.

## Context

Phase 1 established deterministic time, scheduling, random streams, and coordinates. Phase 2 established generic physical and pattern primitives. Phase 3 now needs the structural containment substrate required by later geography, biology, causal resolution, and observer work.

## Relevant invariants

- INV-006: hierarchy state contains no human UI labels.
- INV-009: geography remains causal state, not decoration.
- INV-010: containment does not imply simulation resolution.
- INV-014 and INV-023: the construction seed is retained as generation provenance.
- INV-016: this phase defines construction, not scheduler-time mutation.
- INV-017 and INV-018: storage is data-oriented and no throughput claim is made without a benchmark.
- INV-021 and INV-022: no UI or rendering state enters the hierarchy.

## Ontology domains affected

- Space: authoritative nested containment.
- Geography: only the structural substrate; terrain, geology, hydrology, climate, and ecology remain Phase 4 work.

## Causal carriers affected

- Position and containment. The hierarchy provides structural adjacency but does not itself create domain effects.

## Relevant documents

- `docs/world/spatial-hierarchy.md`
- `docs/world/coordinates.md`
- `docs/world/geography-philosophy.md`
- `docs/world/world-generation-provenance.md`
- `docs/ontology/world-ontology.md`
- `docs/ontology/causal-carriers.md`
- `docs/ontology/primitive-vs-emergent.md`
- `docs/architecture/data-oriented.md`
- `docs/architecture/determinism.md`
- `docs/architecture/performance.md`
- `docs/adr/ADR-002.md`
- `docs/adr/ADR-003.md`
- `docs/rfc/RFC-GEO-001.md` (reviewed; implementation remains Phase 4)

## Current state

`causafera-world` contains only a placeholder `World` holding a root `ChunkId`. Coordinate primitives and `PlaceId`/`ChunkId` already exist in `causafera-types`. No spatial hierarchy, validation, or traversal API exists.

## Proposed architecture

Add an immutable `SpatialHierarchy` backed by dense node and child arrays. `PlaceId` is a stable dense index. Each node stores its structural level, optional parent, and a compact range into a contiguous child array. A `SpatialHierarchyBuilder` creates the root, validates each parent-child level transition, assigns IDs in insertion order, and finalizes child adjacency deterministically. `World` owns the finalized hierarchy and records the world seed through it.

Chunk hierarchy nodes map to `ChunkId` using the same numeric identity, avoiding a duplicate lookup table. Conversion APIs validate the node level before crossing the type boundary.

## Primitive vs emergent review

The level taxonomy represents objective containment kinds required by the documented world model. It does not encode political borders, ownership, culture, settlements, biomes, English place names, or agent concepts. Human labels remain absent from authoritative state.

## Non-goals

- Terrain or geographic feature generation.
- Political or administrative hierarchy.
- Ownership, jurisdiction, or land use.
- Causal resolution or aggregation behavior.
- Persistence and observer protocol schemas.
- Mutable hierarchy edits during simulation ticks.

## Implementation stages

1. Define hierarchy levels, validated builder errors, dense nodes, and immutable traversal.
2. Replace the placeholder `World` with a seed-explicit wrapper over a finalized hierarchy.
3. Test exact level transitions, invalid transitions, deterministic construction, traversal, and chunk identity conversion.
4. Update TODO, spatial documentation, changelog, and this plan.

## Verification

- `cargo fmt --all -- --check`
- `cargo test --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- Focused tests verify the complete nine-level chain and deterministic equality.

## Benchmark plan

No performance claim is introduced. Once `TODO-PERF-001` supplies the benchmark harness, benchmark finalization cost, bytes per node, direct parent lookup, and sequential child iteration at representative hierarchy sizes.

## Determinism impact

IDs and child ordering are derived solely from explicit builder call order. Finalization uses stable insertion-order passes and no hash iteration. The retained seed makes the construction input explicit; no randomness or system time is consumed.

## Memory impact

Nodes and child IDs are stored in contiguous vectors. Child ranges use 32-bit offsets/counts, with checked capacity limits. No per-node heap allocation or strings are introduced.

## Observer impact

None in this phase. Future observer read models may derive hierarchy navigation data without accessing mutable internals.

## Explanation impact

None in this phase. The seed is retained as minimal generation provenance; richer causal provenance belongs to later generation systems.

## Persistence impact

No snapshot format is defined. The immutable layout is internal and must not be treated as a stable wire or persistence schema.

## Cross-domain effects

Later geography, hydrology, ecology, structures, interiors, causal resolution, and observer navigation can attach state to validated spatial nodes. Political claims remain separate overlays.

## Risks

- A generic level enum could invite invalid containment; the builder rejects every transition outside the documented chain.
- Dense IDs could be confused with chunk IDs; conversion is allowed only for nodes at the chunk level.
- Mutable insertion after finalization could invalidate ranges; the finalized hierarchy is immutable.

## Documentation changes

Update spatial hierarchy implementation notes, root changelog, and completed-plan registration.

## TODO changes

Mark `TODO-WORLD-001` completed only after all verification passes.

## Decision log

- 2026-07-12: Use one dense `PlaceId` namespace plus validated structural levels instead of nine separate object graphs.
- 2026-07-12: Use CSR-style child adjacency for compact, ordered child traversal.
- 2026-07-12: Retain the explicit world seed but do not invent a random world generator in Phase 3.

## Progress

- [x] Required vision, ontology, architecture, world, ADR, and RFC context reviewed.
- [x] Hierarchy implementation complete.
- [x] Tests and quality gates pass (`cargo test`, Clippy with warnings denied, and formatting check).
- [x] Documentation and TODO updated.
