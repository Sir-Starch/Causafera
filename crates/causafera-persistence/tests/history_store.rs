// allow: SIZE_OK — one integration binary exercises the complete HistoryStore surface.
use std::{
    env,
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    process::Command,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
};

use causafera_core::{
    CausalEffect, CausalTarget, Criticality, DomainEvidenceRecord, EvidenceDecoder,
    EvidenceDecoderRegistry, EvidenceSchemaId, EvidenceSchemaVersion, HistoryAnchor,
    HistoryAnchorFields, HistoryError, HistoryQuery, HistoryReader, HistoryRecord,
    MAX_FRAME_DECODED_BYTES, OwnedCausalRecord, Phase, StateFingerprint,
};
use causafera_persistence::{
    CheckpointMetadata, DurabilityMode, EMPTY_EVIDENCE_MANIFEST_DIGEST, FailurePoint,
    HISTORY_FRAME_HEADER_LEN, HistoryStore, HistoryStoreHooks, HistoryStoreObservation,
    HistoryStoreStatus, PersistenceError, decode_checkpoint, encode_checkpoint,
    evidence_manifest_digest_full_rescan, evidence_manifest_full_rescan_count,
    reset_evidence_manifest_full_rescan_counter,
};
use causafera_types::{
    EventId, EventKindId, SimulationTime, StateObjectKindId, StatePropertyId, TraceId,
};

static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);
static TEMP_CREATION_LOCK: Mutex<()> = Mutex::new(());
static RECOVERY_PROCESS_LOCK: Mutex<()> = Mutex::new(());
const WRITER_LOCK_CHILD_BUNDLE: &str = "CAUSAFERA_HISTORY_WRITER_LOCK_CHILD_BUNDLE";
const RECOVERY_CHILD_BUNDLE: &str = "CAUSAFERA_HISTORY_RECOVERY_CHILD_BUNDLE";
const RECOVERY_CHILD_POINT: &str = "CAUSAFERA_HISTORY_RECOVERY_CHILD_POINT";
const RECOVERY_CHILD_OPERATION: &str = "CAUSAFERA_HISTORY_RECOVERY_CHILD_OPERATION";

fn authoritative_status(mut status: HistoryStoreStatus) -> HistoryStoreStatus {
    status.append_elapsed_wall_time = Duration::ZERO;
    status.durable_sync_count = 0;
    status.durable_sync_elapsed_wall_time = Duration::ZERO;
    status.catalog_commit_count = 0;
    status.catalog_commit_elapsed_wall_time = Duration::ZERO;
    status.logical_digest_full_rescan_count = 0;
    status.catalog_full_scan_count = 0;
    status
}

struct TempBundle(PathBuf);

impl TempBundle {
    fn new(name: &str) -> Self {
        let _guard = TEMP_CREATION_LOCK.lock().expect("test directory lock");
        Self::new_locked(name)
    }

    fn new_with_stale_candidate(name: &str) -> (Self, PathBuf) {
        let _guard = TEMP_CREATION_LOCK.lock().expect("test directory lock");
        let stale_attempt = NEXT_TEMP.load(Ordering::Relaxed);
        let stale_path = Self::candidate(name, stale_attempt);
        fs::create_dir(&stale_path).expect("create stale test directory");
        (Self::new_locked(name), stale_path)
    }

    fn new_locked(name: &str) -> Self {
        for _ in 0..1024 {
            let attempt = NEXT_TEMP.fetch_add(1, Ordering::Relaxed);
            let path = Self::candidate(name, attempt);
            match fs::create_dir(&path) {
                Ok(()) => return Self(path),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
                Err(error) => panic!("create unique test directory: {error}"),
            }
        }
        panic!("exhausted unique test directory attempts");
    }

    fn candidate(name: &str, attempt: u64) -> PathBuf {
        std::env::temp_dir().join(format!(
            "causafera-history-store-{name}-p{}-attempt-{attempt}",
            std::process::id()
        ))
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempBundle {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).expect("remove history-store test directory");
        println!("cleanup_removed={}", self.0.display());
    }
}

#[derive(Default)]
struct TestHooks {
    now_ms: AtomicU64,
    failure: AtomicUsize,
    flushes: AtomicUsize,
    segment_syncs: AtomicUsize,
    catalog_commits: AtomicUsize,
    directory_syncs: AtomicUsize,
}

impl TestHooks {
    fn advance(&self, millis: u64) {
        self.now_ms.fetch_add(millis, Ordering::SeqCst);
    }

    fn fail_once(&self, point: FailurePoint) {
        self.failure.store(point.code(), Ordering::SeqCst);
    }
}

impl HistoryStoreHooks for TestHooks {
    fn monotonic_millis(&self) -> u64 {
        self.now_ms.load(Ordering::SeqCst)
    }

    fn should_fail(&self, point: FailurePoint) -> bool {
        self.failure
            .compare_exchange(point.code(), 0, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
    }

    fn observe(&self, observation: HistoryStoreObservation) {
        match observation {
            HistoryStoreObservation::FrameFlush => self.flushes.fetch_add(1, Ordering::SeqCst),
            HistoryStoreObservation::SegmentSync => {
                self.segment_syncs.fetch_add(1, Ordering::SeqCst)
            }
            HistoryStoreObservation::CatalogCommit => {
                self.catalog_commits.fetch_add(1, Ordering::SeqCst)
            }
            HistoryStoreObservation::DirectorySync => {
                self.directory_syncs.fetch_add(1, Ordering::SeqCst)
            }
        };
    }
}

struct CrashHooks {
    point: String,
}

impl HistoryStoreHooks for CrashHooks {
    fn monotonic_millis(&self) -> u64 {
        0
    }

    fn should_fail(&self, point: FailurePoint) -> bool {
        if point.name() == self.point {
            std::process::exit(87);
        }
        false
    }
}

#[test]
fn history_recovery_child_process() {
    let Ok(root) = env::var(RECOVERY_CHILD_BUNDLE) else {
        return;
    };
    let point = env::var(RECOVERY_CHILD_POINT).expect("crash point");
    let operation = env::var(RECOVERY_CHILD_OPERATION).expect("crash operation");
    let hooks = Arc::new(CrashHooks { point });
    let mut store = HistoryStore::open_with_hooks(Path::new(&root), hooks).expect("child open");
    match operation.as_str() {
        "append" => {
            store
                .append_tick(vec![causal(2, vec![TraceId::new(1)])])
                .expect("child append");
            store.flush_durable().expect("child flush");
        }
        "seal" => {
            store.seal_active().expect("child seal");
        }
        "checkpoint" => {
            let anchor = store
                .durable_reader()
                .expect("reader")
                .anchor()
                .expect("anchor");
            store
                .publish_checkpoint(
                    &anchor,
                    &checkpoint_metadata(anchor.frontier().time),
                    b"checkpoint state",
                )
                .expect("child checkpoint");
        }
        "snapshot" => {
            store
                .publish_snapshot(1, b"portable snapshot")
                .expect("child snapshot");
        }
        "clean" => {
            store
                .append_tick(vec![causal(2, vec![TraceId::new(1)])])
                .expect("child append before clean exit");
            store.flush_durable().expect("clean flush");
            return;
        }
        _ => panic!("unknown recovery operation"),
    }
    panic!("the requested crash boundary was not reached");
}

#[test]
fn history_recovery_clean_exit_publishes_complete_generation_in_both_modes() {
    let _process_guard = RECOVERY_PROCESS_LOCK.lock().expect("recovery process lock");
    for mode in [DurabilityMode::GroupCommit, DurabilityMode::EveryTick] {
        let temp = TempBundle::new(&format!("clean-process-exit-{mode:?}"));
        let mut store = HistoryStore::create(temp.path(), [0x77; 32], mode).expect("store");
        store
            .append_tick(vec![causal(1, vec![])])
            .expect("first frame");
        store.flush_durable().expect("first frame durable");
        drop(store);
        let output = Command::new(env::current_exe().expect("test binary"))
            .arg("--exact")
            .arg("history_recovery_child_process")
            .env(RECOVERY_CHILD_BUNDLE, temp.path())
            .env(RECOVERY_CHILD_POINT, "none")
            .env(RECOVERY_CHILD_OPERATION, "clean")
            .output()
            .expect("start clean child");
        assert!(output.status.success(), "{output:?}");
        let reopened = HistoryStore::open(temp.path()).expect("clean generation reopens");
        assert_eq!(reopened.durable_generation(), 2);
        assert_eq!(reopened.catalog().counts().expect("counts").events, 2);
        assert_eq!(reopened.recovered_tail().frames, 0);
    }
}

#[test]
fn history_recovery_process_crash_matrix() {
    let _process_guard = RECOVERY_PROCESS_LOCK.lock().expect("recovery process lock");
    for mode in [DurabilityMode::GroupCommit, DurabilityMode::EveryTick] {
        for (operation, points) in [
            (
                "append",
                &[
                    FailurePoint::BeforeFrameFlush,
                    FailurePoint::AfterFrameFlushBeforeSync,
                    FailurePoint::AfterSegmentSyncBeforeCatalogCommit,
                    FailurePoint::DuringCatalogCommit,
                    FailurePoint::AfterCatalogCommitBeforeRootSync,
                    FailurePoint::AfterAppendCatalogCommitAfterRootSync,
                ][..],
            ),
            (
                "seal",
                &[
                    FailurePoint::SealRename,
                    FailurePoint::AfterSealRenameBeforeNewActive,
                    FailurePoint::AfterSealNewActiveBeforeCatalogCommit,
                    FailurePoint::AfterSealCatalogCommitBeforeRootSync,
                    FailurePoint::AfterSealCatalogCommitAfterRootSync,
                ][..],
            ),
            (
                "checkpoint",
                &[
                    FailurePoint::AfterCheckpointSyncBeforeCatalogCommit,
                    FailurePoint::AfterCheckpointCatalogCommitBeforeRootSync,
                    FailurePoint::AfterCheckpointCatalogCommitAfterRootSync,
                ][..],
            ),
            (
                "snapshot",
                &[
                    FailurePoint::AfterSnapshotSyncBeforeCatalogCommit,
                    FailurePoint::AfterSnapshotCatalogCommitBeforeRootSync,
                    FailurePoint::AfterSnapshotCatalogCommitAfterRootSync,
                ][..],
            ),
        ] {
            for point in points {
                let temp = TempBundle::new(&format!("process-crash-{mode:?}-{}", point.name()));
                let mut store = HistoryStore::create(temp.path(), [0x77; 32], mode).expect("store");
                store
                    .append_tick(vec![causal(1, vec![])])
                    .expect("first frame");
                store.flush_durable().expect("first frame durable");
                drop(store);

                let output = Command::new(env::current_exe().expect("test binary"))
                    .arg("--exact")
                    .arg("history_recovery_child_process")
                    .env(RECOVERY_CHILD_BUNDLE, temp.path())
                    .env(RECOVERY_CHILD_POINT, point.name())
                    .env(RECOVERY_CHILD_OPERATION, operation)
                    .output()
                    .expect("start crash child");
                assert_eq!(
                    output.status.code(),
                    Some(87),
                    "{} {point:?}: {output:?}",
                    operation
                );

                let reopened = HistoryStore::open(temp.path()).unwrap_or_else(|error| {
                    let files = fs::read_dir(temp.path().join("segments"))
                        .expect("segment directory")
                        .map(|entry| entry.expect("segment entry").file_name())
                        .collect::<Vec<_>>();
                    panic!("complete generation opens: {error:?}; files={files:?}")
                });
                let generation = reopened.durable_generation();
                assert!(matches!(generation, 1 | 2));
                let counts = reopened.catalog().counts().expect("catalog counts");
                assert_eq!(counts.events, generation);
                let catalog_committed = matches!(
                    point,
                    FailurePoint::AfterCatalogCommitBeforeRootSync
                        | FailurePoint::AfterAppendCatalogCommitAfterRootSync
                        | FailurePoint::AfterCheckpointCatalogCommitBeforeRootSync
                        | FailurePoint::AfterCheckpointCatalogCommitAfterRootSync
                        | FailurePoint::AfterSnapshotCatalogCommitBeforeRootSync
                        | FailurePoint::AfterSnapshotCatalogCommitAfterRootSync
                );
                assert_eq!(
                    generation,
                    u64::from(operation == "append" && catalog_committed) + 1
                );
                assert_eq!(
                    counts.checkpoints,
                    u64::from(operation == "checkpoint" && catalog_committed)
                );
                assert_eq!(
                    counts.snapshots,
                    u64::from(operation == "snapshot" && catalog_committed)
                );
                let tail = reopened.recovered_tail();
                assert!(tail.frames <= 1);
                if tail.frames == 1 {
                    assert_eq!(
                        (tail.first_generation, tail.last_generation),
                        (Some(2), Some(2))
                    );
                }
                println!(
                    "{{\"mode\":\"{mode:?}\",\"operation\":\"{operation}\",\"point\":\"{}\",\"durable_generation\":{generation},\"events\":{},\"lost_ticks\":{}}}",
                    point.name(),
                    counts.events,
                    tail.frames
                );
            }
        }
    }
}

fn effect(object_id: u64, property: u64) -> CausalEffect {
    CausalEffect::new(
        CausalTarget::new(
            StateObjectKindId::new(7),
            object_id,
            StatePropertyId::new(property),
        ),
        StateFingerprint::new([0x11; 32]),
        StateFingerprint::new([0x22; 32]),
    )
    .expect("fixture changes state")
}

fn causal(sequence: u64, causes: Vec<TraceId>) -> HistoryRecord {
    HistoryRecord::Causal(
        OwnedCausalRecord::new(
            sequence,
            EventId::new(sequence),
            TraceId::new(sequence),
            SimulationTime::new(sequence * 10),
            Phase::Physics,
            EventKindId::new(13),
            causes,
            vec![effect(100 + sequence, 9)],
        )
        .expect("valid causal fixture"),
    )
}

fn checkpoint_metadata(completed_time: SimulationTime) -> CheckpointMetadata {
    CheckpointMetadata {
        runtime_recipe_fingerprint: [0x91; 32],
        input_fingerprint: [0x92; 32],
        physical_digest: [0x93; 32],
        completed_time,
    }
}

fn anchor_with_manifest_digest(anchor: &HistoryAnchor, digest: [u8; 32]) -> HistoryAnchor {
    HistoryAnchor::new(HistoryAnchorFields {
        namespace: *anchor.namespace(),
        logical_generation: anchor.logical_generation(),
        durable_generation: anchor.durable_generation(),
        checkpoint_digest: *anchor.checkpoint_digest(),
        frontier: anchor.frontier(),
        logical_digest: *anchor.logical_digest(),
        storage_root: *anchor.storage_root(),
        physical_digest_schema: anchor.physical_digest_schema(),
        history_digest_schema: anchor.history_digest_schema(),
        evidence_manifest_digest: digest,
        durability: anchor.durability(),
    })
    .expect("structurally valid anchor")
}

fn evidence(sequence: u64, anchor: u64, key: &[u8]) -> HistoryRecord {
    let payload = format!("payload-{sequence}").into_bytes();
    HistoryRecord::Evidence(
        DomainEvidenceRecord::new(
            sequence,
            EvidenceSchemaId::new(0xABCD),
            EvidenceSchemaVersion::new(1, 0),
            Criticality::ExplanationOnly,
            SimulationTime::new(sequence * 10),
            vec![TraceId::new(anchor)],
            key.to_vec(),
            payload.clone(),
            StateFingerprint::new(*blake3::hash(&payload).as_bytes()),
        )
        .expect("valid evidence fixture"),
    )
}

#[derive(Debug)]
struct SyntheticReplayDecoder;

impl EvidenceDecoder for SyntheticReplayDecoder {
    fn source_schema(&self) -> (EvidenceSchemaId, EvidenceSchemaVersion) {
        (
            EvidenceSchemaId::new(0xDEC0DE),
            EvidenceSchemaVersion::new(1, 0),
        )
    }

