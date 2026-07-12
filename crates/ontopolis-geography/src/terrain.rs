use ontopolis_types::{CHUNK_SIZE, ChunkCoord, MaterialId, TraceId};
use std::error::Error;
use std::fmt;

/// Number of surface cells in one terrain chunk.
pub const TERRAIN_CELLS_PER_CHUNK: usize = CHUNK_SIZE as usize * CHUNK_SIZE as usize;

/// Absolute elevation relative to the world's reference level, in millimetres.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ElevationMm(i32);

impl ElevationMm {
    pub const fn new(millimetres: i32) -> Self {
        Self(millimetres)
    }

    pub const fn millimetres(self) -> i32 {
        self.0
    }
}

/// Local surface-height variation, in millimetres.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RoughnessMm(u32);

impl RoughnessMm {
    pub const fn new(millimetres: u32) -> Self {
        Self(millimetres)
    }

    pub const fn millimetres(self) -> u32 {
        self.0
    }
}

/// Value view of one authoritative terrain surface cell.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TerrainCell {
    pub elevation: ElevationMm,
    pub surface_material: MaterialId,
    pub roughness: RoughnessMm,
}

impl TerrainCell {
    pub const fn new(
        elevation: ElevationMm,
        surface_material: MaterialId,
        roughness: RoughnessMm,
    ) -> Self {
        Self {
            elevation,
            surface_material,
            roughness,
        }
    }
}

/// Stable fingerprint of the generator implementation and revision.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TerrainGeneratorFingerprint(u64);

impl TerrainGeneratorFingerprint {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    pub const fn raw(self) -> u64 {
        self.0
    }
}

/// Stable fingerprint of the complete terrain parameter set.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TerrainParameterFingerprint(u64);

impl TerrainParameterFingerprint {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    pub const fn raw(self) -> u64 {
        self.0
    }
}

/// Compact causal record retained with a generated terrain chunk.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TerrainGenerationProvenance {
    world_seed: u64,
    generation_trace: TraceId,
    generator: TerrainGeneratorFingerprint,
    parameters: TerrainParameterFingerprint,
    causal_inputs: Vec<TraceId>,
}

impl TerrainGenerationProvenance {
    pub fn new(
        world_seed: u64,
        generation_trace: TraceId,
        generator: TerrainGeneratorFingerprint,
        parameters: TerrainParameterFingerprint,
        causal_inputs: Vec<TraceId>,
    ) -> Self {
        Self {
            world_seed,
            generation_trace,
            generator,
            parameters,
            causal_inputs,
        }
    }

    pub const fn world_seed(&self) -> u64 {
        self.world_seed
    }

    pub const fn generation_trace(&self) -> TraceId {
        self.generation_trace
    }

    pub const fn generator(&self) -> TerrainGeneratorFingerprint {
        self.generator
    }

    pub const fn parameters(&self) -> TerrainParameterFingerprint {
        self.parameters
    }

    pub fn causal_inputs(&self) -> &[TraceId] {
        &self.causal_inputs
    }
}

/// Complete explicit input for generating one terrain chunk.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TerrainGenerationRequest {
    chunk: ChunkCoord,
    provenance: TerrainGenerationProvenance,
}

impl TerrainGenerationRequest {
    pub const fn new(chunk: ChunkCoord, provenance: TerrainGenerationProvenance) -> Self {
        Self { chunk, provenance }
    }

    pub const fn chunk(&self) -> ChunkCoord {
        self.chunk
    }

    pub const fn provenance(&self) -> &TerrainGenerationProvenance {
        &self.provenance
    }
}

/// Malformed dense terrain field lengths.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TerrainChunkError {
    pub expected: usize,
    pub elevations: usize,
    pub surface_materials: usize,
    pub roughness: usize,
}

impl fmt::Display for TerrainChunkError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "terrain fields must each contain {} cells (elevation {}, surface material {}, roughness {})",
            self.expected, self.elevations, self.surface_materials, self.roughness
        )
    }
}

impl Error for TerrainChunkError {}

/// Dense structure-of-arrays terrain state for one chunk surface.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TerrainChunk {
    chunk: ChunkCoord,
    provenance: TerrainGenerationProvenance,
    elevations: Vec<ElevationMm>,
    surface_materials: Vec<MaterialId>,
    roughness: Vec<RoughnessMm>,
}

impl TerrainChunk {
    pub fn from_cells(
        chunk: ChunkCoord,
        provenance: TerrainGenerationProvenance,
        cells: Vec<TerrainCell>,
    ) -> Result<Self, TerrainChunkError> {
        let mut elevations = Vec::with_capacity(cells.len());
        let mut surface_materials = Vec::with_capacity(cells.len());
        let mut roughness = Vec::with_capacity(cells.len());

        for cell in cells {
            elevations.push(cell.elevation);
            surface_materials.push(cell.surface_material);
            roughness.push(cell.roughness);
        }

        Self::from_fields(chunk, provenance, elevations, surface_materials, roughness)
    }

