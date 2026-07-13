use crate::codec::{LittleEndianDecoder, LittleEndianEncoder};
use crate::error::PersistenceError;
use blake3::Hasher;
use std::collections::BTreeMap;

/// Fixed magic bytes: "OTPS" in ASCII.
pub const SNAPSHOT_MAGIC: [u8; 4] = [0x4F, 0x54, 0x50, 0x53];

/// Current format version.
pub const FORMAT_MAJOR_V1: u16 = 1;
pub const FORMAT_MINOR_V1: u16 = 0;

/// Maximum supported section count to prevent memory exhaustion.
pub const MAX_SECTION_COUNT: u64 = 256;

/// Maximum total file size (256 MiB for v1).
pub const MAX_TOTAL_SIZE: u64 = 256 * 1024 * 1024;

/// 32-byte integrity digest.
pub type IntegrityDigest = [u8; 32];

/// Snapshot header fields.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SnapshotHeader {
    pub format_major: u16,
    pub format_minor: u16,
    pub codec_revision: u32,
    pub world_seed: u64,
    pub completed_time: u64,
    pub runtime_recipe_fingerprint: IntegrityDigest,
    pub physical_digest_schema: u16,
    pub physical_digest: IntegrityDigest,
    pub history_digest_schema: u16,
    pub history_digest: IntegrityDigest,
    pub section_count: u16,
    pub section_directory_offset: u64,
    pub payload_integrity: IntegrityDigest,
}

impl SnapshotHeader {
    pub const SIZE: usize = 4 // magic
        + 2 // format_major
        + 2 // format_minor
        + 4 // codec_revision
        + 8 // world_seed
        + 8 // completed_time
        + 32 // runtime_recipe_fingerprint
        + 2 // physical_digest_schema
        + 32 // physical_digest
        + 2 // history_digest_schema
        + 32 // history_digest
        + 2 // section_count
        + 8 // section_directory_offset
        + 32; // payload_integrity

    pub fn encode(&self, encoder: &mut LittleEndianEncoder<'_>) {
        encoder.write_fixed(&SNAPSHOT_MAGIC);
        encoder.write_u16(self.format_major);
        encoder.write_u16(self.format_minor);
        encoder.write_u32(self.codec_revision);
        encoder.write_u64(self.world_seed);
        encoder.write_u64(self.completed_time);
        encoder.write_fixed(&self.runtime_recipe_fingerprint);
        encoder.write_u16(self.physical_digest_schema);
        encoder.write_fixed(&self.physical_digest);
        encoder.write_u16(self.history_digest_schema);
        encoder.write_fixed(&self.history_digest);
        encoder.write_u16(self.section_count);
        encoder.write_u64(self.section_directory_offset);
        encoder.write_fixed(&self.payload_integrity);
    }

    pub fn decode(decoder: &mut LittleEndianDecoder<'_>) -> Result<Self, PersistenceError> {
        let magic = *decoder.read_fixed::<4>()?;
        if magic != SNAPSHOT_MAGIC {
            return Err(PersistenceError::MagicMismatch);
        }
        let format_major = decoder.read_u16()?;
        let format_minor = decoder.read_u16()?;
        if format_major != FORMAT_MAJOR_V1 {
            return Err(PersistenceError::UnsupportedMajorVersion {
                major: format_major,
            });
        }
        if format_minor > FORMAT_MINOR_V1 {
            return Err(PersistenceError::UnsupportedMinorVersion {
                major: format_major,
                minor: format_minor,
            });
        }
        let codec_revision = decoder.read_u32()?;
        let world_seed = decoder.read_u64()?;
        let completed_time = decoder.read_u64()?;
        let runtime_recipe_fingerprint = *decoder.read_fixed::<32>()?;
        let physical_digest_schema = decoder.read_u16()?;
        let physical_digest = *decoder.read_fixed::<32>()?;
        let history_digest_schema = decoder.read_u16()?;
        let history_digest = *decoder.read_fixed::<32>()?;
        let section_count = decoder.read_u16()?;
        let section_directory_offset = decoder.read_u64()?;
        let payload_integrity = *decoder.read_fixed::<32>()?;
        Ok(Self {
            format_major,
            format_minor,
            codec_revision,
            world_seed,
            completed_time,
            runtime_recipe_fingerprint,
            physical_digest_schema,
            physical_digest,
            history_digest_schema,
            history_digest,
            section_count,
            section_directory_offset,
            payload_integrity,
        })
    }
}

