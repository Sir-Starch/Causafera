//! The frames an earlier writer flushed after its last catalog commit.
//!
//! A group-commit writer appends each tick's frame to the active segment as the
//! tick commits and publishes them to the catalog only when the group is forced.
//! A writer that dies in between leaves complete, valid frames past the
//! catalog's durable end. They are never visible history and are never promoted,
//! but they are exactly the ticks the crash lost, so opening a bundle counts
//! them instead of letting the loss go unreported.
//!
//! A frame counts only if it continues the committed history: the next
//! generation, the next record sequence, and a `prior_logical_history_digest`
//! equal to the fold of everything before it. That last check is what keeps the
//! leftover of an earlier, longer tail from being mistaken for the loss of the
//! current one when a later writer has overwritten only the start of it.

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

use causafera_core::EvidenceDecoderRegistry;

use crate::PersistenceError;
use crate::history_format::{HISTORY_FRAME_HEADER_LEN, LogicalHistoryDigest, decode_history_frame};
use crate::history_store::MAX_STORED_FRAME_BYTES;

/// The lost logical ticks found after a bundle's last committed generation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct UnpublishedTail {
    /// Complete frames that continue the committed history.
    pub frames: u64,
    /// Records in those frames.
    pub records: u64,
    /// Generation of the first and last such frame.
    pub first_generation: Option<u64>,
    pub last_generation: Option<u64>,
    /// Bytes after the last counted frame that do not continue the history: a
    /// torn write, or what an earlier writer left behind.
    pub unaccounted_bytes: u64,
}

/// The committed history a tail has to continue.
pub(crate) struct TailChain {
    pub namespace: [u8; 32],
    pub durable_generation: u64,
    pub durable_records: u64,
    pub hasher: LogicalHistoryDigest,
}

const PAYLOAD_LENGTH_OFFSET: usize = 128;

pub(crate) fn scan_unpublished_tail(
    path: &Path,
    durable_end: u64,
    registry: &EvidenceDecoderRegistry,
    mut chain: TailChain,
) -> Result<UnpublishedTail, PersistenceError> {
    let io_error = |operation: &'static str, error: std::io::Error| PersistenceError::HistoryIo {
        operation,
        path: path.display().to_string(),
        kind: error.kind(),
    };
    let mut file = File::open(path).map_err(|error| io_error("open active segment tail", error))?;
    let length = file
        .metadata()
        .map_err(|error| io_error("measure active segment tail", error))?
        .len();
    let mut tail = UnpublishedTail::default();
    let mut position = durable_end;
    let mut expected_generation = chain.durable_generation;
    let mut records_so_far = chain.durable_records;
    let header_len = u64::try_from(HISTORY_FRAME_HEADER_LEN)?;
    while position
        .checked_add(header_len)
        .is_some_and(|header_end| header_end <= length)
    {
        let mut header = vec![0_u8; HISTORY_FRAME_HEADER_LEN];
        file.seek(SeekFrom::Start(position))
            .and_then(|_| file.read_exact(&mut header))
            .map_err(|error| io_error("read tail frame header", error))?;
        let payload_length = u64::from_le_bytes(
            header[PAYLOAD_LENGTH_OFFSET..PAYLOAD_LENGTH_OFFSET + 8]
                .try_into()
                .map_err(|_| PersistenceError::HistoryArithmeticOverflow)?,
        );
        let Some(total) = header_len.checked_add(payload_length) else {
            break;
        };
        if total > MAX_STORED_FRAME_BYTES || position.saturating_add(total) > length {
            break;
        }
        let mut frame = vec![0_u8; usize::try_from(total)?];
        file.seek(SeekFrom::Start(position))
            .and_then(|_| file.read_exact(&mut frame))
            .map_err(|error| io_error("read tail frame", error))?;
        let Ok(decoded) = decode_history_frame(&frame, registry) else {
            break;
        };
        let fields = &decoded.header().fields;
        expected_generation = expected_generation
            .checked_add(1)
            .ok_or(PersistenceError::HistoryArithmeticOverflow)?;
        if fields.namespace != chain.namespace
            || fields.catalog_generation != expected_generation
            || fields.first_record_sequence != records_so_far.saturating_add(1)
            || fields.prior_logical_history_digest != chain.hasher.finish()
        {
            break;
        }
        for record in decoded.records() {
            chain.hasher.append(record)?;
        }
        let count = u64::try_from(decoded.records().len())?;
        records_so_far = records_so_far
            .checked_add(count)
            .ok_or(PersistenceError::HistoryArithmeticOverflow)?;
        tail.frames += 1;
        tail.records += count;
        tail.first_generation.get_or_insert(expected_generation);
        tail.last_generation = Some(expected_generation);
        position += total;
    }
    tail.unaccounted_bytes = length.saturating_sub(position);
    Ok(tail)
}
