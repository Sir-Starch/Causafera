use causafera_types::{ChartChunkCoord, ThermalEnergy, TraceId};
use thiserror::Error;

pub const THERMAL_SCALE: i64 = 1_024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ThermalParameters {
    pub transfer_fraction: i64,
    pub heat_capacity: i64,
    pub scale: i64,
    /// Fraction (out of `scale`) of the signed cell/material energy difference exchanged each tick.
    /// `0` disables material exchange for every surface: the flux formula computes
    /// `floor(magnitude * 0 / scale) == 0` unconditionally, so no special-cased "disabled" branch
    /// is needed anywhere the fraction is read.
    pub material_exchange_fraction: i64,
    /// Upper bound on a single material surface's retained energy, in the same fixed-point unit as
    /// `ThermalEnergy`. Independent of `ThermalEnergy::MAX`, which bounds a *cell*.
    pub material_thermal_capacity: i64,
}

impl ThermalParameters {
    pub const fn new(
        transfer_fraction: i64,
        heat_capacity: i64,
        scale: i64,
        material_exchange_fraction: i64,
        material_thermal_capacity: i64,
    ) -> Result<Self, ThermalError> {
        let parameters = Self {
            transfer_fraction,
            heat_capacity,
            scale,
            material_exchange_fraction,
            material_thermal_capacity,
        };
        parameters.validate()
    }

    pub const fn validate(self) -> Result<Self, ThermalError> {
        // `material_thermal_capacity` is `i64` and `ThermalEnergy::MAX == i64::MAX`, so an
        // upper-bound comparison against `ThermalEnergy::MAX` would never reject anything; only
        // non-positivity is a real constraint here.
        if self.scale <= 0
            || self.heat_capacity <= 0
            || self.transfer_fraction <= 0
            || self.material_exchange_fraction < 0
            || self.material_thermal_capacity <= 0
        {
            return Err(ThermalError::InvalidParameters);
        }
        let Some(six_faces) = self.transfer_fraction.checked_mul(6) else {
            return Err(ThermalError::InvalidParameters);
        };
        let Some(worst_case_outflow) = six_faces.checked_add(self.material_exchange_fraction)
        else {
            return Err(ThermalError::InvalidParameters);
        };
        if worst_case_outflow > self.scale {
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

/// A material surface's thermal state, keyed by its co-located `ThermalCellKey`. The domain layer
/// is deliberately ignorant of `MaterialSurfaceId`/contact history; the runtime maps its own
/// surfaces onto this shape.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ThermalMaterialSite {
    pub retained_before: ThermalEnergy,
    /// The surface's prior thermal-exchange trace, if any. `None` before its first non-zero
    /// exchange. Threaded through so a cell-change event can cite it as a parent.
    pub last_exchange: Option<TraceId>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ThermalMaterialTransferRecord {
    pub retained_before: ThermalEnergy,
    pub retained_after: ThermalEnergy,
    /// Positive: flowed cell -> material. Negative: flowed material -> cell. Same sign convention
    /// as `ThermalFaceRecord::signed_flux`.
    pub signed_flux: i64,
    /// Non-zero only when heating was capped by `material_thermal_capacity`; the rejected amount
    /// stays in the cell rather than being destroyed.
    pub rejected: ThermalEnergy,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ThermalCellChange {
    pub cell: ThermalCellKey,
    pub before: ThermalEnergy,
    pub after: ThermalEnergy,
    pub parent_traces: Vec<TraceId>,
    pub incident_faces: Vec<ThermalFaceRecord>,
    pub reservoirs: Vec<ThermalReservoirTransferRecord>,
    pub material: Option<ThermalMaterialTransferRecord>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ThermalCellTransferReceipt {
    pub cell: ThermalCellKey,
    pub pre_state: ThermalEnergy,
    pub post_state: ThermalEnergy,
    pub cell_change_trace_id: Option<TraceId>,
    pub faces: Vec<ThermalFaceRecord>,
    pub reservoirs: Vec<ThermalReservoirTransferRecord>,
    pub material: Option<ThermalMaterialTransferRecord>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ThermalConservationReceipt {
    pub tick: u64,
    pub total_cell_energy_before: i128,
    pub total_cell_energy_after: i128,
    pub total_reservoir_budget_before: i128,
    pub total_reservoir_budget_after: i128,
    pub total_material_retained_before: i128,
    pub total_material_retained_after: i128,
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
