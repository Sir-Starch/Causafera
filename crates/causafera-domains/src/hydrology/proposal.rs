use std::collections::BTreeMap;

use causafera_core::{CausalEventDagCause, CausalEventProposalKey, StateFingerprint};
use causafera_geography::{
    HydrologyActiveRegion, HydrologyBoundaryMap, HydrologyCarrierKey, HydrologyCellKey,
    HydrologyConveyanceGraph, HydrologyFieldSet, HydrologyForcingRecord, HydrologyGridMetrics,
    TerrainChunk,
};
use causafera_types::{ChartChunkCoord, TraceId, WaterVolume};

use super::{
    HydrologyBucket, HydrologyCellChange, HydrologyConservationReceipt, HydrologyEdgeChange,
    HydrologyEvolutionLimits, HydrologyForcingAllocation, HydrologyForcingSettlement,
    HydrologyTransferReceipt,
};

/// Everything one hydrology Physics execution reads.
///
/// Borrowed rather than owned: the solver never mutates any of it. Every
/// substage reads one frozen state and produces a complete delta, so nothing
/// here can change part-way through a tick.
#[derive(Clone, Copy, Debug)]
pub struct HydrologyEvolutionRequest<'a> {
    pub tick: u64,
    pub metrics: &'a HydrologyGridMetrics,
    /// The authoritative terrain of every resident hydrology chunk.
    ///
    /// Borrowed from geography rather than copied into hydrology state: terrain
    /// elevation already exists as a carrier, and a second persisted copy could
    /// drift from it while both claimed to be the ground water flows over.
    pub terrain: &'a BTreeMap<ChartChunkCoord, TerrainChunk>,
    pub active: &'a HydrologyActiveRegion,
    pub conveyance: &'a HydrologyConveyanceGraph,
    pub boundaries: &'a HydrologyBoundaryMap,
    /// Records scheduled for this tick that have not been applied, in canonical
    /// `(scheduled_tick, forcing_id)` order.
    pub forcing: &'a [HydrologyForcingRecord],
    pub previous_conservation: TraceId,
    pub limits: HydrologyEvolutionLimits,
}

/// Which authoritative property one causal effect transitions.
///
/// A domain-side name, not a schema identifier. The runtime maps these onto its
/// allocated `StatePropertyId`s; putting the numbers here would make the
/// runtime's causal schema a domain concern.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum HydrologyProperty {
    Surface,
    Soil,
    Groundwater,
    Conveyance,
    ForcingInput,
    ForcingRecord,
    Resolution,
    BatchSequence,
    BatchRoot,
    CoarseInput,
}

impl HydrologyProperty {
    /// The bucket this property stores, for receipt and aggregation tagging.
    pub const fn bucket(self) -> HydrologyBucket {
        match self {
            Self::Surface => HydrologyBucket::Surface,
            Self::Soil => HydrologyBucket::Soil,
            Self::Groundwater => HydrologyBucket::Groundwater,
            Self::Conveyance => HydrologyBucket::Conveyance,
            Self::ForcingInput => HydrologyBucket::ForcingInput,
            Self::ForcingRecord => HydrologyBucket::ForcingRecord,
            Self::Resolution => HydrologyBucket::Resolution,
            Self::BatchSequence | Self::BatchRoot | Self::CoarseInput => {
                HydrologyBucket::CoarseProcess
            }
        }
    }
}

/// One property transition, already fingerprinted.
///
/// The payloads being hashed are domain data — a water volume, an ordered list
/// of forcing allocations — so the domain hashes them. The runtime maps the
/// carrier and property onto a `CausalTarget` and carries these fingerprints
/// through unchanged.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HydrologyEventEffect {
    pub carrier: HydrologyCarrierKey,
    pub property: HydrologyProperty,
    pub before: StateFingerprint,
    pub after: StateFingerprint,
}

/// Which family of hydrology event this is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum HydrologyEventKind {
    /// A scheduled record transitioning to applied, citing its one origin.
    ForcingApplication,
    /// One cell's fold of every record targeting it this tick.
    ForcingSettlement,
    CellChange,
    EdgeTransfer,
    Representation,
}

/// One logical event, with causes that may name siblings in the same batch.
///
/// The runtime turns this into a `CausalEventDagProposal`: it supplies the
/// event kind and object identities, and the trace store supplies atomicity and
/// the canonical commit order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HydrologyEventPlan {
    pub key: CausalEventProposalKey,
    pub kind: HydrologyEventKind,
    pub causes: Vec<CausalEventDagCause>,
    pub effects: Vec<HydrologyEventEffect>,
}

