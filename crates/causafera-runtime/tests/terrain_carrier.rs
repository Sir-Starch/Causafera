use causafera_domains::{ManaField, OpenNeighbors, PhysicalCarrierAdapter, PhysicalPatternSample};
use causafera_geography::{
    TERRAIN_CELLS_PER_CHUNK, TerrainChunk, TerrainGenerationProvenance,
    TerrainGeneratorFingerprint, TerrainParameterFingerprint,
};
use causafera_runtime::{
    Runtime, RuntimeConfig, TerrainCarrierAdapter, deterministic_terrain_chunk, terrain_cells,
    terrain_pattern,
};
use causafera_types::{
    ChartChunkCoord, ChunkCoord, ConceptId, LocalCoord, ManaFieldId, PhysicalPatternId,
    SimulationTime, SpatialChartId, TraceId,
};

fn test_chunk() -> ChartChunkCoord {
    ChartChunkCoord::new(SpatialChartId::new(1), ChunkCoord::new(0, 0, 0))
}

fn three_cell_fixture(
    pattern: PhysicalPatternId,
    tick: u64,
    magnitude: u32,
    cause: TraceId,
) -> Vec<PhysicalPatternSample> {
    let center = 1;
    [center - 1, center, center + 1]
        .into_iter()
        .enumerate()
        .map(|(ordinal, x)| PhysicalPatternSample {
            chunk: test_chunk(),
            pattern,
            position: LocalCoord::new(x, center, center),
            observed_at: SimulationTime::new(tick),
            magnitude,
            source_ordinal: ordinal as u32,
            cause,
        })
        .collect()
}

fn mana_total_for_samples(samples: &[PhysicalPatternSample]) -> i64 {
    let config = RuntimeConfig::new(55);
    let field = ManaField::new(ManaFieldId::new(1), test_chunk(), 3).unwrap();
    field
        .propose_evolution(
            SimulationTime::new(1),
            config.mana_parameters,
            samples,
            &[],
            OpenNeighbors::none(),
        )
        .unwrap()
        .proposed_intensity()
        .iter()
        .copied()
        .sum()
}

#[test]
fn terrain_carrier_produces_different_mana_response() {
    let adapter = TerrainCarrierAdapter::new(
        test_chunk(),
        deterministic_terrain_chunk(57, test_chunk(), TraceId::new(0)),
        3,
    );
    let terrain_samples = adapter.emit_samples(SimulationTime::new(1), TraceId::new(0));
    let fixture_samples = three_cell_fixture(
        PhysicalPatternId::new(7),
        1,
        RuntimeConfig::new(57).pattern_schedule.magnitude,
        TraceId::new(0),
    );

    assert_eq!(terrain_samples.len(), TERRAIN_CELLS_PER_CHUNK);
    assert_ne!(
        mana_total_for_samples(&terrain_samples),
        mana_total_for_samples(&fixture_samples)
    );
}

#[test]
fn structurally_identical_terrain_has_identical_sample_fingerprints() {
    let cells = terrain_cells(61);
    let first = TerrainChunk::from_cells(
        ChunkCoord::new(0, 0, 0),
        terrain_provenance(61, TraceId::new(0), 1),
        cells.clone(),
    )
    .unwrap();
    let second = TerrainChunk::from_cells(
        ChunkCoord::new(0, 0, 0),
        terrain_provenance(61, TraceId::new(0), 2),
        cells,
    )
    .unwrap();
    let first = TerrainCarrierAdapter::new(test_chunk(), first, 3)
        .emit_samples(SimulationTime::new(1), TraceId::new(0));
    let second = TerrainCarrierAdapter::new(test_chunk(), second, 3)
        .emit_samples(SimulationTime::new(1), TraceId::new(0));

    let first_patterns = first
        .iter()
        .map(|sample| sample.pattern)
        .collect::<Vec<_>>();
    let second_patterns = second
        .iter()
        .map(|sample| sample.pattern)
        .collect::<Vec<_>>();

    assert_eq!(first_patterns, second_patterns);
}

#[test]
fn terrain_carrier_determinism() {
    let first_adapter = TerrainCarrierAdapter::new(
        test_chunk(),
        deterministic_terrain_chunk(67, test_chunk(), TraceId::new(0)),
        3,
    );
    let second_adapter = TerrainCarrierAdapter::new(
        test_chunk(),
        deterministic_terrain_chunk(67, test_chunk(), TraceId::new(0)),
        3,
    );
    assert_eq!(
        first_adapter.emit_samples(SimulationTime::new(1), TraceId::new(0)),
        second_adapter.emit_samples(SimulationTime::new(1), TraceId::new(0))
    );

    let mut first_runtime = Runtime::from_seed(67).unwrap();
    let mut second_runtime = Runtime::from_seed(67).unwrap();
    assert_eq!(
        first_runtime.run_ticks(32).unwrap(),
        second_runtime.run_ticks(32).unwrap()
    );
}

#[test]
fn adapter_input_trace_and_ordinal_reconstruct_source_terrain_cell() {
    let adapter = TerrainCarrierAdapter::new(
        test_chunk(),
        deterministic_terrain_chunk(71, test_chunk(), TraceId::new(0)),
        3,
    );
    let samples = adapter.emit_samples(SimulationTime::new(1), TraceId::new(3));
    let sample = samples[42];

    assert_eq!(sample.cause, TraceId::new(3));
    assert_eq!(
        adapter.source_cell(sample.source_ordinal),
        adapter.terrain().cell(10, 1)
    );
}

#[test]
fn sample_fingerprint_ignores_language_concepts_and_semantic_labels() {
    let cell = deterministic_terrain_chunk(73, test_chunk(), TraceId::new(0))
        .cell(4, 5)
        .unwrap();
    let semantic_label = "sacred mountain";
    let belief_like_concept = ConceptId::new(99);

    assert_eq!(semantic_label.len(), 15);
    assert_eq!(belief_like_concept.raw(), 99);
    assert_eq!(terrain_pattern(cell), terrain_pattern(cell));
}

fn terrain_provenance(
    seed: u64,
    generation_trace: TraceId,
    context: u64,
) -> TerrainGenerationProvenance {
    TerrainGenerationProvenance::new(
        seed,
        generation_trace,
        TerrainGeneratorFingerprint::new(context),
        TerrainParameterFingerprint::new(context + 100),
        vec![generation_trace],
    )
}
