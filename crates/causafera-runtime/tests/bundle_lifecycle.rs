//! A durable bundle as a session's history: create, save, open, shut down.
//!
//! See `plans/causal-history-storage.md` Todo 8.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use causafera_persistence::{DurabilityMode, HistoryStore};

mod support_hydrology;
use causafera_runtime::{BundleOptions, CheckpointCadence, Runtime, RuntimeConfig, RuntimeError};

struct TempDir(PathBuf);

impl TempDir {
    fn new(name: &str) -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "causafera-bundle-{name}-p{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&path);
        Self(path)
    }

    /// A path inside the directory, which itself is not created.
    fn bundle(&self) -> PathBuf {
        self.0.join("bundle")
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn options(durability: DurabilityMode) -> BundleOptions {
    BundleOptions {
        durability,
        ..BundleOptions::default()
    }
}

const MODES: [(&str, DurabilityMode); 2] = [
    ("group", DurabilityMode::GroupCommit),
    ("every-tick", DurabilityMode::EveryTick),
];

fn entries(directory: &Path) -> Vec<String> {
    let mut names = fs::read_dir(directory)
        .expect("directory reads")
        .map(|entry| {
            entry
                .expect("entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect::<Vec<_>>();
    names.sort();
    names
}

#[test]
fn a_new_bundle_has_its_complete_layout_before_the_first_tick() {
    for (name, mode) in MODES {
        // Given
        let temp = TempDir::new(name);
        let root = temp.bundle();

        // When
        let runtime = Runtime::create_bundle(RuntimeConfig::new(81), &root, options(mode))
            .expect("the bundle is created");

        // Then: before any tick, the catalog, a segment, a checkpoint and a
        // snapshot of tick zero are on disk, and nothing is waiting in a group.
        assert_eq!(runtime.current_time().raw(), 0);
        assert_eq!(runtime.bundle_path().as_deref(), Some(root.as_path()));
        assert!(root.join("catalog.sqlite").is_file());
        assert_eq!(entries(&root.join("checkpoints")).len(), 1);
        let snapshots = entries(&root.join("snapshots"));
        assert_eq!(snapshots.len(), 1);
        assert!(snapshots[0].ends_with(".snap"));
        assert!(!entries(&root.join("segments")).is_empty());
        let status = runtime.history_status().expect("status");
        assert_eq!(status.logical_generation, status.durable_generation);
        assert_eq!(status.pending_ticks, 0);
        println!(
            "MARKER:bundle_layout mode={name} generation={} checkpoints=1 snapshots=1",
            status.durable_generation
        );
    }
}

#[test]
fn a_bundle_is_never_created_over_existing_data() {
    let temp = TempDir::new("exists");
    let root = temp.bundle();
    fs::create_dir_all(&root).expect("directory");

    // An empty directory is fine.
    let first = Runtime::create_bundle(RuntimeConfig::new(82), &root, BundleOptions::default())
        .expect("an empty directory takes a bundle");
    drop(first);
    let listing = entries(&root);

    // A bundle, a file with other contents, and a plain file are not.
    for target in [root.clone(), temp.path().join("file")] {
        if !target.exists() {
            fs::write(&target, b"data").expect("file");
        }
        let error =
            match Runtime::create_bundle(RuntimeConfig::new(82), &target, BundleOptions::default())
            {
                Ok(_) => panic!(
                    "a bundle was created over existing data at {}",
                    target.display()
                ),
                Err(error) => error,
            };
        assert!(
            matches!(error, RuntimeError::BundleExists { .. }),
            "{error}"
        );
    }
    assert_eq!(entries(&root), listing, "the existing bundle is untouched");
}

#[test]
fn a_session_that_is_not_a_bundle_cannot_save_one() {
    let mut runtime = Runtime::from_seed(83).expect("session");
    assert!(runtime.bundle_path().is_none());
    assert!(matches!(
        runtime.save_bundle(),
        Err(RuntimeError::NotABundle)
    ));
}

#[test]
fn a_saved_bundle_resumes_to_the_uninterrupted_run() {
    for (name, mode) in MODES {
        for saved_at in [0_u64, 1, 40] {
            // Given: a control that never stopped.
            let temp = TempDir::new(&format!("resume-{name}-{saved_at}"));
            let root = temp.bundle();
            let total = saved_at + 8;
            let mut control = Runtime::new(RuntimeConfig::new(84)).expect("control");
            control.run_ticks(total).expect("control runs");
            let control_summary = control.snapshot().expect("control summary");

            // When: another session saves at `saved_at`, shuts down, and a third
            // opens the bundle from disk and finishes the run.
            let mut saved = Runtime::create_bundle(RuntimeConfig::new(84), &root, options(mode))
                .expect("bundle");
            if saved_at > 0 {
                saved.run_ticks(saved_at).expect("saved session runs");
            }
            let receipt = saved.save_bundle().expect("save");
            assert_eq!(receipt.completed_time.raw(), saved_at);
            saved.shutdown().expect("clean shutdown");
            let (mut resumed, recovery) = Runtime::open_bundle(&root).expect("open");
            assert_eq!(recovery.completed_time.raw(), saved_at);
            assert_eq!(recovery.generation, receipt.generation);
            assert_eq!(recovery.lost, None, "a clean shutdown loses nothing");
            resumed.run_ticks(8).expect("resumed session runs");

            // Then: every digest and the complete causal history agree.
            let resumed_summary = resumed.snapshot().expect("resumed summary");
            assert_eq!(
                resumed_summary.physical_state_digest,
                control_summary.physical_state_digest
            );
            assert_eq!(
                resumed_summary.history_digest,
                control_summary.history_digest
            );
            assert_eq!(
                resumed_summary.canonical_state,
                control_summary.canonical_state
            );
            assert_eq!(
                resumed
                    .export_snapshot_with_full_history()
                    .expect("resumed"),
                control
                    .export_snapshot_with_full_history()
                    .expect("control")
            );
            println!(
                "MARKER:bundle_resume mode={name} saved_at={saved_at} resumed_generation={} equal=true",
                recovery.generation
            );
        }
    }
}

#[test]
fn a_crash_reports_the_ticks_it_lost_and_the_resumed_run_replays_them() {
    // Given: a group-commit session saved at tick 5 that ran seven more ticks
    // inside one unpublished group and then died.
    let temp = TempDir::new("crash");
    let root = temp.bundle();
    let mut session = Runtime::create_bundle(
        RuntimeConfig::new(85),
        &root,
        options(DurabilityMode::GroupCommit),
    )
    .expect("bundle");
    session.run_ticks(5).expect("first ticks");
    let receipt = session.save_bundle().expect("save");
    session.run_ticks(7).expect("ticks after the save");
    assert!(
        session.history_status().expect("status").pending_ticks > 0,
        "the later ticks are an unpublished group"
    );
    session.simulate_crash();

    // When
    let (mut resumed, recovery) = Runtime::open_bundle(&root).expect("open after the crash");

    // Then: it resumes at the save, and the loss is named exactly.
    assert_eq!(recovery.generation, receipt.generation);
    assert_eq!(recovery.completed_time.raw(), 5);
    let lost = recovery.lost.expect("seven ticks were lost");
    assert_eq!(lost.ticks, 7);
    assert_eq!(lost.first_generation, receipt.generation + 1);
    assert_eq!(lost.last_generation, receipt.generation + 7);
    assert_eq!(recovery.unaccounted_bytes, 0);

    // And: the lost ticks simulate again to exactly what they were.
    resumed.run_ticks(7).expect("resumed session reruns them");
    let mut control = Runtime::new(RuntimeConfig::new(85)).expect("control");
    control.run_ticks(12).expect("control runs");
    assert_eq!(
        resumed.snapshot().expect("resumed").canonical_state,
        control.snapshot().expect("control").canonical_state
    );
    println!(
        "MARKER:bundle_crash lost_ticks={} first_generation={} last_generation={}",
        lost.ticks, lost.first_generation, lost.last_generation
    );
}

#[test]
fn a_clean_shutdown_forces_the_group_and_a_crash_does_not() {
    for clean in [true, false] {
        // Given: five ticks, fewer than a group, after the genesis save.
        let temp = TempDir::new(if clean { "clean" } else { "dirty" });
        let root = temp.bundle();
        let mut session = Runtime::create_bundle(
            RuntimeConfig::new(86),
            &root,
            options(DurabilityMode::GroupCommit),
        )
        .expect("bundle");
        let genesis = session.history_status().expect("status").durable_generation;
        session.run_ticks(5).expect("ticks");

        // When
        if clean {
            drop(session);
        } else {
            session.simulate_crash();
        }

        // Then: a clean exit published the five ticks; a crash left them as a tail.
        let store = HistoryStore::open(&root).expect("the catalog opens");
        let tail = store.recovered_tail();
        if clean {
            assert_eq!(store.durable_generation(), genesis + 5);
            assert_eq!(tail.frames, 0);
        } else {
            assert_eq!(store.durable_generation(), genesis);
            assert_eq!(tail.frames, 5);
        }
    }
}

#[test]
fn every_tick_durability_never_runs_behind() {
    let temp = TempDir::new("every-tick-lag");
    let mut session = Runtime::create_bundle(
        RuntimeConfig::new(87),
        &temp.bundle(),
        options(DurabilityMode::EveryTick),
    )
    .expect("bundle");
    for _ in 0..40 {
        session.tick().expect("tick");
        let status = session.history_status().expect("status");
        assert_eq!(status.logical_generation, status.durable_generation);
        assert_eq!(status.pending_ticks, 0);
    }
}

#[test]
fn a_damaged_bundle_is_refused_rather_than_resumed() {
    let temp = TempDir::new("damaged");
    let root = temp.bundle();
    let mut session = Runtime::create_bundle(
        RuntimeConfig::new(88),
        &root,
        options(DurabilityMode::EveryTick),
    )
    .expect("bundle");
    session.run_ticks(3).expect("ticks");
    session.save_bundle().expect("save");
    session.shutdown().expect("shutdown");
    Runtime::open_bundle(&root).expect("the intact bundle opens");

    // A checkpoint, a snapshot and the catalog are each necessary.
    let checkpoint = fs::read_dir(root.join("checkpoints"))
        .expect("checkpoints")
        .next()
        .expect("a checkpoint")
        .expect("entry")
        .path();
    let snapshot = fs::read_dir(root.join("snapshots"))
        .expect("snapshots")
        .next()
        .expect("a snapshot")
        .expect("entry")
        .path();
    for (case, victim) in [
        ("checkpoint", checkpoint.clone()),
        ("snapshot", snapshot.clone()),
        ("catalog", root.join("catalog.sqlite")),
    ] {
        let original = fs::read(&victim).expect("read");
        let mut damaged = original.clone();
        let middle = damaged.len() / 2;
        damaged[middle] ^= 0xFF;
        fs::write(&victim, &damaged).expect("damage");
        let outcome = Runtime::open_bundle(&root);
        fs::write(&victim, &original).expect("restore");
        let error = match outcome {
            Ok(_) => panic!("a bundle with a damaged {case} resumed"),
            Err(error) => error,
        };
        println!("MARKER:bundle_damage case={case} error={error}");
    }
    Runtime::open_bundle(&root).expect("the restored bundle opens again");
    let segment = fs::read_dir(root.join("segments"))
        .expect("segments")
        .next()
        .expect("a segment")
        .expect("entry")
        .path();
    let kept = fs::read(&segment).expect("segment bytes");
    fs::remove_file(&segment).expect("remove segment");
    let error = match Runtime::open_bundle(&root) {
        Ok(_) => panic!("a bundle missing its segment resumed"),
        Err(error) => error,
    };
    println!("MARKER:bundle_damage case=segment error={error}");
    fs::write(&segment, kept).expect("restore segment");
    Runtime::open_bundle(&root).expect("the restored bundle opens again");
    fs::remove_file(&checkpoint).expect("remove checkpoint");
    assert!(
        Runtime::open_bundle(&root).is_err(),
        "a bundle missing a registered checkpoint is refused"
    );
}

fn control_after(seed: u64, ticks: u64) -> Runtime {
    let mut control = Runtime::new(RuntimeConfig::new(seed)).expect("control");
    control.run_ticks(ticks).expect("control runs");
    control
}

fn assert_same_run(resumed: &mut Runtime, control: &mut Runtime) {
    let resumed_summary = resumed.snapshot().expect("resumed summary");
    let control_summary = control.snapshot().expect("control summary");
    assert_eq!(
        resumed_summary.physical_state_digest,
        control_summary.physical_state_digest
    );
    assert_eq!(
        resumed_summary.history_digest,
        control_summary.history_digest
    );
    assert_eq!(
        resumed_summary.canonical_state,
        control_summary.canonical_state
    );
    assert_eq!(
        resumed
            .export_snapshot_with_full_history()
            .expect("resumed"),
        control
            .export_snapshot_with_full_history()
            .expect("control")
    );
}

#[test]
fn a_crash_after_published_groups_is_replayed_to_the_durable_head() {
    // Given: a group-commit session that ran 40 ticks and died. How many of them
    // were published depends on when its groups filled (32 ticks or 250 ms), so
    // the assertions below are about how the published and the lost ticks relate,
    // not about a particular split.
    let temp = TempDir::new("replay-group");
    let root = temp.bundle();
    let mut session = Runtime::create_bundle(
        RuntimeConfig::new(90),
        &root,
        options(DurabilityMode::GroupCommit),
    )
    .expect("bundle");
    let genesis = session.history_status().expect("status").durable_generation;
    session.run_ticks(40).expect("ticks");
    session.simulate_crash();

    // When
    let (mut resumed, recovery) = Runtime::open_bundle(&root).expect("open after the crash");

    // Then: the open restored the genesis checkpoint and re-executed, checking each
    // frame, every tick that had been published; the rest are the reported loss.
    let lost_ticks = recovery.lost.map_or(0, |lost| lost.ticks);
    assert_eq!(recovery.checkpoint_generation, genesis);
    assert_eq!(recovery.generation, genesis + recovery.replayed_ticks);
    assert_eq!(recovery.completed_time.raw(), recovery.replayed_ticks);
    assert!(recovery.replayed_ticks > 0, "a group had been published");
    assert_eq!(recovery.replayed_ticks + lost_ticks, 40);
    if let Some(lost) = recovery.lost {
        assert_eq!(lost.first_generation, recovery.generation + 1);
        assert_eq!(lost.last_generation, genesis + 40);
    }

    // And: the session is the one that was lost.
    resumed
        .run_ticks(40 - recovery.completed_time.raw())
        .expect("the lost ticks run again");
    assert_same_run(&mut resumed, &mut control_after(90, 40));
    println!(
        "MARKER:bundle_replay mode=group replayed={} lost={lost_ticks} head_generation={}",
        recovery.replayed_ticks, recovery.generation
    );
}

#[test]
fn every_tick_durability_loses_nothing_to_a_crash() {
    let temp = TempDir::new("replay-every-tick");
    let root = temp.bundle();
    let mut session = Runtime::create_bundle(
        RuntimeConfig::new(91),
        &root,
        options(DurabilityMode::EveryTick),
    )
    .expect("bundle");
    session.run_ticks(12).expect("ticks");
    session.simulate_crash();

    let (mut resumed, recovery) = Runtime::open_bundle(&root).expect("open after the crash");

    assert_eq!(recovery.completed_time.raw(), 12);
    assert_eq!(recovery.replayed_ticks, 12);
    assert_eq!(recovery.lost, None, "no tick was reported done and lost");
    resumed.run_ticks(8).expect("more ticks");
    assert_same_run(&mut resumed, &mut control_after(91, 20));
}

#[test]
fn checkpoints_roll_over_and_bound_what_a_crash_replays() {
    // Every-tick durability publishes each tick, so a checkpoint every ten ticks is
    // exact. Group commit publishes when 32 ticks or 250 ms have accumulated, so
    // there only the bound is deterministic.
    for (name, mode, ticks) in [
        ("every-tick", DurabilityMode::EveryTick, 35_u64),
        ("group", DurabilityMode::GroupCommit, 70),
    ] {
        // Given: a checkpoint due after ten durably published ticks.
        let temp = TempDir::new(&format!("rollover-{name}"));
        let root = temp.bundle();
        let cadence = CheckpointCadence {
            ticks: 10,
            frame_bytes: u64::MAX,
        };
        let mut session = Runtime::create_bundle(
            RuntimeConfig::new(92),
            &root,
            BundleOptions {
                durability: mode,
                cadence,
            },
        )
        .expect("bundle");
        session.run_ticks(ticks).expect("ticks");
        assert!(session.last_checkpoint_error().is_none());
        session.simulate_crash();

        // Then: the session checkpointed on its own, the crash replays only what
        // is past the latest checkpoint, and the resumed run is the same run.
        let checkpoints = entries(&root.join("checkpoints")).len();
        let (mut resumed, recovery) =
            Runtime::open_bundle_with(&root, cadence).expect("open after the crash");
        let lost_ticks = recovery.lost.map_or(0, |lost| lost.ticks);
        if mode == DurabilityMode::EveryTick {
            assert_eq!(checkpoints, 4, "{name}: genesis plus ticks 10, 20 and 30");
            assert_eq!(recovery.replayed_ticks, 5, "{name}: ticks 31 to 35");
            assert_eq!(lost_ticks, 0);
        } else {
            assert!(checkpoints >= 2, "{name}: a group boundary checkpointed");
            assert!(
                recovery.replayed_ticks <= 32,
                "{name}: a checkpoint bounds the replay to under one group, got {}",
                recovery.replayed_ticks
            );
        }
        assert_eq!(
            recovery.completed_time.raw() + lost_ticks,
            ticks,
            "{name}: every tick is either recovered or reported lost"
        );
        resumed.run_ticks(12).expect("more ticks");
        let total = recovery.completed_time.raw() + 12;
        assert_same_run(&mut resumed, &mut control_after(92, total));
        println!(
            "MARKER:bundle_rollover mode={name} checkpoints={checkpoints} replayed={} lost={lost_ticks}",
            recovery.replayed_ticks
        );
    }
}

#[test]
fn a_thousand_tick_bundle_resumes_to_the_uninterrupted_run() {
    for (name, mode) in MODES {
        // Given: a thousand ticks saved, and eight more run after resuming.
        let temp = TempDir::new(&format!("thousand-{name}"));
        let root = temp.bundle();
        let mut saved =
            Runtime::create_bundle(RuntimeConfig::new(94), &root, options(mode)).expect("bundle");
        saved.run_ticks(1_000).expect("a thousand ticks");
        let receipt = saved.save_bundle().expect("save");
        saved.shutdown().expect("shutdown");

        // When
        let (mut resumed, recovery) = Runtime::open_bundle(&root).expect("open");
        assert_eq!(recovery.generation, receipt.generation);
        assert_eq!(recovery.replayed_ticks, 0);
        resumed.run_ticks(8).expect("eight more");

        // Then
        assert_same_run(&mut resumed, &mut control_after(94, 1_008));
        println!(
            "MARKER:bundle_thousand mode={name} generation={}",
            receipt.generation
        );
    }
}

#[test]
fn a_hydrology_bundle_saves_resumes_and_replays_exactly() {
    // Given: a world with water, whose ticks commit tens of thousands of records
    // and `ReplayRequired` hydrology evidence, run for three ticks in an every-tick
    // bundle that is saved after the first and then dies.
    let temp = TempDir::new("hydrology");
    let root = temp.bundle();
    let config = support_hydrology::wet_runtime_config();
    let mut session =
        Runtime::create_bundle(config.clone(), &root, options(DurabilityMode::EveryTick))
            .expect("hydrology bundle");
    session.run_ticks(1).expect("first tick");
    session.save_bundle().expect("save");
    session.run_ticks(2).expect("two more ticks");
    session.simulate_crash();

    // When: it is opened, re-executing the two ticks after the save, and run on.
    let (mut resumed, recovery) = Runtime::open_bundle(&root).expect("open after the crash");
    assert_eq!(recovery.replayed_ticks, 2);
    assert_eq!(recovery.completed_time.raw(), 3);
    resumed.run_ticks(1).expect("one more tick");

    // Then: it is the run that was lost, in every digest and in its complete
    // history, typed hydrology evidence included.
    let mut control = Runtime::new(config).expect("control");
    control.run_ticks(4).expect("control runs");
    assert_same_run(&mut resumed, &mut control);
    println!(
        "MARKER:bundle_hydrology replayed={} equal=true",
        recovery.replayed_ticks
    );
}
