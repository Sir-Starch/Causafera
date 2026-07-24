use std::collections::{BTreeMap, VecDeque};

use causafera_types::{ChunkId, SimulationTime};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[repr(u32)]
pub enum StreamKind {
    RuntimeSummary = 1,
    Explanation = 2,
    Metrics = 3,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeliveryPolicy {
    ReliableOrdered,
    LatestStateWins,
    Coalesced,
    Sampled { every_ticks: u64 },
    RequestResponse,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct StreamScope {
    pub kind: StreamKind,
    pub chunk: Option<ChunkId>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StreamHeader {
    pub stream_id: u64,
    pub schema_version: u32,
    pub sequence_number: u64,
    pub simulation_time: SimulationTime,
    pub physical_digest: [u8; 32],
    pub history_digest: [u8; 32],
    pub is_snapshot: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StreamEnvelope {
    pub header: StreamHeader,
    pub scope: StreamScope,
    pub payload: Vec<u8>,
}

#[derive(Debug)]
struct StreamState {
    scope: StreamScope,
    policy: DeliveryPolicy,
    capacity: usize,
    next_sequence: u64,
    delivered_snapshot: bool,
    queue: VecDeque<StreamEnvelope>,
}

#[derive(Debug, Default)]
pub struct ObserverStreamHub {
    streams: BTreeMap<u64, StreamState>,
}

impl ObserverStreamHub {
    pub fn subscribe(
        &mut self,
        stream_id: u64,
        scope: StreamScope,
        policy: DeliveryPolicy,
        capacity: usize,
    ) -> Result<(), StreamError> {
        if capacity == 0 {
            return Err(StreamError::ZeroCapacity);
        }
        if matches!(policy, DeliveryPolicy::Sampled { every_ticks: 0 }) {
            return Err(StreamError::ZeroSampleInterval);
        }
        if self.streams.contains_key(&stream_id) {
            return Err(StreamError::DuplicateStream(stream_id));
        }
        self.streams.insert(
            stream_id,
            StreamState {
                scope,
                policy,
                capacity,
                next_sequence: 0,
                delivered_snapshot: false,
                queue: VecDeque::with_capacity(capacity),
            },
        );
        Ok(())
    }

    pub fn unsubscribe(&mut self, stream_id: u64) -> bool {
        self.streams.remove(&stream_id).is_some()
    }

    pub fn publish(
        &mut self,
        stream_id: u64,
        simulation_time: SimulationTime,
        physical_digest: [u8; 32],
        history_digest: [u8; 32],
        is_snapshot: bool,
        payload: Vec<u8>,
    ) -> Result<PublishOutcome, StreamError> {
        let state = self
            .streams
            .get_mut(&stream_id)
            .ok_or(StreamError::UnknownStream(stream_id))?;
        if !is_snapshot && !state.delivered_snapshot {
            return Err(StreamError::SnapshotRequired(stream_id));
        }
        if let DeliveryPolicy::Sampled { every_ticks } = state.policy
            && !is_snapshot
            && !simulation_time.raw().is_multiple_of(every_ticks)
        {
            return Ok(PublishOutcome::Dropped);
        }
        if state.policy == DeliveryPolicy::RequestResponse && !is_snapshot {
            return Err(StreamError::RequestResponseDoesNotStream(stream_id));
        }

        let envelope = StreamEnvelope {
            header: StreamHeader {
                stream_id,
                schema_version: 1,
                sequence_number: state.next_sequence,
                simulation_time,
                physical_digest,
                history_digest,
                is_snapshot,
            },
            scope: state.scope,
            payload,
        };

        let outcome = if state.queue.len() < state.capacity {
            state.queue.push_back(envelope);
            PublishOutcome::Queued
        } else {
            match state.policy {
                DeliveryPolicy::ReliableOrdered => {
                    return Err(StreamError::Backpressure(stream_id));
                }
                DeliveryPolicy::LatestStateWins => {
                    state.queue.clear();
                    state.queue.push_back(envelope);
                    PublishOutcome::Replaced
                }
                DeliveryPolicy::Coalesced => {
                    state.queue.pop_back();
                    state.queue.push_back(envelope);
                    PublishOutcome::Coalesced
                }
                DeliveryPolicy::Sampled { .. } => PublishOutcome::Dropped,
                DeliveryPolicy::RequestResponse => {
                    return Err(StreamError::Backpressure(stream_id));
                }
            }
        };
        if outcome != PublishOutcome::Dropped {
            state.next_sequence = state
                .next_sequence
                .checked_add(1)
                .ok_or(StreamError::SequenceExhausted(stream_id))?;
            state.delivered_snapshot |= is_snapshot;
        }
        Ok(outcome)
    }

    pub fn pop(&mut self, stream_id: u64) -> Result<Option<StreamEnvelope>, StreamError> {
        Ok(self
            .streams
            .get_mut(&stream_id)
            .ok_or(StreamError::UnknownStream(stream_id))?
            .queue
            .pop_front())
    }

    pub fn queued_len(&self, stream_id: u64) -> Result<usize, StreamError> {
        Ok(self
            .streams
            .get(&stream_id)
            .ok_or(StreamError::UnknownStream(stream_id))?
            .queue
            .len())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PublishOutcome {
    Queued,
    Replaced,
    Coalesced,
    Dropped,
}

#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum StreamError {
    #[error("observer stream capacity must be non-zero")]
    ZeroCapacity,
    #[error("sample interval must be non-zero")]
    ZeroSampleInterval,
    #[error("observer stream {0} already exists")]
    DuplicateStream(u64),
    #[error("observer stream {0} does not exist")]
    UnknownStream(u64),
    #[error("observer stream {0} requires a snapshot before deltas")]
    SnapshotRequired(u64),
    #[error("observer stream {0} is backpressured")]
    Backpressure(u64),
    #[error("request/response stream {0} cannot accept deltas")]
    RequestResponseDoesNotStream(u64),
    #[error("observer stream {0} exhausted its sequence space")]
    SequenceExhausted(u64),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn publish(hub: &mut ObserverStreamHub, id: u64, tick: u64, snapshot: bool) -> PublishOutcome {
        hub.publish(
            id,
            SimulationTime::new(tick),
            [1; 32],
            [2; 32],
            snapshot,
            vec![tick as u8],
        )
        .unwrap()
    }

    #[test]
    fn delta_requires_initial_snapshot() {
        let mut hub = ObserverStreamHub::default();
        hub.subscribe(
            1,
            StreamScope {
                kind: StreamKind::RuntimeSummary,
                chunk: None,
            },
            DeliveryPolicy::LatestStateWins,
            1,
        )
        .unwrap();
        assert_eq!(
            hub.publish(1, SimulationTime::new(1), [0; 32], [0; 32], false, vec![]),
            Err(StreamError::SnapshotRequired(1))
        );
    }

    #[test]
    fn latest_state_wins_is_bounded() {
        let mut hub = ObserverStreamHub::default();
        hub.subscribe(
            7,
            StreamScope {
                kind: StreamKind::RuntimeSummary,
                chunk: None,
            },
            DeliveryPolicy::LatestStateWins,
            1,
        )
        .unwrap();
        assert_eq!(publish(&mut hub, 7, 1, true), PublishOutcome::Queued);
        assert_eq!(publish(&mut hub, 7, 2, false), PublishOutcome::Replaced);
        assert_eq!(hub.queued_len(7), Ok(1));
        let message = hub.pop(7).unwrap().unwrap();
        assert_eq!(message.header.simulation_time, SimulationTime::new(2));
        assert_eq!(message.header.sequence_number, 1);
    }

    #[test]
    fn reliable_stream_signals_backpressure() {
        let mut hub = ObserverStreamHub::default();
        hub.subscribe(
            3,
            StreamScope {
                kind: StreamKind::Explanation,
                chunk: None,
            },
            DeliveryPolicy::ReliableOrdered,
            1,
        )
        .unwrap();
        publish(&mut hub, 3, 1, true);
        assert_eq!(
            hub.publish(3, SimulationTime::new(2), [0; 32], [0; 32], false, vec![]),
            Err(StreamError::Backpressure(3))
        );
    }
}
