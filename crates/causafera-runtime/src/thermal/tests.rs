use super::*;

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
    {
        let mut state = state.lock().expect("runtime state must be available");
        unknown_cause = TraceId::new(state.traces.export_snapshot().next_trace_id);
        assert!(state.traces.event(unknown_cause).is_none());
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
    assert_eq!(evolution_system.next_time, before_time);
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
