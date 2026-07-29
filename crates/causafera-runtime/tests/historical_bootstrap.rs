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

/// A stage's parameter fingerprint has to cover everything the stage actually
/// executes under, not just the fields its own struct happened to carry first.
/// `chunk_extent` decides the lattice terrain and thermal fields are built at,
/// and `sensor_count` decides how promoted actors are equipped; neither is
/// visible in the committed per-object effects, so if they are missing from the
/// parameters then two genuinely different recipes share a plan identity *and* a
/// set of stage results.
#[test]
fn configuration_the_stages_execute_under_reaches_their_parameter_fingerprints() {
    let baseline = snapshot_of(populated_config(4_147));

    // When: only the lattice edge changes.
    let mut wider = populated_config(4_147);
    wider.chunk_extent = 4;
    let wider = snapshot_of(wider);

    // Then: the parameter drives the lattice the stages build, not just the
    // fingerprint. Without this the value could be disconnected from terrain and
    // thermal generation entirely and the test would stay green.
    assert_eq!(baseline.thermal.field_set.fields[0].extent, 3);
    assert_eq!(wider.thermal.field_set.fields[0].extent, 4);
    assert_eq!(baseline.thermal.field_set.fields[0].energy.len(), 27);
    assert_eq!(wider.thermal.field_set.fields[0].energy.len(), 64);

    // And: the plan identity moves, and so do the two stages that build on it.
    assert_ne!(baseline.bootstrap.plan.id, wider.bootstrap.plan.id);
    assert_ne!(
        baseline.bootstrap.plan.stages[0].parameters,
        wider.bootstrap.plan.stages[0].parameters
    );
    assert_ne!(
        baseline.bootstrap.plan.stages[5].parameters,
        wider.bootstrap.plan.stages[5].parameters
    );
    assert_ne!(
        baseline.bootstrap.receipts[0].result,
        wider.bootstrap.receipts[0].result
    );
    assert_ne!(
        baseline.bootstrap.receipts[5].result,
        wider.bootstrap.receipts[5].result
    );

    // When: only the sensor count changes.
    let mut equipped = populated_config(4_147);
    equipped.sensor_count = 2;
    let equipped = snapshot_of(equipped);

    // Then: the parameter is not merely declared — it drives what the stage
    // produced, so the fingerprint and the effect describe one configuration.
    assert!(
        baseline
            .actors_objective
            .actors
            .iter()
            .all(|(_, actor)| actor.sensors.len() == 1)
    );
    assert!(
        equipped
            .actors_objective
            .actors
            .iter()
            .all(|(_, actor)| actor.sensors.len() == 2)
    );

    // And: the promotion stage's parameters and result move with it.
    assert_ne!(baseline.bootstrap.plan.id, equipped.bootstrap.plan.id);
    assert_ne!(
        baseline.bootstrap.plan.stages[3].parameters,
        equipped.bootstrap.plan.stages[3].parameters
    );
    assert_ne!(
        baseline.bootstrap.receipts[3].result,
        equipped.bootstrap.receipts[3].result
    );
}

