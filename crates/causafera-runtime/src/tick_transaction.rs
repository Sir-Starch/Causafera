//! The whole-tick staging transaction for a hydrology-enabled session.
//!
//! A hydrology tick is not accepted because its own arithmetic closed. It is
//! accepted because the *complete* world it produced — every later phase
//! included — still validates and still fits inside a snapshot. Neither of those
//! is knowable from inside `Phase::Physics`: hydrology runs before the actor,
//! lifecycle and resolution work, and a later-phase event is what can push the
//! would-be envelope past the persistence cap.
//!
//! So the tick boundary owns the decision. Before any phase runs, the transaction
//! stages every mutable tick-owned value; after the last phase it validates the
//! domain invariants and computes the exact encoded size of the complete
//! would-be envelope. Publishing is then one infallible move, and so is undoing:
//! a refused tick restores the staged copy and leaves state, traces, counters,
//! queues, time and stream keys byte-identical to what the tick started from.
//!
//! Hydrology-disabled sessions never enter here. They keep the frozen legacy tick
//! path, which is what keeps their bytes and stream probes comparable with the
//! pre-hydrology evidence (V22).
//!
//! See `plans/hydrology.md` §10, §11 and verification gates V18, V24 and V33.

use causafera_geography::MAX_HYDROLOGY_SECTION_BYTES;
use causafera_persistence::{
    MAX_TOTAL_SIZE, SectionDirectoryEntry, SnapshotEnvelope, SnapshotHeader,
};
use causafera_types::SimulationTime;

use crate::snapshot_sections::{HYDROLOGY_SECTION_ID, assemble_envelope};
use crate::{RuntimeError, RuntimeState};

/// One tick's staged pre-image, and the decision about whether to keep it.
///
/// The staged copy is the whole of [`RuntimeState`]: authoritative domain state,
/// the causal trace store and its counters, the history-digest prefix, simulation
/// time, pending inputs and their consumption cursors, receipt-retention
/// bookkeeping, and hydrology's synthetic-node counter. Immutable system
/// registrations and configuration are shared rather than staged — no tick may
/// change them.
///
/// Scheduler-side values are staged as `completed_time`; the per-system
/// next-execution times are restored from it exactly as a snapshot resume
/// restores them, and RNG streams need nothing staged because
/// `StreamKey { world_seed, time, phase, system_id }` derives them from time
/// rather than from an accumulating counter.
pub(crate) struct RuntimeTickTransaction {
    staged: RuntimeState,
    completed_time: SimulationTime,
    /// The complete-envelope cap this tick is held to.
    ///
    /// Always [`MAX_TOTAL_SIZE`] in production. It is a field rather than a
    /// constant so a test can set a *reachable* cap and prove the near-cap
    /// rejection on a real session instead of asserting it about 256 MiB of
    /// fixture.
    budget: u64,
    /// The hydrology-section cap this tick is held to, for the same reason and
    /// with the same production value.
    section_budget: usize,
}

impl RuntimeTickTransaction {
    /// Stage everything the tick may change, before it changes anything.
    pub(crate) fn open(state: &RuntimeState, budget: u64, section_budget: usize) -> Self {
        Self {
            staged: state.clone(),
            completed_time: state.advanced_through,
            budget,
            section_budget,
        }
    }

    /// The tick this transaction can restore the session to.
    pub(crate) fn completed_time(&self) -> SimulationTime {
        self.completed_time
    }

    /// Publish the tick, or restore the staged pre-image and report why not.
    ///
    /// `Ok` means every staged value is now the authoritative one — which it
    /// already was, since the phases wrote in place and this call only decides
    /// whether to keep them. `Err` means `state` is byte-identical to what
    /// `open` saw, and the caller must roll the scheduler back to
    /// [`Self::completed_time`].
    pub(crate) fn close(
        self,
        state: &mut RuntimeState,
        scheduler_time: SimulationTime,
    ) -> Result<(), RuntimeError> {
        match validate_completed_tick(state, scheduler_time, self.budget, self.section_budget) {
            Ok(()) => Ok(()),
            Err(error) => {
                // One move, and it cannot fail. Anything that could fail here
                // would leave a half-restored world, which is the exact state
                // this module exists to make unreachable.
                *state = self.staged;
                Err(error)
            }
        }
    }
}

