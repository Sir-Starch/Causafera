use crate::*;
use causafera_core::*;
use causafera_domains::*;
use causafera_types::*;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex};

pub(crate) struct ManaRuntimeSystem {
    pub(crate) state: Arc<Mutex<RuntimeState>>,
    pub(crate) next_time: SimulationTime,
    pub(crate) parameters: ManaParameters,
}

impl ManaRuntimeSystem {
    pub(crate) fn new(state: Arc<Mutex<RuntimeState>>, parameters: ManaParameters) -> Self {
        Self {
            state,
            next_time: SimulationTime::new(1),
            parameters,
        }
    }

    pub(crate) fn execute(&mut self) -> Result<(), RuntimeError> {
        let mut state = self.state.lock().map_err(|_| RuntimeError::StatePoisoned)?;
        let pending_patterns = state
            .pending_samples
            .iter()
            .map(|sample| sample.pattern)
            .collect::<BTreeSet<_>>();
        let mut history = Vec::new();
        for pattern in pending_patterns {
            history.extend(
                state
                    .pattern_history
                    .get_window(pattern, MANA_PATTERN_HISTORY_TICKS)
                    .into_iter()
                    .filter(|sample| sample.observed_at < self.next_time),
            );
        }
        history.sort_unstable();
        let pending_samples = state.pending_samples.clone();
        let proposal = state.mana.propose_evolution(
            self.next_time,
            self.parameters,
            &pending_samples,
            &history,
        )?;
        let changes = proposal.changes();
        let mut ordered_changes = changes.clone();
        ordered_changes.sort_unstable_by_key(|(chunk, change)| {
            EventProposalKey::new(MANA_SYSTEM_ID, cell_object_id(*chunk, change.cell_index), 0)
        });
        let events = ordered_changes
            .iter()
            .map(|(chunk, change)| {
                let object_id = cell_object_id(*chunk, change.cell_index);
                CausalEventProposal::new(
                    EventProposalKey::new(MANA_SYSTEM_ID, object_id, 0),
                    EventKindId::new(MANA_EVENT_KIND),
                    change.causes.clone(),
                    vec![CausalEffect::new(
                        CausalTarget::new(
                            StateObjectKindId::new(MANA_OBJECT_KIND),
                            object_id,
                            StatePropertyId::new(MANA_PROPERTY),
                        ),
                        fingerprint_i64(0x0301, change.before),
                        fingerprint_i64(0x0301, change.after),
                    )?],
                )
                .map_err(RuntimeError::from)
            })
            .collect::<Result<Vec<_>, _>>()?;
        let traces = state
            .traces
            .commit_batch(self.next_time, Phase::Mana, events)?;
        let changed_count = changes.len() as u32;
        let traces_by_change = ordered_changes
            .iter()
            .zip(traces.iter().copied())
            .map(|((chunk, change), trace)| ((*chunk, change.cell_index), trace))
            .collect::<BTreeMap<_, _>>();
        let mut traces_by_chunk = BTreeMap::<ChartChunkCoord, Vec<TraceId>>::new();
        for (chunk, field_proposal) in proposal.field_proposals() {
            let field_traces = field_proposal
                .changes()
                .iter()
                .map(|change| {
                    traces_by_change
                        .get(&(*chunk, change.cell_index))
                        .copied()
                        .ok_or(RuntimeError::InvalidSnapshot(
                            "mana proposal is missing its committed trace",
                        ))
                })
                .collect::<Result<Vec<_>, _>>()?;
            traces_by_chunk.insert(*chunk, field_traces);
        }
        state.mana = proposal.commit(&traces_by_chunk)?;
        state.pending_samples.clear();
        state.last_mana_changes = changed_count;
        state.mana_cell_changes = state
            .mana_cell_changes
            .saturating_add(u64::from(changed_count));
        let source_descendant = state.latest_mana_trace.and_then(|source_trace| {
            traces
                .iter()
                .copied()
                .filter(|trace| trace_descends_from(&state.traces, *trace, source_trace))
                .min()
        });
        if let Some(trace) = source_descendant.or_else(|| traces.last().copied()) {
            state.latest_mana_trace = Some(trace);
        }
        self.next_time = self.next_time.tick();
        Ok(())
    }
}

pub(crate) struct ManaEffectsSystem {
    pub(crate) state: Arc<Mutex<RuntimeState>>,
    pub(crate) next_time: SimulationTime,
    pub(crate) parameters: ManaParameters,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct LocalManaMaterialSurfaceProposal {
    pub(crate) key: EventProposalKey,
    pub(crate) surface: MaterialSurfaceId,
    pub(crate) before: MaterialSurface,
    pub(crate) after_active: bool,
    pub(crate) after_condition: Option<i64>,
    pub(crate) local_mana_before: i64,
    pub(crate) local_mana_after: i64,
    pub(crate) local_mana_trace: TraceId,
    pub(crate) contact_trace: Option<TraceId>,
    pub(crate) causes: Vec<TraceId>,
}

impl ManaEffectsSystem {
    pub(crate) fn new(state: Arc<Mutex<RuntimeState>>, parameters: ManaParameters) -> Self {
        Self {
            state,
            next_time: SimulationTime::new(1),
            parameters,
        }
    }

