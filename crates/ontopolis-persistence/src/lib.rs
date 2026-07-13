pub mod codec;
pub mod envelope;
pub mod error;
pub mod file;
pub mod snapshot;

#[cfg(test)]
mod tests;

pub use codec::{LittleEndianDecoder, LittleEndianEncoder};
pub use envelope::{
    FORMAT_MAJOR_V1, FORMAT_MINOR_V1, MAX_SECTION_COUNT, MAX_TOTAL_SIZE, SNAPSHOT_MAGIC,
    SectionDirectoryEntry, SectionPayload, SnapshotEnvelope, SnapshotHeader,
};
pub use error::PersistenceError;
pub use file::{atomic_write, read_snapshot_file, write_snapshot_file};
pub use snapshot::{Snapshot, SnapshotSection};