/// Section directory entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SectionDirectoryEntry {
    pub section_schema_id: u64,
    pub section_major: u16,
    pub section_minor: u16,
    pub flags: u32,
    pub payload_offset: u64,
    pub payload_length: u64,
    pub decoded_size_limit: u64,
    pub section_integrity: IntegrityDigest,
}

impl SectionDirectoryEntry {
    pub const SIZE: usize = 8 // section_schema_id
        + 2 // section_major
        + 2 // section_minor
        + 4 // flags
        + 8 // payload_offset
        + 8 // payload_length
        + 8 // decoded_size_limit
        + 32; // section_integrity

    pub fn encode(&self, encoder: &mut LittleEndianEncoder<'_>) {
        encoder.write_u64(self.section_schema_id);
        encoder.write_u16(self.section_major);
        encoder.write_u16(self.section_minor);
        encoder.write_u32(self.flags);
        encoder.write_u64(self.payload_offset);
        encoder.write_u64(self.payload_length);
        encoder.write_u64(self.decoded_size_limit);
        encoder.write_fixed(&self.section_integrity);
    }

    pub fn decode(decoder: &mut LittleEndianDecoder<'_>) -> Result<Self, PersistenceError> {
        let section_schema_id = decoder.read_u64()?;
        let section_major = decoder.read_u16()?;
        let section_minor = decoder.read_u16()?;
        let flags = decoder.read_u32()?;
        let payload_offset = decoder.read_u64()?;
        let payload_length = decoder.read_u64()?;
        let decoded_size_limit = decoder.read_u64()?;
        let section_integrity = *decoder.read_fixed::<32>()?;
        Ok(Self {
            section_schema_id,
            section_major,
            section_minor,
            flags,
            payload_offset,
            payload_length,
            decoded_size_limit,
            section_integrity,
        })
    }
}

/// Canonical snapshot envelope with header and ordered sections.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SnapshotEnvelope {
    pub header: SnapshotHeader,
    pub sections: BTreeMap<u64, SectionPayload>,
}

/// Opaque section payload with schema revision.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SectionPayload {
    pub section_major: u16,
    pub section_minor: u16,
    pub flags: u32,
    pub decoded_size_limit: u64,
    pub bytes: Vec<u8>,
}

impl SnapshotEnvelope {
    /// Create a new envelope from a header and section payloads.
    /// Sections are ordered by schema ID.
    pub fn new(header: SnapshotHeader, sections: BTreeMap<u64, SectionPayload>) -> Self {
        Self { header, sections }
    }

    /// Encode the complete envelope to a byte vector.
    pub fn encode(&self) -> Result<Vec<u8>, PersistenceError> {
        // First pass: encode everything except the header to compute sizes and integrity.
        let mut payload_buf = Vec::new();
        let mut payload_encoder = LittleEndianEncoder::new(&mut payload_buf);

        // Encode section payloads in schema-ID order.
        let header_size = SnapshotHeader::SIZE as u64;
        let mut entries = Vec::with_capacity(self.sections.len());
        for (&schema_id, payload) in &self.sections {
            let payload_offset = header_size + payload_encoder.written() as u64;
            payload_encoder.write_bytes(&payload.bytes);
            let payload_length = payload.bytes.len() as u64;
            let section_integrity = compute_integrity(&payload.bytes);
            entries.push(SectionDirectoryEntry {
                section_schema_id: schema_id,
                section_major: payload.section_major,
                section_minor: payload.section_minor,
                flags: payload.flags,
                payload_offset,
                payload_length,
                decoded_size_limit: payload.decoded_size_limit,
                section_integrity,
            });
        }

        // Compute payload integrity (sections only, before directory).
        // Drop encoder first to release mutable borrow.
        let sections_end = payload_encoder.written();
        drop(payload_encoder);
        let payload_integrity = compute_integrity(&payload_buf[..sections_end]);

        // Encode section directory.
        let section_directory_offset = header_size + sections_end as u64;
        let mut payload_encoder = LittleEndianEncoder::new(&mut payload_buf);
        for entry in &entries {
            entry.encode(&mut payload_encoder);
        }

        // Build final header.
        let header = SnapshotHeader {
            section_count: entries.len() as u16,
            section_directory_offset,
            payload_integrity,
            ..self.header
        };

        // Second pass: encode header + payload.
        let mut final_buf = Vec::with_capacity(SnapshotHeader::SIZE + payload_buf.len());
        let mut final_encoder = LittleEndianEncoder::new(&mut final_buf);
        header.encode(&mut final_encoder);
        final_encoder.write_bytes(&payload_buf);

        Ok(final_buf)
    }

