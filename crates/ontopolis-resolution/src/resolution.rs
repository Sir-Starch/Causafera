use ontopolis_types::{
    ChartChunkCoord, ResolutionChannelId, ResolutionFieldId, SimulationTime, TraceId,
};
use thiserror::Error;

pub const RELEVANCE_SCALE: i64 = 1_000;
pub const MAX_RESOLUTION_ENTRIES: usize = 65_536;
pub const MAX_RESOLUTION_CHANNELS: usize = 64;
pub const MAX_RESOLUTION_SIGNALS: usize = 131_072;
pub const MAX_RESOLUTION_LEVEL: u8 = 15;

/// Weight assigned to one opaque causal carrier channel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChannelWeight {
    channel: ResolutionChannelId,
    weight: i64,
}

impl ChannelWeight {
    pub fn new(channel: ResolutionChannelId, weight: i64) -> Result<Self, ResolutionError> {
        if weight <= 0 || weight > RELEVANCE_SCALE {
            return Err(ResolutionError::InvalidChannelWeight { weight });
        }
        Ok(Self { channel, weight })
    }

    pub const fn channel(self) -> ResolutionChannelId {
        self.channel
    }

    pub const fn weight(self) -> i64 {
        self.weight
    }
}

/// Fixed-point policy for reducing causal inputs into numeric detail levels.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolutionPolicy {
    maximum_relevance: i64,
    retained_relevance: i64,
    hysteresis: i64,
    thresholds: Vec<i64>,
    channels: Vec<ChannelWeight>,
}

impl ResolutionPolicy {
    pub fn new(
        maximum_relevance: i64,
        retained_relevance: i64,
        hysteresis: i64,
        thresholds: Vec<i64>,
        mut channels: Vec<ChannelWeight>,
    ) -> Result<Self, ResolutionError> {
        if maximum_relevance <= 0 {
            return Err(ResolutionError::InvalidMaximumRelevance);
        }
        if !(0..=RELEVANCE_SCALE).contains(&retained_relevance) {
            return Err(ResolutionError::InvalidRetainedRelevance);
        }
        if hysteresis < 0 || hysteresis > maximum_relevance {
            return Err(ResolutionError::InvalidHysteresis);
        }
        if thresholds.len() > MAX_RESOLUTION_LEVEL as usize {
            return Err(ResolutionError::TooManyLevels {
                count: thresholds.len(),
            });
        }
        if channels.is_empty() || channels.len() > MAX_RESOLUTION_CHANNELS {
            return Err(ResolutionError::InvalidChannelCount {
                count: channels.len(),
            });
        }
        validate_thresholds(&thresholds, maximum_relevance)?;
        channels.sort_unstable_by_key(|entry| entry.channel());
        for index in 1..channels.len() {
            if channels[index - 1].channel() == channels[index].channel() {
                return Err(ResolutionError::DuplicateChannel {
                    channel: channels[index].channel(),
                });
            }
        }
        Ok(Self {
            maximum_relevance,
            retained_relevance,
            hysteresis,
            thresholds,
            channels,
        })
    }

    pub const fn maximum_relevance(&self) -> i64 {
        self.maximum_relevance
    }

    pub fn thresholds(&self) -> &[i64] {
        &self.thresholds
    }

    pub fn channels(&self) -> &[ChannelWeight] {
        &self.channels
    }

    fn channel_weight(&self, channel: ResolutionChannelId) -> Option<i64> {
        self.channels
            .binary_search_by_key(&channel, |entry| entry.channel())
            .ok()
            .map(|index| self.channels[index].weight())
    }

    fn next_level(&self, current: u8, relevance: i64) -> u8 {
        use std::cmp::Ordering;

        let raw = self
            .thresholds
            .partition_point(|threshold| relevance >= *threshold) as u8;
        match raw.cmp(&current) {
            Ordering::Greater => {
                let boundary = self.thresholds[current as usize];
                if relevance < boundary.saturating_add(self.hysteresis) {
                    return current;
                }
            }
            Ordering::Less => {
                let boundary = self.thresholds[(current - 1) as usize];
                if relevance >= boundary.saturating_sub(self.hysteresis) {
                    return current;
                }
            }
            Ordering::Equal => {}
        }
        raw
    }
}

fn validate_thresholds(thresholds: &[i64], maximum: i64) -> Result<(), ResolutionError> {
    for (index, threshold) in thresholds.iter().copied().enumerate() {
        if threshold <= 0 || threshold > maximum {
            return Err(ResolutionError::InvalidThreshold { index, threshold });
        }
        if index > 0 && thresholds[index - 1] >= threshold {
            return Err(ResolutionError::ThresholdsNotStrictlyOrdered { index });
        }
    }
    Ok(())
}

