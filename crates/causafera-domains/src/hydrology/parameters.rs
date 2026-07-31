//! Opaque numeric identities and structural limits for hydrology evolution.
//!
//! Every process is a number, never a human-language string. A receipt that
//! said `"infiltration"` would be a developer analytical label sitting inside
//! authoritative state; the observer may render one, the simulation may not
//! carry one.

/// Substage ordinals from the plan's deterministic process order.
///
/// The ordinal is a *validation boundary*: a local causal edge may only point
/// at the same substage or an earlier one, because each substage reads the
/// state the previous one produced. It is not the commit order — that is the
/// canonical byte order of the whole proposal key.
pub mod substage {
    pub const FORCING: u8 = 1;
    pub const INFILTRATION: u8 = 2;
    pub const PERCOLATION: u8 = 3;
    pub const EVAPOTRANSPIRATION: u8 = 4;
    pub const SURFACE_ROUTING: u8 = 5;
    pub const GROUNDWATER_ROUTING: u8 = 6;
    pub const CONVEYANCE_ROUTING: u8 = 7;
    pub const BOUNDARY_EXPORT: u8 = 8;
    pub const CONSERVATION: u8 = 9;
    /// Not a Physics substage: a representation change is committed in
    /// `Phase::Resolution`, in a batch of its own.
    pub const REPRESENTATION: u8 = 0;
}

/// Opaque process identities carried by transfer receipts and proposal keys.
pub mod process {
    pub const FORCING_SETTLEMENT: u32 = 0;
    pub const PRECIPITATION: u32 = 1;
    pub const EXTERNAL_INFLOW: u32 = 2;
    pub const INFILTRATION: u32 = 3;
    pub const PERCOLATION: u32 = 4;
    pub const EVAPOTRANSPIRATION_SURFACE: u32 = 5;
    pub const EVAPOTRANSPIRATION_SOIL: u32 = 6;
    pub const SURFACE_LATERAL: u32 = 7;
    pub const GROUNDWATER_LATERAL: u32 = 8;
    pub const BASEFLOW: u32 = 9;
    pub const CONVEYANCE_INFLOW: u32 = 10;
    pub const CONVEYANCE_RELEASE: u32 = 11;
    pub const SURFACE_BOUNDARY_EXPORT: u32 = 12;
    pub const GROUNDWATER_BOUNDARY_EXPORT: u32 = 13;
    pub const CONSERVATION: u32 = 14;
    pub const BATCH_AGGREGATE: u32 = 15;
    pub const COARSE_INPUT_LEAF: u32 = 16;
    pub const COARSE_INPUT_AGGREGATE: u32 = 17;
    pub const COARSE_PROCESS: u32 = 18;
    /// A scheduled record transitioning to applied. Distinct from
    /// `FORCING_SETTLEMENT`: one names a record becoming spent, the other names
    /// a cell folding every record that reached it. Sharing an identity would
    /// make two different processes indistinguishable in a receipt.
    pub const FORCING_APPLICATION: u32 = 19;
    /// The evapotranspiration *event*, as distinct from its two receipts. One
    /// event settles whichever of the two buckets it actually drew from, so
    /// naming it after either bucket would misreport the half that did not move.
    pub const EVAPOTRANSPIRATION: u32 = 20;
    /// One cell's surface-routing settlement *event*. Its net change may come
    /// from lateral outflow, lateral inflow, conveyance inflow, and boundary
    /// export at once, so borrowing any one of those process identities would
    /// misreport the others — the same reason `EVAPOTRANSPIRATION` exists.
    pub const SURFACE_ROUTING: u32 = 21;
    /// One cell's groundwater-routing settlement event, for the same reason.
    pub const GROUNDWATER_ROUTING: u32 = 22;
    /// One conveyance receiver's settlement event, which folds every release
    /// allocation that reached it this tick.
    pub const CONVEYANCE_SETTLEMENT: u32 = 23;
    /// One chunk's detail level changing. Committed in `Phase::Resolution`, not
    /// in a Physics substage, so its proposal key carries substage ordinal zero.
    pub const REPRESENTATION: u32 = 24;
}

/// Structural caps one hydrology batch is validated against.
///
/// These mirror the plan's hard bounds and are passed to the trace store rather
/// than baked into it, so one domain's event shapes stay one domain's contract.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HydrologyEvolutionLimits {
    pub max_causes_per_event: usize,
    pub max_effects_per_event: usize,
    pub max_transfers_per_tick: usize,
}

impl Default for HydrologyEvolutionLimits {
    fn default() -> Self {
        Self {
            max_causes_per_event: causafera_geography::MAX_HYDROLOGY_CAUSES_PER_EVENT,
            max_effects_per_event: causafera_geography::MAX_HYDROLOGY_EFFECTS_PER_EVENT,
            max_transfers_per_tick: causafera_geography::MAX_HYDROLOGY_TRANSFERS_PER_TICK,
        }
    }
}

/// Arity of the canonical terminal and coarse-input aggregation trees.
pub const HYDROLOGY_AGGREGATION_ARITY: usize = 16;
