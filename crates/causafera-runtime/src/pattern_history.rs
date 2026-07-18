use std::collections::{BTreeMap, VecDeque};

use causafera_domains::PhysicalPatternSample;
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
    per_pattern_cap: usize,
    global_cap: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PatternHistorySnapshot {
    pub samples: Vec<PhysicalPatternSample>,
    pub global_cap: usize,
    pub per_pattern_cap: usize,
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
        if self.global_cap == 0 || self.per_pattern_cap == 0 {
            return;
        }
        self.per_pattern
            .entry(sample.pattern)
            .or_default()
            .push_back(sample);
        self.insertion_order.push_back(sample);
        self.enforce_per_pattern_cap(sample.pattern);
        self.enforce_global_cap();
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

    fn enforce_per_pattern_cap(&mut self, pattern: PhysicalPatternId) {
        loop {
            let should_evict = self
                .per_pattern
                .get(&pattern)
                .is_some_and(|samples| samples.len() > self.per_pattern_cap);
            if !should_evict {
                return;
            }
            self.evict_oldest_for_pattern(pattern);
        }
    }

    fn enforce_global_cap(&mut self) {
        while self.insertion_order.len() > self.global_cap {
            self.evict_oldest_global();
        }
    }

    fn evict_oldest_global(&mut self) {
        let Some(sample) = self.insertion_order.pop_front() else {
            return;
        };
        remove_first_matching(&mut self.per_pattern, sample);
    }

    fn evict_oldest_for_pattern(&mut self, pattern: PhysicalPatternId) {
        let Some(sample) = self
            .per_pattern
            .get_mut(&pattern)
            .and_then(VecDeque::pop_front)
        else {
            return;
        };
        if self
            .per_pattern
            .get(&pattern)
            .is_some_and(VecDeque::is_empty)
        {
            self.per_pattern.remove(&pattern);
        }
        if let Some(index) = self
            .insertion_order
            .iter()
            .position(|candidate| *candidate == sample)
        {
            self.insertion_order.remove(index);
        }
    }
}

fn within_window(observed_at: SimulationTime, newest: SimulationTime, max_ticks: u64) -> bool {
    newest.raw().saturating_sub(observed_at.raw()) < max_ticks
}

fn remove_first_matching(
    per_pattern: &mut BTreeMap<PhysicalPatternId, VecDeque<PhysicalPatternSample>>,
    sample: PhysicalPatternSample,
) {
    let Some(samples) = per_pattern.get_mut(&sample.pattern) else {
        return;
    };
    if let Some(index) = samples.iter().position(|candidate| *candidate == sample) {
        samples.remove(index);
    }
    if samples.is_empty() {
        per_pattern.remove(&sample.pattern);
    }
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
