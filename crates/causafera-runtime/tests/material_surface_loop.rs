use std::collections::BTreeMap;

use causafera_runtime::{Runtime, RuntimeConfig, RuntimeSnapshotData};
use causafera_types::TraceId;

fn production_loop_config(seed: u64, mana_enabled: bool) -> RuntimeConfig {
    let mut config = RuntimeConfig::new(seed);
    config.actor_count = 1;
    config.sensor_count = 1;
    config.bootstrap_population = 8;
    config.mana_parameters.effect_threshold = if mana_enabled { 1 } else { 0 };
    config.mana_parameters.effect_hysteresis = 0;
    config
}

fn has_mana_material_consequence(snapshot: &RuntimeSnapshotData) -> bool {
    snapshot.material_surfaces.records.iter().any(|record| {
        record.surface.condition
            > i64::try_from(record.surface.contact_count)
                .expect("bounded integration contact count fits i64")
                .saturating_add(1)
    })
}

#[test]
fn actor_contact_material_surface_commits_causal_transition() {
    // Given: a production runtime with a causally bootstrapped actor and surface site.
    let mut runtime = Runtime::new(production_loop_config(971, true))
        .expect("production runtime bootstrap must succeed");

    // When: the scheduler executes perception, subjective-scene construction, and a later action.
    let snapshot = runtime.run_ticks(4).expect("production loop must execute");
    let exported = runtime
        .export_snapshot()
        .expect("completed production state must export");

    // Then: a bootstrapped material site records physical contact with a committed trace.
    assert!(snapshot.actor_actions_committed > 0);
    assert!(
        exported
            .material_surfaces
            .records
            .iter()
            .any(|record| record.surface.contact_count > 0)
    );
    assert!(exported.material_surfaces.records.iter().all(|record| {
        exported
            .traces
            .events
            .iter()
            .any(|event| event.trace_id == record.surface.last_transition)
    }));
}

#[test]
fn material_surface_loop_without_repetition_has_no_mana_material_consequence() {
    // Given: a production runtime with exactly one contact-derived carrier sample available.
    let mut runtime = Runtime::new(production_loop_config(972, true))
        .expect("production runtime bootstrap must succeed");

    // When: the scheduler reaches the first mana phase after one contact.
    runtime
        .run_ticks(2)
        .expect("single-contact production loop must execute");
    let exported = runtime
        .export_snapshot()
        .expect("single-contact production state must export");

    // Then: contact changes material, but no mana-mediated material transition exists yet.
    assert!(
        exported
            .material_surfaces
            .records
            .iter()
            .any(|record| record.surface.contact_count > 0)
    );
    assert!(!has_mana_material_consequence(&exported));
}

#[test]
fn material_surface_loop_replays_with_contact_and_mana_material_consequence() {
    // Given: two production runtimes with the same deterministic historical bootstrap.
    let config = production_loop_config(973, true);
    let mut first = Runtime::new(config.clone()).expect("first production runtime must bootstrap");
    let mut second = Runtime::new(config).expect("second production runtime must bootstrap");

    // When: both execute the repeated material loop.
    let first_snapshot = first.run_ticks(8).expect("first loop must execute");
    let second_snapshot = second.run_ticks(8).expect("second loop must execute");
    let first_export = first.export_snapshot().expect("first state must export");
    let second_export = second.export_snapshot().expect("second state must export");

    // Then: contact, mana-mediated material change, and causal history replay exactly.
    assert!(has_mana_material_consequence(&first_export));
    assert_eq!(first_snapshot, second_snapshot);
    assert_eq!(first_export, second_export);
}

