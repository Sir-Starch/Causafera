//! Canonical hydrology state and physical input schemas.
//!
//! Geography owns what water *is* and where it sits; `causafera-domains` owns
//! how it moves and `causafera-runtime` owns when that is committed. The split
//! follows the existing crate dependency direction — `causafera-domains`
//! already depends on this crate, so canonical state could not live there
//! without a cycle.
//!
//! Nothing in this module is a semantic water body. There is no `River`,
//! `Lake`, `Wetland`, `Flood`, or `Watershed`: those are classifications an
//! observer may compute over measurable storage, conductance, geometry, and
//! history, and they are never authoritative simulation meaning.
//!
//! See `plans/hydrology.md` and `docs/rfc/RFC-HYDRO-001.md`.

mod forcing;
mod metric;
mod state;
mod substrate;

pub use forcing::{
    BOOTSTRAP_HYDROLOGY_FORCING_POLICY_V1, HydrologyForcingMember, HydrologyForcingParts,
    HydrologyForcingRecord, HydrologyForcingSchedule,
};
pub use metric::{HydrologyDepthSplit, HydrologyGridMetric, HydrologyGridMetrics};
pub use state::{
    FaceDirection, FluxBoundary, HYDROLOGY_CARRIER_KEY_VERSION, HydrologyActiveRegion,
    HydrologyBoundaryCondition, HydrologyBoundaryMap, HydrologyCarrierKey, HydrologyCellKey,
    HydrologyCellState, HydrologyCellStorage, HydrologyConveyanceEdge, HydrologyConveyanceGraph,
    HydrologyEdgeKey, HydrologyExteriorFaceKey, HydrologyField, HydrologyFieldSet,
    HydrologyResolutionState, MAX_HYDROLOGY_RESOLUTION_LEVEL,
};
pub use substrate::{
    HYDRAULIC_SUBSTRATE_KEY_LEN, HydraulicFraction, HydraulicSubstrateCell, HydraulicSubstrateKey,
    HydraulicSubstrateParts,
};

use causafera_types::CHUNK_SIZE;

/// Hydrology cells in one chunk.
///
/// The hydrology lattice is the terrain surface lattice: `CHUNK_SIZE²`
/// cells addressed row-major as `y * CHUNK_SIZE + x`. Runtime `chunk_extent`
/// sizes the mana volume and has no bearing here — a chunk is an addressing
/// and computation unit, not a metric length (INV-036, INV-037, INV-043).
pub const SURFACE_CELL_COUNT: usize = CHUNK_SIZE as usize * CHUNK_SIZE as usize;

// ---------------------------------------------------------------------------
// Hard allocation bounds
//
// Every one of these is checked *before* the allocation it governs, so a
// malformed count in a decoded snapshot cannot reserve memory on the way to
// being rejected. They also compose: the per-record and total forcing-member
// caps are both enforced, and the chunk, cell, and edge caps are consistent
// with each other at `SURFACE_CELL_COUNT` cells per chunk.
// ---------------------------------------------------------------------------

pub const MAX_HYDROLOGY_CHARTS: usize = 64;
pub const MAX_HYDROLOGY_CHUNKS: usize = 128;
pub const MAX_HYDROLOGY_CELLS: usize = 131_072;
pub const MAX_HYDROLOGY_EDGES: usize = 262_144;
pub const MAX_HYDROLOGY_FORCING_RECORDS: usize = 8_192;
pub const MAX_HYDROLOGY_TARGETS_PER_FORCING: usize = 4_096;
pub const MAX_HYDROLOGY_TOTAL_FORCING_MEMBERS: usize = 262_144;
pub const MAX_HYDROLOGY_FORCING_ORIGINS_PER_TICK: usize = 8;
pub const MAX_HYDROLOGY_FORCING_ORIGINS_PER_CELL_PER_TICK: usize = 6;
pub const MAX_HYDROLOGY_FORCING_HORIZON_TICKS: u64 = 1_000_000;
pub const MAX_HYDROLOGY_TRANSFERS_PER_TICK: usize = 262_144;
pub const MAX_HYDROLOGY_STORED_RECEIPT_BATCHES: usize = 8;
pub const MAX_HYDROLOGY_PERSISTED_TRANSFER_RECEIPTS: usize = 262_144;
pub const MAX_HYDROLOGY_CHART_OVERRIDES: usize = 64;
pub const MAX_HYDROLOGY_CELL_OVERRIDES: usize = 131_072;
pub const MAX_HYDROLOGY_BOUNDARY_RECORDS: usize = 524_288;
pub const MAX_HYDROLOGY_CAUSES_PER_EVENT: usize = 16;
pub const MAX_HYDROLOGY_EFFECTS_PER_EVENT: usize = 8;
pub const MAX_HYDROLOGY_SECTION_BYTES: usize = 201_326_592;

