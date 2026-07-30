use causafera_core::{CausalEventProposalError, CausalEventProposalKey, StateFingerprint};
use causafera_geography::{
    HydrologyCarrierKey, HydrologyCellKey, HydrologyEdgeKey, HydrologyStateError,
};
use causafera_types::{TraceId, WaterVolume, WaterVolumeError};

/// Which storage a receipt or effect names.
///
/// The tag bytes are the plan's aggregation-tree bucket tags and are part of
/// the fingerprint contract, not an implementation detail.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum HydrologyBucket {
    Surface,
    Soil,
    Groundwater,
    ForcingInput,
    Conveyance,
    Resolution,
    ForcingRecord,
    CoarseProcess,
    /// Outside the modelled world: the atmosphere an ET sink leaves into, or an
    /// open boundary's far side. Never a storage the ledger has to close over —
    /// it is the ledger's `sinks` and `sources` terms.
    External,
}

impl HydrologyBucket {
    pub const fn tag(self) -> u8 {
        match self {
            Self::Surface => 0x01,
            Self::Soil => 0x02,
            Self::Groundwater => 0x03,
            Self::ForcingInput => 0x04,
            Self::Conveyance => 0x05,
            Self::Resolution => 0x06,
            Self::ForcingRecord => 0x07,
            Self::CoarseProcess => 0x08,
            Self::External => 0x09,
        }
    }
}

/// One accepted, partly accepted, or wholly rejected movement of water.
///
/// `requested`, `accepted`, and `unaccepted` are all carried because a limiter
/// is evidence. Recording only what moved would make a bound that engaged
/// indistinguishable from a process that had nothing to do — and a clamp that
/// silently swallowed water indistinguishable from both.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HydrologyTransferReceipt {
    batch_sequence: u64,
    tick: u64,
    process_kind: u32,
    source: HydrologyCarrierKey,
    source_bucket: HydrologyBucket,
    target: HydrologyCarrierKey,
    target_bucket: HydrologyBucket,
    requested: WaterVolume,
    accepted: WaterVolume,
    unaccepted: WaterVolume,
    source_before: WaterVolume,
    source_after: WaterVolume,
    target_before: WaterVolume,
    target_after: WaterVolume,
    causal_parents: Vec<TraceId>,
    forcing_origin: Option<TraceId>,
    /// The batch-local proposal key of the event that carried this transfer.
    /// Resolved to a committed trace by the runtime after the DAG commits.
    transfer_event: Option<CausalEventProposalKey>,
    /// The batch-local proposal key of the event that settled the storage this
    /// transfer changed, when that is a different event.
    storage_event: Option<CausalEventProposalKey>,
}

/// Complete constructor input; fifteen fields of mostly the same types.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HydrologyTransferParts {
    pub batch_sequence: u64,
    pub tick: u64,
    pub process_kind: u32,
    pub source: HydrologyCarrierKey,
    pub source_bucket: HydrologyBucket,
    pub target: HydrologyCarrierKey,
    pub target_bucket: HydrologyBucket,
    pub requested: WaterVolume,
    pub accepted: WaterVolume,
    pub source_before: WaterVolume,
    pub source_after: WaterVolume,
    pub target_before: WaterVolume,
    pub target_after: WaterVolume,
    pub causal_parents: Vec<TraceId>,
    pub forcing_origin: Option<TraceId>,
    pub transfer_event: Option<CausalEventProposalKey>,
    pub storage_event: Option<CausalEventProposalKey>,
}

