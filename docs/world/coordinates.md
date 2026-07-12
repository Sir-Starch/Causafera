# Coordinates

Ontopolis uses multiple explicit coordinate spaces. There is no single infinite Cartesian coordinate system that losslessly embeds the entire planetary world.

## Global Geometry

The default world is a finite closed planetary surface represented by a versioned atlas of overlapping two-dimensional charts. `WorldGeometrySchemaId` identifies the registered topology, metric, curvature, chart seams, and transforms. `SpatialChartId` identifies one chart.

Exact planetary shape and cross-chart transforms remain future implementation. A chart edge is not a world edge.

## Geographic Surface

World-scale geography is a fixed-point 2.5D representation:

```text
SurfacePositionMm {
    chart,
    u_mm,
    v_mm,
    elevation_mm,
}
```

`u/v` are chart coordinates; elevation follows the chart's local outward normal. Terrain is a chart-local two-dimensional height field. `ChartChunkCoord` qualifies a chunk so bare chunk coordinates are never compared across charts.

## Local Physical Space

Local physics is fully three-dimensional and Euclidean:

```text
LocalPointMm {
    frame,
    x_mm,
    y_mm,
    z_mm,
}
```

Within `LocalMetricFrame`, `x/y` follow surface tangents and `z` follows the local normal. Positive `z` is locally upward; negative `z` may describe subsurface depth. A frame is bounded so planetary curvature cannot be ignored over arbitrary distance.

Phase 23 implements exact integer translation between a local point and its anchoring surface chart within that bound. Cross-chart and curvature-aware transforms require the geometry-schema implementation.

## Legacy Lattice Types

`WorldCoord`, `ChunkCoord`, and `LocalCoord` predate the geometry RFC. They are now explicitly chart-local Cartesian lattice types:

```text
WorldCoord ↔ ChunkCoord + LocalCoord
```

The conversion is deterministic and lossless within one chart lattice. Despite its historical name, `WorldCoord` is not a global planetary embedding. New global geographic state must carry `SpatialChartId` or `ChartChunkCoord`.

## Structures, Interiors, and Bodies

Parcel, structure, interior, body, and object coordinates use their own local frames and explicit transforms. Containment alone does not define shape, pose, or metric geometry. Rendering coordinates are derived observer state and never authoritative.

## Precision and Determinism

Surface/local metric boundaries use signed integer millimetres. Dense domain grids may use coarser cells only with an explicit metric scale. No hidden floating-point rounding, modulo wrapping, clamping, locale, or rendering transform may affect authoritative state.

## Resolution

Causal resolution may promote a surface region, underground feature, structure, or interior to explicit local volumetric 3D. Demotion must preserve conserved state and provenance. Resolution changes representation detail, not topology or physical distance.

## Related Documents

- `docs/rfc/RFC-GEO-002.md` — accepted spatial geometry model
- `spatial-hierarchy.md` — containment, not geometry
- `terrain.md` — 2.5D surface state
- `geology.md` — layered subsurface state
