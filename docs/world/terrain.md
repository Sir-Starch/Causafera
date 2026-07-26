# Terrain

Terrain is the physical surface of the world. It determines movement, visibility, drainage, settlement viability, and many other simulation outcomes.

## Terrain Representation

The Phase 4 authoritative terrain boundary is a chart-local 2.5D height field with three causal properties:

```text
TerrainCell:
    elevation: signed integer millimetres
    roughness: unsigned integer millimetres
    surface_material: MaterialId
```

Later geographic work may add derived or generated fields:

```text
DerivedOrFutureTerrainState:
    slope: fixed-point gradient
    aspect: physical direction
    soil_depth: float
    bedrock_depth: float
    vegetation_cover: float
    water_depth: float
```

## Generation Pipeline

Terrain generation follows a causal pipeline:

```text
Tectonic simulation
    ↓
Geological structure
    ↓
Erosion simulation
    ↓
Sediment deposition
    ↓
Hydrological carving
    ↓
Soil formation
    ↓
Vegetation cover
```

Each stage must be deterministic and preserve provenance.

## Terrain Properties

### Elevation
Absolute height above reference level. Elevation determines temperature, pressure, and drainage direction.

### Slope
Local gradient. Slope determines:
- construction difficulty
- erosion rate
- agricultural suitability
- movement speed

### Aspect
Cardinal direction of slope. Aspect determines:
- sun exposure
- snow melt timing
- wind exposure
- vegetation patterns

### Roughness
Local elevation variation. Roughness determines:
- movement difficulty
- visibility
- construction cost
- tactical cover

### Surface Material
The property-defined material identity at the surface. Human categories such as soil, rock, and sand are not authoritative terrain enums. Material properties determine:
- agricultural productivity
- construction properties
- water permeability
- mineral availability

## Terrain and Other Domains

Terrain interacts with:

- **Hydrology**: elevation determines drainage; slope determines flow velocity
- **Climate**: elevation determines temperature; aspect determines sun exposure
- **Ecology**: terrain determines biome boundaries; slope determines vegetation
- **Geology**: bedrock depth and type determine surface material
- **Settlements**: flat, well-drained terrain attracts construction
- **Mana**: implemented. The terrain carrier presents its standing structure to the physical
  pattern stream on every emitting tick, projected onto the mana lattice as one sample per
  plan-view column. A column's magnitude is its mean per-cell structure — relief contrast against
  neighbours, surface-material discontinuity, and roughness — so featureless ground contributes
  nothing and only landform drives the field. See `plans/terrain-carrier-participation.md`

## Level of Detail

Terrain representation varies by resolution:

- **High resolution** (chunk level): full height field, detailed surface properties
- **Medium resolution** (territory level): averaged elevation, dominant slope and aspect
- **Low resolution** (region level): elevation range, typical terrain character

## Determinism

Terrain generation must be fully deterministic given:

- world_seed
- generation parameters
- chunk coordinates

Generator implementations receive these values explicitly through ordered batch requests. Accepted output must preserve request order, chunk identity, and generation provenance. System time, global RNG state, locale, and thread scheduling must not affect output.

## Performance

Terrain data may be large. Strategies:

- Store terrain as compact height fields
- Use GPU for terrain field operations
- Cache frequently accessed chunks
- Compress distant or inactive terrain

## Phase 4 Implementation

`causafera-geography` now defines the terrain generation contract:

- `ElevationMm`, `RoughnessMm`, and `TerrainCell` provide fixed-point physical values without semantic terrain categories.
- `TerrainChunk` stores elevation, `MaterialId`, and roughness in three contiguous vectors using deterministic row-major indexing.
- construction requires exactly `CHUNK_SIZE × CHUNK_SIZE` values in every field.
- `TerrainGenerationProvenance` retains world seed, generation trace, generator and parameter fingerprints, and ordered causal input traces once per chunk.
- `TerrainGenerator` is batch-first, and `generate_validated_batch` rejects output count, order, chunk identity, or provenance mismatches.

No production terrain synthesis algorithm exists yet. Tectonics, erosion, geological layers, hydrology, climate, ecology, movement costs, visibility, persistence, and observer schemas remain outside this phase.

Existing `TerrainChunk` uses a bare chart-local `ChunkCoord`. Global terrain synthesis must migrate to `ChartChunkCoord` under RFC-GEO-002 before spanning chart seams. Elevation follows the local chart normal; it is not a universal Cartesian `z` coordinate.

## Related Documents

- `geography-philosophy.md` — geographic causality
- `spatial-hierarchy.md` — spatial hierarchy
- `coordinates.md` — coordinate systems
- `docs/rfc/RFC-GEO-002.md` — multiscale geometry and chart boundary
- `geology.md` — geological base for terrain
- `hydrology.md` — water drainage on terrain
- `climate.md` — climate effects on terrain

## TODO Categories

- `TERRAIN` — terrain generation and representation
- `WORLD` — general world systems
