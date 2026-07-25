use serde::{Deserialize, Serialize};

/// Size of a chunk in each dimension.
pub const CHUNK_SIZE: u8 = 32;

/// Legacy local Cartesian lattice coordinates within one spatial chart.
///
/// Despite the historical name, this is not a unique embedding of an entire
/// planetary surface. Global geography must pair chart-local coordinates with
/// a [`crate::SpatialChartId`] or use the geometry contracts in `geometry`.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct WorldCoord {
    pub x: i64,
    pub y: i64,
    pub z: i64,
}

impl WorldCoord {
    pub const fn new(x: i64, y: i64, z: i64) -> Self {
        Self { x, y, z }
    }

    /// Convert to chunk coordinates and local coordinates within that chunk.
    pub fn to_chunk_local(self) -> (ChunkCoord, LocalCoord) {
        let chunk_x = if self.x >= 0 {
            (self.x / CHUNK_SIZE as i64) as i32
        } else {
            ((self.x + 1) / CHUNK_SIZE as i64 - 1) as i32
        };
        let chunk_y = if self.y >= 0 {
            (self.y / CHUNK_SIZE as i64) as i32
        } else {
            ((self.y + 1) / CHUNK_SIZE as i64 - 1) as i32
        };
        let chunk_z = if self.z >= 0 {
            (self.z / CHUNK_SIZE as i64) as i32
        } else {
            ((self.z + 1) / CHUNK_SIZE as i64 - 1) as i32
        };

        let local_x = (self.x - chunk_x as i64 * CHUNK_SIZE as i64) as u8;
        let local_y = (self.y - chunk_y as i64 * CHUNK_SIZE as i64) as u8;
        let local_z = (self.z - chunk_z as i64 * CHUNK_SIZE as i64) as u8;

        (
            ChunkCoord::new(chunk_x, chunk_y, chunk_z),
            LocalCoord::new(local_x, local_y, local_z),
        )
    }

    /// Construct from chunk and local coordinates.
    pub const fn from_chunk_local(chunk: ChunkCoord, local: LocalCoord) -> Self {
        Self {
            x: chunk.x as i64 * CHUNK_SIZE as i64 + local.x as i64,
            y: chunk.y as i64 * CHUNK_SIZE as i64 + local.y as i64,
            z: chunk.z as i64 * CHUNK_SIZE as i64 + local.z as i64,
        }
    }
}

impl std::fmt::Debug for WorldCoord {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "WorldCoord({}, {}, {})", self.x, self.y, self.z)
    }
}

impl std::fmt::Display for WorldCoord {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({}, {}, {})", self.x, self.y, self.z)
    }
}

/// Chart-local three-dimensional chunk coordinates.
///
/// Each chunk is a `CHUNK_SIZE`³ cube of world space. Chunk coordinates
/// identify which cube a position falls into. They cannot be compared across
/// global surface charts without an explicit chart transform.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ChunkCoord {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl ChunkCoord {
    pub const fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }

    /// Return the world-space coordinate of this chunk's origin
    /// (minimum-corner, inclusive).
    pub const fn world_origin(self) -> WorldCoord {
        WorldCoord {
            x: self.x as i64 * CHUNK_SIZE as i64,
            y: self.y as i64 * CHUNK_SIZE as i64,
            z: self.z as i64 * CHUNK_SIZE as i64,
        }
    }

    /// Return the Manhattan distance to another chunk.
    pub const fn manhattan_distance(self, other: Self) -> i32 {
        (self.x - other.x).abs() + (self.y - other.y).abs() + (self.z - other.z).abs()
    }
}

impl std::fmt::Debug for ChunkCoord {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ChunkCoord({}, {}, {})", self.x, self.y, self.z)
    }
}

impl std::fmt::Display for ChunkCoord {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}, {}, {}]", self.x, self.y, self.z)
    }
}

/// Local coordinates within a chunk.
///
/// Each component is in the range `[0, CHUNK_SIZE)`.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct LocalCoord {
    pub x: u8,
    pub y: u8,
    pub z: u8,
}

impl LocalCoord {
    pub const fn new(x: u8, y: u8, z: u8) -> Self {
        Self { x, y, z }
    }

    /// Pack into a single flat index for dense array indexing.
    pub const fn flat_index(self) -> u16 {
        let size = CHUNK_SIZE as u16;
        self.z as u16 * size * size + self.y as u16 * size + self.x as u16
    }

    /// Total number of cells in a chunk.
    pub const fn cells_per_chunk() -> u16 {
        let size = CHUNK_SIZE as u16;
        size * size * size
    }
}

pub const fn flat_index(position: LocalCoord, extent: u8) -> Option<usize> {
    if extent == 0 || position.x >= extent || position.y >= extent || position.z >= extent {
        return None;
    }
    let size = extent as usize;
    Some(position.z as usize * size * size + position.y as usize * size + position.x as usize)
}

impl std::fmt::Debug for LocalCoord {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "LocalCoord({}, {}, {})", self.x, self.y, self.z)
    }
}

impl std::fmt::Display for LocalCoord {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({}.{}.{})", self.x, self.y, self.z)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_through_chunk_local() {
        let world = WorldCoord::new(100, -50, 7);
        let (chunk, local) = world.to_chunk_local();
        let back = WorldCoord::from_chunk_local(chunk, local);
        assert_eq!(world, back);
    }

    #[test]
    fn negative_coords_roundtrip() {
        let world = WorldCoord::new(-1, -32, -33);
        let (chunk, local) = world.to_chunk_local();
        let back = WorldCoord::from_chunk_local(chunk, local);
        assert_eq!(world, back);
    }

    #[test]
    fn chunk_origin() {
        let chunk = ChunkCoord::new(2, -1, 0);
        let origin = chunk.world_origin();
        assert_eq!(origin.x, 64);
        assert_eq!(origin.y, -32);
        assert_eq!(origin.z, 0);
    }

    #[test]
    fn local_flat_index_range() {
        let max = LocalCoord::cells_per_chunk();
        assert_eq!(max, 32 * 32 * 32);

        let last = LocalCoord::new(31, 31, 31);
        assert_eq!(last.flat_index(), max - 1);
    }
}