/// Directed evidence that one chunk is causally relevant to another.
///
/// Channel IDs are opaque schema identities. They must not be interpreted as
/// semantic domain categories by this reducer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CausalRelevanceSignal {
    source: ChartChunkCoord,
    target: ChartChunkCoord,
    channel: ResolutionChannelId,
    strength: i64,
    trace: TraceId,
    ordinal: u32,
}

impl CausalRelevanceSignal {
    pub fn new(
        source: ChartChunkCoord,
        target: ChartChunkCoord,
        channel: ResolutionChannelId,
        strength: i64,
        trace: TraceId,
        ordinal: u32,
    ) -> Result<Self, ResolutionError> {
        if strength <= 0 || strength > RELEVANCE_SCALE {
            return Err(ResolutionError::InvalidSignalStrength { strength });
        }
        Ok(Self {
            source,
            target,
            channel,
            strength,
            trace,
            ordinal,
        })
    }

    pub const fn source(self) -> ChartChunkCoord {
        self.source
    }

    pub const fn target(self) -> ChartChunkCoord {
        self.target
    }

    fn canonical_key(
        self,
    ) -> (
        ChartChunkCoord,
        ResolutionChannelId,
        ChartChunkCoord,
        u32,
        TraceId,
    ) {
        (
            self.target,
            self.channel,
            self.source,
            self.ordinal,
            self.trace,
        )
    }
}

/// Dense hot state for a bounded set of spatial chunks.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolutionField {
    id: ResolutionFieldId,
    evaluated_through: SimulationTime,
    chunks: Vec<ChartChunkCoord>,
    relevance: Vec<i64>,
    levels: Vec<u8>,
    last_traces: Vec<TraceId>,
}

impl ResolutionField {
    pub fn new(
        id: ResolutionFieldId,
        evaluated_through: SimulationTime,
        mut chunks: Vec<ChartChunkCoord>,
        initial_traces: Vec<TraceId>,
    ) -> Result<Self, ResolutionError> {
        if chunks.is_empty() || chunks.len() > MAX_RESOLUTION_ENTRIES {
            return Err(ResolutionError::InvalidEntryCount {
                count: chunks.len(),
            });
        }
        if chunks.len() != initial_traces.len() {
            return Err(ResolutionError::TraceCountMismatch {
                expected: chunks.len(),
                actual: initial_traces.len(),
            });
        }
        let mut entries: Vec<_> = chunks.drain(..).zip(initial_traces).collect();
        entries.sort_unstable_by_key(|entry| entry.0);
        for index in 1..entries.len() {
            if entries[index - 1].0 == entries[index].0 {
                return Err(ResolutionError::DuplicateChunk {
                    chunk: entries[index].0,
                });
            }
        }
        let (chunks, last_traces): (Vec<_>, Vec<_>) = entries.into_iter().unzip();
        let count = chunks.len();
        Ok(Self {
            id,
            evaluated_through,
            chunks,
            relevance: vec![0; count],
            levels: vec![0; count],
            last_traces,
        })
    }

    pub const fn id(&self) -> ResolutionFieldId {
        self.id
    }

    pub const fn evaluated_through(&self) -> SimulationTime {
        self.evaluated_through
    }

    pub fn entry(&self, chunk: ChartChunkCoord) -> Option<ResolutionEntry> {
        let index = self.chunks.binary_search(&chunk).ok()?;
        Some(ResolutionEntry {
            chunk,
            relevance: self.relevance[index],
            level: self.levels[index],
            last_trace: self.last_traces[index],
        })
    }

