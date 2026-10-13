use causafera_types::{EventId, SimulationTime, TraceId};

use super::HistoryError;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct HistoryFrontier {
    pub generation: u64,
    pub event_id: EventId,
    pub trace_id: TraceId,
    pub time: SimulationTime,
}

impl HistoryFrontier {
    pub const fn new(
        generation: u64,
        event_id: EventId,
        trace_id: TraceId,
        time: SimulationTime,
    ) -> Self {
        Self {
            generation,
            event_id,
            trace_id,
            time,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HistoryDurability {
    EveryTick,
    GroupCommit {
        max_ticks: u16,
        max_wall_time_ms: u32,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HistoryAnchorFields {
    pub namespace: [u8; 16],
    pub logical_generation: u64,
    pub durable_generation: u64,
    pub checkpoint_digest: [u8; 32],
    pub frontier: HistoryFrontier,
    pub logical_digest: [u8; 32],
    pub storage_root: [u8; 32],
    pub physical_digest_schema: u16,
    pub history_digest_schema: u16,
    pub evidence_manifest_digest: [u8; 32],
    pub durability: HistoryDurability,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HistoryAnchor {
    fields: HistoryAnchorFields,
}

impl HistoryAnchor {
    pub fn new(fields: HistoryAnchorFields) -> Result<Self, HistoryError> {
        if fields.durable_generation > fields.logical_generation {
            return Err(HistoryError::DurableGenerationAhead {
                logical: fields.logical_generation,
                durable: fields.durable_generation,
            });
        }
        Ok(Self { fields })
    }
    pub const fn namespace(&self) -> &[u8; 16] {
        &self.fields.namespace
    }
    pub const fn logical_generation(&self) -> u64 {
        self.fields.logical_generation
    }
    pub const fn durable_generation(&self) -> u64 {
        self.fields.durable_generation
    }
    pub const fn checkpoint_digest(&self) -> &[u8; 32] {
        &self.fields.checkpoint_digest
    }
    pub const fn frontier(&self) -> HistoryFrontier {
        self.fields.frontier
    }
    pub const fn logical_digest(&self) -> &[u8; 32] {
        &self.fields.logical_digest
    }
    pub const fn storage_root(&self) -> &[u8; 32] {
        &self.fields.storage_root
    }
    pub const fn physical_digest_schema(&self) -> u16 {
        self.fields.physical_digest_schema
    }
    pub const fn history_digest_schema(&self) -> u16 {
        self.fields.history_digest_schema
    }
    pub const fn evidence_manifest_digest(&self) -> &[u8; 32] {
        &self.fields.evidence_manifest_digest
    }
    pub const fn durability(&self) -> HistoryDurability {
        self.fields.durability
    }
}
