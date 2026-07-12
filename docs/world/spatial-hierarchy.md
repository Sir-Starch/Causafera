# Spatial Hierarchy

Ontopolis organizes places into a nested containment hierarchy. This hierarchy is not a geometric embedding, metric, boundary model, or ownership system.

## Geographic Hierarchy

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

### World
The entire simulated world container. The default physical topology is a finite closed charted planetary surface under RFC-GEO-002; its geometry is stored separately from containment.

### Landmass
A continent, island, or other major division of exposed crust. Landmasses have distinct geological histories and may be separated by ocean.

### Geographic Basin
A large-scale drainage or geological basin. Basins collect water, sediment, and ecological patterns. They often correspond to broad economic and cultural zones.

### Landscape Region
A coherent area with similar terrain, climate, and ecology. Examples include mountain ranges, coastal plains, or forested highlands. Regions are the primary unit for climate and ecological simulation.

### Local Territory
A specific area within a landscape region, typically tens to hundreds of square kilometers. Territories have consistent local geography and may correspond to a settlement's hinterland.

### Spatial Chunk
The primary simulation unit for active geographic computation. Chunks are fixed-size regions (candidate: 1 km²) that contain terrain, hydrology, ecology, and mana field state. The Causal Resolution Field determines which chunks receive detailed simulation.

### Parcel / Site
A specific piece of land with defined boundaries. Parcels may be owned, cultivated, built upon, or designated for specific uses. Sites are parcels with attached structures or significant human modification.

### Structure
A building, ruin, or other constructed feature. Structures have physical extent, material composition, and interior organization.

### Interior Space
Rooms, chambers, or other subdivisions within a structure. Interior spaces have their own microclimate, lighting, and spatial properties.

## Political vs Geographic Separation

Political regions are separate from geographic regions:

- A kingdom may span multiple landscape regions
- A single territory may contain multiple political jurisdictions
- Political borders may follow geographic features or ignore them
- Geographic features continue to function regardless of political claims

## Resolution and Aggregation

Phase 18 implements a separate chunk-keyed Causal Resolution Field. Trace-backed numeric carrier signals, rather than distance alone, determine fixed-point relevance and a bounded detail ordinal. A causally connected distant chunk may therefore receive more detail than a nearby isolated chunk.

The field establishes the decision contract but does not yet aggregate domain state. Future consumers may use levels to select behavior such as:

- Active chunks near the observation focus receive full simulation
- Distant chunks may be aggregated to landscape region averages
- Historical chunks may be stored at lower resolution
- Inactive interior spaces may be represented as simple containers

## Coordinate Addressing

Containment identity does not itself provide a coordinate address. A domain adapter may associate a `PlaceId` with chart-qualified surface extent, a bounded local 3D frame, connectivity, or other geometry. See `coordinates.md` and RFC-GEO-002.

Two contained places need not be geometrically adjacent. Two geometrically overlapping extents need not share a parent, owner, or jurisdiction. Geometry and containment must never be inferred from one another without an explicit validated relation.

## Phase 3 Implementation

The authoritative containment skeleton is implemented in `ontopolis-world` as an immutable `SpatialHierarchy`:

- `PlaceId` values are dense, stable indexes assigned in deterministic insertion order.
- `SpatialHierarchyBuilder` accepts only direct transitions in the documented hierarchy.
- nodes and child references are stored in contiguous arrays; direct parents and child slices require no graph search.
- hierarchy construction retains the explicit world seed as minimal generation provenance.
- chunk nodes convert to and from `ChunkId` only after their structural level is validated.

The hierarchy does not itself generate geography. Phase 4 terrain chunks can attach deterministic surface fields and generation provenance to chunk coordinates through the separate `ontopolis-geography` contract. Phase 18 causal resolution is a separate traced overlay and does not alter containment. Geological state, hydrology, climate, ecology, parcels, structures, interiors, and domain-specific aggregation remain future work. Political claims remain separate overlays.

## Ownership and Jurisdiction

Spatial units may have multiple overlapping claims:

- Geographic ownership (who controls the land)
- Political jurisdiction (which authority governs)
- Economic use (who exploits resources)
- Social claim (which community considers it theirs)

These claims may conflict. The simulation must represent conflict, not resolve it automatically.

## Related Documents

- `geography-philosophy.md` — geographic causality principles
- `coordinates.md` — coordinate systems
- `docs/rfc/RFC-GEO-002.md` — global topology and multiscale geometry
- `terrain.md` — terrain representation
- `world-generation-provenance.md` — generation provenance

## RFCs

- `RFC-RES-001: Causal Resolution and Aggregation`

## TODO Categories

- `WORLD` — general world systems
- `COORD` — coordinate systems
