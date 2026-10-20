//! The whole-tick staging transaction for every session.
//!
//! Before any phase runs, the transaction captures a bounded pre-image of the
//! mutable fields the tick may change. After the last phase it validates domain
//! invariants and the envelope cap. A refused tick restores the bounded
//! pre-image; a successful one publishes exactly one logical generation and hands
//! the bounded frame records to the durability coordinator.
//!
//! Both hydrology-enabled and disabled ticks run through this transaction.
//!
//! See `plans/causal-history-storage.md` Todo 5.

use causafera_geography::MAX_HYDROLOGY_SECTION_BYTES;
use causafera_persistence::MAX_TOTAL_SIZE;
use causafera_types::SimulationTime;

use crate::history_transaction::{HistoryPublishReceipt, PreparedHistoryFrame};
use crate::snapshot_sections::{encode_hydrology_section, envelope_encoded_size_from_data};
use crate::{RuntimeError, RuntimeState};

/// One tick's staged pre-image and the decision about whether to keep it.
pub(crate) struct RuntimeTickTransaction {
    staged: crate::tick_preimage::TickPreImage,
    completed_time: SimulationTime,
    budget: u64,
    section_budget: usize,
}

impl RuntimeTickTransaction {
    /// Stage only the bounded mutable fields, before the tick changes anything.
    pub(crate) fn open(
        state: &mut RuntimeState,
        budget: u64,
        section_budget: usize,
    ) -> Result<Self, RuntimeError> {
        if state.tick_history.is_some() {
            return Err(RuntimeError::HistoryTransactionAlreadyOpen);
        }
        let history = state.history.begin()?;
        state.begin_tick_undo()?;
        state.traces.begin_transaction();
        state.tick_history = Some(history);
        Ok(Self {
            staged: crate::tick_preimage::TickPreImage::new(),
            completed_time: state.advanced_through,
            budget,
            section_budget,
        })
    }

    pub(crate) fn completed_time(&self) -> SimulationTime {
        self.completed_time
    }

    /// Publish the tick, or restore the bounded pre-image.
    pub(crate) fn close(
        self,
        state: &mut RuntimeState,
        scheduler_time: SimulationTime,
    ) -> Result<(), RuntimeError> {
        if let Err(error) =
            validate_completed_tick(state, scheduler_time, self.budget, self.section_budget)
        {
            let _ = state.tick_history.take();
            self.staged.restore(state);
            return Err(error);
        }
        if let Err(error) = state.stage_history_evidence(&[]) {
            let _ = state.tick_history.take();
            self.staged.restore(state);
            return Err(error);
        }
        let history = match state.tick_history.take() {
            Some(history) => history,
            None => {
                self.staged.restore(state);
                return Err(RuntimeError::HistoryTransactionNotOpen);
            }
        };
        let frame = match history.prepare(&state.traces) {
            Ok(frame) => frame,
            Err(error) => {
                self.staged.restore(state);
                return Err(error);
            }
        };
        if state.traces.hot_bytes() > state.traces.hot_budget()
            && let Err(error) = state
                .traces
                .trim_staged_frame_to_hot_budget(frame.records())
        {
            self.staged.restore(state);
            return Err(error.into());
        }
        let receipt = match state.history.publish(&frame) {
            Ok(receipt) => receipt,
            Err(error) => {
                self.staged.restore(state);
                return Err(error);
            }
        };
        if let Err(error) = state.history.finish_state_publication() {
            rollback_publication(state, &frame, receipt, self.staged)?;
            return Err(error);
        }
        if let Err(error) = state.traces.end_transaction() {
            rollback_publication(state, &frame, receipt, self.staged)?;
            return Err(error.into());
        }
        state.commit_tick_undo();
        state.checkpoint_if_due();
        Ok(())
    }
}

impl RuntimeState {
    pub(crate) fn stage_history_evidence(
        &mut self,
        records: &[causafera_core::DomainEvidenceRecord],
    ) -> Result<(), RuntimeError> {
        let transaction = self
            .tick_history
            .as_mut()
            .ok_or(RuntimeError::HistoryTransactionNotOpen)?;
        transaction.stage_tick_evidence(&self.traces, records)
    }

    #[cfg(test)]
    pub(crate) fn staged_history_evidence(
        &self,
        schema_id: causafera_core::EvidenceSchemaId,
        canonical_key: &[u8],
        query: causafera_core::HistoryQuery,
    ) -> Result<causafera_core::HistoryPage<causafera_core::DomainEvidenceRecord>, RuntimeError>
    {
        self.tick_history
            .as_ref()
            .ok_or(RuntimeError::HistoryTransactionNotOpen)?
            .staged_evidence(schema_id, canonical_key, query)
    }
}

fn rollback_publication(
    state: &mut RuntimeState,
    frame: &PreparedHistoryFrame,
    receipt: HistoryPublishReceipt,
    staged: crate::tick_preimage::TickPreImage,
) -> Result<(), RuntimeError> {
    state.history.rollback(frame, receipt)?;
    staged.restore(state);
    Ok(())
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
    crate::hydrology_validation::validate_hydrology_state(
        &state.hydrology,
        state.config.hydrology.enabled,
        scheduler_time.raw(),
    )?;
    if !state.hydrology.enabled {
        return Ok(());
    }
    let hydrology = encode_hydrology_section(&state.hydrology).len();
    if hydrology > section_budget {
        return Err(RuntimeError::HydrologySectionWouldExceedBound {
            size: hydrology,
            max: section_budget,
        });
    }
    #[cfg(test)]
    if budget != MAX_TOTAL_SIZE {
        return tests::require_lowered_snapshot_budget(state, budget);
    }
    #[cfg(not(test))]
    let _ = budget;
    Ok(())
}

