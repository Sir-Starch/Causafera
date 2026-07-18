# Geography Philosophy

Geography in Causafera is not a decorative backdrop. It is causal state.

## Principle

Every geographic feature must be capable of influencing simulation outcomes through physical mechanisms. Mountains do not merely look impressive. They alter trade routes, isolate populations, create distinct microclimates, and determine where water flows. The simulation must treat terrain, geology, hydrology, climate, and ecology as systems that continuously modify and are modified by other domains.

## Geography as Active Cause

Geographic state participates in causal chains:

- **Geology** determines material availability, which constrains construction, tools, and technology
- **Hydrology** determines settlement viability, agricultural potential, and disease ecology
- **Climate** determines growing seasons, building practices, and migration pressure
- **Terrain** determines transport cost, military strategy, and communication patterns
- **Ecology** determines food sources, disease reservoirs, and available materials

These effects must emerge from lower-level properties rather than being applied as narrative modifiers.

## Geographic Provenance

All geographic features must carry generation provenance. A river valley should be traceable to:

- underlying geological structure
- precipitation patterns
- erosion processes
- historical hydrological state

This provenance supports causal explanation. When a user asks why a settlement exists in a particular location, the explanation system must be able to trace the answer through geographic causality.

## Spatial Hierarchy

Geographic organization follows a strict spatial hierarchy:

```text
World
└── Landmass
    └── Geographic Basin
        └── Landscape Region
            └── Local Territory
                └── Spatial Chunk
                    └── Parcel / Site
                        └── Structure
                            └── Interior Space
```

Political regions are separate from geographic regions. A kingdom may span multiple landscape regions. A single territory may contain multiple political jurisdictions.

## Spatial Geometry

RFC-GEO-002 separates the containment hierarchy above from physical geometry. The default world is a finite closed charted planetary surface. World-scale geography is 2.5D surface state with elevation; local causally relevant regions use bounded full 3D Euclidean frames; coarse subsurface uses layered depth and may promote to volumetric 3D.

Bare Cartesian chunk coordinates are chart-local. Long-distance distance, adjacency, curvature, and chart transitions must come from the registered world-geometry schema, not integer subtraction or UI map edges.

## Deterministic Requirements

Geographic generation must be deterministic given a world seed. Identical seeds must produce identical terrain, geology, and hydrology. This supports:

- reproducible experiments
- causal analysis
- regression testing
- historical replay

## Performance Considerations

Geographic data may be large. The architecture must support:

- spatial chunking for partial loading
- level-of-detail representation
- GPU acceleration for field operations
- sparse representation for distant or inactive regions

## Related Documents

- `spatial-hierarchy.md` — detailed hierarchy specification
- `coordinates.md` — coordinate systems and spatial addressing
- `terrain.md` — terrain generation and representation
- `geology.md` — geological formations and material provenance
- `hydrology.md` — water systems and drainage
- `climate.md` — weather and climate patterns
- `ecology.md` — ecosystems and biomes
- `world-generation-provenance.md` — provenance tracking for generated features

## RFCs

- `RFC-GEO-001: Minimal Causal Geological World Model`
- `RFC-GEO-002: Multiscale World Spatial Geometry and Coordinate Model`
- `RFC-HYDRO-001: Multi-Resolution Hydrology`

## TODO Categories

- `WORLD` — general world systems
- `COORD` — coordinate systems
- `TERRAIN` — terrain generation
- `GEO` — geology
- `HYDRO` — hydrology
- `CLIMATE` — climate
- `ECO` — ecology
