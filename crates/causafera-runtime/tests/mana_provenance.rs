//! Every committed mana cell change must carry causal ancestry.
//!
//! A mana cell that changed for no recorded reason is authoritative state
//! without provenance, which is the one thing the field model's proposal/commit
//! boundary exists to prevent. The cross-chunk seam used to produce exactly
//! that: it attributed a delivered share to the two participants' previous
//! changes only, so a cell injected for the first time handed its share across
//! the seam with nothing to point at. `TODO-MANA-005`.

use causafera_core::Phase;
use causafera_runtime::{
    EXPERIMENT_RECIPE_MANA_SOURCE_EVENT_KIND, Runtime, RuntimeConfig, RuntimeSnapshotData,
    TerrainParticipation,
};

/// The production-shaped loop: actors contact material surfaces, contacts and
/// standing terrain feed the mana field, and the field spans several chunks so
/// the seam is exercised.
fn seamed_config(seed: u64) -> RuntimeConfig {
    let mut config = RuntimeConfig::new(seed);
    config.actor_count = 4;
    config.sensor_count = 2;
    config.bootstrap_population = 64;
    config.active_chunk_radius = 1;
    config
}

/// Mana-phase events that claim no cause.
///
/// The experiment-recipe source is the one mana commit entitled to be a root:
/// it introduces mana from outside the simulation by an immutable recorded
/// policy. Every other mana event must descend from something.
fn causeless_mana_events(snapshot: &RuntimeSnapshotData) -> Vec<u64> {
    snapshot
        .traces
        .events
        .iter()
        .filter(|event| {
            event.phase == Phase::Mana
                && event.kind.raw() != EXPERIMENT_RECIPE_MANA_SOURCE_EVENT_KIND
                && event.causes.is_empty()
        })
        .map(|event| event.trace_id.raw())
        .collect()
}

#[test]
fn no_mana_change_is_committed_without_a_cause() {
    // Given: a world whose mana field spans several chunks, so mana crosses
    // seams rather than only moving inside one field.
    let mut runtime = Runtime::new(seamed_config(7)).expect("world bootstraps");

    // When: it runs long enough for injection, diffusion and seam delivery to
    // have happened many times over.
    let summary = runtime.run_ticks(24).expect("world runs");
    let snapshot = runtime.export_snapshot().expect("world exports");

    // Then: the seam was actually exercised, and nothing changed for no reason.
    assert!(
        snapshot.mana.fields.len() > 1,
        "the seam must exist for this test to mean anything"
    );
    assert!(summary.mana_cell_changes > 0, "mana must have moved");
    assert_eq!(
        causeless_mana_events(&snapshot),
        Vec::<u64>::new(),
        "mana changed with no recorded cause"
    );
}

/// The defect was reachable with terrain inert as well as standing, so terrain
/// is not its source and neither configuration may reintroduce it.
#[test]
fn seam_provenance_does_not_depend_on_the_terrain_carrier() {
    for participation in [TerrainParticipation::Standing, TerrainParticipation::Inert] {
        let mut config = seamed_config(59);
        config.terrain_participation = participation;
        let mut runtime = Runtime::new(config).expect("world bootstraps");

        runtime.run_ticks(24).expect("world runs");
        let snapshot = runtime.export_snapshot().expect("world exports");

        assert_eq!(
            causeless_mana_events(&snapshot),
            Vec::<u64>::new(),
            "mana changed with no recorded cause under {participation:?} terrain"
        );
    }
}
