# Hydrology

Hydrology is the movement and distribution of water. It determines settlement viability, agriculture, disease ecology, transport, and many other simulation outcomes.

## Hydrological Representation

Water is represented at multiple scales:

```text
HydrologicalState:
    surface_water: [WaterBody]
    groundwater: GroundwaterField
    precipitation: PrecipitationField
    evaporation: EvaporationField
    drainage_network: DrainageNetwork
    water_table: HeightField
```

### Water Body

```text
WaterBody:
    type: WaterBodyType
    boundary: Polygon
    volume: float
    flow_rate: float
    temperature: float
    sediment_load: float
    source: [WaterSource]
    sink: [WaterSink]
```

Water body types:
- River
- Stream
- Lake
- Pond
- Wetland
- Reservoir
- Canal

### Groundwater Field

```text
GroundwaterField:
    aquifer_depth: HeightField
    water_table_depth: HeightField
    flow_direction: VectorField
    flow_velocity: ScalarField
    recharge_rate: ScalarField
```

### Drainage Network

```text
DrainageNetwork:
    nodes: [DrainageNode]
    edges: [DrainageEdge]
    catchment_areas: [Catchment]
```

## Hydrological Cycle

The simulation models:

- **Precipitation**: rain, snow, derived from climate
- **Infiltration**: water entering soil
- **Runoff**: water flowing over surface
- **Evaporation**: water returning to atmosphere
- **Transpiration**: water released by vegetation
- **Groundwater flow**: subsurface water movement
- **Baseflow**: groundwater contribution to streams

## Hydrology and Other Domains

Hydrology interacts with:

- **Terrain**: elevation determines drainage direction
- **Geology**: permeability determines infiltration and groundwater flow
- **Climate**: precipitation and evaporation rates
- **Ecology**: water availability determines biome and species distribution
- **Settlements**: reliable water sources attract habitation
- **Health**: standing water may harbor disease vectors
- **Agriculture**: irrigation depends on water availability
- **Mana**: water flow patterns may create mana field structures

## Multi-Resolution Representation

Hydrology operates at multiple resolutions:

- **Chunk level**: detailed stream networks, individual water bodies
- **Territory level**: catchment areas, aquifer boundaries
- **Region level**: major river systems, climate-driven water balance

The Causal Resolution Field determines which resolution is active.

## Determinism

Hydrological simulation must be deterministic given:

- world_seed
- terrain state
- climate state
- geological state

## Performance

Hydrological computation may be expensive. Strategies:

- Batch hydrological updates
- GPU acceleration for field operations
- Sparse update for inactive regions
- Cached drainage networks for stable terrain

## Related Documents

- `geography-philosophy.md` — geographic causality
- `terrain.md` — terrain elevation for drainage
- `geology.md` — permeability and aquifers
- `climate.md` — precipitation and evaporation
- `world-generation-provenance.md` — provenance tracking

## RFCs

- `RFC-HYDRO-001: Multi-Resolution Hydrology`

## TODO Categories

- `HYDRO` — hydrology
- `WORLD` — general world systems
