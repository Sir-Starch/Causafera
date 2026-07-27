use causafera_explanation::{
    ClaimEvidenceState, DeterministicExplanationRenderer, MATERIAL_SURFACE_LOOP_CLAIM_SCHEMA,
    MATERIAL_SURFACE_LOOP_LOCAL_MANA_TRANSITION_SCHEMA,
    MATERIAL_SURFACE_LOOP_MANA_TRANSITION_SCHEMA, MATERIAL_SURFACE_LOOP_WINDOW_SCHEMA,
    NumericClaimValue, ObserverLocale,
};
use causafera_runtime::{
    EXPERIMENT_RECIPE_MANA_SOURCE_EVENT_KIND, EXPERIMENT_RECIPE_MANA_SOURCE_POLICY_SCHEMA_V1,
    ExperimentRecipeManaSource, ExperimentRecipeManaSourceRecipe, Runtime, RuntimeConfig,
    TerrainParticipation,
};
use causafera_types::{ChartChunkCoord, ChunkCoord};

fn production_loop_config(seed: u64) -> RuntimeConfig {
    let mut config = RuntimeConfig::new(seed);
    config.actor_count = 1;
    config.sensor_count = 1;
    config.bootstrap_population = 8;
    config.mana_parameters.effect_threshold = 1;
    config.mana_parameters.effect_hysteresis = 0;
    config
}

