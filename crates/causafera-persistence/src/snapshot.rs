pub use crate::codec::{LittleEndianDecoder, LittleEndianEncoder};
pub use crate::envelope::{
    FORMAT_MAJOR_V1, FORMAT_MINOR_V1, MAX_SECTION_COUNT, MAX_TOTAL_SIZE, SNAPSHOT_MAGIC,
    SectionDirectoryEntry, SectionPayload, SnapshotEnvelope, SnapshotHeader,
};
pub use crate::error::PersistenceError;

/// Simulation snapshot for persistence.
///
/// This is the developer-facing type that wraps the canonical envelope.
/// It replaces the previous serde/JSON placeholder.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Snapshot {
    pub world_seed: u64,
    pub completed_time: u64,
    pub physical_digest: [u8; 32],
    pub history_digest: [u8; 32],
    pub sections: Vec<SnapshotSection>,
}

/// A single opaque section within a snapshot.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SnapshotSection {
    pub schema_id: u64,
    pub major: u16,
    pub minor: u16,
    pub bytes: Vec<u8>,
}

impl Snapshot {
    /// Build a snapshot from an envelope.
    pub fn from_envelope(envelope: SnapshotEnvelope) -> Self {
        let mut sections = Vec::with_capacity(envelope.sections.len());
        for (schema_id, payload) in envelope.sections {
            sections.push(SnapshotSection {
                schema_id,
                major: payload.section_major,
                minor: payload.section_minor,
                bytes: payload.bytes,
            });
        }
        Self {
            world_seed: envelope.header.world_seed,
            completed_time: envelope.header.completed_time,
            physical_digest: envelope.header.physical_digest,
            history_digest: envelope.header.history_digest,
            sections,
        }
    }

    /// Build an envelope from this snapshot.
    pub fn to_envelope(&self) -> Result<SnapshotEnvelope, PersistenceError> {
        let mut sections = std::collections::BTreeMap::new();
        for section in &self.sections {
            sections.insert(
                section.schema_id,
                SectionPayload {
                    section_major: section.major,
                    section_minor: section.minor,
                    flags: 0,
                    decoded_size_limit: 0,
                    bytes: section.bytes.clone(),
                },
            );
        }
        let header = SnapshotHeader {
            format_major: FORMAT_MAJOR_V1,
            format_minor: FORMAT_MINOR_V1,
            codec_revision: 1,
            world_seed: self.world_seed,
            completed_time: self.completed_time,
            runtime_recipe_fingerprint: [0u8; 32],
            physical_digest_schema: 1,
            physical_digest: self.physical_digest,
            history_digest_schema: 1,
            history_digest: self.history_digest,
            section_count: 0,             // computed during encode
            section_directory_offset: 0,  // computed during encode
            payload_integrity: [0u8; 32], // computed during encode
        };
        Ok(SnapshotEnvelope::new(header, sections))
    }
}
