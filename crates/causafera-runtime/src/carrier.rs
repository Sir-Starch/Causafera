use causafera_domains::{CarrierAdapterSchemaId, PhysicalCarrierAdapter, PhysicalPatternSample};
use causafera_geography::{
    ElevationMm, RoughnessMm, TERRAIN_CELLS_PER_CHUNK, TerrainCell, TerrainChunk,
    TerrainGenerationProvenance, TerrainGeneratorFingerprint, TerrainParameterFingerprint,
};
use causafera_types::{
    CHUNK_SIZE, ChartChunkCoord, LocalCoord, MaterialId, PhysicalPatternId, SimulationTime, TraceId,
};

use crate::{MaterialSurface, MaterialSurfaceId};

pub const TERRAIN_CARRIER_SCHEMA: CarrierAdapterSchemaId = CarrierAdapterSchemaId::new(1);
pub const MATERIAL_SURFACE_CARRIER_SCHEMA: CarrierAdapterSchemaId = CarrierAdapterSchemaId::new(2);

const TERRAIN_PATTERN_DOMAIN: u64 = 0x5445_5252_4149_4E50;
const MATERIAL_SURFACE_PATTERN_DOMAIN: u64 = 0x4D41_5453_5552_4643;
const TERRAIN_GENERATOR: TerrainGeneratorFingerprint =
    TerrainGeneratorFingerprint::new(0x2405_0001);