#[test]
fn material_surface_loop_save_resume_after_contact_and_mana_material_consequence() {
    // Given: a production loop checkpointed after contact and a mana-mediated material change.
    let config = production_loop_config(974, true);
    let mut continuous =
        Runtime::new(config).expect("continuous production runtime must bootstrap");
    continuous
        .run_ticks(4)
        .expect("pre-checkpoint production loop must execute");
    let checkpoint = continuous
        .export_snapshot()
        .expect("contact-mutated production state must export");
    assert!(has_mana_material_consequence(&checkpoint));
    let mut resumed =
        Runtime::from_snapshot(checkpoint).expect("contact-mutated production state must resume");

    // When: both continuations execute the same remaining production ticks.
    let continuous_snapshot = continuous
        .run_ticks(4)
        .expect("continuous production loop must execute");
    let resumed_snapshot = resumed
        .run_ticks(4)
        .expect("resumed production loop must execute");

    // Then: authoritative material state and provenance remain equal.
    assert_eq!(continuous_snapshot, resumed_snapshot);
    assert_eq!(
        continuous
            .export_snapshot()
            .expect("continuous state must export"),
        resumed
            .export_snapshot()
            .expect("resumed state must export")
    );
}

#[test]
fn material_surface_loop_without_mana_has_no_material_consequence() {
    // Given: equivalent production loops with the mana-to-material threshold enabled or disabled.
    let mut enabled = Runtime::new(production_loop_config(975, true))
        .expect("mana-enabled production runtime must bootstrap");
    let mut disabled = Runtime::new(production_loop_config(975, false))
        .expect("mana-disabled production runtime must bootstrap");

    // When: both execute repeated material carrier samples.
    enabled
        .run_ticks(8)
        .expect("mana-enabled loop must execute");
    disabled
        .run_ticks(8)
        .expect("mana-disabled loop must execute");
    let enabled = enabled
        .export_snapshot()
        .expect("enabled state must export");
    let disabled = disabled
        .export_snapshot()
        .expect("disabled state must export");

    // Then: only the enabled loop has a material condition beyond physical contact.
    assert!(has_mana_material_consequence(&enabled));
    assert!(!has_mana_material_consequence(&disabled));
    assert_ne!(enabled.material_surfaces, disabled.material_surfaces);
}

#[test]
fn mana_material_consequence_changes_later_bounded_signal_scene_and_action() {
    // Given: equivalent production runtimes whose bounded material signals remain enabled.
    let mana_enabled_config = production_loop_config(979, true);
    let mana_disabled_config = production_loop_config(979, false);
    assert!(mana_enabled_config.material_surface_signals_enabled);
    assert!(mana_disabled_config.material_surface_signals_enabled);
    let mut mana_enabled =
        Runtime::new(mana_enabled_config).expect("mana-enabled production runtime must bootstrap");
    let mut mana_disabled = Runtime::new(mana_disabled_config)
        .expect("mana-disabled production runtime must bootstrap");

    // When: repeated contact reaches the Mana phase and later perception/cognition/action phases.
    mana_enabled
        .run_ticks(8)
        .expect("mana-enabled production loop must execute");
    mana_disabled
        .run_ticks(8)
        .expect("mana-disabled production loop must execute");
    let mana_enabled = mana_enabled
        .export_snapshot()
        .expect("mana-enabled state must export");
    let mana_disabled = mana_disabled
        .export_snapshot()
        .expect("mana-disabled state must export");

    // Then: the mana-mediated material condition changes an accessible physical signal and
    // produces a later subjective/action difference without exposing material identity.
    assert!(has_mana_material_consequence(&mana_enabled));
    assert!(!has_mana_material_consequence(&mana_disabled));
    assert_ne!(
        mana_enabled.actors_subjective,
        mana_disabled.actors_subjective
    );
    assert_ne!(
        mana_enabled.actors_objective.actors[0].1.body.energy,
        mana_disabled.actors_objective.actors[0].1.body.energy
    );
}