/// One terminal `(carrier, bucket)` membership of the batch aggregation tree.
///
/// Sorted by `(carrier bytes, bucket tag, proposal key bytes)`. One settlement
/// event can be terminal for several buckets of the same cell, so a single
/// event may appear here more than once — each conserved carrier keeps its own
/// leaf record even though the tree deduplicates repeated causes.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct HydrologyTerminalLeaf {
    pub carrier_bytes: Vec<u8>,
    pub bucket_tag: u8,
    pub event: CausalEventProposalKey,
}

/// The complete result of one hydrology Physics execution, before commit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HydrologyEvolutionProposal {
    tick: u64,
    batch_sequence: u64,
    after_state: HydrologyFieldSet,
    after_conveyance: HydrologyConveyanceGraph,
    applied_forcing: Vec<(u64, u64)>,
    forcing_settlements: Vec<HydrologyForcingSettlement>,
    cell_changes: Vec<HydrologyCellChange>,
    edge_changes: Vec<HydrologyEdgeChange>,
    transfer_receipts: Vec<HydrologyTransferReceipt>,
    conservation: HydrologyConservationReceipt,
    events: Vec<HydrologyEventPlan>,
    terminal_leaves: Vec<HydrologyTerminalLeaf>,
}

/// Complete constructor input for a proposal.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HydrologyProposalParts {
    pub tick: u64,
    pub batch_sequence: u64,
    pub after_state: HydrologyFieldSet,
    pub after_conveyance: HydrologyConveyanceGraph,
    pub applied_forcing: Vec<(u64, u64)>,
    pub forcing_settlements: Vec<HydrologyForcingSettlement>,
    pub cell_changes: Vec<HydrologyCellChange>,
    pub edge_changes: Vec<HydrologyEdgeChange>,
    pub transfer_receipts: Vec<HydrologyTransferReceipt>,
    pub conservation: HydrologyConservationReceipt,
    pub events: Vec<HydrologyEventPlan>,
    pub terminal_leaves: Vec<HydrologyTerminalLeaf>,
}

impl HydrologyEvolutionProposal {
    pub(crate) fn new(parts: HydrologyProposalParts) -> Self {
        Self {
            tick: parts.tick,
            batch_sequence: parts.batch_sequence,
            after_state: parts.after_state,
            after_conveyance: parts.after_conveyance,
            applied_forcing: parts.applied_forcing,
            forcing_settlements: parts.forcing_settlements,
            cell_changes: parts.cell_changes,
            edge_changes: parts.edge_changes,
            transfer_receipts: parts.transfer_receipts,
            conservation: parts.conservation,
            events: parts.events,
            terminal_leaves: parts.terminal_leaves,
        }
    }

    pub const fn tick(&self) -> u64 {
        self.tick
    }

    pub const fn batch_sequence(&self) -> u64 {
        self.batch_sequence
    }

    pub const fn after_state(&self) -> &HydrologyFieldSet {
        &self.after_state
    }

    pub const fn after_conveyance(&self) -> &HydrologyConveyanceGraph {
        &self.after_conveyance
    }

    pub fn applied_forcing(&self) -> &[(u64, u64)] {
        &self.applied_forcing
    }

    pub fn forcing_settlements(&self) -> &[HydrologyForcingSettlement] {
        &self.forcing_settlements
    }

    pub fn cell_changes(&self) -> &[HydrologyCellChange] {
        &self.cell_changes
    }

    pub fn edge_changes(&self) -> &[HydrologyEdgeChange] {
        &self.edge_changes
    }

    pub fn transfer_receipts(&self) -> &[HydrologyTransferReceipt] {
        &self.transfer_receipts
    }

    pub const fn conservation(&self) -> &HydrologyConservationReceipt {
        &self.conservation
    }

    pub fn events(&self) -> &[HydrologyEventPlan] {
        &self.events
    }

    pub fn terminal_leaves(&self) -> &[HydrologyTerminalLeaf] {
        &self.terminal_leaves
    }
}

// ---------------------------------------------------------------------------
// Canonical fingerprints
// ---------------------------------------------------------------------------