    fn target_version(&self) -> EvidenceSchemaVersion {
        EvidenceSchemaVersion::new(1, 0)
    }

    fn decode_or_upcast(&self, payload: &[u8]) -> Result<Vec<u8>, HistoryError> {
        Ok(payload.to_vec())
    }
}

fn replay_evidence(sequence: u64, anchor: u64) -> HistoryRecord {
    let payload = format!("required-{sequence}").into_bytes();
    HistoryRecord::Evidence(
        DomainEvidenceRecord::new(
            sequence,
            EvidenceSchemaId::new(0xDEC0DE),
            EvidenceSchemaVersion::new(1, 0),
            Criticality::ReplayRequired,
            SimulationTime::new(sequence * 10),
            vec![TraceId::new(anchor)],
            sequence.to_be_bytes().to_vec(),
            payload.clone(),
            StateFingerprint::new(*blake3::hash(&payload).as_bytes()),
        )
        .expect("valid replay-required evidence fixture"),
    )
}

fn large_evidence(sequence: u64) -> HistoryRecord {
    let payload = vec![u8::try_from(sequence % 251).expect("bounded byte"); 34 * 1024];
    HistoryRecord::Evidence(
        DomainEvidenceRecord::new(
            sequence,
            EvidenceSchemaId::new(0xDCBA),
            EvidenceSchemaVersion::new(1, 0),
            Criticality::ExplanationOnly,
            SimulationTime::new(sequence),
            Vec::new(),
            sequence.to_be_bytes().to_vec(),
            payload.clone(),
            StateFingerprint::new(*blake3::hash(&payload).as_bytes()),
        )
        .expect("valid large evidence fixture"),
    )
}

fn query() -> HistoryQuery {
    HistoryQuery::new(0, 256, 4 * 1024 * 1024).expect("bounded query")
}

#[test]
fn history_store_reader_validates_causal_overlay_from_indexed_frames() {
    // Given: a real durable store with a causal suffix spanning two canonical frames.
    let temp = TempBundle::new("indexed-overlay-validation");
    let mut store =
        HistoryStore::create(temp.path(), [0x71; 32], DurabilityMode::EveryTick).expect("store");
    let first = causal(1, vec![]);
    let second = causal(2, vec![TraceId::new(1)]);
    store
        .append_tick(vec![first.clone()])
        .expect("first durable frame");
    store
        .append_tick(vec![second.clone()])
        .expect("second durable frame");
    let reader = store.durable_reader().expect("indexed reader");
    let expected = [first, second]
        .into_iter()
        .map(|record| match record {
            HistoryRecord::Causal(record) => record,
            HistoryRecord::Evidence(_) => unreachable!("causal fixture"),
        })
        .collect::<Vec<_>>();

    // When: the reader validates exact records, then sees one same-ID effect mismatch and one miss.
    reader
        .validate_causal_overlay(&expected)
        .expect("indexed overlay is exact");
    reader
        .validate_causal_overlay(&[])
        .expect("empty overlay is valid");
    let mismatched = OwnedCausalRecord::new(
        expected[1].commit_sequence(),
        expected[1].event_id(),
        expected[1].trace_id(),
        expected[1].time(),
        expected[1].phase(),
        expected[1].kind(),
        expected[1].causes().to_vec(),
        vec![effect(999, 9)],
    )
    .expect("mismatch fixture is structurally valid");
    let mismatch = reader.validate_causal_overlay(&[expected[0].clone(), mismatched]);
    let unknown = reader.validate_causal_overlay(&[OwnedCausalRecord::new(
        3,
        EventId::new(3),
        TraceId::new(3),
        SimulationTime::new(30),
        Phase::Physics,
        EventKindId::new(13),
        vec![TraceId::new(2)],
        vec![effect(103, 9)],
    )
    .expect("unknown fixture is structurally valid")]);

    // Then: parents/effects are exact and both mismatch classes fail closed.
    assert_eq!(
        reader.event(TraceId::new(2)).expect("event"),
        Some(expected[1].clone())
    );
    assert!(matches!(
        mismatch,
        Err(causafera_core::HistoryError::ColdRecordMismatch { trace_id })
            if trace_id == TraceId::new(2)
    ));
    assert!(matches!(
        unknown,
        Err(causafera_core::HistoryError::ColdRecordMismatch { trace_id })
            if trace_id == TraceId::new(3)
    ));
    println!("indexed_overlay exact=2 frames=2 empty=accepted mismatch=rejected unknown=rejected");
}

#[test]
fn history_store_creates_bundle_and_reopens_durable_generation() {
    // Given
    let temp = TempBundle::new("create-reopen");
    let store = HistoryStore::create(temp.path(), [0x41; 32], DurabilityMode::EveryTick)
        .expect("create history bundle");

    // When
    drop(store);
    let reopened = HistoryStore::open(temp.path()).expect("reopen history bundle");

    // Then
    for entry in [
        "catalog.sqlite",
        "segments",
        "staging",
        "checkpoints",
        "snapshots",
        "migration",
    ] {
        assert!(
            temp.path().join(entry).exists(),
            "missing bundle entry {entry}"
        );
    }
    assert_eq!(reopened.logical_generation(), 0);
    assert_eq!(reopened.durable_generation(), 0);
    assert_eq!(reopened.catalog().durable_generation().expect("catalog"), 0);
    let settings = reopened.catalog().settings().expect("catalog settings");
    assert_eq!(settings.journal_mode.to_ascii_lowercase(), "wal");
    assert_eq!(settings.synchronous, 2);
    assert!(settings.foreign_keys);
    println!(
        "create_reopen logical=0 durable=0 journal=wal synchronous=FULL foreign_keys=ON bundle={}",
        temp.path().display()
    );
}

#[test]
fn history_store_advances_logical_generation_without_publishing_a_frame() {
    // Given
    let temp = TempBundle::new("logical-generation-gap");
    let mut store = HistoryStore::create(temp.path(), [0x44; 32], DurabilityMode::EveryTick)
        .expect("create history bundle");
    store
        .append_tick(vec![causal(1, vec![])])
        .expect("publish nonempty frame");
    let before = store.status().expect("read preimage status");

    // When
    store
        .advance_logical_generation(3)
        .expect("advance metadata generation");

    // Then
    let after = store.status().expect("read advanced status");
    assert_eq!(after.logical_generation, 3);
    assert_eq!(after.durable_generation, 3);
    assert_eq!(after.catalog_counts.generations, 4);
    assert_eq!(after.catalog_counts.frames, before.catalog_counts.frames);
    assert_eq!(after.logical_records, before.logical_records);
    assert_eq!(after.durable_records, before.durable_records);
    assert_eq!(after.logical_digest, before.logical_digest);
    assert_eq!(after.durable_digest, before.durable_digest);
    assert_eq!(
        after.evidence_manifest_digest,
        before.evidence_manifest_digest
    );
    let reader = store.durable_reader().expect("read advanced generation");
    assert_eq!(reader.anchor().expect("anchor").logical_generation(), 3);
    assert_eq!(reader.all_records_for_oracle().expect("records").len(), 1);

    drop(store);
    let reopened = HistoryStore::open(temp.path()).expect("reopen advanced bundle");
    let reopened_status = reopened.status().expect("read reopened status");
    assert_eq!(reopened_status.logical_generation, 3);
    assert_eq!(reopened_status.durable_generation, 3);
    assert_eq!(reopened_status.catalog_counts.generations, 4);
    assert_eq!(reopened_status.catalog_counts.frames, 1);
    assert_eq!(reopened_status.logical_records, 1);
    assert_eq!(
        reopened
            .durable_reader()
            .expect("reopened reader")
            .all_records_for_oracle()
            .expect("reopened records")
            .len(),
        1
    );
}

#[test]
fn history_store_logical_generation_advance_failure_is_atomic() {
    // Given
    let temp = TempBundle::new("logical-generation-gap-failure");
    let hooks = Arc::new(TestHooks::default());
    let mut store = HistoryStore::create_with_hooks(
        temp.path(),
        [0x45; 32],
        DurabilityMode::EveryTick,
        hooks.clone(),
    )
    .expect("create history bundle");
    store
        .append_tick(vec![causal(1, vec![])])
        .expect("publish nonempty frame");
    let before = store.status().expect("read preimage status");

    // When
    hooks.fail_once(FailurePoint::DuringCatalogCommit);
    let error = store
        .advance_logical_generation(3)
        .expect_err("injected catalog failure must reject the advance");

    // Then
    assert!(matches!(
        error,
        PersistenceError::InjectedHistoryFailure {
            point: FailurePoint::DuringCatalogCommit
        }
    ));
    let after = store.status().expect("read rolled-back status");
    assert_eq!(authoritative_status(after), authoritative_status(before));
    drop(store);
    let reopened = HistoryStore::open(temp.path()).expect("reopen rolled-back bundle");
    let reopened_status = reopened.status().expect("read reopened status");
    assert_eq!(reopened_status.logical_generation, 1);
    assert_eq!(reopened_status.durable_generation, 1);
    assert_eq!(reopened_status.catalog_counts.generations, 2);
    assert_eq!(reopened_status.catalog_counts.frames, 1);
    assert_eq!(reopened_status.logical_records, 1);
}

#[test]
fn history_store_rejects_second_writer_and_preserves_ordering() {
    // Given
    let temp = TempBundle::new("writer-lock");
    let mut first = HistoryStore::create(temp.path(), [0x42; 32], DurabilityMode::EveryTick)
        .expect("first writer");

    // When
    let error = HistoryStore::open(temp.path()).expect_err("second writer must be rejected");
    first
        .append_tick(vec![causal(1, vec![])])
        .expect("first tick");
    first
        .append_tick(vec![causal(2, vec![TraceId::new(1)])])
        .expect("second tick");

    // Then
    assert!(matches!(
        error,
        PersistenceError::HistoryWriterLocked { .. }
    ));
    let ids = first
        .durable_reader()
        .expect("durable reader")
        .records(query())
        .expect("records")
        .records()
        .iter()
        .map(|record| match record {
            HistoryRecord::Causal(record) => record.trace_id().raw(),
            HistoryRecord::Evidence(_) => 0,
        })
        .collect::<Vec<_>>();
    assert_eq!(ids, vec![1, 2]);
}

#[test]
fn history_store_writer_lock_child_process() {
    let Ok(root) = env::var(WRITER_LOCK_CHILD_BUNDLE) else {
        return;
    };

    let error = HistoryStore::open(Path::new(&root)).expect_err("cross-process writer rejected");
    assert!(matches!(
        error,
        PersistenceError::HistoryWriterLocked { .. }
    ));
    println!("cross_process_writer_rejection=HistoryWriterLocked bundle={root}");
}

#[test]
fn history_store_rejects_cross_process_writer() {
    // Given
    let temp = TempBundle::new("cross-process-writer-lock");
    let _first = HistoryStore::create(temp.path(), [0x53; 32], DurabilityMode::EveryTick)
        .expect("first writer");

    // When
    let output = Command::new(env::current_exe().expect("current integration test binary"))
        .arg("--exact")
        .arg("history_store_writer_lock_child_process")
        .arg("--nocapture")
        .env(WRITER_LOCK_CHILD_BUNDLE, temp.path())
        .output()
        .expect("run independent writer process");

    // Then
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "independent writer process unexpectedly opened bundle\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
    assert!(stdout.contains("cross_process_writer_rejection=HistoryWriterLocked"));
    println!("{stdout}");
}

#[test]
fn history_store_group_overlay_is_logical_but_not_durable() {
    // Given
    let temp = TempBundle::new("overlay");
    let hooks = Arc::new(TestHooks::default());
    let mut store = HistoryStore::create_with_hooks(
        temp.path(),
        [0x43; 32],
        DurabilityMode::GroupCommit,
        hooks.clone(),
    )
    .expect("group store");

    // When
    let receipt = store
        .append_tick(vec![causal(1, vec![])])
        .expect("logical tick");

    // Then
    assert_eq!(
        (receipt.logical_generation(), receipt.durable_generation()),
        (1, 0)
    );
    assert_eq!(
        store
            .logical_reader()
            .expect("logical reader")
            .records(query())
            .expect("records")
            .records()
            .len(),
        1
    );
    assert_eq!(
        store
            .durable_reader()
            .expect("durable reader")
            .records(query())
            .expect("records")
            .records()
            .len(),
        0
    );
    assert_eq!(hooks.flushes.load(Ordering::SeqCst), 0);
    assert_eq!(hooks.segment_syncs.load(Ordering::SeqCst), 0);
    assert_eq!(hooks.catalog_commits.load(Ordering::SeqCst), 0);
    println!("group_overlay logical=1 durable=0 overlay=1 durable_records=0 syncs=0 commits=0");
}

#[test]
fn history_store_group_commit_honors_tick_and_elapsed_bounds() {
    // Given
    let temp = TempBundle::new("group-bounds");
    let hooks = Arc::new(TestHooks::default());
    let mut store = HistoryStore::create_with_hooks(
        temp.path(),
        [0x44; 32],
        DurabilityMode::GroupCommit,
        hooks.clone(),
    )
    .expect("group store");

    // When
    for sequence in 1..=31 {
        store
            .append_tick(vec![causal(
                sequence,
                if sequence == 1 {
                    vec![]
                } else {
                    vec![TraceId::new(sequence - 1)]
                },
            )])
            .expect("bounded overlay tick");
    }
    let before = store.durable_generation();
    store
        .append_tick(vec![causal(32, vec![TraceId::new(31)])])
        .expect("tick bound force");

    // Then
    assert_eq!(before, 0);
    assert_eq!(store.durable_generation(), 32);
    assert_eq!(hooks.segment_syncs.load(Ordering::SeqCst), 1);
    assert_eq!(hooks.catalog_commits.load(Ordering::SeqCst), 1);
    store
        .append_tick(vec![causal(33, vec![TraceId::new(32)])])
        .expect("new overlay");
    hooks.advance(250);
    assert!(store.flush_if_due().expect("elapsed force"));
    assert_eq!(store.durable_generation(), 33);
    assert_eq!(hooks.segment_syncs.load(Ordering::SeqCst), 2);
    println!(
        "group_bounds generations=[32,33] max_ticks=32 max_ms=250 segment_syncs=2 catalog_commits=2 timer_poll=true"
    );
}

#[test]
fn history_store_every_tick_publishes_verified_frame_and_indexes() {
    // Given
    let temp = TempBundle::new("every-tick");
    let hooks = Arc::new(TestHooks::default());
    let mut store = HistoryStore::create_with_hooks(
        temp.path(),
        [0x45; 32],
        DurabilityMode::EveryTick,
        hooks.clone(),
    )
    .expect("every-tick store");
    let records = vec![causal(1, vec![]), evidence(1, 1, b"rain/a")];

    // When
    let receipt = store.append_tick(records).expect("durable tick");
    let reader = store.durable_reader().expect("durable reader");

    // Then
    assert_eq!(
        (receipt.logical_generation(), receipt.durable_generation()),
        (1, 1)
    );
    assert_eq!(receipt.range().start, 0);
    assert!(receipt.range().end > receipt.range().start);
    assert_eq!(reader.records(query()).expect("records").records().len(), 2);
    assert_eq!(
        reader
            .event(TraceId::new(1))
            .expect("event")
            .expect("present")
            .event_id(),
        EventId::new(1)
    );
    assert_eq!(
        reader
            .evidence(EvidenceSchemaId::new(0xABCD), b"rain/a", query())
            .expect("evidence")
            .records()
            .len(),
        1
    );
    assert_eq!(hooks.flushes.load(Ordering::SeqCst), 1);
    assert_eq!(hooks.segment_syncs.load(Ordering::SeqCst), 1);
    assert_eq!(hooks.catalog_commits.load(Ordering::SeqCst), 1);
    println!(
        "every_tick generation=1 range={}..{} digest={} syncs=1 commits=1",
        receipt.range().start,
        receipt.range().end,
        receipt.frame_digest_hex()
    );
}

#[test]
fn history_store_manifest_digest_tracks_authoritative_evidence_and_checkpoint_reopen() {
    // Given: one durable evidence record at a verified causal frontier.
    let temp = TempBundle::new("evidence-manifest");
    let mut store =
        HistoryStore::create(temp.path(), [0x58; 32], DurabilityMode::EveryTick).expect("store");
    store
        .append_tick(vec![causal(1, vec![]), evidence(1, 1, b"rain/a")])
        .expect("first evidence generation");
    let first = store
        .durable_reader()
        .expect("first reader")
        .anchor()
        .expect("first anchor");

    // When: authoritative evidence changes and the resulting anchor is checkpointed.
    store
        .append_tick(vec![
            causal(2, vec![TraceId::new(1)]),
            evidence(2, 2, b"rain/a"),
        ])
        .expect("second evidence generation");
    let second = store
        .durable_reader()
        .expect("second reader")
        .anchor()
        .expect("second anchor");
    store
        .publish_checkpoint(
            &second,
            &checkpoint_metadata(second.frontier().time),
            b"bounded-current-state",
        )
        .expect("checkpoint with manifest");
    drop(store);
    let reopened = HistoryStore::open(temp.path()).expect("reopen verified manifest bundle");
    let checkpoint = reopened
        .checkpoint_at_durable_generation(2)
        .expect("checkpoint lookup")
        .expect("checkpoint present");
    let reopened_anchor = reopened
        .durable_reader()
        .expect("reopened durable reader")
        .anchor()
        .expect("reopened durable anchor");

    // Then: nonempty evidence is nonzero, content-sensitive, and verified on reopen.
    assert_ne!(first.evidence_manifest_digest(), &[0; 32]);
    assert_ne!(
        first.evidence_manifest_digest(),
        second.evidence_manifest_digest()
    );
    assert_eq!(
        checkpoint.header.evidence_manifest_digest,
        *second.evidence_manifest_digest()
    );
    assert_eq!(
        reopened_anchor.evidence_manifest_digest(),
        second.evidence_manifest_digest()
    );
    println!(
        "evidence_manifest nonempty=true nonzero=true changed=true checkpoint_verified=true reopen_verified=true first={} second={}",
        first
            .evidence_manifest_digest()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>(),
        second
            .evidence_manifest_digest()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    );
}

#[test]
fn history_store_empty_evidence_manifest_is_canonical() {
    // Given
    let temp = TempBundle::new("empty-evidence-manifest");
    let store =
        HistoryStore::create(temp.path(), [0x55; 32], DurabilityMode::EveryTick).expect("store");

    // When
    let digest = *store
        .durable_reader()
        .expect("empty durable reader")
        .anchor()
        .expect("empty durable anchor")
        .evidence_manifest_digest();

    // Then
    assert_eq!(digest, EMPTY_EVIDENCE_MANIFEST_DIGEST);
    println!(
        "evidence_manifest empty_digest={}",
        digest
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    );
}

#[test]
fn history_store_rejects_stale_evidence_manifest_at_checkpoint_publication() {
    // Given
    let temp = TempBundle::new("stale-evidence-manifest-publication");
    let mut store =
        HistoryStore::create(temp.path(), [0x57; 32], DurabilityMode::EveryTick).expect("store");
    store
        .append_tick(vec![causal(1, vec![]), evidence(1, 1, b"rain/a")])
        .expect("durable evidence");
    let anchor = store
        .durable_reader()
        .expect("durable reader")
        .anchor()
        .expect("durable anchor");
    let stale = anchor_with_manifest_digest(&anchor, [0xEE; 32]);

    // When
    let error = store
        .publish_checkpoint(
            &stale,
            &checkpoint_metadata(stale.frontier().time),
            b"bounded-current-state",
        )
        .expect_err("stale evidence manifest must not be published");

    // Then
    assert!(matches!(
        error,
        PersistenceError::HistoryCatalogInvariant {
            invariant: "checkpoint evidence manifest does not match durable history"
        }
    ));
    assert_eq!(
        store
            .catalog()
            .counts()
            .expect("catalog counts")
            .checkpoints,
        0
    );
    println!("evidence_manifest publication_mismatch_rejected=true checkpoint_catalog_rows=0");
}

#[test]
fn history_store_rejects_stale_evidence_manifest_on_checkpoint_reopen() {
    // Given
    let temp = TempBundle::new("stale-evidence-manifest-reopen");
    let mut store =
        HistoryStore::create(temp.path(), [0x56; 32], DurabilityMode::EveryTick).expect("store");
    store
        .append_tick(vec![causal(1, vec![]), evidence(1, 1, b"rain/a")])
        .expect("durable evidence");
    let anchor = store
        .durable_reader()
        .expect("durable reader")
        .anchor()
        .expect("durable anchor");
    let stale = anchor_with_manifest_digest(&anchor, [0xEE; 32]);
    let bytes = encode_checkpoint(
        &stale,
        &checkpoint_metadata(stale.frontier().time),
        b"bounded-current-state",
    )
    .expect("encode stale checkpoint fixture");
    let envelope = decode_checkpoint(&bytes).expect("valid content-addressed checkpoint");
    let digest_hex = envelope
        .digest
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    let relative_path = PathBuf::from("checkpoints").join(format!("{digest_hex}.ckp"));
    fs::write(temp.path().join(&relative_path), bytes).expect("write stale checkpoint fixture");
    drop(store);
    let connection =
        rusqlite::Connection::open(temp.path().join("catalog.sqlite")).expect("catalog");
    connection
        .execute(
            "INSERT INTO checkpoints(generation,time,digest,relative_path) VALUES (1,?1,?2,?3)",
            rusqlite::params![
                anchor.frontier().time.raw().to_be_bytes(),
                envelope.digest.as_slice(),
                relative_path.to_str().expect("UTF-8 checkpoint path")
            ],
        )
        .expect("link stale checkpoint fixture");
    connection
        .execute(
            "UPDATE generations SET checkpoint_generation=1 WHERE id=1",
            [],
        )
        .expect("reference stale checkpoint fixture");
    drop(connection);

    // When
    let error = HistoryStore::open(temp.path()).expect_err("stale manifest must fail reopen");

    // Then
    assert!(matches!(
        error,
        PersistenceError::HistoryCatalogInvariant {
            invariant: "checkpoint evidence manifest does not match durable history"
        }
    ));
    println!("evidence_manifest reopen_mismatch_rejected=true outer_digest_valid=true");
}

#[test]
fn history_store_publishes_checkpoint_file_and_catalog_link() {
    // Given
    let temp = TempBundle::new("checkpoint-publication");
    let mut store =
        HistoryStore::create(temp.path(), [0x59; 32], DurabilityMode::EveryTick).expect("store");
    store.append_tick(vec![causal(1, vec![])]).expect("tick");
    let anchor = store
        .durable_reader()
        .expect("durable reader")
        .anchor()
        .expect("durable anchor");
    let payload = b"bounded-current-state";

    // When
    let receipt = store
        .publish_checkpoint(
            &anchor,
            &checkpoint_metadata(anchor.frontier().time),
            payload,
        )
        .expect("publish checkpoint");
    store
        .append_tick(vec![causal(2, vec![TraceId::new(1)])])
        .expect("subsequent tick");
    drop(store);
    let reopened = HistoryStore::open(temp.path()).expect("reopen checkpoint bundle");
    let checkpoint = reopened
        .checkpoint_at_durable_generation(2)
        .expect("checkpoint readback")
        .expect("linked checkpoint");
    let reopened_anchor = reopened
        .reader_at_durable_generation(2)
        .expect("generation reader")
        .anchor()
        .expect("generation anchor");

    // Then
    assert_eq!(receipt.generation(), 1);
    assert!(receipt.path().is_file());
    assert_eq!(
        reopened
            .catalog()
            .counts()
            .expect("catalog counts")
            .checkpoints,
        1
    );
    assert_eq!(checkpoint.payload, payload);
    assert_eq!(checkpoint.digest, *receipt.digest());
    assert_eq!(checkpoint.header.logical_generation, 1);
    assert_eq!(checkpoint.header.durable_generation, 1);
    assert_eq!(reopened_anchor.checkpoint_digest(), receipt.digest());
    println!(
        "checkpoint_publication checkpoint_generation=1 read_generation=2 digest={} file={} catalog_rows=1 inherited_link=true payload_bytes={}",
        receipt.digest_hex(),
        receipt.path().display(),
        checkpoint.payload.len()
    );
}

#[test]
fn history_store_rejects_corrupt_linked_checkpoint_on_reopen() {
    // Given
    let temp = TempBundle::new("corrupt-checkpoint");
    let mut store =
        HistoryStore::create(temp.path(), [0x5A; 32], DurabilityMode::EveryTick).expect("store");
    store.append_tick(vec![causal(1, vec![])]).expect("tick");
    let anchor = store
        .durable_reader()
        .expect("reader")
        .anchor()
        .expect("anchor");
    let receipt = store
        .publish_checkpoint(
            &anchor,
            &checkpoint_metadata(anchor.frontier().time),
            b"current-state",
        )
        .expect("checkpoint");
    let path = receipt.path().to_path_buf();
    drop(store);
    let mut bytes = fs::read(&path).expect("checkpoint bytes");
    let last = bytes.len() - 1;
    bytes[last] ^= 0x01;
    fs::write(&path, bytes).expect("corrupt checkpoint");

    // When
    let error = HistoryStore::open(temp.path()).expect_err("corrupt checkpoint must fail closed");

    // Then
    assert!(matches!(error, PersistenceError::CheckpointDigestMismatch));
    println!("checkpoint_corruption linked_file=true result=CheckpointDigestMismatch");
}

#[test]
fn history_store_rejects_truncated_linked_checkpoint_on_reopen() {
    // Given
    let temp = TempBundle::new("truncated-checkpoint");
    let mut store =
        HistoryStore::create(temp.path(), [0x5B; 32], DurabilityMode::EveryTick).expect("store");
    store.append_tick(vec![causal(1, vec![])]).expect("tick");
    let anchor = store
        .durable_reader()
        .expect("reader")
        .anchor()
        .expect("anchor");
    let receipt = store
        .publish_checkpoint(
            &anchor,
            &checkpoint_metadata(anchor.frontier().time),
            b"current-state",
        )
        .expect("checkpoint");
    let path = receipt.path().to_path_buf();
    drop(store);
    let length = path.metadata().expect("checkpoint metadata").len();
    OpenOptions::new()
        .write(true)
        .open(&path)
        .expect("checkpoint file")
        .set_len(length - 1)
        .expect("truncate checkpoint");

    // When
    let error = HistoryStore::open(temp.path()).expect_err("truncated checkpoint must fail closed");

    // Then
    assert!(matches!(error, PersistenceError::Codec { .. }));
    println!("checkpoint_corruption linked_file=true truncated=true result=Codec");
}

#[test]
fn history_store_rejects_stale_checkpoint_generation_link_on_reopen() {
    // Given
    let temp = TempBundle::new("stale-checkpoint-link");
    let mut store =
        HistoryStore::create(temp.path(), [0x5C; 32], DurabilityMode::EveryTick).expect("store");
    store
        .append_tick(vec![causal(1, vec![])])
        .expect("first tick");
    let anchor = store
        .durable_reader()
        .expect("reader")
        .anchor()
        .expect("anchor");
    store
        .publish_checkpoint(
            &anchor,
            &checkpoint_metadata(anchor.frontier().time),
            b"current-state",
        )
        .expect("checkpoint");
    store
        .append_tick(vec![causal(2, vec![TraceId::new(1)])])
        .expect("second tick");
    drop(store);
    let connection =
        rusqlite::Connection::open(temp.path().join("catalog.sqlite")).expect("catalog");
    connection
        .execute(
            "UPDATE generations SET checkpoint_generation=NULL WHERE id=2",
            [],
        )
        .expect("stale checkpoint link");
    drop(connection);

    // When
    let error = HistoryStore::open(temp.path()).expect_err("stale link must fail closed");

    // Then
    assert!(matches!(
        error,
        PersistenceError::HistoryCatalogInvariant {
            invariant: "durable generation checkpoint link is stale"
        }
    ));
    println!("checkpoint_catalog generation=2 stale_link=true result=typed_catalog_invariant");
}

#[test]
fn history_store_checkpoint_publication_resumes_after_interruption() {
    // Given
    let temp = TempBundle::new("checkpoint-resume");
    let hooks = Arc::new(TestHooks::default());
    let mut store = HistoryStore::create_with_hooks(
        temp.path(),
        [0x5D; 32],
        DurabilityMode::EveryTick,
        hooks.clone(),
    )
    .expect("store");
    store.append_tick(vec![causal(1, vec![])]).expect("tick");
    let anchor = store
        .durable_reader()
        .expect("reader")
        .anchor()
        .expect("anchor");
    hooks.fail_once(FailurePoint::AfterCheckpointSyncBeforeCatalogCommit);

    // When
    let error = store
        .publish_checkpoint(
            &anchor,
            &checkpoint_metadata(anchor.frontier().time),
            b"current-state",
        )
        .expect_err("interrupted publication");
    assert!(matches!(
        error,
        PersistenceError::InjectedHistoryFailure {
            point: FailurePoint::AfterCheckpointSyncBeforeCatalogCommit
        }
    ));
    assert_eq!(store.catalog().counts().expect("counts").checkpoints, 0);
    drop(store);
    let mut reopened = HistoryStore::open(temp.path()).expect("reopen orphan checkpoint bundle");
    let retry_anchor = reopened
        .durable_reader()
        .expect("reader")
        .anchor()
        .expect("anchor");
    let receipt = reopened
        .publish_checkpoint(
            &retry_anchor,
            &checkpoint_metadata(retry_anchor.frontier().time),
            b"current-state",
        )
        .expect("resume checkpoint publication");

    // Then
    assert_eq!(reopened.durable_generation(), 1);
    assert_eq!(reopened.catalog().counts().expect("counts").checkpoints, 1);
    assert!(receipt.path().is_file());
    println!(
        "checkpoint_interruption boundary=after_file_sync_before_catalog_commit previous_generation=1 partial_catalog_rows=0 retry_generation=1 resumed=true digest={}",
        receipt.digest_hex()
    );
}

#[test]
fn history_store_post_commit_checkpoint_failure_is_reversible() {
    let temp = TempBundle::new("post-checkpoint-commit");
    let hooks = Arc::new(TestHooks::default());
    let mut store = HistoryStore::create_with_hooks(
        temp.path(),
        [0x5E; 32],
        DurabilityMode::EveryTick,
        hooks.clone(),
    )
    .expect("store");
    store.append_tick(vec![causal(1, vec![])]).expect("tick");
    let anchor = store
        .durable_reader()
        .expect("reader")
        .anchor()
        .expect("anchor");
    let metadata = checkpoint_metadata(anchor.frontier().time);
    for attempt in 1..=2 {
        let before = store.status().expect("status before checkpoint");
        let directory_syncs_before = hooks.directory_syncs.load(Ordering::SeqCst);
        hooks.fail_once(FailurePoint::AfterCheckpointCatalogCommitAfterRootSync);

        let error = store
            .publish_checkpoint(&anchor, &metadata, b"post-commit-checkpoint")
            .expect_err("post-commit checkpoint failure");
        assert!(matches!(
            error,
            PersistenceError::InjectedHistoryFailure {
                point: FailurePoint::AfterCheckpointCatalogCommitAfterRootSync
            }
        ));
        assert_eq!(
            authoritative_status(store.status().expect("status after failure")),
            authoritative_status(before)
        );
        assert_eq!(
            fs::read_dir(temp.path().join("checkpoints"))
                .expect("checkpoint directory")
                .count(),
            0
        );
        let directory_syncs_after = hooks.directory_syncs.load(Ordering::SeqCst);
        assert_eq!(directory_syncs_after - directory_syncs_before, 4);
        println!(
            "post_backend_failure point={} attempt={} after_root_sync=true directory_syncs_delta={} unchanged=true catalog_rows_unchanged=true no_orphan_file=true",
            FailurePoint::AfterCheckpointCatalogCommitAfterRootSync.name(),
            attempt,
            directory_syncs_after - directory_syncs_before
        );
    }

    let receipt = store
        .publish_checkpoint(&anchor, &metadata, b"post-commit-checkpoint")
        .expect("same-process checkpoint retry");
    let after_retry = store.status().expect("status after retry");
    assert_eq!(after_retry.catalog_counts.checkpoints, 1);
    assert!(receipt.path().is_file());
    drop(store);
    let reopened = HistoryStore::open(temp.path()).expect("reopen checkpoint");
    assert_eq!(
        authoritative_status(reopened.status().expect("reopened status")),
        authoritative_status(after_retry)
    );
    println!(
        "post_backend_retry point={} same_process=true reopen=true",
        FailurePoint::AfterCheckpointCatalogCommitAfterRootSync.name()
    );
}

#[test]
fn history_store_checkpoint_forces_pending_group_before_publication() {
    // Given
    let temp = TempBundle::new("checkpoint-forces-group");
    let hooks = Arc::new(TestHooks::default());
    let mut store = HistoryStore::create_with_hooks(
        temp.path(),
        [0x5E; 32],
        DurabilityMode::GroupCommit,
        hooks.clone(),
    )
    .expect("store");
    store
        .append_tick(vec![causal(1, vec![])])
        .expect("logical tick");
    let anchor = store
        .logical_reader()
        .expect("logical reader")
        .anchor()
        .expect("logical anchor");
    assert_eq!(
        (anchor.logical_generation(), anchor.durable_generation()),
        (1, 0)
    );

    // When
    let receipt = store
        .publish_checkpoint(
            &anchor,
            &checkpoint_metadata(anchor.frontier().time),
            b"current-state",
        )
        .expect("forced checkpoint");

    // Then
    assert_eq!(receipt.generation(), 1);
    assert_eq!(store.durable_generation(), 1);
    assert_eq!(hooks.segment_syncs.load(Ordering::SeqCst), 1);
    assert_eq!(hooks.catalog_commits.load(Ordering::SeqCst), 1);
    assert_eq!(store.catalog().counts().expect("counts").checkpoints, 1);
    println!(
        "checkpoint_explicit_save pending_generation=1 forced_durable=1 frame_syncs=1 catalog_commits=1 checkpoint_rows=1"
    );
}

#[test]
fn history_store_pages_parent_child_evidence_and_latest_before_deterministically() {
    // Given
    let temp = TempBundle::new("queries");
    let mut store =
        HistoryStore::create(temp.path(), [0x46; 32], DurabilityMode::EveryTick).expect("store");
    store
        .append_tick(vec![causal(1, vec![]), evidence(1, 1, b"rain/a")])
        .expect("first frame");
    store
        .append_tick(vec![
            causal(2, vec![TraceId::new(1)]),
            evidence(2, 2, b"rain/a"),
        ])
        .expect("second frame");

    // When
    let reader = store.durable_reader().expect("reader");
    let target = CausalTarget::new(StateObjectKindId::new(7), 102, StatePropertyId::new(9));

    // Then
    assert_eq!(
        reader
            .parents(TraceId::new(2), query())
            .expect("parents")
            .records(),
        &[TraceId::new(1)]
    );
    assert_eq!(
        reader
            .children(TraceId::new(1), query())
            .expect("children")
            .records(),
        &[TraceId::new(2)]
    );
    assert_eq!(
        reader
            .evidence(EvidenceSchemaId::new(0xABCD), b"rain/a", query())
            .expect("evidence")
            .records()
            .len(),
        2
    );
    assert_eq!(
        reader
            .latest_matching_event_before(target, TraceId::new(3))
            .expect("latest")
            .expect("event")
            .trace_id(),
        TraceId::new(2)
    );
    let one = HistoryQuery::new(0, 1, 4 * 1024 * 1024).expect("one-record page");
    assert_eq!(reader.records(one).expect("page").next_offset(), Some(1));
    assert_eq!(
        store
            .reader_at_durable_generation(1)
            .expect("generation one")
            .records(query())
            .expect("records")
            .records()
            .len(),
        2
    );
    assert!(matches!(
        store.reader_at_durable_generation(3),
        Err(PersistenceError::HistoryGenerationUnavailable { generation: 3 })
    ));
    println!(
        "query_order traces=[1,2] parents(2)=[1] children(1)=[2] evidence=2 latest_before_3=2"
    );
}

#[test]
fn history_store_seals_at_four_mib_only_after_complete_frames() {
    // Given
    let temp = TempBundle::new("automatic-seal");
    let hooks = Arc::new(TestHooks::default());
    let mut store = HistoryStore::create_with_hooks(
        temp.path(),
        [0x4D; 32],
        DurabilityMode::GroupCommit,
        hooks,
    )
    .expect("store");

    // When
    for sequence in 1..=128 {
        store
            .append_tick(vec![large_evidence(sequence)])
            .expect("large frame");
    }

    // Then
    let counts = store.catalog().counts().expect("catalog counts");
    assert_eq!(store.durable_generation(), 128);
    assert_eq!(counts.frames, 128);
    assert_eq!(counts.segments, 2);
    assert_eq!(
        store
            .active_segment_path()
            .metadata()
            .expect("new active")
            .len(),
        0
    );
    let reader = store.durable_reader().expect("reader");
    let first_page = reader.records(query()).expect("first page");
    assert_eq!(first_page.records().len(), 120);
    assert_eq!(first_page.next_offset(), Some(120));
    let second_query = HistoryQuery::new(120, 256, 4 * 1024 * 1024).expect("second page");
    assert_eq!(
        reader
            .records(second_query)
            .expect("second page")
            .records()
            .len(),
        8
    );
    println!(
        "automatic_seal target_bytes=4194304 complete_frames=128 segments=2 durable_generation=128 pages=[120,8]"
    );
}

#[test]
fn history_store_auto_seal_failure_restores_active_file_and_catalog() {
    let temp = TempBundle::new("automatic-seal-failure");
    let hooks = Arc::new(TestHooks::default());
    let mut store = HistoryStore::create_with_hooks(
        temp.path(),
        [0x4E; 32],
        DurabilityMode::EveryTick,
        hooks.clone(),
    )
    .expect("store");
    hooks.fail_once(FailurePoint::SealRename);

    let (failed_sequence, before) = loop {
        let sequence = u64::try_from(store.status().expect("status").logical_records)
            .expect("sequence")
            .checked_add(1)
            .expect("sequence increment");
        let before = store.status().expect("status before append");
        match store.append_tick(vec![large_evidence(sequence)]) {
            Ok(_) => {}
            Err(PersistenceError::InjectedHistoryFailure {
                point: FailurePoint::SealRename,
            }) => break (sequence, before),
            Err(error) => panic!("unexpected automatic seal error: {error:?}"),
        }
    };

    assert_eq!(
        authoritative_status(store.status().expect("status after seal failure")),
        authoritative_status(before)
    );
    let active = store.active_segment_path();
    assert!(active.is_file());
    assert_eq!(store.catalog().counts().expect("catalog").segments, 1);
    assert_eq!(
        store.catalog().counts().expect("catalog").frames,
        u64::try_from(before.durable_records).expect("catalog frame count"),
    );

    store
        .append_tick(vec![large_evidence(failed_sequence)])
        .expect("automatic seal retry");
    let after = store.status().expect("status after retry");
    assert_eq!(after.logical_records, before.logical_records + 1);
    assert_eq!(after.durable_records, after.logical_records);
    assert_eq!(after.catalog_counts.segments, 2);
    drop(store);
    let reopened = HistoryStore::open(temp.path()).expect("reopen after automatic seal retry");
    assert_eq!(
        authoritative_status(reopened.status().expect("reopened status")),
        authoritative_status(after)
    );
    println!(
        "automatic_seal_failure point=seal_rename failed_sequence={} unchanged=true catalog_rows_unchanged=true retry=true reopen=true",
        failed_sequence
    );
}

#[test]
fn history_store_exercises_representative_group_and_every_tick_frame_counts() {
    const GROUP_FRAMES: u64 = 10_000;
    const EVERY_TICK_FRAMES: u64 = 10_000;

    // Given
    let group_temp = TempBundle::new("representative-group");
    let every_temp = TempBundle::new("representative-every");
    let group_hooks = Arc::new(TestHooks::default());
    let every_hooks = Arc::new(TestHooks::default());
    let mut group = HistoryStore::create_with_hooks(
        group_temp.path(),
        [0x4E; 32],
        DurabilityMode::GroupCommit,
        group_hooks.clone(),
    )
    .expect("group store");
    let mut every = HistoryStore::create_with_hooks(
        every_temp.path(),
        [0x4F; 32],
        DurabilityMode::EveryTick,
        every_hooks.clone(),
    )
    .expect("every store");

    // When
    let group_started = Instant::now();
    for sequence in 1..=GROUP_FRAMES {
        group
            .append_tick(long_run_records(sequence))
            .expect("group frame");
    }
    group.flush_durable().expect("final group flush");
    let group_elapsed = group_started.elapsed();
    let every_started = Instant::now();
    for sequence in 1..=EVERY_TICK_FRAMES {
        every
            .append_tick(long_run_records(sequence))
            .expect("every frame");
    }
    let every_elapsed = every_started.elapsed();
    let group_sealed = group.seal_active().expect("seal group segment");
    let every_sealed = every.seal_active().expect("seal every-tick segment");
    drop(group);
    drop(every);
    let group = HistoryStore::open(group_temp.path()).expect("reopen group bundle");
    let every = HistoryStore::open(every_temp.path()).expect("reopen every-tick bundle");

    // Then
    assert_eq!(group.durable_generation(), GROUP_FRAMES);
    assert_eq!(every.durable_generation(), EVERY_TICK_FRAMES);
    assert_eq!(group_hooks.catalog_commits.load(Ordering::SeqCst), 313);
    assert_eq!(
        every_hooks.catalog_commits.load(Ordering::SeqCst),
        usize::try_from(EVERY_TICK_FRAMES).expect("representative count")
    );
    assert_eq!(
        group.catalog().counts().expect("group counts").frames,
        GROUP_FRAMES
    );
    assert_eq!(
        every.catalog().counts().expect("every counts").frames,
        EVERY_TICK_FRAMES
    );
    assert_long_run_reads(&group, GROUP_FRAMES);
    assert_long_run_reads(&every, EVERY_TICK_FRAMES);
    println!(
        "long_run frames_per_mode=10000 group_generation=10000 group_frames=10000 group_commits=313 group_segment_syncs={} group_directory_syncs={} every_generation=10000 every_frames=10000 every_commits=10000 every_segment_syncs={} every_directory_syncs={} group_elapsed_ms={} every_elapsed_ms={} group_seal={} every_seal={} indexed_readback=true",
        group_hooks.segment_syncs.load(Ordering::SeqCst),
        group_hooks.directory_syncs.load(Ordering::SeqCst),
        every_hooks.segment_syncs.load(Ordering::SeqCst),
        every_hooks.directory_syncs.load(Ordering::SeqCst),
        group_elapsed.as_millis(),
        every_elapsed.as_millis(),
        group_sealed.identity_hex(),
        every_sealed.identity_hex(),
    );
}

fn long_run_records(sequence: u64) -> Vec<HistoryRecord> {
    let mut records = vec![causal(
        sequence,
        if sequence == 1 {
            vec![]
        } else {
            vec![TraceId::new(sequence - 1)]
        },
    )];
    if sequence.is_multiple_of(1_000) {
        records.push(evidence(sequence, sequence, b"long-run"));
    }
    records
}

fn assert_long_run_reads(store: &HistoryStore, frames: u64) {
    let reader = store.durable_reader().expect("long-run reader");
    assert_eq!(
        reader
            .event(TraceId::new(frames))
            .expect("indexed event")
            .expect("event present")
            .event_id(),
        EventId::new(frames)
    );
    assert_eq!(
        reader
            .parents(TraceId::new(frames), query())
            .expect("indexed parents")
            .records(),
        &[TraceId::new(frames - 1)]
    );
    assert_eq!(
        reader
            .children(TraceId::new(frames - 1), query())
            .expect("indexed children")
            .records(),
        &[TraceId::new(frames)]
    );
    let expected_evidence = usize::try_from(frames / 1_000).expect("evidence count");
    assert_eq!(
        reader
            .evidence(EvidenceSchemaId::new(0xABCD), b"long-run", query())
            .expect("indexed evidence")
            .records()
            .len(),
        expected_evidence
    );
    let target = CausalTarget::new(
        StateObjectKindId::new(7),
        frames.checked_add(99).expect("target id"),
        StatePropertyId::new(9),
    );
    assert_eq!(
        reader
            .latest_matching_event_before(target, TraceId::new(frames))
            .expect("indexed latest-before")
            .expect("latest event")
            .trace_id(),
        TraceId::new(frames - 1)
    );
}

#[test]
fn history_store_seals_content_addressed_segment_without_mutation() {
    // Given
    let temp = TempBundle::new("seal");
    let mut store =
        HistoryStore::create(temp.path(), [0x47; 32], DurabilityMode::EveryTick).expect("store");
    store.append_tick(vec![causal(1, vec![])]).expect("tick");

    // When
    let sealed = store.seal_active().expect("seal active segment");
    let sealed_before = fs::read(sealed.path()).expect("sealed bytes");
    store
        .append_tick(vec![causal(2, vec![TraceId::new(1)])])
        .expect("new active tick");
    let sealed_after = fs::read(sealed.path()).expect("sealed bytes after append");
    drop(store);
    let reopened = HistoryStore::open(temp.path()).expect("reopen sealed bundle");

    // Then
    assert_eq!(sealed_before, sealed_after);
    assert!(
        sealed
            .path()
            .file_name()
            .expect("name")
            .to_string_lossy()
            .contains(&sealed.identity_hex())
    );
    assert_eq!(reopened.durable_generation(), 2);
    assert_eq!(
        reopened
            .durable_reader()
            .expect("reader")
            .records(query())
            .expect("records")
            .records()
            .len(),
        2
    );
    println!(
        "seal identity={} bytes={} reopened_generation=2 immutable=true",
        sealed.identity_hex(),
        sealed_before.len()
    );
}

#[test]
fn history_store_ignores_orphan_active_tail_on_reopen() {
    // Given
    let temp = TempBundle::new("orphan-tail");
    let mut store =
        HistoryStore::create(temp.path(), [0x48; 32], DurabilityMode::EveryTick).expect("store");
    let receipt = store.append_tick(vec![causal(1, vec![])]).expect("tick");
    let active = store.active_segment_path().to_path_buf();
    drop(store);
    OpenOptions::new()
        .append(true)
        .open(&active)
        .expect("open active")
        .write_all(b"orphan-tail")
        .expect("append orphan");

    // When
    let reopened = HistoryStore::open(temp.path()).expect("reopen ignores orphan tail");

    // Then
    assert_eq!(
        fs::metadata(&active).expect("active metadata").len(),
        receipt.range().end + 11
    );
    assert_eq!(
        reopened
            .durable_reader()
            .expect("reader")
            .records(query())
            .expect("records")
            .records()
            .len(),
        1
    );
    println!(
        "orphan_tail ignored_bytes=11 retained_unreachable=true committed_end={} reopened_records=1",
        receipt.range().end
    );
}

#[test]
fn history_store_rebuilds_deleted_indexes_from_canonical_frames() {
    // Given
    let temp = TempBundle::new("rebuild");
    let mut store =
        HistoryStore::create(temp.path(), [0x49; 32], DurabilityMode::EveryTick).expect("store");
    store
        .append_tick(vec![causal(1, vec![]), evidence(1, 1, b"rain/a")])
        .expect("first");
    store
        .append_tick(vec![causal(2, vec![TraceId::new(1)])])
        .expect("second");
    let before = store.catalog().counts().expect("counts before");
    drop(store);
    let mut connection =
        rusqlite::Connection::open(temp.path().join("catalog.sqlite")).expect("catalog");
    let transaction = connection
        .transaction()
        .expect("index deletion transaction");
    transaction
        .execute_batch(
            "DELETE FROM cause_edges; DELETE FROM effect_edges; DELETE FROM evidence; DELETE FROM events;",
        )
        .expect("delete derived indexes");
    transaction.commit().expect("commit index deletion");
    drop(connection);

    // When
    let store = HistoryStore::open(temp.path()).expect("open rebuilds indexes");
    let after = store.catalog().counts().expect("counts after");

    // Then
    assert_eq!(before, after);
    assert_eq!(after.events, 2);
    assert_eq!(after.cause_edges, 1);
    assert_eq!(after.effect_edges, 2);
    assert_eq!(after.evidence, 1);
    println!(
        "index_rebuild deleted_rows=true reopen_rebuilt=true events=2 causes=1 effects=2 evidence=1 deterministic=true"
    );
}

#[test]
fn decoder_registry_survives_create_open_reopen_and_index_rebuild() {
    // Given: one caller-owned registry for a synthetic replay-required schema.
    let temp = TempBundle::new("decoder-registry");
    let mut registry = EvidenceDecoderRegistry::new();
    registry
        .register_typed(SyntheticReplayDecoder)
        .expect("unique synthetic decoder");
    let registry = Arc::new(registry);
    let mut store = HistoryStore::create_with_registry(
        temp.path(),
        [0x72; 32],
        DurabilityMode::EveryTick,
        Arc::clone(&registry),
    )
    .expect("create with caller-owned decoder registry");

    // When: the required record is appended and every decode path is exercised.
    let records = vec![causal(1, vec![]), replay_evidence(1, 1)];
    store
        .append_tick(records.clone())
        .expect("registered append");
    assert_eq!(
        store
            .durable_reader()
            .expect("registered durable reader")
            .records(query())
            .expect("registered read")
            .records(),
        records
    );
    store
        .rebuild_indexes()
        .expect("registered explicit rebuild");
    drop(store);

    let default_error = HistoryStore::open(temp.path())
        .expect_err("default empty registry must reject required evidence");
    assert!(
        matches!(
            default_error,
            PersistenceError::History(HistoryError::UnsupportedReplaySchema { .. })
        ),
        "unexpected default-registry open error: {default_error:?}"
    );

    let connection = rusqlite::Connection::open(temp.path().join("catalog.sqlite"))
        .expect("catalog for recovery fixture");
    connection
        .execute_batch(
            "DELETE FROM cause_edges; DELETE FROM effect_edges; DELETE FROM evidence; DELETE FROM events;",
        )
        .expect("delete derived indexes");
    drop(connection);

    let reopened = HistoryStore::open_with_registry(temp.path(), Arc::clone(&registry))
        .expect("registered open rebuilds indexes");
    assert_eq!(
        reopened
            .catalog()
            .counts()
            .expect("rebuilt counts")
            .evidence,
        1
    );
    assert_eq!(
        reopened
            .durable_reader()
            .expect("reopened registered reader")
            .records(query())
            .expect("reopened registered read")
            .records(),
        records
    );
    drop(reopened);

    // Then: the same caller-owned registry can be reused for a second reopen.
    let reopened_again =
        HistoryStore::open_with_registry(temp.path(), registry).expect("registered second reopen");
    assert_eq!(
        reopened_again
            .durable_reader()
            .expect("second reopened reader")
            .records(query())
            .expect("second reopened read")
            .records(),
        records
    );
}

#[test]
fn history_store_rejects_deleted_catalog_rows_before_index_backed_readback() {
    // Given
    let temp = TempBundle::new("deleted-index-readback");
    let mut store =
        HistoryStore::create(temp.path(), [0x54; 32], DurabilityMode::EveryTick).expect("store");
    store
        .append_tick(vec![causal(1, vec![])])
        .expect("first frame");
    store
        .append_tick(vec![causal(2, vec![TraceId::new(1)])])
        .expect("second frame");
    let connection =
        rusqlite::Connection::open(temp.path().join("catalog.sqlite")).expect("catalog");

    // When
    connection
        .execute(
            "DELETE FROM cause_edges WHERE child_trace=?1",
            [2_u64.to_be_bytes()],
        )
        .expect("delete required parent index row");
    drop(connection);
    let error = store
        .durable_reader()
        .expect_err("deleted catalog row must prevent indexed readback");

    // Then
    assert!(matches!(
        error,
        PersistenceError::HistoryCatalogInvariant {
            invariant: "derived catalog index rows do not match canonical frames"
        }
    ));
    println!("index_backed_query deleted_row=cause_edges result=typed_catalog_invariant");
}

#[test]
fn history_store_rejects_corrupt_catalog_rows_before_index_backed_readback() {
    // Given
    let temp = TempBundle::new("corrupt-index-readback");
    let mut store =
        HistoryStore::create(temp.path(), [0x55; 32], DurabilityMode::EveryTick).expect("store");
    store
        .append_tick(vec![causal(1, vec![]), evidence(1, 1, b"rain/a")])
        .expect("frame");
    let connection =
        rusqlite::Connection::open(temp.path().join("catalog.sqlite")).expect("catalog");

    // When
    connection
        .execute(
            "UPDATE events SET record_ordinal=1 WHERE trace_id=?1",
            [1_u64.to_be_bytes()],
        )
        .expect("corrupt event index location");
    drop(connection);
    let error = store
        .durable_reader()
        .expect_err("corrupt catalog row must prevent indexed readback");

    // Then
    assert!(matches!(
        error,
        PersistenceError::HistoryCatalogInvariant {
            invariant: "derived catalog index rows do not match canonical frames"
        }
    ));
    println!("index_backed_query corrupt_row=events.record_ordinal result=typed_catalog_invariant");
}

#[test]
fn history_store_rejects_corrupt_effect_and_evidence_rows_before_index_backed_readback() {
    // Given
    let effect_temp = TempBundle::new("corrupt-effect-index-readback");
    let mut effect_store =
        HistoryStore::create(effect_temp.path(), [0x56; 32], DurabilityMode::EveryTick)
            .expect("effect store");
    effect_store
        .append_tick(vec![causal(1, vec![])])
        .expect("effect frame");
    let effect_connection =
        rusqlite::Connection::open(effect_temp.path().join("catalog.sqlite")).expect("catalog");
    effect_connection
        .execute(
            "UPDATE effect_edges SET property_id=?1",
            [99_u64.to_be_bytes()],
        )
        .expect("corrupt latest-before index row");
    drop(effect_connection);

    // When
    let effect_error = effect_store
        .durable_reader()
        .expect_err("corrupt effect row must prevent indexed readback");

    // Then
    assert!(matches!(
        effect_error,
        PersistenceError::HistoryCatalogInvariant {
            invariant: "derived catalog index rows do not match canonical frames"
        }
    ));

    // Given
    let evidence_temp = TempBundle::new("corrupt-evidence-index-readback");
    let mut evidence_store =
        HistoryStore::create(evidence_temp.path(), [0x57; 32], DurabilityMode::EveryTick)
            .expect("evidence store");
    evidence_store
        .append_tick(vec![causal(1, vec![]), evidence(1, 1, b"rain/a")])
        .expect("evidence frame");
    let evidence_connection =
        rusqlite::Connection::open(evidence_temp.path().join("catalog.sqlite")).expect("catalog");
    evidence_connection
        .execute(
            "UPDATE evidence SET canonical_key=?1",
            [b"corrupt".as_slice()],
        )
        .expect("corrupt evidence index row");
    drop(evidence_connection);

    // When
    let evidence_error = evidence_store
        .durable_reader()
        .expect_err("corrupt evidence row must prevent indexed readback");

    // Then
    assert!(matches!(
        evidence_error,
        PersistenceError::HistoryCatalogInvariant {
            invariant: "derived catalog index rows do not match canonical frames"
        }
    ));
    println!(
        "index_backed_query corrupt_rows=[effect_edges.property_id,evidence.canonical_key] result=typed_catalog_invariant"
    );
}

#[test]
fn history_store_rebuilds_same_count_corrupt_indexes_before_readback() {
    // Given
    let temp = TempBundle::new("corrupt-index-rebuild");
    let mut store =
        HistoryStore::create(temp.path(), [0x58; 32], DurabilityMode::EveryTick).expect("store");
    store
        .append_tick(vec![causal(1, vec![]), evidence(1, 1, b"rain/a")])
        .expect("frame");
    drop(store);
    let connection =
        rusqlite::Connection::open(temp.path().join("catalog.sqlite")).expect("catalog");
    connection
        .execute(
            "UPDATE events SET record_ordinal=1 WHERE trace_id=?1",
            [1_u64.to_be_bytes()],
        )
        .expect("corrupt event location without changing row counts");
    drop(connection);

    // When
    let store = HistoryStore::open(temp.path()).expect("open rebuilds corrupt indexes");
    let reader = store.durable_reader().expect("catalog-backed reader");

    // Then
    assert_eq!(
        reader
            .event(TraceId::new(1))
            .expect("event query")
            .expect("event")
            .event_id(),
        EventId::new(1)
    );
    assert_eq!(
        reader
            .evidence(EvidenceSchemaId::new(0xABCD), b"rain/a", query())
            .expect("evidence query")
            .records()
            .len(),
        1
    );
    println!(
        "index_rebuild corrupt_same_count=true reopen_rebuilt=true indexed_readback=true events=1 evidence=1"
    );
}

#[test]
fn history_store_rejects_truncated_referenced_frame_on_reopen() {
    // Given
    let temp = TempBundle::new("truncated");
    let mut store =
        HistoryStore::create(temp.path(), [0x52; 32], DurabilityMode::EveryTick).expect("store");
    store.append_tick(vec![causal(1, vec![])]).expect("tick");
    let active = store.active_segment_path().to_path_buf();
    drop(store);
    let length = fs::metadata(&active).expect("frame metadata").len();
    OpenOptions::new()
        .write(true)
        .open(&active)
        .expect("open frame")
        .set_len(length - 1)
        .expect("truncate referenced frame");

    // When
    let error = HistoryStore::open(temp.path()).expect_err("truncation must fail closed");

    // Then
    assert!(matches!(
        error,
        PersistenceError::HistoryIo {
            operation: "read frame range",
            kind: std::io::ErrorKind::UnexpectedEof,
            ..
        }
    ));
    println!("corruption boundary=truncated_referenced_frame result=typed_unexpected_eof");
}

#[test]
fn history_store_rejects_corrupted_referenced_frame_on_reopen() {
    // Given
    let temp = TempBundle::new("corrupt");
    let mut store =
        HistoryStore::create(temp.path(), [0x4A; 32], DurabilityMode::EveryTick).expect("store");
    store.append_tick(vec![causal(1, vec![])]).expect("tick");
    let active = store.active_segment_path().to_path_buf();
    drop(store);
    let mut bytes = fs::read(&active).expect("frame bytes");
    let last = bytes.len() - 1;
    bytes[last] ^= 0x01;
    fs::write(&active, bytes).expect("corrupt referenced frame");

    // When
    let error = HistoryStore::open(temp.path()).expect_err("corruption must fail closed");

    // Then
    assert!(matches!(error, PersistenceError::FrameDigestMismatch));
    println!("corruption boundary=referenced_frame result=typed_frame_digest_mismatch");
}

#[test]
fn history_store_rejects_mutated_sealed_segment_identity_on_reopen() {
    // Given
    let temp = TempBundle::new("mutated-sealed");
    let mut store =
        HistoryStore::create(temp.path(), [0x51; 32], DurabilityMode::EveryTick).expect("store");
    store.append_tick(vec![causal(1, vec![])]).expect("tick");
    let sealed = store.seal_active().expect("sealed segment");
    let sealed_path = sealed.path().to_path_buf();
    drop(store);
    OpenOptions::new()
        .append(true)
        .open(&sealed_path)
        .expect("open sealed segment")
        .write_all(b"mutation")
        .expect("mutate sealed segment fixture");

    // When
    let error = HistoryStore::open(temp.path()).expect_err("sealed mutation must fail closed");

    // Then
    assert!(matches!(error, PersistenceError::SegmentDigestMismatch));
    println!("corruption boundary=sealed_segment_mutation result=typed_segment_digest_mismatch");
}

#[test]
fn history_store_rejects_malformed_catalog_frame_reference() {
    // Given
    let temp = TempBundle::new("malformed-reference");
    let mut store =
        HistoryStore::create(temp.path(), [0x50; 32], DurabilityMode::EveryTick).expect("store");
    store.append_tick(vec![causal(1, vec![])]).expect("tick");
    drop(store);
    let connection =
        rusqlite::Connection::open(temp.path().join("catalog.sqlite")).expect("catalog");
    connection
        .execute("UPDATE frames SET end_offset=end_offset+1", [])
        .expect("malform range");
    drop(connection);

    // When
    let error = HistoryStore::open(temp.path()).expect_err("malformed range must fail closed");

    // Then
    assert!(matches!(
        error,
        PersistenceError::HistoryIo {
            operation: "read frame range",
            ..
        }
    ));
    println!("corruption boundary=catalog_range result=typed_history_io_error");
}

#[test]
fn history_store_rejects_injected_publication_failures_without_partial_generation() {
    for point in [
        FailurePoint::BeforeFrameFlush,
        FailurePoint::AfterFrameFlushBeforeSync,
        FailurePoint::AfterSegmentSyncBeforeCatalogCommit,
        FailurePoint::DuringCatalogCommit,
    ] {
        // Given
        let temp = TempBundle::new(point.name());
        let hooks = Arc::new(TestHooks::default());
        let mut store = HistoryStore::create_with_hooks(
            temp.path(),
            [0x4B; 32],
            DurabilityMode::EveryTick,
            hooks.clone(),
        )
        .expect("store");
        hooks.fail_once(point);

        // When
        let error = store
            .append_tick(vec![causal(1, vec![])])
            .expect_err("injected failure");

        // Then
        assert!(
            matches!(error, PersistenceError::InjectedHistoryFailure { point: actual } if actual == point)
        );
        assert_eq!(store.durable_generation(), 0);
        assert_eq!(
            store
                .durable_reader()
                .expect("reader")
                .records(query())
                .expect("records")
                .records()
                .len(),
            0
        );
        drop(store);
        let mut reopened = HistoryStore::open_with_hooks(temp.path(), hooks)
            .expect("restart loads previous complete generation");
        assert_eq!(reopened.durable_generation(), 0);
        assert_eq!(
            reopened
                .durable_reader()
                .expect("reopened reader")
                .records(query())
                .expect("reopened records")
                .records()
                .len(),
            0
        );
        reopened
            .append_tick(vec![causal(1, vec![])])
            .expect("restart publishes complete next generation");
        assert_eq!(reopened.durable_generation(), 1);
        assert_eq!(
            reopened
                .durable_reader()
                .expect("reader")
                .records(query())
                .expect("records")
                .records()
                .len(),
            1
        );
        println!(
            "failure_boundary={} typed=InjectedHistoryFailure restart_previous=0 retry_next=1 reachable_partial=false overlay_loss=1",
            point.name()
        );
    }
}

#[test]
fn history_store_failed_durable_append_is_atomic_in_process() {
    for point in [
        FailurePoint::BeforeFrameFlush,
        FailurePoint::AfterFrameFlushBeforeSync,
        FailurePoint::AfterSegmentSyncBeforeCatalogCommit,
        FailurePoint::DuringCatalogCommit,
    ] {
        let temp = TempBundle::new(&format!("atomic-{}", point.name()));
        let hooks = Arc::new(TestHooks::default());
        let mut store = HistoryStore::create_with_hooks(
            temp.path(),
            [0x4D; 32],
            DurabilityMode::EveryTick,
            hooks.clone(),
        )
        .expect("store");
        let before = store.status().expect("status before failure");

        for _ in 0..2 {
            hooks.fail_once(point);
            let error = store
                .append_tick(vec![causal(1, vec![])])
                .expect_err("injected failure");
            assert!(matches!(
                error,
                PersistenceError::InjectedHistoryFailure { point: actual } if actual == point
            ));
            assert_eq!(
                authoritative_status(store.status().expect("status after failure")),
                authoritative_status(before)
            );
            assert_eq!(
                fs::metadata(store.active_segment_path())
                    .expect("active segment metadata")
                    .len(),
                before.active_extent
            );
        }

        store
            .append_tick(vec![causal(1, vec![])])
            .expect("retry succeeds");
        let after = store.status().expect("status after retry");
        assert_eq!(after.logical_generation, before.logical_generation + 1);
        assert_eq!(after.durable_generation, after.logical_generation);
        assert_eq!(after.pending_frames, 0);
        assert_eq!(after.logical_records, before.logical_records + 1);
        assert_eq!(after.durable_records, after.logical_records);
        assert_eq!(
            after.catalog_counts.frames,
            before.catalog_counts.frames + 1
        );
        drop(store);

        let reopened = HistoryStore::open(temp.path()).expect("reopen retry result");
        assert_eq!(
            authoritative_status(reopened.status().expect("reopened status")),
            authoritative_status(after)
        );
    }
}

#[test]
fn history_store_rejects_seal_and_index_failures_transactionally() {
    // Given
    let temp = TempBundle::new("seal-index-failures");
    let hooks = Arc::new(TestHooks::default());
    let mut store = HistoryStore::create_with_hooks(
        temp.path(),
        [0x4C; 32],
        DurabilityMode::EveryTick,
        hooks.clone(),
    )
    .expect("store");
    store.append_tick(vec![causal(1, vec![])]).expect("tick");
    let active = store.active_segment_path().to_path_buf();
    hooks.fail_once(FailurePoint::SealRename);

    // When
    let seal_error = store.seal_active().expect_err("seal failure");
    hooks.fail_once(FailurePoint::IndexDeletion);
    let rebuild_error = store.rebuild_indexes().expect_err("index failure");

    // Then
    assert!(matches!(
        seal_error,
        PersistenceError::InjectedHistoryFailure {
            point: FailurePoint::SealRename
        }
    ));
    assert!(active.exists());
    assert!(matches!(
        rebuild_error,
        PersistenceError::InjectedHistoryFailure {
            point: FailurePoint::IndexDeletion
        }
    ));
    assert_eq!(store.catalog().counts().expect("indexes remain").events, 1);
    assert_eq!(
        store
            .durable_reader()
            .expect("reader")
            .records(query())
            .expect("records")
            .records()
            .len(),
        1
    );
    println!("failure_boundary=seal_rename durable=1 active_restored=true");
    println!("failure_boundary=index_deletion durable=1 indexes_rolled_back=true");
}

fn update_active_catalog_path(root: &Path, relative_path: &str) {
    let connection =
        rusqlite::Connection::open(root.join("catalog.sqlite")).expect("catalog connection");
    connection
        .execute(
            "UPDATE segments SET relative_path=?1 WHERE sealed=0",
            [relative_path],
        )
        .expect("update active catalog path");
}

#[test]
fn history_store_rejects_unsafe_catalog_segment_paths_before_open_or_create() {
    // Given: catalog-selected paths that are absolute, traversing, malformed, or missing.
    let foreign = TempBundle::new("path-foreign-target");
    let foreign_store = HistoryStore::create(foreign.path(), [0x61; 32], DurabilityMode::EveryTick)
        .expect("foreign store");
    let foreign_path = foreign_store.active_segment_path();
    drop(foreign_store);
    let foreign_bytes = fs::read(&foreign_path).expect("foreign active bytes");
    let cases = [
        (
            "absolute",
            foreign_path.to_string_lossy().into_owned(),
            false,
        ),
        ("parent", "../causafera-foreign.cfs".to_owned(), false),
        (
            "nested_parent",
            "segments/../causafera-foreign.cfs".to_owned(),
            false,
        ),
        ("empty_name", "segments/".to_owned(), false),
        ("invalid_name", "segments/active".to_owned(), false),
        (
            "missing_active",
            "segments/missing-active.cfs".to_owned(),
            true,
        ),
    ];

    for (label, catalog_path, missing) in cases {
        let temp = TempBundle::new(&format!("path-{label}"));
        let store = HistoryStore::create(temp.path(), [0x62; 32], DurabilityMode::EveryTick)
            .expect("store");
        drop(store);
        update_active_catalog_path(temp.path(), &catalog_path);

        // When: reopen resolves the catalog-selected active object.
        let error =
            HistoryStore::open(temp.path()).expect_err("unsafe active path must fail closed");

        // Then: no catalog-selected object is created or exposed.
        assert!(matches!(
            error,
            PersistenceError::HistoryCatalogPathUnsafe { .. }
                | PersistenceError::HistoryCatalogPathMissing { .. }
        ));
        if missing {
            assert!(!temp.path().join("segments/missing-active.cfs").exists());
        }
        assert_eq!(
            fs::read(&foreign_path).expect("foreign bytes after rejection"),
            foreign_bytes
        );
        println!(
            "malformed_path case={label} typed_error=true missing_created={} outside_unchanged=true",
            temp.path().join("segments/missing-active.cfs").exists()
        );
    }

    #[cfg(unix)]
    {
        let temp = TempBundle::new("path-symlink");
        let store = HistoryStore::create(temp.path(), [0x63; 32], DurabilityMode::EveryTick)
            .expect("store");
        drop(store);
        let symlink_path = temp.path().join("segments/escape.cfs");
        std::os::unix::fs::symlink(&foreign_path, &symlink_path).expect("symlink fixture");
        update_active_catalog_path(temp.path(), "segments/escape.cfs");

        let error = HistoryStore::open(temp.path()).expect_err("symlink escape must fail closed");

        assert!(matches!(
            error,
            PersistenceError::HistoryCatalogPathUnsafe { .. }
        ));
        assert_eq!(
            fs::read(&foreign_path).expect("foreign bytes after symlink rejection"),
            foreign_bytes
        );
        println!("malformed_path case=symlink typed_error=true outside_unchanged=true");
    }

    let temp = TempBundle::new("path-missing-valid-active");
    let store =
        HistoryStore::create(temp.path(), [0x69; 32], DurabilityMode::EveryTick).expect("store");
    let active_path = store.active_segment_path().to_path_buf();
    drop(store);
    fs::remove_file(&active_path).expect("remove active fixture");
    let error = HistoryStore::open(temp.path()).expect_err("missing active must fail closed");
    assert!(matches!(
        error,
        PersistenceError::HistoryCatalogPathMissing { .. }
    ));
    assert!(!active_path.exists());
    println!(
        "malformed_path case=missing_valid_active typed_error=HistoryCatalogPathMissing missing_created=false"
    );
}

#[test]
fn history_store_rejects_huge_and_impossible_catalog_ranges_before_allocation() {
    // Given: a real frame and catalog ranges that are too large or arithmetically impossible.
    let huge = TempBundle::new("range-huge");
    let mut store =
        HistoryStore::create(huge.path(), [0x64; 32], DurabilityMode::EveryTick).expect("store");
    store.append_tick(vec![causal(1, vec![])]).expect("frame");
    drop(store);
    let huge_end = u64::try_from(HISTORY_FRAME_HEADER_LEN + MAX_FRAME_DECODED_BYTES)
        .expect("range bound")
        .checked_add(1)
        .expect("range bound increment");
    let connection =
        rusqlite::Connection::open(huge.path().join("catalog.sqlite")).expect("catalog");
    connection
        .execute(
            "UPDATE frames SET start_offset=0,end_offset=?1",
            [i64::try_from(huge_end).expect("SQLite range")],
        )
        .expect("huge range");
    drop(connection);

    // When: reopen validates the range before allocating the requested bytes.
    let error = HistoryStore::open(huge.path()).expect_err("huge range must fail closed");

    // Then: the typed frame bound wins without a large read/allocation.
    assert!(matches!(
        error,
        PersistenceError::HistoryFrameSizeExceeded { .. }
    ));
    println!("malformed_range case=huge typed_error=HistoryFrameSizeExceeded allocation=bounded");

    let impossible = TempBundle::new("range-impossible");
    let mut store = HistoryStore::create(impossible.path(), [0x65; 32], DurabilityMode::EveryTick)
        .expect("store");
    store.append_tick(vec![causal(1, vec![])]).expect("frame");
    drop(store);
    let connection =
        rusqlite::Connection::open(impossible.path().join("catalog.sqlite")).expect("catalog");
    connection
        .execute_batch("PRAGMA ignore_check_constraints=ON;")
        .expect("disable checks for malformed fixture");
    connection
        .execute("UPDATE frames SET start_offset=10,end_offset=9", [])
        .expect("impossible range");
    drop(connection);

    let error =
        HistoryStore::open(impossible.path()).expect_err("impossible range must fail closed");
    assert!(matches!(error, PersistenceError::HistoryArithmeticOverflow));
    println!(
        "malformed_range case=impossible typed_error=HistoryArithmeticOverflow unchanged_catalog=true"
    );
}

#[test]
fn history_store_post_commit_append_and_seal_failures_are_reversible() {
    // Given: real catalog publication seams after the SQLite transaction has committed.
    for point in [
        FailurePoint::AfterCatalogCommitBeforeRootSync,
        FailurePoint::AfterAppendCatalogCommitAfterRootSync,
    ] {
        let temp = TempBundle::new(&format!("post-commit-{}", point.name()));
        let hooks = Arc::new(TestHooks::default());
        let mut store = HistoryStore::create_with_hooks(
            temp.path(),
            [0x66; 32],
            DurabilityMode::EveryTick,
            hooks.clone(),
        )
        .expect("store");
        let before = store.status().expect("status before");
        let active = store.active_segment_path();

        for attempt in 1..=2 {
            hooks.fail_once(point);
            let directory_syncs_before = hooks.directory_syncs.load(Ordering::SeqCst);
            let error = store
                .append_tick(vec![causal(1, vec![])])
                .expect_err("post-commit append failure");
            assert!(matches!(
                error,
                PersistenceError::InjectedHistoryFailure { point: actual } if actual == point
            ));
            assert_eq!(
                authoritative_status(store.status().expect("status after failure")),
                authoritative_status(before)
            );
            assert_eq!(
                fs::metadata(&active).expect("active metadata").len(),
                before.active_extent
            );
            let directory_syncs_after = hooks.directory_syncs.load(Ordering::SeqCst);
            let after_root_sync = point == FailurePoint::AfterAppendCatalogCommitAfterRootSync;
            if after_root_sync {
                assert_eq!(directory_syncs_after - directory_syncs_before, 2);
            }
            println!(
                "post_backend_failure point={} attempt={} after_root_sync={} directory_syncs_delta={} unchanged=true catalog_rows_unchanged=true no_orphan_frame=true",
                point.name(),
                attempt,
                after_root_sync,
                directory_syncs_after - directory_syncs_before
            );
        }

        store
            .append_tick(vec![causal(1, vec![])])
            .expect("same-process retry");
        let after = store.status().expect("status after retry");
        assert_eq!(after.logical_generation, 1);
        assert_eq!(after.durable_generation, 1);
        drop(store);
        let reopened = HistoryStore::open(temp.path()).expect("reopen after retry");
        assert_eq!(
            authoritative_status(reopened.status().expect("reopened status")),
            authoritative_status(after)
        );
        println!(
            "post_backend_retry point={} same_process=true reopen=true generation=1",
            point.name()
        );
    }

    for point in [
        FailurePoint::AfterSealRenameBeforeNewActive,
        FailurePoint::AfterSealNewActiveBeforeCatalogCommit,
        FailurePoint::AfterSealCatalogCommitBeforeRootSync,
        FailurePoint::AfterSealCatalogCommitAfterRootSync,
    ] {
        let temp = TempBundle::new(&format!("post-seal-{}", point.name()));
        let hooks = Arc::new(TestHooks::default());
        let mut store = HistoryStore::create_with_hooks(
            temp.path(),
            [0x67; 32],
            DurabilityMode::EveryTick,
            hooks.clone(),
        )
        .expect("store");
        store.append_tick(vec![causal(1, vec![])]).expect("frame");
        let before = store.status().expect("status before seal");
        let active = store.active_segment_path();
        let segment_names = || {
            let mut names = fs::read_dir(temp.path().join("segments"))
                .expect("segments")
                .map(|entry| {
                    entry
                        .expect("segment entry")
                        .file_name()
                        .to_string_lossy()
                        .into_owned()
                })
                .collect::<Vec<_>>();
            names.sort();
            names
        };
        let before_segments = segment_names();
        for attempt in 1..=2 {
            hooks.fail_once(point);
            let directory_syncs_before = hooks.directory_syncs.load(Ordering::SeqCst);

            let error = store.seal_active().expect_err("post-seal failure");

            assert!(matches!(
                error,
                PersistenceError::InjectedHistoryFailure { point: actual } if actual == point
            ));
            assert_eq!(
                authoritative_status(store.status().expect("status after seal failure")),
                authoritative_status(before)
            );
            assert_eq!(store.active_segment_path(), active);
            assert_eq!(segment_names(), before_segments);
            let directory_syncs_after = hooks.directory_syncs.load(Ordering::SeqCst);
            let after_root_sync = point == FailurePoint::AfterSealCatalogCommitAfterRootSync;
            if after_root_sync {
                assert_eq!(directory_syncs_after - directory_syncs_before, 3);
            }
            println!(
                "post_backend_failure point={} attempt={} after_root_sync={} directory_syncs_delta={} unchanged=true catalog_rows_unchanged=true no_orphan_files=true",
                point.name(),
                attempt,
                after_root_sync,
                directory_syncs_after - directory_syncs_before
            );
        }
        let retry = store.seal_active().expect("same-process seal retry");
        let after_retry = store.status().expect("status after seal retry");
        assert_ne!(retry.path(), active);
        drop(store);
        let reopened = HistoryStore::open(temp.path()).expect("reopen after seal failure");
        assert_eq!(
            authoritative_status(reopened.status().expect("reopened seal status")),
            authoritative_status(after_retry)
        );
        println!(
            "post_backend_retry point={} same_process=true reopen=true unchanged_before_retry=true",
            point.name()
        );
    }
}

#[test]
fn history_store_incremental_evidence_digest_matches_cold_oracle_without_rescan() {
    // Given: a large evidence history built through the ordinary append path.
    let temp = TempBundle::new("incremental-evidence");
    let mut store =
        HistoryStore::create(temp.path(), [0x68; 32], DurabilityMode::EveryTick).expect("store");
    let mut records = Vec::new();
    reset_evidence_manifest_full_rescan_counter();
    for sequence in 1..=1024 {
        let record = evidence(sequence, sequence, b"incremental/evidence");
        let before = evidence_manifest_full_rescan_count();
        records.push(record.clone());

        store.append_tick(vec![record]).expect("evidence tick");

        assert_eq!(
            evidence_manifest_full_rescan_count(),
            before,
            "ordinary evidence append invoked the cold oracle at tick {sequence}"
        );
        if sequence == 4 || sequence == 1024 {
            let oracle = evidence_manifest_digest_full_rescan(&records).expect("cold oracle");
            assert_eq!(
                store.status().expect("status").evidence_manifest_digest,
                oracle
            );
            println!(
                "evidence_digest tick={} incremental_equals_oracle=true whole_history_scans=0",
                sequence
            );
        }
    }
}

#[test]
fn history_store_test_bundle_attempts_are_unique_and_raii_cleaned() {
    // Given: two same-name test attempts in one process.
    let first = TempBundle::new("unique-attempt");
    let second = TempBundle::new("unique-attempt");

    // Then: each attempt owns a distinct directory and Drop removes it.
    assert_ne!(first.path(), second.path());
    let first_path = first.path().to_path_buf();
    let second_path = second.path().to_path_buf();
    assert!(first_path.is_dir());
    assert!(second_path.is_dir());
    drop(first);
    assert!(!first_path.exists());
    drop(second);
    assert!(!second_path.exists());

    let (replacement, stale_path) = TempBundle::new_with_stale_candidate("stale-pid");
    let replacement_path = replacement.path().to_path_buf();
    assert_ne!(replacement.path(), stale_path.as_path());
    assert!(stale_path.is_dir());
    drop(replacement);
    assert!(!replacement_path.exists());
    fs::remove_dir_all(&stale_path).expect("remove stale collision fixture");
    assert!(!stale_path.exists());
    println!(
        "temp_bundle_collision attempts=2 unique=true stale_pid_reused=true raii_cleanup=true"
    );
}

#[test]
fn history_store_logical_digest_full_rescan_agrees_with_the_incremental_fold() {
    // Given
    let first = TempBundle::new("rescan-per-tick");
    let second = TempBundle::new("rescan-repartitioned");
    let mut per_tick = HistoryStore::create(first.path(), [0x61; 32], DurabilityMode::GroupCommit)
        .expect("per tick store");
    let mut repartitioned =
        HistoryStore::create(second.path(), [0x61; 32], DurabilityMode::GroupCommit)
            .expect("repartitioned store");

    // When: one store publishes a frame per tick, the other splits the same
    // record stream into four frames.
    per_tick
        .append_tick(vec![causal(1, vec![]), evidence(1, 1, b"k1")])
        .expect("first tick");
    per_tick
        .append_tick(vec![
            causal(2, vec![TraceId::new(1)]),
            evidence(2, 2, b"k2"),
        ])
        .expect("second tick");
    repartitioned
        .append_tick(vec![causal(1, vec![])])
        .expect("first causal frame");
    repartitioned
        .append_tick(vec![evidence(1, 1, b"k1")])
        .expect("first evidence frame");
    repartitioned
        .append_tick(vec![causal(2, vec![TraceId::new(1)])])
        .expect("second causal frame");
    repartitioned
        .append_tick(vec![evidence(2, 2, b"k2")])
        .expect("second evidence frame");

    // Then: the cold rescan reproduces the incremental digest while the frames
    // are still only logical, after they are forced durable, and after reopen.
    let pending = per_tick.status().expect("pending status").logical_digest;
    assert_eq!(
        per_tick
            .logical_digest_full_rescan()
            .expect("pending rescan"),
        pending
    );
    per_tick.flush_durable().expect("force durable");
    let durable = per_tick.status().expect("durable status").logical_digest;
    assert_eq!(
        per_tick
            .logical_digest_full_rescan()
            .expect("durable rescan"),
        durable
    );
    assert_eq!(
        durable, pending,
        "forcing durability does not change the fold"
    );
    assert_eq!(
        repartitioned
            .logical_digest_full_rescan()
            .expect("repartitioned rescan"),
        durable,
        "repartitioning the same records across frames must not change the digest"
    );
    drop(per_tick);
    let reopened = HistoryStore::open(first.path()).expect("reopen");
    assert_eq!(
        reopened
            .logical_digest_full_rescan()
            .expect("reopened rescan"),
        durable
    );
    println!(
        "logical_digest_full_rescan frames=2/4 records=4 digest={}",
        hex_digest(&durable)
    );
}

#[test]
fn history_store_append_metrics_track_sync_policy_without_rescanning() {
    // Given: stores configured with each durability policy and three identical ticks.
    let every_tick_bundle = TempBundle::new("append-metrics-every-tick");
    let group_commit_bundle = TempBundle::new("append-metrics-group-commit");
    let mut every_tick = HistoryStore::create(
        every_tick_bundle.path(),
        [0x63; 32],
        DurabilityMode::EveryTick,
    )
    .expect("every-tick store");
    let mut group_commit = HistoryStore::create(
        group_commit_bundle.path(),
        [0x63; 32],
        DurabilityMode::GroupCommit,
    )
    .expect("group-commit store");

    // When: each store receives three ordinary append calls.
    for id in 1..=3 {
        every_tick
            .append_tick(vec![causal(id, vec![])])
            .expect("every-tick append");
        group_commit
            .append_tick(vec![causal(id, vec![])])
            .expect("group-commit append");
    }

    // Then: append never invokes the cold digest oracle, and durability policy
    // determines the count of segment syncs and catalog commits.
    let every_tick_status = every_tick.status().expect("every-tick status");
    let group_commit_status = group_commit.status().expect("group-commit status");
    assert_eq!(every_tick_status.logical_digest_full_rescan_count, 0);
    assert_eq!(group_commit_status.logical_digest_full_rescan_count, 0);
    assert_eq!(every_tick_status.catalog_full_scan_count, 0);
    assert_eq!(group_commit_status.catalog_full_scan_count, 0);
    assert_eq!(every_tick_status.durable_sync_count, 3);
    assert_eq!(every_tick_status.catalog_commit_count, 3);
    assert_eq!(group_commit_status.durable_sync_count, 0);
    assert_eq!(group_commit_status.catalog_commit_count, 0);
    assert!(!every_tick_status.append_elapsed_wall_time.is_zero());
    assert!(!group_commit_status.append_elapsed_wall_time.is_zero());

    group_commit.flush_durable().expect("flush group commit");
    let flushed_group_status = group_commit.status().expect("flushed group status");
    assert_eq!(flushed_group_status.durable_sync_count, 1);
    assert_eq!(flushed_group_status.catalog_commit_count, 1);
    assert_eq!(flushed_group_status.logical_digest_full_rescan_count, 0);
    assert_eq!(flushed_group_status.catalog_full_scan_count, 0);

    every_tick
        .logical_digest_full_rescan()
        .expect("explicit full rescan");
    assert_eq!(
        every_tick
            .status()
            .expect("status after explicit rescan")
            .logical_digest_full_rescan_count,
        1
    );
    assert_eq!(
        every_tick
            .status()
            .expect("status after explicit rescan")
            .catalog_full_scan_count,
        1
    );
    println!(
        "append_metrics every_tick_syncs={} group_commit_syncs={} explicit_rescans={}",
        every_tick_status.durable_sync_count,
        flushed_group_status.durable_sync_count,
        every_tick
            .status()
            .expect("final every-tick status")
            .logical_digest_full_rescan_count
    );
}

fn hex_digest(digest: &[u8; 32]) -> String {
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[test]
fn history_store_logical_digest_ignores_where_segments_are_sealed() {
    // Given: the same 240 large evidence records (about 8 MiB), once as one
    // frame per record and once as a single frame, so the stores seal a
    // different number of segments around identical content.
    let split = TempBundle::new("seal-per-record");
    let whole = TempBundle::new("seal-single-frame");
    let mut per_record =
        HistoryStore::create(split.path(), [0x62; 32], DurabilityMode::GroupCommit)
            .expect("per record store");
    let mut single = HistoryStore::create(whole.path(), [0x62; 32], DurabilityMode::GroupCommit)
        .expect("single frame store");
    let records = (1..=240).map(large_evidence).collect::<Vec<_>>();

    // When
    for record in &records {
        per_record
            .append_tick(vec![record.clone()])
            .expect("one record frame");
    }
    single.append_tick(records).expect("single frame");
    per_record.flush_durable().expect("force per record store");
    single.flush_durable().expect("force single store");

    // Then: the segment layouts differ and the logical digest does not.
    let per_record_status = per_record.status().expect("per record status");
    let single_status = single.status().expect("single status");
    assert_ne!(
        per_record_status.catalog_counts.segments, single_status.catalog_counts.segments,
        "the fixture must seal a different number of segments in each store"
    );
    assert_eq!(
        per_record_status.logical_digest,
        single_status.logical_digest
    );
    assert_eq!(
        per_record
            .logical_digest_full_rescan()
            .expect("per record rescan"),
        single.logical_digest_full_rescan().expect("single rescan")
    );
    println!(
        "segment_independent_digest segments={}/{} digest={}",
        per_record_status.catalog_counts.segments,
        single_status.catalog_counts.segments,
        hex_digest(&per_record_status.logical_digest)
    );
}

fn causal_variant(sequence: u64, causes: Vec<TraceId>, object_offset: u64) -> HistoryRecord {
    HistoryRecord::Causal(
        OwnedCausalRecord::new(
            sequence,
            EventId::new(sequence),
            TraceId::new(sequence),
            SimulationTime::new(sequence * 10),
            Phase::Physics,
            EventKindId::new(13),
            causes,
            vec![effect(object_offset + sequence, 9)],
        )
        .expect("valid causal fixture"),
    )
}

#[test]
fn history_store_open_reports_the_unpublished_group_tail() {
    // Given: a group-commit writer that flushed three ticks and then died before
    // its group reached the catalog.
    let temp = TempBundle::new("tail-group");
    let mut store = HistoryStore::create(temp.path(), [0x71; 32], DurabilityMode::GroupCommit)
        .expect("group store");
    store.append_tick(vec![causal(1, vec![])]).expect("tick 1");
    store
        .append_tick(vec![causal(2, vec![TraceId::new(1)])])
        .expect("tick 2");
    store
        .append_tick(vec![causal(3, vec![TraceId::new(2)])])
        .expect("tick 3");
    drop(store);

    // When: the bundle is reopened.
    let reopened = HistoryStore::open(temp.path()).expect("reopen after the crash");

    // Then: the committed generation is untouched, and the lost ticks are named
    // exactly rather than silently promoted or forgotten.
    assert_eq!(reopened.logical_generation(), 0);
    assert_eq!(reopened.durable_generation(), 0);
    let tail = reopened.recovered_tail();
    assert_eq!(
        (tail.frames, tail.records),
        (3, 3),
        "three flushed ticks were lost"
    );
    assert_eq!(
        (tail.first_generation, tail.last_generation),
        (Some(1), Some(3))
    );
    assert_eq!(tail.unaccounted_bytes, 0);
    println!(
        "unpublished_tail frames={} first_generation=1 last_generation=3 unaccounted_bytes=0",
        tail.frames
    );
}

#[test]
fn history_store_open_does_not_count_a_torn_final_frame() {
    // Given: the same crash, with the last frame cut short.
    let temp = TempBundle::new("tail-torn");
    let mut store = HistoryStore::create(temp.path(), [0x72; 32], DurabilityMode::GroupCommit)
        .expect("group store");
    store.append_tick(vec![causal(1, vec![])]).expect("tick 1");
    store
        .append_tick(vec![causal(2, vec![TraceId::new(1)])])
        .expect("tick 2");
    let path = store.active_segment_path();
    drop(store);
    let length = fs::metadata(&path).expect("segment metadata").len();
    OpenOptions::new()
        .write(true)
        .open(&path)
        .expect("open segment")
        .set_len(length - 5)
        .expect("tear the final frame");

    // When
    let tail = HistoryStore::open(temp.path())
        .expect("reopen after the torn write")
        .recovered_tail();

    // Then: only the complete frame counts, and the torn bytes are accounted for.
    assert_eq!((tail.frames, tail.last_generation), (1, Some(1)));
    assert!(tail.unaccounted_bytes > 0);
}

#[test]
fn history_store_open_ignores_frames_left_by_an_earlier_writer() {
    // Given: a crash that left three frames, then a second writer that wrote a
    // different but same-sized first frame over the start of that tail and also
    // died.
    let temp = TempBundle::new("tail-stale");
    let mut first = HistoryStore::create(temp.path(), [0x73; 32], DurabilityMode::GroupCommit)
        .expect("first writer");
    for sequence in 1..=3 {
        let causes = if sequence == 1 {
            vec![]
        } else {
            vec![TraceId::new(sequence - 1)]
        };
        first
            .append_tick(vec![causal(sequence, causes)])
            .expect("first writer tick");
    }
    drop(first);
    let mut second = HistoryStore::open(temp.path()).expect("second writer");
    second
        .append_tick(vec![causal_variant(1, vec![], 900)])
        .expect("second writer tick");
    drop(second);

    // When
    let tail = HistoryStore::open(temp.path())
        .expect("reopen")
        .recovered_tail();

    // Then: the second writer's one frame is the tail; the first writer's
    // leftover frames, whose digest chain does not continue from it, are not.
    assert_eq!(
        (tail.frames, tail.first_generation, tail.last_generation),
        (1, Some(1), Some(1))
    );
    assert!(
        tail.unaccounted_bytes > 0,
        "the leftovers are reported as unaccounted"
    );
}

#[test]
fn history_store_publishes_reads_and_verifies_snapshots() {
    // Given: a durable generation.
    let temp = TempBundle::new("snapshot-publish");
    let mut store =
        HistoryStore::create(temp.path(), [0x74; 32], DurabilityMode::EveryTick).expect("store");
    store.append_tick(vec![causal(1, vec![])]).expect("tick 1");
    let bytes = b"bounded snapshot bytes".to_vec();

    // When
    let receipt = store
        .publish_snapshot(1, &bytes)
        .expect("snapshot publishes");
    let again = store
        .publish_snapshot(1, &bytes)
        .expect("publishing the same bytes is idempotent");

    // Then: it is registered under its generation, reads back verified, and
    // survives a reopen.
    assert_eq!(receipt.generation(), 1);
    assert_eq!(receipt.digest(), again.digest());
    assert!(receipt.path().ends_with("snapshots/1.snap"));
    assert_eq!(store.read_snapshot(1).expect("read"), bytes);
    assert_eq!(store.catalog().counts().expect("counts").snapshots, 1);
    drop(store);
    let reopened = HistoryStore::open(temp.path()).expect("reopen");
    let latest = reopened
        .latest_snapshot()
        .expect("latest snapshot")
        .expect("one snapshot is registered");
    assert_eq!(latest.generation, 1);
    assert_eq!(&latest.digest, receipt.digest());
    assert_eq!(reopened.read_snapshot(1).expect("read after reopen"), bytes);
}

#[test]
fn history_store_refuses_snapshots_it_cannot_stand_behind() {
    // Given: a group store with one unpublished tick and one durable snapshot.
    let temp = TempBundle::new("snapshot-refuse");
    let mut store =
        HistoryStore::create(temp.path(), [0x75; 32], DurabilityMode::GroupCommit).expect("store");
    store.append_tick(vec![causal(1, vec![])]).expect("tick 1");

    // Then: a snapshot of a generation that is not durable, or does not exist, is refused.
    assert!(matches!(
        store.publish_snapshot(1, b"early"),
        Err(PersistenceError::HistoryGenerationUnavailable { generation: 1 })
    ));
    assert!(matches!(
        store.publish_snapshot(9, b"far"),
        Err(PersistenceError::HistoryGenerationUnavailable { generation: 9 })
    ));
    store.flush_durable().expect("force the group");
    store.publish_snapshot(1, b"first").expect("durable now");

    // And: different bytes for a published generation are refused, and an altered
    // file is detected on read.
    assert!(store.publish_snapshot(1, b"second").is_err());
    let path = temp.path().join("snapshots").join("1.snap");
    fs::write(&path, b"tampered").expect("tamper with the snapshot file");
    assert!(matches!(
        store.read_snapshot(1),
        Err(PersistenceError::SnapshotDigestMismatch)
    ));
}

#[test]
fn history_recovery_snapshot_faults_restore_catalog_and_retry() {
    let _process_guard = RECOVERY_PROCESS_LOCK.lock().expect("recovery process lock");
    for point in [
        FailurePoint::AfterSnapshotSyncBeforeCatalogCommit,
        FailurePoint::AfterSnapshotCatalogCommitBeforeRootSync,
        FailurePoint::AfterSnapshotCatalogCommitAfterRootSync,
    ] {
        // Given: a complete durable frame and an injected snapshot publication fault.
        let temp = TempBundle::new(point.name());
        let hooks = Arc::new(TestHooks::default());
        let mut store = HistoryStore::create_with_hooks(
            temp.path(),
            [0x76; 32],
            DurabilityMode::EveryTick,
            hooks.clone(),
        )
        .expect("store");
        store
            .append_tick(vec![causal(1, vec![])])
            .expect("durable frame");
        hooks.fail_once(point);

        // When: publication reaches the injected boundary and returns an error.
        let failure = store
            .publish_snapshot(1, b"bounded snapshot")
            .expect_err("injected snapshot failure");
        assert!(matches!(
            failure,
            PersistenceError::InjectedHistoryFailure { point: actual } if actual == point
        ));
        drop(store);

        // Then: restart sees the complete durable frame and no partial snapshot.
        let mut reopened = HistoryStore::open(temp.path()).expect("reopen durable head");
        assert_eq!(reopened.durable_generation(), 1);
        assert_eq!(reopened.catalog().counts().expect("counts").snapshots, 0);
        reopened
            .publish_snapshot(1, b"bounded snapshot")
            .expect("retry publishes the snapshot once");
        assert_eq!(reopened.catalog().counts().expect("counts").snapshots, 1);
    }
}