impl HydrologyTransferReceipt {
    pub fn new(parts: HydrologyTransferParts) -> Result<Self, HydrologyError> {
        // Accepting more than was requested is not a generous process, it is a
        // source of water that no ledger term accounts for.
        let unaccepted = parts
            .requested
            .checked_sub(parts.accepted)
            .map_err(|_| HydrologyError::AcceptedExceedsRequested)?;
        Ok(Self {
            batch_sequence: parts.batch_sequence,
            tick: parts.tick,
            process_kind: parts.process_kind,
            source: parts.source,
            source_bucket: parts.source_bucket,
            target: parts.target,
            target_bucket: parts.target_bucket,
            requested: parts.requested,
            accepted: parts.accepted,
            unaccepted,
            source_before: parts.source_before,
            source_after: parts.source_after,
            target_before: parts.target_before,
            target_after: parts.target_after,
            causal_parents: parts.causal_parents,
            forcing_origin: parts.forcing_origin,
            transfer_event: parts.transfer_event,
            storage_event: parts.storage_event,
        })
    }

    pub const fn batch_sequence(&self) -> u64 {
        self.batch_sequence
    }

    pub const fn tick(&self) -> u64 {
        self.tick
    }

    pub const fn process_kind(&self) -> u32 {
        self.process_kind
    }

    pub const fn source(&self) -> HydrologyCarrierKey {
        self.source
    }

    pub const fn source_bucket(&self) -> HydrologyBucket {
        self.source_bucket
    }

    pub const fn target(&self) -> HydrologyCarrierKey {
        self.target
    }

    pub const fn target_bucket(&self) -> HydrologyBucket {
        self.target_bucket
    }

    pub const fn requested(&self) -> WaterVolume {
        self.requested
    }

    pub const fn accepted(&self) -> WaterVolume {
        self.accepted
    }

    pub const fn unaccepted(&self) -> WaterVolume {
        self.unaccepted
    }

    pub const fn source_before(&self) -> WaterVolume {
        self.source_before
    }

    pub const fn source_after(&self) -> WaterVolume {
        self.source_after
    }

    pub const fn target_before(&self) -> WaterVolume {
        self.target_before
    }

    pub const fn target_after(&self) -> WaterVolume {
        self.target_after
    }

    pub fn causal_parents(&self) -> &[TraceId] {
        &self.causal_parents
    }

    pub const fn forcing_origin(&self) -> Option<TraceId> {
        self.forcing_origin
    }

    pub fn transfer_event(&self) -> Option<&CausalEventProposalKey> {
        self.transfer_event.as_ref()
    }

    pub fn storage_event(&self) -> Option<&CausalEventProposalKey> {
        self.storage_event.as_ref()
    }

    /// Whether the source lost exactly what the target gained.
    ///
    /// False for a source (nothing was withdrawn from a real bucket) and for a
    /// sink (nothing was deposited into one); those terms close through the
    /// ledger's `sources` and `sinks` instead.
    pub fn is_internal_transfer(&self) -> bool {
        self.source_bucket != HydrologyBucket::External
            && self.target_bucket != HydrologyBucket::External
    }

    /// The unique canonical identity of this receipt within a tick.
    pub fn canonical_key(&self) -> (u64, u32, Vec<u8>, Vec<u8>) {
        (
            self.tick,
            self.process_kind,
            self.source.encode(),
            self.target.encode(),
        )
    }
}

/// One bucket's committed change, used to install trace anchors after commit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HydrologyCellChange {
    pub cell: HydrologyCellKey,
    pub bucket: HydrologyBucket,
    pub before: WaterVolume,
    pub after: WaterVolume,
    pub settlement_event: CausalEventProposalKey,
}

/// One conveyance edge's committed change.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HydrologyEdgeChange {
    pub edge: HydrologyEdgeKey,
    pub before: WaterVolume,
    pub after: WaterVolume,
    pub settlement_event: CausalEventProposalKey,
}

/// One cell's durable record of the forcing it was handed this tick.
///
/// Written whenever a scheduled record targets the cell, including when every
/// allocation came out zero or was wholly rejected. "The record targeted this
/// cell and nothing fit" is evidence; dropping it would make a rejected input
/// indistinguishable from an absent one after the typed receipts are evicted.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HydrologyForcingSettlement {
    pub cell: HydrologyCellKey,
    /// Per-record allocations in canonical `(scheduled_tick, forcing_id)` order.
    pub allocations: Vec<HydrologyForcingAllocation>,
    pub accepted_source: WaterVolume,
    pub rejected_source: WaterVolume,
    pub accepted_et: WaterVolume,
    pub unmet_et: WaterVolume,
    pub fingerprint_before: StateFingerprint,
    pub fingerprint_after: StateFingerprint,
    pub settlement_event: CausalEventProposalKey,
}