const DOMAIN_VOLUME: &[u8] = b"causafera.hydrology.volume.v1";
const DOMAIN_ABSENT: &[u8] = b"causafera.hydrology.absent.v1";
const DOMAIN_FORCING_INPUT: &[u8] = b"causafera.hydrology.forcing-input.v1";
const DOMAIN_FORCING_RECORD: &[u8] = b"causafera.hydrology.forcing-record.v1";

fn write_prefixed(hasher: &mut blake3::Hasher, bytes: &[u8]) {
    hasher.update(&(bytes.len() as u64).to_be_bytes());
    hasher.update(bytes);
}

/// One storage bucket's canonical fingerprint.
///
/// The carrier and property are hashed alongside the value, so two cells
/// holding the same volume do not share a fingerprint. Without that, a causal
/// effect could be replayed against the wrong object and still verify.
pub fn volume_fingerprint(
    carrier: &HydrologyCarrierKey,
    property: HydrologyProperty,
    value: WaterVolume,
) -> StateFingerprint {
    let mut hasher = blake3::Hasher::new();
    write_prefixed(&mut hasher, DOMAIN_VOLUME);
    hasher.update(&[property.bucket().tag()]);
    write_prefixed(&mut hasher, &carrier.encode());
    hasher.update(&value.get().to_be_bytes());
    StateFingerprint::new(*hasher.finalize().as_bytes())
}

/// The fingerprint of a property that does not exist yet.
///
/// Distinct per carrier and property for the same reason volumes are, and
/// distinct from any volume fingerprint because the domain string differs — so
/// "absent" can never collide with "holds zero", which are different claims.
pub fn absent_fingerprint(
    carrier: &HydrologyCarrierKey,
    property: HydrologyProperty,
) -> StateFingerprint {
    let mut hasher = blake3::Hasher::new();
    write_prefixed(&mut hasher, DOMAIN_ABSENT);
    hasher.update(&[property.bucket().tag()]);
    write_prefixed(&mut hasher, &carrier.encode());
    StateFingerprint::new(*hasher.finalize().as_bytes())
}

/// One cell's durable forcing-input fingerprint for a tick.
///
/// Covers every record's allocation in canonical order, so the receipts stay
/// attributable to their origins without each origin having to become a
/// separate cause. Written even when everything allocated to zero.
pub fn forcing_settlement_fingerprint(
    tick: u64,
    cell: HydrologyCellKey,
    allocations: &[HydrologyForcingAllocation],
) -> StateFingerprint {
    let mut hasher = blake3::Hasher::new();
    write_prefixed(&mut hasher, DOMAIN_FORCING_INPUT);
    hasher.update(&tick.to_be_bytes());
    write_prefixed(&mut hasher, &HydrologyCarrierKey::Cell(cell).encode());
    hasher.update(&(allocations.len() as u64).to_be_bytes());
    for allocation in allocations {
        hasher.update(&allocation.scheduled_tick.to_be_bytes());
        hasher.update(&allocation.forcing_id.to_be_bytes());
        hasher.update(&allocation.origin.raw().to_be_bytes());
        hasher.update(&allocation.precipitation.get().to_be_bytes());
        hasher.update(&allocation.external_inflow.get().to_be_bytes());
        hasher.update(&allocation.potential_et.get().to_be_bytes());
        hasher.update(&allocation.accepted_source.get().to_be_bytes());
        hasher.update(&allocation.accepted_et.get().to_be_bytes());
    }
    StateFingerprint::new(*hasher.finalize().as_bytes())
}

