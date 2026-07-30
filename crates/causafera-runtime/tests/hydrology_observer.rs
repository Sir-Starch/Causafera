//! The hydrology observer projection over engine-produced state.
//!
//! Every scenario here ticks a production-bootstrapped runtime rather than
//! constructing observer values by hand: the point of the projection is that it
//! reports what the simulation actually did, and a hand-built summary would
//! prove only that the struct has fields.
//!
//! Covers `plans/hydrology.md` §12, V27 (observer neutrality), and the
//! projection half of V28 and V33.

mod support_hydrology;

use causafera_observer_api::{
    FieldRasterKind, FieldRasterRequest, HYDROLOGY_RASTER_VALUES_SCHEMA_V1,
    HYDROLOGY_SUMMARY_SCHEMA_V1, MAX_HYDROLOGY_CONVEYANCE_SUMMARIES, MAX_HYDROLOGY_DELTAS,
    MAX_HYDROLOGY_TRANSFER_SUMMARIES, ObserverWorldSnapshot, validate_hydrology_carrier_key,
};
use causafera_observer_wire::{
    decode_field_raster, decode_observer_snapshot, decode_world_snapshot, encode_field_raster,
    encode_observer_snapshot, encode_world_snapshot,
};
use causafera_runtime::{Runtime, RuntimeConfig};

use support_hydrology::{enabled_runtime_config, wet_runtime_config};

/// A runtime advanced far enough that hydrology has committed several batches.
fn ticked(config: RuntimeConfig, ticks: u64) -> Runtime {
    let mut runtime = Runtime::new(config).expect("the runtime must bootstrap");
    for _ in 0..ticks {
        runtime.tick().expect("a hydrology tick must commit");
    }
    runtime
}

fn world_of(runtime: &Runtime) -> ObserverWorldSnapshot {
    runtime
        .observer_world_snapshot()
        .expect("the world projection must succeed")
}

#[test]
fn the_summary_totals_match_the_authoritative_field_set() {
    // Given: a world that has moved water for several ticks.
    let runtime = ticked(wet_runtime_config(), 4);
    let state = runtime.hydrology_state();

    // When: the observer summary is projected.
    let summary = runtime
        .snapshot()
        .expect("the runtime snapshot must succeed")
        .observer_snapshot()
        .hydrology;

    // Then: every total equals the sum over authoritative state, exactly.
    let (mut surface, mut soil, mut groundwater) = (0_u128, 0_u128, 0_u128);
    for field in state.fields.fields().values() {
        for cell in field.cells() {
            surface += u128::from(cell.surface_water().get());
            soil += u128::from(cell.soil_water().get());
            groundwater += u128::from(cell.groundwater().get());
        }
    }
    let conveyance: u128 = state
        .conveyance
        .edges()
        .values()
        .map(|edge| u128::from(edge.storage().get()))
        .sum();
    assert_eq!(summary.schema_version, HYDROLOGY_SUMMARY_SCHEMA_V1);
    assert_eq!(summary.total_surface, surface);
    assert_eq!(summary.total_soil, soil);
    assert_eq!(summary.total_groundwater, groundwater);
    assert_eq!(summary.total_conveyance, conveyance);
    assert!(
        surface + soil + groundwater > 0,
        "the fixture must hold water"
    );
    // A committed batch closes exactly, and the projection says so rather than
    // leaving the reader to assume it.
    assert_eq!(summary.latest_residual, 0);
    assert_eq!(
        summary.active_chunk_count as usize,
        state.active.active_chunks().len()
    );
}

#[test]
fn a_disabled_session_reports_a_present_summary_holding_nothing() {
    // Given: a session that never asked for water.
    let runtime = ticked(RuntimeConfig::new(4_242), 2);

    // When: the summary is projected.
    let summary = runtime
        .snapshot()
        .expect("the runtime snapshot must succeed")
        .observer_snapshot()
        .hydrology;

    // Then: it is present and zero. "This build has no hydrology" is a fact
    // about a payload's age; "this world holds no water" is a measurement, and
    // an observer must be able to tell them apart.
    assert_eq!(summary.schema_version, HYDROLOGY_SUMMARY_SCHEMA_V1);
    assert_eq!(summary.total_surface, 0);
    assert_eq!(summary.total_soil, 0);
    assert_eq!(summary.total_groundwater, 0);
    assert_eq!(summary.total_conveyance, 0);
    assert_eq!(summary.active_chunk_count, 0);
    assert!(summary.latest_forcing.is_none());
}

