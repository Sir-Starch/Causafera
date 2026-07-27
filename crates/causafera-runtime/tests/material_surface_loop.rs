use std::collections::BTreeMap;

use causafera_core::{Phase, StateFingerprint};
use causafera_persistence::SnapshotEnvelope;
use causafera_runtime::snapshot_sections::{
    EXPERIMENT_RECIPE_MANA_SOURCE_RECEIPTS_SECTION_MAJOR,
    SECTION_EXPERIMENT_RECIPE_MANA_SOURCE_RECEIPTS, assemble_envelope, disassemble_envelope,
};
use causafera_runtime::{
    ActionValidationResult, EXPERIMENT_RECIPE_MANA_SOURCE_EVENT_KIND,
    EXPERIMENT_RECIPE_MANA_SOURCE_OBJECT_KIND, EXPERIMENT_RECIPE_MANA_SOURCE_POLICY_SCHEMA_V1,
    EXPERIMENT_RECIPE_MANA_SOURCE_PROPERTY, ExperimentRecipeManaSource,
    ExperimentRecipeManaSourceRecipe, MAX_RUNTIME_TICKS, Runtime, RuntimeConfig,
    RuntimeSnapshotData, RuntimeState, TerrainParticipation,
};
use causafera_types::{ChartChunkCoord, ChunkCoord, SpatialChartId, TraceId};

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

fn valid_recipe_source(config: &RuntimeConfig) -> ExperimentRecipeManaSource {
    ExperimentRecipeManaSource {
        source_record_id: 1,
        enabled: true,
        scheduled_tick: 2,
        target_chunk: ChartChunkCoord::new(config.chart_id, ChunkCoord::new(0, 0, 0)),
        cell_index: u16::from(config.chunk_extent).pow(3) - 1,
        amount: 3,
        per_record_maximum: 10,
        policy_schema_id: EXPERIMENT_RECIPE_MANA_SOURCE_POLICY_SCHEMA_V1,
    }
}

fn source_events(snapshot: &RuntimeSnapshotData) -> Vec<&causafera_core::CausalEventSnapshot> {
    snapshot
        .traces
        .events
        .iter()
        .filter(|event| {
            event.phase == Phase::Mana
                && event.kind.raw() == EXPERIMENT_RECIPE_MANA_SOURCE_EVENT_KIND
        })
        .collect()
}

fn mana_intensity(snapshot: &RuntimeSnapshotData, chunk: ChartChunkCoord, cell_index: u16) -> i64 {
    snapshot
        .mana
        .fields
        .iter()
        .find(|field| field.chunk == chunk)
        .and_then(|field| field.intensity.get(usize::from(cell_index)))
        .copied()
        .expect("source target cell must be exported")
}

fn descendant_closure(snapshot: &RuntimeSnapshotData, root: TraceId) -> BTreeMap<TraceId, ()> {
    let mut closure = BTreeMap::from([(root, ())]);
    loop {
        let mut added = false;
        for event in &snapshot.traces.events {
            if closure.contains_key(&event.trace_id)
                || !event.causes.iter().any(|cause| closure.contains_key(cause))
            {
                continue;
            }
            closure.insert(event.trace_id, ());
            added = true;
        }
        if !added {
            return closure;
        }
    }
}