/// Everything that must hold about a tick before it may be published.
fn validate_completed_tick(
    state: &RuntimeState,
    scheduler_time: SimulationTime,
    budget: u64,
    section_budget: usize,
) -> Result<(), RuntimeError> {
    if let Some(failure) = state.failure.clone() {
        return Err(failure);
    }
    if state.advanced_through != scheduler_time {
        return Err(RuntimeError::PhaseDesynchronized);
    }
    // The same cross-cutting invariants an imported state has to satisfy. A tick
    // that produced a state import would refuse has produced a state this session
    // could not resume from, and that is not a state worth keeping.
    crate::hydrology_validation::validate_hydrology_state(
        &state.hydrology,
        state.config.hydrology.enabled,
        scheduler_time.raw(),
    )?;
    require_exportable_within(state, budget, section_budget)
}

/// Refuse a state that could be accepted and then never exported.
///
/// Called before configuration is accepted, before an imported state is accepted,
/// and before any hydrology-enabled tick is published. The size is the exact
/// encoded length of the complete canonical envelope — every section, the causal
/// trace bytes, the header and the section directory — not an estimate of the
/// hydrology part.
pub(crate) fn require_exportable(state: &RuntimeState) -> Result<(), RuntimeError> {
    if !state.hydrology.enabled {
        // A session without water keeps the frozen legacy acceptance path. Its
        // size behaviour is whatever it was before hydrology existed, which is
        // exactly what the pre-hydrology evidence pins (V22); adding a new refusal
        // to it would be a contract change this plan does not make.
        return Ok(());
    }
    require_exportable_within(state, MAX_TOTAL_SIZE, MAX_HYDROLOGY_SECTION_BYTES)
}

fn require_exportable_within(
    state: &RuntimeState,
    budget: u64,
    section_budget: usize,
) -> Result<(), RuntimeError> {
    let envelope = assemble_envelope(&state.export_snapshot())?;
    let hydrology = envelope
        .sections
        .get(&u64::from(HYDROLOGY_SECTION_ID))
        .map_or(0, |section| section.bytes.len());
    if hydrology > section_budget {
        return Err(RuntimeError::HydrologySectionWouldExceedBound {
            size: hydrology,
            max: section_budget,
        });
    }
    let size = envelope_encoded_size(&envelope);
    if size > budget {
        return Err(RuntimeError::SnapshotWouldExceedTotalSize { size, max: budget });
    }
    Ok(())
}

