//! The canonical production bootstrap record.
//!
//! Every assertion here is about the contract in `causafera-world`
//! (`HistoricalBootstrapPlan` / `HistoricalStageReceipt`) as the runtime's six
//! executable stages actually satisfy it: one canonical plan, one terminal
//! receipt per stage including stages with no domain effect, and a record that
//! survives export, import, and resume unchanged.

use std::collections::{BTreeMap, BTreeSet};

use causafera_core::Phase;
use causafera_runtime::{
    ACTOR_PROMOTION_PROCESS_SCHEMA, BOOTSTRAP_STAGE_COMPLETION_EVENT_KIND, BOOTSTRAP_STAGE_COUNT,
    BOOTSTRAP_STAGE_OBJECT_KIND, BOOTSTRAP_STAGE_RESULT_PROPERTY, MATERIAL_ACTIVITY_PROCESS_SCHEMA,
    MATERIAL_SURFACE_PROCESS_SCHEMA, PHYSICAL_GEOGRAPHY_PROCESS_SCHEMA,
    POPULATION_LIFECYCLE_PROCESS_SCHEMA, Runtime, RuntimeConfig, RuntimeError, RuntimeSnapshotData,
    RuntimeState, THERMAL_RESERVOIR_PROCESS_SCHEMA, assemble_envelope, decode_population_section,
    disassemble_envelope, encode_population_section,
};
use causafera_types::{HistoricalProcessSchemaId, HistoricalStageId, SimulationTime, TraceId};

/// The bounded envelope the observer session already uses: nine active chunks,
/// a real bootstrap population, and promoted actors.
fn populated_config(seed: u64) -> RuntimeConfig {
    let mut config = RuntimeConfig::new(seed);
    config.active_chunk_radius = 1;
    config.active_chunk_shape = causafera_runtime::ActiveChunkShape::Area;
    config.bootstrap_population = 512;
    config.actor_count = 8;
    config.sensor_count = 1;
    config
}

/// The default configuration, whose population, promotion, and material activity
/// stages commit no domain effect at all.
fn empty_stage_config(seed: u64) -> RuntimeConfig {
    RuntimeConfig::new(seed)
}

fn snapshot_of(config: RuntimeConfig) -> RuntimeSnapshotData {
    Runtime::new(config)
        .expect("production bootstrap must succeed")
        .export_snapshot()
        .expect("bootstrap state must export")
}

fn completion_event(
    data: &RuntimeSnapshotData,
    trace: TraceId,
) -> &causafera_core::CausalEventSnapshot {
    data.traces
        .events
        .iter()
        .find(|event| event.trace_id == trace)
        .expect("every receipt trace must exist in the causal trace store")
}

#[test]
fn production_bootstrap_declares_one_canonical_six_stage_plan() {
    // Given: a runtime built through the production path only.
    let data = snapshot_of(populated_config(4_101));
    let plan = &data.bootstrap.plan;

    // Then: the plan carries the six current stages with stable numeric identity.
    assert_eq!(plan.stages.len(), BOOTSTRAP_STAGE_COUNT);
    assert_eq!(plan.world_seed, 4_101);
    let expected_processes = [
        PHYSICAL_GEOGRAPHY_PROCESS_SCHEMA,
        MATERIAL_SURFACE_PROCESS_SCHEMA,
        POPULATION_LIFECYCLE_PROCESS_SCHEMA,
        ACTOR_PROMOTION_PROCESS_SCHEMA,
        MATERIAL_ACTIVITY_PROCESS_SCHEMA,
        THERMAL_RESERVOIR_PROCESS_SCHEMA,
    ];
    for (ordinal, stage) in plan.stages.iter().enumerate() {
        assert_eq!(stage.stage, HistoricalStageId::new(ordinal as u64 + 1));
        assert_eq!(stage.process, expected_processes[ordinal]);

        // And: the canonical timeline is six non-overlapping one-unit intervals.
        assert_eq!(stage.starts_at, SimulationTime::new(ordinal as u64));
        assert_eq!(stage.ends_at, SimulationTime::new(ordinal as u64 + 1));

        // And: the dependency graph is the current ordered chain.
        let expected_dependencies = if ordinal == 0 {
            Vec::new()
        } else {
            vec![HistoricalStageId::new(ordinal as u64)]
        };
        assert_eq!(stage.dependencies, expected_dependencies);
        assert!(stage.external_causes.is_empty());

        // And: targets are the sorted, distinct active chunk identities.
        assert_eq!(
            stage.targets.len(),
            9,
            "nine active chunks in the Area chart"
        );
        assert!(
            stage.targets.windows(2).all(|pair| pair[0] < pair[1]),
            "stage targets must be strictly sorted"
        );
    }

    // And: process identity is opaque and numeric, never a named event.
    assert!(
        expected_processes
            .iter()
            .all(|process| *process != HistoricalProcessSchemaId::new(0))
    );
}

