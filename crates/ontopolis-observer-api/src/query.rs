use ontopolis_types::{ChunkId, SimulationTime, TraceId};
use serde::{Deserialize, Serialize};

pub const OBSERVER_PROTOCOL_V1: u32 = 1;
pub const MAX_QUERY_PAYLOAD_BYTES: usize = 1 << 20;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[repr(u32)]
pub enum QueryKind {
    RuntimeSummary = 1,
    ExplanationIr = 2,
    WorldChunks = 3,
}

impl TryFrom<u32> for QueryKind {
    type Error = ObserverApiError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::RuntimeSummary),
            2 => Ok(Self::ExplanationIr),
            3 => Ok(Self::WorldChunks),
            value => Err(ObserverApiError::UnknownQueryKind(value)),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObserverQuery {
    pub request_id: u64,
    pub protocol_version: u32,
    pub kind: QueryKind,
    pub scope: Option<ChunkId>,
    pub payload: Vec<u8>,
}

impl ObserverQuery {
    pub fn runtime_summary(request_id: u64) -> Self {
        Self {
            request_id,
            protocol_version: OBSERVER_PROTOCOL_V1,
            kind: QueryKind::RuntimeSummary,
            scope: None,
            payload: Vec::new(),
        }
    }

    pub fn world_chunks(request_id: u64) -> Self {
        Self {
            request_id,
            protocol_version: OBSERVER_PROTOCOL_V1,
            kind: QueryKind::WorldChunks,
            scope: None,
            payload: Vec::new(),
        }
    }

    pub fn validate(&self) -> Result<(), ObserverApiError> {
        if self.protocol_version != OBSERVER_PROTOCOL_V1 {
            return Err(ObserverApiError::UnsupportedProtocolVersion(
                self.protocol_version,
            ));
        }
        if self.payload.len() > MAX_QUERY_PAYLOAD_BYTES {
            return Err(ObserverApiError::PayloadTooLarge(self.payload.len()));
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u32)]
pub enum QueryStatus {
    Ok = 1,
    InvalidRequest = 2,
    Unsupported = 3,
    NotAvailable = 4,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObserverResponse {
    pub request_id: u64,
    pub protocol_version: u32,
    pub status: QueryStatus,
    pub payload: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObserverSnapshot {
    pub time: SimulationTime,
    pub digest_schema_version: u32,
    pub physical_digest: [u8; 32],
    pub history_digest: [u8; 32],
    pub mana_total: i64,
    pub mana_maximum: i64,
    pub active_chunk_count: u32,
    pub resolution_relevance: i64,
    pub resolution_level: u32,
    pub causal_trace_count: u64,
    pub actor_count: u32,
    pub population_total: u64,
    pub physical_events: u64,
    pub mana_cell_changes: u64,
    pub mana_physical_effects: u64,
    pub resolution_transitions: u64,
    pub actor_actions_committed: u64,
    pub actor_actions_rejected: u64,
    pub population_births: u64,
    pub population_deaths: u64,
    pub population_movements: u64,
    pub bytes_per_chunk: u64,
    pub latest_trace: TraceId,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObserverWorldSnapshot {
    pub time: SimulationTime,
    pub chunks: Vec<ObserverChunkSummary>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObserverChunkSummary {
    pub chart_id: u64,
    pub chunk_x: i32,
    pub chunk_y: i32,
    pub chunk_z: i32,
    pub minimum_elevation_mm: i32,
    pub maximum_elevation_mm: i32,
    pub mean_roughness_mm: u32,
    pub mana_total: i64,
    pub resolution_relevance: i64,
    pub resolution_level: u32,
    pub population_total: u64,
    pub causal_event_count: u64,
    pub latest_trace: TraceId,
}

#[derive(Clone, Debug, thiserror::Error, PartialEq, Eq)]
pub enum ObserverApiError {
    #[error("unsupported observer protocol version {0}")]
    UnsupportedProtocolVersion(u32),
    #[error("unknown observer query kind {0}")]
    UnknownQueryKind(u32),
    #[error("observer payload is too large: {0} bytes")]
    PayloadTooLarge(usize),
}
