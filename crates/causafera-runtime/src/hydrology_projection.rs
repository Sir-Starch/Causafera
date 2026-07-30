//! The bounded read-only hydrology projection the observer protocol carries.
//!
//! A sibling module rather than more surface on `runtime.rs`: hydrology already
//! owns its evolution, events, config, and validation modules, and the read
//! model belongs beside them. Nothing here mutates anything — every function
//! takes `&HydrologyRuntimeState` and returns owned observer types (INV-022).
//!
//! Two rules shape everything below. The projection is derived from the latest
//! *retained* batch, so what an observer can see and what the runtime still
//! holds evidence for are the same set: once a batch is evicted its typed detail
//! is gone from both, rather than surviving as an unsourced number here. And
//! every quantity is carried at the width it is stored at — `u64` per carrier,
//! `u128` for whole-scope totals, `i128` for signed residuals and net flows — so
//! the projection never narrows what the ledger closed over.
//!
//! See `plans/hydrology.md` §12.

use std::collections::BTreeMap;

use causafera_domains::{HydrologyBucket, HydrologyTransferReceipt, process};
use causafera_geography::{HydrologyCarrierKey, HydrologyCellKey};
use causafera_observer_api::{
    HYDROLOGY_CONVEYANCE_SUMMARY_SCHEMA_V1, HYDROLOGY_DELTA_SCHEMA_V1, HYDROLOGY_SUMMARY_SCHEMA_V1,
    HYDROLOGY_TRANSFER_SUMMARY_SCHEMA_V1, HydrologyCellDelta, HydrologyConveyanceSummary,
    HydrologyTransferSummary, MAX_HYDROLOGY_CONVEYANCE_SUMMARIES, MAX_HYDROLOGY_DELTAS,
    MAX_HYDROLOGY_TRANSFER_SUMMARIES, ObserverHydrologyForcing, ObserverHydrologySummary,
};
use causafera_types::TraceId;

use crate::HydrologyRuntimeState;

/// The three bounded lists a world snapshot carries, with their schema markers.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct HydrologyWorldProjection {
    pub(crate) deltas: Vec<HydrologyCellDelta>,
    pub(crate) delta_schema_version: u32,
    pub(crate) transfers: Vec<HydrologyTransferSummary>,
    pub(crate) transfer_schema_version: u32,
    pub(crate) conveyance: Vec<HydrologyConveyanceSummary>,
    pub(crate) conveyance_schema_version: u32,
}

impl HydrologyRuntimeState {
    /// The whole-session summary carried by every runtime summary.
    ///
    /// Present with zeroes for a disabled session rather than absent: "this
    /// build has no hydrology" is a fact about the payload's age and belongs to
    /// the wire layer, while "this world holds no water" is a measurement and
    /// belongs here.
    pub(crate) fn observer_summary(&self) -> ObserverHydrologySummary {
        let mut summary = ObserverHydrologySummary {
            schema_version: HYDROLOGY_SUMMARY_SCHEMA_V1,
            ..ObserverHydrologySummary::default()
        };
        if !self.enabled {
            return summary;
        }
        for field in self.fields.fields().values() {
            for cell in field.cells() {
                summary.total_surface += u128::from(cell.surface_water().get());
                summary.total_soil += u128::from(cell.soil_water().get());
                summary.total_groundwater += u128::from(cell.groundwater().get());
            }
        }
        for edge in self.conveyance.edges().values() {
            summary.total_conveyance += u128::from(edge.storage().get());
        }
        summary.active_chunk_count =
            u32::try_from(self.active.active_chunks().len()).unwrap_or(u32::MAX);
        summary.latest_residual = self
            .latest_batch()
            .and_then(|trace| self.conservation_receipts.get(&trace))
            .map_or(0, |receipt| receipt.residual());
        summary.latest_forcing = self.observer_latest_forcing();
        summary
    }

    /// The conservation trace of the newest retained batch.
    fn latest_batch(&self) -> Option<TraceId> {
        self.retained_batches.last().copied()
    }

    /// The greatest applied `(scheduled_tick, forcing_id)` and its accepted
    /// totals, or nothing when none has been applied or its batch is gone.
    ///
    /// The accepted volumes come from that record's own receipts rather than
    /// from the tick's conservation totals: two records may apply in one tick,
    /// and attributing the tick's whole accepted precipitation to whichever
    /// sorted last would be a fabricated per-record measurement. When the batch
    /// has been evicted the group is absent in full — reporting the identity
    /// beside zeroes would state that the record moved nothing.
    fn observer_latest_forcing(&self) -> Option<ObserverHydrologyForcing> {
        let record = self
            .forcing
            .iter()
            .filter(|record| record.is_applied())
            .max_by_key(|record| record.key())?;
        let applied_at = record.applied_at()?;
        let receipts = self.retained_receipts_at(applied_at)?;
        let carrier = HydrologyCarrierKey::ForcingRecord {
            scheduled_tick: record.scheduled_tick(),
            forcing_id: record.forcing_id(),
        };
        let mut accepted_source = 0_u128;
        let mut accepted_et = 0_u64;
        for receipt in receipts {
            if receipt.source() == carrier {
                accepted_source += u128::from(receipt.accepted().get());
            }
            if is_evapotranspiration(receipt)
                && receipt.forcing_origin() == Some(record.origin_trace())
            {
                accepted_et = accepted_et.saturating_add(receipt.accepted().get());
            }
        }
        Some(ObserverHydrologyForcing {
            tick: record.scheduled_tick(),
            forcing_id: record.forcing_id(),
            origin_trace: record.origin_trace(),
            accepted_source,
            accepted_et,
        })
    }

