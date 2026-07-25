use causafera_types::{ChunkId, SimulationTime, TraceId};
use serde::{Deserialize, Serialize};

pub const OBSERVER_PROTOCOL_V1: u32 = 1;
pub const MAX_QUERY_PAYLOAD_BYTES: usize = 1 << 20;
pub const MATERIAL_SURFACE_DELTA_SCHEMA_V1: u32 = 1;
pub const MATERIAL_SURFACE_DELTA_SCHEMA_V2: u32 = 2;
pub const MATERIAL_SURFACE_DELTA_SCHEMA_V3: u32 = 3;
pub const MAX_MATERIAL_SURFACE_DELTAS: usize = 64;
pub const THERMAL_DELTA_SCHEMA_V1: u32 = 1;
pub const MAX_THERMAL_DELTAS: usize = 64;

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
    pub thermal_total_cell_energy: i128,
    pub thermal_total_reservoir_budget: i128,
    pub thermal_active_chunk_count: u32,
    pub thermal_active_cell_count: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObserverWorldSnapshot {
    pub time: SimulationTime,
    pub chunks: Vec<ObserverChunkSummary>,
    pub material_surface_delta_schema_version: u32,
    pub material_surface_deltas: Vec<MaterialSurfaceDelta>,
    pub material_surface_gate_deltas: Vec<MaterialSurfaceGateDelta>,
    pub thermal_delta_schema_version: u32,
    pub thermal_deltas: Vec<ThermalFieldDelta>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MaterialSurfaceDelta {
    pub chart_id: u64,
    pub chunk_x: i32,
    pub chunk_y: i32,
    pub chunk_z: i32,
    pub cell_ordinal: u16,
    pub before_condition: i64,
    pub after_condition: i64,
    pub mana_total: i64,
    pub contact_trace: Option<TraceId>,
    pub mana_effect_trace: Option<TraceId>,
    pub transition_tick: u64,
    pub mana_transition_trace: Option<TraceId>,
    pub mana_before: Option<i64>,
    pub mana_after: Option<i64>,
    pub local_mana_before: Option<i64>,
    pub local_mana_after: Option<i64>,
    pub local_mana_transition_trace_id: Option<TraceId>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MaterialSurfaceGateDelta {
    pub chart_id: u64,
    pub chunk_x: i32,
    pub chunk_y: i32,
    pub chunk_z: i32,
    pub cell_ordinal: u16,
    pub before_active: bool,
    pub after_active: bool,
    pub local_mana_before: i64,
    pub local_mana_after: i64,
    pub local_mana_transition_trace_id: TraceId,
    pub gate_transition_trace_id: TraceId,
    pub contact_trace_id: Option<TraceId>,
    pub transition_tick: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThermalFieldDelta {
    pub chart_id: u64,
    pub chunk_x: i32,
    pub chunk_y: i32,
    pub chunk_z: i32,
    pub cell_ordinal: u16,
    pub pre_state_energy: i64,
    pub post_state_energy: i64,
    pub reservoir_scheduled_injection: i64,
    pub reservoir_accepted_injection: i64,
    pub reservoir_rejected_injection: i64,
    pub net_face_flux: i64,
    pub face_count: u32,
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
