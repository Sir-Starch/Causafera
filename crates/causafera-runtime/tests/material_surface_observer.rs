use causafera_explanation::{
    ClaimEvidenceState, MATERIAL_SURFACE_LOOP_CLAIM_SCHEMA, MATERIAL_SURFACE_LOOP_WINDOW_SCHEMA,
};
use causafera_runtime::{Runtime, RuntimeConfig};

fn production_loop_config(seed: u64) -> RuntimeConfig {
    let mut config = RuntimeConfig::new(seed);
    config.actor_count = 1;
    config.sensor_count = 1;
    config.bootstrap_population = 8;
    config.mana_parameters.effect_threshold = 1;
    config.mana_parameters.effect_hysteresis = 0;
    config
}

#[test]
fn live_runtime_material_surface_loop_claim_is_traced_and_queryable() {
    // Given: the production historical bootstrap with repeated actor contact and mana enabled.
    let mut runtime = Runtime::new(production_loop_config(981))
        .expect("production runtime bootstrap must succeed");

    // When: the real scheduler completes the material/mana loop before a read-only query.
    runtime
        .run_ticks(4)
        .expect("production material loop must execute");
    let report = runtime
        .observer_material_surface_loop_explanation()
        .expect("live material loop explanation must be available");

    // Then: typed loop and observation-window claims retain their causal anchors.
    let claims = &report.frames[0].claims;
    let loop_claim = claims
        .iter()
        .find(|claim| claim.schema == MATERIAL_SURFACE_LOOP_CLAIM_SCHEMA)
        .expect("material-loop claim must be present");
    assert_eq!(loop_claim.evidence_state, ClaimEvidenceState::Supported);
    assert!(loop_claim.evidence_traces.len() >= 2);
    assert!(
        claims
            .iter()
            .any(|claim| claim.schema == MATERIAL_SURFACE_LOOP_WINDOW_SCHEMA)
    );
}

#[test]
fn live_runtime_material_surface_loop_claim_reports_insufficient_evidence() {
    // Given: the same production bootstrap before repeated structure can cause mana.
    let mut runtime = Runtime::new(production_loop_config(982))
        .expect("production runtime bootstrap must succeed");

    // When: one scheduler tick produces at most the initial actor contact.
    runtime
        .run_ticks(1)
        .expect("single-contact production loop must execute");
    let report = runtime
        .observer_material_surface_loop_explanation()
        .expect("live material loop explanation must be available");

    // Then: the loop claim remains explicitly insufficient instead of narrating a result.
    let loop_claim = report.frames[0]
        .claims
        .iter()
        .find(|claim| claim.schema == MATERIAL_SURFACE_LOOP_CLAIM_SCHEMA)
        .expect("material-loop claim must be present");
    assert_eq!(loop_claim.evidence_state, ClaimEvidenceState::Unknown);
    assert!(loop_claim.evidence_traces.is_empty());
}

#[test]
fn observer_transition_anchors_distinguish_bootstrap_from_actor_contact() {
    // Given: the production runtime immediately after bootstrap, before any actor action.
    let mut runtime = Runtime::new(production_loop_config(983))
        .expect("production runtime bootstrap must succeed");
    let bootstrap_world = runtime
        .observer_world_snapshot()
        .expect("bootstrap observer projection must succeed");
    let bootstrap_explanation = runtime
        .observer_material_surface_loop_explanation()
        .expect("bootstrap explanation must be available");

    // When: the scheduler reaches the first actor Action phase.
    runtime
        .run_ticks(1)
        .expect("first production action must execute");
    let contacted_world = runtime
        .observer_world_snapshot()
        .expect("contacted observer projection must succeed");
    let contacted_explanation = runtime
        .observer_material_surface_loop_explanation()
        .expect("contacted explanation must be available");

    // Then: bootstrap retains no claimed contact, while the committed action exposes one.
    assert!(
        bootstrap_world
            .material_surface_deltas
            .iter()
            .all(|delta| delta.contact_trace.is_none())
    );
    assert!(
        bootstrap_explanation.frames[0]
            .claims
            .iter()
            .all(|claim| claim.evidence_traces.is_empty())
    );
    assert!(
        contacted_world
            .material_surface_deltas
            .iter()
            .any(|delta| delta.contact_trace.is_some())
    );
    assert!(
        contacted_explanation.frames[0]
            .claims
            .iter()
            .any(|claim| { claim.evidence_traces.iter().any(|trace| trace.raw() != 0) })
    );
}

#[test]
fn bounded_history_retains_latest_mana_anchor_for_observer_and_explanation() {
    // Given: a production loop whose bounded material history will evict ordinary old contacts.
    let mut config = production_loop_config(984);
    config.actor_count = 16;
    config.bootstrap_population = 64;
    let mut runtime = Runtime::new(config).expect("production runtime bootstrap must succeed");

    // When: more than one complete material-transition history window executes.
    runtime
        .run_ticks(140)
        .expect("long production material loop must execute");
    let exported = runtime
        .export_snapshot()
        .expect("long production state must export");
    Runtime::from_snapshot(exported.clone())
        .expect("bounded production history must resume from its authoritative snapshot");
    let world = runtime
        .observer_world_snapshot()
        .expect("bounded observer projection must succeed");
    let report = runtime
        .observer_material_surface_loop_explanation()
        .expect("bounded explanation projection must succeed");

    // Then: the bounded newest observer window and live Explanation retain a traced mana result.
    assert_eq!(exported.material_surfaces.transitions.len(), 128);
    assert!(
        world
            .material_surface_deltas
            .iter()
            .any(|delta| delta.mana_effect_trace.is_some())
    );
    let loop_claim = report.frames[0]
        .claims
        .iter()
        .find(|claim| claim.schema == MATERIAL_SURFACE_LOOP_CLAIM_SCHEMA)
        .expect("material-loop claim must be present");
    assert_eq!(loop_claim.evidence_state, ClaimEvidenceState::Supported);
    assert!(loop_claim.evidence_traces.len() >= 2);
}
