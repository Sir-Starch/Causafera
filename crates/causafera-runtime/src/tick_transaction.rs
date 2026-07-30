//! The whole-tick staging guarantee for a hydrology-enabled session.
//!
//! Hydrology runs first in `Phase::Physics` and commits its causal batch before
//! installing any state, and `commit_dag_batch` either appends the complete batch
//! or leaves the trace store byte-identical. What this module adds is the
//! *tick-level* half of the same property: a failure anywhere in the tick closes
//! the session rather than leaving it readable at a state no complete tick
//! produced.
//!
//! See `plans/hydrology.md` verification gate V18.

use crate::{RuntimeError, RuntimeState};

/// What a tick is allowed to have changed when it fails.
///
/// Nothing observable. A caller that asks a failed session anything gets the
/// failure back, so a half-advanced tick is not a state anyone can read — which
/// is the property, not a side effect of one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TickStagingGuard {
    /// The tick the session had completed before this one began.
    advanced_through: u64,
    /// How many causal events the store held before this tick.
    committed_events: usize,
    /// The hydrology batch sequence before this tick.
    hydrology_batch_sequence: u64,
}

impl TickStagingGuard {
    /// Record what a tick must not have changed if it fails.
    pub(crate) fn open(state: &RuntimeState) -> Self {
        Self {
            advanced_through: state.advanced_through.raw(),
            committed_events: state.traces.len(),
            hydrology_batch_sequence: state.hydrology.fields.batch_sequence(),
        }
    }

    /// Check the guarantee after the tick ran.
    ///
    /// On failure the trace store must be exactly as it was: every hydrology batch
    /// is atomic, and every later system bails on a recorded failure rather than
    /// committing on top of one. A store that grew while the tick failed would mean
    /// some system committed into a tick that then did not happen.
    pub(crate) fn close(self, state: &RuntimeState) -> Result<(), RuntimeError> {
        let Some(failure) = state.failure.clone() else {
            return Ok(());
        };
        if state.traces.len() != self.committed_events {
            return Err(RuntimeError::TickStagingViolated {
                what: "the causal trace store grew during a failed tick",
            });
        }
        if state.hydrology.fields.batch_sequence() != self.hydrology_batch_sequence {
            return Err(RuntimeError::TickStagingViolated {
                what: "hydrology installed a batch during a failed tick",
            });
        }
        if state.advanced_through.raw() != self.advanced_through {
            return Err(RuntimeError::TickStagingViolated {
                what: "a failed tick advanced the completed-tick counter",
            });
        }
        Err(failure)
    }
}

/// Refuse a tick before anything runs when hydrology could not complete it.
///
/// Hydrology is not the first system in `Phase::Physics` — appending it was what
/// preserved every existing stream key — so by the time it refuses, earlier systems
/// have already committed. Restoring them would need a whole-state snapshot the
/// runtime does not take; refusing *before* the phase starts gives the same
/// observable result for every failure that is knowable from the pre-tick state,
/// which is all of them that depend on residency, metrics, resolution coverage, or
/// which cells a scheduled record targets.
///
/// What this cannot pre-empt is a failure that depends on the tick's own
/// arithmetic. Those still reach `TickStagingGuard`, which reports the violation
/// rather than the failure — see the plan's Progress section.
pub(crate) fn admit_hydrology_tick(state: &RuntimeState, tick: u64) -> Result<(), RuntimeError> {
    if !state.hydrology.enabled {
        return Ok(());
    }
    let hydrology = &state.hydrology;

    // The same preconditions `propose` opens with, read from the state the tick is
    // about to start from.
    for chunk in hydrology.fields.fields().keys() {
        hydrology.metrics.get(chunk.chart)?;
        if !hydrology.active.resident_chunks().contains(chunk) {
            return Err(RuntimeError::HydrologyResidencyMismatch);
        }
    }
    if hydrology.active.resident_chunks().len() != hydrology.fields.fields().len() {
        return Err(RuntimeError::HydrologyResidencyMismatch);
    }
    causafera_domains::HydrologyResolutionPlan::build(
        &hydrology.fields,
        &hydrology.boundaries,
        &hydrology.metrics,
        &hydrology.resolution,
        hydrology.resolution_policy,
    )?;

    // Every record due this tick must target resident cells. Configuration cannot
    // check this — it does not know which chunks a session will hold — so the tick
    // is where it is knowable, and the start of the tick is where it is actionable.
    for record in &hydrology.forcing {
        if record.scheduled_tick() != tick || record.applied_at().is_some() {
            continue;
        }
        for member in record.targets() {
            if !hydrology.fields.is_resident(member.cell) {
                return Err(RuntimeError::HydrologyForcingTargetNotResident);
            }
        }
    }
    Ok(())
}