/// A forcing record's applied state.
pub fn forcing_applied_fingerprint(
    scheduled_tick: u64,
    forcing_id: u64,
    applied_at: Option<u64>,
) -> StateFingerprint {
    let mut hasher = blake3::Hasher::new();
    write_prefixed(&mut hasher, DOMAIN_FORCING_RECORD);
    hasher.update(&scheduled_tick.to_be_bytes());
    hasher.update(&forcing_id.to_be_bytes());
    match applied_at {
        Some(tick) => {
            hasher.update(&[1]);
            hasher.update(&tick.to_be_bytes());
        }
        None => {
            hasher.update(&[0]);
        }
    }
    StateFingerprint::new(*hasher.finalize().as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;
    use causafera_types::{ChartChunkCoord, ChunkCoord, SpatialChartId};

    fn cell(ordinal: u16) -> HydrologyCellKey {
        HydrologyCellKey::new(
            ChartChunkCoord::new(SpatialChartId::new(1), ChunkCoord::new(0, 0, 0)),
            ordinal,
        )
        .unwrap()
    }

    #[test]
    fn a_volume_fingerprint_separates_carrier_property_and_value() {
        let a = HydrologyCarrierKey::Cell(cell(0));
        let b = HydrologyCarrierKey::Cell(cell(1));
        let base = volume_fingerprint(&a, HydrologyProperty::Surface, WaterVolume::new(5));

        assert_eq!(
            base,
            volume_fingerprint(&a, HydrologyProperty::Surface, WaterVolume::new(5)),
            "the same claim fingerprints the same"
        );
        assert_ne!(
            base,
            volume_fingerprint(&b, HydrologyProperty::Surface, WaterVolume::new(5)),
            "another cell holding five is a different claim"
        );
        assert_ne!(
            base,
            volume_fingerprint(&a, HydrologyProperty::Soil, WaterVolume::new(5)),
            "another bucket holding five is a different claim"
        );
        assert_ne!(
            base,
            volume_fingerprint(&a, HydrologyProperty::Surface, WaterVolume::new(6))
        );
    }

    #[test]
    fn absent_never_collides_with_holding_zero() {
        // "This property does not exist yet" and "this property holds nothing"
        // are different claims, and a bootstrap effect transitions between them.
        let carrier = HydrologyCarrierKey::Cell(cell(0));
        assert_ne!(
            absent_fingerprint(&carrier, HydrologyProperty::Surface),
            volume_fingerprint(&carrier, HydrologyProperty::Surface, WaterVolume::ZERO)
        );
        assert_ne!(
            absent_fingerprint(&carrier, HydrologyProperty::Surface),
            absent_fingerprint(&carrier, HydrologyProperty::Soil)
        );
    }

    #[test]
    fn a_forcing_settlement_fingerprint_covers_every_allocation_in_order() {
        let allocation = |id: u64, precipitation: u64| HydrologyForcingAllocation {
            scheduled_tick: 5,
            forcing_id: id,
            origin: TraceId::new(9),
            precipitation: WaterVolume::new(precipitation),
            external_inflow: WaterVolume::ZERO,
            potential_et: WaterVolume::ZERO,
            accepted_source: WaterVolume::new(precipitation),
            accepted_et: WaterVolume::ZERO,
        };
        let ordered = [allocation(1, 10), allocation(2, 20)];
        let swapped = [allocation(2, 20), allocation(1, 10)];

        assert_eq!(
            forcing_settlement_fingerprint(5, cell(0), &ordered),
            forcing_settlement_fingerprint(5, cell(0), &ordered)
        );
        assert_ne!(
            forcing_settlement_fingerprint(5, cell(0), &ordered),
            forcing_settlement_fingerprint(5, cell(0), &swapped),
            "order is part of the claim, so a reordering is a different one"
        );
        assert_ne!(
            forcing_settlement_fingerprint(5, cell(0), &ordered),
            forcing_settlement_fingerprint(5, cell(1), &ordered)
        );
        assert_ne!(
            forcing_settlement_fingerprint(5, cell(0), &[]),
            forcing_settlement_fingerprint(6, cell(0), &[]),
            "an empty settlement is still tick-specific"
        );
    }

    #[test]
    fn a_forcing_record_fingerprint_distinguishes_pending_from_applied() {
        assert_ne!(
            forcing_applied_fingerprint(5, 1, None),
            forcing_applied_fingerprint(5, 1, Some(5))
        );
        assert_ne!(
            forcing_applied_fingerprint(5, 1, Some(5)),
            forcing_applied_fingerprint(5, 2, Some(5))
        );
    }

    #[test]
    fn every_property_maps_to_a_bucket_tag() {
        for property in [
            HydrologyProperty::Surface,
            HydrologyProperty::Soil,
            HydrologyProperty::Groundwater,
            HydrologyProperty::Conveyance,
            HydrologyProperty::ForcingInput,
            HydrologyProperty::ForcingRecord,
            HydrologyProperty::Resolution,
            HydrologyProperty::BatchSequence,
            HydrologyProperty::BatchRoot,
            HydrologyProperty::CoarseInput,
        ] {
            assert!(property.bucket().tag() > 0);
        }
        assert_eq!(
            HydrologyProperty::Surface.bucket(),
            HydrologyBucket::Surface
        );
        assert_eq!(
            HydrologyProperty::ForcingInput.bucket(),
            HydrologyBucket::ForcingInput
        );
    }
}
