pub mod checkpoint;
pub mod checkpoint_trigger;
pub mod codec;
pub mod envelope;
pub mod error;
pub mod file;
pub mod history_anchor;
pub mod history_format;
pub mod history_store;
mod history_store_reader;
pub mod snapshot;

#[cfg(test)]
mod tests;

pub use checkpoint::{
    CHECKPOINT_HEADER_FIXED_LEN, CHECKPOINT_MAGIC, CHECKPOINT_MAX_ANCHOR_FIELDS,
    CHECKPOINT_MAX_PAYLOAD_BYTES, CHECKPOINT_VERSION_MAJOR, CHECKPOINT_VERSION_MINOR,
    CheckpointEnvelope, CheckpointHeader, CheckpointMetadata, CheckpointPolicy,
    CheckpointThreshold, decode_checkpoint, decode_checkpoint_with_generations, encode_checkpoint,
    should_checkpoint,
};
pub use checkpoint_trigger::*;
pub use codec::{LittleEndianDecoder, LittleEndianEncoder};
pub use envelope::{
    FORMAT_MAJOR_V1, FORMAT_MINOR_V1, MAX_SECTION_COUNT, MAX_TOTAL_SIZE, SNAPSHOT_MAGIC,
    SectionDirectoryEntry, SectionPayload, SnapshotEnvelope, SnapshotHeader,
};
pub use error::PersistenceError;
pub use file::{atomic_write, read_snapshot_file, write_snapshot_file};
pub use history_anchor::*;
pub use history_format::*;
pub use history_store::*;
pub use history_store_reader::HistoryStoreReader;
pub use snapshot::{Snapshot, SnapshotSection};
