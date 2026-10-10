use crate::*;
use causafera_core::*;
use causafera_types::*;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex};

use std::collections::VecDeque;

use causafera_domains::{PhysicalCarrierAdapter, PhysicalPatternSample};
use causafera_types::{PhysicalPatternId, SimulationTime};

/// Deterministic bounded FIFO history for physical pattern samples.
///
/// The policy is insertion-order FIFO with two hard limits: each pattern retains
/// at most `per_pattern_cap` samples, and the whole history retains at most
/// `global_cap` samples. When a cap is exceeded, the oldest inserted matching
/// entry is evicted. Samples store only physical pattern fields and trace
/// ancestry; no semantic, English, observer, or subjective data is retained.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PhysicalPatternHistory {
    per_pattern: BTreeMap<PhysicalPatternId, VecDeque<PhysicalPatternSample>>,
    insertion_order: VecDeque<PhysicalPatternSample>,
    pub(crate) per_pattern_cap: usize,
    pub(crate) global_cap: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PatternHistorySnapshot {
    pub samples: Vec<PhysicalPatternSample>,
    pub global_cap: usize,
    pub per_pattern_cap: usize,
}

pub(crate) struct PatternHistoryUndo {
    sample: PhysicalPatternSample,
    evictions: Vec<PatternHistoryEviction>,
}

struct PatternHistoryEviction {
    sample: PhysicalPatternSample,
    insertion_index: usize,
    per_pattern_index: usize,
}

impl PhysicalPatternHistory {
    pub fn new(global_cap: usize, per_pattern_cap: usize) -> Self {
        Self {
            per_pattern: BTreeMap::new(),
            insertion_order: VecDeque::new(),
            per_pattern_cap,
            global_cap,
        }
    }

    pub fn push(&mut self, sample: PhysicalPatternSample) {
        let _ = self.push_with_undo(sample);
    }

    pub(crate) fn push_with_undo(&mut self, sample: PhysicalPatternSample) -> PatternHistoryUndo {
        let mut undo = PatternHistoryUndo {
            sample,
            evictions: Vec::new(),
        };
        if self.global_cap == 0 || self.per_pattern_cap == 0 {
            return undo;
        }
        self.per_pattern
            .entry(sample.pattern)
            .or_default()
            .push_back(sample);
        self.insertion_order.push_back(sample);
        while self
            .per_pattern
            .get(&sample.pattern)
            .is_some_and(|samples| samples.len() > self.per_pattern_cap)
        {
            if let Some(eviction) = self.evict_oldest_for_pattern_with_position(sample.pattern) {
                undo.evictions.push(eviction);
            }
        }
        while self.insertion_order.len() > self.global_cap {
            if let Some(eviction) = self.evict_oldest_global_with_position() {
                undo.evictions.push(eviction);
            }
        }
        undo
    }

    pub(crate) fn undo_push(&mut self, undo: PatternHistoryUndo) {
        if self.global_cap == 0 || self.per_pattern_cap == 0 {
            return;
        }
        if self.insertion_order.back() == Some(&undo.sample) {
            self.insertion_order.pop_back();
        }
        if let Some(samples) = self.per_pattern.get_mut(&undo.sample.pattern) {
            if samples.back() == Some(&undo.sample) {
                samples.pop_back();
            }
            if samples.is_empty() {
                self.per_pattern.remove(&undo.sample.pattern);
            }
        }
        for eviction in undo.evictions.into_iter().rev() {
            self.insertion_order
                .insert(eviction.insertion_index, eviction.sample);
            self.per_pattern
                .entry(eviction.sample.pattern)
                .or_default()
                .insert(eviction.per_pattern_index, eviction.sample);
        }
    }

    pub fn extend<I>(&mut self, samples: I)
    where
        I: IntoIterator<Item = PhysicalPatternSample>,
    {
        for sample in samples {
            self.push(sample);
        }
    }

    pub fn get_window(
        &self,
        pattern: PhysicalPatternId,
        max_ticks: u64,
    ) -> Vec<PhysicalPatternSample> {
        if max_ticks == 0 {
            return Vec::new();
        }
        let Some(samples) = self.per_pattern.get(&pattern) else {
            return Vec::new();
        };
        let Some(newest) = samples.iter().map(|sample| sample.observed_at).max() else {
            return Vec::new();
        };
        samples
            .iter()
            .copied()
            .filter(|sample| within_window(sample.observed_at, newest, max_ticks))
            .collect()
    }

    pub fn len(&self) -> usize {
        self.insertion_order.len()
    }

    pub fn is_empty(&self) -> bool {
        self.insertion_order.is_empty()
    }

    pub fn samples(&self) -> impl Iterator<Item = &PhysicalPatternSample> {
        self.insertion_order.iter()
    }

    pub const fn global_cap(&self) -> usize {
        self.global_cap
    }