#[test]
fn the_latest_applied_forcing_record_is_named_with_its_accepted_totals() {
    // Given: the fixture schedules one record for tick three.
    let runtime = ticked(enabled_runtime_config(), 5);

    let summary = runtime
        .snapshot()
        .expect("the runtime snapshot must succeed")
        .observer_snapshot()
        .hydrology;

    // Then: the record is named with its own identity and the volumes its own
    // receipts accepted. The fixture runs five ticks and retention holds eight
    // batches, so the evidence is still there and the group must be present —
    // an absent group here would be a projection that lost it.
    let state = runtime.hydrology_state();
    let record = state
        .forcing
        .iter()
        .filter(|record| record.is_applied())
        .max_by_key(|record| record.key())
        .expect("the fixture's record applies at tick three");
    let projected = summary
        .latest_forcing
        .expect("its batch is still retained, so the group must be present");
    assert_eq!(projected.tick, record.scheduled_tick());
    assert_eq!(projected.forcing_id, record.forcing_id());
    assert_eq!(projected.origin_trace, record.origin_trace());
    // Accepted, not scheduled: the record asks for 9,000 and the projection
    // reports what the cells took.
    assert!(projected.accepted_source > 0);
    assert!(projected.accepted_source <= u128::from(record.precipitation_volume().get()));
    assert!(projected.accepted_et <= record.potential_et_volume().get());
}

#[test]
fn an_evicted_batch_takes_the_forcing_group_with_it() {
    // V33: eviction removes typed detail rather than leaving an unsourced
    // number behind. Retention holds eight whole batches, so a session run well
    // past that has no receipts left for tick three.
    let runtime = ticked(enabled_runtime_config(), 14);
    let state = runtime.hydrology_state();
    assert!(
        state
            .forcing
            .iter()
            .any(causafera_geography::HydrologyForcingRecord::is_applied),
        "the record still applied; only its receipts are gone"
    );
    assert!(
        state.retained_batches.len() <= 8,
        "retention holds at most eight whole batches"
    );

    let summary = runtime
        .snapshot()
        .expect("the runtime snapshot must succeed")
        .observer_snapshot()
        .hydrology;
    assert!(
        summary.latest_forcing.is_none(),
        "an identity beside fabricated zeroes would be worse than absence"
    );
}

#[test]
fn the_world_projection_is_bounded_and_internally_consistent() {
    let runtime = ticked(wet_runtime_config(), 3);
    let world = world_of(&runtime);

    assert!(
        !world.hydrology_deltas.is_empty(),
        "the fixture moves water"
    );
    assert!(world.hydrology_deltas.len() <= MAX_HYDROLOGY_DELTAS);
    assert!(world.hydrology_transfer_summaries.len() <= MAX_HYDROLOGY_TRANSFER_SUMMARIES);
    assert!(world.hydrology_conveyance_summaries.len() <= MAX_HYDROLOGY_CONVEYANCE_SUMMARIES);

    for summary in &world.hydrology_transfer_summaries {
        // Every projected key is one this build's own validator accepts, which
        // is what ties the producing encoder to the independent decoder.
        validate_hydrology_carrier_key(&summary.source_key).expect("a source key must be valid");
        validate_hydrology_carrier_key(&summary.target_key).expect("a target key must be valid");
        assert_eq!(
            summary.requested_volume - summary.accepted_volume,
            summary.unaccepted_volume,
            "a transfer's three volumes are one statement"
        );
        assert_ne!(
            summary.transfer_trace.raw(),
            0,
            "a summary must be anchored"
        );
    }
    for delta in &world.hydrology_deltas {
        assert_ne!(delta.transition_trace.raw(), 0);
        assert_ne!(delta.conservation_trace.raw(), 0);
    }
}

#[test]
fn the_world_projection_is_in_canonical_order_and_free_of_duplicates() {
    let runtime = ticked(wet_runtime_config(), 3);
    let world = world_of(&runtime);

    // Canonical cell order, strictly ascending: a repeat would be two answers
    // about one cell in one tick.
    let cells = world
        .hydrology_deltas
        .iter()
        .map(|delta| {
            (
                delta.chart_id,
                delta.chunk_x,
                delta.chunk_y,
                delta.chunk_z,
                delta.cell_ordinal,
            )
        })
        .collect::<Vec<_>>();
    assert!(cells.windows(2).all(|pair| pair[0] < pair[1]));

    let keys = world
        .hydrology_transfer_summaries
        .iter()
        .map(|summary| summary.canonical_key())
        .collect::<Vec<_>>();
    assert!(keys.windows(2).all(|pair| pair[0] < pair[1]));

    let edges = world
        .hydrology_conveyance_summaries
        .iter()
        .map(|summary| summary.edge_key.clone())
        .collect::<Vec<_>>();
    assert!(edges.windows(2).all(|pair| pair[0] < pair[1]));
}