#[test]
fn every_stage_emits_exactly_one_terminal_receipt() {
    // Given: a populated production bootstrap.
    let data = snapshot_of(populated_config(4_102));

    // Then: there are exactly six receipts, strictly ordered by stage.
    assert_eq!(data.bootstrap.receipts.len(), BOOTSTRAP_STAGE_COUNT);
    assert!(
        data.bootstrap
            .receipts
            .windows(2)
            .all(|pair| pair[0].stage < pair[1].stage)
    );

    // And: each receipt closes its stage at the canonical stage end.
    for (stage, receipt) in data
        .bootstrap
        .plan
        .stages
        .iter()
        .zip(&data.bootstrap.receipts)
    {
        assert_eq!(receipt.stage, stage.stage);
        assert_eq!(receipt.completed_at, stage.ends_at);
    }

    // And: no two receipts reuse a completion trace.
    let traces = data
        .bootstrap
        .receipts
        .iter()
        .map(|receipt| receipt.trace)
        .collect::<BTreeSet<_>>();
    assert_eq!(traces.len(), BOOTSTRAP_STAGE_COUNT);

    // And: each receipt's causes are exactly its declared dependency ancestry.
    let mut previous: Option<TraceId> = None;
    for receipt in &data.bootstrap.receipts {
        assert_eq!(receipt.causes, previous.into_iter().collect::<Vec<_>>());
        previous = Some(receipt.trace);
    }
}

#[test]
fn a_completion_is_a_real_bounded_state_transition_not_metadata() {
    // Given: a populated production bootstrap.
    let data = snapshot_of(populated_config(4_103));

    for receipt in &data.bootstrap.receipts {
        let event = completion_event(&data, receipt.trace);

        // Then: the completion is a Lifecycle-phase bootstrap completion event.
        assert_eq!(event.phase, Phase::Lifecycle);
        assert_eq!(event.kind.raw(), BOOTSTRAP_STAGE_COMPLETION_EVENT_KIND);

        // And: its single effect transitions this stage's bounded result state to
        // the receipt's result fingerprint.
        assert_eq!(event.effects.len(), 1);
        let effect = event.effects[0];
        assert_eq!(
            effect.target().object_kind().raw(),
            BOOTSTRAP_STAGE_OBJECT_KIND
        );
        assert_eq!(effect.target().object_id(), receipt.stage.raw());
        assert_eq!(
            effect.target().property().raw(),
            BOOTSTRAP_STAGE_RESULT_PROPERTY
        );
        assert_eq!(effect.after(), receipt.result);
        assert_ne!(effect.before(), effect.after());
    }

    // And: the materialized stage-result state agrees with every receipt.
    let results = data
        .bootstrap
        .stage_results
        .iter()
        .map(|entry| (entry.stage, entry.result))
        .collect::<BTreeMap<_, _>>();
    assert_eq!(results.len(), BOOTSTRAP_STAGE_COUNT);
    for receipt in &data.bootstrap.receipts {
        assert_eq!(results.get(&receipt.stage), Some(&receipt.result));
    }
}

#[test]
fn a_completion_carries_both_its_stage_effects_and_the_previous_receipt() {
    // Given: a populated bootstrap, whose stages all commit domain effects.
    let data = snapshot_of(populated_config(4_104));

    // The stage effects a completion must name are every non-completion bootstrap
    // event committed between the previous completion and this one.
    let completion_traces = data
        .bootstrap
        .receipts
        .iter()
        .map(|receipt| receipt.trace)
        .collect::<Vec<_>>();
    let ordered = data
        .traces
        .events
        .iter()
        .map(|event| (event.trace_id, event.kind.raw()))
        .collect::<Vec<_>>();

    // The store's first event is the runtime root, which precedes stage one and
    // is not one of its effects.
    let mut cursor = 1_usize;
    let mut previous: Option<TraceId> = None;
    for completion in completion_traces {
        let end = ordered
            .iter()
            .position(|(trace, _)| *trace == completion)
            .expect("completion trace must be committed");
        let stage_effects = ordered[cursor..end]
            .iter()
            .filter(|(_, kind)| *kind != BOOTSTRAP_STAGE_COMPLETION_EVENT_KIND)
            .map(|(trace, _)| *trace)
            .collect::<Vec<_>>();

        let event = completion_event(&data, completion);
        for effect_trace in &stage_effects {
            assert!(
                event.causes.contains(effect_trace),
                "stage completion {completion:?} must name effect trace {effect_trace:?}"
            );
        }
        if let Some(previous) = previous {
            assert!(
                event.causes.contains(&previous),
                "stage completion {completion:?} must name the previous receipt"
            );
        }
        assert!(
            event.causes.windows(2).all(|pair| pair[0] < pair[1]),
            "causes must be sorted and deduplicated"
        );

        cursor = end + 1;
        previous = Some(completion);
    }
}

