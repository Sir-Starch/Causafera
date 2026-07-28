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
    RuntimeState, THERMAL_RESERVOIR_PROCESS_SCHEMA, assemble_envelope, disassemble_envelope,
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