/// RFC-HIST-001 requires every stage to receive a domain-separated seed
/// contribution. No current stage has stage-local variation, so what is checked
/// here is the contract itself: the seed exists for all six, is stable, and
/// separates stages and seeds.
#[test]
fn every_stage_receives_a_stable_domain_separated_seed() {
    let recipe =
        causafera_runtime::RuntimeBootstrapRecipe::from_runtime_config(&populated_config(4_148))
            .expect("the recipe must build");
    let other =
        causafera_runtime::RuntimeBootstrapRecipe::from_runtime_config(&populated_config(4_149))
            .expect("the recipe must build");

    let seeds = (1..=BOOTSTRAP_STAGE_COUNT as u64)
        .map(|stage| {
            recipe
                .stage_seed(HistoricalStageId::new(stage))
                .expect("every planned stage has a seed")
        })
        .collect::<Vec<_>>();

    assert_eq!(seeds.len(), BOOTSTRAP_STAGE_COUNT);
    assert_eq!(
        seeds.iter().copied().collect::<BTreeSet<_>>().len(),
        BOOTSTRAP_STAGE_COUNT,
        "stages must not share a seed contribution"
    );
    for (ordinal, seed) in seeds.iter().enumerate() {
        let stage = HistoricalStageId::new(ordinal as u64 + 1);
        assert_eq!(
            recipe.stage_seed(stage),
            Some(*seed),
            "seeds must be stable"
        );
        assert_ne!(
            other.stage_seed(stage),
            Some(*seed),
            "a different world seed must move every stage seed"
        );
    }
    assert_eq!(
        recipe.stage_seed(HistoricalStageId::new(BOOTSTRAP_STAGE_COUNT as u64 + 1)),
        None,
        "a stage the plan does not declare has no seed"
    );
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

/// Assert import fails closed **for the stated reason**.
///
/// Taking any error was how a vacuous test hid for four audit rounds: a test
/// named for the active-chunk target guard actually tripped thermal region
/// validation, and deleting the guard it was named for changed nothing. The
/// expected message is the difference between "something rejected this" and
/// "the check this test exists for rejected this".
fn rejects(data: RuntimeSnapshotData, expected: &str) {
    match RuntimeState::import_snapshot(data) {
        Err(RuntimeError::InvalidSnapshot(reason)) => assert!(
            reason.contains(expected),
            "expected rejection mentioning {expected:?}, got {reason:?}"
        ),
        Err(other) => panic!("expected InvalidSnapshot({expected:?}), got {other}"),
        Ok(_) => panic!("a corrupted bootstrap record must fail closed: {expected:?}"),
    }
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
    rejects(data, "invalid canonical bootstrap record");
}

#[test]
fn a_stale_completion_trace_is_rejected() {
    // Given: a valid production snapshot.
    let mut data = snapshot_of(populated_config(4_125));

    // When: a receipt names a trace the store never committed.
    let stale = TraceId::new(u64::MAX);
    data.bootstrap.receipts[5].trace = stale;

    // Then: import rejects it.
    rejects(data, "bootstrap receipt references unknown trace");
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
    rejects(
        data,
        "bootstrap completion does not transition its stage result",
    );
}

#[test]
fn a_stage_result_that_disagrees_with_its_receipt_is_rejected() {
    // Given: a valid production snapshot.
    let mut data = snapshot_of(populated_config(4_127));

    // When: the materialized stage result no longer matches its receipt.
    let other = data.bootstrap.stage_results[3].result;
    data.bootstrap.stage_results[0].result = other;

    // Then: import rejects it.
    rejects(data, "bootstrap stage result does not match its receipt");
}

#[test]
fn a_missing_stage_result_entry_is_rejected() {
    // Given: a valid production snapshot.
    let mut data = snapshot_of(populated_config(4_128));

    // When: the bounded stage-result state no longer covers every receipt.
    data.bootstrap.stage_results.pop();

    // Then: import rejects it.
    rejects(data, "bootstrap stage results do not cover the receipt set");
}

/// The three internally-consistent halves of a stage's result — the completion
/// event's committed effect, the receipt's `result`, and the materialized
/// stage-result entry — can all be rewritten together. Each of the pairwise
/// checks then passes, so what has to catch it is recomputing the result from
/// what the trace store says the stage actually committed.
#[test]
fn a_coherently_forged_stage_result_is_rejected() {
    // Given: a valid production snapshot.
    let mut data = snapshot_of(populated_config(4_143));
    let forged = causafera_core::StateFingerprint::new([0xAB; 32]);
    let trace = data.bootstrap.receipts[0].trace;

    // When: the receipt, the materialized stage result, and the completion
    // event's own effect are rewritten to agree on a forged fingerprint.
    data.bootstrap.receipts[0].result = forged;
    data.bootstrap.stage_results[0].result = forged;
    let event = data
        .traces
        .events
        .iter_mut()
        .find(|event| event.trace_id == trace)
        .expect("the completion event is committed");
    let target = event.effects[0].target();
    let before = event.effects[0].before();
    event.effects[0] = causafera_core::CausalEffect::new(target, before, forged)
        .expect("a forged transition is still a transition");

    // Then: the recomputed result does not match, so import fails closed even
    // though every pairwise check agrees.
    rejects(
        data,
        "bootstrap stage result does not match what the stage committed",
    );
}

/// A completion whose causes have been cut back to just its predecessor is a
/// receipt that has hidden its stage's detailed ancestry, which is the one thing
/// the terminal-receipt design exists to prevent.
#[test]
fn a_completion_stripped_of_its_stage_effects_is_rejected() {
    // Given: a valid production snapshot whose first completion names its
    // terrain effects alongside nothing else.
    let mut data = snapshot_of(populated_config(4_144));
    let trace = data.bootstrap.receipts[0].trace;
    let event = data
        .traces
        .events
        .iter_mut()
        .find(|event| event.trace_id == trace)
        .expect("the completion event is committed");
    assert!(
        event.causes.len() > 1,
        "stage one names its terrain effects"
    );

    // When: everything but the first cause is dropped.
    event.causes.truncate(1);

    // Then: import rejects the summarized ancestry.
    rejects(
        data,
        "bootstrap completion does not name exactly its stage effects and predecessor",
    );
}

#[test]
fn a_stage_effect_rewritten_under_an_unchanged_receipt_is_rejected() {
    // Given: a valid production snapshot.
    let mut data = snapshot_of(populated_config(4_145));
    let completions = data
        .bootstrap
        .receipts
        .iter()
        .map(|receipt| receipt.trace)
        .collect::<BTreeSet<_>>();

    // When: one ordinary stage effect's committed payload is rewritten, leaving
    // the record itself untouched.
    let effect_event = data
        .traces
        .events
        .iter_mut()
        // The runtime root is not one of any stage's effects, so skip it.
        .skip(1)
        .find(|event| !completions.contains(&event.trace_id) && !event.effects.is_empty())
        .expect("bootstrap commits ordinary stage effects");
    let target = effect_event.effects[0].target();
    let before = effect_event.effects[0].before();
    effect_event.effects[0] = causafera_core::CausalEffect::new(
        target,
        before,
        causafera_core::StateFingerprint::new([0xCD; 32]),
    )
    .expect("a rewritten transition is still a transition");

    // Then: the stage result no longer matches what the stage committed.
    rejects(
        data,
        "bootstrap stage result does not match what the stage committed",
    );
}

#[test]
fn a_stage_completion_the_record_does_not_name_is_rejected() {
    // Given: a valid production snapshot.
    let mut data = snapshot_of(populated_config(4_146));
    let template = data
        .traces
        .events
        .iter()
        .find(|event| event.kind.raw() == BOOTSTRAP_STAGE_COMPLETION_EVENT_KIND)
        .expect("the store carries completions")
        .clone();

    // When: a seventh completion is appended that no receipt names.
    let mut extra = template;
    extra.trace_id = TraceId::new(data.traces.next_trace_id);
    extra.event_id = causafera_types::EventId::new(data.traces.next_event_id);
    extra.causes.clear();
    data.traces.next_trace_id += 1;
    data.traces.next_event_id += 1;
    data.traces.events.push(extra);

    // Then: the store no longer describes the run the record claims.
    rejects(data, "committed stage completions do not match the record");
}

#[test]
fn a_target_the_configuration_does_not_produce_is_rejected() {
    // Given: a valid production snapshot.
    let mut data = snapshot_of(populated_config(4_129));

    // When: one stage claims a target the configured active chunk set does not
    // produce.
    for stage in &mut data.bootstrap.plan.stages {
        let last = stage.targets.len() - 1;
        stage.targets[last] = causafera_types::ChunkId::new(u64::MAX);
    }

    // Then: the plan no longer matches the persisted configuration. This is the
    // plan-versus-configuration comparison, not the active-chunk check — see
    // the test below for that one.
    rejects(
        data,
        "bootstrap plan does not match the persisted configuration",
    );
}

/// The plan-versus-configuration comparison derives both sides from the same
/// configuration, so on its own it says nothing about whether those chunks are
/// present in the restored state. This is the separate check — and reaching it
/// means removing the chunk from every section that also tracks it, because
/// thermal region validation otherwise rejects the snapshot first. An earlier
/// version of this test removed the chunk from the spatial section only, and so
/// proved nothing about the guard it is named for.
#[test]
fn a_target_that_is_not_an_active_chunk_is_rejected() {
    let mut data = snapshot_of(populated_config(4_151));
    let dropped = data.spatial.active_chunks[8].chunk;
    assert!(
        data.bootstrap.plan.stages[0]
            .targets
            .contains(&causafera_runtime::bootstrap_chunk_target(dropped)),
        "the plan targets the chunk this test removes"
    );

    data.spatial
        .active_chunks
        .retain(|chunk| chunk.chunk != dropped);
    data.spatial
        .carrier_adapters
        .retain(|carrier| carrier.chunk != dropped);
    data.mana.fields.retain(|field| field.chunk != dropped);
    data.thermal
        .field_set
        .fields
        .retain(|field| field.chunk != dropped);
    data.thermal
        .active_region
        .active_chunks
        .retain(|chunk| *chunk != dropped);
    data.thermal
        .active_region
        .resident_chunks
        .retain(|chunk| *chunk != dropped);
    data.material_surfaces
        .records
        .retain(|record| record.id.chunk != dropped);
    data.material_surfaces
        .transitions
        .retain(|transition| transition.id.chunk != dropped);
    data.material_surfaces
        .pending_physical_changes
        .retain(|id| id.chunk != dropped);
    data.resolution
        .entries
        .retain(|entry| entry.chunk != dropped);

    rejects(data, "bootstrap stage targets a chunk that is not active");
}

/// `advanced_through` arrives from the snapshot, and `export_snapshot` writes
/// `completed_time` from it, so the two agree in every honest snapshot. When it
/// was the sole gate on the bootstrap-time population and ancestry checks, a
/// snapshot presenting as bootstrap-time while claiming to have advanced skipped
/// both guards and could delete residents and erase promoted ancestry unnoticed.
/// The gate is now the store's shape, with the clock as a mutual pin; this test
/// keeps the clock half of that pin honest.
#[test]
fn a_snapshot_whose_clocks_disagree_is_rejected() {
    // Given: a valid bootstrap-time snapshot.
    let mut data = snapshot_of(populated_config(4_152));
    assert_eq!(data.recipe.completed_time, SimulationTime::new(0));

    // When: it claims to have advanced while still presenting as bootstrap-time,
    // and uses that to delete residents and erase every promoted ancestry.
    data.physical_counters.advanced_through = SimulationTime::new(1);
    data.population.aggregates[0].count -= 100;
    for (_, ancestry) in data.actors_objective.actor_ancestry.iter_mut() {
        ancestry.clear();
    }

    // Then: the disagreement itself is rejected, which re-arms both guards.
    rejects(
        data,
        "snapshot completed time does not match the advanced-through counter",
    );
}

/// The clock check must not be satisfiable by moving the other clock either.
#[test]
fn a_snapshot_that_moves_only_its_completed_time_is_rejected() {
    let mut data = snapshot_of(populated_config(4_153));
    data.recipe.completed_time = SimulationTime::new(1);
    rejects(
        data,
        "snapshot completed time does not match the advanced-through counter",
    );
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

    // When: an aggregate gains ten members, and the birth counter is moved to
    // match so that the births-minus-deaths identity — which now holds at every
    // simulation time and would otherwise catch this first — stays satisfied.
    // What remains false is that bootstrap has net zero births and deaths.
    data.population.aggregates[0].count += 10;
    data.physical_counters.population_births = 10;

    // Then: import rejects the unconserved population.
    rejects(data, "bootstrap population is not conserved");
}

/// The identity above is scoped to a store that ends at bootstrap. This one is
/// not: residents are created only by bootstrap or a birth and removed only by a
/// death, whatever the clock says.
#[test]
fn a_living_population_that_contradicts_the_birth_and_death_counters_is_rejected() {
    let mut data = snapshot_of(populated_config(4_162));
    data.population.aggregates[0].count -= 1;
    rejects(
        data,
        "living population does not match the bootstrap population plus births minus deaths",
    );

    // And: counters that no history could have produced are rejected as such,
    // rather than wrapping into an accidentally satisfiable total.
    let mut underflow = snapshot_of(populated_config(4_162));
    underflow.physical_counters.population_deaths = u64::MAX;
    rejects(
        underflow,
        "population birth and death counters are not a reachable history",
    );
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
    rejects(
        data,
        "promoted actor ancestry is absent from the actor promotion receipt",
    );
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
    /// A function whose name carries one of these as a whole `_`-delimited
    /// segment is a fixture or demo constructor. Matching segments rather than
    /// four fixed literals catches `make_fixture`, `seed_demo_world` and
    /// `demo_residents`; matching segments rather than raw substrings keeps
    /// `demote_actor_to_aggregate` out of it, which a substring match flags.
    const BANNED_NAME_PARTS: [&str; 2] = ["fixture", "demo"];
    /// Test-only declarations permitted to carry a banned segment, named one by
    /// one rather than by pattern.
    ///
    /// No fixture *constructor* is allowed anywhere, which is why the two that
    /// existed were removed rather than moved behind `#[cfg(test)]`. The single
    /// entry here is a test asserting the absence of demo residents, inside a
    /// `#[cfg(test)]` module in a production file — the scan is textual and
    /// cannot see the attribute, so the exemption is written down instead of
    /// being pattern-matched into invisibility.
    const ALLOWLIST: [&str; 1] = [
        "crates/causafera-runtime/src/runtime.rs: fn production_runtime_does_not_create_demo_residents",
    ];

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
            if relative.split('/').any(|segment| segment == "tests") {
                continue;
            }
            scanned += 1;
            let source = strip_comments(&std::fs::read_to_string(&path).expect("readable source"));
            for declaration in source.match_indices("fn ") {
                let name = source[declaration.0 + 3..]
                    .split(|character: char| !character.is_alphanumeric() && character != '_')
                    .next()
                    .unwrap_or_default()
                    .to_ascii_lowercase();
                if name
                    .split('_')
                    .any(|segment| BANNED_NAME_PARTS.contains(&segment))
                    && !ALLOWLIST.contains(&format!("{relative}: fn {name}").as_str())
                {
                    offenders.push(format!("{relative}: fn {name}"));
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
    // Line comments first. Stripping block comments first meant an unmatched
    // `/*` inside a line comment or a string literal swallowed the rest of the
    // file, hiding every declaration after it from the scan.
    let without_line_comments = source
        .lines()
        .map(|line| match line.find("//") {
            Some(index) => &line[..index],
            None => line,
        })
        .collect::<Vec<_>>()
        .join("\n");

    let mut out = String::with_capacity(without_line_comments.len());
    let mut rest = without_line_comments.as_str();
    while let Some(index) = rest.find("/*") {
        out.push_str(&rest[..index]);
        // An unterminated block comment keeps the remainder in scope rather
        // than discarding it: a scan that silently stops reading is worse than
        // one that reports a comment body.
        rest = match rest[index + 2..].find("*/") {
            Some(end) => &rest[index + 2 + end + 2..],
            None => &rest[index + 2..],
        };
    }
    out.push_str(rest);
    out
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

    // And: every figure is a real nonzero measurement, not a placeholder, and
    // every distribution retained the raw samples its summary was taken from.
    for distribution in [
        &first.bootstrap_wall_time,
        &first.import_wall_time,
        &first.control_poll_wall_time,
        &first.observer_poll_wall_time,
    ] {
        assert_eq!(
            distribution.samples_ns.len(),
            config.iterations as usize,
            "every measured repetition must be retained as a raw sample"
        );
        assert!(distribution.mean_ns > 0);
        assert!(distribution.median_ns > 0);
        assert!(distribution.minimum_ns > 0);
        assert!(distribution.maximum_ns >= distribution.minimum_ns);
        assert!(distribution.mean_ns >= distribution.minimum_ns);
        assert!(distribution.mean_ns <= distribution.maximum_ns);
        assert!(distribution.median_ns >= distribution.minimum_ns);
        assert!(distribution.median_ns <= distribution.maximum_ns);
    }
    assert!(first.encoded_snapshot_bytes > 0);
    assert!(first.bootstrap_record_bytes > 0);
    assert!(first.bootstrap_provenance_events > 0);
    assert!(first.observer_summary_bytes > 0);

    // And: the authoritative outputs are identical between runs. Two runs is what
    // this test needs — it is an identity check, not a sample. The wall-time
    // ranges recorded in the plan come from running the benchmark by hand and
    // are not asserted here: they are measurements of a machine, and this
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
fn benchmark_summary_statistics_are_computed_over_the_retained_samples() {
    use causafera_runtime::BenchmarkSamples;

    // Given: a known distribution with an even sample count, so the median is an
    // average of two values rather than one of them.
    let samples = vec![10_u128, 20, 30, 40];
    let summary = BenchmarkSamples::from_samples(samples.clone())
        .expect("a non-empty distribution summarizes");

    // Then: the summary describes exactly those samples.
    assert_eq!(summary.samples_ns, samples);
    assert_eq!(summary.mean_ns, 25);
    assert_eq!(summary.median_ns, 25);
    assert_eq!(summary.minimum_ns, 10);
    assert_eq!(summary.maximum_ns, 40);
    // Population variance is 125; its integer square root is 11.
    assert_eq!(summary.stddev_ns, 11);

    // And: an odd count takes the middle sample, and a single sample has no spread.
    let odd =
        BenchmarkSamples::from_samples(vec![5, 1, 9]).expect("an odd distribution summarizes");
    assert_eq!(odd.median_ns, 5);
    let one = BenchmarkSamples::from_samples(vec![7]).expect("one sample is a distribution");
    assert_eq!(one.mean_ns, 7);
    assert_eq!(one.stddev_ns, 0);

    // And: an empty distribution is not a distribution.
    assert!(BenchmarkSamples::from_samples(Vec::new()).is_err());
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

// ---------------------------------------------------------------------------
// The domain state must agree with the effects the stages committed. Every case
// below was an accepted import before this pass: the record was validated
// against the trace store thoroughly, and the state was read on the snapshot's
// word.
// ---------------------------------------------------------------------------

/// Both clocks are snapshot data, so requiring them to agree with each other is
/// not enough — every bootstrap event sits at simulation time zero, so a record
/// could move both and turn off every bootstrap-time check at once.
#[test]
fn a_snapshot_claiming_to_have_advanced_with_no_later_event_is_rejected() {
    let mut data = snapshot_of(populated_config(4_154));
    assert_eq!(
        data.traces
            .events
            .iter()
            .map(|event| event.time)
            .max()
            .expect("bootstrap commits events"),
        SimulationTime::new(0),
        "every bootstrap event is committed at simulation time zero"
    );

    // When: both clocks move together, and the state is gutted behind them.
    data.recipe.completed_time = SimulationTime::new(1);
    data.physical_counters.advanced_through = SimulationTime::new(1);
    data.population.aggregates.clear();
    data.population.aggregate_actor_pool.clear();
    data.actors_objective.actors.clear();
    data.actors_objective.actor_ancestry.clear();
    data.actors_subjective.actors.clear();

    rejects(
        data,
        "snapshot claims to have advanced with no event committed after bootstrap",
    );
}

/// Conservation as a single sum is one equation in two unknowns: deleting every
/// actor and adding the same number to an aggregate balances it exactly.
#[test]
fn deleting_promoted_actors_and_inflating_an_aggregate_is_rejected() {
    let mut data = snapshot_of(populated_config(4_155));
    let promoted = data.actors_objective.actors.len() as u64;
    assert_eq!(promoted, 8);

    data.actors_objective.actors.clear();
    data.actors_objective.actor_ancestry.clear();
    data.actors_objective.actor_objects.clear();
    data.actors_subjective.actors.clear();
    data.population.aggregate_actor_pool.clear();
    data.population.aggregates[0].count += promoted;

    rejects(
        data,
        "promoted actor count does not match the promotions the run committed",
    );
}

#[test]
fn a_population_aggregate_outside_the_active_chunks_is_rejected() {
    let mut data = snapshot_of(populated_config(4_156));
    data.population.aggregates[0].chart = causafera_types::ChartChunkCoord::new(
        data.recipe.config.chart_id,
        causafera_types::ChunkCoord::new(9_999, 9_999, 9_999),
    );
    rejects(data, "population aggregate outside active chunks");
}

/// A bootstrap-time store ends at the last stage completion, so any event after
/// it contradicts the clock that says nothing has happened yet.
///
/// Named for the prefix guard it actually reaches, not for the aggregate
/// ancestry the mutation also breaks: the ancestry guard is reached and covered
/// separately below, because a test that never gets that far cannot protect it.
#[test]
fn a_bootstrap_time_snapshot_with_a_trailing_event_is_rejected() {
    let mut data = snapshot_of(populated_config(4_157));
    let mut forged = data
        .traces
        .events
        .iter()
        .find(|event| {
            event.kind.raw() != BOOTSTRAP_STAGE_COMPLETION_EVENT_KIND && !event.effects.is_empty()
        })
        .expect("bootstrap commits ordinary effects")
        .clone();
    forged.trace_id = TraceId::new(data.traces.next_trace_id);
    forged.event_id = causafera_types::EventId::new(data.traces.next_event_id);
    forged.causes.clear();
    data.traces.next_trace_id += 1;
    data.traces.next_event_id += 1;
    data.traces.events.push(forged.clone());
    data.population.aggregates[0].causal_ancestry = vec![forged.trace_id];

    rejects(
        data,
        "bootstrap-time snapshot holds events after the last stage completion",
    );
}

/// The active-chunk check on material surfaces was one-directional: every
/// surface had to be in an active chunk, but no active chunk had to have one.
#[test]
fn deleting_a_material_surface_is_rejected() {
    let mut data = snapshot_of(populated_config(4_158));
    assert_eq!(data.material_surfaces.records.len(), 9);
    let removed = data.material_surfaces.records.remove(0).id;
    data.material_surfaces
        .transitions
        .retain(|transition| transition.id != removed);
    data.material_surfaces
        .pending_physical_changes
        .retain(|id| *id != removed);

    // A chunk losing its surface is not a bootstrap-only impossibility: surfaces
    // are never destroyed, so this is caught at every simulation time.
    rejects(data, "an active chunk has no material surface");

    // The bootstrap-time exact-equality check keeps the other direction, but it
    // is not reachable through a fabricated surface and this test does not
    // pretend otherwise: `validate_material_surface_last_transition` requires
    // every surface to be anchored to a committed effect naming that exact
    // surface, so an invented one is rejected before the count is ever compared.
}

/// Reservoir validation read the trace's phase and kind but never its payload,
/// while stage six commits exactly that payload.
#[test]
fn a_forged_thermal_reservoir_budget_or_target_is_rejected() {
    let mut budget = snapshot_of(populated_config(4_159));
    budget.thermal.reservoirs[0].budget = 999_999;
    rejects(
        budget,
        "thermal reservoir does not match the transition that created it",
    );

    let mut target = snapshot_of(populated_config(4_159));
    target.thermal.reservoirs[0].target.cell_index = 3;
    rejects(
        target,
        "thermal reservoir does not match the transition that created it",
    );
}

#[test]
fn actor_ancestry_that_does_not_match_the_actor_set_is_rejected() {
    let mut orphan = snapshot_of(populated_config(4_160));
    let ancestry = orphan.actors_objective.actor_ancestry[0].1.clone();
    orphan
        .actors_objective
        .actor_ancestry
        .push((causafera_runtime::ActorId::new(9_999), ancestry));
    rejects(
        orphan,
        "actor ancestry does not cover exactly the promoted actors",
    );

    let mut missing = snapshot_of(populated_config(4_160));
    missing.actors_objective.actor_ancestry.pop();
    rejects(
        missing,
        "actor ancestry does not cover exactly the promoted actors",
    );
}

/// Rolling the identifier counters back makes the next commit re-issue live IDs.
/// Both arrays are binary-searched, so an unsorted store resolves provenance to
/// the wrong event — silently, and only failing to re-import one save later.
#[test]
fn rolled_back_trace_identifier_counters_are_rejected() {
    let mut data = snapshot_of(populated_config(4_161));
    data.traces.next_trace_id = 0;
    data.traces.next_event_id = 0;
    rejects(data, "trace store failed validation");
}

/// The same rollback on the counter the fix above did not cover. `promote_actor`
/// issues `ActorId::new(next_actor_id)` into a map, so a rolled-back counter
/// makes the next promotion replace a live actor instead of adding one:
/// residents disappear with no death event and the state re-exports clean.
#[test]
fn a_rolled_back_actor_identifier_counter_is_rejected() {
    let mut data = snapshot_of(populated_config(4_163));
    assert_eq!(data.physical_counters.next_actor_id, 9);
    data.physical_counters.next_actor_id = 1;
    rejects(data, "next actor identifier is not above every live actor");

    // The boundary is exclusive: the counter must be above the last live actor,
    // not equal to it.
    let mut boundary = snapshot_of(populated_config(4_163));
    boundary.physical_counters.next_actor_id = 8;
    rejects(
        boundary,
        "next actor identifier is not above every live actor",
    );
}

/// An actor's physical object is what perception reads. Deleting the map left
/// the actors present to the scheduler and invisible to one another.
#[test]
fn actor_objects_that_do_not_cover_the_actors_are_rejected() {
    let mut missing = snapshot_of(populated_config(4_164));
    assert_eq!(missing.actors_objective.actor_objects.len(), 8);
    missing.actors_objective.actor_objects.clear();
    rejects(
        missing,
        "actor objects do not cover exactly the promoted actors",
    );

    let mut orphan = snapshot_of(populated_config(4_164));
    let mut extra = orphan.actors_objective.actor_objects[0].1;
    extra.object_key = 9_999;
    orphan.actors_objective.actor_objects.push((9_999, extra));
    rejects(
        orphan,
        "actor objects do not cover exactly the promoted actors",
    );
}

/// The pool names actors by identity and is folded into `state_digest`, so an
/// actor that does not exist is digest content no run can produce.
#[test]
fn an_aggregate_actor_pool_naming_a_phantom_actor_is_rejected() {
    let mut data = snapshot_of(populated_config(4_165));
    assert!(data.population.aggregate_actor_pool.is_empty());
    let chart = data.population.aggregates[0].chart;
    data.population
        .aggregate_actor_pool
        .push((chart, vec![causafera_runtime::ActorId::new(70_001)]));
    rejects(
        data,
        "aggregate actor pool names an actor that does not exist",
    );
}

/// The guard that bounds what may claim bootstrap provenance. Reaching it needs
/// a trace that exists, is not a stage effect, and does not first trip the
/// prefix guard — the runtime root trace is all three.
#[test]
fn an_aggregate_whose_ancestry_is_not_a_stage_effect_is_rejected() {
    let mut root = snapshot_of(populated_config(4_166));
    root.population.aggregates[0].causal_ancestry = vec![TraceId::new(0)];
    rejects(
        root,
        "population aggregate ancestry is not a bootstrap stage effect",
    );

    // A stage *completion* is committed by the run and named by the record, but
    // it is a terminal receipt rather than one of the effects a stage produced,
    // so it cannot lend provenance either.
    let mut completion = snapshot_of(populated_config(4_166));
    let terminal = completion.bootstrap.receipts[0].trace;
    completion.population.aggregates[0].causal_ancestry = vec![terminal];
    rejects(
        completion,
        "population aggregate ancestry is not a bootstrap stage effect",
    );
}

/// Two shapes the commit path forbids that import accepted, both of which made a
/// forged trailing event cheap: `CausalEventProposal::new` rejects an empty
/// effect set, and commits advance a tick at a time so store order is time order.
#[test]
fn a_trace_store_holding_events_the_commit_path_forbids_is_rejected() {
    let mut effectless = snapshot_of(populated_config(4_167));
    let mut padding = effectless.traces.events[0].clone();
    padding.trace_id = TraceId::new(effectless.traces.next_trace_id);
    padding.event_id = causafera_types::EventId::new(effectless.traces.next_event_id);
    padding.causes.clear();
    padding.effects.clear();
    effectless.traces.next_trace_id += 1;
    effectless.traces.next_event_id += 1;
    effectless.traces.events.push(padding);
    rejects(effectless, "trace store failed validation");

    // Times running 0…0, 5, 1 hide the later event behind the larger one, which
    // every bound phrased as `any(time > limit)` then misses.
    let mut unordered = snapshot_of(populated_config(4_167));
    for time in [5u64, 1] {
        let mut event = unordered.traces.events[0].clone();
        event.trace_id = TraceId::new(unordered.traces.next_trace_id);
        event.event_id = causafera_types::EventId::new(unordered.traces.next_event_id);
        event.causes.clear();
        event.time = SimulationTime::new(time);
        unordered.traces.next_trace_id += 1;
        unordered.traces.next_event_id += 1;
        unordered.traces.events.push(event);
    }
    rejects(unordered, "trace store failed validation");
}

/// The whole point of moving these out of the bootstrap-only scope: a snapshot
/// that appends a real event and claims to have advanced no longer disables
/// them. The population identity holds at every simulation time, so gutting the
/// world behind an advanced clock is caught rather than waved through.
#[test]
fn an_advanced_snapshot_cannot_disable_the_persistent_domain_checks() {
    let mut data = snapshot_of(populated_config(4_168));
    let mut appended = data
        .traces
        .events
        .iter()
        .find(|event| {
            event.kind.raw() != BOOTSTRAP_STAGE_COMPLETION_EVENT_KIND && !event.effects.is_empty()
        })
        .expect("bootstrap commits ordinary effects")
        .clone();
    appended.trace_id = TraceId::new(data.traces.next_trace_id);
    appended.event_id = causafera_types::EventId::new(data.traces.next_event_id);
    appended.causes.clear();
    appended.time = SimulationTime::new(1);
    data.traces.next_trace_id += 1;
    data.traces.next_event_id += 1;
    data.traces.events.push(appended);
    data.recipe.completed_time = SimulationTime::new(1);
    data.physical_counters.advanced_through = SimulationTime::new(1);

    // The store now ends past the last completion, so every bootstrap-scoped
    // check is out of scope by construction. What is left must still hold.
    let mut gutted = data.clone();
    gutted.population.aggregates.clear();
    gutted.actors_objective.actors.clear();
    gutted.actors_objective.actor_ancestry.clear();
    gutted.actors_objective.actor_objects.clear();
    gutted.actors_subjective.actors.clear();
    rejects(
        gutted,
        "living population does not match the bootstrap population plus births minus deaths",
    );

    let mut surface = data.clone();
    let removed = surface.material_surfaces.records.remove(0).id;
    surface
        .material_surfaces
        .transitions
        .retain(|transition| transition.id != removed);
    surface
        .material_surfaces
        .pending_physical_changes
        .retain(|id| *id != removed);
    rejects(surface, "an active chunk has no material surface");

    // And the control: the same appended event with the world left alone is
    // accepted, so the checks above are rejecting the forgery and not the
    // advancement.
    assert!(
        RuntimeState::import_snapshot(data).is_ok(),
        "an advanced snapshot with consistent domain state must still import"
    );
}

/// The negative control for every guard above, and the one this file was
/// missing: each round of hardening tightens what import accepts, and a check
/// phrased slightly too strongly does not fail loudly — it makes a legitimate
/// save unloadable, which no adversarial test can detect.
///
/// So the whole configuration space is swept, at bootstrap and after
/// advancement, through the three paths a real save actually takes. Anything
/// the runtime itself produced must import.
#[test]
fn every_state_the_runtime_produces_survives_import() {
    let mut configs = vec![RuntimeConfig::new(1), populated_config(2)];
    for population in [0_u64, 1, 7, 10_000] {
        let mut config = populated_config(3);
        config.bootstrap_population = population;
        configs.push(config);
    }
    // Actor and sensor counts, including none of either, and the cue budget's
    // upper corner.
    for (actors, sensors) in [(0_u8, 0_u8), (0, 1), (8, 0), (4, 2), (13, 4)] {
        let mut config = populated_config(4);
        config.actor_count = actors;
        config.sensor_count = sensors;
        configs.push(config);
    }
    // Both chart shapes across the radius range, and a wider chunk extent.
    for radius in 0_u8..=2 {
        for shape in [
            causafera_runtime::ActiveChunkShape::Line,
            causafera_runtime::ActiveChunkShape::Area,
        ] {
            let mut config = populated_config(5);
            config.active_chunk_radius = radius;
            config.active_chunk_shape = shape;
            config.actor_count = 2;
            configs.push(config);
        }
    }
    let mut wide = populated_config(6);
    wide.chunk_extent = 4;
    configs.push(wide);

    // Some combinations are not valid worlds at all — more promoted actors than
    // residents, for one — and the runtime refuses to build them. That is a
    // configuration verdict, not an import verdict, so those are skipped rather
    // than counted; the assertion below keeps the skip from hollowing the sweep.
    let mut exercised = 0_usize;
    for config in configs {
        let Ok(mut runtime) = Runtime::new(config) else {
            continue;
        };
        exercised += 1;
        let mut reached = 0_u64;
        // Tick zero is the bootstrap prefix; the rest are past it, where the
        // lifecycle has moved aggregates, demoted actors, and advanced clocks.
        for checkpoint in [0_u64, 1, 5, 13, 17] {
            if checkpoint > reached {
                runtime
                    .run_ticks(checkpoint - reached)
                    .expect("a production run must advance");
                reached = checkpoint;
            }
            let data = runtime.export_snapshot().expect("state must export");

            RuntimeState::import_snapshot(data.clone())
                .expect("import must accept a self-produced snapshot");

            let envelope = assemble_envelope(&data).expect("envelope must assemble");
            let round_tripped = disassemble_envelope(&envelope).expect("envelope must disassemble");
            RuntimeState::import_snapshot(round_tripped)
                .expect("import must accept an envelope round trip");

            // Resume, advance, and re-export: the state a second save is made
            // from is the one a tightened guard is likeliest to reject.
            let mut resumed = Runtime::from_snapshot(data).expect("snapshot must resume");
            resumed.run_ticks(3).expect("a resumed run must advance");
            let second = resumed
                .export_snapshot()
                .expect("resumed state must export");
            RuntimeState::import_snapshot(second)
                .expect("import must accept an export-resume-export chain");
        }
    }
    assert!(
        exercised >= 14,
        "the sweep must cover the configuration space, not skip it: {exercised} built"
    );
}