#[test]
fn an_empty_stage_still_produces_a_completion_receipt() {
    // Given: a configuration whose population, promotion, and activity stages
    // commit no domain effect at all.
    let config = empty_stage_config(4_105);
    assert_eq!(config.bootstrap_population, 0);
    assert_eq!(config.actor_count, 0);
    let data = snapshot_of(config);

    // Then: the record is still complete, and the empty stages still transition
    // their bounded result state through a real committed effect.
    assert_eq!(data.bootstrap.receipts.len(), BOOTSTRAP_STAGE_COUNT);
    assert!(data.population.aggregates.is_empty());
    for stage in [
        HistoricalStageId::new(3),
        HistoricalStageId::new(4),
        HistoricalStageId::new(5),
    ] {
        let receipt = data
            .bootstrap
            .receipts
            .iter()
            .find(|receipt| receipt.stage == stage)
            .expect("an empty stage still has a receipt");
        let event = completion_event(&data, receipt.trace);
        assert_eq!(event.effects.len(), 1);
        assert_ne!(event.effects[0].before(), event.effects[0].after());
        assert_eq!(event.effects[0].after(), receipt.result);
    }

    // And: no two empty stages share a result, so the completion is not a
    // repeated sentinel.
    let results = data
        .bootstrap
        .receipts
        .iter()
        .map(|receipt| receipt.result)
        .collect::<BTreeSet<_>>();
    assert_eq!(results.len(), BOOTSTRAP_STAGE_COUNT);
}

#[test]
fn the_same_seed_and_recipe_produce_an_identical_record_and_digests() {
    // Given: two runtimes constructed from the same configuration.
    let first = Runtime::new(populated_config(4_106)).expect("first bootstrap must succeed");
    let second = Runtime::new(populated_config(4_106)).expect("second bootstrap must succeed");

    // Then: their canonical records and digests are identical.
    let first_data = first.export_snapshot().expect("first must export");
    let second_data = second.export_snapshot().expect("second must export");
    assert_eq!(first_data.bootstrap, second_data.bootstrap);

    let first_snapshot = first.snapshot().expect("first summary must read");
    let second_snapshot = second.snapshot().expect("second summary must read");
    assert_eq!(
        first_snapshot.physical_state_digest,
        second_snapshot.physical_state_digest
    );
    assert_eq!(
        first_snapshot.history_digest,
        second_snapshot.history_digest
    );

    // And: the persisted envelope is byte-identical.
    let first_envelope = assemble_envelope(&first_data).expect("first envelope must assemble");
    let second_envelope = assemble_envelope(&second_data).expect("second envelope must assemble");
    assert_eq!(
        first_envelope.sections[&0x0009_u64].bytes,
        second_envelope.sections[&0x0009_u64].bytes
    );
}

#[test]
fn different_stage_output_changes_that_stages_result_fingerprint() {
    // Given: a baseline record.
    let baseline = snapshot_of(populated_config(4_107));

    // When: only the terrain seed changes.
    let mut terrain = populated_config(4_107);
    terrain.carrier_adapter = causafera_runtime::CarrierAdapterConfig::terrain_seed(9_999);
    let terrain = snapshot_of(terrain);

    // Then: the geography stage's result moves.
    assert_ne!(
        baseline.bootstrap.receipts[0].result,
        terrain.bootstrap.receipts[0].result
    );

    // When: only the bootstrap population changes.
    let mut population = populated_config(4_107);
    population.bootstrap_population = 256;
    let population = snapshot_of(population);

    // Then: the population stage's result moves.
    assert_ne!(
        baseline.bootstrap.receipts[2].result,
        population.bootstrap.receipts[2].result
    );

    // And: so does the plan identity, because a stage parameter changed.
    assert_ne!(baseline.bootstrap.plan.id, population.bootstrap.plan.id);
}

