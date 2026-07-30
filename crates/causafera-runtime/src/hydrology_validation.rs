//! Import validation for hydrology state.
//!
//! Decoding already rebuilds every collection through its validating constructor,
//! so a malformed section cannot become state. What is left is the cross-cutting
//! agreement a single constructor cannot see: that the addressing is a bijection
//! over the carriers it addresses, that every retained ledger closes, that the
//! forcing schedule is consistent with the runtime time it was imported at, and
//! that receipts recompute to the aggregates they claim.
//!
//! See `plans/hydrology.md` verification gate V25.

use causafera_domains::{
    HydrologyReceiptTotals, validate_boundary_transfers, validate_paired_transfers,
};

use crate::{HydrologyRuntimeState, RuntimeError};

/// Check everything about an imported hydrology state that no single constructor
/// could have checked on its own.
pub(crate) fn validate_imported_hydrology(
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

    // The forcing schedule against the time it was imported at. A pending record
    // scheduled in the past would never apply, and an applied record whose
    // timestamp is not its scheduled tick is a record of something that did not
    // happen.
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
        for member in record.targets() {
            if !state.fields.is_resident(member.cell) {
                return Err(RuntimeError::HydrologyForcingTargetNotResident);
            }
        }
    }

    // Every retained ledger closes, and its aggregates recompute from the
    // per-transfer receipts rather than being believed.
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
    }
    if state.receipts.len() != state.retained_batches.len() {
        return Err(RuntimeError::HydrologyRetainedBatchIncomplete);
    }
    Ok(())
}
