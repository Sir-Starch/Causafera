use ontopolis_core::StateFingerprint;
use ontopolis_types::{
    CrossWorldTransferId, PlaceId, SimulationTime, SourceWorldId, StateObjectKindId,
    StatePropertyId, TraceId, TransferMechanismSchemaId, TransferPayloadId,
};
use thiserror::Error;

pub const MAX_TRANSFER_PAYLOADS: usize = 64;
pub const MAX_TRANSFER_PROPERTIES: usize = 128;
pub const MAX_TRANSFER_CAUSES: usize = 64;

/// One objective carrier named by an opaque schema and canonical state digest.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct TransferPayload {
    id: TransferPayloadId,
    object_kind: StateObjectKindId,
    source_state: StateFingerprint,
}

impl TransferPayload {
    pub const fn new(
        id: TransferPayloadId,
        object_kind: StateObjectKindId,
        source_state: StateFingerprint,
    ) -> Self {
        Self {
            id,
            object_kind,
            source_state,
        }
    }
    pub const fn id(self) -> TransferPayloadId {
        self.id
    }
    pub const fn object_kind(self) -> StateObjectKindId {
        self.object_kind
    }
    pub const fn source_state(self) -> StateFingerprint {
        self.source_state
    }
}

/// Explicit source/target property correspondence. Absence means no claim of persistence.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct PropertyCorrespondence {
    payload: TransferPayloadId,
    source: StatePropertyId,
    target: StatePropertyId,
}

impl PropertyCorrespondence {
    pub const fn new(
        payload: TransferPayloadId,
        source: StatePropertyId,
        target: StatePropertyId,
    ) -> Self {
        Self {
            payload,
            source,
            target,
        }
    }
    pub const fn payload(self) -> TransferPayloadId {
        self.payload
    }
    pub const fn source(self) -> StatePropertyId {
        self.source
    }
    pub const fn target(self) -> StatePropertyId {
        self.target
    }
}

/// Metaphysically neutral, proposal-only description of a crossing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CrossWorldTransferPlan {
    id: CrossWorldTransferId,
    seed: u64,
    mechanism: TransferMechanismSchemaId,
    scheduled_at: SimulationTime,
    source_world: SourceWorldId,
    source_location: StateFingerprint,
    target_place: PlaceId,
    payloads: Vec<TransferPayload>,
    correspondences: Vec<PropertyCorrespondence>,
    causes: Vec<TraceId>,
}