#[test]
fn constructing_the_record_does_not_advance_scheduler_time() {
    // Given: a runtime whose canonical stage timeline runs to six.
    let runtime = Runtime::new(populated_config(4_108)).expect("bootstrap must succeed");
    let data = runtime.export_snapshot().expect("state must export");
    assert_eq!(
        data.bootstrap
            .plan
            .stages
            .last()
            .expect("six stages")
            .ends_at,
        SimulationTime::new(6)
    );

    // Then: the scheduler and the authoritative runtime clock are still at zero,
    // and every stage effect kept the existing Lifecycle timestamp convention.
    assert_eq!(runtime.current_time(), SimulationTime::new(0));
    assert_eq!(data.recipe.completed_time, SimulationTime::new(0));
    assert_eq!(
        data.physical_counters.advanced_through,
        SimulationTime::new(0)
    );
    for receipt in &data.bootstrap.receipts {
        assert_eq!(
            completion_event(&data, receipt.trace).time,
            SimulationTime::new(0)
        );
    }
}

#[test]
fn production_bootstrap_save_resume_preserves_record() {
    // Given: a production runtime advanced past bootstrap.
    let mut uninterrupted =
        Runtime::new(populated_config(4_109)).expect("uninterrupted bootstrap must succeed");
    uninterrupted.run_ticks(4).expect("warm-up must run");
    let checkpoint = uninterrupted
        .export_snapshot()
        .expect("checkpoint must export");
    uninterrupted
        .run_ticks(4)
        .expect("uninterrupted continuation must run");
    let uninterrupted_snapshot = uninterrupted.snapshot().expect("summary must read");

    // When: the checkpoint is written, read back, and resumed.
    let envelope = assemble_envelope(&checkpoint).expect("envelope must assemble");
    let restored = disassemble_envelope(&envelope).expect("envelope must disassemble");
    assert_eq!(restored.bootstrap, checkpoint.bootstrap);

    let mut resumed = Runtime::from_snapshot(restored).expect("checkpoint must resume");
    resumed.run_ticks(4).expect("resumed continuation must run");
    let resumed_snapshot = resumed.snapshot().expect("resumed summary must read");
    let resumed_data = resumed
        .export_snapshot()
        .expect("resumed state must export");

    // Then: the canonical record and both digests agree.
    assert_eq!(resumed_data.bootstrap, checkpoint.bootstrap);
    assert_eq!(
        uninterrupted_snapshot.physical_state_digest,
        resumed_snapshot.physical_state_digest
    );
    assert_eq!(
        uninterrupted_snapshot.history_digest,
        resumed_snapshot.history_digest
    );
}

#[test]
fn an_empty_bootstrap_record_cannot_be_imported_as_production_state() {
    // Given: a valid production snapshot.
    let mut data = snapshot_of(populated_config(4_110));

    // When: its receipts are dropped, as a major-1 snapshot would have carried.
    data.bootstrap.receipts.clear();

    // Then: import fails closed rather than defaulting an empty record in.
    assert!(matches!(
        RuntimeState::import_snapshot(data),
        Err(RuntimeError::InvalidSnapshot(_))
    ));
}

#[test]
fn a_receipt_set_that_breaks_declared_ancestry_is_rejected() {
    // Given: a valid production snapshot.
    let mut data = snapshot_of(populated_config(4_111));

    // When: the second stage's receipt no longer names the first stage's receipt.
    data.bootstrap.receipts[1].causes.clear();

    // Then: canonical receipt validation rejects the record.
    assert!(matches!(
        RuntimeState::import_snapshot(data),
        Err(RuntimeError::InvalidSnapshot(_))
    ));
}

#[test]
fn a_plan_with_a_missing_stage_is_rejected() {
    // Given: a valid production snapshot.
    let mut data = snapshot_of(populated_config(4_112));

    // When: one stage is removed from the plan while its receipt remains.
    data.bootstrap.plan.stages.pop();

    // Then: the receipt count no longer matches the plan and import fails closed.
    assert!(matches!(
        RuntimeState::import_snapshot(data),
        Err(RuntimeError::InvalidSnapshot(_))
    ));
}

// ---------------------------------------------------------------------------
// Persistence: canonical bytes and fail-closed corruption handling.
// ---------------------------------------------------------------------------

fn rejects(data: RuntimeSnapshotData) {
    assert!(
        matches!(
            RuntimeState::import_snapshot(data),
            Err(RuntimeError::InvalidSnapshot(_))
        ),
        "a corrupted bootstrap record must fail closed"
    );
}

#[test]
fn the_bootstrap_section_roundtrips_and_is_canonical() {
    // Given: a real production bootstrap record.
    let data = snapshot_of(populated_config(4_120));
    let encoded = encode_population_section(&data.population, &data.bootstrap);

    // Then: it decodes back unchanged and re-encodes to the same bytes.
    let (population, bootstrap) =
        decode_population_section(&encoded).expect("canonical bytes must decode");
    assert_eq!(population, data.population);
    assert_eq!(bootstrap, data.bootstrap);
    assert_eq!(
        encode_population_section(&population, &bootstrap),
        encoded,
        "re-encoding a decoded record must be byte-identical"
    );
}

