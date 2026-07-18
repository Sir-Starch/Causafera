# Phase 4 Terrain Generation Contracts ExecPlan

## Goal

Define deterministic, batch-oriented contracts for causal terrain state containing elevation, surface material identity, and roughness per surface cell, without implementing a terrain synthesis algorithm or fictional geography.

## Context

Phase 3 established deterministic spatial containment. Phase 4 begins minimal causal geography with `TODO-GEO-001`: the terrain data and generation boundaries that later geology, erosion, hydrology, climate, ecology, movement, and visibility systems can consume.

## Relevant invariants

- INV-006: authoritative terrain contains no human-language labels.
- INV-009: terrain is physical causal state, not rendering decoration.
- INV-014 and INV-023: generated chunks retain explicit generation provenance.
- INV-016: this phase defines generation contracts, not ad hoc scheduler mutation.
- INV-017 and INV-018: terrain storage is dense and batch-oriented; no throughput claim is made without a benchmark.
- INV-021 and INV-022: observer and rendering representations remain outside authoritative terrain state.

## Ontology domains affected

- Space: surface cells are addressed within spatial chunks.
- Matter: surface state references property-defined `MaterialId` values.
- Geography: elevation and roughness become authoritative physical state.

## Causal carriers affected

- Position and motion: elevation and roughness can later affect movement and visibility.
- Material contact: surface material identity can later connect terrain to permeability, hardness, extraction, and other physical effects.
- Energy transfer and ecological pressure are downstream consumers, not implemented here.

## Relevant documents

- `docs/world/geography-philosophy.md`
- `docs/world/terrain.md`
- `docs/world/geology.md`
- `docs/world/hydrology.md`
- `docs/world/climate.md`
- `docs/world/ecology.md`
- `docs/world/coordinates.md`
- `docs/world/spatial-hierarchy.md`
- `docs/world/world-generation-provenance.md`
- `docs/ontology/domain-coverage-matrix.md`
- `docs/ontology/primitive-vs-emergent.md`
- `docs/ontology/causal-carriers.md`
- `docs/architecture/data-oriented.md`
- `docs/architecture/determinism.md`
- `docs/architecture/performance.md`
- `docs/adr/ADR-002.md`
- `docs/adr/ADR-003.md`
- `docs/rfc/RFC-GEO-001.md`
- `docs/rfc/RFC-HYDRO-001.md` (reviewed; hydrology remains out of scope)

## Current state

`causafera-geography` contains placeholder `TerrainCell`, `GeologyLayer`, `HydrologyCell`, and `ClimateCell` structs. `TerrainCell` stores only an unqualified `f32` elevation. There is no terrain chunk layout, generation request, provenance record, validation, or batch contract.

## Proposed architecture

Replace the terrain placeholder with fixed-point value types for elevation and roughness, both with explicit millimetre units. Store each terrain chunk as structure-of-arrays vectors for elevation, `MaterialId`, and roughness, indexed in deterministic row-major surface order. A validated constructor enforces exactly `CHUNK_SIZE × CHUNK_SIZE` cells.

Define a `TerrainGenerationRequest` containing a chunk coordinate and a compact `TerrainGenerationProvenance` record. Provenance records the world seed, generation trace, generator fingerprint, parameter fingerprint, and ordered causal input traces. Define a `TerrainGenerator` trait whose required operation accepts an ordered slice of requests and returns an equally ordered batch of chunks. A validation helper rejects output count, coordinate, and provenance mismatches at the boundary.

## Primitive vs emergent review

Elevation and roughness are measurable physical quantities. Surface material is a typed identity whose physical properties live in `Material`; it is not an enum such as rock, soil, sand, sacred stone, or ore. Terrain chunks carry no biome, settlement, political, visual, or human-language category.

## Non-goals

- A noise function, tectonic model, erosion model, or generated terrain fixture.
- Geological layers, hydrology, climate, ecology, biomes, or vegetation.
- Terrain mutation during simulation ticks.
- Movement-cost, visibility, settlement-suitability, or rendering derivations.
- Persistence and observer protocol schemas.
- GPU implementation or performance claims.

## Implementation stages

1. Define fixed-point terrain cell values and dense chunk storage with validation.
2. Define generation requests, compact causal provenance, the required batch trait, and output validation.
3. Test layout, validation, batch ordering, provenance preservation, and deterministic replay with a property-based test generator.
4. Develop and accept RFC-GEO-001, then update terrain/ontology documentation, TODO, roadmap, changelog, and this plan.

## Verification

- `cargo fmt --all -- --check`
- `cargo test --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `git diff --check`

## Benchmark plan

No performance claim is introduced. Once the benchmark harness exists, measure terrain-chunk construction, sequential per-field iteration, batch validation, and bytes per surface cell for representative batch sizes.

## Determinism impact

Authoritative terrain values use integers with explicit units. Generation inputs are explicit and ordered. Implementations must derive output solely from request state and must not consume system time, hardware entropy, global RNG state, pointer values, or batch scheduling order. Validation requires output order to match request order.

## Memory impact

Hot cell fields use three contiguous vectors and no per-cell allocation. Provenance is stored once per chunk, with causal input traces in a cold per-chunk vector. The constructor rejects malformed field lengths.

## Observer impact

None in this phase. Future observer read models may derive height maps and material glosses without exposing mutable terrain storage or feeding classifications back into state.

## Explanation impact

Each terrain chunk retains machine-readable generation trace and input trace references. The Explanation Engine may later traverse those records but cannot mutate them.

## Persistence impact

No persistence schema is defined. Integer units and deterministic field order make a future canonical encoding possible, but the internal vectors are not a stable snapshot format.

## Cross-domain effects

Later geology can provide surface material and elevation inputs; hydrology can consume elevation; climate can consume elevation; ecology can consume physical terrain and material properties; movement and visibility can consume elevation and roughness. None of those effects is implemented here.

## Risks

- A material field could become a hidden narrative taxonomy; using only `MaterialId` preserves the property-based ontology.
- Floating-point storage could weaken replay guarantees; fixed-point millimetre values avoid that ambiguity at the terrain boundary.
- Batch implementations could reorder results; validation rejects output whose coordinate or provenance does not match the same request index.
- Per-cell provenance would be prohibitively large; provenance is recorded once per generated chunk in this phase.

## Documentation changes

Develop RFC-GEO-001 and update terrain, ontology status, roadmap, root changelog, and completed-plan registration.

## TODO changes

Mark `TODO-GEO-001` completed only after all verification passes.

## Decision log

- 2026-07-12: Represent authoritative terrain elevation and roughness as fixed-point millimetres.
- 2026-07-12: Use `MaterialId`, never a semantic terrain-material enum.
- 2026-07-12: Require batch generation as the primary trait operation and validate ordered output at the contract boundary.
- 2026-07-12: Store generation provenance once per chunk with trace references rather than per cell.

## Progress

- [x] Required vision, ontology, architecture, geography, ADR, and RFC context reviewed.
- [x] Terrain state and batch generation contracts implemented.
- [x] Focused and workspace verification passes (`cargo test`, Clippy with warnings denied, formatting check, and `git diff --check`).
- [x] RFC, documentation, TODO, roadmap, and changelog updated.
