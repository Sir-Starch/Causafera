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

use causafera_domains::{HydrologyBucket, HydrologyTransferReceipt, groundwater_head_mm, process};
use causafera_explanation::{
    ExplanationClaim, ExplanationIrError, HydrologyConservationClaim, HydrologyForcingClaim,
    HydrologyStorageClaim, HydrologyTransferPathClaim,
};
use causafera_geography::{HydrologyCarrierKey, HydrologyCellKey};
use causafera_observer_api::{
    HYDROLOGY_CONVEYANCE_SUMMARY_SCHEMA_V1, HYDROLOGY_DELTA_SCHEMA_V1, HYDROLOGY_SUMMARY_SCHEMA_V1,
    HYDROLOGY_TRANSFER_SUMMARY_SCHEMA_V1, HydrologyCellDelta, HydrologyConveyanceSummary,
    HydrologyTransferSummary, MAX_HYDROLOGY_CONVEYANCE_SUMMARIES, MAX_HYDROLOGY_DELTAS,
    MAX_HYDROLOGY_TRANSFER_SUMMARIES, ObserverHydrologyForcing, ObserverHydrologySummary,
};
use causafera_types::{ChartChunkCoord, TraceId};

/// How many trace anchors one claim carries.
///
/// A claim is evidence, not a log: an unbounded ancestry would make one query's
/// answer grow with the size of the batch it describes. The anchors that survive
/// are the smallest ones, which is a deterministic choice rather than whichever
/// arrived first.
const MAX_EXPLANATION_EVIDENCE_TRACES: usize = 64;

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

/* ------------------------------------------------------------- explanation -- */

impl HydrologyRuntimeState {
    /// Typed hydrology evidence for one chunk, or for the whole resident scope.
    ///
    /// Insufficiency rather than error is the rule throughout: a chunk that is
    /// not resident, a session with no water, and a batch that retention has
    /// evicted all answer with `Unknown` claims. An observer must be able to
    /// tell "no evidence" apart from "a failed query", and neither may be
    /// answered with a fabricated classification (V29).
    pub(crate) fn explanation_claims(
        &self,
        scope: Option<ChartChunkCoord>,
    ) -> Result<Vec<ExplanationClaim>, ExplanationIrError> {
        let mut claims = Vec::new();
        claims.extend(self.storage_claim(scope)?.to_explanation_claims()?);
        claims.extend(self.forcing_claims()?);
        claims.extend(self.conservation_claims()?);
        claims.extend(self.transfer_claims(scope)?);
        Ok(claims)
    }

    /// Storage bounds, whole total, and water-table span over the scope.
    fn storage_claim(
        &self,
        scope: Option<ChartChunkCoord>,
    ) -> Result<HydrologyStorageClaim, ExplanationIrError> {
        let mut claim = HydrologyStorageClaim {
            carrier_count: 0,
            minimum_volume: u64::MAX,
            maximum_volume: 0,
            total_volume: 0,
            water_table_minimum_mm: i64::MAX,
            water_table_maximum_mm: i64::MIN,
            evidence_traces: Vec::new(),
        };
        for (chunk, field) in self.fields.fields() {
            if scope.is_some_and(|wanted| wanted != *chunk) {
                continue;
            }
            let Ok(metric) = self.metrics.get(chunk.chart) else {
                continue;
            };
            for (ordinal, cell) in field.cells().iter().enumerate() {
                let Some(ground) = field.ground(ordinal as u16) else {
                    continue;
                };
                // The water table comes from the solver's own formula rather
                // than from a second copy of it, so the claim describes the
                // number routing actually used.
                let Ok(head) = groundwater_head_mm(metric, ground, cell.groundwater()) else {
                    continue;
                };
                let Ok(head) = i64::try_from(head) else {
                    continue;
                };
                for volume in [
                    cell.surface_water().get(),
                    cell.soil_water().get(),
                    cell.groundwater().get(),
                ] {
                    claim.carrier_count += 1;
                    claim.minimum_volume = claim.minimum_volume.min(volume);
                    claim.maximum_volume = claim.maximum_volume.max(volume);
                    claim.total_volume += u128::from(volume);
                }
                claim.water_table_minimum_mm = claim.water_table_minimum_mm.min(head);
                claim.water_table_maximum_mm = claim.water_table_maximum_mm.max(head);
                for trace in [
                    cell.surface_last_change(),
                    cell.soil_last_change(),
                    cell.groundwater_last_change(),
                ] {
                    claim.evidence_traces.push(trace);
                }
            }
        }
        if claim.carrier_count == 0 {
            // Leave the sentinels behind: an empty scope reports itself empty
            // rather than as a set of extreme measurements.
            claim.minimum_volume = 0;
            claim.water_table_minimum_mm = 0;
            claim.water_table_maximum_mm = 0;
            claim.evidence_traces.clear();
        }
        claim.evidence_traces.sort_unstable();
        claim.evidence_traces.dedup();
        claim
            .evidence_traces
            .truncate(MAX_EXPLANATION_EVIDENCE_TRACES);
        Ok(claim)
    }