impl CrossWorldTransferPlan {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: CrossWorldTransferId,
        seed: u64,
        mechanism: TransferMechanismSchemaId,
        scheduled_at: SimulationTime,
        source_world: SourceWorldId,
        source_location: StateFingerprint,
        target_place: PlaceId,
        mut payloads: Vec<TransferPayload>,
        mut correspondences: Vec<PropertyCorrespondence>,
        causes: Vec<TraceId>,
    ) -> Result<Self, TransferError> {
        validate_nonempty_bounded(
            &mut payloads,
            MAX_TRANSFER_PAYLOADS,
            TransferError::NoPayloads,
            TransferError::PayloadCapacity,
        )?;
        validate_bounded_sorted(
            &mut correspondences,
            MAX_TRANSFER_PROPERTIES,
            TransferError::PropertyCapacity,
            TransferError::DuplicateProperty,
        )?;
        validate_sorted_bounded(
            &causes,
            MAX_TRANSFER_CAUSES,
            TransferError::CauseCapacity,
            TransferError::CausesNotCanonical,
        )?;
        if correspondences.iter().any(|item| {
            payloads
                .binary_search_by_key(&item.payload(), |p| p.id())
                .is_err()
        }) {
            return Err(TransferError::UnknownPropertyPayload);
        }
        Ok(Self {
            id,
            seed,
            mechanism,
            scheduled_at,
            source_world,
            source_location,
            target_place,
            payloads,
            correspondences,
            causes,
        })
    }
    pub const fn id(&self) -> CrossWorldTransferId {
        self.id
    }
    pub const fn mechanism(&self) -> TransferMechanismSchemaId {
        self.mechanism
    }
    pub const fn scheduled_at(&self) -> SimulationTime {
        self.scheduled_at
    }
    pub const fn source_world(&self) -> SourceWorldId {
        self.source_world
    }
    pub const fn source_location(&self) -> StateFingerprint {
        self.source_location
    }
    pub const fn target_place(&self) -> PlaceId {
        self.target_place
    }
    pub fn payloads(&self) -> &[TransferPayload] {
        &self.payloads
    }
    pub fn correspondences(&self) -> &[PropertyCorrespondence] {
        &self.correspondences
    }
    pub fn causes(&self) -> &[TraceId] {
        &self.causes
    }
    pub fn execution_seed(&self) -> u64 {
        mix64(
            self.seed
                ^ self.id.raw().rotate_left(11)
                ^ self.mechanism.raw().rotate_left(29)
                ^ self.scheduled_at.raw().rotate_left(43),
        )
    }
    pub fn validate_receipt(
        self,
        receipt: CrossWorldTransferReceipt,
    ) -> Result<CrossWorldTransferRecord, TransferError> {
        if receipt.transfer != self.id || receipt.completed_at < self.scheduled_at {
            return Err(TransferError::ReceiptMismatch);
        }
        if receipt.causes != self.causes {
            return Err(TransferError::ReceiptCausesMismatch);
        }
        if receipt.results.len() != self.payloads.len()
            || receipt
                .results
                .iter()
                .zip(&self.payloads)
                .any(|(r, p)| r.payload != p.id())
        {
            return Err(TransferError::ReceiptPayloadMismatch);
        }
        Ok(CrossWorldTransferRecord {
            plan: self,
            receipt,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct TransferPayloadResult {
    payload: TransferPayloadId,
    target_state: StateFingerprint,
}
impl TransferPayloadResult {
    pub const fn new(payload: TransferPayloadId, target_state: StateFingerprint) -> Self {
        Self {
            payload,
            target_state,
        }
    }
    pub const fn payload(self) -> TransferPayloadId {
        self.payload
    }
    pub const fn target_state(self) -> StateFingerprint {
        self.target_state
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CrossWorldTransferReceipt {
    transfer: CrossWorldTransferId,
    completed_at: SimulationTime,
    results: Vec<TransferPayloadResult>,
    trace: TraceId,
    causes: Vec<TraceId>,
}
impl CrossWorldTransferReceipt {
    pub fn new(
        transfer: CrossWorldTransferId,
        completed_at: SimulationTime,
        mut results: Vec<TransferPayloadResult>,
        trace: TraceId,
        causes: Vec<TraceId>,
    ) -> Result<Self, TransferError> {
        validate_nonempty_bounded(
            &mut results,
            MAX_TRANSFER_PAYLOADS,
            TransferError::NoPayloads,
            TransferError::PayloadCapacity,
        )?;
        validate_sorted_bounded(
            &causes,
            MAX_TRANSFER_CAUSES,
            TransferError::CauseCapacity,
            TransferError::CausesNotCanonical,
        )?;
        Ok(Self {
            transfer,
            completed_at,
            results,
            trace,
            causes,
        })
    }
    pub const fn trace(&self) -> TraceId {
        self.trace
    }
    pub fn results(&self) -> &[TransferPayloadResult] {
        &self.results
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CrossWorldTransferRecord {
    plan: CrossWorldTransferPlan,
    receipt: CrossWorldTransferReceipt,
}
impl CrossWorldTransferRecord {
    pub const fn plan(&self) -> &CrossWorldTransferPlan {
        &self.plan
    }
    pub const fn receipt(&self) -> &CrossWorldTransferReceipt {
        &self.receipt
    }
}

#[derive(Clone, Copy, Debug, Error, PartialEq, Eq)]
pub enum TransferError {
    #[error("transfer has no objective payload")]
    NoPayloads,
    #[error("transfer payload capacity exceeded")]
    PayloadCapacity,
    #[error("transfer property capacity exceeded")]
    PropertyCapacity,
    #[error("transfer cause capacity exceeded")]
    CauseCapacity,
    #[error("duplicate payload or result identity")]
    DuplicatePayload,
    #[error("duplicate property correspondence")]
    DuplicateProperty,
    #[error("property correspondence names an unknown payload")]
    UnknownPropertyPayload,
    #[error("causes are not strictly canonical")]
    CausesNotCanonical,
    #[error("receipt does not match transfer identity or time")]
    ReceiptMismatch,
    #[error("receipt causes do not exactly continue the plan")]
    ReceiptCausesMismatch,
    #[error("receipt payloads do not exactly cover the plan")]
    ReceiptPayloadMismatch,
}

fn validate_nonempty_bounded<T: Ord>(
    values: &mut [T],
    cap: usize,
    empty: TransferError,
    full: TransferError,
) -> Result<(), TransferError> {
    if values.is_empty() {
        return Err(empty);
    }
    if values.len() > cap {
        return Err(full);
    }
    values.sort_unstable();
    if values.windows(2).any(|w| w[0] == w[1]) {
        return Err(TransferError::DuplicatePayload);
    }
    Ok(())
}
fn validate_bounded_sorted<T: Ord>(
    values: &mut [T],
    cap: usize,
    full: TransferError,
    duplicate: TransferError,
) -> Result<(), TransferError> {
    if values.len() > cap {
        return Err(full);
    }
    values.sort_unstable();
    if values.windows(2).any(|w| w[0] == w[1]) {
        return Err(duplicate);
    }
    Ok(())
}
fn validate_sorted_bounded<T: Ord>(
    values: &[T],
    cap: usize,
    full: TransferError,
    order: TransferError,
) -> Result<(), TransferError> {
    if values.len() > cap {
        return Err(full);
    }
    if values.windows(2).any(|w| w[0] >= w[1]) {
        return Err(order);
    }
    Ok(())
}
fn mix64(mut value: u64) -> u64 {
    value ^= value >> 30;
    value = value.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value ^= value >> 27;
    value = value.wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fp(n: u8) -> StateFingerprint {
        StateFingerprint::new([n; 32])
    }
    fn plan(payloads: Vec<TransferPayload>) -> CrossWorldTransferPlan {
        CrossWorldTransferPlan::new(
            CrossWorldTransferId::new(2),
            7,
            TransferMechanismSchemaId::new(9),
            SimulationTime::new(20),
            SourceWorldId::new(4),
            fp(1),
            PlaceId::new(8),
            payloads,
            vec![],
            vec![TraceId::new(3)],
        )
        .unwrap()
    }
    #[test]
    fn plan_is_canonical_and_seed_is_order_independent() {
        let a = TransferPayload::new(TransferPayloadId::new(2), StateObjectKindId::new(1), fp(2));
        let b = TransferPayload::new(TransferPayloadId::new(1), StateObjectKindId::new(1), fp(3));
        assert_eq!(plan(vec![a, b]).payloads(), plan(vec![b, a]).payloads());
        assert_eq!(
            plan(vec![a, b]).execution_seed(),
            plan(vec![b, a]).execution_seed()
        );
    }
    #[test]
    fn receipt_requires_exact_payload_and_causal_continuation() {
        let p = TransferPayload::new(TransferPayloadId::new(1), StateObjectKindId::new(1), fp(2));
        let receipt = CrossWorldTransferReceipt::new(
            CrossWorldTransferId::new(2),
            SimulationTime::new(21),
            vec![TransferPayloadResult::new(p.id(), fp(5))],
            TraceId::new(6),
            vec![TraceId::new(3)],
        )
        .unwrap();
        assert!(plan(vec![p]).validate_receipt(receipt).is_ok());
    }
}