#[test]
fn the_bootstrap_section_rejects_truncated_and_trailing_bytes() {
    // Given: canonical bootstrap section bytes.
    let data = snapshot_of(populated_config(4_121));
    let encoded = encode_population_section(&data.population, &data.bootstrap);

    // Then: a payload one byte short and a payload one byte long both fail.
    let mut truncated = encoded.clone();
    truncated.pop();
    assert!(decode_population_section(&truncated).is_err());

    let mut trailing = encoded;
    trailing.push(0);
    assert!(decode_population_section(&trailing).is_err());
}

#[test]
fn the_bootstrap_section_rejects_unsorted_and_duplicated_entries() {
    // Given: a real record whose receipts are strictly ordered by stage.
    let data = snapshot_of(populated_config(4_122));

    // When: two receipts are swapped out of canonical order.
    let mut unsorted = data.clone();
    unsorted.bootstrap.receipts.swap(0, 1);
    assert!(
        decode_population_section(&encode_population_section(
            &unsorted.population,
            &unsorted.bootstrap
        ))
        .is_err()
    );

    // When: a stage identity is duplicated in the plan.
    let mut duplicated = data.clone();
    duplicated.bootstrap.plan.stages[1].stage = duplicated.bootstrap.plan.stages[0].stage;
    assert!(
        decode_population_section(&encode_population_section(
            &duplicated.population,
            &duplicated.bootstrap
        ))
        .is_err()
    );

    // When: a stage's target list is written out of order.
    let mut targets = data;
    targets.bootstrap.plan.stages[0].targets.swap(0, 1);
    assert!(
        decode_population_section(&encode_population_section(
            &targets.population,
            &targets.bootstrap
        ))
        .is_err()
    );
}

#[test]
fn an_unsupported_bootstrap_section_major_is_rejected() {
    // Given: a current envelope, whose bootstrap section is major 2.
    let data = snapshot_of(populated_config(4_123));
    let envelope = assemble_envelope(&data).expect("envelope must assemble");
    assert_eq!(envelope.sections[&0x0009_u64].section_major, 2);

    // Then: the superseded major 1 and any unknown major fail closed rather than
    // being migrated or defaulted.
    for major in [0, 1, 3] {
        let mut tampered = envelope.clone();
        tampered
            .sections
            .get_mut(&0x0009_u64)
            .expect("bootstrap section is present")
            .section_major = major;
        assert!(
            disassemble_envelope(&tampered).is_err(),
            "bootstrap section major {major} must be rejected"
        );
    }
}

#[test]
fn a_forged_completion_trace_is_rejected() {
    // Given: a valid production snapshot.
    let mut data = snapshot_of(populated_config(4_124));

    // When: a receipt points at a trace that is not its stage's completion.
    let other = data.bootstrap.receipts[1].trace;
    data.bootstrap.receipts[0].trace = other;
    data.bootstrap.receipts[1].causes = vec![other];

    // Then: import rejects it.
    rejects(data);
}

#[test]
fn a_stale_completion_trace_is_rejected() {
    // Given: a valid production snapshot.
    let mut data = snapshot_of(populated_config(4_125));

    // When: a receipt names a trace the store never committed.
    let stale = TraceId::new(u64::MAX);
    data.bootstrap.receipts[5].trace = stale;

    // Then: import rejects it.
    rejects(data);
}

#[test]
fn a_forged_stage_result_is_rejected() {
    // Given: a valid production snapshot.
    let mut data = snapshot_of(populated_config(4_126));

    // When: a receipt's result no longer matches its committed completion effect.
    let other = data.bootstrap.receipts[2].result;
    data.bootstrap.receipts[0].result = other;
    data.bootstrap.stage_results[0].result = other;

    // Then: import rejects it.
    rejects(data);
}

#[test]
fn a_stage_result_that_disagrees_with_its_receipt_is_rejected() {
    // Given: a valid production snapshot.
    let mut data = snapshot_of(populated_config(4_127));

    // When: the materialized stage result no longer matches its receipt.
    let other = data.bootstrap.stage_results[3].result;
    data.bootstrap.stage_results[0].result = other;

    // Then: import rejects it.
    rejects(data);
}

#[test]
fn a_missing_stage_result_entry_is_rejected() {
    // Given: a valid production snapshot.
    let mut data = snapshot_of(populated_config(4_128));

    // When: the bounded stage-result state no longer covers every receipt.
    data.bootstrap.stage_results.pop();

    // Then: import rejects it.
    rejects(data);
}

