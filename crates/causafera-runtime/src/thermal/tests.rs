use super::*;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use causafera_core::{EvidenceDecoderRegistry, EvidenceResolution, HistoryQuery, HistoryRecord};
use causafera_persistence::{DurabilityMode, HistoryStore, PersistenceError};

struct ThermalHistoryBundle {
    path: PathBuf,
}

impl ThermalHistoryBundle {
    fn new(name: &str) -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let ordinal = NEXT.fetch_add(1, Ordering::Relaxed);
        Self {
            path: std::env::temp_dir().join(format!(
                "causafera-thermal-evidence-{name}-{}-{ordinal}",
                std::process::id()
            )),
        }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for ThermalHistoryBundle {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

fn runtime_with_durability(
    seed: u64,
    name: &str,
    mode: DurabilityMode,
) -> (Runtime, ThermalHistoryBundle) {
    let runtime = Runtime::new(RuntimeConfig::new(seed)).expect("runtime must bootstrap");
    let bundle = ThermalHistoryBundle::new(name);
    let store = HistoryStore::create(
        bundle.path(),
        crate::history_transaction::runtime_history_namespace(seed),
        mode,
    )
    .expect("history store must create");
    {
        let mut state = runtime.state.lock().expect("runtime state");
        let hot_budget = state.traces.hot_budget();
        let (history, traces) =
            crate::history_transaction::RuntimeHistoryCoordinator::bootstrap_with_store(
                &state.traces,
                seed,
                hot_budget,
                store,
            )
            .expect("history coordinator must install");
        state.history = history;
        state.traces = traces;
    }
    (runtime, bundle)
}

fn thermal_evidence_from_reader(runtime: &Runtime) -> Vec<DomainEvidenceRecord> {
    let reader = runtime.history_reader().expect("history reader");
    let mut offset = 0;
    let mut evidence = Vec::new();
    loop {
        let page = reader
            .indexed_range(HistoryQuery::new(offset, 256, 4 * 1024 * 1024).expect("bounded query"))
            .expect("history page");
        evidence.extend(page.records().iter().filter_map(|record| match record {
            HistoryRecord::Evidence(record)
                if matches!(
                    record.schema_id(),
                    THERMAL_TRANSFER_EVIDENCE_SCHEMA
                        | THERMAL_CONSERVATION_EVIDENCE_SCHEMA
                        | THERMAL_BOUNDARY_EVIDENCE_SCHEMA
                ) =>
            {
                Some(record.clone())
            }
            HistoryRecord::Causal(_) | HistoryRecord::Evidence(_) => None,
        }));
        match page.next_offset() {
            Some(next) => offset = next,
            None => return evidence,
        }
    }
}

fn first_thermal_evidence(runtime: &Runtime) -> DomainEvidenceRecord {
    thermal_evidence_from_reader(runtime)
        .into_iter()
        .find(|record| record.schema_id() == THERMAL_TRANSFER_EVIDENCE_SCHEMA)
        .expect("thermal transfer evidence")
}

#[test]
fn atomic_failure_rollback() {
    // Given: a real pending injection whose reservoir anchor names an absent prior trace.
    let runtime = Runtime::new(RuntimeConfig::new(1_700)).expect("runtime must bootstrap");
    let state = Arc::clone(&runtime.state);
    let mut reservoir_system = ThermalReservoirSystem::new(Arc::clone(&state));
    reservoir_system
        .execute()
        .expect("reservoir proposal must succeed");

    let unknown_cause;
    let before_traces;
    let before_fields;
    let before_reservoirs;
    let before_active_region;
    let before_parameters;
    let before_receipts;
    let before_conservation_receipts;
    let before_pending_injections;
    let before_boundary_records;
    let before_failure;
    let before_material_surfaces;
    let before_material_transitions;
    {
        let mut state = state.lock().expect("runtime state must be available");
        unknown_cause = TraceId::new(state.traces.export_snapshot().next_trace_id);
        assert!(
            state
                .traces
                .event(unknown_cause)
                .expect("history read")
                .is_none()
        );
        let reservoir_id = state
            .pending_thermal_injections
            .first()
            .expect("production bootstrap must schedule an injection")
            .reservoir_id;
        state
            .thermal_reservoirs
            .get_mut(&reservoir_id)
            .expect("scheduled reservoir must exist")
            .last_change = unknown_cause;

        before_traces = state.traces.export_snapshot();
        before_fields = state.thermal_fields.clone();
        before_reservoirs = state.thermal_reservoirs.clone();
        before_active_region = state.thermal_active_region.clone();
        before_parameters = state.thermal_parameters;
        before_receipts = state.thermal_receipts.clone();
        before_conservation_receipts = state.thermal_conservation_receipts.clone();
        before_pending_injections = state.pending_thermal_injections.clone();
        before_boundary_records = state.thermal_boundary_records.clone();
        before_failure = state.failure.clone();
        before_material_surfaces = state.material_surfaces.clone();
        before_material_transitions = state.material_surface_thermal_transitions.clone();
    }
    let mut evolution_system = ThermalEvolutionSystem::new(Arc::clone(&state));
    let before_time = evolution_system.next_time;

    // When: the real thermal causal batch rejects the unknown prior cause.
    let error = evolution_system
        .execute()
        .expect_err("unknown cause must reject the thermal batch");

    // Then: the rejection leaves every authoritative thermal surface unchanged.
    assert_eq!(
        error,
        RuntimeError::CausalCommit(CausalCommitError::UnknownCause {
            key: EventProposalKey::new(THERMAL_EVOLUTION_SYSTEM_ID, 0, 0),
            cause: unknown_cause,
        })
    );
    let state = state.lock().expect("runtime state must remain available");
    assert_eq!(state.traces.export_snapshot(), before_traces);
    assert_eq!(state.thermal_fields, before_fields);
    assert_eq!(state.thermal_reservoirs, before_reservoirs);
    assert_eq!(state.thermal_active_region, before_active_region);
    assert_eq!(state.thermal_parameters, before_parameters);
    assert_eq!(state.thermal_receipts, before_receipts);
    assert_eq!(
        state.thermal_conservation_receipts,
        before_conservation_receipts
    );
    assert_eq!(state.pending_thermal_injections, before_pending_injections);
    assert_eq!(state.thermal_boundary_records, before_boundary_records);
    assert_eq!(state.failure, before_failure);
    assert_eq!(state.material_surfaces, before_material_surfaces);
    assert_eq!(
        state.material_surface_thermal_transitions,
        before_material_transitions
    );
    assert_eq!(evolution_system.next_time, before_time);
}

#[test]
fn second_material_exchange_cites_prior_tick_cell_trace_not_current() {
    // Given: a production runtime whose first tick both diffuses and exchanges material heat.
    let mut runtime = Runtime::new(RuntimeConfig::new(1_702)).expect("runtime must bootstrap");
    runtime.tick().expect("first thermal batch must commit");
    let (first_transition, first_field_trace) = {
        let state = runtime
            .state
            .lock()
            .expect("runtime state must be available");
        let transition = *state
            .material_surface_thermal_transitions
            .last()
            .expect("first tick must record at least one material exchange");
        let field_trace = state
            .thermal_fields
            .field(transition.id.chunk)
            .and_then(|field| {
                field
                    .last_change()
                    .get(usize::from(transition.id.cell_index))
            })
            .copied()
            .expect("material cell must have a field anchor");
        (transition, field_trace)
    };

    // When: a second tick exchanges material heat again for the same surface.
    runtime.tick().expect("second thermal batch must commit");

    // Then: the second exchange cites the trace the field anchor held *before* this tick's own
    // mutation, not a trace this tick just produced for the same cell.
    let state = runtime
        .state
        .lock()
        .expect("runtime state must be available");
    let second_transition = *state
        .material_surface_thermal_transitions
        .iter()
        .rev()
        .find(|transition| transition.id == first_transition.id)
        .expect("second tick must record another exchange for the same surface");
    assert_eq!(second_transition.cell_trace, first_field_trace);
    let second_event = state
        .traces
        .event(second_transition.transition_trace)
        .expect("history read failed")
        .expect("second transition trace must exist");
    assert!(
        second_event
            .causes()
            .contains(&first_transition.transition_trace),
        "second exchange must chain onto the first exchange as its prior parent"
    );
}

#[test]
fn current_batch_boundary_records_replace_prior_batch() {
    // Given: a production runtime that has committed one thermal batch.
    let mut runtime = Runtime::new(RuntimeConfig::new(1_701)).expect("runtime must bootstrap");
    runtime.tick().expect("first thermal batch must commit");
    let first_batch = runtime
        .state
        .lock()
        .expect("runtime state must be available")
        .thermal_boundary_records
        .clone();
    assert!(!first_batch.is_empty());

    // When: a second thermal batch commits with a different frozen pre-state.
    runtime.tick().expect("second thermal batch must commit");

    // Then: runtime retains exactly the current batch, rather than both batches.
    let second_batch = runtime
        .state
        .lock()
        .expect("runtime state must be available")
        .thermal_boundary_records
        .clone();
    assert_eq!(second_batch.len(), first_batch.len());
    assert_ne!(second_batch, first_batch);
}

#[test]
fn thermal_evidence_rejects_malformed_stale_and_unknown_required_records() {
    // Given: one committed thermal record and a new enclosing transaction.
    let mut runtime = Runtime::new(RuntimeConfig::new(1_706)).expect("runtime must bootstrap");
    runtime.tick().expect("thermal batch must commit");
    let record = first_thermal_evidence(&runtime);
    let anchor = record.causal_anchors()[0];
    let duplicate_anchor = DomainEvidenceRecord::new(
        record.commit_sequence(),
        record.schema_id(),
        record.version(),
        record.criticality(),
        record.time(),
        vec![anchor, anchor],
        b"duplicate-anchor".to_vec(),
        record.canonical_payload().to_vec(),
        record.payload_digest(),
    );
    let mut state = runtime.state.lock().expect("runtime state");
    let _transaction = crate::RuntimeTickTransaction::open(
        &mut state,
        causafera_persistence::MAX_TOTAL_SIZE,
        causafera_geography::MAX_HYDROLOGY_SECTION_BYTES,
    )
    .expect("transaction must open");
    let mut malformed_payload = record.canonical_payload().to_vec();
    malformed_payload[0] ^= 1;
    let malformed = DomainEvidenceRecord::new(
        record.commit_sequence(),
        record.schema_id(),
        record.version(),
        record.criticality(),
        record.time(),
        record.causal_anchors().to_vec(),
        b"malformed-payload".to_vec(),
        malformed_payload,
        record.payload_digest(),
    )
    .expect("record shape remains canonical");
    let unknown_anchor_payload = b"unknown-anchor".to_vec();
    let unknown_anchor = DomainEvidenceRecord::new(
        record.commit_sequence(),
        record.schema_id(),
        record.version(),
        record.criticality(),
        record.time(),
        vec![TraceId::new(u64::MAX)],
        b"unknown-anchor".to_vec(),
        unknown_anchor_payload.clone(),
        StateFingerprint::new(*blake3::hash(&unknown_anchor_payload).as_bytes()),
    )
    .expect("record shape remains canonical");

    // When: malformed, unknown-anchor, and already-committed identities cross the staging boundary.
    let malformed_error = state
        .stage_history_evidence(std::slice::from_ref(&malformed))
        .expect_err("payload digest mismatch must fail closed");
    let unknown_anchor_error = state
        .stage_history_evidence(std::slice::from_ref(&unknown_anchor))
        .expect_err("unknown anchor must fail closed");
    let stale_error = state
        .stage_history_evidence(std::slice::from_ref(&record))
        .expect_err("committed schema/key identity must be stale");
    drop(state);
    let required = DomainEvidenceRecord::new(
        record.commit_sequence(),
        EvidenceSchemaId::new(0x0014_17ff),
        record.version(),
        Criticality::ReplayRequired,
        record.time(),
        record.causal_anchors().to_vec(),
        b"unknown-required".to_vec(),
        record.canonical_payload().to_vec(),
        record.payload_digest(),
    )
    .expect("required record shape remains canonical");
    let registry = EvidenceDecoderRegistry::new();

    // Then: every authoritative fault is typed, while unknown ExplanationOnly bytes stay raw.
    assert!(matches!(
        duplicate_anchor,
        Err(HistoryError::AnchorsNotStrictlyOrdered { .. })
    ));
    assert!(matches!(
        malformed_error,
        RuntimeError::SnapshotAssembly(PersistenceError::EvidencePayloadDigestMismatch { .. })
    ));
    assert!(matches!(
        unknown_anchor_error,
        RuntimeError::CausalCommit(CausalCommitError::HistoryRead(
            HistoryError::UnknownEvidenceAnchor { .. }
        ))
    ));
    assert!(matches!(
        stale_error,
        RuntimeError::CausalCommit(CausalCommitError::HistoryRead(
            HistoryError::DuplicateRecordKey
        ))
    ));
    assert!(matches!(
        registry.resolve(&required),
        Err(HistoryError::UnsupportedReplaySchema { .. })
    ));
    assert!(matches!(
        registry.resolve(&record),
        Ok(EvidenceResolution::Raw(_))
    ));
    eprintln!(
        "MARKER:thermal_evidence malformed_payload=rejected malformed_anchor=rejected unknown_anchor=rejected stale=rejected unknown_required=rejected explanation_unknown=raw"
    );
}

#[test]
fn thermal_evidence_rollback_retry_matches_control_in_both_durability_modes() {
    for (name, mode) in [
        ("group", DurabilityMode::GroupCommit),
        ("every-tick", DurabilityMode::EveryTick),
    ] {
        // Given: equivalent runtimes using the selected real history durability mode.
        let seed = 1_710 + u64::from(mode == DurabilityMode::EveryTick);
        let (mut runtime, _runtime_bundle) = runtime_with_durability(seed, name, mode);
        let (mut control, _control_bundle) =
            runtime_with_durability(seed, &format!("{name}-control"), mode);
        let before_status = runtime.history_status().expect("history status");
        let before_snapshot = runtime.export_snapshot().expect("snapshot must export");
        runtime
            .state
            .lock()
            .expect("runtime state")
            .history
            .inject_failure(Some(
                crate::history_transaction::HistoryCommitPhase::AfterStatePublication,
            ));

        // When: publication fails once, then the exact thermal tick is retried.
        let error = runtime
            .tick()
            .expect_err("injected publication failure must reject the tick");
        let failed_status = runtime.history_status().expect("failed history status");
        let failed_snapshot = runtime
            .export_snapshot()
            .expect("failed snapshot must export");
        let failed_evidence = thermal_evidence_from_reader(&runtime);
        runtime
            .state
            .lock()
            .expect("runtime state")
            .history
            .inject_failure(None);
        runtime.tick().expect("retry must commit");
        control.tick().expect("control must commit");

        // Then: no failed evidence leaks and retry equals the direct control in this mode.
        assert!(matches!(error, RuntimeError::HistoryCommitRejected { .. }));
        assert_eq!(failed_status, before_status);
        assert_eq!(failed_snapshot, before_snapshot);
        assert!(failed_evidence.is_empty());
        assert_eq!(
            runtime.history_status().expect("retry status"),
            control.history_status().expect("control status")
        );
        assert_eq!(
            runtime.export_snapshot().expect("retry snapshot"),
            control.export_snapshot().expect("control snapshot")
        );
        assert_eq!(
            thermal_evidence_from_reader(&runtime),
            thermal_evidence_from_reader(&control)
        );
        eprintln!(
            "MARKER:thermal_evidence durability={name} rollback_clean=true retry_matches_control=true"
        );
    }
}