fn structurally_equivalent_traces(
    snapshots: (&RuntimeSnapshotData, &RuntimeSnapshotData),
    traces: (TraceId, TraceId),
) -> bool {
    let (sourced, control) = snapshots;
    let (sourced_trace, control_trace) = traces;
    if sourced_trace == control_trace {
        return true;
    }
    let Some(sourced_event) = sourced
        .traces
        .events
        .iter()
        .find(|event| event.trace_id == sourced_trace)
    else {
        return false;
    };
    let Some(control_event) = control
        .traces
        .events
        .iter()
        .find(|event| event.trace_id == control_trace)
    else {
        return false;
    };
    sourced_event.time == control_event.time
        && sourced_event.phase == control_event.phase
        && sourced_event.kind == control_event.kind
        && sourced_event.effects == control_event.effects
        && sourced_event.causes.len() == control_event.causes.len()
        && sourced_event.causes.iter().zip(&control_event.causes).all(
            |(sourced_cause, control_cause)| {
                structurally_equivalent_traces(snapshots, (*sourced_cause, *control_cause))
            },
        )
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
    // Given: a production runtime with exactly one contact-derived carrier sample
    // available, and no other physical source. The terrain carrier is held inert
    // so this measures the mana model's requirement for repetition rather than
    // the field the standing ground already sustains, which the companion test
    // below measures instead.
    let mut config = production_loop_config(972, true);
    config.terrain_participation = TerrainParticipation::Inert;
    let mut runtime = Runtime::new(config).expect("production runtime bootstrap must succeed");

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
fn standing_terrain_sustains_the_field_a_contact_lands_in() {
    // Given: the same production loop with the terrain carrier standing, and one
    // with it inert.
    let standing = production_loop_config(972, true);
    let mut inert = standing.clone();
    inert.terrain_participation = TerrainParticipation::Inert;

    // When: both reach the first mana phase, before any contact has repeated.
    let standing = Runtime::new(standing)
        .expect("standing runtime must bootstrap")
        .run_ticks(1)
        .expect("standing loop must execute");
    let inert = Runtime::new(inert)
        .expect("inert runtime must bootstrap")
        .run_ticks(1)
        .expect("inert loop must execute");

    // Then: the ground has already put mana in the world, and the empty world
    // has none. Terrain is world state that participates, not world state that
    // is merely stored.
    assert_eq!(inert.mana_total, 0);
    assert!(standing.mana_total > 0);
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

#[test]
fn suppressed_physical_signal_keeps_source_mana_material_traced_without_subjective_divergence() {
    // Given: same-seed runtimes with the physical-signal boundary suppressed, one with a source
    // at the existing material cell and one with no recipe records.
    let mut source_config = production_loop_config(990, true);
    source_config.material_surface_signals_enabled = false;
    source_config.mana_parameters.effect_threshold = 1;
    source_config.mana_parameters.diffusion = 0;
    source_config.mana_parameters.decay = 0;
    let mut source = valid_recipe_source(&source_config);
    source.target_chunk = ChartChunkCoord::new(source_config.chart_id, ChunkCoord::new(0, 0, 0));
    source.cell_index = 0;
    source.scheduled_tick = 2;
    source.amount = 3;
    source_config.experiment_recipe_mana_sources = ExperimentRecipeManaSourceRecipe {
        records: vec![source],
        recipe_budget: 3,
    };
    let mut control_config = source_config.clone();
    control_config.experiment_recipe_mana_sources = ExperimentRecipeManaSourceRecipe {
        records: Vec::new(),
        recipe_budget: 0,
    };
    let mut sourced =
        Runtime::new(source_config).expect("suppressed source runtime must bootstrap");
    let mut control =
        Runtime::new(control_config).expect("suppressed control runtime must bootstrap");

    // When: both runtimes execute through the source and downstream material-effect ticks.
    sourced
        .run_ticks(8)
        .expect("suppressed source runtime must execute");
    control
        .run_ticks(8)
        .expect("suppressed control runtime must execute");
    let sourced = sourced
        .export_snapshot()
        .expect("suppressed source state must export");
    let control = control
        .export_snapshot()
        .expect("suppressed control state must export");

    // Then: physical source, gate, and material events remain causally traced, while suppressing
    // their signal keeps actor-visible subjective and objective state equal.
    let source_event = source_events(&sourced)
        .into_iter()
        .next()
        .expect("suppressed source event must exist");
    assert_eq!(source_events(&sourced).len(), 1);
    assert_eq!(sourced.experiment_recipe_mana_source_receipts.len(), 1);
    let closure = descendant_closure(&sourced, source_event.trace_id);
    assert!(sourced.traces.events.iter().any(|event| {
        event.kind.raw() == 15
            && closure.contains_key(&event.trace_id)
            && event
                .effects
                .iter()
                .any(|effect| effect.target().property().raw() == 12)
    }));
    assert!(
        sourced
            .traces
            .events
            .iter()
            .any(|event| event.kind.raw() == 15 && closure.contains_key(&event.trace_id))
    );
    assert!(source_events(&control).is_empty());
    assert!(control.experiment_recipe_mana_source_receipts.is_empty());
    assert_eq!(sourced.actors_subjective, control.actors_subjective);
    let trace_difference_is_explained = |sourced_trace: TraceId, control_trace: TraceId| {
        sourced_trace == control_trace
            || (closure.contains_key(&sourced_trace) && closure.contains_key(&control_trace))
            || match (
                sourced
                    .traces
                    .events
                    .iter()
                    .find(|event| event.trace_id == sourced_trace),
                control
                    .traces
                    .events
                    .iter()
                    .find(|event| event.trace_id == control_trace),
            ) {
                (Some(_), Some(_)) => structurally_equivalent_traces(
                    (&sourced, &control),
                    (sourced_trace, control_trace),
                ),
                _ => false,
            }
    };
    let sourced_actor = &sourced.actors_objective.actors[0].1;
    let control_actor = &control.actors_objective.actors[0].1;
    assert_eq!(sourced_actor.body, control_actor.body);
    assert_eq!(sourced_actor.sensors, control_actor.sensors);
    assert_eq!(sourced_actor.features, control_actor.features);
    assert_eq!(sourced_actor.proposals, control_actor.proposals);
    assert_eq!(
        sourced_actor.validation_results.len(),
        control_actor.validation_results.len()
    );
    for (sourced_result, control_result) in sourced_actor
        .validation_results
        .iter()
        .zip(&control_actor.validation_results)
    {
        let explained = match (sourced_result, control_result) {
            (
                ActionValidationResult::Valid {
                    trace: sourced_trace,
                },
                ActionValidationResult::Valid {
                    trace: control_trace,
                },
            ) => trace_difference_is_explained(*sourced_trace, *control_trace),
            (
                ActionValidationResult::Invalid {
                    cause: sourced_cause,
                    trace: sourced_trace,
                },
                ActionValidationResult::Invalid {
                    cause: control_cause,
                    trace: control_trace,
                },
            ) => {
                sourced_cause == control_cause
                    && trace_difference_is_explained(*sourced_trace, *control_trace)
            }
            _ => false,
        };
        assert!(
            explained,
            "objective validation difference must be source-traced or structurally equivalent: sourced={sourced_result:?}, control={control_result:?}"
        );
    }
    assert_eq!(
        sourced.actors_objective.actor_ancestry.len(),
        control.actors_objective.actor_ancestry.len()
    );
    for ((sourced_id, sourced_traces), (control_id, control_traces)) in sourced
        .actors_objective
        .actor_ancestry
        .iter()
        .zip(&control.actors_objective.actor_ancestry)
    {
        assert_eq!(sourced_id, control_id);
        assert_eq!(sourced_traces.len(), control_traces.len());
        for (sourced_trace, control_trace) in sourced_traces.iter().zip(control_traces) {
            assert!(trace_difference_is_explained(
                *sourced_trace,
                *control_trace
            ));
        }
    }
    assert_eq!(
        sourced.actors_objective.actor_objects,
        control.actors_objective.actor_objects
    );
    assert_eq!(
        sourced.actors_objective.actor_action_bounds,
        control.actors_objective.actor_action_bounds
    );
}

#[test]
fn enabled_recipe_source_commits_once_and_drives_production_loop() {
    // Given: a source at the production surface cell with decay and diffusion disabled, and
    // terrain held inert, so the exported post-source intensity isolates the configured amount.
    // Terrain participation defaults to Standing, and a standing carrier's structure at this cell
    // is real ground state that a material-generation change can move (`TODO-GEO-004`); this test
    // is about the recipe source, not about what the standing carrier happens to read here.
    let mut config = production_loop_config(982, true);
    config.mana_parameters.effect_threshold = 1;
    config.mana_parameters.diffusion = 0;
    config.mana_parameters.decay = 0;
    config.terrain_participation = TerrainParticipation::Inert;
    let mut source = valid_recipe_source(&config);
    source.target_chunk = ChartChunkCoord::new(config.chart_id, ChunkCoord::new(0, 0, 0));
    source.cell_index = 0;
    source.scheduled_tick = 2;
    source.amount = 3;
    config.experiment_recipe_mana_sources = ExperimentRecipeManaSourceRecipe {
        records: vec![source.clone()],
        recipe_budget: source.amount,
    };
    let mut runtime = Runtime::new(config).expect("enabled source runtime must bootstrap");

    // When: the scheduler reaches the source tick, then continues through the material loop.
    runtime.run_ticks(2).expect("source tick must execute");
    let source_checkpoint = runtime
        .export_snapshot()
        .expect("source checkpoint must export");
    let source_receipts = runtime
        .executed_experiment_recipe_mana_sources()
        .expect("source receipts must be readable");
    runtime
        .run_ticks(6)
        .expect("source production loop must execute");
    let exported = runtime
        .export_snapshot()
        .expect("source production state must export");

    // Then: exactly one root source event changes the target by the configured amount and the
    // existing mana-to-material consequence still occurs later.
    let source_events_at_tick = source_events(&source_checkpoint);
    assert_eq!(source_events_at_tick.len(), 1);
    assert!(source_events_at_tick[0].causes.is_empty());
    assert_eq!(source_events_at_tick[0].effects.len(), 2);
    assert!(source_events_at_tick[0].effects.iter().all(|effect| {
        effect.target().object_kind().raw() == EXPERIMENT_RECIPE_MANA_SOURCE_OBJECT_KIND
            && effect.target().property().raw() == EXPERIMENT_RECIPE_MANA_SOURCE_PROPERTY
            && effect.before() != effect.after()
    }));
    assert_eq!(source_receipts.len(), 1);
    assert_eq!(source_receipts[0].source_record_id, source.source_record_id);
    assert_eq!(source_receipts[0].scheduled_tick, source.scheduled_tick);
    assert_eq!(source_receipts[0].executed_tick, source.scheduled_tick);
    assert_eq!(
        source_receipts[0].source_trace,
        source_events_at_tick[0].trace_id
    );
    assert_eq!(source_receipts[0].before_intensity, 0);
    assert_eq!(source_receipts[0].after_intensity, source.amount);
    assert_eq!(source_receipts[0].policy_schema_id, source.policy_schema_id);
    assert_eq!(
        source_receipts[0].recipe_hash,
        source_checkpoint
            .recipe
            .config
            .experiment_recipe_mana_sources
            .recipe_hash()
    );
    assert_eq!(
        mana_intensity(&source_checkpoint, source.target_chunk, source.cell_index),
        source.amount
    );
    assert_eq!(source_events(&exported).len(), 1);
    assert!(has_mana_material_consequence(&exported));
    let source_trace = source_events_at_tick[0].trace_id;
    let source_cell_object_id = source_events_at_tick[0]
        .effects
        .iter()
        .find(|effect| effect.target().object_id() != source.source_record_id)
        .expect("source event must include a cell effect")
        .target()
        .object_id();
    assert!(exported.traces.events.iter().any(|event| {
        event.kind.raw() == 3
            && event.time.raw() > source.scheduled_tick
            && event
                .effects
                .iter()
                .any(|effect| effect.target().object_id() == source_cell_object_id)
            && event.causes.contains(&source_trace)
    }));
}

#[test]
fn zero_amount_recipe_source_matches_control_without_source_commit() {
    // Given: a zero-amount source recipe and an otherwise identical control recipe.
    let control_config = production_loop_config(983, true);
    let mut source_config = control_config.clone();
    let mut source = valid_recipe_source(&source_config);
    source.amount = 0;
    source_config.experiment_recipe_mana_sources = ExperimentRecipeManaSourceRecipe {
        records: vec![source],
        recipe_budget: 0,
    };
    let mut source_runtime =
        Runtime::new(source_config).expect("zero source runtime must bootstrap");
    let mut control_runtime = Runtime::new(control_config).expect("control runtime must bootstrap");

    // When: both runtimes execute the same production ticks.
    let source_snapshot = source_runtime
        .run_ticks(8)
        .expect("zero source runtime must execute");
    let control_snapshot = control_runtime
        .run_ticks(8)
        .expect("control runtime must execute");
    let source_export = source_runtime
        .export_snapshot()
        .expect("zero source state must export");
    let control_export = control_runtime
        .export_snapshot()
        .expect("control state must export");

    // Then: zero amount is silent and leaves the production trajectory identical.
    assert_eq!(
        source_export.recipe.system_registrations[1].system_schema_id,
        19
    );
    assert!(source_events(&source_export).is_empty());
    assert!(
        source_export
            .traces
            .events
            .iter()
            .all(|event| event.kind.raw() != EXPERIMENT_RECIPE_MANA_SOURCE_EVENT_KIND)
    );
    assert!(
        source_runtime
            .executed_experiment_recipe_mana_sources()
            .expect("zero source receipts must be readable")
            .is_empty()
    );
    assert_eq!(
        source_snapshot.physical_state_digest,
        control_snapshot.physical_state_digest
    );
    assert_eq!(
        source_snapshot.history_digest,
        control_snapshot.history_digest
    );
    assert_eq!(
        source_snapshot.canonical_state,
        control_snapshot.canonical_state
    );
    assert_eq!(
        source_export.physical_counters.latest_mana_trace,
        control_export.physical_counters.latest_mana_trace
    );
    assert_eq!(source_export.mana, control_export.mana);
    assert_eq!(
        source_export.physical_counters,
        control_export.physical_counters
    );
}

#[test]
fn disabled_recipe_source_matches_control_without_source_commit() {
    // Given: a disabled source recipe and an otherwise identical control recipe.
    let control_config = production_loop_config(984, true);
    let mut source_config = control_config.clone();
    let mut source = valid_recipe_source(&source_config);
    source.enabled = false;
    source_config.experiment_recipe_mana_sources = ExperimentRecipeManaSourceRecipe {
        records: vec![source],
        recipe_budget: 0,
    };
    let mut source_runtime =
        Runtime::new(source_config).expect("disabled source runtime must bootstrap");
    let mut control_runtime = Runtime::new(control_config).expect("control runtime must bootstrap");

    // When: both runtimes execute the same production ticks.
    let source_snapshot = source_runtime
        .run_ticks(8)
        .expect("disabled source runtime must execute");
    let control_snapshot = control_runtime
        .run_ticks(8)
        .expect("control runtime must execute");
    let source_export = source_runtime
        .export_snapshot()
        .expect("disabled source state must export");
    let control_export = control_runtime
        .export_snapshot()
        .expect("control state must export");

    // Then: disabled input is silent and leaves the production trajectory identical.
    assert_eq!(
        source_export.recipe.system_registrations[1].system_schema_id,
        19
    );
    assert!(source_events(&source_export).is_empty());
    assert!(
        source_export
            .traces
            .events
            .iter()
            .all(|event| event.kind.raw() != EXPERIMENT_RECIPE_MANA_SOURCE_EVENT_KIND)
    );
    assert!(
        source_runtime
            .executed_experiment_recipe_mana_sources()
            .expect("disabled source receipts must be readable")
            .is_empty()
    );
    assert_eq!(
        source_snapshot.physical_state_digest,
        control_snapshot.physical_state_digest
    );
    assert_eq!(
        source_snapshot.history_digest,
        control_snapshot.history_digest
    );
    assert_eq!(
        source_snapshot.canonical_state,
        control_snapshot.canonical_state
    );
    assert_eq!(source_export.mana, control_export.mana);
    assert_eq!(
        source_export.physical_counters,
        control_export.physical_counters
    );
}

#[test]
fn scheduled_source_waits_until_tick_and_executes_once() {
    // Given: a source scheduled for tick three and a control with no source records.
    let control_config = production_loop_config(985, true);
    let mut source_config = control_config.clone();
    let mut source = valid_recipe_source(&source_config);
    source.target_chunk = ChartChunkCoord::new(source_config.chart_id, ChunkCoord::new(0, 0, 0));
    source.cell_index = 0;
    source.scheduled_tick = 3;
    source_config.experiment_recipe_mana_sources = ExperimentRecipeManaSourceRecipe {
        records: vec![source],
        recipe_budget: 3,
    };
    let mut source_runtime = Runtime::new(source_config).expect("scheduled source must bootstrap");
    let mut control_runtime = Runtime::new(control_config).expect("control runtime must bootstrap");

    // When: both runtimes advance through ticks one and two, then through tick three and beyond.
    for _ in 0..2 {
        let source_snapshot = source_runtime.tick().expect("source pre-tick must execute");
        let control_snapshot = control_runtime
            .tick()
            .expect("control pre-tick must execute");
        let source_export = source_runtime
            .export_snapshot()
            .expect("source pre-tick state must export");
        let control_export = control_runtime
            .export_snapshot()
            .expect("control pre-tick state must export");
        assert!(source_events(&source_export).is_empty());
        assert_eq!(
            source_snapshot.physical_state_digest,
            control_snapshot.physical_state_digest
        );
        assert_eq!(
            source_snapshot.history_digest,
            control_snapshot.history_digest
        );
        assert_eq!(
            source_snapshot.canonical_state,
            control_snapshot.canonical_state
        );
        assert_eq!(source_export.mana, control_export.mana);
        assert_eq!(
            source_export.physical_counters,
            control_export.physical_counters
        );
    }
    source_runtime
        .tick()
        .expect("scheduled source tick must execute");
    let at_source = source_runtime
        .export_snapshot()
        .expect("scheduled source state must export");
    assert_eq!(source_events(&at_source).len(), 1);
    assert_eq!(
        source_runtime
            .executed_experiment_recipe_mana_sources()
            .expect("scheduled source receipts must be readable")
            .len(),
        1
    );
    for _ in 0..5 {
        source_runtime
            .tick()
            .expect("post-source tick must execute");
    }
    let after = source_runtime
        .export_snapshot()
        .expect("post-source state must export");

    // Then: the source appears exactly at its schedule and never re-executes.
    assert_eq!(source_events(&after).len(), 1);
    assert_eq!(
        source_runtime
            .executed_experiment_recipe_mana_sources()
            .expect("post-source receipts must be readable")
            .len(),
        1
    );
}

#[test]
fn enabled_source_save_resume_equal_pre_and_post_source() {
    // Given: an enabled source scheduled for tick three and a continuous reference run.
    let mut config = production_loop_config(987, true);
    config.mana_parameters.diffusion = 0;
    config.mana_parameters.decay = 0;
    let mut source = valid_recipe_source(&config);
    source.target_chunk = ChartChunkCoord::new(config.chart_id, ChunkCoord::new(0, 0, 0));
    source.cell_index = 0;
    source.scheduled_tick = 3;
    source.amount = 3;
    config.experiment_recipe_mana_sources = ExperimentRecipeManaSourceRecipe {
        records: vec![source],
        recipe_budget: 3,
    };
    let mut continuous = Runtime::new(config.clone()).expect("continuous source must bootstrap");
    let continuous_snapshot = continuous
        .run_ticks(8)
        .expect("continuous source run must execute");
    let continuous_receipts = continuous
        .executed_experiment_recipe_mana_sources()
        .expect("continuous receipts must be readable");

    // When: equivalent branches resume from both before and after the source tick.
    for checkpoint_tick in [2, 5] {
        let mut branch = Runtime::new(config.clone()).expect("branch source must bootstrap");
        branch
            .run_ticks(checkpoint_tick)
            .expect("branch checkpoint must execute");
        let checkpoint = branch
            .export_snapshot()
            .expect("branch checkpoint must export");
        let mut resumed = Runtime::from_snapshot(checkpoint).expect("branch must resume");
        let resumed_snapshot = resumed
            .run_ticks(8 - checkpoint_tick)
            .expect("resumed source run must execute");
        let resumed_export = resumed
            .export_snapshot()
            .expect("resumed source state must export");

        // Then: both checkpoint positions produce exactly one persisted source receipt/event and
        // the same authoritative results as uninterrupted execution.
        assert_eq!(
            resumed_snapshot.physical_state_digest,
            continuous_snapshot.physical_state_digest
        );
        assert_eq!(
            resumed_snapshot.history_digest,
            continuous_snapshot.history_digest
        );
        assert_eq!(
            resumed_snapshot.canonical_state,
            continuous_snapshot.canonical_state
        );
        assert_eq!(
            resumed
                .executed_experiment_recipe_mana_sources()
                .expect("resumed receipts must be readable"),
            continuous_receipts
        );
        assert_eq!(source_events(&resumed_export).len(), 1);
    }
}

#[test]
fn enabled_source_same_seed_replays_exactly() {
    // Given: two independent runtimes with the same enabled source recipe and seed.
    let mut config = production_loop_config(988, true);
    config.mana_parameters.diffusion = 0;
    config.mana_parameters.decay = 0;
    let mut source = valid_recipe_source(&config);
    source.target_chunk = ChartChunkCoord::new(config.chart_id, ChunkCoord::new(0, 0, 0));
    source.cell_index = 0;
    source.scheduled_tick = 3;
    source.amount = 3;
    config.experiment_recipe_mana_sources = ExperimentRecipeManaSourceRecipe {
        records: vec![source],
        recipe_budget: 3,
    };
    let mut first = Runtime::new(config.clone()).expect("first source runtime must bootstrap");
    let mut second = Runtime::new(config).expect("second source runtime must bootstrap");

    // When: both runs advance through the source schedule and downstream causal loop.
    let first_snapshot = first.run_ticks(8).expect("first source run must execute");
    let second_snapshot = second.run_ticks(8).expect("second source run must execute");
    let first_receipts = first
        .executed_experiment_recipe_mana_sources()
        .expect("first receipts must be readable");
    let second_receipts = second
        .executed_experiment_recipe_mana_sources()
        .expect("second receipts must be readable");

    // Then: the complete digest tuple and source receipt are deterministic under the same seed.
    assert_eq!(
        first_snapshot.physical_state_digest,
        second_snapshot.physical_state_digest
    );
    assert_eq!(
        first_snapshot.history_digest,
        second_snapshot.history_digest
    );
    assert_eq!(
        first_snapshot.canonical_state,
        second_snapshot.canonical_state
    );
    assert_eq!(first_receipts, second_receipts);
    assert_eq!(first_snapshot.physical_state_digest.schema_version.raw(), 5);
}

#[test]
fn source_trace_precedes_derived_mana_material_and_signal_transitions() {
    // Given: an enabled source whose amount crosses the existing material-effect threshold.
    let mut config = production_loop_config(986, true);
    config.mana_parameters.effect_threshold = 1;
    config.mana_parameters.diffusion = 0;
    config.mana_parameters.decay = 0;
    let mut source = valid_recipe_source(&config);
    source.target_chunk = ChartChunkCoord::new(config.chart_id, ChunkCoord::new(0, 0, 0));
    source.cell_index = 0;
    source.scheduled_tick = 2;
    source.amount = 3;
    config.experiment_recipe_mana_sources = ExperimentRecipeManaSourceRecipe {
        records: vec![source.clone()],
        recipe_budget: source.amount,
    };
    let mut runtime = Runtime::new(config).expect("ancestry source runtime must bootstrap");

    // When: the source and the existing production path run long enough to produce descendants.
    runtime
        .run_ticks(8)
        .expect("ancestry production loop must execute");
    let exported = runtime
        .export_snapshot()
        .expect("ancestry state must export");
    let source_event = source_events(&exported)
        .into_iter()
        .next()
        .expect("one source event must exist");
    let closure = descendant_closure(&exported, source_event.trace_id);
    let source_cell_object_id = source_event
        .effects
        .iter()
        .find(|effect| effect.target().object_id() != source.source_record_id)
        .expect("source event must identify its changed cell")
        .target()
        .object_id();
    let positions = exported
        .traces
        .events
        .iter()
        .enumerate()
        .map(|(index, event)| (event.trace_id, index))
        .collect::<BTreeMap<TraceId, usize>>();

    // Then: all relevant derived event traces are descendants of the external source trace.
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
    let later_mana_events = exported.traces.events.iter().filter(|event| {
        event.kind.raw() == 3
            && event.time.raw() > source_event.time.raw()
            && event
                .effects
                .iter()
                .any(|effect| effect.target().object_id() == source_cell_object_id)
    });
    assert!(later_mana_events.clone().next().is_some());
    assert!(
        exported
            .traces
            .events
            .iter()
            .filter(|event| {
                event.kind.raw() == 3
                    && event.time.raw() > source_event.time.raw()
                    && event
                        .effects
                        .iter()
                        .any(|effect| effect.target().object_id() == source_cell_object_id)
            })
            .all(|event| closure.contains_key(&event.trace_id))
    );
    let mut gate_events = exported.traces.events.iter().filter(|event| {
        event.kind.raw() == 15
            && event
                .effects
                .iter()
                .any(|effect| effect.target().property().raw() == 12)
    });
    assert!(gate_events.clone().next().is_some());
    assert!(gate_events.all(|event| closure.contains_key(&event.trace_id)));
    let mut material_events = exported.traces.events.iter().filter(|event| {
        event.kind.raw() == 15
            && event
                .effects
                .iter()
                .any(|effect| effect.target().property().raw() == 11)
    });
    assert!(material_events.clone().next().is_some());
    assert!(material_events.all(|event| closure.contains_key(&event.trace_id)));
}

#[test]
fn invalid_recipe_sources_reject_before_authoritative_commit() {
    // Given: a production runtime configuration and one valid bounded source record.
    let config = production_loop_config(980, true);
    let valid = valid_recipe_source(&config);
    let mut too_many_records = Vec::with_capacity(17);
    for source_record_id in 1..=17 {
        let mut record = valid.clone();
        record.source_record_id = source_record_id;
        record.scheduled_tick = source_record_id;
        too_many_records.push(record);
    }
    let mut zero_id = valid.clone();
    zero_id.source_record_id = 0;
    let mut duplicate_id = valid.clone();
    duplicate_id.scheduled_tick = 3;
    let mut duplicate_key = valid.clone();
    duplicate_key.source_record_id = 2;
    let mut invalid_chart = valid.clone();
    invalid_chart.target_chunk =
        ChartChunkCoord::new(SpatialChartId::new(2), ChunkCoord::new(0, 0, 0));
    let mut inactive_chunk = valid.clone();
    inactive_chunk.target_chunk = ChartChunkCoord::new(config.chart_id, ChunkCoord::new(2, 0, 0));
    let mut invalid_cell = valid.clone();
    invalid_cell.cell_index = u16::from(config.chunk_extent).pow(3);
    let mut zero_tick = valid.clone();
    zero_tick.scheduled_tick = 0;
    let mut late_tick = valid.clone();
    late_tick.scheduled_tick = MAX_RUNTIME_TICKS + 1;
    let mut negative_amount = valid.clone();
    negative_amount.amount = -1;
    let mut excessive_amount = valid.clone();
    excessive_amount.amount = 11;
    let mut negative_maximum = valid.clone();
    negative_maximum.per_record_maximum = -1;
    let negative_budget = valid.clone();
    let mut over_budget_amount = valid.clone();
    over_budget_amount.source_record_id = 2;
    over_budget_amount.scheduled_tick = 3;
    let mut invalid_policy = valid.clone();
    invalid_policy.policy_schema_id = EXPERIMENT_RECIPE_MANA_SOURCE_POLICY_SCHEMA_V1 + 1;

    let cases = [
        (
            "too many records",
            ExperimentRecipeManaSourceRecipe {
                records: too_many_records,
                recipe_budget: 100,
            },
        ),
        (
            "zero source record ID",
            ExperimentRecipeManaSourceRecipe {
                records: vec![zero_id],
                recipe_budget: 10,
            },
        ),
        (
            "duplicate source record ID",
            ExperimentRecipeManaSourceRecipe {
                records: vec![valid.clone(), duplicate_id],
                recipe_budget: 20,
            },
        ),
        (
            "duplicate canonical key",
            ExperimentRecipeManaSourceRecipe {
                records: vec![valid.clone(), duplicate_key],
                recipe_budget: 20,
            },
        ),
        (
            "zero scheduled tick",
            ExperimentRecipeManaSourceRecipe {
                records: vec![zero_tick],
                recipe_budget: 10,
            },
        ),
        (
            "scheduled tick exceeds runtime limit",
            ExperimentRecipeManaSourceRecipe {
                records: vec![late_tick],
                recipe_budget: 10,
            },
        ),
        (
            "negative amount",
            ExperimentRecipeManaSourceRecipe {
                records: vec![negative_amount],
                recipe_budget: 10,
            },
        ),
        (
            "amount exceeds per-record maximum",
            ExperimentRecipeManaSourceRecipe {
                records: vec![excessive_amount],
                recipe_budget: 20,
            },
        ),
        (
            "negative per-record maximum",
            ExperimentRecipeManaSourceRecipe {
                records: vec![negative_maximum],
                recipe_budget: 10,
            },
        ),
        (
            "negative recipe budget",
            ExperimentRecipeManaSourceRecipe {
                records: vec![negative_budget],
                recipe_budget: -1,
            },
        ),
        (
            "enabled amount exceeds recipe budget",
            ExperimentRecipeManaSourceRecipe {
                records: vec![valid.clone(), over_budget_amount],
                recipe_budget: 5,
            },
        ),
        (
            "invalid policy schema",
            ExperimentRecipeManaSourceRecipe {
                records: vec![invalid_policy],
                recipe_budget: 10,
            },
        ),
        (
            "target chart differs from configuration",
            ExperimentRecipeManaSourceRecipe {
                records: vec![invalid_chart],
                recipe_budget: 10,
            },
        ),
        (
            "target chunk is inactive",
            ExperimentRecipeManaSourceRecipe {
                records: vec![inactive_chunk],
                recipe_budget: 10,
            },
        ),
        (
            "cell is outside configured extent",
            ExperimentRecipeManaSourceRecipe {
                records: vec![invalid_cell],
                recipe_budget: 10,
            },
        ),
    ];

    // When: each malformed recipe is presented to Runtime::new before scheduler construction.
    for (label, recipe) in cases {
        let mut malformed_config = config.clone();
        malformed_config.experiment_recipe_mana_sources = recipe;

        // Then: every malformed recipe is rejected before authoritative runtime creation.
        assert!(
            Runtime::new(malformed_config).is_err(),
            "malformed recipe case must reject: {label}"
        );
    }

    let mut valid_config = config;
    valid_config.experiment_recipe_mana_sources = ExperimentRecipeManaSourceRecipe {
        records: vec![valid],
        recipe_budget: 10,
    };
    assert!(Runtime::new(valid_config).is_ok());
}

#[test]
fn equivalent_source_input_order_has_equal_digests_and_receipts() {
    // Given: equivalent valid source records supplied in two different input orders.
    let base_config = production_loop_config(981, true);
    let mut first_record = valid_recipe_source(&base_config);
    first_record.source_record_id = 10;
    first_record.scheduled_tick = 2;
    let mut second_record = valid_recipe_source(&base_config);
    second_record.source_record_id = 20;
    second_record.scheduled_tick = 4;

    let mut first_config = base_config.clone();
    first_config.experiment_recipe_mana_sources = ExperimentRecipeManaSourceRecipe {
        records: vec![first_record.clone(), second_record.clone()],
        recipe_budget: 20,
    };
    let mut second_config = base_config;
    second_config.experiment_recipe_mana_sources = ExperimentRecipeManaSourceRecipe {
        records: vec![second_record, first_record],
        recipe_budget: 20,
    };
    let mut first = Runtime::new(first_config).expect("first recipe must validate");
    let mut second = Runtime::new(second_config).expect("second recipe must validate");

    // When: both validated runtimes execute the same deterministic ticks.
    let first_snapshot = first.run_ticks(8).expect("first recipe loop must execute");
    let second_snapshot = second
        .run_ticks(8)
        .expect("second recipe loop must execute");
    let first_recipe = first
        .export_snapshot()
        .expect("first runtime state must export")
        .recipe;
    let second_recipe = second
        .export_snapshot()
        .expect("second runtime state must export")
        .recipe;
    let first_receipts = first
        .executed_experiment_recipe_mana_sources()
        .expect("first source receipts must be readable");
    let second_receipts = second
        .executed_experiment_recipe_mana_sources()
        .expect("second source receipts must be readable");

    // Then: canonical validation and digest inputs are independent of source input order.
    assert_eq!(first_recipe.config, second_recipe.config);
    assert_eq!(
        first_recipe
            .config
            .experiment_recipe_mana_sources
            .recipe_hash(),
        second_recipe
            .config
            .experiment_recipe_mana_sources
            .recipe_hash()
    );
    assert_eq!(first_snapshot, second_snapshot);
    assert_eq!(first_receipts, second_receipts);
}

#[test]
fn authoritative_recipe_source_sections_are_versioned_and_fail_closed() {
    // Given: a complete current snapshot envelope containing an executed source receipt.
    let mut config = production_loop_config(55, true);
    let mut source = valid_recipe_source(&config);
    source.scheduled_tick = 1;
    config.experiment_recipe_mana_sources = ExperimentRecipeManaSourceRecipe {
        records: vec![source],
        recipe_budget: 3,
    };
    let mut runtime = Runtime::new(config).expect("runtime must bootstrap");
    runtime.run_ticks(1).expect("runtime must execute");
    let data = runtime.export_snapshot().expect("snapshot must export");
    let envelope = assemble_envelope(&data).expect("snapshot envelope must assemble");

    // When: the dedicated source-receipt section is round-tripped through the envelope.
    assert!(
        envelope
            .sections
            .contains_key(&u64::from(SECTION_EXPERIMENT_RECIPE_MANA_SOURCE_RECEIPTS))
    );
    assert_eq!(
        envelope.sections[&u64::from(SECTION_EXPERIMENT_RECIPE_MANA_SOURCE_RECEIPTS)].section_major,
        EXPERIMENT_RECIPE_MANA_SOURCE_RECEIPTS_SECTION_MAJOR
    );
    let encoded = envelope.encode().expect("snapshot envelope must encode");
    let decoded = SnapshotEnvelope::decode(&encoded).expect("snapshot envelope must decode");
    let restored = disassemble_envelope(&decoded).expect("source section must decode");

    // Then: the receipt section is preserved and unsupported or missing required versions fail
    // closed.
    assert_eq!(
        restored.experiment_recipe_mana_source_receipts,
        data.experiment_recipe_mana_source_receipts
    );
    let mut unsupported_zero = decoded.clone();
    unsupported_zero
        .sections
        .get_mut(&u64::from(SECTION_EXPERIMENT_RECIPE_MANA_SOURCE_RECEIPTS))
        .expect("source receipt section must exist")
        .section_major = 0;
    let mut unsupported_two = decoded.clone();
    unsupported_two
        .sections
        .get_mut(&u64::from(SECTION_EXPERIMENT_RECIPE_MANA_SOURCE_RECEIPTS))
        .expect("source receipt section must exist")
        .section_major = 2;
    let mut missing = decoded;
    missing
        .sections
        .remove(&u64::from(SECTION_EXPERIMENT_RECIPE_MANA_SOURCE_RECEIPTS));
    assert!(disassemble_envelope(&unsupported_zero).is_err());
    assert!(disassemble_envelope(&unsupported_two).is_err());
    assert!(disassemble_envelope(&missing).is_err());
}

#[test]
fn runtime_state_import_rejects_invalid_source_receipt_correspondence() {
    // Given: a real completed source execution with its persisted receipts and source events.
    let mut config = RuntimeConfig::new(57);
    let source = ExperimentRecipeManaSource {
        source_record_id: 1,
        enabled: true,
        scheduled_tick: 2,
        target_chunk: ChartChunkCoord::new(config.chart_id, ChunkCoord::new(0, 0, 0)),
        cell_index: 0,
        amount: 3,
        per_record_maximum: 3,
        policy_schema_id: EXPERIMENT_RECIPE_MANA_SOURCE_POLICY_SCHEMA_V1,
    };
    let mut second_source = source.clone();
    second_source.source_record_id = 2;
    second_source.scheduled_tick = 3;
    second_source.cell_index = 1;
    config.experiment_recipe_mana_sources = ExperimentRecipeManaSourceRecipe {
        records: vec![source, second_source],
        recipe_budget: 6,
    };
    let mut runtime = Runtime::new(config).expect("source runtime must bootstrap");
    runtime.run_ticks(3).expect("source runtime must execute");
    let data = runtime
        .export_snapshot()
        .expect("source runtime snapshot must export");

    let assert_rejected = |invalid: RuntimeSnapshotData| {
        assert!(RuntimeState::import_snapshot(invalid).is_err());
    };

    // When: each receipt/event/config correspondence is independently malformed.
    let mut unsorted = data.clone();
    unsorted.experiment_recipe_mana_source_receipts.swap(0, 1);
    assert_rejected(unsorted);

    let mut duplicate = data.clone();
    duplicate.experiment_recipe_mana_source_receipts[1].source_record_id =
        duplicate.experiment_recipe_mana_source_receipts[0].source_record_id;
    assert_rejected(duplicate);

    let mut unknown_record = data.clone();
    unknown_record.experiment_recipe_mana_source_receipts[0].source_record_id = 99;
    assert_rejected(unknown_record);

    let mut wrong_executed_tick = data.clone();
    wrong_executed_tick.experiment_recipe_mana_source_receipts[0].executed_tick += 1;
    assert_rejected(wrong_executed_tick);

    let mut disabled_record = data.clone();
    disabled_record
        .recipe
        .config
        .experiment_recipe_mana_sources
        .records[0]
        .enabled = false;
    assert_rejected(disabled_record);

    let mut non_source_trace = data.clone();
    non_source_trace.experiment_recipe_mana_source_receipts[0].source_trace =
        non_source_trace.traces.events[0].trace_id;
    assert_rejected(non_source_trace);

    let mut tampered_before = data.clone();
    tampered_before.experiment_recipe_mana_source_receipts[0].before_intensity += 1;
    assert_rejected(tampered_before);

    let mut tampered_recipe_hash = data;
    tampered_recipe_hash.experiment_recipe_mana_source_receipts[0].recipe_hash =
        StateFingerprint::new([0; 32]);
    assert_rejected(tampered_recipe_hash);
}