#[test]
fn a_target_outside_the_active_chunk_set_is_rejected() {
    // Given: a valid production snapshot.
    let mut data = snapshot_of(populated_config(4_129));

    // When: one stage claims a target the configured active chunk set does not
    // produce.
    for stage in &mut data.bootstrap.plan.stages {
        let last = stage.targets.len() - 1;
        stage.targets[last] = causafera_types::ChunkId::new(u64::MAX);
    }

    // Then: the plan no longer matches the persisted configuration.
    rejects(data);
}

#[test]
fn a_forged_plan_identity_or_seed_is_rejected() {
    // Given: two valid production snapshots.
    let mut identity = snapshot_of(populated_config(4_130));
    let mut seed = snapshot_of(populated_config(4_130));

    // When: the opaque plan identity, then the world seed, are rewritten.
    identity.bootstrap.plan.id = causafera_types::HistoricalBootstrapId::new(1);
    seed.bootstrap.plan.world_seed = 1;

    // Then: both fail against the plan the persisted configuration reproduces.
    rejects(identity);
    rejects(seed);
}

#[test]
fn a_forged_stage_parameter_fingerprint_is_rejected() {
    // Given: a valid production snapshot.
    let mut data = snapshot_of(populated_config(4_131));

    // When: a stage's parameter fingerprint is swapped for another stage's.
    data.bootstrap.plan.stages[0].parameters = data.bootstrap.plan.stages[1].parameters;

    // Then: import rejects it.
    rejects(data);
}

#[test]
fn bootstrap_population_that_is_not_conserved_is_rejected() {
    // Given: a valid bootstrap-time snapshot whose 512 configured residents are
    // split between 504 aggregate members and 8 promoted actors.
    let mut data = snapshot_of(populated_config(4_132));
    let total: u64 = data
        .population
        .aggregates
        .iter()
        .map(|aggregate| aggregate.count)
        .sum();
    assert_eq!(total + data.actors_objective.actors.len() as u64, 512);

    // When: an aggregate loses a member with no corresponding promotion.
    data.population.aggregates[0].count -= 1;

    // Then: import rejects the unconserved population.
    rejects(data);
}

#[test]
fn promoted_actor_ancestry_outside_the_promotion_receipt_is_rejected() {
    // Given: a valid bootstrap-time snapshot with promoted actors.
    let mut data = snapshot_of(populated_config(4_133));
    assert!(!data.actors_objective.actor_ancestry.is_empty());

    // When: an actor claims ancestry the actor promotion stage never named.
    let root = data.traces.events[0].trace_id;
    data.actors_objective.actor_ancestry[0].1 = vec![root];

    // Then: import rejects it.
    rejects(data);
}

#[test]
fn the_persisted_configuration_preserves_the_active_chunk_shape() {
    // Given: an Area-shaped chart, whose nine chunks the canonical plan targets.
    let data = snapshot_of(populated_config(4_134));
    assert_eq!(data.bootstrap.plan.stages[0].targets.len(), 9);

    // When: the snapshot is written and read back.
    let envelope = assemble_envelope(&data).expect("envelope must assemble");
    let restored = disassemble_envelope(&envelope).expect("envelope must disassemble");

    // Then: the shape survives, so the resumed configuration reproduces the same
    // plan rather than a three-chunk Line chart's.
    assert_eq!(restored.recipe.config, data.recipe.config);
    assert_eq!(restored.bootstrap.plan, data.bootstrap.plan);
    RuntimeState::import_snapshot(restored).expect("a faithfully restored snapshot must import");
}

// ---------------------------------------------------------------------------
// Production entry points, and the absence of fixture reachability.
// ---------------------------------------------------------------------------

#[test]
fn every_headless_entry_point_produces_the_same_record() {
    // Given: the same configuration reached three ways a caller can construct a
    // runtime — a validated config, a bare seed, and a resumed snapshot.
    let direct = snapshot_of(populated_config(4_140));
    let repeat = snapshot_of(populated_config(4_140));

    let envelope = assemble_envelope(&direct).expect("envelope must assemble");
    let restored = disassemble_envelope(&envelope).expect("envelope must disassemble");
    let resumed = Runtime::from_snapshot(restored)
        .expect("checkpoint must resume")
        .export_snapshot()
        .expect("resumed state must export");

    let seeded = Runtime::from_seed(4_140)
        .expect("a bare seed must bootstrap")
        .export_snapshot()
        .expect("seeded state must export");
    let seeded_direct = snapshot_of(RuntimeConfig::new(4_140));

    // Then: identical configurations produce identical records, and the bare
    // seed matches its own explicit configuration rather than the populated one.
    assert_eq!(direct.bootstrap, repeat.bootstrap);
    assert_eq!(direct.bootstrap, resumed.bootstrap);
    assert_eq!(seeded.bootstrap, seeded_direct.bootstrap);
    assert_ne!(direct.bootstrap.plan.id, seeded.bootstrap.plan.id);
}

