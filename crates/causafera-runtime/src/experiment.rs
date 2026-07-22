use crate::*;
use causafera_core::*;
use causafera_types::*;
use std::sync::{Arc, Mutex};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PhysicalPatternSchedule {
    pub interval_ticks: u64,
    pub magnitude: u32,
    pub suppressed_from: Option<SimulationTime>,
    pub suppressed_through: Option<SimulationTime>,
}

impl PhysicalPatternSchedule {
    pub const fn continuous(magnitude: u32) -> Self {
        Self {
            interval_ticks: 1,
            magnitude,
            suppressed_from: None,
            suppressed_through: None,
        }
    }

    pub fn with_suppression(
        mut self,
        from: SimulationTime,
        through: SimulationTime,
    ) -> Result<Self, RuntimeError> {
        if from.raw() == 0 || through < from {
            return Err(RuntimeError::InvalidPatternSchedule);
        }
        self.suppressed_from = Some(from);
        self.suppressed_through = Some(through);
        Ok(self)
    }

    pub(crate) fn validate(self) -> Result<Self, RuntimeError> {
        if self.interval_ticks == 0 || self.magnitude == 0 {
            return Err(RuntimeError::InvalidPatternSchedule);
        }
        match (self.suppressed_from, self.suppressed_through) {
            (None, None) => {}
            (Some(from), Some(through)) if from.raw() > 0 && through >= from => {}
            _ => return Err(RuntimeError::InvalidPatternSchedule),
        }
        Ok(self)
    }