/// One record's share of one cell's forcing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HydrologyForcingAllocation {
    pub scheduled_tick: u64,
    pub forcing_id: u64,
    pub origin: TraceId,
    pub precipitation: WaterVolume,
    pub external_inflow: WaterVolume,
    pub potential_et: WaterVolume,
    pub accepted_source: WaterVolume,
    pub accepted_et: WaterVolume,
}

/// Every way hydrology evolution can refuse to produce a proposal.
#[derive(Clone, Debug, thiserror::Error, PartialEq, Eq)]
pub enum HydrologyError {
    #[error("hydrology state is invalid: {0}")]
    State(#[from] HydrologyStateError),
    #[error("hydrology water arithmetic failed: {0}")]
    Arithmetic(#[from] WaterVolumeError),
    #[error("a hydrology proposal key could not be built: {0}")]
    ProposalKey(#[from] CausalEventProposalError),
    #[error("a hydrology transfer accepted more than it requested")]
    AcceptedExceedsRequested,
    #[error("a hydrology forcing record targets cell that is not resident")]
    ForcingTargetNotResident,
    #[error("a hydrology forcing record scheduled for tick {scheduled} reached tick {tick}")]
    ForcingTickMismatch { scheduled: u64, tick: u64 },
    #[error("hydrology proposed {count} transfers this tick, at most {max} are allowed")]
    TransferLimitExceeded { count: usize, max: usize },
    #[error("a hydrology allocation had a positive total but no eligible member")]
    UnallocatableTotal,
    #[error("a hydrology allocation total exceeded the sum of its member ceilings")]
    AllocationExceedsCeilings,
    #[error("the hydrology conservation residual is {residual}, not zero")]
    ConservationResidual { residual: i128 },
    #[error("a hydrology receipt's arithmetic does not match the state it describes")]
    ReceiptInconsistent,
    #[error("hydrology synthetic aggregation node identifiers are exhausted")]
    NodeIdentifiersExhausted,
    /// Surface head is terrain elevation plus ponded depth, so a resident chunk
    /// with no terrain has no head — and a solver that defaulted the elevation
    /// to zero would invent a flat world and route water across it.
    #[error("a resident hydrology chunk has no terrain elevation")]
    TerrainMissing,
    #[error("a terrain chunk is filed under the wrong hydrology chunk address")]
    TerrainChunkMismatch,
    /// A face with no resident neighbour and no boundary record. Exporting and
    /// blocking are both physical claims, and neither may be assumed (V13).
    #[error("an exterior hydrology face has no boundary record")]
    UnspecifiedBoundaryFace,
    /// Saturated depth divides by `cell_area * specific_yield_num`, so a cell
    /// that stores groundwater without a specific yield has no water table.
    #[error("a hydrology cell stores groundwater without a specific yield")]
    GroundwaterWithoutSpecificYield,
    #[error("the hydrology request's resident chunks are not the field set's")]
    ResidencyMismatch,
    #[error("a hydrology event cites {count} causes, at most {max} are allowed")]
    EventCauseLimitExceeded { count: usize, max: usize },
    #[error("a hydrology event carries {count} effects, at most {max} are allowed")]
    EventEffectLimitExceeded { count: usize, max: usize },
}

#[cfg(test)]
mod tests {
    use super::*;
    use causafera_geography::HydrologyCellKey;
    use causafera_types::{ChartChunkCoord, ChunkCoord, SpatialChartId};

    fn cell(ordinal: u16) -> HydrologyCellKey {
        HydrologyCellKey::new(
            ChartChunkCoord::new(SpatialChartId::new(1), ChunkCoord::new(0, 0, 0)),
            ordinal,
        )
        .unwrap()
    }

    fn parts(requested: u64, accepted: u64) -> HydrologyTransferParts {
        HydrologyTransferParts {
            batch_sequence: 3,
            tick: 7,
            process_kind: super::super::process::INFILTRATION,
            source: HydrologyCarrierKey::Cell(cell(0)),
            source_bucket: HydrologyBucket::Surface,
            target: HydrologyCarrierKey::Cell(cell(0)),
            target_bucket: HydrologyBucket::Soil,
            requested: WaterVolume::new(requested),
            accepted: WaterVolume::new(accepted),
            source_before: WaterVolume::new(100),
            source_after: WaterVolume::new(100 - accepted),
            target_before: WaterVolume::new(0),
            target_after: WaterVolume::new(accepted),
            causal_parents: vec![TraceId::new(1)],
            forcing_origin: None,
            transfer_event: None,
            storage_event: None,
        }
    }

    #[test]
    fn a_receipt_derives_its_own_unaccepted_remainder() {
        let receipt = HydrologyTransferReceipt::new(parts(40, 25)).unwrap();
        assert_eq!(receipt.requested(), WaterVolume::new(40));
        assert_eq!(receipt.accepted(), WaterVolume::new(25));
        assert_eq!(receipt.unaccepted(), WaterVolume::new(15));
        assert!(receipt.is_internal_transfer());
        assert_eq!(receipt.batch_sequence(), 3);
        assert_eq!(receipt.tick(), 7);
    }

    #[test]
    fn accepting_more_than_was_requested_is_rejected() {
        // Not a generous process: a source of water no ledger term accounts for.
        assert_eq!(
            HydrologyTransferReceipt::new(parts(10, 11)),
            Err(HydrologyError::AcceptedExceedsRequested)
        );
    }

    #[test]
    fn a_receipt_touching_the_outside_world_is_not_an_internal_transfer() {
        let mut sink = parts(10, 10);
        sink.target_bucket = HydrologyBucket::External;
        assert!(
            !HydrologyTransferReceipt::new(sink)
                .unwrap()
                .is_internal_transfer()
        );

        let mut source = parts(10, 10);
        source.source_bucket = HydrologyBucket::External;
        assert!(
            !HydrologyTransferReceipt::new(source)
                .unwrap()
                .is_internal_transfer()
        );
    }

    #[test]
    fn bucket_tags_are_the_aggregation_tree_contract() {
        let tags = [
            HydrologyBucket::Surface,
            HydrologyBucket::Soil,
            HydrologyBucket::Groundwater,
            HydrologyBucket::ForcingInput,
            HydrologyBucket::Conveyance,
            HydrologyBucket::Resolution,
            HydrologyBucket::ForcingRecord,
            HydrologyBucket::CoarseProcess,
            HydrologyBucket::External,
        ]
        .map(HydrologyBucket::tag);
        assert_eq!(tags, [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09]);
        let mut unique = tags.to_vec();
        unique.sort_unstable();
        unique.dedup();
        assert_eq!(unique.len(), tags.len(), "bucket tags must not collide");
    }

    #[test]
    fn a_receipts_canonical_key_separates_processes_and_endpoints() {
        let a = HydrologyTransferReceipt::new(parts(1, 1)).unwrap();
        let mut other_process = parts(1, 1);
        other_process.process_kind = super::super::process::PERCOLATION;
        let b = HydrologyTransferReceipt::new(other_process).unwrap();
        let mut other_target = parts(1, 1);
        other_target.target = HydrologyCarrierKey::Cell(cell(1));
        let c = HydrologyTransferReceipt::new(other_target).unwrap();

        assert_ne!(a.canonical_key(), b.canonical_key());
        assert_ne!(a.canonical_key(), c.canonical_key());
    }
}