const TERRAIN_PARAMETERS: TerrainParameterFingerprint =
    TerrainParameterFingerprint::new(0x2405_0001);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TerrainCarrierAdapter {
    chunk: ChartChunkCoord,
    terrain: TerrainChunk,
    field_extent: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MaterialSurfaceCarrierAdapter {
    field_extent: u8,
}

impl MaterialSurfaceCarrierAdapter {
    pub const fn new(field_extent: u8) -> Self {
        Self { field_extent }
    }

    pub fn emit_samples(
        &self,
        id: MaterialSurfaceId,
        surface: MaterialSurface,
        time: SimulationTime,
    ) -> Vec<PhysicalPatternSample> {
        let source_ordinal = u32::from(id.cell_index);
        let magnitude = surface.condition.unsigned_abs().min(u64::from(u32::MAX)) as u32;
        vec![PhysicalPatternSample {
            chunk: id.chunk,
            pattern: PhysicalPatternId::new(mix64(
                MATERIAL_SURFACE_PATTERN_DOMAIN
                    ^ material_surface_chart_hash(id.chunk)
                    ^ u64::from(id.cell_index).rotate_left(17),
            )),
            position: field_position(usize::from(id.cell_index), self.field_extent),
            observed_at: time,
            magnitude: magnitude
                .saturating_add(surface.contact_count.min(u64::from(u32::MAX)) as u32),
            source_ordinal,
            cause: surface.last_transition,
        }]
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TerrainCarrierSnapshot {
    pub chunk: ChartChunkCoord,
    pub field_extent: u8,
    pub world_seed: u64,
    pub generation_trace: TraceId,
    pub generator: u64,
    pub parameters: u64,
    pub causal_inputs: Vec<TraceId>,
    pub elevations_mm: Vec<i32>,
    pub surface_materials: Vec<MaterialId>,
    pub roughness_mm: Vec<u32>,
}

impl TerrainCarrierAdapter {
    pub const fn new(chunk: ChartChunkCoord, terrain: TerrainChunk, field_extent: u8) -> Self {
        Self {
            chunk,
            terrain,
            field_extent,
        }
    }

    pub const fn chunk(&self) -> ChartChunkCoord {
        self.chunk
    }

    pub const fn terrain(&self) -> &TerrainChunk {
        &self.terrain
    }

    pub fn source_cell(&self, source_ordinal: u32) -> Option<TerrainCell> {
        let index = usize::try_from(source_ordinal).ok()?;
        let x = u8::try_from(index % usize::from(CHUNK_SIZE)).ok()?;
        let y = u8::try_from(index / usize::from(CHUNK_SIZE)).ok()?;
        self.terrain.cell(x, y)
    }

    pub fn export_snapshot(&self) -> TerrainCarrierSnapshot {
        let provenance = self.terrain.provenance();
        TerrainCarrierSnapshot {
            chunk: self.chunk,
            field_extent: self.field_extent,
            world_seed: provenance.world_seed(),
            generation_trace: provenance.generation_trace(),
            generator: provenance.generator().raw(),
            parameters: provenance.parameters().raw(),
            causal_inputs: provenance.causal_inputs().to_vec(),
            elevations_mm: self
                .terrain
                .elevations()
                .iter()
                .map(|value| value.millimetres())
                .collect(),
            surface_materials: self.terrain.surface_materials().to_vec(),
            roughness_mm: self
                .terrain
                .roughness()
                .iter()
                .map(|value| value.millimetres())
                .collect(),
        }
    }

    pub fn import_snapshot(
        snapshot: TerrainCarrierSnapshot,
    ) -> Result<Self, causafera_geography::TerrainChunkError> {
        let provenance = TerrainGenerationProvenance::new(
            snapshot.world_seed,
            snapshot.generation_trace,
            TerrainGeneratorFingerprint::new(snapshot.generator),
            TerrainParameterFingerprint::new(snapshot.parameters),
            snapshot.causal_inputs,
        );
        let terrain = TerrainChunk::from_fields(
            snapshot.chunk.chunk,
            provenance,
            snapshot
                .elevations_mm
                .into_iter()
                .map(ElevationMm::new)
                .collect(),
            snapshot.surface_materials,
            snapshot
                .roughness_mm
                .into_iter()
                .map(RoughnessMm::new)
                .collect(),
        )?;
        Ok(Self::new(snapshot.chunk, terrain, snapshot.field_extent))
    }

    fn sample_at(
        &self,
        time: SimulationTime,
        cause: TraceId,
        index: usize,
    ) -> PhysicalPatternSample {
        let cell = terrain_cell_at(&self.terrain, index);
        PhysicalPatternSample {
            chunk: self.chunk,
            pattern: terrain_pattern(cell),
            position: field_position(index, self.field_extent),
            observed_at: time,
            magnitude: terrain_magnitude(&self.terrain, index),
            source_ordinal: index as u32,
            cause,
        }
    }
}

impl PhysicalCarrierAdapter for TerrainCarrierAdapter {
    fn schema(&self) -> CarrierAdapterSchemaId {
        TERRAIN_CARRIER_SCHEMA
    }

    fn emit_samples(&self, time: SimulationTime, cause: TraceId) -> Vec<PhysicalPatternSample> {
        (0..TERRAIN_CELLS_PER_CHUNK)
            .map(|index| self.sample_at(time, cause, index))
            .collect()
    }
}

pub fn deterministic_terrain_chunk(
    seed: u64,
    chunk: ChartChunkCoord,
    generation_trace: TraceId,
) -> TerrainChunk {
    let provenance = TerrainGenerationProvenance::new(
        seed,
        generation_trace,
        TERRAIN_GENERATOR,
        TERRAIN_PARAMETERS,
        vec![generation_trace],
    );
    TerrainChunk::from_cells(chunk.chunk, provenance, terrain_cells(seed))
        .expect("deterministic terrain fixture has one complete chunk")
}

pub fn terrain_cells(seed: u64) -> Vec<TerrainCell> {
    (0..TERRAIN_CELLS_PER_CHUNK)
        .map(|index| {
            let x = index % usize::from(CHUNK_SIZE);
            let y = index / usize::from(CHUNK_SIZE);
            let base = mix64(seed ^ (index as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15));
            let ridge = ((x as i32 - y as i32) * 17) + i32::from((base & 0x3F) as u8);
            let material_band = ((base >> 17) ^ (x as u64 * 3) ^ (y as u64 * 5)) & 0xF;
            TerrainCell::new(
                ElevationMm::new(ridge * 64),
                MaterialId::new(material_band + 1),
                RoughnessMm::new(((base >> 33) as u32 & 0x7F) + ((x ^ y) as u32 & 0x1F)),
            )
        })
        .collect()
}

pub fn terrain_pattern(cell: TerrainCell) -> PhysicalPatternId {
    PhysicalPatternId::new(mix64(
        TERRAIN_PATTERN_DOMAIN
            ^ TERRAIN_CARRIER_SCHEMA.raw()
            ^ cell.surface_material.raw().rotate_left(17)
            ^ u64::from(cell.roughness.millimetres()).rotate_left(41),
    ))
}

fn terrain_magnitude(terrain: &TerrainChunk, index: usize) -> u32 {
    let cell = terrain_cell_at(terrain, index);
    let contrast = elevation_contrast(terrain, index);
    let material_delta = material_difference(terrain, index);
    let roughness = cell.roughness.millimetres();
    128_u32
        .saturating_add(contrast / 32)
        .saturating_add(material_delta.saturating_mul(16))
        .saturating_add(roughness)
}

fn terrain_cell_at(terrain: &TerrainChunk, index: usize) -> TerrainCell {
    TerrainCell::new(
        terrain.elevations()[index],
        terrain.surface_materials()[index],
        terrain.roughness()[index],
    )
}

fn elevation_contrast(terrain: &TerrainChunk, index: usize) -> u32 {
    let center = terrain.elevations()[index].millimetres();
    neighbor_indices(index)
        .into_iter()
        .map(|neighbor| center.abs_diff(terrain.elevations()[neighbor].millimetres()))
        .max()
        .unwrap_or(0)
}

fn material_difference(terrain: &TerrainChunk, index: usize) -> u32 {
    let center = terrain.surface_materials()[index].raw();
    neighbor_indices(index)
        .into_iter()
        .map(|neighbor| (center ^ terrain.surface_materials()[neighbor].raw()).count_ones())
        .max()
        .unwrap_or(0)
}

fn neighbor_indices(index: usize) -> Vec<usize> {
    let side = usize::from(CHUNK_SIZE);
    let x = index % side;
    let y = index / side;
    let mut neighbors = Vec::with_capacity(4);
    if x > 0 {
        neighbors.push(index - 1);
    }
    if x + 1 < side {
        neighbors.push(index + 1);
    }
    if y > 0 {
        neighbors.push(index - side);
    }
    if y + 1 < side {
        neighbors.push(index + side);
    }
    neighbors
}

fn field_position(index: usize, extent: u8) -> LocalCoord {
    let side = usize::from(CHUNK_SIZE);
    let extent = usize::from(extent);
    let x = index % side;
    let y = index / side;
    LocalCoord::new((x % extent) as u8, (y % extent) as u8, 0)
}

fn mix64(mut value: u64) -> u64 {
    value ^= value >> 30;
    value = value.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value ^= value >> 27;
    value = value.wrapping_mul(0x94D0_49BB_1331_11EB);
    value ^ (value >> 31)
}

fn material_surface_chart_hash(chunk: ChartChunkCoord) -> u64 {
    mix64(
        chunk.chart.raw()
            ^ (chunk.chunk.x as u64).rotate_left(7)
            ^ (chunk.chunk.y as u64).rotate_left(19)
            ^ (chunk.chunk.z as u64).rotate_left(31),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use causafera_types::ChunkCoord;

    #[test]
    fn terrain_adapter_emits_one_sample_per_cell() {
        let chunk = ChartChunkCoord::new(
            causafera_types::SpatialChartId::new(1),
            ChunkCoord::new(0, 0, 0),
        );
        let adapter = TerrainCarrierAdapter::new(
            chunk,
            deterministic_terrain_chunk(7, chunk, TraceId::new(0)),
            3,
        );

        let samples = adapter.emit_samples(SimulationTime::new(1), TraceId::new(0));

        assert_eq!(samples.len(), TERRAIN_CELLS_PER_CHUNK);
        assert!(samples.iter().all(|sample| sample.chunk == chunk));
        assert!(samples.iter().any(|sample| sample.magnitude > 128));
    }
}
