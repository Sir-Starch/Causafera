//! The cross-cutting invariants of a whole hydrology state.
//!
//! Decoding already rebuilds every collection through its validating constructor,
//! so a malformed section cannot become state. What is left is the agreement a
//! single constructor cannot see: that the addressing is a bijection over the
//! carriers it addresses, that every retained ledger closes, that the forcing
//! schedule is consistent with the runtime time it is read at, and that receipts
//! recompute to the aggregates they claim.
//!
//! Checked at import, and again at every hydrology-enabled tick boundary. The two
//! are the same question: a tick that produced a state import would refuse has
//! produced a state this session could not resume from.
//!
//! See `plans/hydrology.md` verification gates V18 and V25.

use causafera_core::{Phase, provenance::CausalTraceStore};
use causafera_domains::{
    HydrologyReceiptTotals, validate_boundary_transfers, validate_paired_transfers,
};
use causafera_types::TraceId;

use crate::{
    HYDROLOGY_BOOTSTRAP_EVENT_KIND, HYDROLOGY_CONSERVATION_EVENT_KIND, HydrologyConfig,
    HydrologyRuntimeState, RuntimeError,
};

/// Check everything about a hydrology state that no single constructor could have
/// checked on its own.
pub(crate) fn validate_hydrology_state(
    state: &HydrologyRuntimeState,
    configured: bool,
    runtime_time: u64,
) -> Result<(), RuntimeError> {
    // The recipe and the section have to agree about whether this world has water.
    // If they disagree one of them is lying and import cannot tell which, so it
    // refuses both rather than picking the more convenient one.
    if state.enabled != configured {
        return Err(RuntimeError::HydrologyStateDisagreesWithRecipe);
    }
    if !state.enabled {
        // A disabled domain must be canonically empty, for the same reason a
        // disabled configuration must be: a snapshot carrying water it says it does
        // not have is a snapshot whose author believed something the runtime does
        // not.
        if !state.fields.fields().is_empty()
            || !state.conveyance.is_empty()
            || !state.boundaries.is_empty()
            || !state.forcing.is_empty()
            || !state.retained_batches.is_empty()
            || state.next_node_id != 1
        {
            return Err(RuntimeError::HydrologyDisabledStateNotCanonical);
        }
        return Ok(());
    }

    // The registry is what every causal target in the store was written against, so
    // an ordinal that is missing, duplicated, or out of range would let two
    // carriers share one target.
    if !state.registry.is_dense() {
        return Err(RuntimeError::HydrologyRegistryNotDense);
    }
    if state.registry.cells().len() != state.fields.cell_count() {
        return Err(RuntimeError::HydrologyRegistryIncomplete);
    }
    for cell in state.registry.cells().keys() {
        if !state.fields.is_resident(*cell) {
            return Err(RuntimeError::HydrologyRegistryIncomplete);
        }
    }
    if state.registry.edges().len() != state.conveyance.len() {
        return Err(RuntimeError::HydrologyRegistryIncomplete);
    }
    for edge in state.registry.edges().keys() {
        if state.conveyance.edge(*edge).is_none() {
            return Err(RuntimeError::HydrologyRegistryIncomplete);
        }
    }
    if state.registry.forcing().len() != state.forcing.len() {
        return Err(RuntimeError::HydrologyRegistryIncomplete);
    }
    for record in &state.forcing {
        if !state
            .registry
            .forcing()
            .contains_key(&(record.scheduled_tick(), record.forcing_id()))
        {
            return Err(RuntimeError::HydrologyRegistryIncomplete);
        }
    }
    if state.registry.resolution().len() != state.resolution.len() {
        return Err(RuntimeError::HydrologyRegistryIncomplete);
    }

    // Every resident chunk needs a metric and a resolution entry; hydrology cannot
    // evaluate a chunk whose scale or detail level it does not know.
    for chunk in state.fields.fields().keys() {
        state.metrics.get(chunk.chart)?;
        if !state.resolution.contains_key(chunk) {
            return Err(RuntimeError::HydrologyResolutionEntryMissing);
        }
        if !state.active.resident_chunks().contains(chunk) {
            return Err(RuntimeError::HydrologyResidencyMismatch);
        }
    }
    if state.active.resident_chunks().len() != state.fields.fields().len() {
        return Err(RuntimeError::HydrologyResidencyMismatch);
    }
    for entry in state.resolution.values() {
        if entry.level() > state.resolution_policy.max_level {
            return Err(RuntimeError::HydrologyResolutionLevelUnsupported {
                level: entry.level(),
            });
        }
    }

    // Both endpoints and the outlet of every conveyance edge. The graph's own
    // constructor checks adjacency and one-outgoing-edge-per-cell, but it holds
    // no field set and so cannot ask whether those cells exist. Routing settles
    // a release directly into the outlet's storage, so an edge reaching outside
    // the field set is not an edge to somewhere far away — it is a write with
    // nowhere to land, and the solver's residency expectation would abort the
    // process on the first tick that released through it.
    for edge in state.conveyance.edges().values() {
        let key = edge.key();
        if !state.fields.is_resident(key.low())
            || !state.fields.is_resident(key.high())
            || !state.fields.is_resident(edge.outlet())
        {
            return Err(RuntimeError::HydrologyConveyanceCellNotResident);
        }
    }

    // The forcing schedule against the time it is read at. A pending record
    // scheduled in the past would never apply, and an applied record whose
    // timestamp is not its scheduled tick is a record of something that did not
    // happen.
    //
    // Residency of a record's targets is deliberately *not* checked here. The
    // contract is residency at the record's scheduled tick, which the proposal
    // enforces when the record fires; requiring it now would reject a pending
    // record for ground that becomes resident before then, and would make a
    // legally configured session unexportable while still being tickable.
    let mut seen = std::collections::BTreeSet::new();
    for record in &state.forcing {
        if !seen.insert((record.scheduled_tick(), record.forcing_id())) {
            return Err(RuntimeError::HydrologyForcingScheduleNotCanonical);
        }
        match record.applied_at() {
            Some(applied) if applied != record.scheduled_tick() => {
                return Err(RuntimeError::HydrologyForcingAppliedAtMismatch);
            }
            Some(applied) if applied > runtime_time => {
                return Err(RuntimeError::HydrologyForcingAppliedInTheFuture);
            }
            None if record.scheduled_tick() <= runtime_time => {
                return Err(RuntimeError::HydrologyForcingMissedItsTick);
            }
            _ => {}
        }
    }

    // Every retained ledger closes, and its aggregates recompute from the
    // per-transfer receipts rather than being believed.
    //
    // The window is also checked for continuity as it is walked. Retention
    // pushes the newest batch and evicts whole ones from the front, so the
    // retained sequence numbers are consecutive, ascending, and end at the field
    // set's own counter. A gap in the middle is not a batch that was evicted —
    // eviction cannot reach the middle — it is a batch that was removed from the
    // record, and the ledger either side of it would then attest to a
    // storage-before it never saw.
    let mut previous: Option<(u64, u64)> = None;
    for trace in &state.retained_batches {
        let receipts = state
            .receipts
            .get(trace)
            .ok_or(RuntimeError::HydrologyRetainedBatchIncomplete)?;
        let ledger = state
            .conservation_receipts
            .get(trace)
            .ok_or(RuntimeError::HydrologyRetainedBatchIncomplete)?;
        ledger.require_balanced()?;
        validate_paired_transfers(receipts)?;
        validate_boundary_transfers(receipts)?;
        if !HydrologyReceiptTotals::from_receipts(receipts)?.agrees_with(ledger) {
            return Err(RuntimeError::HydrologyReceiptsDisagreeWithLedger);
        }

        // Checked, because the sequence arrives from a snapshot: a forged
        // `u64::MAX` would otherwise decide the question by overflowing here.
        if let Some((sequence, tick)) = previous
            && (Some(ledger.batch_sequence()) != sequence.checked_add(1) || ledger.tick() <= tick)
        {
            return Err(RuntimeError::HydrologyBatchSequenceNotContinuous);
        }
        previous = Some((ledger.batch_sequence(), ledger.tick()));

        // A receipt filed under a batch it does not belong to would let one
        // tick's transfers be read as another's, and the ledger comparison above
        // would still pass because the totals are computed from whatever is in
        // the list.
        for receipt in receipts {
            if receipt.batch_sequence() != ledger.batch_sequence()
                || receipt.tick() != ledger.tick()
            {
                return Err(RuntimeError::HydrologyReceiptBatchMismatch);
            }
        }
    }
    // The newest retained batch is the one the field set counts, and a state
    // that has committed batches must retain at least the newest one.
    match previous {
        Some((sequence, _)) if sequence != state.fields.batch_sequence() => {
            return Err(RuntimeError::HydrologyBatchSequenceNotContinuous);
        }
        None if state.fields.batch_sequence() != 0 => {
            return Err(RuntimeError::HydrologyBatchSequenceNotContinuous);
        }
        _ => {}
    }
    if state.receipts.len() != state.retained_batches.len() {
        return Err(RuntimeError::HydrologyRetainedBatchIncomplete);
    }
    Ok(())
}

