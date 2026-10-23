//! Canonical checkpoint storage format version 1.
//!
//! A checkpoint captures the immutable anchor fields that name a durable
//! simulation boundary, followed by a bounded opaque current-state payload. The
//! on-disk layout is self-describing and integrity-checked: magic, version,
//! canonical little-endian anchor fields, payload length, a 32-byte BLAKE3 digest
//! over the domain-separated header and payload, and the payload itself.

use causafera_core::{HistoryAnchor, HistoryDurability, HistoryFrontier};
use causafera_types::{EventId, SimulationTime, TraceId};

use crate::{LittleEndianDecoder, LittleEndianEncoder, PersistenceError};

pub const CHECKPOINT_MAGIC: [u8; 4] = *b"CFCK";
pub const CHECKPOINT_VERSION_MAJOR: u16 = 1;
pub const CHECKPOINT_VERSION_MINOR: u16 = 0;

/// Maximum number of canonical anchor field groups in one checkpoint.
pub const CHECKPOINT_MAX_ANCHOR_FIELDS: usize = 4096;

/// Maximum decoded current-state payload bytes in one checkpoint.
pub const CHECKPOINT_MAX_PAYLOAD_BYTES: usize = 67_108_864;

const CHECKPOINT_DOMAIN: &[u8] = b"causafera/checkpoint/v1\0";

/// Header field count is encoded as a small integer to keep the format
/// self-describing while the rest of the header remains fixed-width.
pub const CHECKPOINT_HEADER_FIXED_LEN: usize = 4 // magic
    + 2 // major
    + 2 // minor
    + 2 // anchor_field_count
    + 16 // namespace
    + 8 // logical_generation
    + 8 // durable_generation
    + 32 // checkpoint_digest
    + 8 // frontier.generation
    + 8 // frontier.event_id
    + 8 // frontier.trace_id
    + 8 // frontier.time
    + 32 // logical_digest
    + 32 // storage_root
    + 2 // physical_digest_schema
    + 2 // history_digest_schema
    + 32 // evidence_manifest_digest
    + 32 // runtime_recipe_fingerprint
    + 32 // input_fingerprint
    + 32 // physical_digest
    + 8 // completed_time
    + 1 // durability tag
    + 2 // group max ticks
    + 4 // group max wall time ms
    + 8; // payload_length

const CHECKPOINT_DIGEST_LEN: usize = 32;

/// Canonical header decoded from a checkpoint envelope.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheckpointHeader {
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
    pub runtime_recipe_fingerprint: [u8; 32],
    pub input_fingerprint: [u8; 32],
    pub physical_digest: [u8; 32],
    pub completed_time: SimulationTime,
    pub durability: HistoryDurability,
    pub payload_length: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CheckpointMetadata {
    pub runtime_recipe_fingerprint: [u8; 32],
    pub input_fingerprint: [u8; 32],
    pub physical_digest: [u8; 32],
    pub completed_time: SimulationTime,
}

/// A decoded checkpoint envelope with verified integrity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheckpointEnvelope {
    pub header: CheckpointHeader,
    pub payload: Vec<u8>,
    pub digest: [u8; 32],
}

/// Threshold rule that decides whether a checkpoint should be produced.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CheckpointThreshold {
    /// Every tick is eligible.
    EveryTick,
    /// Eligible once at least this many ticks have passed.
    Ticks(u64),
    /// Eligible once at least this many bytes have been written.
    Bytes(u64),
    /// Eligible when either the tick or byte threshold is exceeded.
    Coordinator { ticks: u64, bytes: u64 },
}

/// Checkpoint policy with a threshold coordinator and a force override.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CheckpointPolicy {
    pub threshold: CheckpointThreshold,
    pub force: bool,
}

/// Decide whether a checkpoint should be produced given the distance since the
/// last checkpoint and the configured policy.
///
/// `force` always returns true; otherwise the threshold coordinator fires when
/// any of its configured bounds are met.
pub fn should_checkpoint(
    ticks_since_last: u64,
    bytes_since_last: u64,
    policy: &CheckpointPolicy,
) -> bool {
    if policy.force {
        return true;
    }
    match policy.threshold {
        CheckpointThreshold::EveryTick => true,
        CheckpointThreshold::Ticks(ticks) => ticks_since_last >= ticks,
        CheckpointThreshold::Bytes(bytes) => bytes_since_last >= bytes,
        CheckpointThreshold::Coordinator { ticks, bytes } => {
            ticks_since_last >= ticks || bytes_since_last >= bytes
        }
    }
}