fn source_config(seed: u64, amount: i64, effect_threshold: i64) -> RuntimeConfig {
    let mut config = production_loop_config(seed);
    // Diffusion, decay and the terrain carrier are all held off so the cell the
    // recipe targets holds the recipe's contribution and nothing else. These
    // assertions are about one bounded source, not about the field the standing
    // ground sustains around it.
    config.mana_parameters.diffusion = 0;
    config.mana_parameters.decay = 0;
    config.terrain_participation = TerrainParticipation::Inert;
    config.mana_parameters.effect_threshold = effect_threshold;
    config.experiment_recipe_mana_sources = ExperimentRecipeManaSourceRecipe {
        records: vec![ExperimentRecipeManaSource {
            source_record_id: 1,
            enabled: true,
            scheduled_tick: 2,
            target_chunk: ChartChunkCoord::new(config.chart_id, ChunkCoord::new(0, 0, 0)),
            cell_index: 0,
            amount,
            per_record_maximum: 10,
            policy_schema_id: EXPERIMENT_RECIPE_MANA_SOURCE_POLICY_SCHEMA_V1,
        }],
        recipe_budget: amount,
    };
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

#[test]
fn observer_source_evidence_is_bounded_and_redacted() {
    // Given: an enabled source recipe that produces a material consequence through the live loop.
    let mut runtime = Runtime::new(source_config(985, 3, 1))
        .expect("source-enabled production runtime must bootstrap");

    // When: the production scheduler executes through the source and material-effect ticks.
    runtime
        .run_ticks(4)
        .expect("source-enabled material loop must execute");
    let world = runtime
        .observer_world_snapshot()
        .expect("source observer projection must succeed");
    let delta = world
        .material_surface_deltas
        .iter()
        .find(|delta| delta.mana_effect_trace.is_some())
        .expect("source-caused material delta must be retained");

    // Then: the world-facing evidence is bounded and contains only typed in-world evidence.
    assert!(world.material_surface_deltas.len() <= 64);
    assert_eq!(delta.transition_tick, 2);
    assert!(delta.mana_transition_trace.is_some());
    assert_eq!(delta.mana_before, Some(0));
    assert_eq!(delta.mana_after, Some(3));
    assert_eq!(world.material_surface_delta_schema_version, 3);
    assert_eq!(delta.local_mana_before, Some(0));
    assert_eq!(delta.local_mana_after, Some(3));
    assert!(delta.local_mana_transition_trace_id.is_some());
    let serialized = format!("{world:?}").to_ascii_lowercase();
    for forbidden in [
        "source_record_id",
        "recipe_hash",
        "policy_schema",
        "operator",
        "divine",
        "reward",
        "punishment",
        "worship",
    ] {
        assert!(
            !serialized.contains(forbidden),
            "observer leaked {forbidden}"
        );
    }
}

#[test]
fn explanation_source_evidence_is_typed_and_redacted() {
    // Given: an enabled source recipe that produces a material consequence through the live loop.
    let mut runtime = Runtime::new(source_config(986, 3, 1))
        .expect("source-enabled production runtime must bootstrap");

    // When: the production scheduler executes and the Explanation read model is requested.
    runtime
        .run_ticks(4)
        .expect("source-enabled material loop must execute");
    let report = runtime
        .observer_material_surface_loop_explanation()
        .expect("source Explanation projection must succeed");
    let world = runtime
        .observer_world_snapshot()
        .expect("source observer projection must succeed");
    let mana_trace = world
        .material_surface_deltas
        .iter()
        .find_map(|delta| delta.mana_transition_trace)
        .expect("source mana transition trace must be retained");
    let claim = report.frames[0]
        .claims
        .iter()
        .find(|claim| claim.schema == MATERIAL_SURFACE_LOOP_MANA_TRANSITION_SCHEMA)
        .expect("typed material mana-transition claim must be present");

    // Then: the typed claim is supported by the same in-world trace and values.
    assert_eq!(claim.evidence_state, ClaimEvidenceState::Supported);
    assert!(claim.evidence_traces.contains(&mana_trace));
    assert_eq!(claim.value, NumericClaimValue::Range { start: 0, end: 3 });
    let local_claim = report.frames[0]
        .claims
        .iter()
        .find(|claim| claim.schema == MATERIAL_SURFACE_LOOP_LOCAL_MANA_TRANSITION_SCHEMA)
        .expect("verified local mana claim must be present");
    assert_eq!(local_claim.evidence_state, ClaimEvidenceState::Supported);
    assert_eq!(
        local_claim.value,
        NumericClaimValue::Range { start: 0, end: 3 }
    );
    let rendered = DeterministicExplanationRenderer
        .render(&report, ObserverLocale::En)
        .text;
    let serialized = format!("{report:?}").to_ascii_lowercase();
    for forbidden in [
        "source_record_id",
        "recipe_hash",
        "policy_schema",
        "operator",
        "divine",
        "reward",
        "punishment",
        "worship",
    ] {
        assert!(!rendered.to_ascii_lowercase().contains(forbidden));
        assert!(!serialized.contains(forbidden));
    }
}

#[test]
fn below_threshold_source_changes_mana_without_material_consequence_or_supported_explanation() {
    // Given: a source-only runtime with the default material-effect threshold.
    let mut config = RuntimeConfig::new(987);
    config.experiment_recipe_mana_sources =
        source_config(987, 3, 4_096).experiment_recipe_mana_sources;
    let mut runtime = Runtime::new(config).expect("below-threshold source runtime must bootstrap");

    // When: the scheduler reaches the source's execution tick.
    runtime
        .run_ticks(2)
        .expect("below-threshold source must execute");
    let exported = runtime
        .export_snapshot()
        .expect("below-threshold source state must export");
    let report = runtime
        .observer_material_surface_loop_explanation()
        .expect("below-threshold Explanation projection must succeed");

    // Then: mana changes, but no material mana transition or supported material claim exists.
    let source_chunk =
        ChartChunkCoord::new(exported.recipe.config.chart_id, ChunkCoord::new(0, 0, 0));
    let source_field = exported
        .mana
        .fields
        .iter()
        .find(|field| field.chunk == source_chunk)
        .expect("source chunk mana field must be exported");
    // Unlike `source_config`, this test leaves terrain participation at its
    // default `Standing`, so cell 0 also carries the standing carrier's own
    // structure alongside the injected amount of 3. `TODO-GEO-004` made
    // surface material spatially coherent, which moves this incidental value
    // again (it was 182 before that change): coherent regions make many
    // columns share the same dominant material, activating the field's
    // spatial-repetition response channel where the old per-cell noise did
    // not. 154 is still far below the production effect threshold of 6 144
    // (`TODO-MANA-007`), which is what "below threshold" below asserts.
    assert_eq!(source_field.intensity[0], 154);
    assert!(
        exported
            .material_surfaces
            .transitions
            .iter()
            .all(|transition| transition.mana_effect_trace.is_none())
    );
    let material_claim = report.frames[0]
        .claims
        .iter()
        .find(|claim| claim.schema == MATERIAL_SURFACE_LOOP_CLAIM_SCHEMA)
        .expect("material-loop claim must be present");
    assert_eq!(material_claim.evidence_state, ClaimEvidenceState::Unknown);
    assert!(report.frames[0].claims.iter().all(|claim| {
        claim.schema != MATERIAL_SURFACE_LOOP_MANA_TRANSITION_SCHEMA
            || claim.evidence_state != ClaimEvidenceState::Supported
    }));
    assert!(exported.traces.events.iter().all(|event| {
        event.kind.raw() != EXPERIMENT_RECIPE_MANA_SOURCE_EVENT_KIND || event.causes.is_empty()
    }));
}
