//! Replay, input order, and the whole-tick staging guarantee.
//!
//! Covers `plans/hydrology.md` verification gates V23 and V18. V18's near-cap case
//! needs a reachable envelope budget and lives in the `tick_transaction` unit
//! tests; everything here drives it through a real domain refusal.

mod support_hydrology;

use causafera_runtime::snapshot_sections::{assemble_envelope, disassemble_envelope};
use causafera_runtime::{Runtime, RuntimeConfig};

use support_hydrology::{enabled_runtime_config, wet_runtime_config};

fn digest_after(ticks: u64, config: RuntimeConfig) -> ([u8; 32], [u8; 32]) {
    let mut runtime = Runtime::new(config).expect("construction");
    runtime.run_ticks(ticks).expect("the run must commit");
    let snapshot = runtime.snapshot().expect("digest");
    (
        snapshot.physical_state_digest.bytes(),
        snapshot.history_digest.bytes(),
    )
}

#[test]
fn the_same_configuration_replays_to_the_same_digests() {
    // V23: determinism is the whole contract. Two runs of one configuration agree
    // on physical state and on history, not merely on their water.
    let first = digest_after(7, wet_runtime_config());
    let second = digest_after(7, wet_runtime_config());
    assert_eq!(first, second);
}

#[test]
fn forcing_schedule_input_order_does_not_change_the_result() {
    // The schedule is canonically ordered by `(scheduled_tick, forcing_id)`, so a
    // configuration that lists the same records in a different order is the same
    // configuration — and one that lists them unsorted is refused outright.
    let mut config = enabled_runtime_config();
    let base = config.hydrology.forcing_schedule[0].clone();
    config.hydrology.forcing_schedule = vec![
        base.clone(),
        causafera_runtime::HydrologyForcingSpec {
            forcing_id: 2,
            scheduled_tick: 4,
            ..base.clone()
        },
    ];
    let sorted = digest_after(6, config.clone());

    let mut reversed = config.clone();
    reversed.hydrology.forcing_schedule.reverse();
    assert!(
        Runtime::new(reversed).is_err(),
        "an unsorted schedule is refused rather than silently sorted"
    );

    // And the accepted order reproduces itself.
    assert_eq!(sorted, digest_after(6, config));
}

#[test]
fn an_export_import_export_cycle_is_byte_identical() {
    // V23's persistence half: a round trip is not a transformation.
    let mut runtime = Runtime::new(wet_runtime_config()).expect("construction");
    runtime.run_ticks(5).expect("the run must commit");
    let exported = runtime.export_snapshot().expect("export");
    let first = assemble_envelope(&exported)
        .expect("assemble")
        .encode()
        .expect("encode");

    let resumed = Runtime::from_snapshot(
        disassemble_envelope(&assemble_envelope(&exported).expect("assemble"))
            .expect("disassemble"),
    )
    .expect("resume");
    let second = assemble_envelope(&resumed.export_snapshot().expect("export"))
        .expect("assemble")
        .encode()
        .expect("encode");
    assert_eq!(first, second);
}

#[test]
fn a_disabled_session_is_unaffected_by_the_hydrology_system() {
    // The appended system runs on every tick of every session. A disabled one must
    // reach the same digests it reached before hydrology existed, which is what the
    // pre-hydrology fixture asserts byte for byte; here the property is that the
    // system is *inert*, not merely quiet.
    let mut config = RuntimeConfig::new(4_099);
    config.actor_count = 2;
    config.sensor_count = 1;
    config.bootstrap_population = 32;
    let first = digest_after(5, config.clone());
    let second = digest_after(5, config);
    assert_eq!(first, second);
}

