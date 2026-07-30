//! Per-cell projections of the spatial fields the runtime already maintains.
//!
//! The world snapshot reduces a chunk to a handful of aggregates because that is
//! all a register needs. A map needs the lattice itself, so this module projects
//! one chunk of one field at a time: bounded by construction, read-only, and
//! carrying the provenance the cells already hold.
//!
//! Two reductions are deliberately absent. The runtime never flattens the mana
//! volume to plan view, because a column sum and a column maximum answer
//! different questions and choosing between them is a reading of the field
//! rather than a property of it. It never shades a surface either: lighting is
//! presentation (INV-022). Block-mean downsampling is the one reduction here,
//! and it is order-independent and introduces no value the field does not
//! contain.

use causafera_geography::TERRAIN_CELLS_PER_CHUNK;
use causafera_observer_api::{
    FieldRasterKind, FieldRasterRequest, HYDROLOGY_RASTER_VALUES_SCHEMA_V1, ObserverFieldRaster,
};
use causafera_types::{CHUNK_SIZE, ChartChunkCoord, ChunkCoord, SpatialChartId};

use crate::{RuntimeState, TerrainCarrierAdapter};

impl RuntimeState {
    /// Project one chunk of one field, or nothing when the chunk is outside the
    /// active set or the field is not present for it.
    pub(crate) fn observer_field_raster(
        &self,
        request: &FieldRasterRequest,
    ) -> Option<ObserverFieldRaster> {
        request.validate().ok()?;
        let chunk = ChartChunkCoord::new(
            SpatialChartId::new(request.chart_id),
            ChunkCoord::new(request.chunk_x, request.chunk_y, request.chunk_z),
        );
        if !self.active_chunks.contains_key(&chunk) {
            return None;
        }
        match request.field {
            FieldRasterKind::TerrainElevation | FieldRasterKind::TerrainRoughness => {
                self.terrain_raster(request, chunk)
            }
            FieldRasterKind::ManaIntensity => self.mana_raster(request, chunk),
            FieldRasterKind::HydrologySurfaceWater
            | FieldRasterKind::HydrologySoilWater
            | FieldRasterKind::HydrologyGroundwater => self.hydrology_raster(request, chunk),
        }
    }

    /// One chunk of one water bucket, as exact `u64` volumes.
    ///
    /// Projected whole, exactly as the mana volume is: a block mean of volumes
    /// would report a quantity no cell holds, and changing hydrology's detail is
    /// a conservative resolution transition inside the simulation rather than a
    /// reduction an observer may ask for.
    fn hydrology_raster(
        &self,
        request: &FieldRasterRequest,
        chunk: ChartChunkCoord,
    ) -> Option<ObserverFieldRaster> {
        let field = self.hydrology.fields.field(chunk)?;
        let mut unsigned_values = Vec::with_capacity(field.cells().len());
        let mut cell_traces = Vec::with_capacity(field.cells().len());
        for cell in field.cells() {
            let (volume, trace) = match request.field {
                FieldRasterKind::HydrologySurfaceWater => {
                    (cell.surface_water(), cell.surface_last_change())
                }
                FieldRasterKind::HydrologySoilWater => (cell.soil_water(), cell.soil_last_change()),
                _ => (cell.groundwater(), cell.groundwater_last_change()),
            };
            unsigned_values.push(volume.get());
            cell_traces.push(trace.raw());
        }
        Some(ObserverFieldRaster {
            chart_id: chunk.chart.raw(),
            chunk_x: chunk.chunk.x,
            chunk_y: chunk.chunk.y,
            chunk_z: chunk.chunk.z,
            field: request.field,
            detail_level: 0,
            edge: u32::from(CHUNK_SIZE),
            depth: 1,
            // The signed bands stay empty: a water volume is a `u64` and the
            // upper half of its range has no signed image.
            values: Vec::new(),
            auxiliary: Vec::new(),
            cell_traces,
            // The batch that last closed the ledger is the whole field's anchor;
            // per-cell provenance is carried per cell above.
            generation_trace: self.hydrology.fields.conservation_last_change().raw(),
            unsigned_values,
            unsigned_values_schema_version: HYDROLOGY_RASTER_VALUES_SCHEMA_V1,
        })
    }