#[test]
fn material_surface_loop_parents_precede_children() {
    // Given: a production loop with repeated contact and mana-mediated material change.
    let mut runtime =
        Runtime::new(production_loop_config(976, true)).expect("production runtime must bootstrap");

    // When: the full causal loop executes.
    runtime.run_ticks(8).expect("production loop must execute");
    let exported = runtime
        .export_snapshot()
        .expect("production state must export");
    let positions = exported
        .traces
        .events
        .iter()
        .enumerate()
        .map(|(index, event)| (event.trace_id, index))
        .collect::<BTreeMap<TraceId, usize>>();

    // Then: every explicit provenance parent is committed before its child.
    assert!(has_mana_material_consequence(&exported));
    for (index, event) in exported.traces.events.iter().enumerate() {
        for parent in &event.causes {
            assert!(
                positions
                    .get(parent)
                    .is_some_and(|parent_index| *parent_index < index),
                "parent trace {} must precede child trace {}",
                parent.raw(),
                event.trace_id.raw()
            );
        }
    }
}

#[test]
fn material_surface_loop_retains_exact_committed_transition_history() {
    // Given: a production loop configured to reach a mana-mediated material transition.
    let mut runtime =
        Runtime::new(production_loop_config(977, true)).expect("production runtime must bootstrap");

    // When: contact, repeated structure, and mana execute through the production scheduler.
    runtime.run_ticks(4).expect("production loop must execute");
    let exported = runtime
        .export_snapshot()
        .expect("production state must export");

    // Then: typed before/after values and causal anchors are retained exactly as committed.
    assert!(
        exported
            .material_surfaces
            .transitions
            .iter()
            .any(|transition| {
                transition.mana_effect_trace.is_some()
                    && transition.mana_total > 0
                    && transition.after_condition == transition.before_condition.saturating_add(1)
                    && exported
                        .traces
                        .events
                        .iter()
                        .any(|event| Some(event.trace_id) == transition.contact_trace)
            })
    );
}

#[test]
fn material_surface_signal_access_changes_later_actor_action() {
    // Given: equivalent production runtimes differing only at the material physical-signal boundary.
    let mut accessible_config = production_loop_config(978, false);
    accessible_config.material_surface_signals_enabled = true;
    let mut suppressed_config = accessible_config.clone();
    suppressed_config.material_surface_signals_enabled = false;
    let mut accessible = Runtime::new(accessible_config)
        .expect("accessible-signal production runtime must bootstrap");
    let mut suppressed = Runtime::new(suppressed_config)
        .expect("suppressed-signal production runtime must bootstrap");

    // When: contact produces the material signal and a later scheduler action consumes perception.
    accessible
        .run_ticks(1)
        .expect("accessible-signal production loop must execute");
    suppressed
        .run_ticks(1)
        .expect("suppressed-signal production loop must execute");
    let accessible_checkpoint = accessible
        .export_snapshot()
        .expect("accessible-signal checkpoint must export");
    let suppressed_checkpoint = suppressed
        .export_snapshot()
        .expect("suppressed-signal checkpoint must export");
    assert!(
        accessible_checkpoint
            .recipe
            .config
            .material_surface_signals_enabled
    );
    assert!(
        !suppressed_checkpoint
            .recipe
            .config
            .material_surface_signals_enabled
    );
    let mut accessible = Runtime::from_snapshot(accessible_checkpoint)
        .expect("accessible-signal checkpoint must resume");
    let mut suppressed = Runtime::from_snapshot(suppressed_checkpoint)
        .expect("suppressed-signal checkpoint must resume");
    accessible
        .run_ticks(1)
        .expect("resumed accessible-signal production loop must execute");
    suppressed
        .run_ticks(1)
        .expect("resumed suppressed-signal production loop must execute");
    let accessible = accessible
        .export_snapshot()
        .expect("accessible-signal state must export");
    let suppressed = suppressed
        .export_snapshot()
        .expect("suppressed-signal state must export");

    // Then: public authoritative action results differ without exposing material identity to cognition.
    assert_ne!(
        accessible.actors_objective.actors[0].1.body.position,
        suppressed.actors_objective.actors[0].1.body.position
    );
    assert_ne!(accessible.actors_subjective, suppressed.actors_subjective);
}
