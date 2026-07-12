# Coordinates

Ontopolis uses a unified coordinate system that supports multiple spatial scales and reference frames.

## Coordinate System

### Global Coordinates

The world uses a three-dimensional Cartesian coordinate system:

- **Origin**: Arbitrary but deterministic point (e.g., planetary center or reference landmass corner)
- **X axis**: Eastward
- **Y axis**: Northward
- **Z axis**: Upward (elevation)

All global coordinates are deterministic given the world seed.

### Chunk Coordinates

Spatial chunks use integer grid coordinates:

```text
chunk_x = floor(world_x / chunk_size)
chunk_y = floor(world_y / chunk_size)
```

Chunk coordinates support efficient spatial indexing and neighbor lookup.

### Local Coordinates

Within a chunk, local coordinates range from [0, chunk_size) in X and Y. Local coordinates support fine-grained positioning without large integer values.

### Elevation

Elevation (Z) is stored as a continuous value relative to a local reference surface (e.g., mean sea level or local terrain baseline). Elevation supports:

- terrain height
- underground depth
- structure height
- water table depth

## Coordinate Types

```text
WorldCoord    — global 3D position
ChunkCoord    — integer chunk grid position
LocalCoord    — position within a chunk
ParcelCoord   — position within a parcel boundary
StructureCoord — position within a structure
InteriorCoord  — position within an interior space
```

## Conversion

Coordinate conversion must be deterministic and lossless where possible:

```text
WorldCoord → ChunkCoord + LocalCoord
WorldCoord → ParcelCoord (via parcel boundary lookup)
WorldCoord → StructureCoord (via structure occupancy)
```

## Spatial Indexing

The coordinate system supports spatial indexing structures:

- Chunk hash maps for O(1) chunk lookup
- Spatial trees for range queries
- Neighbor iteration for local operations

## Determinism

Coordinate operations must be deterministic:

- Conversion between coordinate types must yield identical results across platforms
- Floating-point operations must use defined rounding behavior
- Spatial hashing must use deterministic hash functions

## Performance

Coordinate representation must be compact:

- Chunk coordinates: 32-bit integers
- Local coordinates: 16-bit or 32-bit fixed point
- Elevation: 32-bit floating point

Hot paths should avoid coordinate conversion where possible.

## Related Documents

- `spatial-hierarchy.md` — spatial unit hierarchy
- `terrain.md` — terrain elevation representation
- `world-generation-provenance.md` — coordinate provenance

## TODO Categories

- `COORD` — coordinate systems
- `WORLD` — general world systems