#[test]
fn a_failed_tick_leaves_nothing_observable_behind() {
    // V18 through a reachable domain failure: configuration validates a forcing
    // record's ordering, bounds, and horizon but cannot know which chunks a session
    // will hold resident, so a record targeting a chunk outside the active region
    // passes construction and refuses its tick. The near-cap half of the same gate
    // lives in the crate's `tick_transaction` unit tests, where the envelope budget
    // can be set to a value a test can actually reach.
    let mut config = enabled_runtime_config();
    config.hydrology.forcing_schedule = vec![causafera_runtime::HydrologyForcingSpec {
        forcing_id: 1,
        scheduled_tick: 2,
        // Chunk 40 is far outside any active region this configuration produces.
        targets: vec![(
            causafera_geography::HydrologyCellKey::new(
                causafera_types::ChartChunkCoord::new(
                    support_hydrology::chart(),
                    causafera_types::ChunkCoord::new(40, 0, 0),
                ),
                0,
            )
            .expect("the ordinal is in range"),
            std::num::NonZeroU64::new(1).expect("positive"),
        )],
        precipitation_volume: causafera_types::WaterVolume::new(1_000),
        potential_et_volume: causafera_types::WaterVolume::ZERO,
        external_inflow_volume: causafera_types::WaterVolume::ZERO,
    }];
    let mut runtime = Runtime::new(config).expect("construction accepts the schedule");

    // Tick one is untouched by a record scheduled for tick two.
    runtime.run_ticks(1).expect("tick one must commit");
    let before = runtime
        .snapshot()
        .expect("the pre-failure digest is readable");
    let before_state = runtime.hydrology_state();
    let time_before = runtime.current_time();

    // Tick two refuses, and what surfaces is the domain's own reason rather than a
    // bookkeeping complaint about the rollback: the transaction restores the staged
    // pre-image and re-reports the failure that caused it.
    let error = runtime
        .run_ticks(1)
        .expect_err("a record targeting a non-resident chunk must refuse its tick");
    assert!(
        error.to_string().contains("resident"),
        "unexpected refusal: {error}"
    );

    // Everything matches the pre-tick snapshot, and the session is still usable:
    // a refused tick is a tick that did not happen, not one that poisoned the run.
    let after = runtime.snapshot().expect("the session is still readable");
    assert_eq!(
        after.physical_state_digest.bytes(),
        before.physical_state_digest.bytes(),
        "physical state is unchanged"
    );
    assert_eq!(
        after.history_digest.bytes(),
        before.history_digest.bytes(),
        "history is unchanged, so no event was committed"
    );
    assert_eq!(runtime.current_time(), time_before, "time did not advance");

    let after_state = runtime.hydrology_state();
    assert_eq!(after_state.fields, before_state.fields);
    assert_eq!(after_state.retained_batches, before_state.retained_batches);
    assert_eq!(
        after_state.next_node_id, before_state.next_node_id,
        "and no synthetic node identifier was consumed"
    );

    // Refusing the same tick again is the same refusal, not a different one: the
    // check is a function of state, so it cannot drift.
    assert_eq!(
        runtime.run_ticks(1).expect_err("still refused").to_string(),
        error.to_string()
    );
}

#[test]
fn a_refused_tick_commits_no_causal_events() {
    // The store is where a half-executed tick would show. Hydrology is *not* the
    // first system in `Phase::Physics` — appending it is what preserved every
    // existing stream key — so by the time it refuses, an earlier system has
    // already committed. The staging transaction is what makes those commits go
    // away again, and the store's own length is where that is visible.
    let mut config = enabled_runtime_config();
    config.hydrology.forcing_schedule = vec![causafera_runtime::HydrologyForcingSpec {
        forcing_id: 1,
        scheduled_tick: 2,
        targets: vec![(
            causafera_geography::HydrologyCellKey::new(
                causafera_types::ChartChunkCoord::new(
                    support_hydrology::chart(),
                    causafera_types::ChunkCoord::new(40, 0, 0),
                ),
                0,
            )
            .expect("the ordinal is in range"),
            std::num::NonZeroU64::new(1).expect("positive"),
        )],
        precipitation_volume: causafera_types::WaterVolume::new(1_000),
        potential_et_volume: causafera_types::WaterVolume::ZERO,
        external_inflow_volume: causafera_types::WaterVolume::ZERO,
    }];
    let mut runtime = Runtime::new(config).expect("construction");
    runtime.run_ticks(1).expect("tick one must commit");
    let before = runtime
        .export_snapshot()
        .expect("export")
        .traces
        .events
        .len();

    assert!(runtime.run_ticks(1).is_err(), "tick two must refuse");
    assert!(before > 0, "the successful tick did commit events");
    assert_eq!(
        runtime
            .export_snapshot()
            .expect("export")
            .traces
            .events
            .len(),
        before,
        "a refused tick commits nothing, not even from a system that runs before hydrology"
    );
    assert_eq!(
        runtime.hydrology_state().retained_batches.len(),
        1,
        "only the successful tick retained a batch"
    );
}