/// Encode a checkpoint from the immutable anchor fields and a current-state
/// payload.
///
/// The payload is opaque to this codec and is bounded by
/// [`CHECKPOINT_MAX_PAYLOAD_BYTES`]. The returned bytes are deterministic for
/// the same inputs.
pub fn encode_checkpoint(
    anchor: &HistoryAnchor,
    metadata: &CheckpointMetadata,
    payload: &[u8],
) -> Result<Vec<u8>, PersistenceError> {
    let payload_len =
        u64::try_from(payload.len()).map_err(|_| PersistenceError::CheckpointPayloadTooLarge {
            length: payload.len() as u64,
            max: CHECKPOINT_MAX_PAYLOAD_BYTES as u64,
        })?;
    if payload.len() > CHECKPOINT_MAX_PAYLOAD_BYTES {
        return Err(PersistenceError::CheckpointPayloadTooLarge {
            length: payload_len,
            max: CHECKPOINT_MAX_PAYLOAD_BYTES as u64,
        });
    }
    if anchor.frontier().generation != anchor.logical_generation() {
        return Err(PersistenceError::CheckpointCodec {
            message: "checkpoint frontier generation does not match logical generation".into(),
        });
    }
    if metadata.completed_time != anchor.frontier().time {
        return Err(PersistenceError::CheckpointCodec {
            message: "checkpoint completed time does not match frontier time".into(),
        });
    }
    if anchor.physical_digest_schema() != crate::CURRENT_DIGEST_SCHEMA_VERSION
        || anchor.history_digest_schema() != crate::CURRENT_DIGEST_SCHEMA_VERSION
    {
        return Err(PersistenceError::CheckpointCodec {
            message: "checkpoint requires physical and history digest schema 9".into(),
        });
    }
    let (durability_tag, max_ticks, max_wall_time_ms) = match anchor.durability() {
        HistoryDurability::EveryTick => (0_u8, 0_u16, 0_u32),
        HistoryDurability::GroupCommit {
            max_ticks,
            max_wall_time_ms,
        } if max_ticks > 0 && max_wall_time_ms > 0 => (1, max_ticks, max_wall_time_ms),
        HistoryDurability::GroupCommit { .. } => {
            return Err(PersistenceError::CheckpointCodec {
                message: "checkpoint group durability bounds must be nonzero".into(),
            });
        }
    };

    let mut header = Vec::with_capacity(CHECKPOINT_HEADER_FIXED_LEN);
    {
        let mut encoder = LittleEndianEncoder::new(&mut header);
        encoder.write_fixed(&CHECKPOINT_MAGIC);
        encoder.write_u16(CHECKPOINT_VERSION_MAJOR);
        encoder.write_u16(CHECKPOINT_VERSION_MINOR);
        encoder.write_u16(1); // anchor_field_count
        encoder.write_fixed(anchor.namespace());
        encoder.write_u64(anchor.logical_generation());
        encoder.write_u64(anchor.durable_generation());
        encoder.write_fixed(anchor.checkpoint_digest());
        let frontier = anchor.frontier();
        encoder.write_u64(frontier.generation);
        encoder.write_u64(frontier.event_id.raw());
        encoder.write_u64(frontier.trace_id.raw());
        encoder.write_u64(frontier.time.raw());
        encoder.write_fixed(anchor.logical_digest());
        encoder.write_fixed(anchor.storage_root());
        encoder.write_u16(anchor.physical_digest_schema());
        encoder.write_u16(anchor.history_digest_schema());
        encoder.write_fixed(anchor.evidence_manifest_digest());
        encoder.write_fixed(&metadata.runtime_recipe_fingerprint);
        encoder.write_fixed(&metadata.input_fingerprint);
        encoder.write_fixed(&metadata.physical_digest);
        encoder.write_u64(metadata.completed_time.raw());
        encoder.write_u8(durability_tag);
        encoder.write_u16(max_ticks);
        encoder.write_u32(max_wall_time_ms);
        encoder.write_u64(payload_len);
    }
    debug_assert_eq!(header.len(), CHECKPOINT_HEADER_FIXED_LEN);

    let digest = checkpoint_digest(&header, payload);

    let total = CHECKPOINT_HEADER_FIXED_LEN
        .checked_add(CHECKPOINT_DIGEST_LEN)
        .and_then(|len| len.checked_add(payload.len()))
        .ok_or_else(|| PersistenceError::CheckpointCodec {
            message: "checkpoint total size overflow".into(),
        })?;
    let mut bytes = Vec::with_capacity(total);
    bytes.extend_from_slice(&header);
    bytes.extend_from_slice(&digest);
    bytes.extend_from_slice(payload);
    Ok(bytes)
}

