//! A durable bundle as a session's history: create, save, open, shut down.
//!
//! A bundle is a directory holding the whole causal and evidence history of a
//! session (`catalog.sqlite`, `segments/`, `checkpoints/`, `snapshots/`). It
//! exists before the first tick, is never created over anything that is already
//! there, and is the only thing a later session needs to continue the run.
//!
//! - [`Runtime::create_bundle`] builds the world over a new bundle and saves its
//!   genesis, so the complete layout and a checkpoint exist before tick one.
//! - [`Runtime::save_bundle`] forces the pending group to the catalog, then
//!   publishes a checkpoint and a portable snapshot of that durable frontier.
//! - [`Runtime::open_bundle`] verifies a bundle and continues from its latest
//!   checkpoint, reporting the ticks a crash lost.
//! - Dropping a bundle session is a clean shutdown and forces the group; a
//!   session that must not (a simulated crash) says so with
//!   [`Runtime::simulate_crash`].
//!
//! See `plans/causal-history-storage.md` Todo 8.

use std::path::{Path, PathBuf};

use causafera_persistence::{
    CHECKPOINT_DURABLE_FRAME_BYTES, CHECKPOINT_DURABLE_TICKS, CheckpointEnvelope,
    CheckpointMetadata, DurabilityMode, HistoryStore, PersistenceError, SnapshotEnvelope,
};
use causafera_types::SimulationTime;

use crate::history_transaction::{
    HistoryTarget, ReplayPlan, RuntimeHistoryCoordinator, runtime_history_decoder_registry,
};
use crate::{
    BoundedSnapshotMetadata, Runtime, RuntimeConfig, RuntimeError, RuntimeState,
    assemble_bounded_envelope, history_anchor_from_bounded_envelope, runtime_recipe_fingerprint,
};

/// How far the durable history may run past its latest checkpoint before the
/// next quiescent durable frontier is checkpointed, which bounds how many ticks
/// reopening a crashed bundle has to re-execute.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CheckpointCadence {
    /// Durably published ticks since the latest checkpoint.
    pub ticks: u64,
    /// Canonical frame bytes durably published since the latest checkpoint.
    pub frame_bytes: u64,
}

impl Default for CheckpointCadence {
    fn default() -> Self {
        Self {
            ticks: CHECKPOINT_DURABLE_TICKS,
            frame_bytes: CHECKPOINT_DURABLE_FRAME_BYTES,
        }
    }
}

/// How a new bundle makes its history durable.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BundleOptions {
    /// `GroupCommit` (the default: at most 32 ticks or 250 ms behind) or
    /// `EveryTick` (power-loss durable after every tick).
    pub durability: DurabilityMode,
    /// When to checkpoint without being asked.
    pub cadence: CheckpointCadence,
}

impl Default for BundleOptions {
    fn default() -> Self {
        Self {
            durability: DurabilityMode::GroupCommit,
            cadence: CheckpointCadence::default(),
        }
    }
}

/// What a save published.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BundleSave {
    /// The durable generation the checkpoint and snapshot are anchored at.
    pub generation: u64,
    pub completed_time: SimulationTime,
    pub checkpoint_digest: [u8; 32],
    pub snapshot_digest: [u8; 32],
    pub snapshot_path: PathBuf,
}

/// Logical ticks a crash lost: flushed by the dead writer, never published.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LostLogicalRange {
    pub first_generation: u64,
    pub last_generation: u64,
    pub ticks: u64,
    pub records: u64,
}

/// What opening a bundle found and did.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BundleRecovery {
    /// The durable generation the session resumed at: the bundle's head.
    pub generation: u64,
    /// The generation of the checkpoint the session was restored from.
    pub checkpoint_generation: u64,
    /// Ticks re-executed, and checked against the stored frames, to get from
    /// that checkpoint to the head.
    pub replayed_ticks: u64,
    pub completed_time: SimulationTime,
    /// The ticks a crash lost after that generation, if any.
    pub lost: Option<LostLogicalRange>,
    /// Bytes after the committed history that belong to no counted tick.
    pub unaccounted_bytes: u64,
}

fn bundle_io(operation: &'static str, error: &std::io::Error) -> RuntimeError {
    RuntimeError::SnapshotAssembly(PersistenceError::codec(format!("{operation}: {error}")))
}