    pub fn from_fields(
        chunk: ChunkCoord,
        provenance: TerrainGenerationProvenance,
        elevations: Vec<ElevationMm>,
        surface_materials: Vec<MaterialId>,
        roughness: Vec<RoughnessMm>,
    ) -> Result<Self, TerrainChunkError> {
        if elevations.len() != TERRAIN_CELLS_PER_CHUNK
            || surface_materials.len() != TERRAIN_CELLS_PER_CHUNK
            || roughness.len() != TERRAIN_CELLS_PER_CHUNK
        {
            return Err(TerrainChunkError {
                expected: TERRAIN_CELLS_PER_CHUNK,
                elevations: elevations.len(),
                surface_materials: surface_materials.len(),
                roughness: roughness.len(),
            });
        }

        Ok(Self {
            chunk,
            provenance,
            elevations,
            surface_materials,
            roughness,
        })
    }

    pub const fn chunk(&self) -> ChunkCoord {
        self.chunk
    }

    pub const fn provenance(&self) -> &TerrainGenerationProvenance {
        &self.provenance
    }

    pub fn elevations(&self) -> &[ElevationMm] {
        &self.elevations
    }

    pub fn surface_materials(&self) -> &[MaterialId] {
        &self.surface_materials
    }

    pub fn roughness(&self) -> &[RoughnessMm] {
        &self.roughness
    }

    /// Return a cell in row-major order (`y * CHUNK_SIZE + x`).
    pub fn cell(&self, x: u8, y: u8) -> Option<TerrainCell> {
        if x >= CHUNK_SIZE || y >= CHUNK_SIZE {
            return None;
        }

        let index = y as usize * CHUNK_SIZE as usize + x as usize;
        Some(TerrainCell {
            elevation: self.elevations[index],
            surface_material: self.surface_materials[index],
            roughness: self.roughness[index],
        })
    }
}

/// Batch-first terrain generation boundary.
///
/// Implementations must be deterministic functions of the ordered request slice. Output chunks
/// must correspond one-for-one, in order, to those requests.
pub trait TerrainGenerator {
    type Error;

    fn generate_batch(
        &self,
        requests: &[TerrainGenerationRequest],
    ) -> Result<Vec<TerrainChunk>, Self::Error>;
}

/// Invalid output returned by a terrain generator implementation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TerrainBatchValidationError {
    OutputCount {
        expected: usize,
        actual: usize,
    },
    ChunkMismatch {
        index: usize,
        expected: ChunkCoord,
        actual: ChunkCoord,
    },
    ProvenanceMismatch {
        index: usize,
    },
}

impl fmt::Display for TerrainBatchValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OutputCount { expected, actual } => {
                write!(
                    formatter,
                    "expected {expected} terrain chunks, received {actual}"
                )
            }
            Self::ChunkMismatch {
                index,
                expected,
                actual,
            } => write!(
                formatter,
                "terrain output {index} targets chunk {actual}, expected {expected}"
            ),
            Self::ProvenanceMismatch { index } => {
                write!(
                    formatter,
                    "terrain output {index} did not preserve request provenance"
                )
            }
        }
    }
}

impl Error for TerrainBatchValidationError {}

/// Validate a generator batch at the authoritative-state boundary.
pub fn validate_terrain_batch(
    requests: &[TerrainGenerationRequest],
    chunks: &[TerrainChunk],
) -> Result<(), TerrainBatchValidationError> {
    if requests.len() != chunks.len() {
        return Err(TerrainBatchValidationError::OutputCount {
            expected: requests.len(),
            actual: chunks.len(),
        });
    }

    for (index, (request, chunk)) in requests.iter().zip(chunks).enumerate() {
        if request.chunk() != chunk.chunk() {
            return Err(TerrainBatchValidationError::ChunkMismatch {
                index,
                expected: request.chunk(),
                actual: chunk.chunk(),
            });
        }
        if request.provenance() != chunk.provenance() {
            return Err(TerrainBatchValidationError::ProvenanceMismatch { index });
        }
    }

    Ok(())
}

/// Error from generation or authoritative-boundary validation.
#[derive(Debug)]
pub enum TerrainGenerationError<E> {
    Generator(E),
    InvalidOutput(TerrainBatchValidationError),
}

impl<E: fmt::Display> fmt::Display for TerrainGenerationError<E> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Generator(error) => write!(formatter, "terrain generator failed: {error}"),
            Self::InvalidOutput(error) => error.fmt(formatter),
        }
    }
}

impl<E: Error + 'static> Error for TerrainGenerationError<E> {}

