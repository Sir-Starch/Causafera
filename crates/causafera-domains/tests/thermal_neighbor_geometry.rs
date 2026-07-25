use std::collections::BTreeSet;

use causafera_domains::{ThermalActiveRegion, ThermalCellKey, ThermalField, ThermalFieldSet};
use causafera_types::{ChartChunkCoord, ChunkCoord, SpatialChartId, TraceId};

fn chunk(x: i32, y: i32, z: i32) -> ChartChunkCoord {
    ChartChunkCoord::new(SpatialChartId::new(1), ChunkCoord::new(x, y, z))
}

#[test]
fn boundary_neighbor_keys_returns_only_inactive_faces() {
    // Given: one active 1x1x1 field and one active neighbor on its positive X face.
    let source = chunk(0, 0, 0);
    let active_neighbor = chunk(1, 0, 0);
    let fields = ThermalFieldSet::new(
        vec![
            ThermalField::new(source, 1, TraceId::new(1)).expect("source field must be valid"),
            ThermalField::new(active_neighbor, 1, TraceId::new(1))
                .expect("neighbor field must be valid"),
        ],
        TraceId::new(2),
    )
    .expect("field set must be valid");
    let active_region = ThermalActiveRegion::new(
        BTreeSet::from([source, active_neighbor]),
        BTreeSet::from([source, active_neighbor]),
    )
    .expect("active region must be valid");

    // When: the source cell asks the domain geometry for boundary neighbors.
    let boundary = fields
        .boundary_neighbor_keys(&active_region, ThermalCellKey::new(source, 0))
        .expect("boundary geometry must resolve");

    // Then: only the five faces outside the active region are returned.
    assert_eq!(boundary.len(), 5);
    assert!(!boundary.contains(&ThermalCellKey::new(active_neighbor, 0)));
    assert!(boundary.contains(&ThermalCellKey::new(chunk(-1, 0, 0), 0)));
    assert!(boundary.contains(&ThermalCellKey::new(chunk(0, -1, 0), 0)));
    assert!(boundary.contains(&ThermalCellKey::new(chunk(0, 1, 0), 0)));
    assert!(boundary.contains(&ThermalCellKey::new(chunk(0, 0, -1), 0)));
    assert!(boundary.contains(&ThermalCellKey::new(chunk(0, 0, 1), 0)));
}
