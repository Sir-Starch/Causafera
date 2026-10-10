use std::cmp::Ordering;

use causafera_types::{SimulationTime, TraceId};

use crate::StateFingerprint;

use super::HistoryError;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EvidenceSchemaId(u64);

impl EvidenceSchemaId {
    pub const fn new(raw: u64) -> Self {
        Self(raw)
    }
    pub const fn raw(self) -> u64 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EvidenceSchemaVersion {
    pub major: u16,
    pub minor: u16,
}

impl EvidenceSchemaVersion {
    pub const fn new(major: u16, minor: u16) -> Self {
        Self { major, minor }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Criticality {
    ReplayRequired,
    ExplanationOnly,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DomainEvidenceRecord {
    commit_sequence: u64,
    schema_id: EvidenceSchemaId,
    version: EvidenceSchemaVersion,
    criticality: Criticality,
    time: SimulationTime,
    causal_anchors: Vec<TraceId>,
    canonical_key: Vec<u8>,
    canonical_payload: Vec<u8>,
    payload_digest: StateFingerprint,
}

impl Ord for DomainEvidenceRecord {
    fn cmp(&self, other: &Self) -> Ordering {
        self.commit_sequence
            .cmp(&other.commit_sequence)
            .then_with(|| self.canonical_key.cmp(&other.canonical_key))
            .then_with(|| self.schema_id.cmp(&other.schema_id))
            .then_with(|| self.payload_digest.cmp(&other.payload_digest))
    }
}

impl PartialOrd for DomainEvidenceRecord {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl DomainEvidenceRecord {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        commit_sequence: u64,
        schema_id: EvidenceSchemaId,
        version: EvidenceSchemaVersion,
        criticality: Criticality,
        time: SimulationTime,
        causal_anchors: Vec<TraceId>,
        canonical_key: Vec<u8>,
        canonical_payload: Vec<u8>,
        payload_digest: StateFingerprint,
    ) -> Result<Self, HistoryError> {
        for index in 1..causal_anchors.len() {
            if causal_anchors[index - 1] >= causal_anchors[index] {
                return Err(HistoryError::AnchorsNotStrictlyOrdered { index });
            }
        }
        Ok(Self {
            commit_sequence,
            schema_id,
            version,
            criticality,
            time,
            causal_anchors,
            canonical_key,
            canonical_payload,
            payload_digest,
        })
    }
    pub const fn commit_sequence(&self) -> u64 {
        self.commit_sequence
    }
    pub const fn schema_id(&self) -> EvidenceSchemaId {
        self.schema_id
    }
    pub const fn version(&self) -> EvidenceSchemaVersion {
        self.version
    }
    pub const fn criticality(&self) -> Criticality {
        self.criticality
    }
    pub const fn time(&self) -> SimulationTime {
        self.time
    }
    pub fn causal_anchors(&self) -> &[TraceId] {
        &self.causal_anchors
    }
    pub fn canonical_key(&self) -> &[u8] {
        &self.canonical_key
    }
    pub fn canonical_payload(&self) -> &[u8] {
        &self.canonical_payload
    }
    pub const fn payload_digest(&self) -> StateFingerprint {
        self.payload_digest
    }
}