#[test]
fn resuming_a_snapshot_does_not_rebootstrap_it() {
    // Given: a production runtime advanced past bootstrap.
    let mut runtime = Runtime::new(populated_config(4_141)).expect("bootstrap must succeed");
    runtime.run_ticks(3).expect("warm-up must run");
    let data = runtime.export_snapshot().expect("state must export");
    let completions = |data: &RuntimeSnapshotData| {
        data.traces
            .events
            .iter()
            .filter(|event| event.kind.raw() == BOOTSTRAP_STAGE_COMPLETION_EVENT_KIND)
            .count()
    };
    assert_eq!(completions(&data), BOOTSTRAP_STAGE_COUNT);

    // When: it is resumed.
    let resumed = Runtime::from_snapshot(data.clone())
        .expect("snapshot must resume")
        .export_snapshot()
        .expect("resumed state must export");

    // Then: import is the only resume path — the record is the imported one and
    // no second set of completions was committed on top of it.
    assert_eq!(completions(&resumed), BOOTSTRAP_STAGE_COUNT);
    assert_eq!(resumed.traces.events.len(), data.traces.events.len());
    assert_eq!(resumed.bootstrap, data.bootstrap);
}

#[test]
fn promoted_actor_ancestry_names_the_actor_promotion_receipt() {
    // Given: a bootstrap that promoted eight actors from the aggregate.
    let data = snapshot_of(populated_config(4_142));
    assert_eq!(data.actors_objective.actor_ancestry.len(), 8);

    // The actor promotion stage is stage four.
    let promotion = data
        .bootstrap
        .receipts
        .iter()
        .find(|receipt| receipt.stage == HistoricalStageId::new(4))
        .expect("the record has an actor promotion receipt");
    let completion = completion_event(&data, promotion.trace);

    // Then: every promoted actor's ancestry is a trace that receipt named, and
    // the configured population is conserved across aggregates and actors.
    for (actor, ancestry) in &data.actors_objective.actor_ancestry {
        assert!(!ancestry.is_empty(), "actor {actor:?} has no ancestry");
        for trace in ancestry {
            assert!(
                completion.causes.contains(trace),
                "actor {actor:?} ancestry {trace:?} is not named by the promotion receipt"
            );
        }
    }
    let aggregate_total: u64 = data
        .population
        .aggregates
        .iter()
        .map(|aggregate| aggregate.count)
        .sum();
    assert_eq!(
        aggregate_total + data.actors_objective.actor_ancestry.len() as u64,
        512
    );
}

/// Production source may not reach a fixture or demo constructor.
///
/// This is a source audit, not the positive claim on its own: what makes the
/// production path fixture-free is that every entry point runs the same
/// `RuntimeBootstrapRecipe` and that every actor at bootstrap carries ancestry
/// the actor-promotion receipt named — asserted by the tests above. This test
/// keeps a fixture constructor from being reintroduced later without anyone
/// noticing.
///
/// Comments are stripped before scanning so prose may still name what was
/// removed and why. Anything genuinely test-only belongs under a `tests/`
/// directory, which is not production source and is not scanned.
#[test]
fn production_source_reaches_no_fixture_constructor() {
    const BANNED: [&str; 4] = [
        "fixture_actors",
        "fixture_sensors",
        "fn fixture_",
        "fn demo_",
    ];
    /// Test-only paths permitted to name a fixture constructor. Empty today:
    /// no test requires one, which is why the constructors were removed rather
    /// than moved behind `#[cfg(test)]`.
    const ALLOWLIST: [&str; 0] = [];

    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .expect("the runtime crate sits two levels below the repository root")
        .to_path_buf();

    let mut scanned = 0_usize;
    let mut offenders = Vec::new();
    for directory in ["crates", "apps"] {
        for path in rust_sources(&root.join(directory)) {
            let relative = path
                .strip_prefix(&root)
                .expect("collected below the root")
                .to_string_lossy()
                .replace('\\', "/");
            // Only production source is in scope; a `tests` directory anywhere in
            // the path is test support by construction.
            if relative.split('/').any(|segment| segment == "tests")
                || ALLOWLIST.contains(&relative.as_str())
            {
                continue;
            }
            scanned += 1;
            let source = strip_comments(&std::fs::read_to_string(&path).expect("readable source"));
            for banned in BANNED {
                if source.contains(banned) {
                    offenders.push(format!("{relative}: {banned}"));
                }
            }
        }
    }

    // The scan must actually have read the production tree; a path mistake that
    // walked nothing would otherwise pass vacuously.
    assert!(
        scanned > 100,
        "the audit scanned only {scanned} production source files"
    );
    assert!(
        offenders.is_empty(),
        "production source reaches a fixture constructor: {offenders:?}"
    );
}

