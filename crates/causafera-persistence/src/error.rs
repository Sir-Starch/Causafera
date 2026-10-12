use thiserror::Error;

use causafera_core::HistoryError;

use crate::history_store::FailurePoint;

/// Errors that can occur during snapshot encoding, decoding, or validation.
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum PersistenceError {
    #[error("snapshot magic mismatch")]
    MagicMismatch,
    #[error("unsupported format major version: {major}")]
    UnsupportedMajorVersion { major: u16 },
    #[error("unsupported format minor version: {major}.{minor}")]
    UnsupportedMinorVersion { major: u16, minor: u16 },
    #[error("section schema {schema_id} required but not present")]
    MissingRequiredSection { schema_id: u64 },
    #[error("duplicate section schema {schema_id}")]
    DuplicateSection { schema_id: u64 },
    #[error("section IDs are not canonical: {previous} is followed by {current}")]
    NonCanonicalSectionOrder { previous: u64, current: u64 },
    #[error("overlapping section payloads")]
    OverlappingSections,
    #[error("trailing bytes after declared payload")]
    TrailingBytes,
    #[error("section {schema_id} payload offset {offset} exceeds buffer length {length}")]
    OffsetOutOfBounds {
        schema_id: u64,
        offset: u64,
        length: usize,
    },
    #[error(
        "section {schema_id} declared payload length {declared} exceeds remaining buffer {remaining}"
    )]
    LengthOutOfBounds {
        schema_id: u64,
        declared: u64,
        remaining: u64,
    },
    #[error("section {schema_id} decoded size limit {limit} exceeded")]
    DecodedSizeLimitExceeded { schema_id: u64, limit: u64 },
    #[error("section count {count} exceeds maximum {max}")]
    SectionCountExceeded { count: u64, max: u64 },
    #[error("total file size {size} exceeds maximum {max}")]
    TotalSizeExceeded { size: u64, max: u64 },
    #[error("integrity mismatch for section {schema_id}")]
    SectionIntegrityMismatch { schema_id: u64 },
    #[error("payload integrity mismatch")]
    PayloadIntegrityMismatch,
    #[error("invalid flags: {flags:#010x}")]
    InvalidFlags { flags: u32 },
    #[error("codec error: {message}")]
    Codec { message: String },
    #[error("invalid null-terminated string")]
    InvalidString,
    #[error("history format magic mismatch")]
    HistoryMagicMismatch,
    #[error("unsupported history format version {major}.{minor}")]
    UnsupportedHistoryFormat { major: u16, minor: u16 },
    #[error("unsupported history record tag {tag} version {major}.{minor}")]
    UnsupportedHistoryRecord { tag: u8, major: u16, minor: u16 },
    #[error("unknown history record tag {tag}")]
    UnknownHistoryRecordTag { tag: u8 },
    #[error("history frame is empty")]
    EmptyHistoryFrame,
    #[error("history frame size {size} exceeds maximum {max}")]
    HistoryFrameSizeExceeded { size: u64, max: usize },
    #[error(
        "history length at offset {offset} declares {declared} bytes with {remaining} remaining"
    )]
    MalformedHistoryLength {
        offset: usize,
        declared: u64,
        remaining: usize,
    },
    #[error("history arithmetic overflow")]
    HistoryArithmeticOverflow,
    #[error("invalid history header range for {field}: {first}..={last}")]
    InvalidHistoryRange {
        field: &'static str,
        first: u64,
        last: u64,
    },
    #[error("history frame digest mismatch")]
    FrameDigestMismatch,
    #[error("sealed history segment identity mismatch")]
    SegmentDigestMismatch,
    #[error("logical history digest mismatch")]
    LogicalDigestMismatch,
    #[error("history evidence payload digest mismatch for schema {schema_id}")]
    EvidencePayloadDigestMismatch { schema_id: u64 },
    #[error("unsupported history evidence criticality {value}")]
    UnsupportedEvidenceCriticality { value: u8 },
    #[error("unsupported scheduler phase ID {value}")]
    UnsupportedPhase { value: u64 },
    #[error("causal effect before and after fingerprints are equal")]
    UnchangedCausalEffect,
    #[error("history writer is already open for {path}")]
    HistoryWriterLocked { path: String },
    #[error("history I/O operation {operation} failed for {path}: {kind:?}")]
    HistoryIo {
        operation: &'static str,
        path: String,
        kind: std::io::ErrorKind,
    },
    #[error("SQLite catalog operation {operation} failed (code {code:?}): {message}")]
    HistoryCatalog {
        operation: &'static str,
        code: Option<i32>,
        message: String,
    },
    #[error("history catalog invariant failed: {invariant}")]
    HistoryCatalogInvariant { invariant: &'static str },
    #[error("history catalog path is unsafe: {path}")]
    HistoryCatalogPathUnsafe { path: String },
    #[error("history catalog path is missing: {path}")]
    HistoryCatalogPathMissing { path: String },
    #[error("history generation {generation} is not durable or does not exist")]
    HistoryGenerationUnavailable { generation: u64 },
    #[error("injected history-store failure at {point:?}")]
    InjectedHistoryFailure { point: FailurePoint },
    #[error("{remaining} trailing bytes remain after the history frame")]
    TrailingHistoryBytes { remaining: usize },
    #[error("checkpoint magic mismatch")]
    CheckpointMagicMismatch,
    #[error("unsupported checkpoint version {major}.{minor}")]
    UnsupportedCheckpointVersion { major: u16, minor: u16 },
    #[error("checkpoint digest mismatch")]
    CheckpointDigestMismatch,
    #[error("snapshot file does not match its registered digest")]
    SnapshotDigestMismatch,
    #[error("migration proof does not match its catalog reference")]
    MigrationProofMismatch,
    #[error("checkpoint payload length {length} exceeds maximum {max}")]
    CheckpointPayloadTooLarge { length: u64, max: u64 },
    #[error("{remaining} trailing bytes remain after the checkpoint payload")]
    TrailingCheckpointBytes { remaining: usize },
    #[error(
        "checkpoint generation mismatch: expected logical {expected_logical} durable {expected_durable}, got logical {actual_logical} durable {actual_durable}"
    )]
    CheckpointGenerationMismatch {
        expected_logical: u64,
        expected_durable: u64,
        actual_logical: u64,
        actual_durable: u64,
    },
    #[error("checkpoint codec error: {message}")]
    CheckpointCodec { message: String },
    #[error("history anchor magic mismatch")]
    HistoryAnchorMagicMismatch,
    #[error("unsupported history anchor version {major}.{minor}")]
    UnsupportedHistoryAnchorVersion { major: u16, minor: u16 },
    #[error("history anchor digest mismatch")]
    HistoryAnchorDigestMismatch,
    #[error("history anchor length {actual} does not equal {expected}")]
    HistoryAnchorLengthMismatch { actual: usize, expected: usize },
    #[error("unsupported history durability tag {tag}")]
    UnsupportedHistoryDurability { tag: u8 },
    #[error("invalid history anchor: {invariant}")]
    InvalidHistoryAnchor { invariant: &'static str },
    #[error("{remaining} trailing bytes remain after the history anchor")]
    TrailingHistoryAnchorBytes { remaining: usize },
    #[error(transparent)]
    History(#[from] HistoryError),
}

impl PersistenceError {
    pub fn codec(message: impl Into<String>) -> Self {
        Self::Codec {
            message: message.into(),
        }
    }
}
