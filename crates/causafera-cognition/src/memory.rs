use causafera_types::{EpisodeId, PerceptId, SimulationTime, WorkingItemId, WorkingItemKindId};
use thiserror::Error;

use crate::{AppearanceSignature, CognitiveWeight};

pub const MAX_WORKING_ITEMS: usize = 8;
pub const MAX_WORKING_CANDIDATES: usize = 32;
pub const MAX_EPISODES: usize = 64;
pub const MAX_REACTIVATED_EPISODES: usize = 4;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WorkingCandidate {
    pub id: WorkingItemId,
    pub kind: WorkingItemKindId,
    pub activation: CognitiveWeight,
    pub supporting_percept: PerceptId,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WorkingItem {
    pub id: WorkingItemId,
    pub kind: WorkingItemKindId,
    pub activation: CognitiveWeight,
    pub supporting_percept: PerceptId,
    pub last_rehearsed: SimulationTime,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkingContext {
    items: [WorkingItem; MAX_WORKING_ITEMS],
    len: u8,
    capacity: u8,
    decay_per_tick: u32,
    last_update: Option<SimulationTime>,
}

impl WorkingContext {
    pub fn new(capacity: u8, decay_per_tick: u32) -> Result<Self, MemoryError> {
        if capacity == 0 || usize::from(capacity) > MAX_WORKING_ITEMS {
            return Err(MemoryError::WorkingCapacity { capacity });
        }
        Ok(Self {
            items: [WorkingItem::default(); MAX_WORKING_ITEMS],
            len: 0,
            capacity,
            decay_per_tick,
            last_update: None,
        })
    }

    pub fn items(&self) -> &[WorkingItem] {
        &self.items[..self.len as usize]
    }

    pub fn update(
        &mut self,
        time: SimulationTime,
        candidates: &[WorkingCandidate],
    ) -> Result<(), MemoryError> {
        if self.last_update.is_some_and(|last| time < last) {
            return Err(MemoryError::TimeRegression);
        }
        if candidates.len() > MAX_WORKING_CANDIDATES {
            return Err(MemoryError::TooManyWorkingCandidates);
        }
        let elapsed = self
            .last_update
            .map_or(0, |last| time.raw().saturating_sub(last.raw()));
        let decay = u64::from(self.decay_per_tick)
            .saturating_mul(elapsed)
            .min(u64::from(u32::MAX)) as u32;

        let mut combined: Vec<WorkingItem> = self
            .items()
            .iter()
            .copied()
            .filter_map(|mut item| {
                let remaining = item.activation.raw().saturating_sub(decay);
                (remaining > 0).then(|| {
                    item.activation =
                        CognitiveWeight::new(remaining).expect("decay cannot increase weight");
                    item
                })
            })
            .collect();
        let mut candidates = candidates.to_vec();
        candidates.sort_by_key(|candidate| candidate.id);
        if candidates.windows(2).any(|pair| pair[0].id == pair[1].id) {
            return Err(MemoryError::DuplicateWorkingItem);
        }
        for candidate in candidates {
            if let Some(item) = combined.iter_mut().find(|item| item.id == candidate.id) {
                if candidate.activation > item.activation {
                    item.activation = candidate.activation;
                    item.kind = candidate.kind;
                    item.supporting_percept = candidate.supporting_percept;
                }
                item.last_rehearsed = time;
            } else {
                combined.push(WorkingItem {
                    id: candidate.id,
                    kind: candidate.kind,
                    activation: candidate.activation,
                    supporting_percept: candidate.supporting_percept,
                    last_rehearsed: time,
                });
            }
        }
        combined.sort_by(|a, b| {
            b.activation
                .cmp(&a.activation)
                .then_with(|| a.id.cmp(&b.id))
        });
        combined.truncate(usize::from(self.capacity));
        self.len = combined.len() as u8;
        self.items[..combined.len()].copy_from_slice(&combined);
        self.last_update = Some(time);
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct EpisodicMemory {
    pub id: EpisodeId,
    pub signature: AppearanceSignature,
    pub encoded_at: SimulationTime,
    pub strength: CognitiveWeight,
    pub supporting_percept: PerceptId,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ReactivatedEpisode {
    pub id: EpisodeId,
    pub activation: CognitiveWeight,
    pub supporting_percept: PerceptId,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EpisodicStore {
    episodes: [EpisodicMemory; MAX_EPISODES],
    len: u8,
}

impl EpisodicStore {
    pub const fn new() -> Self {
        Self {
            episodes: [EpisodicMemory {
                id: EpisodeId::new(0),
                signature: AppearanceSignature([0; 4]),
                encoded_at: SimulationTime::new(0),
                strength: CognitiveWeight::ZERO,
                supporting_percept: PerceptId::new(0),
            }; MAX_EPISODES],
            len: 0,
        }
    }

    pub fn encode(&mut self, episode: EpisodicMemory) {
        if let Some(existing) = self.episodes[..self.len as usize]
            .iter_mut()
            .find(|existing| existing.id == episode.id)
        {
            *existing = episode;
            return;
        }
        let index = if self.len as usize == MAX_EPISODES {
            self.episodes[..self.len as usize]
                .iter()
                .enumerate()
                .min_by_key(|(_, episode)| (episode.strength, episode.encoded_at, episode.id))
                .map(|(index, _)| index)
                .expect("episode capacity is non-zero")
        } else {
            let index = self.len as usize;
            self.len += 1;
            index
        };
        self.episodes[index] = episode;
    }

    pub fn reactivate(
        &self,
        cue: AppearanceSignature,
        relevance: CognitiveWeight,
    ) -> [Option<ReactivatedEpisode>; MAX_REACTIVATED_EPISODES] {
        let mut ranked: Vec<_> = self.episodes[..self.len as usize]
            .iter()
            .map(|episode| {
                let distance: u32 = episode
                    .signature
                    .0
                    .into_iter()
                    .zip(cue.0)
                    .map(|(a, b)| u32::from(a.abs_diff(b)))
                    .sum();
                let similarity = 1_000_000u32.saturating_sub(distance.saturating_mul(250));
                let activation = u64::from(similarity)
                    .saturating_mul(u64::from(episode.strength.raw()))
                    .saturating_mul(u64::from(relevance.raw()))
                    / 1_000_000u64
                    / 1_000_000u64;
                ReactivatedEpisode {
                    id: episode.id,
                    activation: CognitiveWeight::new(activation as u32)
                        .expect("bounded fixed-point product"),
                    supporting_percept: episode.supporting_percept,
                }
            })
            .filter(|episode| episode.activation.raw() > 0)
            .collect();
        ranked.sort_by(|a, b| {
            b.activation
                .cmp(&a.activation)
                .then_with(|| a.id.cmp(&b.id))
        });
        let mut output = [None; MAX_REACTIVATED_EPISODES];
        for (slot, episode) in output.iter_mut().zip(ranked) {
            *slot = Some(episode);
        }
        output
    }
}

impl Default for EpisodicStore {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Copy, Debug, Error, PartialEq, Eq)]
pub enum MemoryError {
    #[error("working capacity {capacity} must be between 1 and {MAX_WORKING_ITEMS}")]
    WorkingCapacity { capacity: u8 },
    #[error("working context time regressed")]
    TimeRegression,
    #[error("too many working-context candidates")]
    TooManyWorkingCandidates,
    #[error("working item identifiers must be unique")]
    DuplicateWorkingItem,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn weight(value: u32) -> CognitiveWeight {
        CognitiveWeight::new(value).unwrap()
    }

    #[test]
    fn working_context_is_bounded_and_items_decay_without_rehearsal() {
        let mut context = WorkingContext::new(2, 100).unwrap();
        let candidates = [
            WorkingCandidate {
                id: WorkingItemId::new(3),
                kind: WorkingItemKindId::new(1),
                activation: weight(500),
                supporting_percept: PerceptId::new(3),
            },
            WorkingCandidate {
                id: WorkingItemId::new(1),
                kind: WorkingItemKindId::new(1),
                activation: weight(900),
                supporting_percept: PerceptId::new(1),
            },
            WorkingCandidate {
                id: WorkingItemId::new(2),
                kind: WorkingItemKindId::new(1),
                activation: weight(700),
                supporting_percept: PerceptId::new(2),
            },
        ];
        context.update(SimulationTime::new(1), &candidates).unwrap();
        assert_eq!(
            context
                .items()
                .iter()
                .map(|item| item.id)
                .collect::<Vec<_>>(),
            vec![WorkingItemId::new(1), WorkingItemId::new(2)]
        );
        context.update(SimulationTime::new(3), &[]).unwrap();
        assert_eq!(context.items()[0].activation, weight(700));
    }

    #[test]
    fn episodic_reactivation_is_partial_ranked_and_similarity_based() {
        let mut store = EpisodicStore::new();
        store.encode(EpisodicMemory {
            id: EpisodeId::new(2),
            signature: AppearanceSignature([100; 4]),
            encoded_at: SimulationTime::new(1),
            strength: weight(800_000),
            supporting_percept: PerceptId::new(2),
        });
        store.encode(EpisodicMemory {
            id: EpisodeId::new(1),
            signature: AppearanceSignature([12; 4]),
            encoded_at: SimulationTime::new(1),
            strength: weight(800_000),
            supporting_percept: PerceptId::new(1),
        });
        let active = store.reactivate(AppearanceSignature([10; 4]), weight(1_000_000));
        assert_eq!(active[0].unwrap().id, EpisodeId::new(1));
        assert!(active.iter().flatten().count() <= MAX_REACTIVATED_EPISODES);
    }
}