/// Generate a batch and validate its identity, order, and provenance before use.
pub fn generate_validated_batch<G: TerrainGenerator>(
    generator: &G,
    requests: &[TerrainGenerationRequest],
) -> Result<Vec<TerrainChunk>, TerrainGenerationError<G::Error>> {
    let chunks = generator
        .generate_batch(requests)
        .map_err(TerrainGenerationError::Generator)?;
    validate_terrain_batch(requests, &chunks).map_err(TerrainGenerationError::InvalidOutput)?;
    Ok(chunks)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::convert::Infallible;

    fn provenance(seed: u64, trace: u64) -> TerrainGenerationProvenance {
        TerrainGenerationProvenance::new(
            seed,
            TraceId::new(trace),
            TerrainGeneratorFingerprint::new(7),
            TerrainParameterFingerprint::new(11),
            vec![TraceId::new(3), TraceId::new(5)],
        )
    }

    fn cells(seed: u64) -> Vec<TerrainCell> {
        (0..TERRAIN_CELLS_PER_CHUNK)
            .map(|index| {
                TerrainCell::new(
                    ElevationMm::new(seed as i32 + index as i32),
                    MaterialId::new(index as u64),
                    RoughnessMm::new(index as u32),
                )
            })
            .collect()
    }

    fn chunk(request: &TerrainGenerationRequest) -> TerrainChunk {
        TerrainChunk::from_cells(
            request.chunk(),
            request.provenance().clone(),
            cells(request.provenance().world_seed()),
        )
        .unwrap()
    }

    struct DeterministicTestGenerator;

    impl TerrainGenerator for DeterministicTestGenerator {
        type Error = Infallible;

        fn generate_batch(
            &self,
            requests: &[TerrainGenerationRequest],
        ) -> Result<Vec<TerrainChunk>, Self::Error> {
            Ok(requests.iter().map(chunk).collect())
        }
    }

    #[test]
    fn chunk_requires_complete_dense_fields() {
        let error = TerrainChunk::from_fields(
            ChunkCoord::new(0, 0, 0),
            provenance(1, 2),
            vec![],
            vec![],
            vec![],
        )
        .unwrap_err();

        assert_eq!(error.expected, TERRAIN_CELLS_PER_CHUNK);
        assert_eq!(error.elevations, 0);
    }

    #[test]
    fn cell_access_uses_row_major_surface_order() {
        let request = TerrainGenerationRequest::new(ChunkCoord::new(2, -1, 0), provenance(13, 17));
        let terrain = chunk(&request);

        assert_eq!(terrain.cell(0, 1).unwrap().elevation, ElevationMm::new(45));
        assert_eq!(terrain.cell(CHUNK_SIZE, 0), None);
    }

    #[test]
    fn provenance_retains_ordered_causal_inputs() {
        let record = provenance(19, 23);

        assert_eq!(record.world_seed(), 19);
        assert_eq!(record.generation_trace(), TraceId::new(23));
        assert_eq!(record.causal_inputs(), &[TraceId::new(3), TraceId::new(5)]);
    }

    #[test]
    fn validated_batch_preserves_request_order_and_identity() {
        let requests = vec![
            TerrainGenerationRequest::new(ChunkCoord::new(1, 0, 0), provenance(29, 31)),
            TerrainGenerationRequest::new(ChunkCoord::new(-1, 0, 0), provenance(29, 37)),
        ];

        let output = generate_validated_batch(&DeterministicTestGenerator, &requests).unwrap();

        assert_eq!(output[0].chunk(), requests[0].chunk());
        assert_eq!(output[1].chunk(), requests[1].chunk());
    }

    #[test]
    fn identical_requests_produce_identical_terrain() {
        let requests = vec![TerrainGenerationRequest::new(
            ChunkCoord::new(4, 5, 0),
            provenance(41, 43),
        )];

        let first = generate_validated_batch(&DeterministicTestGenerator, &requests).unwrap();
        let second = generate_validated_batch(&DeterministicTestGenerator, &requests).unwrap();

        assert_eq!(first, second);
    }

    #[test]
    fn validation_rejects_reordered_chunks() {
        let requests = vec![
            TerrainGenerationRequest::new(ChunkCoord::new(0, 0, 0), provenance(47, 53)),
            TerrainGenerationRequest::new(ChunkCoord::new(1, 0, 0), provenance(47, 59)),
        ];
        let chunks = vec![chunk(&requests[1]), chunk(&requests[0])];

        assert!(matches!(
            validate_terrain_batch(&requests, &chunks),
            Err(TerrainBatchValidationError::ChunkMismatch { index: 0, .. })
        ));
    }

    #[test]
    fn validation_rejects_changed_provenance() {
        let request = TerrainGenerationRequest::new(ChunkCoord::new(0, 0, 0), provenance(61, 67));
        let changed = TerrainGenerationRequest::new(request.chunk(), provenance(61, 71));

        assert_eq!(
            validate_terrain_batch(&[request], &[chunk(&changed)]),
            Err(TerrainBatchValidationError::ProvenanceMismatch { index: 0 })
        );
    }
}