    /// The latest applied record's accepted and unmet volumes, with ancestry.
    fn forcing_claims(&self) -> Result<Vec<ExplanationClaim>, ExplanationIrError> {
        let Some(record) = self
            .forcing
            .iter()
            .filter(|record| record.is_applied())
            .max_by_key(|record| record.key())
        else {
            return HydrologyForcingClaim::unknown();
        };
        let Some(applied_at) = record.applied_at() else {
            return HydrologyForcingClaim::unknown();
        };
        let Some(receipts) = self.retained_receipts_at(applied_at) else {
            // The record still applied; its typed detail has been evicted.
            return HydrologyForcingClaim::unknown();
        };
        let carrier = HydrologyCarrierKey::ForcingRecord {
            scheduled_tick: record.scheduled_tick(),
            forcing_id: record.forcing_id(),
        };
        let mut claim = HydrologyForcingClaim {
            scheduled_tick: record.scheduled_tick(),
            forcing_id: record.forcing_id(),
            accepted_source: 0,
            unmet_source: 0,
            accepted_evapotranspiration: 0,
            unmet_evapotranspiration: 0,
            origin_trace: record.origin_trace(),
            settlement_traces: Vec::new(),
        };
        for receipt in receipts {
            if receipt.source() == carrier {
                claim.accepted_source += u128::from(receipt.accepted().get());
                claim.unmet_source = claim
                    .unmet_source
                    .saturating_add(receipt.unaccepted().get());
                claim
                    .settlement_traces
                    .push(self.settlement_trace(receipt, TraceId::new(0)));
            }
            if is_evapotranspiration(receipt)
                && receipt.forcing_origin() == Some(record.origin_trace())
            {
                claim.accepted_evapotranspiration = claim
                    .accepted_evapotranspiration
                    .saturating_add(receipt.accepted().get());
                claim.unmet_evapotranspiration = claim
                    .unmet_evapotranspiration
                    .saturating_add(receipt.unaccepted().get());
            }
        }
        claim.settlement_traces.sort_unstable();
        claim.settlement_traces.dedup();
        claim
            .settlement_traces
            .truncate(MAX_EXPLANATION_EVIDENCE_TRACES);
        claim.to_explanation_claims()
    }

    /// The latest retained batch's residual and boundary export.
    fn conservation_claims(&self) -> Result<Vec<ExplanationClaim>, ExplanationIrError> {
        let Some(trace) = self.latest_batch() else {
            return HydrologyConservationClaim::unknown();
        };
        let Some(receipt) = self.conservation_receipts.get(&trace) else {
            return HydrologyConservationClaim::unknown();
        };
        let mut transfer_traces = self
            .receipts
            .get(&trace)
            .map(|receipts| {
                receipts
                    .iter()
                    .map(|receipt| self.settlement_trace(receipt, trace))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        transfer_traces.sort_unstable();
        transfer_traces.dedup();
        transfer_traces.truncate(MAX_EXPLANATION_EVIDENCE_TRACES);
        HydrologyConservationClaim {
            residual: receipt.residual(),
            boundary_exports: receipt.boundary_exports(),
            conservation_trace: trace,
            transfer_traces,
        }
        .to_explanation_claims()
    }

    /// One accepted/limited pair per transfer in the latest retained batch.
    fn transfer_claims(
        &self,
        scope: Option<ChartChunkCoord>,
    ) -> Result<Vec<ExplanationClaim>, ExplanationIrError> {
        let Some(conservation_trace) = self.latest_batch() else {
            return Ok(Vec::new());
        };
        let Some(receipts) = self.receipts.get(&conservation_trace) else {
            return Ok(Vec::new());
        };
        let mut claims = Vec::new();
        for receipt in receipts.iter().take(MAX_HYDROLOGY_TRANSFER_SUMMARIES) {
            if let Some(wanted) = scope
                && !touches_chunk(receipt, wanted)
            {
                continue;
            }
            claims.extend(
                HydrologyTransferPathClaim {
                    process_kind: receipt.process_kind(),
                    requested_volume: receipt.requested().get(),
                    accepted_volume: receipt.accepted().get(),
                    unaccepted_volume: receipt.unaccepted().get(),
                    transfer_trace: self.settlement_trace(receipt, conservation_trace),
                    conservation_trace,
                    forcing_origin_trace: receipt.forcing_origin(),
                }
                .to_explanation_claims()?,
            );
        }
        Ok(claims)
    }
}

/// Whether either endpoint of a transfer sits in the requested chunk.
fn touches_chunk(receipt: &HydrologyTransferReceipt, chunk: ChartChunkCoord) -> bool {
    [receipt.source(), receipt.target()]
        .into_iter()
        .any(|key| match key {
            HydrologyCarrierKey::Cell(cell) => cell.chunk() == chunk,
            HydrologyCarrierKey::Edge(edge) => {
                edge.low().chunk() == chunk || edge.high().chunk() == chunk
            }
            HydrologyCarrierKey::ExteriorFace(face) => face.cell().chunk() == chunk,
            HydrologyCarrierKey::ResolutionChunk(coord) => coord == chunk,
            _ => false,
        })
}
