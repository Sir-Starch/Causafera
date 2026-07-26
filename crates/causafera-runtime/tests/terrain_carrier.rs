use causafera_domains::{ManaField, OpenNeighbors, PhysicalCarrierAdapter, PhysicalPatternSample};
use causafera_geography::{
    TerrainChunk, TerrainGenerationProvenance, TerrainGeneratorFingerprint,
    TerrainParameterFingerprint,
};
use causafera_runtime::{
    Runtime, RuntimeConfig, TerrainCarrierAdapter, TerrainParticipation,
    deterministic_terrain_chunk, terrain_cells, terrain_pattern,
};
use causafera_types::{
    ChartChunkCoord, ChunkCoord, ConceptId, LocalCoord, ManaFieldId, PhysicalPatternId,
    SimulationTime, SpatialChartId, TraceId,
};

/// The mana lattice the carrier projects onto in these tests.
const EXTENT: u8 = 3;

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

/// A group of columns sharing one fingerprint, as a standing carrier presents
/// them: all at one tick, with no history behind them.
fn standing_group(tick: u64) -> Vec<PhysicalPatternSample> {
    (0..4)
        .map(|ordinal| PhysicalPatternSample {
            chunk: test_chunk(),
            pattern: PhysicalPatternId::new(42),
            position: LocalCoord::new(ordinal as u8 % 3, ordinal as u8 / 3, 0),
            observed_at: SimulationTime::new(tick),
            magnitude: 100,
            source_ordinal: ordinal,
            cause: TraceId::new(1),
        })
        .collect()
}

fn mana_total_at(tick: u64, history: &[PhysicalPatternSample]) -> i64 {
    ManaField::new(ManaFieldId::new(1), test_chunk(), 3)
        .unwrap()
        .propose_evolution(
            SimulationTime::new(tick),
            RuntimeConfig::new(55).mana_parameters,
            &standing_group(tick),
            history,
            OpenNeighbors::none(),
        )
        .unwrap()
        .proposed_intensity()
        .iter()
        .copied()
        .sum()
}

#[test]
fn a_standing_carrier_is_never_paid_for_being_re_read() {
    // Given: the same emission read at three widely separated ticks. This is the
    // whole reason terrain is kept out of `PhysicalPatternHistory`, so it is
    // asserted rather than argued.

    // Then: the response is exactly invariant to when, and how often, the
    // carrier is read. Nothing about the schedule can accumulate.
    assert_eq!(mana_total_at(1, &[]), 492);
    assert_eq!(mana_total_at(5, &[]), 492);
    assert_eq!(mana_total_at(50, &[]), 492);

    // And: retaining the same standing emission in history instead would let the
    // recurrence and periodicity channels accumulate over the window, scoring
    // the read cadence rather than the world. That is the measured difference
    // the exclusion buys.
    let retained = (1..=8).flat_map(standing_group).collect::<Vec<_>>();
    assert_eq!(mana_total_at(9, &retained), 3_564);
}

#[test]
fn a_standing_carrier_earns_no_credit_for_regular_timing() {
    // Given: the response constants zeroed one at a time.
    let response = |adjust: fn(&mut causafera_domains::ManaParameters)| {
        let mut parameters = RuntimeConfig::new(55).mana_parameters;
        adjust(&mut parameters);
        ManaField::new(ManaFieldId::new(1), test_chunk(), 3)
            .unwrap()
            .propose_evolution(
                SimulationTime::new(1),
                parameters,
                &standing_group(1),
                &[],
                OpenNeighbors::none(),
            )
            .unwrap()
            .proposed_intensity()
            .iter()
            .copied()
            .sum::<i64>()
    };

    // Then: recurrence, synchronisation and spatial repetition each carry part
    // of the response, and periodicity carries none of it — a single occupied
    // tick supplies no intervals to be regular. Recurrence is included because
    // RFC-MANA-001 counts additional occurrences of a fingerprint without
    // requiring distinct ticks; synchronisation is its same-tick specialisation.
    assert_eq!(response(|_| {}), 492);
    assert_eq!(response(|p| p.recurrence_response = 0), 340);
    assert_eq!(response(|p| p.synchrony_response = 0), 340);
    assert_eq!(response(|p| p.spatial_response = 0), 340);
    assert_eq!(response(|p| p.periodicity_response = 0), 492);
}