    pub fn propose_evaluation(
        &self,
        through: SimulationTime,
        policy: &ResolutionPolicy,
        signals: &[CausalRelevanceSignal],
    ) -> Result<ResolutionProposal, ResolutionError> {
        if through <= self.evaluated_through {
            return Err(ResolutionError::NonAdvancingTime);
        }
        if signals.len() > MAX_RESOLUTION_SIGNALS {
            return Err(ResolutionError::TooManySignals {
                count: signals.len(),
            });
        }
        let mut ordered = signals.to_vec();
        ordered.sort_unstable_by_key(|signal| signal.canonical_key());
        for index in 1..ordered.len() {
            if ordered[index - 1].canonical_key() == ordered[index].canonical_key() {
                return Err(ResolutionError::DuplicateSignal { index });
            }
        }

        let mut incoming = vec![0_i64; self.chunks.len()];
        let mut causes = vec![Vec::new(); self.chunks.len()];
        for signal in ordered {
            let index = self.chunks.binary_search(&signal.target).map_err(|_| {
                ResolutionError::UnknownTarget {
                    chunk: signal.target,
                }
            })?;
            let weight =
                policy
                    .channel_weight(signal.channel)
                    .ok_or(ResolutionError::UnknownChannel {
                        channel: signal.channel,
                    })?;
            let contribution = signal.strength.saturating_mul(weight) / RELEVANCE_SCALE;
            incoming[index] = incoming[index].saturating_add(contribution);
            causes[index].push(signal.trace);
        }

        let mut next_relevance = Vec::with_capacity(self.chunks.len());
        let mut next_levels = Vec::with_capacity(self.chunks.len());
        let mut changes = Vec::new();
        for index in 0..self.chunks.len() {
            let retained =
                self.relevance[index].saturating_mul(policy.retained_relevance) / RELEVANCE_SCALE;
            let relevance = retained
                .saturating_add(incoming[index])
                .clamp(0, policy.maximum_relevance);
            let level = policy.next_level(self.levels[index], relevance);
            next_relevance.push(relevance);
            next_levels.push(level);
            if relevance != self.relevance[index] || level != self.levels[index] {
                causes[index].push(self.last_traces[index]);
                causes[index].sort_unstable();
                causes[index].dedup();
                changes.push(ResolutionChange {
                    chunk: self.chunks[index],
                    before_relevance: self.relevance[index],
                    after_relevance: relevance,
                    before_level: self.levels[index],
                    after_level: level,
                    causes: std::mem::take(&mut causes[index]),
                });
            }
        }
        Ok(ResolutionProposal {
            field_id: self.id,
            through,
            chunks: self.chunks.clone(),
            relevance: next_relevance,
            levels: next_levels,
            last_traces: self.last_traces.clone(),
            changes,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResolutionEntry {
    pub chunk: ChartChunkCoord,
    pub relevance: i64,
    pub level: u8,
    pub last_trace: TraceId,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolutionChange {
    pub chunk: ChartChunkCoord,
    pub before_relevance: i64,
    pub after_relevance: i64,
    pub before_level: u8,
    pub after_level: u8,
    causes: Vec<TraceId>,
}

impl ResolutionChange {
    pub fn causes(&self) -> &[TraceId] {
        &self.causes
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolutionProposal {
    field_id: ResolutionFieldId,
    through: SimulationTime,
    chunks: Vec<ChartChunkCoord>,
    relevance: Vec<i64>,
    levels: Vec<u8>,
    last_traces: Vec<TraceId>,
    changes: Vec<ResolutionChange>,
}

impl ResolutionProposal {
    pub fn changes(&self) -> &[ResolutionChange] {
        &self.changes
    }

    pub fn commit(mut self, traces: &[TraceId]) -> Result<ResolutionField, ResolutionError> {
        if traces.len() != self.changes.len() {
            return Err(ResolutionError::TraceCountMismatch {
                expected: self.changes.len(),
                actual: traces.len(),
            });
        }
        for (change, trace) in self.changes.iter().zip(traces.iter().copied()) {
            let index = self
                .chunks
                .binary_search(&change.chunk)
                .expect("proposal changes only contain field chunks");
            self.last_traces[index] = trace;
        }
        Ok(ResolutionField {
            id: self.field_id,
            evaluated_through: self.through,
            chunks: self.chunks,
            relevance: self.relevance,
            levels: self.levels,
            last_traces: self.last_traces,
        })
    }
}

#[derive(Clone, Copy, Debug, Error, PartialEq, Eq)]
pub enum ResolutionError {
    #[error("maximum relevance must be positive")]
    InvalidMaximumRelevance,
    #[error("retained relevance must be within the fixed-point scale")]
    InvalidRetainedRelevance,
    #[error("hysteresis is outside the relevance range")]
    InvalidHysteresis,
    #[error("invalid threshold {threshold} at index {index}")]
    InvalidThreshold { index: usize, threshold: i64 },
    #[error("thresholds are not strictly ordered at index {index}")]
    ThresholdsNotStrictlyOrdered { index: usize },
    #[error("too many resolution levels: {count}")]
    TooManyLevels { count: usize },
    #[error("invalid channel count: {count}")]
    InvalidChannelCount { count: usize },
    #[error("invalid channel weight: {weight}")]
    InvalidChannelWeight { weight: i64 },
    #[error("duplicate resolution channel {channel}")]
    DuplicateChannel { channel: ResolutionChannelId },
    #[error("invalid entry count: {count}")]
    InvalidEntryCount { count: usize },
    #[error("duplicate chunk {chunk}")]
    DuplicateChunk { chunk: ChartChunkCoord },
    #[error("invalid signal strength: {strength}")]
    InvalidSignalStrength { strength: i64 },
    #[error("too many resolution signals: {count}")]
    TooManySignals { count: usize },
    #[error("duplicate canonical signal at index {index}")]
    DuplicateSignal { index: usize },
    #[error("signal targets unknown chunk {chunk}")]
    UnknownTarget { chunk: ChartChunkCoord },
    #[error("signal uses unregistered channel {channel}")]
    UnknownChannel { channel: ResolutionChannelId },
    #[error("resolution evaluation time must advance")]
    NonAdvancingTime,
    #[error("trace count mismatch: expected {expected}, got {actual}")]
    TraceCountMismatch { expected: usize, actual: usize },
}

#[cfg(test)]
mod tests {
    use super::*;
    use ontopolis_types::{ChunkCoord, SpatialChartId};

    fn chart_chunk(x: i32) -> ChartChunkCoord {
        ChartChunkCoord::new(SpatialChartId::new(1), ChunkCoord::new(x, 0, 0))
    }

    fn policy() -> ResolutionPolicy {
        ResolutionPolicy::new(
            10_000,
            500,
            50,
            vec![500, 1_500, 3_000],
            vec![
                ChannelWeight::new(ResolutionChannelId::new(2), 500).unwrap(),
                ChannelWeight::new(ResolutionChannelId::new(1), 1_000).unwrap(),
            ],
        )
        .unwrap()
    }

    fn field() -> ResolutionField {
        ResolutionField::new(
            ResolutionFieldId::new(1),
            SimulationTime::new(0),
            vec![chart_chunk(2), chart_chunk(1)],
            vec![TraceId::new(20), TraceId::new(10)],
        )
        .unwrap()
    }

    fn signal(target: u64, strength: i64, trace: u64, ordinal: u32) -> CausalRelevanceSignal {
        CausalRelevanceSignal::new(
            chart_chunk(99),
            chart_chunk(target as i32),
            ResolutionChannelId::new(1),
            strength,
            TraceId::new(trace),
            ordinal,
        )
        .unwrap()
    }

    #[test]
    fn evaluation_is_canonical_and_proposal_only() {
        let field = field();
        let a = signal(1, 800, 31, 0);
        let b = signal(2, 600, 32, 0);
        let first = field
            .propose_evaluation(SimulationTime::new(1), &policy(), &[a, b])
            .unwrap();
        let second = field
            .propose_evaluation(SimulationTime::new(1), &policy(), &[b, a])
            .unwrap();
        assert_eq!(first, second);
        assert_eq!(field.entry(chart_chunk(1)).unwrap().relevance, 0);
    }

    #[test]
    fn causal_strength_not_distance_controls_detail() {
        let proposal = field()
            .propose_evaluation(
                SimulationTime::new(1),
                &policy(),
                &[signal(1, 400, 31, 0), signal(2, 1_000, 32, 0)],
            )
            .unwrap();
        let committed = proposal
            .commit(&[TraceId::new(41), TraceId::new(42)])
            .unwrap();
        assert_eq!(committed.entry(chart_chunk(1)).unwrap().level, 0);
        assert_eq!(committed.entry(chart_chunk(2)).unwrap().level, 1);
    }

    #[test]
    fn decay_and_hysteresis_are_deterministic() {
        let first = field()
            .propose_evaluation(SimulationTime::new(1), &policy(), &[signal(1, 600, 31, 0)])
            .unwrap()
            .commit(&[TraceId::new(41)])
            .unwrap();
        assert_eq!(first.entry(chart_chunk(1)).unwrap().level, 1);
        let second = first
            .propose_evaluation(SimulationTime::new(2), &policy(), &[])
            .unwrap()
            .commit(&[TraceId::new(42)])
            .unwrap();
        assert_eq!(second.entry(chart_chunk(1)).unwrap().level, 0);
    }

    #[test]
    fn changes_retain_canonical_causes_and_require_commit_traces() {
        let proposal = field()
            .propose_evaluation(
                SimulationTime::new(1),
                &policy(),
                &[signal(1, 600, 31, 0), signal(1, 600, 30, 1)],
            )
            .unwrap();
        assert_eq!(
            proposal.changes()[0].causes(),
            &[TraceId::new(10), TraceId::new(30), TraceId::new(31)]
        );
        assert_eq!(
            proposal.clone().commit(&[]),
            Err(ResolutionError::TraceCountMismatch {
                expected: 1,
                actual: 0
            })
        );
        assert_eq!(
            proposal
                .commit(&[TraceId::new(40)])
                .unwrap()
                .entry(chart_chunk(1))
                .unwrap()
                .last_trace,
            TraceId::new(40)
        );
    }

    #[test]
    fn policy_rejects_semantically_ambiguous_duplicate_channels() {
        let weight = ChannelWeight::new(ResolutionChannelId::new(1), 500).unwrap();
        assert_eq!(
            ResolutionPolicy::new(1_000, 500, 0, vec![100], vec![weight, weight]),
            Err(ResolutionError::DuplicateChannel {
                channel: ResolutionChannelId::new(1)
            })
        );
    }
}
