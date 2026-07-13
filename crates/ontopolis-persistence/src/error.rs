use thiserror::Error;

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
}

impl PersistenceError {
    pub fn codec(message: impl Into<String>) -> Self {
        Self::Codec {
            message: message.into(),
        }
    }
}