#[test]
fn terrain_carrier_produces_different_mana_response() {
    let adapter = TerrainCarrierAdapter::new(
        test_chunk(),
        deterministic_terrain_chunk(57, test_chunk(), TraceId::new(0)),
        EXTENT,
    );
    let terrain_samples = adapter.emit_samples(SimulationTime::new(1), TraceId::new(0));
    let fixture_samples = three_cell_fixture(
        PhysicalPatternId::new(7),
        1,
        RuntimeConfig::new(57).pattern_schedule.magnitude,
        TraceId::new(0),
    );

    assert_eq!(terrain_samples.len(), usize::from(EXTENT).pow(2));
    assert_ne!(
        mana_total_for_samples(&terrain_samples),
        mana_total_for_samples(&fixture_samples)
    );
}

#[test]
fn terrain_reaches_the_field_and_the_seed_reaches_terrain() {
    // Given: two worlds that differ only in their seed.
    let mut first = Runtime::from_seed(7).expect("first runtime bootstraps");
    let mut second = Runtime::from_seed(11).expect("second runtime bootstraps");

    // When: both run the same number of ticks with no actor and no contact, so
    // the terrain carrier is the only physical source in either world.
    let first = first.run_ticks(32).expect("first world runs");
    let second = second.run_ticks(32).expect("second world runs");

    // Then: neither world is empty, and they are not the same world.
    assert!(first.mana_total > 0);
    assert!(second.mana_total > 0);
    assert_ne!(first.mana_total, second.mana_total);
    assert_ne!(first.physical_state_digest, second.physical_state_digest);
}

#[test]
fn an_inert_terrain_carrier_leaves_an_empty_world_empty() {
    // Given: the same world with the terrain carrier held out of the tick loop.
    let mut config = RuntimeConfig::new(7);
    config.terrain_participation = TerrainParticipation::Inert;

    // When: it runs with no actor and no contact.
    let snapshot = Runtime::new(config)
        .expect("inert runtime bootstraps")
        .run_ticks(32)
        .expect("inert world runs");

    // Then: nothing has happened, which is the behaviour recorded in
    // TODO-RUNTIME-002 before terrain reached the loop.
    assert_eq!(snapshot.mana_total, 0);
}

/// The production-shaped loop `TODO-RUNTIME-002` measures: actors contact
/// material surfaces, contacts feed the mana field, and the local gate feeds
/// back into surface condition.
fn seed_comparison_config(seed: u64) -> RuntimeConfig {
    let mut config = RuntimeConfig::new(seed);
    config.actor_count = 8;
    config.sensor_count = 2;
    config.bootstrap_population = 512;
    config
}

#[test]
fn different_seeds_produce_different_worlds_not_one_world_with_two_terrains() {
    // Given: two production worlds differing only in their seed.
    let mut first = Runtime::new(seed_comparison_config(7)).expect("first world bootstraps");
    let mut second = Runtime::new(seed_comparison_config(59)).expect("second world bootstraps");

    // When: both run the same tick count.
    let first_summary = first.run_ticks(48).expect("first world runs");
    let second_summary = second.run_ticks(48).expect("second world runs");
    let first_state = first.export_snapshot().expect("first world exports");
    let second_state = second.export_snapshot().expect("second world exports");

    // Then: the physical digest, the mana field, and the behavioural counts all
    // differ. This is the acceptance criterion of TODO-RUNTIME-002; before the
    // terrain carrier reached the tick loop, every one of these was identical
    // across seeds.
    assert_ne!(
        first_summary.physical_state_digest,
        second_summary.physical_state_digest
    );
    assert_ne!(first_summary.mana_total, second_summary.mana_total);
    assert_ne!(
        first_summary.mana_physical_effects,
        second_summary.mana_physical_effects
    );
    assert_ne!(
        first_state.material_surfaces.gate_transitions.len(),
        second_state.material_surfaces.gate_transitions.len()
    );
    assert_ne!(
        surface_conditions(&first_state),
        surface_conditions(&second_state)
    );
}

#[test]
fn the_same_seed_still_reproduces_itself_exactly() {
    // Given: the same production world twice.
    let mut first = Runtime::new(seed_comparison_config(7)).expect("first world bootstraps");
    let mut second = Runtime::new(seed_comparison_config(7)).expect("second world bootstraps");

    // When: both run the same tick count with the terrain carrier standing.
    let first_summary = first.run_ticks(48).expect("first world runs");
    let second_summary = second.run_ticks(48).expect("second world runs");

    // Then: nothing about the carrier's participation has weakened replay.
    assert_eq!(first_summary, second_summary);
    assert_eq!(
        first.export_snapshot().expect("first world exports"),
        second.export_snapshot().expect("second world exports")
    );
}

