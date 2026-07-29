//! Deterministic hydrology evolution.
//!
//! Geography owns what water is and where it sits; this module owns how it
//! moves. It holds no canonical state: `propose` reads one frozen
//! `HydrologyFieldSet` and returns a complete proposal, and the runtime decides
//! whether that proposal ever becomes authoritative.
//!
//! Every substage reads the state the previous one produced and writes a
//! complete delta. No cell observes another cell's same-substage write, which
//! is what makes the result independent of iteration order — and therefore what
//! makes a chunk seam behave like an interior face instead of like a direction
//! the solver happens to sweep.
//!
//! See `plans/hydrology.md` §5-§8.

mod evolution;
mod parameters;
mod proposal;
mod receipts;
mod records;

pub use evolution::{HydrologyEvolutionModel, allocate_largest_remainder};
pub use parameters::{HYDROLOGY_AGGREGATION_ARITY, HydrologyEvolutionLimits, process, substage};
pub use proposal::{
    HydrologyEventEffect, HydrologyEventKind, HydrologyEventPlan, HydrologyEvolutionProposal,
    HydrologyEvolutionRequest, HydrologyProperty, HydrologyProposalParts, HydrologyTerminalLeaf,
    absent_fingerprint, forcing_applied_fingerprint, forcing_settlement_fingerprint,
    volume_fingerprint,
};
pub use receipts::{
    HydrologyConservationParts, HydrologyConservationReceipt, HydrologyReceiptTotals,
    validate_boundary_transfers, validate_paired_transfers,
};
pub use records::{
    HydrologyBucket, HydrologyCellChange, HydrologyEdgeChange, HydrologyError,
    HydrologyForcingAllocation, HydrologyForcingSettlement, HydrologyTransferParts,
    HydrologyTransferReceipt,
};
