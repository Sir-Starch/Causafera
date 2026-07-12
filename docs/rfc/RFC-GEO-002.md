# RFC-GEO-002: Multiscale World Spatial Geometry and Coordinate Model

**Status:** Accepted

## Summary

Ontopolis uses full three-dimensional Euclidean physical geometry inside bounded local frames, a charted two-dimensional geographic surface with fixed-point elevation at world scale, layered depth for coarse subsurface state, and selectively promoted volumetric 3D for causally relevant underground or constructed spaces.

Spatial containment, geometry, and causal resolution are separate systems.

## Motivation

The existing architecture implemented 3D lattice coordinates, surface terrain, containment hierarchy, and causal resolution without explicitly selecting their global geometric relationship. `SpatialHierarchy` answers “contained by”, not global topology, curvature, metric, map boundaries, or coordinate transforms. Treating its nodes as geometry would make physical access, acoustics, body movement, structures, and mana patterns ambiguous.

## Accepted dimensional model

### Global world topology

The default Ontopolis world is a finite closed planetary surface represented by a versioned atlas of overlapping two-dimensional charts. It has no physical map edge. Exact planetary shape, metric coefficients, chart seams, and adjacency transforms belong to an opaque `WorldGeometrySchemaId` registry contract rather than a semantic sphere/plane enum in hot state.

Alternative world topologies require an explicit accepted geometry schema. They may not silently reuse Cartesian wraparound or UI map boundaries.

### Geographic surface

World-scale geography is 2.5D:

```text
surface chart (u, v)
+ signed elevation along the local surface normal
```

`SurfacePositionMm` uses signed integer millimetres and a `SpatialChartId`. Terrain remains a two-dimensional height field per chart-local chunk. Elevation does not turn the entire planet into a dense voxel grid.

### Local physical space

Objects, bodies, buildings, interiors, acoustic paths, collision geometry, and spatial mana patterns use bounded local 3D Euclidean frames:

```text
LocalPointMm(frame, x, y, z)
```

`x/y` follow local surface tangents and `z` follows the local outward normal. A local frame has an explicit maximum valid offset; code must cross into another frame or use a global chart transform outside that range. There is no universal planetary “up” vector.

### Subsurface

Coarse geology is stored as surface-indexed depth intervals and material properties. Negative local `z` is depth below a frame anchor. Caves, mines, aquifers, tunnels, multi-level structures, or other causally relevant regions may promote to explicit local volumetric 3D. Promotion must conserve material/state and provenance; Phase 23 does not implement those domain adapters.

## Coordinate spaces and transforms

The architecture distinguishes:

- global geometry schema and surface atlas;
- chart-qualified surface position;
- chart-qualified chunk address;
- bounded local metric frame;
- object/body/structure-local coordinates;
- rendering coordinates, which are non-authoritative.

Phase 23 adds exact fixed-point tangent-frame translation between `SurfacePositionMm` and `LocalPointMm` within one bounded chart frame. Cross-chart and curvature-aware transforms require the registered world-geometry implementation. Chart seams are explicit; no implicit modulo, clamping, teleportation, or comparison of bare chunk coordinates across charts is allowed.

The historical `WorldCoord` and bare `ChunkCoord` types are reclassified as local Cartesian lattice addresses within one chart. New global systems must qualify chunks with `ChartChunkCoord`. Migrating existing terrain, mana, resolution, and hierarchy identifiers to chart-qualified storage is deferred and must preserve serialized identity when persistence exists.

## Precision and metric

Authoritative geometric contracts use signed fixed-point integer millimetres at the local/surface boundary. Domain-specific dense grids may use coarser cells, but their cell-to-metric scale must be explicit in the registered schema. Floating-point rendering or broad-phase acceleration may be derived, never canonical unless a later RFC specifies cross-platform rounding.

Local distance and orientation are Euclidean within a frame. Long-distance geodesics, horizon, curvature, chart transitions, orbital properties, and climate latitude use the global geometry schema rather than `WorldCoord` subtraction.

## Geometry versus containment

`SpatialHierarchy` remains an immutable containment index:

```text
Place A contained by Place B
```

Geometry separately describes extent, shape, pose, connectivity, and metric relation. Containment does not imply a bounding volume, and geometric overlap does not imply ownership, jurisdiction, or hierarchical parentage. A structure/interior geometry adapter must explicitly link `PlaceId` to geometric state.

## Geometry versus causal resolution

Causal resolution decides how much detail a region needs; it does not alter topology or physical distance. Low-resolution surface state may be represented as fields or aggregates. Promotion may instantiate local 3D geometry, while demotion must conserve causal quantities and provenance. A distant region can be high-detail and a nearby region aggregated.

## Mana

Mana occupies physical 3D positions in local frames. Its internal state may later contain multiple numeric components such as phase or spectral response; those are field-state dimensions, not extra spatial dimensions. Cross-chart mana propagation and curvature-aware stencils remain future work.

The Phase 24 executable runtime uses one bounded local Cartesian chart and a cubic 3D mana field. It makes no claim that this cube is the global world geometry.

## Determinism and performance

Chart IDs, frame IDs, fixed-point positions, explicit bounds, and canonical atlas ordering are deterministic. No system time, locale, pointer identity, hash iteration, or hidden floating transform participates. Multiscale geography avoids a planet-wide millimetre voxel volume; explicit 3D is allocated only where domain state and causal relevance require it.

No planetary-scale performance claim is made. Cross-chart lookup, geodesics, promotion/demotion, and volumetric storage require benchmarks.

## Decisions

- **Accepted:** full local 3D Euclidean physical geometry.
- **Accepted:** finite closed charted planetary surface as the default global topology.
- **Accepted:** 2.5D surface geography, layered coarse subsurface, selective volumetric 3D.
- **Accepted:** local `z` is the chart-normal vertical direction; no universal global up.
- **Accepted:** fixed-point millimetres at coordinate-space boundaries.
- **Accepted:** geometry, containment, jurisdiction, and causal resolution are orthogonal.
- **Accepted:** opaque registered global geometry schema rather than a convenience topology enum.
- **Deferred:** concrete planetary metric/shape, atlas generation, cross-chart transforms, geodesics, horizon, volumetric domain promotion/demotion, and migration of all existing bare chunks.
