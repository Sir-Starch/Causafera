//! Ownership-moving undo for one hydrology tick.
//!
//! Current hydrology state can be large because its bounded retained receipt
//! window contains typed transfer detail. A tick therefore records only values
//! it replaces or evicts instead of cloning the complete state at transaction
//! open.

use std::collections::BTreeMap;

use causafera_domains::{HydrologyConservationReceipt, HydrologyTransferReceipt};
use causafera_geography::{
    HydrologyConveyanceGraph, HydrologyFieldSet, HydrologyForcingRecord, HydrologyResolutionState,
    MAX_HYDROLOGY_PERSISTED_TRANSFER_RECEIPTS, MAX_HYDROLOGY_STORED_RECEIPT_BATCHES,
};
use causafera_types::{ChartChunkCoord, TraceId};

use crate::{HydrologyRuntimeState, RuntimeError, RuntimeState};

pub(crate) struct HydrologyTickUpdate {
    pub(crate) fields: HydrologyFieldSet,
    pub(crate) conveyance: HydrologyConveyanceGraph,
    pub(crate) forcing: Vec<HydrologyForcingRecord>,
    pub(crate) resolution: BTreeMap<ChartChunkCoord, HydrologyResolutionState>,
    pub(crate) next_node_id: u64,
    pub(crate) conservation_trace: TraceId,
    pub(crate) transfer_receipts: Vec<HydrologyTransferReceipt>,
    pub(crate) conservation_receipt: HydrologyConservationReceipt,
}

pub(crate) struct HydrologyUndo {
    fields: HydrologyFieldSet,
    conveyance: HydrologyConveyanceGraph,
    forcing: Vec<HydrologyForcingRecord>,
    resolution: BTreeMap<ChartChunkCoord, HydrologyResolutionState>,
    next_node_id: u64,
    inserted_trace: TraceId,
    replaced_receipts: Option<Vec<HydrologyTransferReceipt>>,
    replaced_conservation: Option<HydrologyConservationReceipt>,
    evicted: Vec<EvictedBatch>,
}

struct EvictedBatch {
    trace: TraceId,
    receipts: Option<Vec<HydrologyTransferReceipt>>,
    conservation: Option<HydrologyConservationReceipt>,
}

impl RuntimeState {
    pub(crate) fn apply_hydrology_tick(
        &mut self,
        update: HydrologyTickUpdate,
    ) -> Result<(), RuntimeError> {
        if !self.tick_undo.is_active() {
            return Err(RuntimeError::PhaseDesynchronized);
        }
        let undo = update.apply(&mut self.hydrology);
        self.tick_undo.push_hydrology(undo);
        Ok(())
    }
}

impl HydrologyTickUpdate {
    fn apply(self, state: &mut HydrologyRuntimeState) -> HydrologyUndo {
        let fields = std::mem::replace(&mut state.fields, self.fields);
        let conveyance = std::mem::replace(&mut state.conveyance, self.conveyance);
        let forcing = std::mem::replace(&mut state.forcing, self.forcing);
        let resolution = std::mem::replace(&mut state.resolution, self.resolution);
        let next_node_id = std::mem::replace(&mut state.next_node_id, self.next_node_id);
        let replaced_receipts = state
            .receipts
            .insert(self.conservation_trace, self.transfer_receipts);
        let replaced_conservation = state
            .conservation_receipts
            .insert(self.conservation_trace, self.conservation_receipt);
        state.retained_batches.push(self.conservation_trace);

        let mut evicted = Vec::new();
        loop {
            let receipts = state
                .retained_batches
                .iter()
                .map(|trace| state.receipts.get(trace).map_or(0, Vec::len))
                .sum::<usize>();
            if (state.retained_batches.len() <= MAX_HYDROLOGY_STORED_RECEIPT_BATCHES
                && receipts <= MAX_HYDROLOGY_PERSISTED_TRANSFER_RECEIPTS)
                || state.retained_batches.len() <= 1
            {
                break;
            }
            let trace = state.retained_batches.remove(0);
            evicted.push(EvictedBatch {
                trace,
                receipts: state.receipts.remove(&trace),
                conservation: state.conservation_receipts.remove(&trace),
            });
        }

        HydrologyUndo {
            fields,
            conveyance,
            forcing,
            resolution,
            next_node_id,
            inserted_trace: self.conservation_trace,
            replaced_receipts,
            replaced_conservation,
            evicted,
        }
    }
}

impl HydrologyUndo {
    pub(crate) fn restore(self, state: &mut HydrologyRuntimeState) {
        state.fields = self.fields;
        state.conveyance = self.conveyance;
        state.forcing = self.forcing;
        state.resolution = self.resolution;
        state.next_node_id = self.next_node_id;

        state.receipts.remove(&self.inserted_trace);
        if let Some(receipts) = self.replaced_receipts {
            state.receipts.insert(self.inserted_trace, receipts);
        }
        state.conservation_receipts.remove(&self.inserted_trace);
        if let Some(receipt) = self.replaced_conservation {
            state
                .conservation_receipts
                .insert(self.inserted_trace, receipt);
        }
        if state.retained_batches.last() == Some(&self.inserted_trace) {
            state.retained_batches.pop();
        }
        for batch in self.evicted.into_iter().rev() {
            state.retained_batches.insert(0, batch.trace);
            if let Some(receipts) = batch.receipts {
                state.receipts.insert(batch.trace, receipts);
            }
            if let Some(receipt) = batch.conservation {
                state.conservation_receipts.insert(batch.trace, receipt);
            }
        }
    }
}
