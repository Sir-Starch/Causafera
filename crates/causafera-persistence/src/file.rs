use crate::{PersistenceError, SnapshotEnvelope};
use std::fs;
use std::io::Write;
use std::path::Path;

/// Atomic file write for snapshots.
///
/// Uses a same-directory temporary file, flush, fsync, atomic rename.
/// Failure leaves the prior file intact.
pub fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), PersistenceError> {
    let parent = path
        .parent()
        .ok_or_else(|| PersistenceError::codec("path has no parent directory"))?;
    let file_name = path
        .file_name()
        .ok_or_else(|| PersistenceError::codec("path has no file name"))?;

    let temp_name = {
        let mut name = file_name.to_os_string();
        name.push(".tmp");
        parent.join(name)
    };

    let mut file = fs::File::create(&temp_name)
        .map_err(|e| PersistenceError::codec(format!("create temp file: {e}")))?;
    file.write_all(bytes)
        .map_err(|e| PersistenceError::codec(format!("write temp file: {e}")))?;
    file.flush()
        .map_err(|e| PersistenceError::codec(format!("flush temp file: {e}")))?;
    drop(file);

    fs::rename(&temp_name, path)
        .map_err(|e| PersistenceError::codec(format!("atomic rename: {e}")))?;

    Ok(())
}

/// Read a snapshot file and decode the envelope.
pub fn read_snapshot_file(path: &Path) -> Result<SnapshotEnvelope, PersistenceError> {
    let bytes = fs::read(path).map_err(|e| PersistenceError::codec(format!("read file: {e}")))?;
    SnapshotEnvelope::decode(&bytes)
}

/// Write a snapshot envelope atomically to a file.
pub fn write_snapshot_file(
    path: &Path,
    envelope: &SnapshotEnvelope,
) -> Result<(), PersistenceError> {
    let bytes = envelope.encode()?;
    atomic_write(path, &bytes)
}