/// Everything an imported hydrology state must additionally satisfy.
///
/// Split from [`validate_hydrology_state`] because these are not questions a
/// tick can change the answer to. Configuration and the trace store are fixed
/// for the life of a session, and the bootstrap stage is the only thing that
/// ever writes a forcing record's ancestry — so a running tick re-deriving them
/// every time would be spending the largest state in the runtime to confirm
/// something it could not have altered. A snapshot is a different matter: every
/// one of these arrived as bytes from outside.
///
/// See `plans/hydrology.md` §11 and verification gate V25.
pub(crate) fn validate_imported_hydrology(
    state: &HydrologyRuntimeState,
    config: &HydrologyConfig,
    traces: &CausalTraceStore,
) -> Result<(), RuntimeError> {
    if !state.enabled {
        return Ok(());
    }

    // The schedule's own invariants. The runtime holds its forcing as a plain
    // record vector so that applied records can be marked in place, which meant
    // nothing on the import path ever asked the schedule constructor's
    // questions: ordering, the aggregate member cap, and the two origin fan-in
    // bounds that keep the terminal conservation event inside its cause limit.
    causafera_geography::validate_forcing_records(&state.forcing)?;

    // The grid metric decides what every volume in the section means as a depth.
    // A section carrying its own metric alongside a recipe that declares another
    // is a section that would evaluate a different world than the one this
    // configuration bootstrapped, using the numbers of the one it did not.
    if state.metrics.entries() != &config.grid_metrics {
        return Err(RuntimeError::HydrologyStateDisagreesWithConfiguration {
            what: "grid metrics",
        });
    }

    // The resolution policy is what every imported level is checked against, so
    // a section carrying its own would be the only thing deciding whether its
    // own detail is acceptable. The contract is that a level above the policy
    // refuses the tick rather than being clamped; a section that raises the
    // policy to fit is the same clamp, done one layer earlier.
    if state.resolution_policy != config.resolution_policy {
        return Err(RuntimeError::HydrologyStateDisagreesWithConfiguration {
            what: "resolution policy",
        });
    }

    // Records are installed one-for-one from the configured specs, in order, so
    // the correspondence is exact. `applied_at` is deliberately not compared:
    // that is the one field a run is supposed to change.
    if state.forcing.len() != config.forcing_schedule.len() {
        return Err(RuntimeError::HydrologyStateDisagreesWithConfiguration {
            what: "forcing schedule length",
        });
    }
    for (record, spec) in state.forcing.iter().zip(&config.forcing_schedule) {
        let targets_agree = record.targets().len() == spec.targets.len()
            && record
                .targets()
                .iter()
                .zip(&spec.targets)
                .all(|(member, (cell, weight))| member.cell == *cell && member.weight == *weight);
        if record.forcing_id() != spec.forcing_id
            || record.scheduled_tick() != spec.scheduled_tick
            || record.precipitation_volume() != spec.precipitation_volume
            || record.potential_et_volume() != spec.potential_et_volume
            || record.external_inflow_volume() != spec.external_inflow_volume
            || !targets_agree
        {
            return Err(RuntimeError::HydrologyStateDisagreesWithConfiguration {
                what: "forcing records",
            });
        }
    }

    // Ancestry is not a record's to declare. Configuration cannot name a trace,
    // and the producer policy is the bootstrap stage's, so a record naming any
    // other origin or any other policy is asserting an authority it was not
    // given — which is exactly what a forged section would do to make its water
    // look attributable.
    for record in &state.forcing {
        let event = traces
            .event(record.origin_trace())
            .ok_or(RuntimeError::HydrologyTraceAnchorUnknown)?;
        if event.phase != Phase::Lifecycle
            || event.kind.raw() != HYDROLOGY_BOOTSTRAP_EVENT_KIND
            || record.producer_policy_schema()
                != causafera_geography::BOOTSTRAP_HYDROLOGY_FORCING_POLICY_V1
        {
            return Err(RuntimeError::HydrologyForcingAncestryForged);
        }
    }

    // Every anchor a carrier carries. An anchor is the whole of that carrier's
    // provenance; one that resolves to nothing leaves the carrier describing an
    // origin that never happened, and no later query could tell the difference.
    require_trace(traces, state.fields.conservation_last_change())?;
    for field in state.fields.fields().values() {
        for cell in field.cells() {
            require_trace(traces, cell.surface_last_change())?;
            require_trace(traces, cell.soil_last_change())?;
            require_trace(traces, cell.groundwater_last_change())?;
            require_trace(traces, cell.forcing_last_change())?;
        }
    }
    for edge in state.conveyance.edges().values() {
        require_trace(traces, edge.last_change())?;
    }
    for entry in state.resolution.values() {
        require_trace(traces, entry.last_change())?;
    }

    // A retained batch is keyed by the conservation event that closed it, and
    // only a committed tick produces one. A batch keyed to any other event —
    // the bootstrap origin, a terrain transition, a trace that does not exist —
    // is a ledger attributed to something that never balanced it.
    for trace in &state.retained_batches {
        let event = traces
            .event(*trace)
            .ok_or(RuntimeError::HydrologyTraceAnchorUnknown)?;
        if event.phase != Phase::Physics || event.kind.raw() != HYDROLOGY_CONSERVATION_EVENT_KIND {
            return Err(RuntimeError::HydrologyBatchTraceNotConservation);
        }
        // Looked up rather than indexed. `validate_hydrology_state` runs first
        // and would already have refused a batch with no receipt list, but a
        // panic guarded only by call order is a panic waiting for a caller.
        let receipts = state
            .receipts
            .get(trace)
            .ok_or(RuntimeError::HydrologyRetainedBatchIncomplete)?;
        for receipt in receipts {
            for parent in receipt.causal_parents() {
                require_trace(traces, *parent)?;
            }
            if let Some(origin) = receipt.forcing_origin() {
                require_trace(traces, origin)?;
            }
        }
    }
    Ok(())
}

fn require_trace(traces: &CausalTraceStore, trace: TraceId) -> Result<(), RuntimeError> {
    if traces.event(trace).is_some() {
        Ok(())
    } else {
        Err(RuntimeError::HydrologyTraceAnchorUnknown)
    }
}