fn rust_sources(directory: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut found = Vec::new();
    let Ok(entries) = std::fs::read_dir(directory) else {
        return found;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if path.file_name().is_some_and(|name| name == "target") {
                continue;
            }
            found.extend(rust_sources(&path));
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            found.push(path);
        }
    }
    found
}

/// Drop `//` line comments and `/* */` block comments.
///
/// Deliberately naive about `//` inside string literals: a false positive there
/// would only hide a fixture call written inside a string, which is not a call.
fn strip_comments(source: &str) -> String {
    let mut out = String::with_capacity(source.len());
    let mut rest = source;
    while let Some(index) = rest.find("/*") {
        out.push_str(&rest[..index]);
        rest = match rest[index + 2..].find("*/") {
            Some(end) => &rest[index + 2 + end + 2..],
            None => "",
        };
    }
    out.push_str(rest);
    out.lines()
        .map(|line| match line.find("//") {
            Some(index) => &line[..index],
            None => line,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

// ---------------------------------------------------------------------------
// Bounded performance evidence.
// ---------------------------------------------------------------------------

#[test]
fn the_bootstrap_benchmark_measures_the_stated_envelope_reproducibly() {
    use causafera_runtime::{BootstrapClosureBenchmarkConfig, run_bootstrap_closure_benchmark};

    // Given: the accepted bounded envelope — nine active chunks, bootstrap
    // population 512, eight promoted actors, one sensor aperture each.
    let config = BootstrapClosureBenchmarkConfig::default();

    // When: the benchmark runs twice at the same seed.
    let first = run_bootstrap_closure_benchmark(config).expect("benchmark must complete");
    let second = run_bootstrap_closure_benchmark(config).expect("benchmark must complete");

    // Then: it measured the envelope it claims to describe.
    assert_eq!(first.active_chunk_count, 9);
    assert_eq!(first.bootstrap_population, 512);
    assert_eq!(first.promoted_actor_count, 8);
    assert_eq!(first.sensor_count, 1);
    assert_eq!(first.receipt_count, BOOTSTRAP_STAGE_COUNT as u32);

    // And: every figure is a real nonzero measurement, not a placeholder.
    assert!(first.bootstrap_wall_time_ns > 0);
    assert!(first.import_wall_time_ns > 0);
    assert!(first.encoded_snapshot_bytes > 0);
    assert!(first.bootstrap_record_bytes > 0);
    assert!(first.bootstrap_provenance_events > 0);
    assert!(first.control_poll_wall_time_ns > 0);
    assert!(first.observer_poll_wall_time_ns > 0);
    assert!(first.observer_summary_bytes > 0);

    // And: the authoritative outputs are identical between runs. Wall times are
    // deliberately not compared — they are measurements of a machine, and this
    // benchmark establishes no threshold.
    assert_eq!(first.plan_id, second.plan_id);
    assert_eq!(first.physical_state_digest, second.physical_state_digest);
    assert_eq!(first.history_digest, second.history_digest);
    assert_eq!(first.encoded_snapshot_bytes, second.encoded_snapshot_bytes);
    assert_eq!(first.bootstrap_record_bytes, second.bootstrap_record_bytes);
    assert_eq!(
        first.bootstrap_provenance_events,
        second.bootstrap_provenance_events
    );
    assert_eq!(first.observer_summary_bytes, second.observer_summary_bytes);
}

#[test]
fn the_bootstrap_benchmark_rejects_an_empty_measurement_window() {
    use causafera_runtime::{BootstrapClosureBenchmarkConfig, run_bootstrap_closure_benchmark};

    // Given: a configuration that would measure nothing.
    for config in [
        BootstrapClosureBenchmarkConfig {
            iterations: 0,
            ..BootstrapClosureBenchmarkConfig::default()
        },
        BootstrapClosureBenchmarkConfig {
            import_iterations: 0,
            ..BootstrapClosureBenchmarkConfig::default()
        },
        BootstrapClosureBenchmarkConfig {
            observer_polls: 0,
            ..BootstrapClosureBenchmarkConfig::default()
        },
    ] {
        // Then: it is rejected rather than reporting an empty envelope.
        assert!(run_bootstrap_closure_benchmark(config).is_err());
    }
}
