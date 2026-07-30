use causafera_types::{ChunkId, SimulationTime, TraceId};
use serde::{Deserialize, Serialize};

pub const OBSERVER_PROTOCOL_V1: u32 = 1;
pub const MAX_QUERY_PAYLOAD_BYTES: usize = 1 << 20;
pub const MATERIAL_SURFACE_DELTA_SCHEMA_V1: u32 = 1;
pub const MATERIAL_SURFACE_DELTA_SCHEMA_V2: u32 = 2;
pub const MATERIAL_SURFACE_DELTA_SCHEMA_V3: u32 = 3;
pub const MATERIAL_SURFACE_DELTA_SCHEMA_V4: u32 = 4;
pub const MAX_MATERIAL_SURFACE_DELTAS: usize = 64;
pub const THERMAL_DELTA_SCHEMA_V1: u32 = 1;
pub const MAX_THERMAL_DELTAS: usize = 64;
/// Schema of the bounded bootstrap summary carried by the runtime summary.
///
/// Zero is not a version: it is what a payload written before the summary
/// existed decodes to, and it means "no bootstrap evidence in this payload"
/// rather than "an empty bootstrap record".
pub const BOOTSTRAP_SUMMARY_SCHEMA_ABSENT: u32 = 0;
pub const BOOTSTRAP_SUMMARY_SCHEMA_V1: u32 = 1;
/// The current production bootstrap runs six stages, and the summary is capped
/// there rather than at a generous round number: a payload claiming more is not
/// a larger world, it is a record this build cannot have produced.
pub const MAX_BOOTSTRAP_RECEIPT_SUMMARIES: usize = 6;
/// One receipt names at most its declared dependency ancestry.
pub const MAX_BOOTSTRAP_RECEIPT_DEPENDENCIES: usize = 8;

/// The coarsest terrain detail level the projection offers.
///
/// Level 0 is the carrier's own 32 x 32 raster; each further level halves the
/// edge by block mean, so level 2 is 8 x 8. Nothing below that carries enough
/// samples to be worth a request.
pub const MAX_FIELD_RASTER_DETAIL_LEVEL: u8 = 2;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[repr(u32)]
pub enum QueryKind {
    RuntimeSummary = 1,
    ExplanationIr = 2,
    WorldChunks = 3,
    FieldRaster = 4,
}

impl TryFrom<u32> for QueryKind {
    type Error = ObserverApiError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::RuntimeSummary),
            2 => Ok(Self::ExplanationIr),
            3 => Ok(Self::WorldChunks),
            4 => Ok(Self::FieldRaster),
            value => Err(ObserverApiError::UnknownQueryKind(value)),
        }
    }
}

/// Which per-cell lattice a raster request asks for.
///
/// One query serves every spatial field because they are all lattices over one
/// chunk wanting identical bounding; a further field is an additive variant
/// rather than a further query kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[repr(u32)]
pub enum FieldRasterKind {
    /// Terrain elevation in millimetres, with roughness as the auxiliary band.
    TerrainElevation = 1,
    /// Terrain roughness in millimetres.
    TerrainRoughness = 2,
    /// Mana intensity over the field's own volumetric lattice.
    ManaIntensity = 3,
}

impl TryFrom<u32> for FieldRasterKind {
    type Error = ObserverApiError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::TerrainElevation),
            2 => Ok(Self::TerrainRoughness),
            3 => Ok(Self::ManaIntensity),
            value => Err(ObserverApiError::UnknownFieldRasterKind(value)),
        }
    }
}

/// A bounded request for one chunk of one field at one detail level.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FieldRasterRequest {
    pub chart_id: u64,
    pub chunk_x: i32,
    pub chunk_y: i32,
    pub chunk_z: i32,
    pub field: FieldRasterKind,
    /// Terrain only; the mana volume is projected whole at its configured extent.
    pub detail_level: u8,
}

impl FieldRasterRequest {
    pub fn validate(&self) -> Result<(), ObserverApiError> {
        if self.detail_level > MAX_FIELD_RASTER_DETAIL_LEVEL {
            return Err(ObserverApiError::InvalidDetailLevel(self.detail_level));
        }
        Ok(())
    }
}

