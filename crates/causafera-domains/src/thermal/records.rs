use causafera_types::{ChartChunkCoord, ThermalEnergy, TraceId};
use thiserror::Error;

pub const THERMAL_SCALE: i64 = 1_024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ThermalParameters {
    pub transfer_fraction: i64,
    pub heat_capacity: i64,
    pub scale: i64,
}

impl ThermalParameters {
    pub const fn new(
        transfer_fraction: i64,
        heat_capacity: i64,
        scale: i64,
    ) -> Result<Self, ThermalError> {
        let parameters = Self {
            transfer_fraction,
            heat_capacity,
            scale,
        };
        parameters.validate()
    }

    pub const fn validate(self) -> Result<Self, ThermalError> {
        if self.scale <= 0
            || self.heat_capacity <= 0
            || self.transfer_fraction <= 0
            || self.transfer_fraction > self.scale / 6
        {
            Err(ThermalError::InvalidParameters)
        } else {
            Ok(self)
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ThermalReservoirId(u64);

impl ThermalReservoirId {
    pub const fn new(raw: u64) -> Self {
        Self(raw)
    }

    pub const fn raw(self) -> u64 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ThermalCellKey {
    pub chunk: ChartChunkCoord,
    pub cell_index: u16,
}

impl ThermalCellKey {
    pub const fn new(chunk: ChartChunkCoord, cell_index: u16) -> Self {
        Self { chunk, cell_index }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThermalReservoirSchedule {
    PerTick(ThermalEnergy),
    OneShot,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ThermalReservoir {
    pub id: ThermalReservoirId,
    pub target: ThermalCellKey,
    pub budget: ThermalEnergy,
    pub schedule: ThermalReservoirSchedule,
    pub bootstrap_trace: TraceId,
    /// The most recent pre-existing trace for this reservoir: bootstrap_trace
    /// before the first transfer, then the previous reservoir-transfer event.
    pub last_change: TraceId,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ThermalInjectionProposal {
    pub reservoir_id: ThermalReservoirId,
    pub target: ThermalCellKey,
    pub scheduled_amount: ThermalEnergy,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ThermalBoundaryRecord {
    pub cell: ThermalCellKey,
    pub neighbor: ThermalCellKey,
    pub cell_pre_state: ThermalEnergy,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThermalBoundaryBehavior {
    NoFluxOutsideActiveRegion,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ThermalFaceRecord {
    pub neighbor: ThermalCellKey,
    /// Positive flux leaves the receipt's cell; negative flux enters it.
    pub signed_flux: i64,
    pub neighbor_pre_state: ThermalEnergy,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ThermalReservoirTransferRecord {
    pub id: ThermalReservoirId,
    pub scheduled_injection: ThermalEnergy,
    pub accepted_injection: ThermalEnergy,
    pub rejected_injection: ThermalEnergy,
    pub transfer_trace_id: Option<TraceId>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ThermalCellChange {
    pub cell: ThermalCellKey,
    pub before: ThermalEnergy,
    pub after: ThermalEnergy,
    pub parent_traces: Vec<TraceId>,
    pub incident_faces: Vec<ThermalFaceRecord>,
    pub reservoirs: Vec<ThermalReservoirTransferRecord>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ThermalCellTransferReceipt {
    pub cell: ThermalCellKey,
    pub pre_state: ThermalEnergy,
    pub post_state: ThermalEnergy,
    pub cell_change_trace_id: Option<TraceId>,
    pub faces: Vec<ThermalFaceRecord>,
    pub reservoirs: Vec<ThermalReservoirTransferRecord>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ThermalConservationReceipt {
    pub tick: u64,
    pub total_cell_energy_before: i128,
    pub total_cell_energy_after: i128,
    pub total_reservoir_budget_before: i128,
    pub total_reservoir_budget_after: i128,
    pub residual: i128,
}

#[derive(Clone, Copy, Debug, Error, PartialEq, Eq)]
pub enum ThermalError {
    #[error("thermal parameters are invalid")]
    InvalidParameters,
    #[error("thermal field extent is invalid")]
    InvalidExtent,
    #[error("thermal field energy length is invalid")]
    InvalidEnergyLength,
    #[error("thermal field set is invalid")]
    InvalidFieldSet,
    #[error("thermal field set contains a duplicate chunk")]
    DuplicateFieldChunk,
    #[error("thermal active region contains a non-active resident chunk")]
    InvalidActiveRegion,
    #[error("thermal field {0} lies outside the active region")]
    FieldOutsideActiveRegion(ChartChunkCoord),
    #[error("thermal active region is incomplete at chunk {0}")]
    ActiveRegionIncomplete(ChartChunkCoord),
    #[error("thermal field extents are incompatible across a face")]
    IncompatibleFieldExtent,
    #[error("thermal reservoir ID is duplicated")]
    DuplicateReservoir,
    #[error("thermal injection proposal is duplicated")]
    DuplicateInjectionProposal,
    #[error("thermal injection refers to an unknown reservoir")]
    UnknownReservoir,
    #[error("thermal injection target does not match its reservoir")]
    InjectionTargetMismatch,
    #[error("thermal cell key is outside its field")]
    PositionOutsideField,
    #[error("thermal arithmetic overflowed its checked intermediate")]
    ArithmeticOverflow,
    #[error("thermal preflight produced an out-of-range cell value")]
    EnergyOutOfBounds,
    #[error("thermal conservation residual is non-zero: {0}")]
    ConservationViolation(i128),
}