/// Decode a checkpoint and verify its integrity and bounds.
///
/// Rejects malformed, truncated, oversized, corrupt, or trailing data.
pub fn decode_checkpoint(bytes: &[u8]) -> Result<CheckpointEnvelope, PersistenceError> {
    if bytes.len() < CHECKPOINT_HEADER_FIXED_LEN + CHECKPOINT_DIGEST_LEN {
        return Err(PersistenceError::CheckpointCodec {
            message: format!(
                "checkpoint buffer {} is shorter than the minimum {} bytes",
                bytes.len(),
                CHECKPOINT_HEADER_FIXED_LEN + CHECKPOINT_DIGEST_LEN
            ),
        });
    }

    let mut cursor = LittleEndianDecoder::new(bytes);
    let magic = *cursor.read_fixed::<4>()?;
    if magic != CHECKPOINT_MAGIC {
        return Err(PersistenceError::CheckpointMagicMismatch);
    }
    let major = cursor.read_u16()?;
    let minor = cursor.read_u16()?;
    if (major, minor) != (CHECKPOINT_VERSION_MAJOR, CHECKPOINT_VERSION_MINOR) {
        return Err(PersistenceError::UnsupportedCheckpointVersion { major, minor });
    }
    let anchor_field_count = cursor.read_u16()?;
    if anchor_field_count != 1 {
        return Err(PersistenceError::CheckpointCodec {
            message: format!(
                "checkpoint must contain exactly one anchor, got {anchor_field_count}"
            ),
        });
    }

    let namespace = *cursor.read_fixed::<16>()?;
    let logical_generation = cursor.read_u64()?;
    let durable_generation = cursor.read_u64()?;
    let checkpoint_digest_bytes = *cursor.read_fixed::<32>()?;
    let frontier_generation = cursor.read_u64()?;
    let frontier_event_id = EventId::new(cursor.read_u64()?);
    let frontier_trace_id = TraceId::new(cursor.read_u64()?);
    let frontier_time = SimulationTime::new(cursor.read_u64()?);
    let logical_digest = *cursor.read_fixed::<32>()?;
    let storage_root = *cursor.read_fixed::<32>()?;
    let physical_digest_schema = cursor.read_u16()?;
    let history_digest_schema = cursor.read_u16()?;
    let evidence_manifest_digest = *cursor.read_fixed::<32>()?;
    let runtime_recipe_fingerprint = *cursor.read_fixed::<32>()?;
    let input_fingerprint = *cursor.read_fixed::<32>()?;
    let physical_digest = *cursor.read_fixed::<32>()?;
    let completed_time = SimulationTime::new(cursor.read_u64()?);
    let durability = match cursor.read_u8()? {
        0 => {
            if cursor.read_u16()? != 0 || cursor.read_u32()? != 0 {
                return Err(PersistenceError::CheckpointCodec {
                    message: "every-tick checkpoint has nonzero group bounds".into(),
                });
            }
            HistoryDurability::EveryTick
        }
        1 => {
            let max_ticks = cursor.read_u16()?;
            let max_wall_time_ms = cursor.read_u32()?;
            if max_ticks == 0 || max_wall_time_ms == 0 {
                return Err(PersistenceError::CheckpointCodec {
                    message: "group checkpoint has zero durability bound".into(),
                });
            }
            HistoryDurability::GroupCommit {
                max_ticks,
                max_wall_time_ms,
            }
        }
        tag => {
            return Err(PersistenceError::CheckpointCodec {
                message: format!("unsupported checkpoint durability tag {tag}"),
            });
        }
    };
    let payload_length = cursor.read_u64()?;

    if payload_length > CHECKPOINT_MAX_PAYLOAD_BYTES as u64 {
        return Err(PersistenceError::CheckpointPayloadTooLarge {
            length: payload_length,
            max: CHECKPOINT_MAX_PAYLOAD_BYTES as u64,
        });
    }
    let payload_length =
        usize::try_from(payload_length).map_err(|_| PersistenceError::CheckpointCodec {
            message: "checkpoint payload length does not fit usize".into(),
        })?;

    // Header must have been consumed exactly before the digest field.
    if cursor.position() != CHECKPOINT_HEADER_FIXED_LEN {
        return Err(PersistenceError::CheckpointCodec {
            message: "checkpoint header length mismatch after parsing".into(),
        });
    }

    let expected_digest = *cursor.read_fixed::<32>()?;
    let payload = cursor.read_bytes(payload_length)?.to_vec();

    if !cursor.is_empty() {
        return Err(PersistenceError::TrailingCheckpointBytes {
            remaining: cursor.remaining(),
        });
    }

    let header = &bytes[..CHECKPOINT_HEADER_FIXED_LEN];
    let actual_digest = checkpoint_digest(header, &payload);
    if actual_digest != expected_digest {
        return Err(PersistenceError::CheckpointDigestMismatch);
    }

    if durable_generation > logical_generation {
        return Err(PersistenceError::CheckpointGenerationMismatch {
            expected_logical: logical_generation,
            expected_durable: logical_generation,
            actual_logical: logical_generation,
            actual_durable: durable_generation,
        });
    }
    if frontier_generation != logical_generation || completed_time != frontier_time {
        return Err(PersistenceError::CheckpointCodec {
            message: "checkpoint frontier does not match generation/completed time".into(),
        });
    }
    if physical_digest_schema != crate::CURRENT_DIGEST_SCHEMA_VERSION
        || history_digest_schema != crate::CURRENT_DIGEST_SCHEMA_VERSION
    {
        return Err(PersistenceError::CheckpointCodec {
            message: "checkpoint requires physical and history digest schema 9".into(),
        });
    }

    Ok(CheckpointEnvelope {
        header: CheckpointHeader {
            namespace,
            logical_generation,
            durable_generation,
            checkpoint_digest: checkpoint_digest_bytes,
            frontier: HistoryFrontier::new(
                frontier_generation,
                frontier_event_id,
                frontier_trace_id,
                frontier_time,
            ),
            logical_digest,
            storage_root,
            physical_digest_schema,
            history_digest_schema,
            evidence_manifest_digest,
            runtime_recipe_fingerprint,
            input_fingerprint,
            physical_digest,
            completed_time,
            durability,
            payload_length: payload.len() as u64,
        },
        payload,
        digest: expected_digest,
    })
}