/// The exact byte length `SnapshotEnvelope::encode` would produce.
///
/// Computed rather than encoded: `encode` concatenates a second full copy of every
/// payload it already built, and the length is what the caps are about. The
/// arithmetic mirrors `encode` exactly — header, then payloads in schema-ID order,
/// then one directory entry per section — and a unit test pins the two against
/// each other so this cannot drift into an estimate.
fn envelope_encoded_size(envelope: &SnapshotEnvelope) -> u64 {
    let payloads: u64 = envelope
        .sections
        .values()
        .map(|section| section.bytes.len() as u64)
        .sum();
    let directory = envelope.sections.len() as u64 * SectionDirectoryEntry::SIZE as u64;
    SnapshotHeader::SIZE as u64 + payloads + directory
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::num::{NonZeroU32, NonZeroU64};

    use causafera_domains::HydrologyResolutionPolicy;
    use causafera_geography::{HydrologyBoundaryCondition, HydrologyGridMetric};
    use causafera_types::{SpatialChartId, WaterVolume};

    use super::*;
    use crate::{
        HYDROLOGY_BOOTSTRAP_PARAMETERS_SCHEMA_V1, HYDROLOGY_LIMITS_SCHEMA_V1,
        HydrologyBootstrapParameters, HydrologyConfig, Runtime, RuntimeConfig,
    };

    fn nz32(value: u32) -> NonZeroU32 {
        NonZeroU32::new(value).expect("positive")
    }

    fn nz64(value: u64) -> NonZeroU64 {
        NonZeroU64::new(value).expect("positive")
    }

    /// A session with water, actors and a population, so a tick commits events in
    /// Physics *and* in the later phases the cap decision has to account for.
    fn near_cap_config(seed: u64) -> RuntimeConfig {
        let mut config = RuntimeConfig::new(seed);
        config.actor_count = 2;
        config.sensor_count = 1;
        config.bootstrap_population = 32;
        config.hydrology = HydrologyConfig {
            enabled: true,
            grid_metrics: [(
                SpatialChartId::new(1),
                HydrologyGridMetric::new(nz64(1_000_000), nz64(1_000), nz64(1_000)),
            )]
            .into_iter()
            .collect(),
            bootstrap_parameters: Some(HydrologyBootstrapParameters {
                schema_version: HYDROLOGY_BOOTSTRAP_PARAMETERS_SCHEMA_V1,
                default_surface_capacity: WaterVolume::new(1_000_000_000),
                default_soil_capacity: WaterVolume::new(1_000_000_000),
                default_groundwater_capacity: WaterVolume::new(1_000_000_000),
                initial_surface: WaterVolume::new(1_000),
                initial_soil: WaterVolume::new(2_000),
                initial_groundwater: WaterVolume::new(3_000),
                infiltration_rate_mm_per_second: 4,
                percolation_fraction_num: 1,
                percolation_fraction_den: nz32(4),
                specific_yield_num: 1,
                specific_yield_den: nz32(5),
                aquifer_base_offset_mm: -2_500,
                baseflow_threshold: WaterVolume::new(500),
                baseflow_fraction_num: 1,
                baseflow_fraction_den: nz32(8),
                base_surface_transmissivity_mm3_per_second: 7,
                base_groundwater_transmissivity_mm3_per_second: 3,
                roughness_reference_mm: nz64(50),
                conveyance_capacity: WaterVolume::new(100_000),
                conveyance_initial_storage: WaterVolume::new(1_000),
                conveyance_inlet_capacity_per_tick: WaterVolume::new(10_000),
                conveyance_release_fraction_num: 1,
                conveyance_release_fraction_den: nz32(4),
                default_boundary: HydrologyBoundaryCondition::CLOSED,
                chart_overrides: BTreeMap::new(),
                cell_overrides: BTreeMap::new(),
            }),
            forcing_schedule: Vec::new(),
            resolution_policy: HydrologyResolutionPolicy::DISABLED,
            limits_schema: HYDROLOGY_LIMITS_SCHEMA_V1,
        };
        config
    }

    /// What one tick of `near_cap_config` costs in encoded envelope bytes.
    fn size_after(ticks: u64, seed: u64) -> u64 {
        let mut runtime = Runtime::new(near_cap_config(seed)).expect("construction");
        runtime.run_ticks(ticks).expect("the run must commit");
        let state = runtime.lock_state().expect("state");
        let envelope = assemble_envelope(&state.export_snapshot()).expect("assemble");
        envelope_encoded_size(&envelope)
    }

    #[test]
    fn a_tick_that_would_outgrow_the_cap_is_refused_whole() {
        // V18's near-cap half. The cap cannot be decided inside `Phase::Physics`:
        // the events that cross it come from the actor, lifecycle and resolution
        // work that runs *after* hydrology, and from the trace bytes they add. So
        // the tick boundary decides, and a tick that would cross it is undone
        // entirely — including the later-phase work that did the crossing.
        let seed = 5_507;
        let ceiling = size_after(2, seed);
        let mut runtime = Runtime::new(near_cap_config(seed)).expect("construction");
        runtime.run_ticks(1).expect("tick one must commit");

        // One byte short of what tick two needs. Nothing else about the session
        // changes, so what the refusal isolates is the size decision alone.
        runtime.snapshot_budget = ceiling - 1;
        let before = runtime.snapshot().expect("readable");
        let before_events = runtime
            .export_snapshot()
            .expect("export")
            .traces
            .events
            .len();
        let before_hydrology = runtime.hydrology_state();
        let before_time = runtime.current_time();

        let error = runtime
            .tick()
            .expect_err("a tick that cannot be exported must be refused");
        let RuntimeError::SnapshotWouldExceedTotalSize { size, max } = error else {
            panic!("unexpected refusal: {error}");
        };
        assert_eq!(max, ceiling - 1);
        assert!(
            size >= ceiling,
            "the refusal must report the composed size it measured"
        );

        // Everything the tick touched is back, in every phase.
        let after = runtime.snapshot().expect("still readable");
        assert_eq!(after, before, "state, digests and counters are unchanged");
        assert_eq!(
            runtime
                .export_snapshot()
                .expect("export")
                .traces
                .events
                .len(),
            before_events,
            "no phase's events survived the refusal"
        );
        assert_eq!(runtime.hydrology_state(), before_hydrology);
        assert_eq!(runtime.current_time(), before_time);
        // The stream keys are a function of time, so a clock that stayed put is a
        // stream that stayed put — and the retried tick is the same tick.
        assert_eq!(
            runtime.scheduler.current_time(),
            before_time,
            "the scheduler came back with the state"
        );

        // And the same tick with room for it commits.
        runtime.snapshot_budget = ceiling;
        runtime.tick().expect("the tick fits inside the real cap");
        assert_eq!(runtime.current_time(), before_time.tick());
    }

    #[test]
    fn a_state_over_the_hydrology_section_bound_is_refused() {
        // The section has its own cap below the envelope's, and it is a distinct
        // refusal: a session can outgrow the section bound while the complete
        // envelope still has room, and that state is just as unexportable.
        //
        // Driven through a lowered budget on a real session, because reaching
        // 192 MiB of hydrology section would mean allocating 192 MiB of it. The
        // envelope budget is left at the production cap throughout, so the only
        // thing that can refuse this tick is the section bound.
        let seed = 7_717;
        let mut runtime = Runtime::new(near_cap_config(seed)).expect("construction");
        runtime.run_ticks(1).expect("tick one must commit");
        let section = {
            let state = runtime.lock_state().expect("state");
            let envelope = assemble_envelope(&state.export_snapshot()).expect("assemble");
            assert!(
                envelope_encoded_size(&envelope) <= MAX_TOTAL_SIZE,
                "the complete envelope is nowhere near the persistence cap"
            );
            envelope
                .sections
                .get(&u64::from(HYDROLOGY_SECTION_ID))
                .expect("an enabled session encodes the section")
                .bytes
                .len()
        };
        assert!(
            section > 0 && section <= MAX_HYDROLOGY_SECTION_BYTES,
            "a real session's section is within its production bound"
        );

        let before = runtime.snapshot().expect("readable");
        let before_time = runtime.current_time();
        let before_hydrology = runtime.hydrology_state();

        // One byte short of what the section already occupies, so the very next
        // tick cannot fit however little it adds.
        runtime.hydrology_section_budget = section - 1;
        let error = runtime.tick().expect_err("the section bound must refuse");
        assert!(
            matches!(
                error,
                RuntimeError::HydrologySectionWouldExceedBound { max, .. } if max == section - 1
            ),
            "the refusal must name the section bound, not the envelope cap: {error}"
        );

        // And the refusal is whole, exactly as the envelope one is.
        assert_eq!(runtime.snapshot().expect("still readable"), before);
        assert_eq!(runtime.hydrology_state(), before_hydrology);
        assert_eq!(runtime.current_time(), before_time);
        assert_eq!(runtime.scheduler.current_time(), before_time);

        runtime.hydrology_section_budget = MAX_HYDROLOGY_SECTION_BYTES;
        runtime.tick().expect("the tick fits inside the real bound");
        assert_eq!(runtime.current_time(), before_time.tick());
    }

    #[test]
    fn the_computed_size_is_the_encoded_size() {
        // The caps are enforced against a number nobody encoded. If that number
        // were an estimate, every rejection and every acceptance near the cap
        // would be wrong by whatever the estimate is off by.
        let mut runtime = Runtime::new(RuntimeConfig::new(9_311)).expect("construction");
        runtime.run_ticks(3).expect("the run must commit");
        let state = runtime.lock_state().expect("state");
        let envelope = assemble_envelope(&state.export_snapshot()).expect("assemble");
        assert_eq!(
            envelope_encoded_size(&envelope),
            envelope.encode().expect("encode").len() as u64
        );
    }

    #[test]
    fn a_production_session_holds_the_persistence_cap() {
        // Both budgets are fields so a test can reach them. That is only safe
        // while production keeps setting them to the real caps.
        let runtime = Runtime::new(RuntimeConfig::new(4_231)).expect("construction");
        assert_eq!(runtime.snapshot_budget, MAX_TOTAL_SIZE);
        assert_eq!(
            runtime.hydrology_section_budget,
            MAX_HYDROLOGY_SECTION_BYTES
        );
    }
}
