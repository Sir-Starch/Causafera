use ontopolis_types::{CHUNK_SIZE, ChunkCoord, LocalCoord, WorldCoord};

#[test]
fn world_to_chunk_local_roundtrip() {
    let world = WorldCoord::new(100, -50, 7);
    let (chunk, local) = world.to_chunk_local();
    let back = WorldCoord::from_chunk_local(chunk, local);
    assert_eq!(world, back);
}

#[test]
fn negative_world_coords_roundtrip() {
    let world = WorldCoord::new(-1, -32, -33);
    let (chunk, local) = world.to_chunk_local();
    let back = WorldCoord::from_chunk_local(chunk, local);
    assert_eq!(world, back);
}

#[test]
fn chunk_world_origin() {
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

#[test]
fn manhattan_distance() {
    let a = ChunkCoord::new(0, 0, 0);
    let b = ChunkCoord::new(1, 2, 3);
    assert_eq!(a.manhattan_distance(b), 6);
}

#[test]
fn chunk_size_constant() {
    assert_eq!(CHUNK_SIZE, 32);
}

#[test]
fn origin_roundtrip() {
    let origin = WorldCoord::new(0, 0, 0);
    let (chunk, local) = origin.to_chunk_local();
    assert_eq!(chunk, ChunkCoord::new(0, 0, 0));
    assert_eq!(local, LocalCoord::new(0, 0, 0));
    assert_eq!(WorldCoord::from_chunk_local(chunk, local), origin);
}

#[test]
fn large_coords_roundtrip() {
    let world = WorldCoord::new(1_000_000, -500_000, 250_000);
    let (chunk, local) = world.to_chunk_local();
    let back = WorldCoord::from_chunk_local(chunk, local);
    assert_eq!(world, back);
}