    fn terrain_raster(
        &self,
        request: &FieldRasterRequest,
        chunk: ChartChunkCoord,
    ) -> Option<ObserverFieldRaster> {
        let terrain = self
            .carrier_adapters
            .get(&chunk)
            .map(TerrainCarrierAdapter::export_snapshot)?;
        if terrain.elevations_mm.len() != TERRAIN_CELLS_PER_CHUNK
            || terrain.roughness_mm.len() != TERRAIN_CELLS_PER_CHUNK
        {
            return None;
        }
        let elevations = terrain
            .elevations_mm
            .iter()
            .map(|value| i64::from(*value))
            .collect::<Vec<_>>();
        let roughness = terrain
            .roughness_mm
            .iter()
            .map(|value| i64::from(*value))
            .collect::<Vec<_>>();
        let source_edge = u32::from(CHUNK_SIZE);
        let (values, auxiliary) = match request.field {
            FieldRasterKind::TerrainElevation => (
                block_mean(&elevations, source_edge, request.detail_level),
                block_mean(&roughness, source_edge, request.detail_level),
            ),
            _ => (
                block_mean(&roughness, source_edge, request.detail_level),
                Vec::new(),
            ),
        };
        Some(ObserverFieldRaster {
            chart_id: chunk.chart.raw(),
            chunk_x: chunk.chunk.x,
            chunk_y: chunk.chunk.y,
            chunk_z: chunk.chunk.z,
            field: request.field,
            detail_level: request.detail_level,
            edge: source_edge >> u32::from(request.detail_level),
            depth: 1,
            values,
            auxiliary,
            // Terrain does not change per tick, so one generation event is the
            // whole provenance of every cell in the chunk.
            cell_traces: Vec::new(),
            generation_trace: terrain.generation_trace.raw(),
            unsigned_values: Vec::new(),
            unsigned_values_schema_version: 0,
        })
    }

    fn mana_raster(
        &self,
        request: &FieldRasterRequest,
        chunk: ChartChunkCoord,
    ) -> Option<ObserverFieldRaster> {
        let field = self.mana.field(chunk)?;
        let extent = u32::from(field.extent());
        // Per-cell provenance is what distinguishes this field: every populated
        // cell records the committed event that last changed it.
        let cell_traces = field
            .last_change()
            .iter()
            .map(|trace| trace.map_or(0, |trace| trace.raw()))
            .collect::<Vec<u64>>();
        let generation_trace = cell_traces.iter().copied().max().unwrap_or(0);
        Some(ObserverFieldRaster {
            chart_id: chunk.chart.raw(),
            chunk_x: chunk.chunk.x,
            chunk_y: chunk.chunk.y,
            chunk_z: chunk.chunk.z,
            field: request.field,
            // The mana volume is projected whole at whatever lattice the
            // runtime is configured for; there is no payload to reduce.
            detail_level: 0,
            edge: extent,
            depth: extent,
            values: field.intensity().to_vec(),
            auxiliary: field.last_change_before().to_vec(),
            cell_traces,
            generation_trace,
            unsigned_values: Vec::new(),
            unsigned_values_schema_version: 0,
        })
    }
}

/// Reduce a square lattice by a power-of-two block mean.
///
/// Level 0 returns the field unchanged. The mean is taken over exact integer
/// division of the block sum, so the reduction is order-independent and every
/// value it produces lies inside the range of the block it came from.
fn block_mean(values: &[i64], edge: u32, detail_level: u8) -> Vec<i64> {
    let factor = 1_usize << u32::from(detail_level);
    let edge = edge as usize;
    if factor == 1 {
        return values.to_vec();
    }
    let reduced = edge / factor;
    let mut out = Vec::with_capacity(reduced * reduced);
    for row in 0..reduced {
        for column in 0..reduced {
            let mut total = 0_i64;
            for inner_row in 0..factor {
                for inner_column in 0..factor {
                    let index = (row * factor + inner_row) * edge + column * factor + inner_column;
                    total += values[index];
                }
            }
            out.push(total / (factor * factor) as i64);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn block_mean_at_level_zero_returns_the_field_unchanged() {
        let values = (0..16).collect::<Vec<i64>>();

        assert_eq!(block_mean(&values, 4, 0), values);
    }

    #[test]
    fn block_mean_halves_the_edge_and_averages_each_block() {
        // Given: a 4 x 4 lattice whose blocks have known means.
        let values = vec![
            0, 2, 10, 12, //
            4, 6, 14, 16, //
            20, 22, 30, 32, //
            24, 26, 34, 36,
        ];

        // When: it is reduced one level.
        let reduced = block_mean(&values, 4, 1);

        // Then: each output value is the mean of its own 2 x 2 block.
        assert_eq!(reduced, vec![3, 13, 23, 33]);
    }

    #[test]
    fn block_mean_never_leaves_the_range_of_its_source() {
        let values = (0..64).map(|value| value * 7 - 100).collect::<Vec<i64>>();
        let minimum = *values.iter().min().unwrap();
        let maximum = *values.iter().max().unwrap();

        for level in [1_u8, 2, 3] {
            for value in block_mean(&values, 8, level) {
                assert!((minimum..=maximum).contains(&value));
            }
        }
    }
}