    pub(crate) fn emits_at(self, time: SimulationTime) -> bool {
        if time.raw() % self.interval_ticks != 0 {
            return false;
        }
        !matches!(
            (self.suppressed_from, self.suppressed_through),
            (Some(from), Some(through)) if time >= from && time <= through
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExperimentRecipeManaSource {
    pub source_record_id: u64,
    pub enabled: bool,
    pub scheduled_tick: u64,
    pub target_chunk: ChartChunkCoord,
    pub cell_index: u16,
    pub amount: i64,
    pub per_record_maximum: i64,
    pub policy_schema_id: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExperimentRecipeManaSourceRecipe {
    pub records: Vec<ExperimentRecipeManaSource>,
    pub recipe_budget: i64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExperimentRecipeManaSourceReceipt {
    pub source_record_id: u64,
    pub scheduled_tick: u64,
    pub executed_tick: u64,
    pub source_trace: TraceId,
    pub before_intensity: i64,
    pub after_intensity: i64,
    pub recipe_hash: StateFingerprint,
    pub policy_schema_id: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExperimentRecipeManaSourceReceiptSnapshot {
    pub source_record_id: u64,
    pub scheduled_tick: u64,
    pub executed_tick: u64,
    pub source_trace: TraceId,
    pub before_intensity: i64,
    pub after_intensity: i64,
    pub recipe_hash: StateFingerprint,
    pub policy_schema_id: u64,
}

impl ExperimentRecipeManaSourceRecipe {
    pub fn recipe_hash(&self) -> StateFingerprint {
        let mut digest = CanonicalDigest::new();
        digest.write(u64::try_from(self.records.len()).unwrap_or(u64::MAX));
        let mut records = self.records.clone();
        records.sort_unstable_by_key(|record| (record.scheduled_tick, record.source_record_id));
        for record in records {
            digest.write(record.source_record_id);
            digest.write(u64::from(record.enabled));
            digest.write(record.scheduled_tick);
            digest.write(record.target_chunk.chart.raw());
            digest.write(u64::from_le_bytes(
                i64::from(record.target_chunk.chunk.x).to_le_bytes(),
            ));
            digest.write(u64::from_le_bytes(
                i64::from(record.target_chunk.chunk.y).to_le_bytes(),
            ));
            digest.write(u64::from_le_bytes(
                i64::from(record.target_chunk.chunk.z).to_le_bytes(),
            ));
            digest.write(u64::from(record.cell_index));
            digest.write(u64::from_le_bytes(record.amount.to_le_bytes()));
            digest.write(u64::from_le_bytes(record.per_record_maximum.to_le_bytes()));
            digest.write(record.policy_schema_id);
        }
        digest.write(u64::from_le_bytes(self.recipe_budget.to_le_bytes()));
        digest.finish()
    }
}

pub(crate) struct ExperimentRecipeManaSourceSystem {
    pub(crate) state: Arc<Mutex<RuntimeState>>,
    pub(crate) next_time: SimulationTime,
}

impl ExperimentRecipeManaSourceSystem {
    pub(crate) fn new(state: Arc<Mutex<RuntimeState>>) -> Self {
        Self {
            state,
            next_time: SimulationTime::new(1),
        }
    }

    pub(crate) fn execute(&mut self) -> Result<(), RuntimeError> {
        let mut state = self.state.lock().map_err(|_| RuntimeError::StatePoisoned)?;
        let recipe_hash = state.config.experiment_recipe_mana_sources.recipe_hash();
        let due_records = state
            .config
            .experiment_recipe_mana_sources
            .records
            .iter()
            .filter(|record| {
                record.enabled
                    && record.amount > 0
                    && record.scheduled_tick == self.next_time.raw()
                    && !state
                        .executed_experiment_recipe_mana_sources
                        .iter()
                        .any(|receipt| receipt.source_record_id == record.source_record_id)
            })
            .cloned()
            .collect::<Vec<_>>();
        let mut pending = due_records
            .into_iter()
            .map(|record| {
                let proposal = state.mana.propose_experiment_recipe_mana_source(
                    record.target_chunk,
                    record.cell_index,
                    record.amount,
                )?;
                Ok::<_, RuntimeError>((record, proposal))
            })
            .collect::<Result<Vec<_>, _>>()?;
        pending.sort_unstable_by_key(|(_, proposal)| {
            EventProposalKey::new(
                EXPERIMENT_RECIPE_MANA_SOURCE_SYSTEM_ID,
                cell_object_id(proposal.chunk, proposal.cell_index),
                0,
            )
        });
        if pending.is_empty() {
            self.next_time = self.next_time.tick();
            return Ok(());
        }
        if state
            .executed_experiment_recipe_mana_sources
            .len()
            .saturating_add(pending.len())
            > MAX_EXPERIMENT_RECIPE_MANA_SOURCES
        {
            return Err(RuntimeError::ExperimentRecipeSourceCountExceeded {
                count: state
                    .executed_experiment_recipe_mana_sources
                    .len()
                    .saturating_add(pending.len()),
            });
        }

        let next_trace_id = state.traces.export_snapshot().next_trace_id;
        let events = pending
            .iter()
            .enumerate()
            .map(|(index, (record, proposal))| {
                let trace_offset = u64::try_from(index).map_err(|_| {
                    RuntimeError::CausalCommit(CausalCommitError::IdentifierExhausted)
                })?;
                let source_trace = TraceId::new(next_trace_id.checked_add(trace_offset).ok_or(
                    RuntimeError::CausalCommit(CausalCommitError::IdentifierExhausted),
                )?);
                let cell_effect = CausalEffect::new(
                    CausalTarget::new(
                        StateObjectKindId::new(EXPERIMENT_RECIPE_MANA_SOURCE_OBJECT_KIND),
                        cell_object_id(proposal.chunk, proposal.cell_index),
                        StatePropertyId::new(EXPERIMENT_RECIPE_MANA_SOURCE_PROPERTY),
                    ),
                    fingerprint_i64(0x0302, proposal.before),
                    fingerprint_i64(0x0302, proposal.after),
                )?;
                let receipt_effect = CausalEffect::new(
                    CausalTarget::new(
                        StateObjectKindId::new(EXPERIMENT_RECIPE_MANA_SOURCE_OBJECT_KIND),
                        record.source_record_id,
                        StatePropertyId::new(EXPERIMENT_RECIPE_MANA_SOURCE_PROPERTY),
                    ),
                    fingerprint_u64(0x0303, 0),
                    experiment_recipe_mana_source_receipt_fingerprint(
                        record,
                        self.next_time.raw(),
                        source_trace,
                        proposal.before,
                        proposal.after,
                        recipe_hash,
                    ),
                )?;
                let mut effects = vec![cell_effect, receipt_effect];
                effects.sort_unstable_by_key(|effect| effect.target());
                CausalEventProposal::new(
                    EventProposalKey::new(
                        EXPERIMENT_RECIPE_MANA_SOURCE_SYSTEM_ID,
                        cell_object_id(proposal.chunk, proposal.cell_index),
                        0,
                    ),
                    EventKindId::new(EXPERIMENT_RECIPE_MANA_SOURCE_EVENT_KIND),
                    Vec::new(),
                    effects,
                )
                .map_err(RuntimeError::from)
            })
            .collect::<Result<Vec<_>, _>>()?;
        let traces = state
            .traces
            .commit_batch(self.next_time, Phase::Mana, events)?;
        for ((record, proposal), trace) in pending.iter().zip(traces.iter().copied()) {
            state.mana = state
                .mana
                .clone()
                .commit_experiment_recipe_mana_source(*proposal, trace)?;
            state.latest_mana_trace = Some(trace);
            state
                .executed_experiment_recipe_mana_sources
                .push(ExperimentRecipeManaSourceReceipt {
                    source_record_id: record.source_record_id,
                    scheduled_tick: record.scheduled_tick,
                    executed_tick: self.next_time.raw(),
                    source_trace: trace,
                    before_intensity: proposal.before,
                    after_intensity: proposal.after,
                    recipe_hash,
                    policy_schema_id: record.policy_schema_id,
                });
        }
        state
            .executed_experiment_recipe_mana_sources
            .sort_unstable_by_key(|receipt| (receipt.executed_tick, receipt.source_record_id));
        self.next_time = self.next_time.tick();
        Ok(())
    }
}

impl System for ExperimentRecipeManaSourceSystem {
    fn run(&mut self, _stream: &mut RandomStream) {
        if let Err(error) = self.execute() {
            if let Ok(mut state) = self.state.lock() {
                state.failure.get_or_insert(error);
            }
        }
    }

    fn restore_time(&mut self, time: SimulationTime) {
        self.next_time = time;
    }
}
