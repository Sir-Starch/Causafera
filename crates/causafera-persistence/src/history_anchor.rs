use causafera_core::{HistoryAnchor, HistoryAnchorFields, HistoryDurability, HistoryFrontier};
use causafera_types::{EventId, SimulationTime, TraceId};

use crate::{LittleEndianDecoder, LittleEndianEncoder, PersistenceError};

pub const HISTORY_ANCHOR_MAGIC: [u8; 4] = *b"CFHA";
pub const HISTORY_ANCHOR_VERSION_MAJOR: u16 = 1;
pub const HISTORY_ANCHOR_VERSION_MINOR: u16 = 0;
pub const HISTORY_ANCHOR_ENCODED_LEN: usize = 243;

const HISTORY_ANCHOR_DOMAIN: &[u8] = b"causafera/history-anchor/v1\0";
const HISTORY_ANCHOR_BODY_LEN: usize = HISTORY_ANCHOR_ENCODED_LEN - 32;

pub fn encode_history_anchor(anchor: &HistoryAnchor) -> Result<Vec<u8>, PersistenceError> {
    let (durability_tag, max_ticks, max_wall_time_ms) = match anchor.durability() {
        HistoryDurability::EveryTick => (0_u8, 0_u16, 0_u32),
        HistoryDurability::GroupCommit {
            max_ticks,
            max_wall_time_ms,
        } => {
            if max_ticks == 0 || max_wall_time_ms == 0 {
                return Err(PersistenceError::InvalidHistoryAnchor {
                    invariant: "group durability bounds must be nonzero",
                });
            }
            (1, max_ticks, max_wall_time_ms)
        }
    };
    if anchor.frontier().generation != anchor.logical_generation() {
        return Err(PersistenceError::InvalidHistoryAnchor {
            invariant: "frontier generation must equal logical generation",
        });
    }

    let mut bytes = Vec::with_capacity(HISTORY_ANCHOR_ENCODED_LEN);
    {
        let mut encoder = LittleEndianEncoder::new(&mut bytes);
        encoder.write_fixed(&HISTORY_ANCHOR_MAGIC);
        encoder.write_u16(HISTORY_ANCHOR_VERSION_MAJOR);
        encoder.write_u16(HISTORY_ANCHOR_VERSION_MINOR);
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
        encoder.write_u8(durability_tag);
        encoder.write_u16(max_ticks);
        encoder.write_u32(max_wall_time_ms);
    }
    let digest = history_anchor_digest(&bytes);
    bytes.extend_from_slice(&digest);
    Ok(bytes)
}

pub fn decode_history_anchor(bytes: &[u8]) -> Result<HistoryAnchor, PersistenceError> {
    if bytes.len() != HISTORY_ANCHOR_ENCODED_LEN {
        return Err(PersistenceError::HistoryAnchorLengthMismatch {
            actual: bytes.len(),
            expected: HISTORY_ANCHOR_ENCODED_LEN,
        });
    }
    let (body, encoded_digest) = bytes.split_at(HISTORY_ANCHOR_BODY_LEN);
    if history_anchor_digest(body) != encoded_digest {
        return Err(PersistenceError::HistoryAnchorDigestMismatch);
    }

    let mut decoder = LittleEndianDecoder::new(body);
    if *decoder.read_fixed::<4>()? != HISTORY_ANCHOR_MAGIC {
        return Err(PersistenceError::HistoryAnchorMagicMismatch);
    }
    let major = decoder.read_u16()?;
    let minor = decoder.read_u16()?;
    if (major, minor) != (HISTORY_ANCHOR_VERSION_MAJOR, HISTORY_ANCHOR_VERSION_MINOR) {
        return Err(PersistenceError::UnsupportedHistoryAnchorVersion { major, minor });
    }
    let namespace = *decoder.read_fixed::<16>()?;
    let logical_generation = decoder.read_u64()?;
    let durable_generation = decoder.read_u64()?;
    let checkpoint_digest = *decoder.read_fixed::<32>()?;
    let frontier = HistoryFrontier::new(
        decoder.read_u64()?,
        EventId::new(decoder.read_u64()?),
        TraceId::new(decoder.read_u64()?),
        SimulationTime::new(decoder.read_u64()?),
    );
    let logical_digest = *decoder.read_fixed::<32>()?;
    let storage_root = *decoder.read_fixed::<32>()?;
    let physical_digest_schema = decoder.read_u16()?;
    let history_digest_schema = decoder.read_u16()?;
    let evidence_manifest_digest = *decoder.read_fixed::<32>()?;
    let durability = match decoder.read_u8()? {
        0 => {
            if decoder.read_u16()? != 0 || decoder.read_u32()? != 0 {
                return Err(PersistenceError::InvalidHistoryAnchor {
                    invariant: "every-tick durability must have zero group bounds",
                });
            }
            HistoryDurability::EveryTick
        }
        1 => {
            let max_ticks = decoder.read_u16()?;
            let max_wall_time_ms = decoder.read_u32()?;
            if max_ticks == 0 || max_wall_time_ms == 0 {
                return Err(PersistenceError::InvalidHistoryAnchor {
                    invariant: "group durability bounds must be nonzero",
                });
            }
            HistoryDurability::GroupCommit {
                max_ticks,
                max_wall_time_ms,
            }
        }
        tag => return Err(PersistenceError::UnsupportedHistoryDurability { tag }),
    };
    if !decoder.is_empty() {
        return Err(PersistenceError::TrailingHistoryAnchorBytes {
            remaining: decoder.remaining(),
        });
    }
    if frontier.generation != logical_generation {
        return Err(PersistenceError::InvalidHistoryAnchor {
            invariant: "frontier generation must equal logical generation",
        });
    }
    HistoryAnchor::new(HistoryAnchorFields {
        namespace,
        logical_generation,
        durable_generation,
        checkpoint_digest,
        frontier,
        logical_digest,
        storage_root,
        physical_digest_schema,
        history_digest_schema,
        evidence_manifest_digest,
        durability,
    })
    .map_err(PersistenceError::from)
}

fn history_anchor_digest(bytes: &[u8]) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new();
    hasher.update(HISTORY_ANCHOR_DOMAIN);
    hasher.update(bytes);
    *hasher.finalize().as_bytes()
}