/// One chunk of one measured lattice, transported unchanged.
///
/// `values` is row-major over `edge` columns and `edge` rows, repeated `depth`
/// times for a volumetric field — terrain is `depth` 1, mana is `depth` equal to
/// its extent. Reductions to plan view are a reading of the field rather than a
/// property of it, so the runtime performs none of them (INV-022).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObserverFieldRaster {
    pub chart_id: u64,
    pub chunk_x: i32,
    pub chunk_y: i32,
    pub chunk_z: i32,
    pub field: FieldRasterKind,
    pub detail_level: u8,
    /// Samples along one edge of the lattice.
    pub edge: u32,
    /// Layers through z. One for a surface field.
    pub depth: u32,
    pub values: Vec<i64>,
    /// A second band over the same lattice, empty when the field has none.
    pub auxiliary: Vec<i64>,
    /// Per-cell provenance: the trace that last changed the cell, zero for none.
    pub cell_traces: Vec<u64>,
    /// The event that produced the field, so a drawn cell has an anchor.
    pub generation_trace: u64,
}

impl ObserverFieldRaster {
    /// The lattice's cell count, or `None` when the declared dimensions cannot
    /// describe one.
    ///
    /// `edge` and `depth` arrive from the wire, so this multiplies attacker-
    /// chosen values: unchecked it panics in debug and wraps silently in
    /// release, on a decode path that has not yet applied any bound.
    pub fn cell_count(&self) -> Option<usize> {
        (self.edge as usize)
            .checked_mul(self.edge as usize)?
            .checked_mul(self.depth as usize)
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

    /// A raster request carries its parameters in the payload, so the query
    /// envelope stays the one shape every kind shares.
    pub fn field_raster(request_id: u64, payload: Vec<u8>) -> Self {
        Self {
            request_id,
            protocol_version: OBSERVER_PROTOCOL_V1,
            kind: QueryKind::FieldRaster,
            scope: None,
            payload,
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
    pub bootstrap: ObserverBootstrapSummary,
}

/// The bounded, read-only projection of the canonical production bootstrap
/// record.
///
/// It carries equality and trace anchors an observer needs to inspect that the
/// initial state was causally initialized, and nothing else: no runtime handles,
/// no authoritative actor or place identity, no stage targets, and no rendered
/// process names. `schema_version == BOOTSTRAP_SUMMARY_SCHEMA_ABSENT` means the
/// payload carried no bootstrap evidence at all.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObserverBootstrapSummary {
    pub schema_version: u32,
    /// Opaque content-addressed plan identity. An equality identity for replay
    /// inspection, never an ordering or a distance.
    pub plan_id: u64,
    pub world_seed: u64,
    pub stage_count: u32,
    /// Whether the record passed canonical validation at construction or import.
    pub complete: bool,
    /// The configured bounds the record's stage parameters were derived from,
    /// not live counts.
    pub configured_population: u64,
    pub configured_promotion_limit: u32,
    pub receipts: Vec<ObserverBootstrapReceipt>,
    /// The appended hydrology stage's receipt, when a session ran one.
    ///
    /// Carried separately rather than as a seventh entry in `receipts`: the
    /// six-summary cap, the projected `stage_count`, and `complete` are all frozen
    /// V1 contract, and a seventh entry would change what an existing consumer
    /// reads. An additive optional field is something a frozen decoder skips.
    pub stage_seven: Option<ObserverBootstrapReceipt>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObserverBootstrapReceipt {
    pub stage: u64,
    pub completed_at: SimulationTime,
    pub result: [u8; 32],
    pub completion_trace: TraceId,
    pub dependency_traces: Vec<TraceId>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObserverWorldSnapshot {
    pub time: SimulationTime,
    pub chunks: Vec<ObserverChunkSummary>,
    pub material_surface_delta_schema_version: u32,
    pub material_surface_deltas: Vec<MaterialSurfaceDelta>,
    pub material_surface_gate_deltas: Vec<MaterialSurfaceGateDelta>,
    pub material_surface_thermal_deltas: Vec<MaterialSurfaceThermalDelta>,
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

/// A material surface's retained-heat exchange with its co-located thermal cell
/// (`TODO-THERMAL-002`), a distinct addressed-object family from `MaterialSurfaceDelta`'s
/// condition/mana pair even though both are keyed by the same `MaterialSurfaceId`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MaterialSurfaceThermalDelta {
    pub chart_id: u64,
    pub chunk_x: i32,
    pub chunk_y: i32,
    pub chunk_z: i32,
    pub cell_ordinal: u16,
    pub before_retained: i64,
    pub after_retained: i64,
    pub cell_pre_state: i64,
    pub signed_flux: i64,
    pub thermal_exchange_trace_id: TraceId,
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
    #[error("unknown observer field raster kind {0}")]
    UnknownFieldRasterKind(u32),
    #[error("field raster detail level {0} is out of range")]
    InvalidDetailLevel(u8),
    #[error("observer payload is too large: {0} bytes")]
    PayloadTooLarge(usize),
}