/// Every way canonical hydrology state can fail to be constructed or decoded.
///
/// One enum across the four submodules on purpose: these values are validated
/// together, an import path has to report them together, and splitting them
/// would mean a conversion layer whose only job is to lose which check failed.
#[derive(Clone, Copy, Debug, thiserror::Error, PartialEq, Eq)]
pub enum HydrologyStateError {
    #[error("hydrology water arithmetic failed: {0}")]
    Arithmetic(#[from] causafera_types::WaterVolumeError),

    // Metric
    #[error("hydrology grid metric schema {0} is not supported")]
    UnsupportedMetricSchema(u16),
    #[error("hydrology grid metrics are empty")]
    EmptyMetrics,
    #[error("hydrology grid metrics cover {count} charts, at most {max} are allowed")]
    ChartCountExceeded { count: usize, max: usize },
    #[error("hydrology grid metrics declare the same chart more than once")]
    DuplicateMetricChart,
    #[error("no hydrology grid metric is registered for the requested chart")]
    UnknownMetricChart,

    // Substrate
    #[error("hydraulic fraction {numerator}/{denominator} is not within [0, 1]")]
    FractionOutOfRange { numerator: u32, denominator: u32 },
    #[error("initial storage exceeds its declared capacity")]
    StorageExceedsCapacity,
    #[error("specific yield must be positive when groundwater storage is enabled")]
    ZeroSpecificYield,

    // Lattice and field
    #[error("hydrology cell ordinal {ordinal} is outside the {count}-cell surface lattice")]
    CellOrdinalOutOfRange { ordinal: u16, count: usize },
    #[error("a hydrology field must contain exactly {expected} cells, not {actual}")]
    InvalidFieldLength { expected: usize, actual: usize },
    #[error("hydrology field set is empty")]
    EmptyFieldSet,
    #[error("hydrology field set declares the same chunk more than once")]
    DuplicateFieldChunk,
    #[error("hydrology field set covers {count} chunks, at most {max} are allowed")]
    ChunkCountExceeded { count: usize, max: usize },
    #[error("hydrology field set covers {count} cells, at most {max} are allowed")]
    CellCountExceeded { count: usize, max: usize },
    #[error("hydrology chunk lies in a chart with no registered grid metric")]
    FieldChartWithoutMetric,

    // Conveyance
    #[error("a conveyance edge cannot join a cell to itself")]
    DegenerateEdge,
    #[error("conveyance edge endpoints are not orthogonally adjacent within one chart")]
    NonAdjacentEdge,
    #[error("a conveyance edge's outlet must be one of its own endpoints")]
    OutletNotAnEndpoint,
    #[error("the same canonical cell face carries more than one conveyance edge")]
    DuplicateEdgeFace,
    #[error("a cell has more than one outgoing conveyance edge")]
    MultipleOutgoingEdges,
    #[error("hydrology declares {count} conveyance edges, at most {max} are allowed")]
    EdgeCountExceeded { count: usize, max: usize },

    // Boundaries
    #[error("the same exterior face carries more than one boundary condition")]
    DuplicateBoundaryFace,
    #[error("hydrology declares {count} boundary records, at most {max} are allowed")]
    BoundaryCountExceeded { count: usize, max: usize },

    // Resolution and residency
    #[error("hydrology resolution level {level} exceeds the maximum of {max}")]
    ResolutionLevelExceeded { level: u8, max: u8 },
    #[error("an active hydrology chunk is not resident")]
    ActiveChunkNotResident,

    // Forcing
    #[error("a hydrology forcing record must target at least one cell")]
    EmptyForcingTargets,
    #[error("a hydrology forcing record targets the same cell more than once")]
    DuplicateForcingMember,
    #[error("hydrology forcing members are not in canonical cell-key order")]
    UnorderedForcingMembers,
    #[error("a hydrology forcing record targets {count} cells, at most {max} are allowed")]
    ForcingTargetCountExceeded { count: usize, max: usize },
    #[error("the hydrology forcing schedule declares {count} records, at most {max} are allowed")]
    ForcingRecordCountExceeded { count: usize, max: usize },
    #[error("the hydrology forcing schedule declares {count} members, at most {max} are allowed")]
    ForcingMemberTotalExceeded { count: usize, max: usize },
    #[error("the hydrology forcing schedule repeats the key (tick {tick}, id {id})")]
    DuplicateForcingKey { tick: u64, id: u64 },
    #[error("the hydrology forcing schedule is not in canonical (tick, id) order")]
    UnorderedForcingSchedule,
    #[error("hydrology forcing producer policy schema {0} is not registered")]
    UnknownForcingPolicy(u64),
    #[error("an applied hydrology forcing record was not applied at its scheduled tick")]
    ForcingAppliedOffSchedule,
    #[error("tick {tick} carries {count} distinct forcing origins, at most {max} are allowed")]
    ForcingOriginsPerTickExceeded { tick: u64, count: usize, max: usize },
    #[error(
        "a cell receives {count} distinct forcing origins in one tick, at most {max} are allowed"
    )]
    ForcingOriginsPerCellExceeded { count: usize, max: usize },

    // Carrier keys
    #[error("hydrology carrier key version {0} is not supported")]
    UnsupportedCarrierKeyVersion(u8),
    #[error("hydrology carrier key variant {0:#04x} is not recognised")]
    UnknownCarrierKeyVariant(u8),
    #[error("hydrology carrier key has {actual} bytes, variant {variant:#04x} requires {expected}")]
    InvalidCarrierKeyLength {
        variant: u8,
        expected: usize,
        actual: usize,
    },
    #[error("hydrology carrier key face direction {0} is not recognised")]
    UnknownFaceDirection(u8),
    #[error("a hydrology edge carrier key is not in canonical endpoint order")]
    NoncanonicalCarrierKeyOrder,
}