    pub const fn per_pattern_cap(&self) -> usize {
        self.per_pattern_cap
    }

    pub fn export_snapshot(&self) -> PatternHistorySnapshot {
        PatternHistorySnapshot {
            samples: self.samples().copied().collect(),
            global_cap: self.global_cap,
            per_pattern_cap: self.per_pattern_cap,
        }
    }

    pub fn import_snapshot(snapshot: PatternHistorySnapshot) -> Self {
        let mut history = Self::new(snapshot.global_cap, snapshot.per_pattern_cap);
        history.extend(snapshot.samples);
        history
    }

    fn evict_oldest_global_with_position(&mut self) -> Option<PatternHistoryEviction> {
        let sample = self.insertion_order.pop_front()?;
        let per_pattern_index = self
            .per_pattern
            .get(&sample.pattern)
            .and_then(|samples| samples.iter().position(|candidate| *candidate == sample))?;
        if let Some(samples) = self.per_pattern.get_mut(&sample.pattern) {
            samples.remove(per_pattern_index);
            if samples.is_empty() {
                self.per_pattern.remove(&sample.pattern);
            }
        }
        Some(PatternHistoryEviction {
            sample,
            insertion_index: 0,
            per_pattern_index,
        })
    }

    fn evict_oldest_for_pattern_with_position(
        &mut self,
        pattern: PhysicalPatternId,
    ) -> Option<PatternHistoryEviction> {
        let sample = self.per_pattern.get_mut(&pattern)?.pop_front()?;
        if self
            .per_pattern
            .get(&pattern)
            .is_some_and(VecDeque::is_empty)
        {
            self.per_pattern.remove(&pattern);
        }
        let insertion_index = self
            .insertion_order
            .iter()
            .position(|candidate| *candidate == sample)?;
        self.insertion_order.remove(insertion_index);
        Some(PatternHistoryEviction {
            sample,
            insertion_index,
            per_pattern_index: 0,
        })
    }
}

fn within_window(observed_at: SimulationTime, newest: SimulationTime, max_ticks: u64) -> bool {
    newest.raw().saturating_sub(observed_at.raw()) < max_ticks
}

pub(crate) struct PhysicalPatternSystem {
    pub(crate) state: Arc<Mutex<RuntimeState>>,
    pub(crate) schedule: PhysicalPatternSchedule,
    pub(crate) next_time: SimulationTime,
}

impl PhysicalPatternSystem {
    pub(crate) fn new(state: Arc<Mutex<RuntimeState>>, schedule: PhysicalPatternSchedule) -> Self {
        Self {
            state,
            schedule,
            next_time: SimulationTime::new(1),
        }
    }

    pub(crate) fn execute(&mut self) -> Result<(), RuntimeError> {
        let mut state = self.state.lock().map_err(|_| RuntimeError::StatePoisoned)?;
        state.clear_pending_samples();
        if self.schedule.emits_at(self.next_time) {
            let changed = state.take_pending_material_surface_changes();
            let adapter = MaterialSurfaceCarrierAdapter::new(state.config.chunk_extent);
            let emitted = changed
                .into_iter()
                .filter_map(|id| {
                    state
                        .material_surfaces
                        .get(&id)
                        .copied()
                        .map(|surface| (id, surface))
                })
                .flat_map(|(id, surface)| adapter.emit_samples(id, surface, self.next_time))
                .map(|sample| PhysicalPatternSample {
                    magnitude: sample.magnitude.saturating_add(self.schedule.magnitude),
                    ..sample
                })
                .collect::<Vec<_>>();
            state.physical_events = state
                .physical_events
                .saturating_add(u64::try_from(emitted.len()).unwrap_or(u64::MAX));
            state.extend_pending_samples(emitted.iter().copied());
            for sample in emitted {
                state.push_pattern_sample(sample);
            }
            // Terrain reaches the field but not the history, and not the event
            // count. Both retain change: a structure that is simply still there
            // has not happened again. Retaining it would let the recurrence and
            // periodicity channels score the rate at which the carrier is read
            // rather than anything about the world, and would evict the
            // change-driven samples those channels exist to measure.
            let terrain = self.terrain_samples(&state);
            state.extend_pending_samples(terrain);
        }
        self.next_time = self.next_time.tick();
        Ok(())
    }

    /// The standing terrain structure of every active chunk.
    ///
    /// The schedule's magnitude is deliberately not added. That constant is the
    /// configured strength of the change-driven physical source; terrain's
    /// magnitude is the structure it actually has, so a featureless chunk
    /// contributes nothing however loud the schedule is. The cause is the
    /// terrain's own generation trace, which is the truthful answer to why this
    /// structure is here at this tick.
    fn terrain_samples(&self, state: &RuntimeState) -> Vec<PhysicalPatternSample> {
        if state.config.terrain_participation == TerrainParticipation::Inert {
            return Vec::new();
        }
        state
            .carrier_adapters
            .values()
            .flat_map(|adapter| {
                PhysicalCarrierAdapter::emit_samples(
                    adapter,
                    self.next_time,
                    adapter.terrain().provenance().generation_trace(),
                )
            })
            .collect()
    }
}