    /// Decode and validate an envelope from bytes.
    pub fn decode(bytes: &[u8]) -> Result<Self, PersistenceError> {
        // Total size cap.
        let total_size = bytes.len() as u64;
        if total_size > MAX_TOTAL_SIZE {
            return Err(PersistenceError::TotalSizeExceeded {
                size: total_size,
                max: MAX_TOTAL_SIZE,
            });
        }

        let mut decoder = LittleEndianDecoder::new(bytes);
        let header = SnapshotHeader::decode(&mut decoder)?;

        // Section count cap.
        let section_count_u64 = u64::from(header.section_count);
        if section_count_u64 > MAX_SECTION_COUNT {
            return Err(PersistenceError::SectionCountExceeded {
                count: section_count_u64,
                max: MAX_SECTION_COUNT,
            });
        }

        // Validate payload integrity.
        let payload_start = SnapshotHeader::SIZE;
        if header.section_directory_offset < payload_start as u64 {
            return Err(PersistenceError::codec("section directory inside header"));
        }
        let payload_end = header.section_directory_offset as usize;
        if payload_end > bytes.len() {
            return Err(PersistenceError::codec("section directory offset past end"));
        }
        let payload_bytes = &bytes[payload_start..payload_end];
        let computed_payload_integrity = compute_integrity(payload_bytes);
        if computed_payload_integrity != header.payload_integrity {
            return Err(PersistenceError::PayloadIntegrityMismatch);
        }

        // Decode section directory.
        decoder.advance(payload_end - decoder.position())?;
        let mut entries = Vec::with_capacity(header.section_count as usize);
        for _ in 0..header.section_count {
            entries.push(SectionDirectoryEntry::decode(&mut decoder)?);
        }

        // Check for trailing bytes.
        if !decoder.is_empty() {
            return Err(PersistenceError::TrailingBytes);
        }

        // Validate entries: sorted, unique, non-overlapping, within bounds.
        let mut last_end: u64 = 0;
        let mut seen_ids = std::collections::BTreeSet::new();
        for entry in &entries {
            if !seen_ids.insert(entry.section_schema_id) {
                return Err(PersistenceError::DuplicateSection {
                    schema_id: entry.section_schema_id,
                });
            }
            if entry.payload_offset < payload_start as u64 {
                return Err(PersistenceError::OffsetOutOfBounds {
                    schema_id: entry.section_schema_id,
                    offset: entry.payload_offset,
                    length: bytes.len(),
                });
            }
            let end = entry
                .payload_offset
                .checked_add(entry.payload_length)
                .ok_or(PersistenceError::OverlappingSections)?;
            if end > header.section_directory_offset {
                return Err(PersistenceError::LengthOutOfBounds {
                    schema_id: entry.section_schema_id,
                    declared: entry.payload_length,
                    remaining: header.section_directory_offset - entry.payload_offset,
                });
            }
            if entry.payload_offset < last_end {
                return Err(PersistenceError::OverlappingSections);
            }
            last_end = end;

            // Validate section integrity.
            let section_bytes = &bytes[entry.payload_offset as usize..end as usize];
            let computed_section_integrity = compute_integrity(section_bytes);
            if computed_section_integrity != entry.section_integrity {
                return Err(PersistenceError::SectionIntegrityMismatch {
                    schema_id: entry.section_schema_id,
                });
            }
        }

        // Build sections map.
        let mut sections = BTreeMap::new();
        for entry in entries {
            let section_bytes = &bytes[entry.payload_offset as usize
                ..(entry.payload_offset + entry.payload_length) as usize];
            sections.insert(
                entry.section_schema_id,
                SectionPayload {
                    section_major: entry.section_major,
                    section_minor: entry.section_minor,
                    flags: entry.flags,
                    decoded_size_limit: entry.decoded_size_limit,
                    bytes: section_bytes.to_vec(),
                },
            );
        }

        Ok(Self { header, sections })
    }
}

fn compute_integrity(bytes: &[u8]) -> IntegrityDigest {
    let mut hasher = Hasher::new();
    hasher.update(bytes);
    hasher.finalize().into()
}
