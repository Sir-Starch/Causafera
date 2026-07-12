# RFC-GEO-001: Minimal Causal Geological World Model

**Status:** Accepted

## Summary

Define the minimal causal terrain boundary and the provenance requirements that connect surface material state to later geological generation and material chains.

## Motivation

Terrain must affect later simulation through physical quantities rather than narrative modifiers. Generated state also needs enough machine-readable provenance to support deterministic replay and later tracing of extracted materials back to geographic causes.

## Details

### Authoritative Terrain State

RFC-GEO-002 subsequently clarifies that each terrain grid is chart-local surface state. Bare `ChunkCoord` is not a unique global planetary address; future global terrain uses `ChartChunkCoord` and registered chart transforms.

Each chunk surface contains a dense `CHUNK_SIZE × CHUNK_SIZE` grid with three Phase 4 fields:

- absolute elevation in signed integer millimetres;
- surface material identity as `MaterialId`;
- non-negative local elevation variation in integer millimetres.

`MaterialId` refers to the property-based `Material` representation. It is not a material taxonomy. Names or categories such as rock, soil, ore, or sacred stone must not enter authoritative terrain state.

Slope, aspect, soil depth, bedrock depth, vegetation cover, and water depth remain later derived or generated fields. They are not fabricated to make the Phase 4 structure look complete.

### Data Layout

Terrain chunks use structure-of-arrays storage: contiguous elevation, material, and roughness fields in row-major order. Construction validates that every field has exactly one value per surface cell. Provenance is stored once per chunk rather than repeated in every cell.

### Generation Contract

Terrain generation is batch-first. A generator accepts an ordered slice of requests and returns one chunk per request in the same order. Each request explicitly supplies:

- chunk coordinate;
- world seed;
- generation trace identity;
- stable generator implementation/revision fingerprint;
- stable complete-parameter fingerprint;
- ordered causal input trace identities.

The authoritative boundary validates output count, chunk coordinate, order, and provenance before accepting a batch. Generator implementations must be pure functions of explicit request state. System time, hardware entropy, global RNG consumption, pointer identity, locale, and thread scheduling may not affect output.

### Provenance Continuity

The Phase 4 record identifies the generation trace and its causal inputs. Later geological synthesis must extend this graph rather than replace it with prose:

```text
generation parameters and seed
    → geological formation trace
    → terrain chunk trace
    → surface MaterialId
    → extraction trace
    → transport and transformation traces
```

Descriptions shown to developers or users are downstream explanations. They are not authoritative provenance.

### Implementation Boundary

Phase 4 implements the terrain state and generation contracts in `ontopolis-geography`. It does not implement tectonics, geological columns, erosion, hydrology, climate, ecology, extraction, or a terrain synthesis algorithm.

## Determinism

Integer terrain units and stable row-major indexing avoid floating-point ambiguity at this boundary. Identical ordered requests must produce byte-equivalent logical terrain fields and identical provenance. Different batch partitioning or parallel scheduling must not change per-request output.

## Performance

The contract supports batch generation and sequential field access. No throughput or scale claim is accepted until reproducible benchmarks exist. Future benchmarks must cover construction, field iteration, validation, and bytes per surface cell.

## Primitive vs Emergent Review

Elevation, local height variation, and material identity are objective physical state. Biomes, landscape character, settlement suitability, political regions, resource names, and observer material classifications are not Phase 4 primitives.

## Unresolved Questions

- Terrain synthesis and erosion algorithms
- Geological column and formation storage
- Canonical persistence encoding for terrain and provenance
- Benchmark-backed choice of compression and accelerator boundaries

## Decision Log

- **Accepted:** fixed-point millimetres for authoritative elevation and roughness.
- **Accepted:** property-linked `MaterialId` instead of a semantic material enum.
- **Accepted:** dense structure-of-arrays chunk storage and batch-first generation.
- **Accepted:** compact per-chunk trace references for Phase 4 provenance.