/// Whether `root` already existed (as an empty directory), refusing anything
/// else so that no bundle is ever created over existing data.
fn require_fresh_root(root: &Path) -> Result<bool, RuntimeError> {
    match std::fs::metadata(root) {
        Ok(metadata) => {
            let empty = metadata.is_dir()
                && std::fs::read_dir(root)
                    .map_err(|error| bundle_io("read bundle directory", &error))?
                    .next()
                    .is_none();
            if empty {
                Ok(true)
            } else {
                Err(RuntimeError::BundleExists {
                    path: root.display().to_string(),
                })
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(bundle_io("inspect bundle path", &error)),
    }
}

/// Remove what a failed `create_bundle` left behind, and only that: the
/// directory it made, or the contents of the empty directory it was given.
fn discard_failed_bundle(root: &Path, existed_empty: bool) {
    if existed_empty {
        if let Ok(entries) = std::fs::read_dir(root) {
            for entry in entries.flatten() {
                let path = entry.path();
                let _ = if path.is_dir() {
                    std::fs::remove_dir_all(&path)
                } else {
                    std::fs::remove_file(&path)
                };
            }
        }
    } else {
        let _ = std::fs::remove_dir_all(root);
    }
}

impl RuntimeState {
    /// The bounded snapshot of the current durable frontier and the metadata
    /// that describes it. The pending group must already be forced.
    pub(crate) fn bounded_snapshot_bytes(
        &self,
    ) -> Result<(Vec<u8>, BoundedSnapshotMetadata, SimulationTime), RuntimeError> {
        let data = self.export_snapshot();
        let anchor = self.history.reader().anchor()?;
        let metadata = BoundedSnapshotMetadata {
            runtime_recipe_fingerprint: runtime_recipe_fingerprint(&self.config),
            physical_digest: self
                .physical_state_digest(data.recipe.completed_time)
                .bytes(),
            anchor,
        };
        let bytes = assemble_bounded_envelope(&data, &metadata)?.encode()?;
        Ok((bytes, metadata, data.recipe.completed_time))
    }
}

impl RuntimeState {
    /// Checkpoint the durable frontier if the history has run far enough past
    /// the last checkpoint.
    ///
    /// Called when a tick has just been published. A failure is recorded rather
    /// than returned: the tick is already durable history, and a missed
    /// checkpoint only lengthens the next replay.
    pub(crate) fn checkpoint_if_due(&mut self) {
        if !self.history.checkpoint_due() {
            return;
        }
        let outcome =
            self.bounded_snapshot_bytes()
                .and_then(|(bytes, metadata, completed_time)| {
                    let checkpoint = CheckpointMetadata {
                        runtime_recipe_fingerprint: metadata.runtime_recipe_fingerprint,
                        input_fingerprint: [0; 32],
                        physical_digest: metadata.physical_digest,
                        completed_time,
                    };
                    self.history
                        .checkpoint_only(&bytes, &metadata.anchor, &checkpoint)
                });
        if let Err(error) = outcome {
            self.history.note_checkpoint_failure(error);
        }
    }
}

impl Runtime {
    /// Build `config`'s world over a new bundle at `root`.
    ///
    /// `root` must not exist or must be an empty directory; nothing is ever
    /// created over existing data. The bundle holds a checkpoint and a snapshot
    /// of tick zero before this returns, so the layout is complete before the
    /// first tick.
    pub fn create_bundle(
        config: RuntimeConfig,
        root: &Path,
        options: BundleOptions,
    ) -> Result<Self, RuntimeError> {
        let existed_empty = require_fresh_root(root)?;
        let target = HistoryTarget::Bundle {
            root: root.to_path_buf(),
            mode: options.durability,
        };
        let created = Self::with_history(config, &target).and_then(|mut runtime| {
            runtime.lock_state()?.history.set_cadence(options.cadence);
            runtime.save_bundle()?;
            Ok(runtime)
        });
        if created.is_err() {
            discard_failed_bundle(root, existed_empty);
        }
        created
    }

    /// Force the pending group, then publish a checkpoint and a portable
    /// snapshot of the durable frontier.
    ///
    /// Saving twice with no tick between publishes nothing new the second time.
    pub fn save_bundle(&mut self) -> Result<BundleSave, RuntimeError> {
        let mut state = self.lock_state()?;
        if let Some(error) = state.failure.clone() {
            return Err(error);
        }
        if state.advanced_through != self.scheduler.current_time() {
            return Err(RuntimeError::PhaseDesynchronized);
        }
        if state.history.bundle_root().is_none() {
            return Err(RuntimeError::NotABundle);
        }
        state.history.flush_durable()?;
        let (bytes, metadata, completed_time) = state.bounded_snapshot_bytes()?;
        let checkpoint = CheckpointMetadata {
            runtime_recipe_fingerprint: metadata.runtime_recipe_fingerprint,
            // No runtime producer of recorded external inputs exists yet.
            input_fingerprint: [0; 32],
            physical_digest: metadata.physical_digest,
            completed_time,
        };
        let receipt = state.history.save(&bytes, &metadata.anchor, &checkpoint)?;
        Ok(BundleSave {
            generation: receipt.generation,
            completed_time,
            checkpoint_digest: receipt.checkpoint_digest,
            snapshot_digest: receipt.snapshot_digest,
            snapshot_path: receipt.snapshot_path,
        })
    }

    /// Verify the bundle at `root` and continue from where it ended.
    ///
    /// The session is restored from the latest checkpoint. If the bundle's
    /// durable history runs past that checkpoint, the ticks after it are
    /// re-executed and every one is checked against the frame the bundle stored,
    /// so the session that comes back is the one that was lost, or the open
    /// fails. Frames a crash had flushed but not published are reported, not
    /// promoted.
    pub fn open_bundle(root: &Path) -> Result<(Self, BundleRecovery), RuntimeError> {
        Self::open_bundle_with(root, CheckpointCadence::default())
    }

    /// [`Self::open_bundle`] with an explicit automatic-checkpoint cadence.
    pub fn open_bundle_with(
        root: &Path,
        cadence: CheckpointCadence,
    ) -> Result<(Self, BundleRecovery), RuntimeError> {
        let registry = runtime_history_decoder_registry()?;
        let store = HistoryStore::open_with_registry(root, registry)?;
        let tail = store.recovered_tail();
        let checkpoint = store
            .latest_checkpoint()?
            .ok_or(RuntimeError::BundleWithoutCheckpoint)?;
        let checkpoint_generation = checkpoint.header.durable_generation;
        let durable_generation = store.durable_generation();
        let envelope = SnapshotEnvelope::decode(&checkpoint.payload)?;
        verify_checkpoint_payload(&store, &checkpoint, &envelope)?;
        let plan = if durable_generation > checkpoint_generation {
            let (first, frames) = store.frames_after_generation(checkpoint_generation)?;
            match first {
                Some(first_ordinal) if frames == durable_generation - checkpoint_generation => {
                    Some(ReplayPlan {
                        checkpoint_generation,
                        first_ordinal,
                        frames,
                    })
                }
                _ => {
                    return Err(RuntimeError::BundleInvalid {
                        reason: "the durable frames after the latest checkpoint are not one per generation",
                    });
                }
            }
        } else {
            None
        };
        let resume = RuntimeHistoryCoordinator::resume_open_bundle(store, root, cadence, plan)?;
        let mut runtime = Self::from_bounded_snapshot_bytes(&checkpoint.payload, resume)?;
        let replayed_ticks = plan.map_or(0, |plan| plan.frames);
        for _ in 0..replayed_ticks {
            runtime.tick()?;
        }
        runtime.lock_state()?.history.finish_replay()?;
        let recovery = BundleRecovery {
            generation: durable_generation,
            checkpoint_generation,
            replayed_ticks,
            completed_time: runtime.current_time(),
            lost: tail.first_generation.zip(tail.last_generation).map(
                |(first_generation, last_generation)| LostLogicalRange {
                    first_generation,
                    last_generation,
                    ticks: tail.frames,
                    records: tail.records,
                },
            ),
            unaccounted_bytes: tail.unaccounted_bytes,
        };
        Ok((runtime, recovery))
    }

    /// The failure of the last automatic checkpoint, if none has succeeded
    /// since. A missed checkpoint never fails a tick; it only lengthens the
    /// replay a crash would cost.
    pub fn last_checkpoint_error(&self) -> Option<RuntimeError> {
        self.state
            .lock()
            .ok()
            .and_then(|state| state.history.checkpoint_error())
    }

    /// The directory of the bundle this session writes to, if it has one.
    pub fn bundle_path(&self) -> Option<PathBuf> {
        self.state
            .lock()
            .ok()
            .and_then(|state| state.history.bundle_root().map(Path::to_path_buf))
    }

    /// Force the pending group to the catalog.
    pub fn flush_durable(&mut self) -> Result<(), RuntimeError> {
        let mut state = self.lock_state()?;
        if let Some(error) = state.failure.clone() {
            return Err(error);
        }
        state.history.flush_durable()
    }

    /// End the session cleanly: force the pending group and report any failure,
    /// which dropping the session would have to ignore.
    pub fn shutdown(self) -> Result<(), RuntimeError> {
        let mut state = self.lock_state()?;
        if state.history.bundle_root().is_some() && !state.history.is_abandoned() {
            state.history.flush_durable()?;
        }
        Ok(())
    }

    /// Drop the session as a crash would: the pending group is not forced.
    ///
    /// For tests and for the crash drills of a bundle's recovery; a production
    /// session ends with [`Self::shutdown`] or an ordinary drop.
    #[doc(hidden)]
    pub fn simulate_crash(self) {
        if let Ok(mut state) = self.state.lock() {
            state.history.abandon();
        }
    }
}

impl Drop for Runtime {
    fn drop(&mut self) {
        let Ok(mut state) = self.state.lock() else {
            return;
        };
        if state.history.bundle_root().is_some() && !state.history.is_abandoned() {
            let _ = state.history.flush_durable();
        }
    }
}

/// What a checkpoint's snapshot payload must say about itself before a
/// session is built from it.
fn verify_checkpoint_payload(
    store: &HistoryStore,
    checkpoint: &CheckpointEnvelope,
    envelope: &SnapshotEnvelope,
) -> Result<(), RuntimeError> {
    let header = &checkpoint.header;
    if envelope.header.runtime_recipe_fingerprint != header.runtime_recipe_fingerprint
        || envelope.header.physical_digest != header.physical_digest
        || envelope.header.completed_time != header.completed_time.raw()
    {
        return Err(RuntimeError::BundleInvalid {
            reason: "the checkpoint header disagrees with the snapshot it carries",
        });
    }
    // The anchor inside the payload points at the latest checkpoint before its
    // own generation, which it cannot name itself.
    let anchor = history_anchor_from_bounded_envelope(envelope)?;
    let expected = store
        .checkpoint_digests()?
        .into_iter()
        .rev()
        .find(|(generation, _)| *generation < header.durable_generation)
        .map_or([0; 32], |(_, digest)| digest);
    if *anchor.checkpoint_digest() != expected {
        return Err(RuntimeError::BundleInvalid {
            reason: "the snapshot's checkpoint pointer is not the checkpoint before it",
        });
    }
    match store.read_snapshot(header.durable_generation) {
        Ok(bytes) if bytes == checkpoint.payload => Ok(()),
        Ok(_) => Err(RuntimeError::BundleInvalid {
            reason: "the registered snapshot differs from its checkpoint",
        }),
        Err(PersistenceError::HistoryGenerationUnavailable { .. }) => Ok(()),
        Err(error) => Err(error.into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TempBundle(PathBuf);

    impl Drop for TempBundle {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn a_seeded_session_keeps_its_history_on_disk_with_bounded_memory() {
        // Given: the test-convenience constructor, not a named bundle.
        let mut runtime = Runtime::from_seed(95).expect("session");
        runtime.run_ticks(64).expect("ticks");

        // Then: its history is a real on-disk bundle, not an in-memory authority,
        // its hot cache stays inside its budget, and it is removed with the session.
        let root = runtime
            .lock_state()
            .expect("state")
            .history
            .temporary_root()
            .map(Path::to_path_buf)
            .expect("a seeded session has a temporary history root");
        assert!(root.join("catalog.sqlite").is_file());
        assert!(
            std::fs::read_dir(root.join("segments"))
                .expect("segments")
                .next()
                .is_some()
        );
        let status = runtime.history_status().expect("status");
        assert!(status.hot_bytes <= status.hot_budget);
        assert!(runtime.bundle_path().is_none());
        drop(runtime);
        assert!(
            !root.exists(),
            "the scratch history is removed with the session"
        );
    }

    #[test]
    fn replay_refuses_a_tick_that_differs_from_the_stored_frame() {
        // Given: an every-tick bundle that ran three ticks past its checkpoint and
        // died, so opening it re-executes those three ticks.
        let root = TempBundle(std::env::temp_dir().join(format!(
            "causafera-replay-divergence-p{}",
            std::process::id()
        )));
        let _ = std::fs::remove_dir_all(&root.0);
        let mut session = Runtime::create_bundle(
            RuntimeConfig::new(93),
            &root.0,
            BundleOptions {
                durability: DurabilityMode::EveryTick,
                ..BundleOptions::default()
            },
        )
        .expect("bundle");
        session.run_ticks(3).expect("ticks");
        session.simulate_crash();

        // When: the stored frames are read back one record short.
        crate::history_transaction::tamper_replayed_stored_frames(true);
        let refused = Runtime::open_bundle(&root.0);
        crate::history_transaction::tamper_replayed_stored_frames(false);

        // Then: the open fails closed, naming the generation, and the intact
        // bundle still opens.
        match refused {
            Ok(_) => panic!("a replay that differs from the bundle was accepted"),
            Err(RuntimeError::ReplayDiverged { reason, .. }) => {
                assert!(reason.contains("different records"), "{reason}");
            }
            Err(other) => panic!("wrong refusal: {other}"),
        }
        let (resumed, recovery) = Runtime::open_bundle(&root.0).expect("the intact bundle opens");
        assert_eq!(recovery.replayed_ticks, 3);
        assert_eq!(resumed.current_time().raw(), 3);
    }
}