impl System for PhysicalPatternSystem {
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

pub(crate) fn validate_trace_ancestry(ancestry: &[TraceId]) -> Result<(), RuntimeError> {
    if ancestry.windows(2).any(|window| window[0] >= window[1]) {
        return Err(RuntimeError::InvalidPopulationAggregate);
    }
    Ok(())
}

pub(crate) fn append_trace(
    ancestry: &[TraceId],
    trace: TraceId,
) -> Result<Vec<TraceId>, RuntimeError> {
    let mut next = ancestry.to_vec();
    next.push(trace);
    next.sort_unstable();
    next.dedup();
    validate_trace_ancestry(&next)?;
    Ok(next)
}

pub(crate) fn trace_descends_from(
    store: &CausalTraceStore,
    trace: TraceId,
    ancestor: TraceId,
) -> bool {
    let mut pending = vec![trace];
    let mut visited = BTreeSet::new();
    while let Some(candidate) = pending.pop() {
        if candidate == ancestor {
            return true;
        }
        if candidate < ancestor {
            continue;
        }
        if !visited.insert(candidate) {
            continue;
        }
        if let Some(event) = store.event(candidate).ok().flatten() {
            pending.extend(event.causes().iter().copied());
        }
    }
    false
}

pub(crate) fn merge_trace_ancestry(
    left: &[TraceId],
    right: &[TraceId],
    trace: TraceId,
) -> Result<Vec<TraceId>, RuntimeError> {
    let mut next = Vec::with_capacity(left.len() + right.len() + 1);
    next.extend_from_slice(left);
    next.extend_from_slice(right);
    next.push(trace);
    next.sort_unstable();
    next.dedup();
    validate_trace_ancestry(&next)?;
    Ok(next)
}

pub(crate) fn pattern_event_counts_by_chunk(
    history: &PhysicalPatternHistory,
) -> BTreeMap<ChartChunkCoord, u64> {
    let mut event_counts = BTreeMap::<ChartChunkCoord, u64>::new();
    for sample in history.samples() {
        *event_counts.entry(sample.chunk).or_default() += 1;
    }
    event_counts
}
#[cfg(test)]
mod tests {
    use super::*;
    use causafera_types::{ChartChunkCoord, ChunkCoord, LocalCoord, SpatialChartId, TraceId};

    fn chunk() -> ChartChunkCoord {
        ChartChunkCoord::new(SpatialChartId::new(1), ChunkCoord::new(0, 0, 0))
    }

    fn sample(pattern: u64, tick: u64, ordinal: u32) -> PhysicalPatternSample {
        PhysicalPatternSample {
            chunk: chunk(),
            pattern: PhysicalPatternId::new(pattern),
            position: LocalCoord::new(1, 1, 1),
            observed_at: SimulationTime::new(tick),
            magnitude: 1_024,
            source_ordinal: ordinal,
            cause: TraceId::new(u64::from(ordinal) + 1),
        }
    }

    #[test]
    fn old_evidence_expires_under_tick_window() {
        let pattern = PhysicalPatternId::new(7);
        let mut history = PhysicalPatternHistory::new(8, 8);
        history.extend([sample(7, 1, 1), sample(7, 2, 2), sample(7, 6, 6)]);

        let window = history.get_window(pattern, 3);

        assert_eq!(window, vec![sample(7, 6, 6)]);
    }

    #[test]
    fn history_respects_global_and_per_pattern_caps() {
        let mut history = PhysicalPatternHistory::new(4, 2);
        history.extend([
            sample(7, 1, 1),
            sample(7, 2, 2),
            sample(7, 3, 3),
            sample(8, 4, 4),
            sample(8, 5, 5),
            sample(9, 6, 6),
        ]);

        assert_eq!(history.len(), 4);
        assert_eq!(history.get_window(PhysicalPatternId::new(7), 10).len(), 1);
        assert_eq!(history.get_window(PhysicalPatternId::new(8), 10).len(), 2);
        assert_eq!(history.get_window(PhysicalPatternId::new(9), 10).len(), 1);
    }

    #[test]
    fn split_batches_produce_identical_history_state() {
        let samples = [sample(7, 1, 1), sample(7, 2, 2), sample(8, 3, 3)];
        let mut whole = PhysicalPatternHistory::new(8, 8);
        whole.extend(samples);
        let mut split = PhysicalPatternHistory::new(8, 8);
        split.extend(samples[..1].iter().copied());
        split.extend(samples[1..].iter().copied());

        assert_eq!(whole, split);
    }
}