    /// The retained receipts of the batch committed at `tick`, if it survives.
    fn retained_receipts_at(&self, tick: u64) -> Option<&[HydrologyTransferReceipt]> {
        for trace in self.retained_batches.iter().rev() {
            let receipts = self.receipts.get(trace)?;
            if receipts
                .first()
                .is_some_and(|receipt| receipt.tick() == tick)
            {
                return Some(receipts);
            }
        }
        None
    }

    /// The three bounded per-tick lists, derived from the newest retained batch.
    pub(crate) fn observer_world_projection(&self) -> HydrologyWorldProjection {
        let mut projection = HydrologyWorldProjection::default();
        if !self.enabled {
            return projection;
        }
        let Some(conservation_trace) = self.latest_batch() else {
            return projection;
        };
        let Some(receipts) = self.receipts.get(&conservation_trace) else {
            return projection;
        };
        let tick = receipts.first().map_or(0, HydrologyTransferReceipt::tick);
        projection.transfers = self.transfer_summaries(receipts, conservation_trace);
        projection.deltas = self.cell_deltas(receipts, conservation_trace, tick);
        projection.conveyance = self.conveyance_summaries(receipts, tick);
        if !projection.deltas.is_empty() {
            projection.delta_schema_version = HYDROLOGY_DELTA_SCHEMA_V1;
        }
        if !projection.transfers.is_empty() {
            projection.transfer_schema_version = HYDROLOGY_TRANSFER_SUMMARY_SCHEMA_V1;
        }
        if !projection.conveyance.is_empty() {
            projection.conveyance_schema_version = HYDROLOGY_CONVEYANCE_SUMMARY_SCHEMA_V1;
        }
        projection
    }

    /// One summary per receipt, in canonical transfer-key order.
    fn transfer_summaries(
        &self,
        receipts: &[HydrologyTransferReceipt],
        conservation_trace: TraceId,
    ) -> Vec<HydrologyTransferSummary> {
        let mut summaries = receipts
            .iter()
            .map(|receipt| HydrologyTransferSummary {
                process_kind: receipt.process_kind(),
                source_key: receipt.source().encode(),
                target_key: receipt.target().encode(),
                requested_volume: receipt.requested().get(),
                accepted_volume: receipt.accepted().get(),
                unaccepted_volume: receipt.unaccepted().get(),
                // The receipt's own settlement anchor, which the cell carries
                // after commit; a receipt whose bucket is not cell storage
                // falls back to the batch's conservation trace so no summary
                // arrives without an anchor.
                transfer_trace: self.settlement_trace(receipt, conservation_trace),
                conservation_trace,
                tick: receipt.tick(),
                forcing_origin_trace: receipt.forcing_origin(),
            })
            .collect::<Vec<_>>();
        summaries.sort_by(|left, right| left.canonical_key().cmp(&right.canonical_key()));
        summaries.truncate(MAX_HYDROLOGY_TRANSFER_SUMMARIES);
        summaries
    }

    /// The committed trace anchoring one receipt's storage change.
    fn settlement_trace(
        &self,
        receipt: &HydrologyTransferReceipt,
        conservation_trace: TraceId,
    ) -> TraceId {
        let anchor = |cell: HydrologyCellKey, bucket: HydrologyBucket| {
            self.fields.cell(cell).map(|state| match bucket {
                HydrologyBucket::Surface => state.surface_last_change(),
                HydrologyBucket::Soil => state.soil_last_change(),
                HydrologyBucket::Groundwater => state.groundwater_last_change(),
                _ => state.forcing_last_change(),
            })
        };
        let from_target = match receipt.target() {
            HydrologyCarrierKey::Cell(cell) => anchor(cell, receipt.target_bucket()),
            _ => None,
        };
        let from_source = match receipt.source() {
            HydrologyCarrierKey::Cell(cell) => anchor(cell, receipt.source_bucket()),
            _ => None,
        };
        from_target.or(from_source).unwrap_or(conservation_trace)
    }

