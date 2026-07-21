use causafera_types::{CHUNK_SIZE, ChartChunkCoord, SimulationTime, TraceId};

pub const MAX_MATERIAL_SURFACE_TRANSITIONS: usize = 128;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MaterialSurfaceId {
    pub chunk: ChartChunkCoord,
    pub cell_index: u16,
}

impl MaterialSurfaceId {
    pub const fn new(chunk: ChartChunkCoord, cell_index: u16) -> Self {
        Self { chunk, cell_index }
    }

    pub const fn is_within_extent(self, extent: u8) -> bool {
        let side = extent as u16;
        self.cell_index < side.saturating_mul(side).saturating_mul(side)
    }

    pub const fn has_valid_cell_ordinal(self) -> bool {
        self.is_within_extent(CHUNK_SIZE)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MaterialSurface {
    pub condition: i64,
    pub contact_count: u64,
    pub last_transition: TraceId,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MaterialSurfaceRecordSnapshot {
    pub id: MaterialSurfaceId,
    pub surface: MaterialSurface,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MaterialSurfaceSnapshot {
    pub records: Vec<MaterialSurfaceRecordSnapshot>,
    pub pending_physical_changes: Vec<MaterialSurfaceId>,
    pub transitions: Vec<MaterialSurfaceTransition>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MaterialSurfaceTransition {
    pub id: MaterialSurfaceId,
    pub occurred_at: SimulationTime,
    pub before_condition: i64,
    pub after_condition: i64,
    pub mana_total: i64,
    pub contact_trace: Option<TraceId>,
    pub mana_effect_trace: Option<TraceId>,
    pub transition_trace: TraceId,
}
