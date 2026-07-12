# Geology

Geology is the foundation of the physical world. It determines terrain, material availability, water flow, and many long-term causal chains.

## Geological Representation

Geology is represented as a three-dimensional structure:

```text
GeologicalColumn:
    surface_layer: SoilLayer
    subsurface_layers: [RockLayer]
    bedrock: RockLayer
    fault_lines: [FaultLine]
    aquifer_properties: AquiferProperties
```

### Rock Layer

```text
RockLayer:
    depth_range: (float, float)
    material: MaterialId
    density: float
    porosity: float
    permeability: float
    formation_age: Time
    formation_process: FormationProcess
```

### Soil Layer

```text
SoilLayer:
    depth: float
    composition: {MaterialId → proportion}
    fertility: float
    drainage: float
    formation_process: SoilFormationProcess
```

## Formation Processes

Geological features have formation histories:

- **Igneous**: volcanic activity, magma cooling
- **Sedimentary**: deposition, compaction, cementation
- **Metamorphic**: heat and pressure transformation
- **Weathering**: breakdown of existing rock
- **Erosion**: transport of material
- **Deposition**: accumulation of sediment

## Material Provenance

Materials must maintain geographic provenance throughout their lifecycle:

```text
g geological formation
    ↓
deposit
    ↓
quarry
    ↓
extraction lot
    ↓
transport batch
    ↓
merchant inventory
    ↓
workshop
    ↓
building component
```

This provenance chain supports causal explanation. A building's stone may be traceable to a specific quarry, which is traceable to a specific geological formation.

## Material Properties

Ground Truth stores physical and mana-relevant properties:

- density
- hardness
- porosity
- permeability
- thermal conductivity
- acoustic properties
- mana coupling characteristics

Agents construct material concepts from observed properties. The engine must not use narrative labels such as `MagicOre` or `SacredStone` as authoritative categories.

## Geology and Other Domains

Geology interacts with:

- **Terrain**: bedrock and erosion determine surface shape
- **Hydrology**: permeability and aquifers determine groundwater flow
- **Climate**: thermal properties affect local temperature
- **Ecology**: soil fertility determines vegetation
- **Economy**: material availability constrains technology
- **Mana**: certain geological formations may have unusual mana properties

## Determinism

Geological generation must be deterministic given:

- world_seed
- tectonic parameters
- geological history parameters

## Performance

Geological data is mostly static after generation. Strategies:

- Generate once, store persistently
- Use sparse representation for deep subsurface
- Share geological columns across similar locations
- Compress inactive regions

## Related Documents

- `geography-philosophy.md` — geographic causality
- `terrain.md` — surface terrain derived from geology
- `hydrology.md` — water flow through geological structures
- `world-generation-provenance.md` — provenance tracking

## RFCs

- `RFC-GEO-001: Minimal Causal Geological World Model`

## TODO Categories

- `GEO` — geology
- `WORLD` — general world systems