#[test]
fn every_projected_tick_belongs_to_one_batch() {
    // The projection describes the latest retained batch, so a row from another
    // tick would be evidence read out of a ledger that did not produce it.
    let runtime = ticked(wet_runtime_config(), 4);
    let world = world_of(&runtime);
    let expected = world
        .hydrology_deltas
        .first()
        .map(|delta| delta.transition_tick)
        .expect("the fixture produces deltas");

    for delta in &world.hydrology_deltas {
        assert_eq!(delta.transition_tick, expected);
    }
    for summary in &world.hydrology_transfer_summaries {
        assert_eq!(summary.tick, expected);
    }
    for summary in &world.hydrology_conveyance_summaries {
        assert_eq!(summary.tick, expected);
    }
}

#[test]
fn an_engine_payload_roundtrips_through_the_wire_unchanged() {
    // V28's round-trip clause, run against a real engine payload rather than a
    // fixture: a projection the codecs cannot carry is not a projection.
    let runtime = ticked(wet_runtime_config(), 3);
    let summary = runtime
        .snapshot()
        .expect("the runtime snapshot must succeed")
        .observer_snapshot();
    let world = world_of(&runtime);

    let encoded_summary = encode_observer_snapshot(&summary);
    assert_eq!(
        decode_observer_snapshot(&encoded_summary).expect("the summary must decode"),
        summary
    );
    let encoded_world = encode_world_snapshot(&world);
    assert_eq!(
        decode_world_snapshot(&encoded_world).expect("the world must decode"),
        world
    );
}

#[test]
fn a_water_raster_carries_exact_unsigned_volumes_with_per_cell_provenance() {
    let runtime = ticked(wet_runtime_config(), 2);
    let state = runtime.hydrology_state();
    let chunk = *state
        .fields
        .fields()
        .keys()
        .next()
        .expect("the fixture is resident");

    for (kind, expected) in [
        (
            FieldRasterKind::HydrologySurfaceWater,
            state.fields.field(chunk).expect("resident").cells()[0]
                .surface_water()
                .get(),
        ),
        (
            FieldRasterKind::HydrologySoilWater,
            state.fields.field(chunk).expect("resident").cells()[0]
                .soil_water()
                .get(),
        ),
        (
            FieldRasterKind::HydrologyGroundwater,
            state.fields.field(chunk).expect("resident").cells()[0]
                .groundwater()
                .get(),
        ),
    ] {
        let raster = runtime
            .observer_field_raster(&FieldRasterRequest {
                chart_id: chunk.chart.raw(),
                chunk_x: chunk.chunk.x,
                chunk_y: chunk.chunk.y,
                chunk_z: chunk.chunk.z,
                field: kind,
                detail_level: 0,
            })
            .expect("the raster query must succeed")
            .expect("a resident chunk must project");

        assert_eq!(
            raster.unsigned_values_schema_version,
            HYDROLOGY_RASTER_VALUES_SCHEMA_V1
        );
        assert!(raster.values.is_empty() && raster.auxiliary.is_empty());
        assert_eq!(
            raster.unsigned_values.len(),
            (raster.edge * raster.edge * raster.depth) as usize
        );
        assert_eq!(raster.unsigned_values[0], expected);
        assert_eq!(raster.cell_traces.len(), raster.unsigned_values.len());

        let encoded = encode_field_raster(&raster);
        assert_eq!(
            decode_field_raster(&encoded).expect("the raster must decode"),
            raster
        );
    }
}

#[test]
fn observing_changes_no_authoritative_digest() {
    // V27: locale is not a runtime input at all, so the axes that remain are
    // query cadence, which projection is asked for, and whether anything
    // observes at all.
    let digests = |mut runtime: Runtime, observe: fn(&mut Runtime)| {
        for _ in 0..4 {
            runtime.tick().expect("a hydrology tick must commit");
            observe(&mut runtime);
        }
        let snapshot = runtime
            .snapshot()
            .expect("the runtime snapshot must succeed");
        (
            snapshot.physical_state_digest.bytes(),
            snapshot.history_digest.bytes(),
        )
    };

    let unobserved = digests(
        Runtime::new(wet_runtime_config()).expect("the runtime must bootstrap"),
        |_| {},
    );
    let summarized = digests(
        Runtime::new(wet_runtime_config()).expect("the runtime must bootstrap"),
        |runtime| {
            runtime.snapshot().expect("snapshot").observer_snapshot();
        },
    );
    let fully_observed = digests(
        Runtime::new(wet_runtime_config()).expect("the runtime must bootstrap"),
        |runtime| {
            for _ in 0..3 {
                runtime.snapshot().expect("snapshot").observer_snapshot();
                runtime.observer_world_snapshot().expect("world");
                runtime
                    .observer_field_raster(&FieldRasterRequest {
                        chart_id: 1,
                        chunk_x: 0,
                        chunk_y: 0,
                        chunk_z: 0,
                        field: FieldRasterKind::HydrologySurfaceWater,
                        detail_level: 0,
                    })
                    .expect("raster");
            }
        },
    );

    assert_eq!(unobserved, summarized);
    assert_eq!(unobserved, fully_observed);
}
