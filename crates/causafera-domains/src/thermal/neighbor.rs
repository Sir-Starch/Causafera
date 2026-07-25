use causafera_types::{ChartChunkCoord, LocalCoord, flat_index};

use super::{ThermalActiveRegion, ThermalCellKey, ThermalError, ThermalFieldSet};

const FACE_DIRECTIONS: [(i8, i8, i8); 6] = [
    (-1, 0, 0),
    (1, 0, 0),
    (0, -1, 0),
    (0, 1, 0),
    (0, 0, -1),
    (0, 0, 1),
];

pub(crate) fn neighbor_keys(
    fields: &ThermalFieldSet,
    active_region: &ThermalActiveRegion,
    key: ThermalCellKey,
) -> Result<(Vec<ThermalCellKey>, Vec<ThermalCellKey>), ThermalError> {
    let field = fields
        .field(key.chunk)
        .ok_or(ThermalError::PositionOutsideField)?;
    let position = position_for(key.cell_index, field.extent())?;
    let mut neighbors = Vec::with_capacity(FACE_DIRECTIONS.len());
    let mut boundary_neighbors = Vec::new();
    for direction in FACE_DIRECTIONS {
        if let Some(neighbor) = neighbor_for(position, field.chunk(), field.extent(), direction)? {
            let index =
                flat_index(neighbor.1, field.extent()).ok_or(ThermalError::PositionOutsideField)?;
            let cell_index =
                u16::try_from(index).map_err(|_| ThermalError::PositionOutsideField)?;
            let computed_neighbor_key = ThermalCellKey::new(neighbor.0, cell_index);

            if !active_region.active_chunks().contains(&neighbor.0) {
                boundary_neighbors.push(computed_neighbor_key);
                continue;
            }
            let neighbor_field = fields
                .field(neighbor.0)
                .ok_or(ThermalError::ActiveRegionIncomplete(neighbor.0))?;
            if neighbor_field.extent() != field.extent() {
                return Err(ThermalError::IncompatibleFieldExtent);
            }
            neighbors.push(computed_neighbor_key);
        }
    }
    Ok((neighbors, boundary_neighbors))
}

fn position_for(index: u16, extent: u8) -> Result<LocalCoord, ThermalError> {
    let side = usize::from(extent);
    let index = usize::from(index);
    if index >= side.pow(3) {
        return Err(ThermalError::PositionOutsideField);
    }
    let plane = side * side;
    let z = u8::try_from(index / plane).map_err(|_| ThermalError::PositionOutsideField)?;
    let y = u8::try_from((index % plane) / side).map_err(|_| ThermalError::PositionOutsideField)?;
    let x = u8::try_from(index % side).map_err(|_| ThermalError::PositionOutsideField)?;
    Ok(LocalCoord::new(x, y, z))
}

fn neighbor_for(
    position: LocalCoord,
    chunk: ChartChunkCoord,
    extent: u8,
    direction: (i8, i8, i8),
) -> Result<Option<(ChartChunkCoord, LocalCoord)>, ThermalError> {
    let (dx, dy, dz) = direction;
    let coordinate = [position.x, position.y, position.z];
    let direction = [dx, dy, dz];
    let mut neighbor = coordinate;
    let mut chunk_delta = [0_i32; 3];
    for axis in 0..3 {
        match direction[axis] {
            -1 if coordinate[axis] == 0 => {
                neighbor[axis] = extent - 1;
                chunk_delta[axis] = -1;
            }
            -1 => neighbor[axis] -= 1,
            0 => {}
            1 if coordinate[axis] + 1 == extent => {
                neighbor[axis] = 0;
                chunk_delta[axis] = 1;
            }
            1 => neighbor[axis] += 1,
            _ => return Err(ThermalError::PositionOutsideField),
        }
    }
    Ok(Some((
        chunk.same_chart_neighbor(chunk_delta[0], chunk_delta[1], chunk_delta[2]),
        LocalCoord::new(neighbor[0], neighbor[1], neighbor[2]),
    )))
}