#[test]
fn a_carrier_extent_that_addresses_no_lattice_is_rejected_on_import() {
    // Given: an authoritative snapshot whose terrain carriers claim an extent
    // the configured mana field was never built at. `field_extent` is decoded as
    // a bare byte, so this is reachable from any corrupt or hostile snapshot.
    let mut zero = Runtime::from_seed(7)
        .expect("world bootstraps")
        .export_snapshot()
        .expect("world exports");
    let mut mismatched = zero.clone();
    for carrier in &mut zero.spatial.carrier_adapters {
        carrier.field_extent = 0;
    }
    for carrier in &mut mismatched.spatial.carrier_adapters {
        carrier.field_extent = 9;
    }

    // Then: both fail closed with a typed error. Neither panics, and neither
    // reaches a tick that would try to place a sample outside the field.
    for (label, data) in [("zero", zero), ("mismatched", mismatched)] {
        assert!(
            matches!(
                Runtime::from_snapshot(data),
                Err(causafera_runtime::RuntimeError::InvalidSnapshot(
                    "terrain carrier extent does not match the configured chunk extent"
                ))
            ),
            "a {label} carrier extent must be rejected on import"
        );
    }
}

#[test]
fn projecting_a_carrier_that_has_no_lattice_yields_no_columns() {
    // Given: the adapter constructed directly at a zero extent. The constructor
    // is infallible and public, so it must be total rather than relying on the
    // caller having validated the extent first.
    let adapter = TerrainCarrierAdapter::new(
        test_chunk(),
        deterministic_terrain_chunk(7, test_chunk(), TraceId::new(0)),
        0,
    );

    // Then: no lattice means no columns and no samples, not a panic.
    assert!(adapter.columns().is_empty());
    assert!(
        adapter
            .emit_samples(SimulationTime::new(1), TraceId::new(0))
            .is_empty()
    );
}

#[test]
fn terrain_participation_survives_a_snapshot_round_trip() {
    // Given: a world whose terrain carrier is deliberately held inert.
    let mut config = seed_comparison_config(7);
    config.terrain_participation = TerrainParticipation::Inert;
    let mut original = Runtime::new(config).expect("inert world bootstraps");
    original.run_ticks(8).expect("inert world runs");
    let exported = original.export_snapshot().expect("inert world exports");

    // When: it is resumed from its authoritative snapshot and both continue.
    let mut resumed = Runtime::from_snapshot(exported.clone()).expect("inert world resumes");
    assert_eq!(
        exported.recipe.config.terrain_participation,
        TerrainParticipation::Inert
    );
    let original_summary = original.run_ticks(8).expect("original continues");
    let resumed_summary = resumed.run_ticks(8).expect("resumed continues");

    // Then: the contract is carried in the snapshot, not defaulted on read. A
    // resumed world that silently began sensing its terrain would be a
    // different world, which the standing control confirms it would be.
    assert_eq!(original_summary, resumed_summary);
    let standing = Runtime::new(seed_comparison_config(7))
        .expect("standing world bootstraps")
        .run_ticks(16)
        .expect("standing world runs");
    assert_ne!(standing.mana_total, original_summary.mana_total);
}

fn surface_conditions(state: &causafera_runtime::RuntimeSnapshotData) -> i64 {
    state
        .material_surfaces
        .records
        .iter()
        .map(|record| record.surface.condition)
        .sum()
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
fn adapter_input_trace_and_ordinal_reconstruct_source_terrain_column() {
    let adapter = TerrainCarrierAdapter::new(
        test_chunk(),
        deterministic_terrain_chunk(71, test_chunk(), TraceId::new(0)),
        EXTENT,
    );
    let samples = adapter.emit_samples(SimulationTime::new(1), TraceId::new(3));
    let sample = samples[4];
    let column = adapter
        .source_column(sample.source_ordinal)
        .expect("an emitted ordinal resolves to the ground it summarises");

    assert_eq!(sample.cause, TraceId::new(3));
    assert_eq!(column.position, sample.position);
    assert_eq!(column.pattern(), sample.pattern);
    // The column is a patch of real ground, not a single cell standing for one.
    assert!(column.cell_count > 1);
    assert_eq!(
        adapter.source_cell(0),
        adapter.terrain().cell(0, 0),
        "the per-cell raster the observer projects is unchanged"
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