    pub(crate) fn execute(&mut self) -> Result<(), RuntimeError> {
        let mut state = self.state.lock().map_err(|_| RuntimeError::StatePoisoned)?;
        if self.parameters.effect_threshold <= 0 {
            self.next_time = self.next_time.tick();
            return Ok(());
        }
        let mut proposals = Vec::new();
        for (ordinal, (surface_id, surface)) in state
            .material_surfaces
            .iter()
            .filter(|(_, surface)| surface.contact_count > 0)
            .enumerate()
        {
            let field = state
                .mana
                .field(surface_id.chunk)
                .ok_or(RuntimeError::InvalidSnapshot(
                    "contacted material surface has no matching mana field",
                ))?;
            let index = usize::from(surface_id.cell_index);
            let local_mana_after =
                *field
                    .intensity()
                    .get(index)
                    .ok_or(RuntimeError::InvalidSnapshot(
                        "material surface cell is outside matching mana field",
                    ))?;
            let transition = if surface.gate.active {
                (local_mana_after
                    < self
                        .parameters
                        .effect_threshold
                        .saturating_sub(self.parameters.effect_hysteresis))
                .then_some((false, None))
            } else {
                (local_mana_after > self.parameters.effect_threshold)
                    .then_some((true, Some(surface.condition.saturating_add(1))))
            };
            let Some((after_active, after_condition)) = transition else {
                continue;
            };
            let local_mana_trace = field.last_change().get(index).copied().flatten().ok_or(
                RuntimeError::UnknownManaPhysicalEffectCause {
                    cause: surface.last_transition,
                },
            )?;
            let contact_trace = surface.last_contact_trace;
            let mut causes = vec![local_mana_trace];
            if after_active {
                let trace = contact_trace.ok_or(RuntimeError::InvalidSnapshot(
                    "contacted material surface is missing its contact trace",
                ))?;
                causes.push(trace);
                if surface.last_transition != trace {
                    causes.push(surface.last_transition);
                }
            }
            if let Some(trace) = surface.gate.last_transition {
                causes.push(trace);
            }
            causes.sort_unstable();
            causes.dedup();
            proposals.push(LocalManaMaterialSurfaceProposal {
                key: EventProposalKey::new(
                    MANA_EFFECTS_SYSTEM_ID,
                    material_surface_object_id(*surface_id),
                    u64::try_from(ordinal).map_err(|_| {
                        RuntimeError::CausalCommit(CausalCommitError::IdentifierExhausted)
                    })?,
                ),
                surface: *surface_id,
                before: *surface,
                after_active,
                after_condition,
                local_mana_before: *field.last_change_before().get(index).ok_or(
                    RuntimeError::InvalidSnapshot("mana prior value is outside matching field"),
                )?,
                local_mana_after,
                local_mana_trace,
                contact_trace: after_active.then_some(contact_trace).flatten(),
                causes,
            });
        }
        proposals.sort_unstable_by_key(|proposal| proposal.key);
        let traces =
            commit_mana_material_surface_effect_events(&mut state, self.next_time, &proposals)?;
        for (proposal, trace) in proposals.iter().zip(traces) {
            if proposal.after_condition.is_some() {
                state.mana_physical_effects = state.mana_physical_effects.saturating_add(1);
            }
            state.latest_physical_trace = trace;
            apply_local_mana_material_surface_transition(
                &mut state,
                self.next_time,
                proposal,
                trace,
            );
        }
        self.next_time = self.next_time.tick();
        Ok(())
    }
}

impl System for ManaEffectsSystem {
    fn run(&mut self, _stream: &mut RandomStream) {
        if let Err(error) = self.execute()
            && let Ok(mut state) = self.state.lock() {
                state.failure.get_or_insert(error);
            }
    }

    fn restore_time(&mut self, time: SimulationTime) {
        self.next_time = time;
    }
}

impl System for ManaRuntimeSystem {
    fn run(&mut self, _stream: &mut RandomStream) {
        if let Err(error) = self.execute()
            && let Ok(mut state) = self.state.lock() {
                state.failure.get_or_insert(error);
            }
    }

    fn restore_time(&mut self, time: SimulationTime) {
        self.next_time = time;
    }
}

pub(crate) fn mana_totals_by_chunk(mana: &ManaFieldSet) -> BTreeMap<ChartChunkCoord, i64> {
    mana.fields()
        .iter()
        .map(|(chunk, field)| (*chunk, field.intensity().iter().copied().sum()))
        .collect()
}

pub(crate) fn refresh_active_chunk_conserved_state(
    active_chunks: &mut BTreeMap<ChartChunkCoord, ActiveChunkState>,
    event_counts: &BTreeMap<ChartChunkCoord, u64>,
    mana_totals: &BTreeMap<ChartChunkCoord, i64>,
) {
    for (chunk, active) in active_chunks {
        active.total_mana = mana_totals.get(chunk).copied().unwrap_or(0);
        active.event_count = event_counts.get(chunk).copied().unwrap_or(0);
    }
}