/// Decode a checkpoint and additionally verify the expected logical and durable
/// generations.
///
/// This is the form used by a storage coordinator that already knows which
/// generation boundary it is asking for.
pub fn decode_checkpoint_with_generations(
    bytes: &[u8],
    logical_generation: u64,
    durable_generation: u64,
) -> Result<CheckpointEnvelope, PersistenceError> {
    let envelope = decode_checkpoint(bytes)?;
    if envelope.header.logical_generation != logical_generation
        || envelope.header.durable_generation != durable_generation
    {
        return Err(PersistenceError::CheckpointGenerationMismatch {
            expected_logical: logical_generation,
            expected_durable: durable_generation,
            actual_logical: envelope.header.logical_generation,
            actual_durable: envelope.header.durable_generation,
        });
    }
    Ok(envelope)
}

fn checkpoint_digest(header: &[u8], payload: &[u8]) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new();
    hasher.update(CHECKPOINT_DOMAIN);
    hasher.update(header);
    hasher.update(payload);
    *hasher.finalize().as_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;
    use causafera_core::{HistoryAnchor, HistoryAnchorFields, HistoryDurability, HistoryFrontier};
    use causafera_types::{EventId, SimulationTime, TraceId};

    fn sample_anchor() -> HistoryAnchor {
        HistoryAnchor::new(HistoryAnchorFields {
            namespace: [0xAB; 16],
            logical_generation: 7,
            durable_generation: 5,
            checkpoint_digest: [0x11; 32],
            frontier: HistoryFrontier::new(
                7,
                EventId::new(42),
                TraceId::new(100),
                SimulationTime::new(1234),
            ),
            logical_digest: [0x22; 32],
            storage_root: [0x33; 32],
            physical_digest_schema: 9,
            history_digest_schema: 9,
            evidence_manifest_digest: [0x44; 32],
            durability: HistoryDurability::GroupCommit {
                max_ticks: 8,
                max_wall_time_ms: 1000,
            },
        })
        .unwrap()
    }

    fn sample_metadata() -> CheckpointMetadata {
        CheckpointMetadata {
            runtime_recipe_fingerprint: [0x55; 32],
            input_fingerprint: [0x66; 32],
            physical_digest: [0x77; 32],
            completed_time: SimulationTime::new(1234),
        }
    }

    #[test]
    fn encode_decode_round_trip() {
        let anchor = sample_anchor();
        let payload = b"unit-test payload".to_vec();
        let encoded = encode_checkpoint(&anchor, &sample_metadata(), &payload).unwrap();
        let decoded = decode_checkpoint(&encoded).unwrap();
        assert_eq!(decoded.payload, payload);
        assert_eq!(
            decoded.header.logical_generation,
            anchor.logical_generation()
        );
        assert_eq!(
            decoded.header.durable_generation,
            anchor.durable_generation()
        );
    }

    #[test]
    fn oversized_payload_is_rejected_at_encode() {
        let anchor = sample_anchor();
        let payload = vec![0u8; CHECKPOINT_MAX_PAYLOAD_BYTES + 1];
        assert!(matches!(
            encode_checkpoint(&anchor, &sample_metadata(), &payload),
            Err(PersistenceError::CheckpointPayloadTooLarge { .. })
        ));
    }

    #[test]
    fn corrupt_digest_is_rejected() {
        let anchor = sample_anchor();
        let payload = b"unit-test payload".to_vec();
        let mut encoded = encode_checkpoint(&anchor, &sample_metadata(), &payload).unwrap();
        let last = encoded.len() - 1;
        encoded[last] ^= 0xFF;
        assert!(matches!(
            decode_checkpoint(&encoded),
            Err(PersistenceError::CheckpointDigestMismatch)
        ));
    }

    #[test]
    fn trailing_bytes_are_rejected() {
        let anchor = sample_anchor();
        let payload = b"unit-test payload".to_vec();
        let mut encoded = encode_checkpoint(&anchor, &sample_metadata(), &payload).unwrap();
        encoded.push(0xFF);
        assert!(matches!(
            decode_checkpoint(&encoded),
            Err(PersistenceError::TrailingCheckpointBytes { .. })
        ));
    }

    #[test]
    fn decode_with_generations_rejects_mismatch() {
        let anchor = sample_anchor();
        let payload = b"unit-test payload".to_vec();
        let encoded = encode_checkpoint(&anchor, &sample_metadata(), &payload).unwrap();
        assert!(matches!(
            decode_checkpoint_with_generations(&encoded, 99, 99),
            Err(PersistenceError::CheckpointGenerationMismatch { .. })
        ));
    }

    #[test]
    fn policy_force_overrides_threshold() {
        let policy = CheckpointPolicy {
            threshold: CheckpointThreshold::Coordinator {
                ticks: 100,
                bytes: 100,
            },
            force: true,
        };
        assert!(should_checkpoint(0, 0, &policy));
    }

    #[test]
    fn coordinator_threshold_or_logic() {
        let policy = CheckpointPolicy {
            threshold: CheckpointThreshold::Coordinator {
                ticks: 10,
                bytes: 1024,
            },
            force: false,
        };
        assert!(!should_checkpoint(0, 0, &policy));
        assert!(should_checkpoint(10, 0, &policy));
        assert!(should_checkpoint(0, 1024, &policy));
        assert!(should_checkpoint(11, 1025, &policy));
    }
}