pub(crate) fn require_exportable(state: &RuntimeState) -> Result<(), RuntimeError> {
    if !state.hydrology.enabled {
        return Ok(());
    }
    require_exportable_within(state, MAX_TOTAL_SIZE, MAX_HYDROLOGY_SECTION_BYTES)
}

fn require_exportable_within(
    state: &RuntimeState,
    budget: u64,
    section_budget: usize,
) -> Result<(), RuntimeError> {
    let data = state.export_snapshot();
    let (size, hydrology) = envelope_encoded_size_from_data(&data);
    if hydrology > section_budget {
        return Err(RuntimeError::HydrologySectionWouldExceedBound {
            size: hydrology,
            max: section_budget,
        });
    }
    if size > budget {
        return Err(RuntimeError::SnapshotWouldExceedTotalSize { size, max: budget });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::io::Write;
    use std::num::{NonZeroU32, NonZeroU64};
    use std::sync::Arc;
    use std::time::Duration;

    use causafera_core::{
        CausalEffect, CausalEventProposal, CausalHistoryReader, CausalTarget, Criticality,
        DomainEvidenceRecord, EventProposalKey, EvidenceSchemaId, EvidenceSchemaVersion,
        HistoryAnchor, HistoryAnchorFields, HistoryDurability, HistoryError, HistoryFrontier,
        HistoryPage, HistoryQuery, HistoryRecord, OwnedCausalRecord, Phase, StateFingerprint,
    };
    use causafera_domains::HydrologyResolutionPolicy;
    use causafera_geography::{HydrologyBoundaryCondition, HydrologyGridMetric};
    use causafera_types::{
        EventId, EventKindId, SpatialChartId, StateObjectKindId, StatePropertyId, TraceId,
        WaterVolume,
    };

    use super::*;

    macro_rules! emit_marker {
        ($($argument:tt)*) => {
            emit_marker_line(format_args!($($argument)*));
        };
    }

    fn emit_marker_line(arguments: std::fmt::Arguments<'_>) {
        let line = format!("{arguments}\n");
        if let Ok(mut terminal) = std::fs::OpenOptions::new().write(true).open("/dev/tty") {
            let _ = terminal.write_all(line.as_bytes());
            let _ = terminal.flush();
        } else {
            eprint!("{line}");
        }
    }
    use crate::snapshot_sections::assemble_envelope;
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

    fn handoff_causal_proposal(anchor: TraceId) -> CausalEventProposal {
        CausalEventProposal::new(
            EventProposalKey::new(0, 0, 0),
            EventKindId::new(0x7f06),
            vec![anchor],
            vec![
                CausalEffect::new(
                    CausalTarget::new(StateObjectKindId::new(0x7f06), 0, StatePropertyId::new(1)),
                    StateFingerprint::new([0x11; 32]),
                    StateFingerprint::new([0x22; 32]),
                )
                .expect("effect is valid"),
            ],
        )
        .expect("proposal is valid")
    }

    fn near_cap_config(seed: u64) -> RuntimeConfig {
        let mut config = RuntimeConfig::new(seed);
        config.hot_budget = Some(4 * 1024 * 1024);
        config.active_chunk_radius = 0;
        config.actor_count = 0;
        config.sensor_count = 0;
        config.bootstrap_population = 0;
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

    fn minimal_config(seed: u64, hydrology: bool) -> RuntimeConfig {
        if hydrology {
            near_cap_config(seed)
        } else {
            let mut config = RuntimeConfig::new(seed);
            config.active_chunk_radius = 0;
            config.hydrology.enabled = false;
            config
        }
    }

    fn synthetic_record(raw: u64) -> OwnedCausalRecord {
        let mut after = [0_u8; 32];
        after[..8].copy_from_slice(&raw.to_le_bytes());
        OwnedCausalRecord::new(
            raw,
            EventId::new(raw),
            TraceId::new(raw),
            SimulationTime::new(raw),
            Phase::Physics,
            EventKindId::new(9_001),
            Vec::new(),
            vec![
                CausalEffect::new(
                    CausalTarget::new(
                        StateObjectKindId::new(9_002),
                        raw,
                        StatePropertyId::new(9_003),
                    ),
                    StateFingerprint::new([0xFF; 32]),
                    StateFingerprint::new(after),
                )
                .expect("synthetic effect"),
            ],
        )
        .expect("synthetic record")
    }

    #[derive(Debug)]
    struct SyntheticHistoryReader {
        events: u64,
    }

    impl CausalHistoryReader for SyntheticHistoryReader {
        fn anchor(&self) -> Result<HistoryAnchor, HistoryError> {
            let last = self
                .events
                .checked_sub(1)
                .ok_or(HistoryError::ArithmeticOverflow)?;
            HistoryAnchor::new(HistoryAnchorFields {
                namespace: [0x51; 16],
                logical_generation: self.events,
                durable_generation: 0,
                checkpoint_digest: [0; 32],
                frontier: HistoryFrontier::new(
                    self.events,
                    EventId::new(last),
                    TraceId::new(last),
                    SimulationTime::new(last),
                ),
                logical_digest: [0x52; 32],
                storage_root: [0; 32],
                physical_digest_schema: crate::CURRENT_DIGEST_SCHEMA_VERSION.raw(),
                history_digest_schema: crate::CURRENT_DIGEST_SCHEMA_VERSION.raw(),
                evidence_manifest_digest: [0; 32],
                durability: HistoryDurability::GroupCommit {
                    max_ticks: 32,
                    max_wall_time_ms: 250,
                },
            })
        }

        fn event(&self, trace_id: TraceId) -> Result<Option<OwnedCausalRecord>, HistoryError> {
            Ok((trace_id.raw() < self.events).then(|| synthetic_record(trace_id.raw())))
        }

        fn event_by_id(
            &self,
            event_id: EventId,
        ) -> Result<Option<OwnedCausalRecord>, HistoryError> {
            self.event(TraceId::new(event_id.raw()))
        }

        fn records(&self, query: HistoryQuery) -> Result<HistoryPage<HistoryRecord>, HistoryError> {
            let available = self.events.saturating_sub(query.offset());
            let count = usize::try_from(available)
                .map_or(query.max_records(), |count| count.min(query.max_records()));
            let mut records = Vec::new();
            let mut bytes = 0_usize;
            for offset in 0..count {
                let raw = query
                    .offset()
                    .checked_add(
                        u64::try_from(offset).map_err(|_| HistoryError::ArithmeticOverflow)?,
                    )
                    .ok_or(HistoryError::ArithmeticOverflow)?;
                let record = synthetic_record(raw);
                let next = bytes
                    .checked_add(record.canonical_field_size()?)
                    .ok_or(HistoryError::ArithmeticOverflow)?;
                if next > query.max_bytes() {
                    break;
                }
                bytes = next;
                records.push(HistoryRecord::Causal(record));
            }
            let consumed = query
                .offset()
                .checked_add(
                    u64::try_from(records.len()).map_err(|_| HistoryError::ArithmeticOverflow)?,
                )
                .ok_or(HistoryError::ArithmeticOverflow)?;
            let next = (consumed < self.events).then_some(consumed);
            HistoryPage::new(records, next, query)
        }
    }

    fn size_after(ticks: u64, seed: u64) -> u64 {
        let mut runtime = Runtime::new(near_cap_config(seed)).expect("construction");
        runtime.run_ticks(ticks).expect("the run must commit");
        let state = runtime.lock_state().expect("state");
        envelope_encoded_size_from_data(&state.export_snapshot()).0
    }

    #[test]
    fn a_tick_that_would_outgrow_the_cap_is_refused_whole() {
        let seed = 5_507;
        let ceiling = size_after(2, seed);
        let mut runtime = Runtime::new(near_cap_config(seed)).expect("construction");
        runtime.run_ticks(1).expect("tick one must commit");

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
        assert_eq!(
            runtime.scheduler.current_time(),
            before_time,
            "the scheduler came back with the state"
        );

        runtime.snapshot_budget = ceiling;
        runtime.tick().expect("the tick fits inside the real cap");
        assert_eq!(runtime.current_time(), before_time.tick());
    }

    #[test]
    fn a_state_over_the_hydrology_section_bound_is_refused() {
        let seed = 7_717;
        let mut runtime = Runtime::new(near_cap_config(seed)).expect("construction");
        runtime.run_ticks(1).expect("tick one must commit");
        let section = {
            let state = runtime.lock_state().expect("state");
            let (total, hydrology) = envelope_encoded_size_from_data(&state.export_snapshot());
            assert!(
                total <= MAX_TOTAL_SIZE,
                "the complete envelope is nowhere near the persistence cap"
            );
            hydrology
        };
        assert!(
            section > 0 && section <= MAX_HYDROLOGY_SECTION_BYTES,
            "a real session's section is within its production bound"
        );

        let before = runtime.snapshot().expect("readable");
        let before_time = runtime.current_time();
        let before_hydrology = runtime.hydrology_state();

        runtime.hydrology_section_budget = section - 1;
        let error = runtime.tick().expect_err("the section bound must refuse");
        assert!(
            matches!(
                error,
                RuntimeError::HydrologySectionWouldExceedBound { max, .. } if max == section - 1
            ),
            "the refusal must name the section bound, not the envelope cap: {error}"
        );

        assert_eq!(runtime.snapshot().expect("still readable"), before);
        assert_eq!(runtime.hydrology_state(), before_hydrology);
        assert_eq!(runtime.current_time(), before_time);
        assert_eq!(runtime.scheduler.current_time(), before_time);
        let after_history = runtime.history_status().expect("history status");
        emit_marker!(
            "MARKER:enabled_refusal_unchanged time={} generation={} durable={} head={} hot_bytes={}",
            before_time.raw(),
            after_history.logical_generation,
            after_history.durable_generation,
            after_history.visible_head.raw(),
            after_history.hot_bytes
        );

        runtime.hydrology_section_budget = MAX_HYDROLOGY_SECTION_BYTES;
        runtime.tick().expect("the tick fits inside the real bound");
        assert_eq!(runtime.current_time(), before_time.tick());
    }

    #[test]
    fn the_computed_size_is_the_encoded_size() {
        let mut runtime = Runtime::new(RuntimeConfig::new(9_311)).expect("construction");
        runtime.tick().expect("the run must commit");
        let state = runtime.lock_state().expect("state");
        let data = state.export_snapshot();
        let envelope = assemble_envelope(&data).expect("assemble");
        assert_eq!(
            envelope_encoded_size_from_data(&data).0,
            envelope.encode().expect("encode").len() as u64
        );
    }

    #[test]
    fn a_production_session_holds_the_persistence_cap() {
        let runtime = Runtime::new(RuntimeConfig::new(4_231)).expect("construction");
        assert_eq!(runtime.snapshot_budget, MAX_TOTAL_SIZE);
        assert_eq!(
            runtime.hydrology_section_budget,
            MAX_HYDROLOGY_SECTION_BYTES
        );
    }

    /// Both enabled and disabled ticks share the same transaction contract.
    #[test]
    fn disabled_hydrology_tick_uses_bounded_transaction() {
        // PIN: a session without hydrology must complete a tick and produce
        // nonzero events, counters, and trace data through the bounded
        // pre-image path.
        let mut config = RuntimeConfig::new(113);
        config.actor_count = 2;
        config.sensor_count = 2;
        config.bootstrap_population = 6;
        config.hydrology.enabled = false;

        let mut runtime = Runtime::new(config).expect("construction");
        emit_marker!(
            "MARKER:disabled_pre_tick_traces={}",
            runtime.lock_state().expect("state").traces.len()
        );

        let before = runtime.history_status().expect("history status");
        let snapshot = runtime.tick().expect("disabled tick must commit");
        let after = runtime.history_status().expect("history status");
        emit_marker!(
            "MARKER:disabled_tick_committed generation={} head={} hot_bytes={}",
            after.logical_generation,
            after.visible_head.raw(),
            after.hot_bytes
        );
        assert_eq!(after.logical_generation, before.logical_generation + 1);
        assert_eq!(after.durable_generation, before.durable_generation);
        assert!(after.hot_bytes <= after.hot_budget);
        assert_eq!(snapshot.time, SimulationTime::new(1));

        let state = runtime.lock_state().expect("state");
        emit_marker!(
            "MARKER:disabled_post_tick_traces={}_physical_events={}_scan_count={}",
            state.traces.len(),
            state.physical_events,
            state.physical_scan_count
        );
        assert!(!state.traces.is_empty());
        assert_eq!(state.history.logical_generation(), after.logical_generation);
    }

    /// A disabled hydrology session refuses a tick that would fail and restores
    /// all bounded pre-image values.
    #[test]
    fn disabled_hydrology_tick_refusal_restores_bounded_preimage() {
        let mut config = RuntimeConfig::new(271);
        config.actor_count = 1;
        config.sensor_count = 1;
        config.bootstrap_population = 2;
        config.hydrology.enabled = false;
        config.hot_budget = Some(4 * 1024 * 1024);

        let mut runtime = Runtime::new(config).expect("construction");
        let before = runtime.snapshot().expect("readable");
        let before_time = runtime.current_time();
        let before_history = runtime.history_status().expect("history status");

        {
            let mut state = runtime.lock_state().expect("state");
            state.history.inject_failure(Some(
                crate::history_transaction::HistoryCommitPhase::AfterFrameValidation,
            ));
        }

        let error = runtime
            .tick()
            .expect_err("tick with injected failure must be refused");
        assert!(matches!(error, RuntimeError::HistoryCommitRejected { .. }));

        let after = runtime.snapshot().expect("still readable");
        assert_eq!(after, before, "snapshot unchanged after refusal");
        assert_eq!(runtime.current_time(), before_time);
        assert_eq!(
            runtime.history_status().expect("history status"),
            before_history
        );
        emit_marker!(
            "MARKER:disabled_refusal_restores_preimage generation={} head={} hot_bytes={}",
            before_history.logical_generation,
            before_history.visible_head.raw(),
            before_history.hot_bytes
        );
    }

    #[test]
    fn every_commit_phase_rejection_is_atomic_twice_in_both_modes() {
        let phases = [
            crate::HistoryCommitPhase::BeforeFramePublication,
            crate::HistoryCommitPhase::AfterFrameValidation,
            crate::HistoryCommitPhase::AfterLogicalPublication,
            crate::HistoryCommitPhase::AfterStatePublication,
        ];
        for (mode, enabled) in [("disabled", false), ("enabled", true)] {
            for phase in phases {
                let config = minimal_config(12_000 + phase as u64, enabled);
                let mut runtime = Runtime::new(config.clone()).expect("construction");
                let mut control = Runtime::new(config).expect("control construction");
                let before_snapshot = runtime.snapshot().expect("snapshot");
                let before_status = runtime.history_status().expect("history status");
                let before_time = runtime.current_time();
                {
                    let mut state = runtime.lock_state().expect("state");
                    state.history.inject_failure(Some(phase));
                }
                let mut prior_error = None;
                for attempt in 1..=2 {
                    let error = runtime.tick().expect_err("phase rejection");
                    assert_eq!(
                        error,
                        RuntimeError::HistoryCommitRejected { phase },
                        "typed phase rejection"
                    );
                    assert_eq!(prior_error.as_ref().unwrap_or(&error), &error);
                    prior_error = Some(error);
                    assert_eq!(runtime.current_time(), before_time);
                    assert_eq!(runtime.scheduler.current_time(), before_time);
                    assert_eq!(runtime.snapshot().expect("snapshot"), before_snapshot);
                    assert_eq!(
                        runtime.history_status().expect("history status"),
                        before_status
                    );
                    let after = runtime.snapshot().expect("snapshot after refusal");
                    let status = runtime.history_status().expect("history status");
                    emit_marker!(
                        "MARKER:baseline_rollback mode={mode} phase={phase:?} attempt={attempt} runtime_time={} scheduler_time={} physical_digest={:?} history_digest={:?} physical_events={} mana_changes={} resolution_changes={} actor_commits={} actor_rejections={} population_births={} population_deaths={} next_trace={} logical_generation={} durable_generation={} history_head={} hot_bytes={}",
                        runtime.current_time().raw(),
                        runtime.scheduler.current_time().raw(),
                        after.physical_state_digest.bytes(),
                        after.history_digest.bytes(),
                        after.physical_events,
                        after.mana_cell_changes,
                        after.resolution_changes,
                        after.actor_actions_committed,
                        after.actor_actions_rejected,
                        after.population_births,
                        after.population_deaths,
                        after.latest_trace.raw(),
                        status.logical_generation,
                        status.durable_generation,
                        status.visible_head.raw(),
                        status.hot_bytes
                    );
                }
                runtime
                    .lock_state()
                    .expect("state")
                    .history
                    .inject_failure(None);
                let retried = runtime.tick().expect("retry succeeds");
                let direct = control.tick().expect("control succeeds");
                assert_eq!(retried, direct, "restored system times preserve replay");
                assert_eq!(
                    runtime.history_status().expect("history status"),
                    control.history_status().expect("history status")
                );
                emit_marker!(
                    "MARKER:baseline_retry mode={mode} phase={phase:?} retry_matches_control=true system_times_replayed=true generation={} head={}",
                    runtime
                        .history_status()
                        .expect("history status")
                        .logical_generation,
                    runtime
                        .history_status()
                        .expect("history status")
                        .visible_head
                        .raw()
                );
            }
        }
    }

    #[test]
    fn same_tick_evidence_handoff_commits_rolls_back_and_retries_in_both_modes() {
        for (mode, enabled) in [("disabled", false), ("enabled", true)] {
            let config = minimal_config(15_000 + u64::from(enabled), enabled);
            let mut runtime = Runtime::new(config.clone()).expect("construction");
            let mut control = Runtime::new(config).expect("control construction");
            let payload = format!("same-tick-{mode}").into_bytes();
            let schema = EvidenceSchemaId::new(0x7f06);
            let key = format!("handoff/{mode}").into_bytes();
            let query = HistoryQuery::new(0, 4, 4096).expect("query is bounded");
            let before = runtime.history_status().expect("history status");

            let (transaction, evidence) = {
                let mut state = runtime.lock_state().expect("state");
                let transaction = RuntimeTickTransaction::open(
                    &mut state,
                    MAX_TOTAL_SIZE,
                    MAX_HYDROLOGY_SECTION_BYTES,
                )
                .expect("transaction opens");
                let cause = state.latest_physical_trace;
                let evidence_anchor = state
                    .traces
                    .commit_batch(
                        SimulationTime::new(0),
                        Phase::Physics,
                        vec![handoff_causal_proposal(cause)],
                    )
                    .expect("same-tick cause commits")[0];
                let evidence = DomainEvidenceRecord::new(
                    evidence_anchor.raw(),
                    schema,
                    EvidenceSchemaVersion::new(1, 0),
                    Criticality::ExplanationOnly,
                    SimulationTime::new(0),
                    vec![evidence_anchor],
                    key.clone(),
                    payload.clone(),
                    StateFingerprint::new(*blake3::hash(&payload).as_bytes()),
                )
                .expect("evidence is canonical");
                state
                    .stage_history_evidence(std::slice::from_ref(&evidence))
                    .expect("same-tick staging succeeds");
                assert_eq!(
                    state
                        .staged_history_evidence(
                            evidence.schema_id(),
                            evidence.canonical_key(),
                            query,
                        )
                        .expect("staged lookup succeeds")
                        .records(),
                    std::slice::from_ref(&evidence),
                    "staged evidence must be readable before publication"
                );
                state.history.inject_failure(Some(
                    crate::history_transaction::HistoryCommitPhase::AfterStatePublication,
                ));
                (transaction, evidence)
            };
            let error = {
                let mut state = runtime.lock_state().expect("state");
                transaction
                    .close(&mut state, SimulationTime::new(0))
                    .expect_err("injected publication failure rejects the transaction")
            };
            assert_eq!(
                error,
                RuntimeError::HistoryCommitRejected {
                    phase: crate::history_transaction::HistoryCommitPhase::AfterStatePublication,
                }
            );
            assert_eq!(runtime.history_status().expect("history status"), before);
            assert!(
                runtime
                    .history_reader()
                    .expect("reader")
                    .evidence(evidence.schema_id(), evidence.canonical_key(), query)
                    .expect("evidence query")
                    .records()
                    .is_empty(),
                "failed enclosing publication leaked evidence"
            );

            for candidate in [&mut runtime, &mut control] {
                let transaction = {
                    let mut state = candidate.lock_state().expect("state");
                    state.history.inject_failure(None);
                    let transaction = RuntimeTickTransaction::open(
                        &mut state,
                        MAX_TOTAL_SIZE,
                        MAX_HYDROLOGY_SECTION_BYTES,
                    )
                    .expect("retry transaction opens");
                    let cause = state.latest_physical_trace;
                    let evidence_anchor = state
                        .traces
                        .commit_batch(
                            SimulationTime::new(0),
                            Phase::Physics,
                            vec![handoff_causal_proposal(cause)],
                        )
                        .expect("same-tick cause commits")[0];
                    assert_eq!(evidence.causal_anchors(), &[evidence_anchor]);
                    state
                        .stage_history_evidence(std::slice::from_ref(&evidence))
                        .expect("retry staging succeeds");
                    transaction
                };
                let mut state = candidate.lock_state().expect("state");
                transaction
                    .close(&mut state, SimulationTime::new(0))
                    .expect("enclosing transaction commits");
            }

            assert_eq!(
                runtime.history_status().expect("retry history status"),
                control.history_status().expect("control history status"),
                "retry must match the direct control"
            );
            let published = runtime
                .history_reader()
                .expect("reader")
                .evidence(evidence.schema_id(), evidence.canonical_key(), query)
                .expect("evidence query");
            assert_eq!(published.records(), std::slice::from_ref(&evidence));
            emit_marker!(
                "MARKER:history_txn_handoff mode={mode} staged_read=true failed_visible=false retry_matches_control=true published_records={}",
                published.records().len()
            );
        }
    }

    #[test]
    fn evidence_handoff_rejects_malformed_duplicate_and_unknown_records_atomically() {
        let runtime = Runtime::new(minimal_config(15_101, false)).expect("construction");
        let anchor = runtime.lock_state().expect("state").latest_physical_trace;
        let payload = b"canonical-evidence".to_vec();
        let schema = EvidenceSchemaId::new(0x7f07);
        let version = EvidenceSchemaVersion::new(1, 0);
        let record = |key: &[u8], anchors: Vec<TraceId>, digest: StateFingerprint| {
            DomainEvidenceRecord::new(
                0,
                schema,
                version,
                Criticality::ExplanationOnly,
                SimulationTime::new(0),
                anchors,
                key.to_vec(),
                payload.clone(),
                digest,
            )
            .expect("record shape is canonical")
        };
        let digest = StateFingerprint::new(*blake3::hash(&payload).as_bytes());
        let malformed = record(
            b"malformed",
            vec![anchor],
            StateFingerprint::new([0x55; 32]),
        );
        let duplicate = record(b"duplicate", vec![anchor], digest);
        let unknown = record(b"unknown", vec![TraceId::new(u64::MAX)], digest);
        let valid = record(b"valid", vec![anchor], digest);
        let query = HistoryQuery::new(0, 4, 4096).expect("query is bounded");

        let transaction = {
            let mut state = runtime.lock_state().expect("state");
            let transaction = RuntimeTickTransaction::open(
                &mut state,
                MAX_TOTAL_SIZE,
                MAX_HYDROLOGY_SECTION_BYTES,
            )
            .expect("transaction opens");
            assert!(matches!(
                RuntimeTickTransaction::open(
                    &mut state,
                    MAX_TOTAL_SIZE,
                    MAX_HYDROLOGY_SECTION_BYTES,
                ),
                Err(RuntimeError::HistoryTransactionAlreadyOpen)
            ));

            let malformed_error = state
                .stage_history_evidence(std::slice::from_ref(&malformed))
                .expect_err("payload digest mismatch must fail closed");
            assert!(matches!(
                malformed_error,
                RuntimeError::SnapshotAssembly(
                    causafera_persistence::PersistenceError::EvidencePayloadDigestMismatch {
                        schema_id
                    }
                ) if schema_id == schema.raw()
            ));
            let duplicate_error = state
                .stage_history_evidence(&[duplicate.clone(), duplicate])
                .expect_err("duplicate key batch must fail closed");
            assert!(matches!(
                duplicate_error,
                RuntimeError::CausalCommit(causafera_core::CausalCommitError::HistoryRead(
                    HistoryError::DuplicateRecordKey
                ))
            ));
            let unknown_error = state
                .stage_history_evidence(std::slice::from_ref(&unknown))
                .expect_err("unknown causal anchor must fail closed");
            assert!(matches!(
                unknown_error,
                RuntimeError::CausalCommit(causafera_core::CausalCommitError::HistoryRead(
                    HistoryError::UnknownEvidenceAnchor { anchor }
                )) if anchor == TraceId::new(u64::MAX)
            ));
            for rejected in [&malformed, &unknown] {
                assert!(
                    state
                        .staged_history_evidence(
                            rejected.schema_id(),
                            rejected.canonical_key(),
                            query,
                        )
                        .expect("rejected-key lookup")
                        .records()
                        .is_empty(),
                    "rejected staging left partial evidence"
                );
            }

            state
                .stage_history_evidence(std::slice::from_ref(&valid))
                .expect("valid retry stages");
            assert_eq!(
                state
                    .staged_history_evidence(schema, valid.canonical_key(), query)
                    .expect("valid staged lookup")
                    .records(),
                std::slice::from_ref(&valid)
            );
            transaction
        };
        let mut state = runtime.lock_state().expect("state");
        transaction
            .close(&mut state, SimulationTime::new(0))
            .expect("valid retry commits");
        emit_marker!(
            "MARKER:history_txn_handoff malformed=rejected duplicate=rejected unknown_anchor=rejected partial_evidence=false retry=committed"
        );
    }

    #[test]
    fn tick_open_does_not_capture_the_whole_runtime_state() {
        crate::tick_preimage::reset_whole_state_capture_count();
        let mut config = minimal_config(16_001, true);
        config.actor_count = 2;
        config.bootstrap_population = 4;
        let mut runtime = Runtime::new(config).expect("construction");

        runtime.tick().expect("real tick");

        let captures = crate::tick_preimage::whole_state_capture_count();
        emit_marker!("MARKER:whole_state_capture_count={captures}");
        assert_eq!(
            captures, 0,
            "RuntimeTickTransaction::open captured the complete RuntimeState"
        );
    }

    #[test]
    fn tick_hot_path_does_not_export_the_whole_runtime_state() {
        let source = include_str!("tick_transaction.rs");
        let validation = source
            .split_once("fn validate_completed_tick")
            .expect("validation function")
            .1
            .split_once("pub(crate) fn require_exportable")
            .expect("cold exportability boundary")
            .0;

        assert!(
            !validation.contains("state.export_snapshot()"),
            "the tick validation path still exports the complete RuntimeState"
        );
    }

    pub(super) fn require_lowered_snapshot_budget(
        state: &RuntimeState,
        budget: u64,
    ) -> Result<(), RuntimeError> {
        let size = envelope_encoded_size_from_data(&state.export_snapshot()).0;
        if size > budget {
            return Err(RuntimeError::SnapshotWouldExceedTotalSize { size, max: budget });
        }
        Ok(())
    }

    #[test]
    fn failed_storage_commit_is_atomic() {
        let storage_points = [
            causafera_persistence::FailurePoint::BeforeFrameFlush,
            causafera_persistence::FailurePoint::AfterFrameFlushBeforeSync,
            causafera_persistence::FailurePoint::AfterSegmentSyncBeforeCatalogCommit,
            causafera_persistence::FailurePoint::DuringCatalogCommit,
            causafera_persistence::FailurePoint::AfterCatalogCommitBeforeRootSync,
        ];
        let snapshot = |runtime: &Runtime| {
            let runtime_snapshot = runtime.snapshot().expect("runtime snapshot");
            let time = runtime.current_time();
            let scheduler_time = runtime.scheduler.current_time();
            let history = runtime.history_status().expect("history status");
            let mut store = runtime
                .lock_state()
                .expect("state")
                .history
                .backend_status()
                .expect("history store status");
            // Failed attempts and clean runs have different instrumentation, not history.
            store.append_elapsed_wall_time = Duration::ZERO;
            store.durable_sync_count = 0;
            store.durable_sync_elapsed_wall_time = Duration::ZERO;
            store.catalog_commit_count = 0;
            store.catalog_commit_elapsed_wall_time = Duration::ZERO;
            store.logical_digest_full_rescan_count = 0;
            store.catalog_full_scan_count = 0;
            (runtime_snapshot, time, scheduler_time, history, store)
        };

        for (mode, hydrology_enabled) in [("disabled", false), ("enabled", true)] {
            for point in storage_points {
                let config = minimal_config(18_000 + point.code() as u64, hydrology_enabled);
                let mut runtime = Runtime::new(config.clone()).expect("construction");
                let mut control = Runtime::new(config).expect("control construction");
                let before = snapshot(&runtime);
                for attempt in 1..=2 {
                    runtime
                        .lock_state()
                        .expect("state")
                        .history
                        .inject_storage_failure(Some(point))
                        .expect("inject storage failure");
                    let error = runtime
                        .tick()
                        .expect_err("storage failure must reject tick");
                    assert!(matches!(
                        error,
                        RuntimeError::SnapshotAssembly(
                            causafera_persistence::PersistenceError::InjectedHistoryFailure {
                                point: actual
                            }
                        ) if actual == point
                    ));
                    let after_failure = snapshot(&runtime);
                    assert_eq!(after_failure, before);
                    emit_marker!(
                        "MARKER:backend_failure mode={mode} point={} attempt={attempt} runtime_time={} scheduler_time={} runtime_logical={} runtime_durable={} visible_head={} hot_bytes={} store_logical={} store_durable={} pending={} active_extent={} logical_records={} durable_records={} causal_events={} causal_traces={} catalog_frames={} digest_unchanged=true evidence_digest_unchanged=true unchanged=true no_orphan_frame=true",
                        point.name(),
                        after_failure.1.raw(),
                        after_failure.2.raw(),
                        after_failure.3.logical_generation,
                        after_failure.3.durable_generation,
                        after_failure.3.visible_head.raw(),
                        after_failure.3.hot_bytes,
                        after_failure.4.logical_generation,
                        after_failure.4.durable_generation,
                        after_failure.4.pending_frames,
                        after_failure.4.active_extent,
                        after_failure.4.logical_records,
                        after_failure.4.durable_records,
                        after_failure.4.causal_events,
                        after_failure.4.causal_traces,
                        after_failure.4.catalog_counts.frames
                    );
                }
                runtime
                    .lock_state()
                    .expect("state")
                    .history
                    .inject_storage_failure(None)
                    .expect("clear storage failure");
                let retry = runtime.tick().expect("retry succeeds");
                let control_retry = control.tick().expect("control retry succeeds");
                control
                    .lock_state()
                    .expect("control state")
                    .history
                    .flush_durable()
                    .expect("control durable handoff");
                assert_eq!(retry, control_retry, "retry keeps deterministic state");
                let after = snapshot(&runtime);
                let control_after = snapshot(&control);
                assert_eq!(after, control_after, "retry matches clean handoff");
                assert_eq!(
                    after.3.logical_generation,
                    before.3.logical_generation + 1,
                    "retry publishes one logical runtime generation"
                );
                assert_eq!(
                    after.4.logical_generation,
                    before.4.logical_generation + 1,
                    "retry publishes one logical backend generation"
                );
                assert_eq!(
                    after.4.durable_generation, after.4.logical_generation,
                    "retry completes the durable handoff"
                );
                assert_eq!(after.3.pending_ticks, 0);
                assert_eq!(after.4.pending_frames, 0);
                assert!(after.4.active_extent > before.4.active_extent);
            }
        }
    }

    #[test]
    fn malformed_runtime_history_records_are_typed_and_atomic() {
        let root = synthetic_record(0);
        let reader = crate::history_reader::RuntimeHistoryReader::new_for_test(
            [0x61; 16],
            vec![root.clone()],
        )
        .expect("reader");
        let before = reader.publication_preimage().expect("preimage");
        let duplicate = reader.validate_append(1, &[HistoryRecord::Causal(root)]);
        assert!(matches!(
            duplicate,
            Err(HistoryError::DuplicateTraceId(trace)) if trace == TraceId::new(0)
        ));
        let unknown = OwnedCausalRecord::new(
            1,
            EventId::new(1),
            TraceId::new(1),
            SimulationTime::new(1),
            Phase::Physics,
            EventKindId::new(9_001),
            vec![TraceId::new(99)],
            synthetic_record(1).effects().to_vec(),
        )
        .expect("unknown-parent record shape");
        assert!(matches!(
            reader.validate_append(1, &[HistoryRecord::Causal(unknown)]),
            Err(HistoryError::UnknownParent { parent, .. }) if parent == TraceId::new(99)
        ));
        assert!(matches!(
            reader.validate_append(1, &[HistoryRecord::Causal(synthetic_record(2))]),
            Err(HistoryError::NonCanonicalOrder { index: 0 })
        ));
        let after = reader.publication_preimage().expect("preimage");
        assert_eq!(after.0, before.0);
        assert_eq!(after.1.finish(), before.1.finish());
        assert_eq!(after.2, before.2);
        emit_marker!(
            "MARKER:malformed_history duplicate=DuplicateTraceId unknown=UnknownParent noncanonical=NonCanonicalOrder unchanged=true"
        );
    }

    #[test]
    fn hot_overlay_stays_bounded_at_required_logical_scales() {
        for events in [100_u64, 1_000, 10_000, 1_000_000] {
            let last = synthetic_record(events - 1);
            let budget = last.canonical_field_size().expect("record bytes");
            let reader: Arc<dyn CausalHistoryReader> = Arc::new(SyntheticHistoryReader { events });
            let store =
                causafera_core::CausalTraceStore::with_cold_reader(reader, budget, vec![last])
                    .expect("bounded hot facade");
            assert_eq!(store.iter_hot().len(), 1);
            assert!(store.hot_bytes() <= store.hot_budget());
            assert!(store.event(TraceId::new(0)).expect("cold read").is_some());
            assert!(
                store
                    .event(TraceId::new(events - 1))
                    .expect("hot read")
                    .is_some()
            );
            emit_marker!(
                "MARKER:hot_scale logical_events={events} hot_bytes={} budget={}",
                store.hot_bytes(),
                store.hot_budget()
            );
        }
    }

    /// One thousand ticks complete under the bounded transaction, hydrology-
    /// disabled, with scan count bounded.
    #[test]
    fn bounded_transaction_completes_one_thousand_ticks() {
        for (mode, config) in [
            ("disabled", {
                let mut config = RuntimeConfig::new(557);
                config.active_chunk_radius = 0;
                config.hydrology.enabled = false;
                config.pattern_schedule.interval_ticks = 1_000;
                config
            }),
            ("enabled", {
                let mut config = near_cap_config(558);
                config.pattern_schedule.interval_ticks = 1_000;
                config
            }),
        ] {
            crate::history_transaction::reset_whole_history_scans();
            let mut runtime = Runtime::new(config).expect("construction");
            let initial = runtime.history_status().expect("history status");
            for expected in 1..=1_000 {
                runtime.tick().expect("1,000 ticks must complete");
                assert_eq!(
                    crate::history_transaction::whole_history_scans(),
                    0,
                    "ordinary tick {expected} never executes a whole-history scan"
                );
                let status = runtime.history_status().expect("history status");
                assert_eq!(
                    status.logical_generation,
                    initial.logical_generation + expected,
                    "one logical generation per successful tick"
                );
                assert!(status.durable_generation >= initial.durable_generation);
                assert!(status.durable_generation <= status.logical_generation);
                assert!(status.hot_bytes <= status.hot_budget);
                if expected == 1 || expected % 100 == 0 {
                    emit_marker!(
                        "MARKER:{mode}_progress tick={} generation={} head={} hot_bytes={}",
                        expected,
                        status.logical_generation,
                        status.visible_head.raw(),
                        status.hot_bytes
                    );
                }
            }
            let status = runtime.history_status().expect("history status");
            let scans = crate::history_transaction::whole_history_scans();
            assert_eq!(scans, 0, "ordinary ticks never invoke the cold oracle");
            assert!(status.pending_ticks <= 32);
            emit_marker!(
                "MARKER:{mode}_1000_ticks generation={} durable={} head={} hot_bytes={} budget={} pending_ticks={} handed_off_generation={} whole_history_scans={scans}",
                status.logical_generation,
                status.durable_generation,
                status.visible_head.raw(),
                status.hot_bytes,
                status.hot_budget,
                status.pending_ticks,
                status.handed_off_generation
            );
        }
    }
}
