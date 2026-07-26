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
/// Roughness is quantised into classes before it reaches a column fingerprint,
/// so two columns of comparably rough ground share a pattern and the field's
/// spatial-repetition channel can see them. A millimetre-exact mean would make
/// every column unique and leave that channel permanently silent.
const TERRAIN_ROUGHNESS_CLASS_MM: u64 = 32;
const TERRAIN_GENERATOR: TerrainGeneratorFingerprint =
    TerrainGeneratorFingerprint::new(0x2405_0001);
const TERRAIN_PARAMETERS: TerrainParameterFingerprint =
    TerrainParameterFingerprint::new(0x2405_0001);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TerrainCarrierAdapter {
    pub(crate) chunk: ChartChunkCoord,
    pub(crate) terrain: TerrainChunk,
    pub(crate) field_extent: u8,
    /// The lattice projection of `terrain`, derived once.
    ///
    /// It is a pure function of the terrain and the extent, so it is never
    /// persisted and never compared: two adapters holding the same terrain at
    /// the same extent hold the same columns by construction. Recomputing it
    /// per tick is what a standing carrier must not cost.
    columns: Vec<TerrainColumn>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MaterialSurfaceCarrierAdapter {
    pub(crate) field_extent: u8,
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
    pub fn new(chunk: ChartChunkCoord, terrain: TerrainChunk, field_extent: u8) -> Self {
        let columns = project_columns(&terrain, field_extent);
        Self {
            chunk,
            terrain,
            field_extent,
            columns,
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

    /// The standing terrain structure, one column per plan-view cell of the
    /// mana lattice.
    ///
    /// A column's block of ground is summarised, not decimated: every terrain
    /// cell in the block contributes to the column's structure and to the
    /// material it is counted as. Columns are in the lattice's canonical
    /// row-major order, which is also `source_ordinal` order.
    pub fn columns(&self) -> &[TerrainColumn] {
        &self.columns
    }

    /// The column a sample's `source_ordinal` came from, so a mana cell change
    /// can be resolved back to the ground under it.
    pub fn source_column(&self, source_ordinal: u32) -> Option<TerrainColumn> {
        let index = usize::try_from(source_ordinal).ok()?;
        self.columns.get(index).copied()
    }
}

fn project_columns(terrain: &TerrainChunk, field_extent: u8) -> Vec<TerrainColumn> {
    let extent = usize::from(field_extent);
    // A zero extent is no lattice, so it has no columns and the carrier emits
    // nothing. `TerrainCarrierAdapter::new` is infallible and reachable from
    // decoded persistence bytes, so this must not be an indexing assumption:
    // `import_carrier_adapters` rejects the extent, and it must be able to.
    if extent == 0 {
        return Vec::new();
    }
    let mut blocks = vec![ColumnAccumulator::default(); extent * extent];
    for index in 0..TERRAIN_CELLS_PER_CHUNK {
        let position = terrain_field_position(index, field_extent);
        let column = usize::from(position.y) * extent + usize::from(position.x);
        let cell = terrain_cell_at(terrain, index);
        let block = &mut blocks[column];
        block.cells += 1;
        block.structure += u64::from(terrain_structure(terrain, index));
        block.roughness += u64::from(cell.roughness.millimetres());
        block.material_counts.push(cell.surface_material);
    }
    blocks
        .into_iter()
        .enumerate()
        .map(|(column, block)| {
            let cells = block.cells.max(1);
            TerrainColumn {
                ordinal: column as u32,
                position: LocalCoord::new((column % extent) as u8, (column / extent) as u8, 0),
                cell_count: block.cells,
                dominant_material: dominant_material(&block.material_counts),
                roughness_class: (block.roughness / cells as u64 / TERRAIN_ROUGHNESS_CLASS_MM)
                    as u32,
                structure: u32::try_from(block.structure / cells as u64).unwrap_or(u32::MAX),
            }
        })
        .collect()
}

/// One plan-view column of the mana lattice, summarised from the terrain cells
/// standing under it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TerrainColumn {
    pub ordinal: u32,
    pub position: LocalCoord,
    pub cell_count: usize,
    pub dominant_material: MaterialId,
    pub roughness_class: u32,
    /// Mean per-cell terrain structure: relief contrast, material discontinuity
    /// and roughness. Featureless ground scores zero.
    pub structure: u32,
}

impl TerrainColumn {
    /// A fingerprint of the column's physical composition. It is derived from
    /// surface material and roughness only, exactly as a single cell's
    /// fingerprint is, so no label, concept or observer classification can
    /// reach it.
    pub fn pattern(self) -> PhysicalPatternId {
        PhysicalPatternId::new(mix64(
            TERRAIN_PATTERN_DOMAIN
                ^ TERRAIN_CARRIER_SCHEMA.raw()
                ^ self.dominant_material.raw().rotate_left(17)
                ^ u64::from(self.roughness_class).rotate_left(41),
        ))
    }
}

#[derive(Clone, Default)]
struct ColumnAccumulator {
    cells: usize,
    structure: u64,
    roughness: u64,
    material_counts: Vec<MaterialId>,
}

/// The most frequent surface material in a column, lowest identifier first on a
/// tie so the choice never depends on iteration order.
fn dominant_material(materials: &[MaterialId]) -> MaterialId {
    let mut counts = std::collections::BTreeMap::<MaterialId, usize>::new();
    for material in materials {
        *counts.entry(*material).or_default() += 1;
    }
    counts
        .into_iter()
        .max_by_key(|(material, count)| (*count, std::cmp::Reverse(*material)))
        .map(|(material, _)| material)
        .unwrap_or(MaterialId::new(0))
}

impl PhysicalCarrierAdapter for TerrainCarrierAdapter {
    fn schema(&self) -> CarrierAdapterSchemaId {
        TERRAIN_CARRIER_SCHEMA
    }

    /// Terrain presents its standing structure at the mana lattice's own
    /// resolution.
    ///
    /// The field can only hold structure at its own lattice. Emitting one
    /// sample per terrain cell would drive `CHUNK_SIZE²` samples into `extent²`
    /// columns — over-sampling the lattice by a factor of a hundred at the
    /// default extent, which is a resolution mismatch rather than physics, and
    /// which would swamp the change-driven carriers sharing the stream.
    fn emit_samples(&self, time: SimulationTime, cause: TraceId) -> Vec<PhysicalPatternSample> {
        self.columns
            .iter()
            .copied()
            .filter(|column| column.structure > 0)
            .map(|column| PhysicalPatternSample {
                chunk: self.chunk,
                pattern: column.pattern(),
                position: column.position,
                observed_at: time,
                magnitude: column.structure,
                source_ordinal: column.ordinal,
                cause,
            })
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

/// The physical structure a terrain cell carries: relief contrast against its
/// neighbours, surface material discontinuity, and roughness.
///
/// There is no constant term. Featureless ground is not a physical pattern, and
/// a floor under every cell would make a flat plain drive the field as hard as
/// a ridge does.
fn terrain_structure(terrain: &TerrainChunk, index: usize) -> u32 {
    let cell = terrain_cell_at(terrain, index);
    let contrast = elevation_contrast(terrain, index);
    let material_delta = material_difference(terrain, index);
    let roughness = cell.roughness.millimetres();
    (contrast / 32)
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

/// Project a terrain cell onto the mana lattice column that covers it.
///
/// Terrain is always `CHUNK_SIZE × CHUNK_SIZE` in plan view; the mana field is
/// `extent³`. A cell belongs to the column whose block of the chunk contains it,
/// so a contiguous patch of ground reaches one column and the landform's spatial
/// structure survives the projection.
fn terrain_field_position(index: usize, extent: u8) -> LocalCoord {
    let side = usize::from(CHUNK_SIZE);
    let extent = usize::from(extent);
    let x = (index % side) * extent / side;
    let y = (index / side) * extent / side;
    LocalCoord::new(x as u8, y as u8, 0)
}

/// Decompose a mana cell index into its lattice position.
fn field_position(index: usize, extent: u8) -> LocalCoord {
    let extent = usize::from(extent);
    let plane = extent * extent;
    LocalCoord::new(
        (index % extent) as u8,
        ((index % plane) / extent) as u8,
        (index / plane) as u8,
    )
}

pub(crate) fn mix64(mut value: u64) -> u64 {
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

    fn test_chunk() -> ChartChunkCoord {
        ChartChunkCoord::new(
            causafera_types::SpatialChartId::new(1),
            ChunkCoord::new(0, 0, 0),
        )
    }

    fn adapter(seed: u64, extent: u8) -> TerrainCarrierAdapter {
        TerrainCarrierAdapter::new(
            test_chunk(),
            deterministic_terrain_chunk(seed, test_chunk(), TraceId::new(0)),
            extent,
        )
    }

    #[test]
    fn terrain_adapter_emits_one_sample_per_lattice_column() {
        for extent in [3, 4, 6, 8] {
            let adapter = adapter(7, extent);

            let samples = adapter.emit_samples(SimulationTime::new(1), TraceId::new(0));

            assert_eq!(samples.len(), usize::from(extent).pow(2));
            assert!(samples.iter().all(|sample| sample.chunk == test_chunk()));
            assert!(samples.iter().all(|sample| sample.position.z == 0));
            assert!(samples.iter().all(|sample| sample.magnitude > 0));
        }
    }

    #[test]
    fn every_terrain_cell_reaches_exactly_one_column() {
        for extent in [3, 4, 6, 8] {
            let adapter = adapter(7, extent);
            let columns = adapter.columns();

            assert_eq!(columns.len(), usize::from(extent).pow(2));
            assert_eq!(
                columns
                    .iter()
                    .map(|column| column.cell_count)
                    .sum::<usize>(),
                TERRAIN_CELLS_PER_CHUNK
            );
            assert!(columns.iter().all(|column| column.cell_count > 0));
        }
    }

    #[test]
    fn a_column_summarises_a_contiguous_patch_of_ground() {
        // A stride sample would put cells 0, extent, 2*extent … in one column
        // and destroy the landform; a projection keeps neighbours together.
        let extent = 4;
        let side = usize::from(CHUNK_SIZE);
        let block = side / usize::from(extent);
        for index in 0..TERRAIN_CELLS_PER_CHUNK {
            let position = terrain_field_position(index, extent);
            assert_eq!(usize::from(position.x), (index % side) / block);
            assert_eq!(usize::from(position.y), (index / side) / block);
        }
    }

    #[test]
    fn a_column_fingerprint_follows_its_composition_not_its_place() {
        let adapter = adapter(7, 3);
        let columns = adapter.columns();
        let shared = columns
            .iter()
            .find(|column| {
                columns.iter().any(|other| {
                    other.ordinal != column.ordinal
                        && other.dominant_material == column.dominant_material
                        && other.roughness_class == column.roughness_class
                })
            })
            .copied();

        // Two columns of the same composition must be one repeated structure to
        // the field, which is what its spatial channel measures.
        if let Some(column) = shared {
            let twin = columns
                .iter()
                .find(|other| {
                    other.ordinal != column.ordinal
                        && other.dominant_material == column.dominant_material
                        && other.roughness_class == column.roughness_class
                })
                .expect("the twin was just found");
            assert_eq!(column.pattern(), twin.pattern());
        }

        let different = columns
            .iter()
            .find(|other| other.dominant_material != columns[0].dominant_material);
        if let Some(different) = different {
            assert_ne!(columns[0].pattern(), different.pattern());
        }
    }

    #[test]
    fn a_sample_ordinal_resolves_back_to_the_ground_it_came_from() {
        let adapter = adapter(11, 3);

        let samples = adapter.emit_samples(SimulationTime::new(1), TraceId::new(3));
        let sample = samples[4];
        let column = adapter
            .source_column(sample.source_ordinal)
            .expect("an emitted ordinal resolves");

        assert_eq!(column.position, sample.position);
        assert_eq!(column.structure, sample.magnitude);
        assert_eq!(column.pattern(), sample.pattern);
    }

    #[test]
    fn featureless_ground_is_not_a_physical_pattern() {
        let flat = TerrainChunk::from_cells(
            test_chunk().chunk,
            TerrainGenerationProvenance::new(
                0,
                TraceId::new(0),
                TERRAIN_GENERATOR,
                TERRAIN_PARAMETERS,
                vec![TraceId::new(0)],
            ),
            vec![
                TerrainCell::new(
                    ElevationMm::new(1_000),
                    MaterialId::new(1),
                    RoughnessMm::new(0),
                );
                TERRAIN_CELLS_PER_CHUNK
            ],
        )
        .expect("a uniform chunk is a complete chunk");

        let samples = TerrainCarrierAdapter::new(test_chunk(), flat, 3)
            .emit_samples(SimulationTime::new(1), TraceId::new(0));

        assert!(samples.is_empty());
    }
}