    /// One delta per cell the batch touched, in canonical cell order.
    ///
    /// Before and after come from the cell's own record of its last change, not
    /// from the receipts: a cell settled by several substages in one tick has
    /// one before-state and one after-state, and summing intermediate receipt
    /// endpoints would report a path rather than a transition.
    fn cell_deltas(
        &self,
        receipts: &[HydrologyTransferReceipt],
        conservation_trace: TraceId,
        tick: u64,
    ) -> Vec<HydrologyCellDelta> {
        let mut net: BTreeMap<HydrologyCellKey, (i128, i128)> = BTreeMap::new();
        for receipt in receipts {
            let accepted = i128::from(receipt.accepted().get());
            let source = as_cell(receipt.source());
            let target = as_cell(receipt.target());
            match (source, target) {
                // A vertical process moves water between buckets inside one
                // cell: it is neither a source, a sink, nor a lateral flow.
                (Some(source), Some(target)) if source == target => {
                    net.entry(source).or_default();
                }
                (Some(source), Some(target)) => {
                    net.entry(source).or_default().1 -= accepted;
                    net.entry(target).or_default().1 += accepted;
                }
                (None, Some(target)) => {
                    // Everything reaching a cell from a non-cell carrier is
                    // either forcing or conveyance release. Forcing is a source
                    // term; a release is water re-entering from an edge.
                    let slot = net.entry(target).or_default();
                    if matches!(receipt.source(), HydrologyCarrierKey::ForcingRecord { .. }) {
                        slot.0 += accepted;
                    } else {
                        slot.1 += accepted;
                    }
                }
                (Some(source), None) => {
                    let slot = net.entry(source).or_default();
                    if is_evapotranspiration(receipt) {
                        slot.0 -= accepted;
                    } else {
                        slot.1 -= accepted;
                    }
                }
                (None, None) => {}
            }
        }
        let mut deltas = net
            .into_iter()
            .filter_map(|(cell, (net_forcing, net_lateral_flow))| {
                let state = self.fields.cell(cell)?;
                let before = state.last_change_before();
                let after = state.storage();
                Some(HydrologyCellDelta {
                    chart_id: cell.chart().raw(),
                    chunk_x: cell.chunk().chunk.x,
                    chunk_y: cell.chunk().chunk.y,
                    chunk_z: cell.chunk().chunk.z,
                    cell_ordinal: cell.cell_ordinal(),
                    surface_before: before.surface.get(),
                    surface_after: after.surface.get(),
                    soil_before: before.soil.get(),
                    soil_after: after.soil.get(),
                    groundwater_before: before.groundwater.get(),
                    groundwater_after: after.groundwater.get(),
                    net_forcing,
                    net_lateral_flow,
                    // The most recent of the cell's three bucket anchors: the
                    // event that last settled any part of this transition.
                    transition_trace: state
                        .surface_last_change()
                        .max(state.soil_last_change())
                        .max(state.groundwater_last_change()),
                    conservation_trace,
                    transition_tick: tick,
                })
            })
            .collect::<Vec<_>>();
        deltas.sort_by_key(|delta| {
            (
                delta.chart_id,
                delta.chunk_x,
                delta.chunk_y,
                delta.chunk_z,
                delta.cell_ordinal,
            )
        });
        deltas.truncate(MAX_HYDROLOGY_DELTAS);
        deltas
    }

    /// One summary per conveyance edge the batch exchanged water with.
    fn conveyance_summaries(
        &self,
        receipts: &[HydrologyTransferReceipt],
        tick: u64,
    ) -> Vec<HydrologyConveyanceSummary> {
        let mut exchange: BTreeMap<_, (u64, u64)> = BTreeMap::new();
        for receipt in receipts {
            if let HydrologyCarrierKey::Edge(edge) = receipt.target() {
                exchange.entry(edge).or_default().0 += receipt.accepted().get();
            }
            if let HydrologyCarrierKey::Edge(edge) = receipt.source() {
                exchange.entry(edge).or_default().1 += receipt.accepted().get();
            }
        }
        let mut summaries = exchange
            .into_iter()
            .filter_map(|(key, (accepted_inflow, accepted_release))| {
                let edge = self.conveyance.edges().get(&key)?;
                Some(HydrologyConveyanceSummary {
                    edge_key: HydrologyCarrierKey::Edge(key).encode(),
                    // Current storage, not the batch's opening balance: the
                    // question a conveyance summary answers is what the channel
                    // holds now.
                    storage: edge.storage().get(),
                    capacity: edge.capacity().get(),
                    accepted_inflow,
                    accepted_release,
                    last_change_trace: edge.last_change(),
                    tick,
                })
            })
            .collect::<Vec<_>>();
        summaries.sort_by(|left, right| left.edge_key.cmp(&right.edge_key));
        summaries.truncate(MAX_HYDROLOGY_CONVEYANCE_SUMMARIES);
        summaries
    }
}

fn as_cell(key: HydrologyCarrierKey) -> Option<HydrologyCellKey> {
    match key {
        HydrologyCarrierKey::Cell(cell) => Some(cell),
        _ => None,
    }
}

fn is_evapotranspiration(receipt: &HydrologyTransferReceipt) -> bool {
    matches!(
        receipt.process_kind(),
        process::EVAPOTRANSPIRATION_SURFACE | process::EVAPOTRANSPIRATION_SOIL
    )
}
