use crate::*;
use causafera_core::*;
use causafera_domains::*;
use causafera_resolution::*;
use causafera_types::*;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

pub(crate) struct ResolutionRuntimeSystem {
    pub(crate) state: Arc<Mutex<RuntimeState>>,
    pub(crate) next_time: SimulationTime,
}

impl ResolutionRuntimeSystem {
    pub(crate) fn new(state: Arc<Mutex<RuntimeState>>) -> Self {
        Self {
            state,
            next_time: SimulationTime::new(1),
        }
    }

    pub(crate) fn execute(&mut self) -> Result<(), RuntimeError> {
        let mut state = self.state.lock().map_err(|_| RuntimeError::StatePoisoned)?;
        let mana_totals = mana_totals_by_chunk(&state.mana);
        let event_counts = pattern_event_counts_by_chunk(&state.pattern_history);
        refresh_active_chunk_conserved_state(&mut state.active_chunks, &event_counts, &mana_totals);
        let signals = resolution_signals(&state, &mana_totals, self.next_time)?;
        let proposal = state.resolution.propose_evaluation(
            self.next_time,
            &state.resolution_policy,
            &signals,
        )?;
        let changes = proposal.changes().to_vec();
        let events = changes
            .iter()
            .map(|change| {
                let object_id = chart_chunk_hash(change.chunk);
                CausalEventProposal::new(
                    EventProposalKey::new(RESOLUTION_SYSTEM_ID, object_id, 0),
                    EventKindId::new(RESOLUTION_EVENT_KIND),
                    change.causes().to_vec(),
                    vec![CausalEffect::new(
                        CausalTarget::new(
                            StateObjectKindId::new(RESOLUTION_OBJECT_KIND),
                            object_id,
                            StatePropertyId::new(RESOLUTION_PROPERTY),
                        ),
                        fingerprint_pair(
                            0x0401,
                            change.before_relevance,
                            i64::from(change.before_level),
                        ),
                        fingerprint_pair(
                            0x0401,
                            change.after_relevance,
                            i64::from(change.after_level),
                        ),
                    )?],
                )
                .map_err(RuntimeError::from)
            })
            .collect::<Result<Vec<_>, _>>()?;
        let traces = state
            .traces
            .commit_batch(self.next_time, Phase::Resolution, events)?;
        state.resolution = proposal.commit(&traces)?;
        apply_resolution_transitions(&mut state.active_chunks, &changes, &traces, &mana_totals);
        state.resolution_changes = state
            .resolution_changes
            .saturating_add(changes.len() as u64);
        state.resolution_transitions = state.resolution_transitions.saturating_add(
            changes
                .iter()
                .filter(|change| change.before_level != change.after_level)
                .count() as u64,
        );
        state.advanced_through = self.next_time;
        self.next_time = self.next_time.tick();
        Ok(())
    }
}

impl System for ResolutionRuntimeSystem {
    fn run(&mut self, _stream: &mut RandomStream) {
        if let Err(error) = self.execute()
            && let Ok(mut state) = self.state.lock()
        {
            state.failure.get_or_insert(error);
        }
    }

    fn restore_time(&mut self, time: SimulationTime) {
        self.next_time = time;
    }
}

fn resolution_signals(
    state: &RuntimeState,
    mana_totals: &BTreeMap<ChartChunkCoord, i64>,
    time: SimulationTime,
) -> Result<Vec<CausalRelevanceSignal>, ResolutionError> {
    let mut signals = Vec::new();
    let Some(mana_trace) = state.latest_mana_trace else {
        return Ok(terrain_relevance_signals(state, time));
    };
    let mut ordinal = 0_u32;
    for (chunk, total) in mana_totals {
        if *total > 0 {
            signals.push(CausalRelevanceSignal::new(
                *chunk,
                *chunk,
                ResolutionChannelId::new(RESOLUTION_CHANNEL),
                (*total).clamp(1, 1_000),
                mana_trace,
                ordinal,
            )?);
            ordinal = ordinal.saturating_add(1);
        }
    }
    for (chunk, total) in mana_totals {
        for neighbor in [
            chunk.same_chart_neighbor(1, 0, 0),
            chunk.same_chart_neighbor(0, 1, 0),
            chunk.same_chart_neighbor(0, 0, 1),
        ] {
            if let Some(neighbor_total) = mana_totals.get(&neighbor) {
                let strength = (*total)
                    .saturating_add(*neighbor_total)
                    .saturating_div(2)
                    .clamp(1, 1_000);
                signals.push(CausalRelevanceSignal::new(
                    *chunk,
                    neighbor,
                    ResolutionChannelId::new(RESOLUTION_CHANNEL),
                    strength,
                    mana_trace,
                    ordinal,
                )?);
                ordinal = ordinal.saturating_add(1);
                signals.push(CausalRelevanceSignal::new(
                    neighbor,
                    *chunk,
                    ResolutionChannelId::new(RESOLUTION_CHANNEL),
                    strength,
                    mana_trace,
                    ordinal,
                )?);
                ordinal = ordinal.saturating_add(1);
            }
        }
    }
    signals.extend(
        terrain_relevance_signals(state, time)
            .into_iter()
            .map(|signal| {
                let next = CausalRelevanceSignal::new(
                    signal.source(),
                    signal.target(),
                    ResolutionChannelId::new(RESOLUTION_CHANNEL),
                    1_000,
                    state.latest_physical_trace,
                    ordinal,
                );
                ordinal = ordinal.saturating_add(1);
                next
            })
            .collect::<Result<Vec<_>, _>>()?,
    );
    Ok(signals)
}

fn terrain_relevance_signals(
    state: &RuntimeState,
    time: SimulationTime,
) -> Vec<CausalRelevanceSignal> {
    state
        .carrier_adapters
        .iter()
        .enumerate()
        .filter_map(|(index, (chunk, adapter))| {
            let strength = adapter
                .emit_samples(time, state.latest_physical_trace)
                .iter()
                .map(|sample| i64::from(sample.magnitude))
                .sum::<i64>()
                .saturating_div(1_024)
                .clamp(1, 1_000);
            CausalRelevanceSignal::new(
                *chunk,
                *chunk,
                ResolutionChannelId::new(RESOLUTION_CHANNEL),
                strength,
                state.latest_physical_trace,
                index as u32,
            )
            .ok()
        })
        .collect()
}

fn apply_resolution_transitions(
    active_chunks: &mut BTreeMap<ChartChunkCoord, ActiveChunkState>,
    changes: &[causafera_resolution::ResolutionChange],
    traces: &[TraceId],
    mana_totals: &BTreeMap<ChartChunkCoord, i64>,
) {
    for (change, trace) in changes.iter().zip(traces.iter().copied()) {
        if let Some(active) = active_chunks.get_mut(&change.chunk) {
            let conserved_mana = mana_totals.get(&change.chunk).copied().unwrap_or(0);
            active.relevance = change.after_relevance;
            active.level = change.after_level;
            active.total_mana = conserved_mana;
            active.last_transition = Some(trace);
        }
    }
}
