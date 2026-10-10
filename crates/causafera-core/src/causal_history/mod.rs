mod anchor;
mod decoder;
mod evidence;
mod reader;
mod records;

use std::ops::Range;

use causafera_types::{EventId, TraceId};
use thiserror::Error;

pub use anchor::*;
pub use decoder::*;
pub use evidence::*;
pub use reader::*;
pub use records::*;

/// One tick is one frame, so this bounds the records a single tick may commit.
/// A default nine-chunk hydrology tick commits roughly 68,000 records, and the
/// widest configurable active region is about nine times that; the bound was
/// raised from 65,536 so the plan's own nine-chunk acceptance workload fits.
/// See the 2026-10-10 entry in the `plans/causal-history-storage.md` Decision Log.
pub const MAX_HISTORY_RECORDS_PER_FRAME: usize = 524_288;
pub const MAX_CAUSES_PER_EVENT: usize = 4_096;
pub const MAX_EFFECTS_PER_EVENT: usize = 4_096;
/// Decoded size of one frame; equal to the checkpoint bound because a frame is
/// at most one tick's worth of the same state.
pub const MAX_FRAME_DECODED_BYTES: usize = 128 * 1024 * 1024;
pub const MAX_CHECKPOINT_DECODED_BYTES: usize = 128 * 1024 * 1024;
pub const MAX_QUERY_PAGE_RECORDS: usize = 256;
pub const MAX_QUERY_PAGE_BYTES: usize = 4 * 1024 * 1024;
pub const DEFAULT_CAUSAL_HOT_BUDGET_BYTES: usize = 4 * 1024 * 1024;

#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum HistoryError {
    #[error("history record count {count} exceeds maximum {max}")]
    RecordLimitExceeded { count: usize, max: usize },
    #[error("causal event has {count} causes, at most {max} are allowed")]
    CauseLimitExceeded { count: usize, max: usize },
    #[error("causal event has {count} effects, at most {max} are allowed")]
    EffectLimitExceeded { count: usize, max: usize },
    #[error("causal event must contain at least one effect")]
    NoEffects,
    #[error("causes are not strictly ordered at index {index}")]
    CausesNotStrictlyOrdered { index: usize },
    #[error("effects are not strictly ordered at index {index}")]
    EffectsNotStrictlyOrdered { index: usize },
    #[error("evidence anchors are not strictly ordered at index {index}")]
    AnchorsNotStrictlyOrdered { index: usize },
    #[error("duplicate event ID {0:?}")]
    DuplicateEventId(EventId),
    #[error("duplicate trace ID {0:?}")]
    DuplicateTraceId(TraceId),
    #[error("duplicate canonical history record key")]
    DuplicateRecordKey,
    #[error("trace {trace_id:?} refers to unknown parent {parent:?}")]
    UnknownParent { trace_id: TraceId, parent: TraceId },
    #[error("evidence refers to unknown causal anchor {anchor:?}")]
    UnknownEvidenceAnchor { anchor: TraceId },
    #[error("causal records are not in canonical order at index {index}")]
    NonCanonicalOrder { index: usize },
    #[error("decoded {kind} size {size} exceeds maximum {max}")]
    DecodedSizeLimitExceeded {
        kind: &'static str,
        size: usize,
        max: usize,
    },
    #[error("query record limit {requested} exceeds maximum {max}")]
    QueryRecordLimitExceeded { requested: usize, max: usize },
    #[error("query byte limit {requested} exceeds maximum {max}")]
    QueryByteLimitExceeded { requested: usize, max: usize },
    #[error("history arithmetic overflow")]
    ArithmeticOverflow,
    #[error("history range {offset}+{length} exceeds buffer length {total}")]
    InvalidOffset {
        offset: usize,
        length: usize,
        total: usize,
    },
    #[error("{remaining} trailing bytes remain after history payload")]
    TrailingBytes { remaining: usize },
    #[error("unsupported replay-required evidence schema {schema_id:?} {version:?}")]
    UnsupportedReplaySchema {
        schema_id: EvidenceSchemaId,
        version: EvidenceSchemaVersion,
    },
    #[error("decoder already registered for evidence schema {schema_id:?} {version:?}")]
    DuplicateDecoder {
        schema_id: EvidenceSchemaId,
        version: EvidenceSchemaVersion,
    },
    #[error("durable generation {durable} is ahead of logical generation {logical}")]
    DurableGenerationAhead { logical: u64, durable: u64 },
    #[error("history reader operation is unavailable")]
    ReaderOperationUnavailable,
    #[error("causal hot cache requires {required} bytes but its budget is {budget}")]
    HotBudgetExceeded { required: usize, budget: usize },
    #[error("hot record {trace_id:?} does not match the cold reader")]
    ColdRecordMismatch { trace_id: TraceId },
}

pub const fn validate_frame_shape(
    record_count: usize,
    decoded_bytes: usize,
) -> Result<(), HistoryError> {
    if record_count > MAX_HISTORY_RECORDS_PER_FRAME {
        return Err(HistoryError::RecordLimitExceeded {
            count: record_count,
            max: MAX_HISTORY_RECORDS_PER_FRAME,
        });
    }
    if decoded_bytes > MAX_FRAME_DECODED_BYTES {
        return Err(HistoryError::DecodedSizeLimitExceeded {
            kind: "frame",
            size: decoded_bytes,
            max: MAX_FRAME_DECODED_BYTES,
        });
    }
    Ok(())
}

pub const fn validate_checkpoint_decoded_size(decoded_bytes: usize) -> Result<(), HistoryError> {
    if decoded_bytes > MAX_CHECKPOINT_DECODED_BYTES {
        return Err(HistoryError::DecodedSizeLimitExceeded {
            kind: "checkpoint",
            size: decoded_bytes,
            max: MAX_CHECKPOINT_DECODED_BYTES,
        });
    }
    Ok(())
}

pub fn checked_history_range(
    offset: usize,
    length: usize,
    total: usize,
) -> Result<Range<usize>, HistoryError> {
    let end = offset
        .checked_add(length)
        .ok_or(HistoryError::ArithmeticOverflow)?;
    if end > total {
        return Err(HistoryError::InvalidOffset {
            offset,
            length,
            total,
        });
    }
    Ok(offset..end)
}

pub fn ensure_history_consumed(offset: usize, total: usize) -> Result<(), HistoryError> {
    if offset > total {
        return Err(HistoryError::InvalidOffset {
            offset,
            length: 0,
            total,
        });
    }
    if offset < total {
        return Err(HistoryError::TrailingBytes {
            remaining: total - offset,
        });
    }
    Ok(())
}
