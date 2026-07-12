# Terrain

Terrain is the physical surface of the world. It determines movement, visibility, drainage, settlement viability, and many other simulation outcomes.

## Terrain Representation

Terrain is represented as a height field with additional surface properties:

```text
TerrainPoint:
    elevation: float
    slope: float
    aspect: direction
    roughness: float
    surface_material: MaterialId
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
The material at the surface (soil, rock, sand, etc.). Surface material determines:
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
- **Mana**: certain terrain configurations may create stable mana patterns

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

## Performance

Terrain data may be large. Strategies:

- Store terrain as compact height fields
- Use GPU for terrain field operations
- Cache frequently accessed chunks
- Compress distant or inactive terrain

## Related Documents

- `geography-philosophy.md` — geographic causality
- `spatial-hierarchy.md` — spatial hierarchy
- `coordinates.md` — coordinate systems
- `geology.md` — geological base for terrain
- `hydrology.md` — water drainage on terrain
- `climate.md` — climate effects on terrain

## TODO Categories

- `TERRAIN` — terrain generation and representation
- `WORLD` — general world systems
