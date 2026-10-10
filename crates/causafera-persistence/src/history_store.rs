//! Append-only causal-history bundle storage.
//!
//! Canonical CFHF frames are the authority. SQLite publishes only verified byte
//! ranges and deterministic derived indexes. Active segment files contain a
//! concatenation of complete CFHF frames; their synthesized immutable identity
//! header is `CFSG`, format 1, namespace, and segment sequence. Group publication
//! keeps written but unforced frames in a process-local overlay. Reopen seeks to
//! the last catalog-published offset without promoting or truncating an orphan
//! tail, so unreachable bytes remain outside durable history. The owning loop calls `flush_if_due` at its timer
//! boundary; no persistence-owned background thread participates in ordering.

use std::{
    collections::BTreeSet,
    fmt,
    fs::{self, File, OpenOptions, TryLockError},
    io::{Read, Seek, SeekFrom, Write},
    ops::Range,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Instant,
};

use causafera_core::{
    EvidenceDecoderRegistry, HistoryAnchor, HistoryAnchorFields, HistoryError, HistoryFrontier,
    HistoryRecord, OwnedCausalRecord, canonical_history_records,
};
use causafera_types::{EventId, SimulationTime, TraceId};
use rusqlite::{
    Connection, OptionalExtension, Transaction, TransactionBehavior, params, params_from_iter,
};

use crate::{
    CheckpointEnvelope, CheckpointMetadata, EvidenceManifestDigest, HISTORY_FRAME_HEADER_LEN,
    HistoryFrameHeaderFields, HistoryStoreReader, LogicalHistoryDigest, PersistenceError,
    decode_checkpoint_with_generations, decode_history_frame, encode_checkpoint,
    encode_history_frame, logical_history_digest, sealed_segment_identity,
};

const SEGMENT_TARGET_BYTES: u64 = 4 * 1024 * 1024;
const GROUP_MAX_TICKS: usize = 32;
const GROUP_MAX_MILLIS: u64 = 250;
const CATALOG_ID_QUERY_BATCH: usize = 900;
const SEGMENT_IDENTITY_MAGIC: &[u8] = b"CFSG";
const CATALOG_SCHEMA_VERSION: i64 = 1;
pub(crate) const MAX_STORED_FRAME_BYTES: u64 =
    (causafera_core::MAX_FRAME_DECODED_BYTES + HISTORY_FRAME_HEADER_LEN) as u64;

#[cfg(test)]
static CAUSAL_VALIDATION_CATALOG_LOOKUPS: AtomicUsize = AtomicUsize::new(0);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DurabilityMode {
    GroupCommit,
    EveryTick,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FailurePoint {
    BeforeFrameFlush,
    AfterFrameFlushBeforeSync,
    AfterSegmentSyncBeforeCatalogCommit,
    DuringCatalogCommit,
    AfterCatalogCommitBeforeRootSync,
    SealRename,
    AfterSealRenameBeforeNewActive,
    AfterSealNewActiveBeforeCatalogCommit,
    AfterSealCatalogCommitBeforeRootSync,
    IndexDeletion,
    AfterCheckpointSyncBeforeCatalogCommit,
    AfterCheckpointCatalogCommitBeforeRootSync,
    AfterAppendCatalogCommitAfterRootSync,
    AfterSealCatalogCommitAfterRootSync,
    AfterCheckpointCatalogCommitAfterRootSync,
}

impl FailurePoint {
    pub const fn code(self) -> usize {
        match self {
            Self::BeforeFrameFlush => 1,
            Self::AfterFrameFlushBeforeSync => 2,
            Self::AfterSegmentSyncBeforeCatalogCommit => 3,
            Self::DuringCatalogCommit => 4,
            Self::AfterCatalogCommitBeforeRootSync => 5,
            Self::SealRename => 6,
            Self::AfterSealRenameBeforeNewActive => 7,
            Self::AfterSealNewActiveBeforeCatalogCommit => 8,
            Self::AfterSealCatalogCommitBeforeRootSync => 9,
            Self::IndexDeletion => 10,
            Self::AfterCheckpointSyncBeforeCatalogCommit => 11,
            Self::AfterCheckpointCatalogCommitBeforeRootSync => 12,
            Self::AfterAppendCatalogCommitAfterRootSync => 13,
            Self::AfterSealCatalogCommitAfterRootSync => 14,
            Self::AfterCheckpointCatalogCommitAfterRootSync => 15,
        }
    }

    pub const fn name(self) -> &'static str {
        match self {
            Self::BeforeFrameFlush => "before_frame_flush",
            Self::AfterFrameFlushBeforeSync => "after_frame_flush_before_sync",
            Self::AfterSegmentSyncBeforeCatalogCommit => "after_segment_sync_before_catalog_commit",
            Self::DuringCatalogCommit => "during_catalog_commit",
            Self::AfterCatalogCommitBeforeRootSync => "after_catalog_commit_before_root_sync",
            Self::SealRename => "seal_rename",
            Self::AfterSealRenameBeforeNewActive => "after_seal_rename_before_new_active",
            Self::AfterSealNewActiveBeforeCatalogCommit => {
                "after_seal_new_active_before_catalog_commit"
            }
            Self::AfterSealCatalogCommitBeforeRootSync => {
                "after_seal_catalog_commit_before_root_sync"
            }
            Self::IndexDeletion => "index_deletion",
            Self::AfterCheckpointSyncBeforeCatalogCommit => {
                "after_checkpoint_sync_before_catalog_commit"
            }
            Self::AfterCheckpointCatalogCommitBeforeRootSync => {
                "after_checkpoint_catalog_commit_before_root_sync"
            }
            Self::AfterAppendCatalogCommitAfterRootSync => {
                "after_append_catalog_commit_after_root_sync"
            }
            Self::AfterSealCatalogCommitAfterRootSync => {
                "after_seal_catalog_commit_after_root_sync"
            }
            Self::AfterCheckpointCatalogCommitAfterRootSync => {
                "after_checkpoint_catalog_commit_after_root_sync"
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HistoryStoreObservation {
    FrameFlush,
    SegmentSync,
    CatalogCommit,
    DirectorySync,
}

pub trait HistoryStoreHooks: Send + Sync {
    fn monotonic_millis(&self) -> u64;
    fn should_fail(&self, _point: FailurePoint) -> bool {
        false
    }
    fn set_failure(&self, _point: Option<FailurePoint>) {}
    fn observe(&self, _observation: HistoryStoreObservation) {}
}

struct SystemHooks {
    started: Instant,
    failure: AtomicUsize,
}

impl Default for SystemHooks {
    fn default() -> Self {
        Self {
            started: Instant::now(),
            failure: AtomicUsize::new(0),
        }
    }
}

impl HistoryStoreHooks for SystemHooks {
    fn monotonic_millis(&self) -> u64 {
        u64::try_from(self.started.elapsed().as_millis()).unwrap_or(u64::MAX)
    }

    fn should_fail(&self, point: FailurePoint) -> bool {
        self.failure
            .compare_exchange(point.code(), 0, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
    }

    fn set_failure(&self, point: Option<FailurePoint>) {
        self.failure
            .store(point.map_or(0, FailurePoint::code), Ordering::SeqCst);
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CatalogSettings {
    pub journal_mode: String,
    pub synchronous: u32,
    pub foreign_keys: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CatalogCounts {
    pub generations: u64,
    pub segments: u64,
    pub frames: u64,
    pub events: u64,
    pub cause_edges: u64,
    pub effect_edges: u64,
    pub evidence: u64,
    pub inputs: u64,
    pub checkpoints: u64,
    pub snapshots: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HistoryStoreStatus {
    pub logical_generation: u64,
    pub durable_generation: u64,
    pub pending_frames: usize,
    pub active_extent: u64,
    pub logical_records: usize,
    pub durable_records: usize,
    pub causal_events: usize,
    pub causal_traces: usize,
    pub logical_digest: [u8; 32],
    pub durable_digest: [u8; 32],
    pub evidence_manifest_digest: [u8; 32],
    pub durable_evidence_manifest_digest: [u8; 32],
    pub catalog_counts: CatalogCounts,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CatalogFrameRange {
    pub generation: u64,
    pub frame_ordinal: u64,
    pub segment_sequence: u64,
    pub relative_path: PathBuf,
    pub range: Range<u64>,
    pub frame_digest: [u8; 32],
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CatalogCheckpoint {
    pub generation: u64,
    pub time: SimulationTime,
    pub digest: [u8; 32],
    pub relative_path: PathBuf,
}

pub trait HistoryCatalog {
    fn durable_generation(&self) -> Result<u64, PersistenceError>;
    fn frame_ranges(&self, generation: u64) -> Result<Vec<CatalogFrameRange>, PersistenceError>;
    fn counts(&self) -> Result<CatalogCounts, PersistenceError>;
    fn settings(&self) -> Result<CatalogSettings, PersistenceError>;
    fn checkpoint_for_generation(
        &self,
        generation: u64,
    ) -> Result<Option<CatalogCheckpoint>, PersistenceError>;
}

pub struct SqliteHistoryCatalog {
    root: PathBuf,
    connection: Connection,
}

impl fmt::Debug for SqliteHistoryCatalog {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SqliteHistoryCatalog")
            .field("root", &self.root)
            .finish_non_exhaustive()
    }
}

#[derive(Clone, Debug)]
struct PendingFrame {
    generation: u64,
    frame_ordinal: u64,
    segment_sequence: u64,
    relative_path: PathBuf,
    range: Range<u64>,
    frame_digest: [u8; 32],
    records: Vec<HistoryRecord>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AppendReceipt {
    logical_generation: u64,
    durable_generation: u64,
    range: Range<u64>,
    frame_digest: [u8; 32],
    logical_digest: [u8; 32],
    evidence_manifest_digest: [u8; 32],
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheckpointReceipt {
    generation: u64,
    digest: [u8; 32],
    path: PathBuf,
}

impl CheckpointReceipt {
    pub const fn generation(&self) -> u64 {
        self.generation
    }
    pub const fn digest(&self) -> &[u8; 32] {
        &self.digest
    }
    pub fn digest_hex(&self) -> String {
        hex(&self.digest)
    }
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl AppendReceipt {
    pub const fn logical_generation(&self) -> u64 {
        self.logical_generation
    }
    pub const fn durable_generation(&self) -> u64 {
        self.durable_generation
    }
    pub const fn range(&self) -> &Range<u64> {
        &self.range
    }
    pub fn frame_digest_hex(&self) -> String {
        hex(&self.frame_digest)
    }
    pub const fn logical_digest(&self) -> &[u8; 32] {
        &self.logical_digest
    }
    pub const fn evidence_manifest_digest(&self) -> &[u8; 32] {
        &self.evidence_manifest_digest
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SealedSegment {
    identity: [u8; 32],
    path: PathBuf,
}

impl SealedSegment {
    pub const fn identity(&self) -> &[u8; 32] {
        &self.identity
    }
    pub fn identity_hex(&self) -> String {
        hex(&self.identity)
    }
    pub fn path(&self) -> &Path {
        &self.path
    }
}

struct WriterLease {
    _file: File,
}

struct AppendPreimage {
    active_sequence: u64,
    active_relative_path: PathBuf,
    active_file: File,
    active_end: u64,
    next_frame_ordinal: u64,
    logical_generation: u64,
    durable_generation: u64,
    logical_records: u64,
    durable_records: u64,
    causal_events: u64,
    causal_traces: u64,
    pending: Vec<PendingFrame>,
    group_started_ms: Option<u64>,
    logical_hasher: LogicalHistoryDigest,
    logical_digest: [u8; 32],
    durable_digest: [u8; 32],
    evidence_manifest_digest: [u8; 32],
    durable_evidence_manifest_digest: [u8; 32],
    evidence_hasher: EvidenceManifestDigest,
}

impl AppendPreimage {
    fn capture(store: &HistoryStore) -> Result<Self, PersistenceError> {
        Ok(Self {
            active_sequence: store.active_sequence,
            active_relative_path: store.active_relative_path.clone(),
            active_file: store.active_file.try_clone().map_err(|error| {
                io_error("clone active segment handle", &store.active_path(), error)
            })?,
            active_end: store.active_end,
            next_frame_ordinal: store.next_frame_ordinal,
            logical_generation: store.logical_generation,
            durable_generation: store.durable_generation,
            logical_records: store.logical_records,
            durable_records: store.durable_records,
            causal_events: store.causal_events,
            causal_traces: store.causal_traces,
            pending: store.pending.clone(),
            group_started_ms: store.group_started_ms,
            logical_hasher: store.logical_hasher.clone(),
            logical_digest: store.logical_digest,
            durable_digest: store.durable_digest,
            evidence_manifest_digest: store.evidence_manifest_digest,
            durable_evidence_manifest_digest: store.durable_evidence_manifest_digest,
            evidence_hasher: store.evidence_hasher.clone(),
        })
    }

    fn rollback(
        self,
        store: &mut HistoryStore,
        _frame_causal: &[OwnedCausalRecord],
    ) -> Result<(), PersistenceError> {
        let mut active_file = self.active_file;
        active_file
            .set_len(self.active_end)
            .and_then(|_| active_file.sync_all())
            .and_then(|_| active_file.seek(SeekFrom::Start(self.active_end)))
            .map_err(|error| {
                io_error(
                    "rollback active segment extent",
                    &store.root.join(&self.active_relative_path),
                    error,
                )
            })?;
        store.active_sequence = self.active_sequence;
        store.active_relative_path = self.active_relative_path;
        store.active_file = active_file;
        store.active_end = self.active_end;
        store.next_frame_ordinal = self.next_frame_ordinal;
        store.logical_generation = self.logical_generation;
        store.durable_generation = self.durable_generation;
        store.logical_records = self.logical_records;
        store.durable_records = self.durable_records;
        store.causal_events = self.causal_events;
        store.causal_traces = self.causal_traces;
        store.pending = self.pending;
        store.group_started_ms = self.group_started_ms;
        store.logical_hasher = self.logical_hasher;
        store.logical_digest = self.logical_digest;
        store.durable_digest = self.durable_digest;
        store.evidence_manifest_digest = self.evidence_manifest_digest;
        store.durable_evidence_manifest_digest = self.durable_evidence_manifest_digest;
        store.evidence_hasher = self.evidence_hasher;
        Ok(())
    }
}

pub struct HistoryStore {
    root: PathBuf,
    namespace: [u8; 32],
    mode: DurabilityMode,
    catalog: SqliteHistoryCatalog,
    registry: Arc<EvidenceDecoderRegistry>,
    active_sequence: u64,
    active_relative_path: PathBuf,
    active_file: File,
    active_end: u64,
    next_frame_ordinal: u64,
    logical_generation: u64,
    durable_generation: u64,
    logical_records: u64,
    durable_records: u64,
    causal_events: u64,
    causal_traces: u64,
    pending: Vec<PendingFrame>,
    group_started_ms: Option<u64>,
    logical_hasher: LogicalHistoryDigest,
    logical_digest: [u8; 32],
    durable_digest: [u8; 32],
    evidence_manifest_digest: [u8; 32],
    durable_evidence_manifest_digest: [u8; 32],
    evidence_hasher: EvidenceManifestDigest,
    hooks: Arc<dyn HistoryStoreHooks>,
    _writer_lease: WriterLease,
}

impl fmt::Debug for HistoryStore {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("HistoryStore")
            .field("root", &self.root)
            .field("logical_generation", &self.logical_generation)
            .field("durable_generation", &self.durable_generation)
            .finish_non_exhaustive()
    }
}

impl HistoryStore {
    pub fn create(
        root: &Path,
        namespace: [u8; 32],
        mode: DurabilityMode,
    ) -> Result<Self, PersistenceError> {
        Self::create_with_registry(
            root,
            namespace,
            mode,
            Arc::new(EvidenceDecoderRegistry::new()),
        )
    }

    pub fn create_with_registry(
        root: &Path,
        namespace: [u8; 32],
        mode: DurabilityMode,
        registry: Arc<EvidenceDecoderRegistry>,
    ) -> Result<Self, PersistenceError> {
        Self::create_with_registry_and_hooks(
            root,
            namespace,
            mode,
            registry,
            Arc::new(SystemHooks::default()),
        )
    }

    pub fn create_with_hooks(
        root: &Path,
        namespace: [u8; 32],
        mode: DurabilityMode,
        hooks: Arc<dyn HistoryStoreHooks>,
    ) -> Result<Self, PersistenceError> {
        Self::create_with_registry_and_hooks(
            root,
            namespace,
            mode,
            Arc::new(EvidenceDecoderRegistry::new()),
            hooks,
        )
    }

    pub fn create_with_registry_and_hooks(
        root: &Path,
        namespace: [u8; 32],
        mode: DurabilityMode,
        registry: Arc<EvidenceDecoderRegistry>,
        hooks: Arc<dyn HistoryStoreHooks>,
    ) -> Result<Self, PersistenceError> {
        create_layout(root)?;
        let lease = acquire_writer(root)?;
        let mut catalog = SqliteHistoryCatalog::create(root, namespace, mode)?;
        let active_relative_path = active_relative_path(0);
        let active_file = open_new_active(root, &active_relative_path)?;
        catalog.insert_active_segment(0, &active_relative_path)?;
        File::open(root)
            .and_then(|directory| directory.sync_all())
            .map_err(|error| io_error("sync new bundle root", root, error))?;
        Ok(Self {
            root: root.to_path_buf(),
            namespace,
            mode,
            catalog,
            registry,
            active_sequence: 0,
            active_relative_path,
            active_file,
            active_end: 0,
            next_frame_ordinal: 0,
            logical_generation: 0,
            durable_generation: 0,
            logical_records: 0,
            durable_records: 0,
            causal_events: 0,
            causal_traces: 0,
            pending: Vec::new(),
            group_started_ms: None,
            logical_hasher: LogicalHistoryDigest::new(),
            logical_digest: logical_history_digest(&[])?,
            durable_digest: logical_history_digest(&[])?,
            evidence_manifest_digest: EvidenceManifestDigest::new().finish(),
            durable_evidence_manifest_digest: EvidenceManifestDigest::new().finish(),
            evidence_hasher: EvidenceManifestDigest::new(),
            hooks,
            _writer_lease: lease,
        })
    }

    pub fn open(root: &Path) -> Result<Self, PersistenceError> {
        Self::open_with_registry(root, Arc::new(EvidenceDecoderRegistry::new()))
    }

    pub fn open_with_registry(
        root: &Path,
        registry: Arc<EvidenceDecoderRegistry>,
    ) -> Result<Self, PersistenceError> {
        Self::open_with_registry_and_hooks(root, registry, Arc::new(SystemHooks::default()))
    }

    pub fn open_with_hooks(
        root: &Path,
        hooks: Arc<dyn HistoryStoreHooks>,
    ) -> Result<Self, PersistenceError> {
        Self::open_with_registry_and_hooks(root, Arc::new(EvidenceDecoderRegistry::new()), hooks)
    }

    pub fn open_with_registry_and_hooks(
        root: &Path,
        registry: Arc<EvidenceDecoderRegistry>,
        hooks: Arc<dyn HistoryStoreHooks>,
    ) -> Result<Self, PersistenceError> {
        let lease = acquire_writer(root)?;
        let mut catalog = SqliteHistoryCatalog::open(root)?;
        let namespace = catalog.namespace()?;
        let mode = catalog.mode()?;
        let durable_generation = catalog.durable_generation()?;
        catalog.verify_sealed_segments(namespace)?;
        catalog.verify_checkpoints(namespace, durable_generation, &registry)?;
        catalog.rebuild_indexes_if_incomplete(&registry, hooks.as_ref())?;
        let (active_sequence, active_relative_path, durable_end) = catalog.active_segment()?;
        let active_path = resolve_segment_path(
            root,
            active_sequence,
            false,
            None,
            &active_relative_path,
            true,
        )?;
        let mut active_file = open_existing_active(&active_path)?;
        active_file
            .seek(SeekFrom::Start(durable_end))
            .map_err(|error| io_error("seek active segment", &active_path, error))?;
        let next_frame_ordinal = catalog.next_frame_ordinal()?;
        let mut logical_hasher = LogicalHistoryDigest::new();
        let mut evidence_hasher = EvidenceManifestDigest::new();
        let mut logical_records = 0_u64;
        let mut causal_events = 0_u64;
        let mut causal_traces = 0_u64;
        catalog.for_each_record(durable_generation, &registry, |record| {
            logical_hasher.append(record)?;
            if matches!(record, HistoryRecord::Causal(_)) {
                causal_events = causal_events
                    .checked_add(1)
                    .ok_or(PersistenceError::HistoryArithmeticOverflow)?;
                causal_traces = causal_traces
                    .checked_add(1)
                    .ok_or(PersistenceError::HistoryArithmeticOverflow)?;
            }
            logical_records = logical_records
                .checked_add(1)
                .ok_or(PersistenceError::HistoryArithmeticOverflow)?;
            evidence_hasher.append(record)
        })?;
        let logical_digest = logical_hasher.finish();
        let evidence_manifest_digest = evidence_hasher.finish();
        Ok(Self {
            root: root.to_path_buf(),
            namespace,
            mode,
            catalog,
            registry,
            active_sequence,
            active_relative_path,
            active_file,
            active_end: durable_end,
            next_frame_ordinal,
            logical_generation: durable_generation,
            durable_generation,
            logical_records,
            durable_records: logical_records,
            causal_events,
            causal_traces,
            pending: Vec::new(),
            group_started_ms: None,
            logical_hasher,
            logical_digest,
            durable_digest: logical_digest,
            evidence_manifest_digest,
            durable_evidence_manifest_digest: evidence_manifest_digest,
            evidence_hasher,
            hooks,
            _writer_lease: lease,
        })
    }

    /// The logical history digest recomputed from the canonical frames.
    ///
    /// Folds every record of the durable generation as decoded from its
    /// segments, then every record of the logical frames not yet published.
    /// It shares nothing with the incrementally maintained fold except the
    /// per-record encoding, so it is the cold oracle that fold is checked
    /// against. It costs a read of the whole history and is never on a tick.
    pub fn logical_digest_full_rescan(&self) -> Result<[u8; 32], PersistenceError> {
        let mut hasher = LogicalHistoryDigest::new();
        self.catalog
            .for_each_record(self.durable_generation, &self.registry, |record| {
                hasher.append(record)
            })?;
        for frame in &self.pending {
            for record in &frame.records {
                hasher.append(record)?;
            }
        }
        Ok(hasher.finish())
    }

    pub const fn logical_generation(&self) -> u64 {
        self.logical_generation
    }
    pub const fn durable_generation(&self) -> u64 {
        self.durable_generation
    }
    pub const fn mode(&self) -> DurabilityMode {
        self.mode
    }
    pub const fn namespace(&self) -> &[u8; 32] {
        &self.namespace
    }
    pub fn catalog(&self) -> &dyn HistoryCatalog {
        &self.catalog
    }
    pub fn active_segment_path(&self) -> PathBuf {
        self.active_path()
    }

    pub fn status(&self) -> Result<HistoryStoreStatus, PersistenceError> {
        let active_path = self.active_path();
        let active_extent = self
            .active_file
            .metadata()
            .map_err(|error| io_error("read active segment metadata", &active_path, error))?
            .len();
        Ok(HistoryStoreStatus {
            logical_generation: self.logical_generation,
            durable_generation: self.durable_generation,
            pending_frames: self.pending.len(),
            active_extent,
            logical_records: usize::try_from(self.logical_records)?,
            durable_records: usize::try_from(self.durable_records)?,
            causal_events: usize::try_from(self.causal_events)?,
            causal_traces: usize::try_from(self.causal_traces)?,
            logical_digest: self.logical_digest,
            durable_digest: self.durable_digest,
            evidence_manifest_digest: self.evidence_manifest_digest,
            durable_evidence_manifest_digest: self.durable_evidence_manifest_digest,
            catalog_counts: self.catalog.counts()?,
        })
    }

    pub fn inject_failure(&self, point: Option<FailurePoint>) {
        self.hooks.set_failure(point);
    }

    pub fn append_tick(
        &mut self,
        records: Vec<HistoryRecord>,
    ) -> Result<AppendReceipt, PersistenceError> {
        self.append_tick_transaction(records, false)
    }

    pub fn append_tick_durable(
        &mut self,
        records: Vec<HistoryRecord>,
    ) -> Result<AppendReceipt, PersistenceError> {
        self.append_tick_transaction(records, true)
    }

    pub fn advance_logical_generation(
        &mut self,
        target_generation: u64,
    ) -> Result<(), PersistenceError> {
        if target_generation < self.logical_generation {
            return Err(PersistenceError::HistoryCatalogInvariant {
                invariant: "logical generation advance target is before current generation",
            });
        }
        if target_generation == self.logical_generation {
            return Ok(());
        }
        if !self.pending.is_empty() || self.logical_generation != self.durable_generation {
            return Err(PersistenceError::HistoryCatalogInvariant {
                invariant: "logical generation advance requires a durable store",
            });
        }
        self.catalog.advance_logical_generation(
            self.durable_generation,
            target_generation,
            self.logical_digest,
            self.hooks.as_ref(),
        )?;
        self.logical_generation = target_generation;
        self.durable_generation = target_generation;
        self.durable_digest = self.logical_digest;
        self.durable_evidence_manifest_digest = self.evidence_manifest_digest;
        Ok(())
    }

    fn append_tick_transaction(
        &mut self,
        records: Vec<HistoryRecord>,
        force_durable: bool,
    ) -> Result<AppendReceipt, PersistenceError> {
        let records = canonical_history_records(records)?;
        let frame_causal = causal_records(&records);
        validate_causal_batch_against_catalog(&self.catalog, &frame_causal, &self.pending)?;
        let preimage = AppendPreimage::capture(self)?;
        let catalog_predecessor = preimage.durable_generation;
        let catalog_active_sequence = preimage.active_sequence;
        let catalog_active_path = preimage.active_relative_path.clone();
        let catalog_active_end = preimage.active_end;
        let result = self.append_tick_inner(records, frame_causal.clone());
        let result = result.and_then(|receipt| {
            if force_durable && receipt.durable_generation() != receipt.logical_generation() {
                self.flush_durable()?;
            }
            Ok(receipt)
        });
        match result {
            Ok(receipt) => Ok(receipt),
            Err(error) => {
                let rollback_error = preimage.rollback(self, &frame_causal).err();
                let catalog_error = self
                    .catalog
                    .rollback_to_preimage(
                        catalog_predecessor,
                        catalog_active_sequence,
                        &catalog_active_path,
                        catalog_active_end,
                    )
                    .err();
                rollback_error.or(catalog_error).map_or(Err(error), Err)
            }
        }
    }

    fn append_tick_inner(
        &mut self,
        records: Vec<HistoryRecord>,
        frame_causal: Vec<OwnedCausalRecord>,
    ) -> Result<AppendReceipt, PersistenceError> {
        let contains_evidence = records
            .iter()
            .any(|record| matches!(record, HistoryRecord::Evidence(_)));
        let generation = self
            .logical_generation
            .checked_add(1)
            .ok_or(PersistenceError::HistoryArithmeticOverflow)?;
        let first_record_sequence = self
            .logical_records
            .checked_add(1)
            .ok_or(PersistenceError::HistoryArithmeticOverflow)?;
        let last_record_sequence = first_record_sequence
            .checked_add(u64::try_from(records.len().saturating_sub(1))?)
            .ok_or(PersistenceError::HistoryArithmeticOverflow)?;
        let (first_event_id, last_event_id, first_trace_id, last_trace_id) =
            causal_ranges(&records);
        let fields = HistoryFrameHeaderFields {
            namespace: self.namespace,
            catalog_generation: generation,
            first_record_sequence,
            last_record_sequence,
            first_event_id,
            last_event_id,
            first_trace_id,
            last_trace_id,
            prior_logical_history_digest: self.logical_digest,
        };
        let bytes = encode_history_frame(fields, &records)?;
        let decoded = decode_history_frame(&bytes, &self.registry)?;
        let frame_digest = decoded.header().frame_digest;
        let start = self.active_end;
        let end = start
            .checked_add(u64::try_from(bytes.len())?)
            .ok_or(PersistenceError::HistoryArithmeticOverflow)?;
        self.active_file
            .seek(SeekFrom::Start(start))
            .and_then(|_| self.active_file.write_all(&bytes))
            .map_err(|error| io_error("append complete frame", &self.active_path(), error))?;
        let pending = PendingFrame {
            generation,
            frame_ordinal: self.next_frame_ordinal,
            segment_sequence: self.active_sequence,
            relative_path: self.active_relative_path.clone(),
            range: start..end,
            frame_digest,
            records: records.clone(),
        };
        self.active_end = end;
        self.next_frame_ordinal = self
            .next_frame_ordinal
            .checked_add(1)
            .ok_or(PersistenceError::HistoryArithmeticOverflow)?;
        self.logical_generation = generation;
        for record in &records {
            self.logical_hasher.append(record)?;
        }
        self.logical_records = self
            .logical_records
            .checked_add(u64::try_from(records.len())?)
            .ok_or(PersistenceError::HistoryArithmeticOverflow)?;
        self.causal_events = self
            .causal_events
            .checked_add(u64::try_from(frame_causal.len())?)
            .ok_or(PersistenceError::HistoryArithmeticOverflow)?;
        self.causal_traces = self
            .causal_traces
            .checked_add(u64::try_from(frame_causal.len())?)
            .ok_or(PersistenceError::HistoryArithmeticOverflow)?;
        if contains_evidence {
            for record in &records {
                self.evidence_hasher.append(record)?;
            }
        }
        self.logical_digest = self.logical_hasher.finish();
        if contains_evidence {
            self.evidence_manifest_digest = self.evidence_hasher.finish();
        }
        if self.pending.is_empty() {
            self.group_started_ms = Some(self.hooks.monotonic_millis());
        }
        self.pending.push(pending);
        let receipt_range = start..end;
        if self.mode == DurabilityMode::EveryTick || self.group_boundary_reached() {
            self.flush_durable()?;
            if self.active_end >= SEGMENT_TARGET_BYTES {
                self.seal_active()?;
            }
        }
        Ok(AppendReceipt {
            logical_generation: generation,
            durable_generation: self.durable_generation,
            range: receipt_range,
            frame_digest,
            logical_digest: self.logical_digest,
            evidence_manifest_digest: self.evidence_manifest_digest,
        })
    }

    pub fn flush_durable(&mut self) -> Result<(), PersistenceError> {
        if self.pending.is_empty() {
            return Ok(());
        }
        let preimage = AppendPreimage::capture(self)?;
        let (_, _, previous_durable_end) = self.catalog.active_segment()?;
        let predecessor = self.durable_generation;
        let segment_sequence = self.active_sequence;
        let active_relative_path = preimage.active_relative_path.clone();
        let result = (|| {
            self.fail(FailurePoint::BeforeFrameFlush)?;
            self.active_file
                .flush()
                .map_err(|error| io_error("flush active segment", &self.active_path(), error))?;
            self.hooks.observe(HistoryStoreObservation::FrameFlush);
            self.fail(FailurePoint::AfterFrameFlushBeforeSync)?;
            self.active_file
                .sync_all()
                .map_err(|error| io_error("sync active segment", &self.active_path(), error))?;
            self.hooks.observe(HistoryStoreObservation::SegmentSync);
            sync_directory(&self.root.join("segments"), self.hooks.as_ref())?;
            self.fail(FailurePoint::AfterSegmentSyncBeforeCatalogCommit)?;
            self.catalog.publish(
                self.durable_generation,
                self.logical_generation,
                self.logical_digest,
                &self.pending,
                &self.registry,
                self.hooks.as_ref(),
            )?;
            self.hooks.observe(HistoryStoreObservation::CatalogCommit);
            self.fail(FailurePoint::AfterCatalogCommitBeforeRootSync)?;
            sync_directory(&self.root, self.hooks.as_ref())?;
            self.fail(FailurePoint::AfterAppendCatalogCommitAfterRootSync)?;
            self.durable_generation = self.logical_generation;
            self.durable_digest = self.logical_digest;
            self.durable_evidence_manifest_digest = self.evidence_manifest_digest;
            self.durable_records = self.logical_records;
            self.pending.clear();
            self.group_started_ms = None;
            Ok(())
        })();
        match result {
            Ok(()) => Ok(()),
            Err(error) => {
                let rollback_error = preimage.rollback(self, &[]).err();
                let catalog_error = self
                    .catalog
                    .rollback_to_preimage(
                        predecessor,
                        segment_sequence,
                        &active_relative_path,
                        previous_durable_end,
                    )
                    .err();
                rollback_error.or(catalog_error).map_or(Err(error), Err)
            }
        }
    }

    pub fn flush_if_due(&mut self) -> Result<bool, PersistenceError> {
        if self.mode == DurabilityMode::GroupCommit && self.group_boundary_reached() {
            self.flush_durable()?;
            return Ok(true);
        }
        Ok(false)
    }

    pub fn durable_reader(&self) -> Result<HistoryStoreReader, PersistenceError> {
        if !self
            .catalog
            .indexes_match_canonical(self.durable_generation, &self.registry)?
        {
            return Err(PersistenceError::HistoryCatalogInvariant {
                invariant: "derived catalog index rows do not match canonical frames",
            });
        }
        let checkpoint = self
            .catalog
            .checkpoint_for_generation(self.durable_generation)?;
        let frontier = self.catalog.frontier(self.durable_generation)?;
        HistoryStoreReader::new_indexed(
            &self.root,
            Arc::clone(&self.registry),
            self.namespace,
            self.mode,
            self.durable_generation,
            self.durable_generation,
            self.durable_generation,
            frontier,
            checkpoint,
            self.durable_digest,
            self.durable_evidence_manifest_digest,
            Vec::new(),
        )
    }

    pub fn logical_reader(&self) -> Result<HistoryStoreReader, PersistenceError> {
        let checkpoint = self
            .catalog
            .checkpoint_for_generation(self.durable_generation)?;
        let mut overlay = Vec::new();
        for frame in &self.pending {
            overlay.extend(frame.records.iter().cloned());
        }
        let frontier = overlay
            .iter()
            .rev()
            .find_map(|record| match record {
                HistoryRecord::Causal(event) => Some(HistoryFrontier::new(
                    self.logical_generation,
                    event.event_id(),
                    event.trace_id(),
                    event.time(),
                )),
                HistoryRecord::Evidence(_) => None,
            })
            .unwrap_or(self.catalog.frontier(self.durable_generation)?);
        let frontier = HistoryFrontier::new(
            self.logical_generation,
            frontier.event_id,
            frontier.trace_id,
            frontier.time,
        );
        HistoryStoreReader::new_indexed(
            &self.root,
            Arc::clone(&self.registry),
            self.namespace,
            self.mode,
            self.logical_generation,
            self.durable_generation,
            self.durable_generation,
            frontier,
            checkpoint,
            self.logical_digest,
            self.evidence_manifest_digest,
            overlay,
        )
    }

    pub fn reader_at_durable_generation(
        &self,
        generation: u64,
    ) -> Result<HistoryStoreReader, PersistenceError> {
        if !self.catalog.has_generation(generation)? {
            return Err(PersistenceError::HistoryGenerationUnavailable { generation });
        }
        let (logical_digest, evidence_digest) = self
            .catalog
            .generation_digests(generation, &self.registry)?;
        let checkpoint = self.catalog.checkpoint_for_generation(generation)?;
        let frontier = self.catalog.frontier(generation)?;
        HistoryStoreReader::new_indexed(
            &self.root,
            Arc::clone(&self.registry),
            self.namespace,
            self.mode,
            generation,
            generation,
            generation,
            frontier,
            checkpoint,
            logical_digest,
            evidence_digest,
            Vec::new(),
        )
    }

    pub fn publish_checkpoint(
        &mut self,
        anchor: &HistoryAnchor,
        metadata: &CheckpointMetadata,
        payload: &[u8],
    ) -> Result<CheckpointReceipt, PersistenceError> {
        if anchor.logical_generation() != self.logical_generation
            || anchor.durable_generation() != self.durable_generation
        {
            return Err(PersistenceError::CheckpointGenerationMismatch {
                expected_logical: self.logical_generation,
                expected_durable: self.durable_generation,
                actual_logical: anchor.logical_generation(),
                actual_durable: anchor.durable_generation(),
            });
        }
        if anchor.namespace() != &self.namespace[..16] {
            return Err(PersistenceError::HistoryCatalogInvariant {
                invariant: "checkpoint namespace does not match history store",
            });
        }

        self.flush_durable()?;
        let generation = self.durable_generation;
        if self.catalog.checkpoint_at_generation(generation)?.is_some() {
            return Err(PersistenceError::HistoryCatalogInvariant {
                invariant: "checkpoint already published for generation",
            });
        }
        let logical_digest = self.durable_digest;
        if anchor.logical_digest() != &logical_digest {
            return Err(PersistenceError::LogicalDigestMismatch);
        }
        let manifest_digest = self.durable_evidence_manifest_digest;
        if anchor.evidence_manifest_digest() != &manifest_digest {
            return Err(PersistenceError::HistoryCatalogInvariant {
                invariant: "checkpoint evidence manifest does not match durable history",
            });
        }
        let durable_anchor = HistoryAnchor::new(HistoryAnchorFields {
            namespace: *anchor.namespace(),
            logical_generation: generation,
            durable_generation: generation,
            checkpoint_digest: *anchor.checkpoint_digest(),
            frontier: anchor.frontier(),
            logical_digest,
            storage_root: *anchor.storage_root(),
            physical_digest_schema: anchor.physical_digest_schema(),
            history_digest_schema: anchor.history_digest_schema(),
            evidence_manifest_digest: manifest_digest,
            durability: anchor.durability(),
        })?;
        let bytes = encode_checkpoint(&durable_anchor, metadata, payload)?;
        let envelope = decode_checkpoint_with_generations(&bytes, generation, generation)?;
        let digest = envelope.digest;
        let relative_path = PathBuf::from("checkpoints").join(format!("{}.ckp", hex(&digest)));
        let final_path =
            resolve_catalog_path(&self.root, &relative_path, "checkpoints", "ckp", false)?;
        let staging_relative =
            PathBuf::from("staging").join(format!("checkpoint-{generation}-{}.tmp", hex(&digest)));
        let staging_path =
            resolve_catalog_path(&self.root, &staging_relative, "staging", "tmp", false)?;
        let final_preexisted = final_path.exists();
        let mut staging_file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&staging_path)
            .map_err(|error| io_error("create staged checkpoint", &staging_path, error))?;
        staging_file
            .write_all(&bytes)
            .and_then(|_| staging_file.flush())
            .and_then(|_| staging_file.sync_all())
            .map_err(|error| io_error("sync staged checkpoint", &staging_path, error))?;
        drop(staging_file);
        if final_preexisted {
            let existing = fs::read(&final_path)
                .map_err(|error| io_error("read orphan checkpoint", &final_path, error))?;
            if existing != bytes {
                return Err(PersistenceError::CheckpointDigestMismatch);
            }
            fs::remove_file(&staging_path).map_err(|error| {
                io_error("remove resumed checkpoint staging", &staging_path, error)
            })?;
        } else {
            fs::rename(&staging_path, &final_path)
                .map_err(|error| io_error("publish checkpoint file", &staging_path, error))?;
        }
        let mut catalog_attempted = false;
        let publication = (|| {
            sync_directory(&self.root.join("checkpoints"), self.hooks.as_ref())?;
            self.fail(FailurePoint::AfterCheckpointSyncBeforeCatalogCommit)?;
            catalog_attempted = true;
            self.catalog.publish_checkpoint(
                generation,
                envelope.header.frontier.time,
                digest,
                &relative_path,
            )?;
            self.fail(FailurePoint::AfterCheckpointCatalogCommitBeforeRootSync)?;
            sync_directory(&self.root, self.hooks.as_ref())?;
            self.fail(FailurePoint::AfterCheckpointCatalogCommitAfterRootSync)
        })();
        if let Err(error) = publication {
            let catalog_error = if catalog_attempted {
                self.catalog
                    .rollback_checkpoint(generation, digest, &relative_path)
                    .err()
            } else {
                None
            };
            let file_error = if !final_preexisted {
                remove_checkpoint_file(&final_path, self.hooks.as_ref()).err()
            } else {
                None
            };
            let root_error = sync_directory(&self.root, self.hooks.as_ref()).err();
            return catalog_error
                .or(file_error)
                .or(root_error)
                .map_or(Err(error), Err);
        }
        Ok(CheckpointReceipt {
            generation,
            digest,
            path: final_path,
        })
    }

    pub fn checkpoint_at_durable_generation(
        &self,
        generation: u64,
    ) -> Result<Option<CheckpointEnvelope>, PersistenceError> {
        if !self.catalog.has_generation(generation)? {
            return Err(PersistenceError::HistoryGenerationUnavailable { generation });
        }
        self.catalog
            .checkpoint_for_generation(generation)?
            .map(|checkpoint| {
                self.catalog
                    .read_checkpoint(&checkpoint, self.namespace, &self.registry)
            })
            .transpose()
    }

    pub fn seal_active(&mut self) -> Result<SealedSegment, PersistenceError> {
        self.flush_durable()?;
        if self.active_end == 0 {
            return Err(PersistenceError::HistoryCatalogInvariant {
                invariant: "cannot seal an empty active segment",
            });
        }
        let active_path = self.active_path();
        let bytes = read_exact_range(
            &active_path,
            0..self.active_end,
            max_stored_segment_bytes()?,
        )?;
        let identity_header = segment_identity_header(self.namespace, self.active_sequence);
        let identity = sealed_segment_identity(&identity_header, &[&bytes]);
        let relative = PathBuf::from("segments").join(format!("{}.cfs", hex(&identity)));
        let sealed_path = resolve_catalog_path(&self.root, &relative, "segments", "cfs", false)?;
        let new_sequence = self
            .active_sequence
            .checked_add(1)
            .ok_or(PersistenceError::HistoryArithmeticOverflow)?;
        let new_relative = active_relative_path(new_sequence);
        let new_path = resolve_catalog_path(&self.root, &new_relative, "segments", "cfs", false)?;
        ensure_absent_seal_path(&sealed_path)?;
        ensure_absent_seal_path(&new_path)?;
        let old_sequence = self.active_sequence;
        let old_relative = self.active_relative_path.clone();
        let old_active_path = active_path.clone();
        let result = (|| {
            self.fail(FailurePoint::SealRename)?;
            fs::rename(&old_active_path, &sealed_path).map_err(|error| {
                io_error("rename active segment to sealed", &old_active_path, error)
            })?;
            self.fail(FailurePoint::AfterSealRenameBeforeNewActive)?;
            let new_file = open_new_active(&self.root, &new_relative)?;
            self.fail(FailurePoint::AfterSealNewActiveBeforeCatalogCommit)?;
            sync_directory(&self.root.join("segments"), self.hooks.as_ref())?;
            self.catalog.seal_segment(
                old_sequence,
                &relative,
                identity,
                new_sequence,
                &new_relative,
            )?;
            self.fail(FailurePoint::AfterSealCatalogCommitBeforeRootSync)?;
            sync_directory(&self.root, self.hooks.as_ref())?;
            self.fail(FailurePoint::AfterSealCatalogCommitAfterRootSync)?;
            Ok(new_file)
        })();
        match result {
            Ok(new_file) => {
                self.active_sequence = new_sequence;
                self.active_relative_path = new_relative;
                self.active_file = new_file;
                self.active_end = 0;
                Ok(SealedSegment {
                    identity,
                    path: sealed_path,
                })
            }
            Err(error) => {
                let catalog_error = self
                    .catalog
                    .rollback_seal(
                        old_sequence,
                        &old_relative,
                        &relative,
                        identity,
                        new_sequence,
                        &new_relative,
                    )
                    .err();
                let cleanup_error = rollback_seal_files(
                    &sealed_path,
                    &new_path,
                    &old_active_path,
                    self.hooks.as_ref(),
                )
                .err();
                catalog_error.or(cleanup_error).map_or(Err(error), Err)
            }
        }
    }

    pub fn rebuild_indexes(&mut self) -> Result<(), PersistenceError> {
        self.catalog
            .rebuild_indexes(&self.registry, self.hooks.as_ref())
    }

    fn group_boundary_reached(&self) -> bool {
        if self.pending.len() >= GROUP_MAX_TICKS {
            return true;
        }
        self.group_started_ms.is_some_and(|started| {
            self.hooks.monotonic_millis().saturating_sub(started) >= GROUP_MAX_MILLIS
        })
    }

    fn fail(&self, point: FailurePoint) -> Result<(), PersistenceError> {
        if self.hooks.should_fail(point) {
            return Err(PersistenceError::InjectedHistoryFailure { point });
        }
        Ok(())
    }

    fn active_path(&self) -> PathBuf {
        self.root.join(&self.active_relative_path)
    }
}

impl SqliteHistoryCatalog {
    fn create(
        root: &Path,
        namespace: [u8; 32],
        mode: DurabilityMode,
    ) -> Result<Self, PersistenceError> {
        let path = root.join("catalog.sqlite");
        if path.exists() {
            return Err(PersistenceError::HistoryCatalogInvariant {
                invariant: "catalog already exists",
            });
        }
        let mut catalog = Self::connect(root, &path)?;
        catalog.create_schema()?;
        let tx = catalog
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| sqlite_error("begin catalog creation", error))?;
        tx.execute(
            "INSERT INTO metadata(key, value) VALUES ('namespace', ?1), ('durability', ?2)",
            params![
                namespace.as_slice(),
                match mode {
                    DurabilityMode::GroupCommit => "group",
                    DurabilityMode::EveryTick => "every_tick",
                }
            ],
        )
        .map_err(|error| sqlite_error("insert catalog metadata", error))?;
        tx.execute(
            "INSERT INTO generations(id, predecessor, logical_digest) VALUES (0, NULL, ?1)",
            params![logical_history_digest(&[])?.as_slice()],
        )
        .map_err(|error| sqlite_error("insert initial generation", error))?;
        tx.commit()
            .map_err(|error| sqlite_error("commit catalog creation", error))?;
        Ok(catalog)
    }

    fn open(root: &Path) -> Result<Self, PersistenceError> {
        let path = root.join("catalog.sqlite");
        if !path.is_file() {
            return Err(PersistenceError::HistoryCatalogInvariant {
                invariant: "catalog.sqlite is missing",
            });
        }
        let catalog = Self::connect(root, &path)?;
        catalog.verify_schema()?;
        Ok(catalog)
    }

    fn connect(root: &Path, path: &Path) -> Result<Self, PersistenceError> {
        let connection =
            Connection::open(path).map_err(|error| sqlite_error("open catalog", error))?;
        connection
            .pragma_update(None, "journal_mode", "WAL")
            .and_then(|_| connection.pragma_update(None, "synchronous", "FULL"))
            .and_then(|_| connection.pragma_update(None, "foreign_keys", "ON"))
            .map_err(|error| sqlite_error("configure catalog pragmas", error))?;
        connection
            .busy_timeout(std::time::Duration::from_millis(0))
            .map_err(|error| sqlite_error("configure catalog writer timeout", error))?;
        Ok(Self {
            root: root.to_path_buf(),
            connection,
        })
    }

    fn create_schema(&mut self) -> Result<(), PersistenceError> {
        self.connection
            .execute_batch(
                "CREATE TABLE metadata(key TEXT PRIMARY KEY, value BLOB NOT NULL) WITHOUT ROWID;
                 CREATE TABLE generations(id INTEGER PRIMARY KEY, predecessor INTEGER REFERENCES generations(id), logical_digest BLOB NOT NULL CHECK(length(logical_digest)=32), checkpoint_generation INTEGER REFERENCES checkpoints(generation));
                 CREATE TABLE segments(sequence INTEGER PRIMARY KEY, relative_path TEXT NOT NULL UNIQUE, sealed INTEGER NOT NULL CHECK(sealed IN (0,1)), durable_end INTEGER NOT NULL, identity BLOB CHECK(identity IS NULL OR length(identity)=32));
                 CREATE TABLE frames(generation INTEGER NOT NULL REFERENCES generations(id), frame_ordinal INTEGER PRIMARY KEY, segment_sequence INTEGER NOT NULL REFERENCES segments(sequence), start_offset INTEGER NOT NULL, end_offset INTEGER NOT NULL, frame_digest BLOB NOT NULL CHECK(length(frame_digest)=32), record_count INTEGER NOT NULL, UNIQUE(segment_sequence,start_offset), CHECK(start_offset >= 0 AND end_offset > start_offset));
                 CREATE TABLE events(generation INTEGER NOT NULL REFERENCES generations(id), trace_id BLOB PRIMARY KEY CHECK(length(trace_id)=8), event_id BLOB NOT NULL UNIQUE CHECK(length(event_id)=8), frame_ordinal INTEGER NOT NULL REFERENCES frames(frame_ordinal), record_ordinal INTEGER NOT NULL, time BLOB NOT NULL CHECK(length(time)=8));
                 CREATE TABLE cause_edges(generation INTEGER NOT NULL REFERENCES generations(id), parent_trace BLOB NOT NULL CHECK(length(parent_trace)=8), child_trace BLOB NOT NULL CHECK(length(child_trace)=8), ordinal INTEGER NOT NULL, PRIMARY KEY(child_trace, ordinal));
                 CREATE TABLE effect_edges(generation INTEGER NOT NULL REFERENCES generations(id), trace_id BLOB NOT NULL REFERENCES events(trace_id), ordinal INTEGER NOT NULL, object_kind BLOB NOT NULL CHECK(length(object_kind)=8), object_id BLOB NOT NULL CHECK(length(object_id)=8), property_id BLOB NOT NULL CHECK(length(property_id)=8), PRIMARY KEY(trace_id, ordinal));
                 CREATE TABLE evidence(generation INTEGER NOT NULL REFERENCES generations(id), schema_id BLOB NOT NULL CHECK(length(schema_id)=8), canonical_key BLOB NOT NULL, frame_ordinal INTEGER NOT NULL REFERENCES frames(frame_ordinal), record_ordinal INTEGER NOT NULL, payload_digest BLOB NOT NULL CHECK(length(payload_digest)=32), PRIMARY KEY(frame_ordinal, record_ordinal));
                 CREATE TABLE inputs(generation INTEGER NOT NULL REFERENCES generations(id), input_sequence BLOB NOT NULL CHECK(length(input_sequence)=8), payload_digest BLOB NOT NULL CHECK(length(payload_digest)=32), PRIMARY KEY(generation,input_sequence));
                 CREATE TABLE checkpoints(generation INTEGER PRIMARY KEY REFERENCES generations(id), time BLOB NOT NULL CHECK(length(time)=8), digest BLOB NOT NULL CHECK(length(digest)=32), relative_path TEXT NOT NULL);
                 CREATE TABLE snapshots(generation INTEGER PRIMARY KEY REFERENCES generations(id), digest BLOB NOT NULL CHECK(length(digest)=32), relative_path TEXT NOT NULL);
                 CREATE INDEX frames_generation_order ON frames(generation, frame_ordinal);
                 CREATE INDEX events_generation_trace ON events(generation, trace_id);
                 CREATE INDEX cause_edges_parent_child ON cause_edges(parent_trace, child_trace);
                 CREATE INDEX effect_latest_matching ON effect_edges(object_kind, object_id, property_id, trace_id DESC);
                 CREATE INDEX evidence_schema_key ON evidence(schema_id, canonical_key, generation, frame_ordinal, record_ordinal);
                 PRAGMA user_version=1;",
            )
            .map_err(|error| sqlite_error("create catalog schema", error))
    }

    fn verify_schema(&self) -> Result<(), PersistenceError> {
        let version: i64 = self
            .connection
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .map_err(|error| sqlite_error("read catalog schema version", error))?;
        if version != CATALOG_SCHEMA_VERSION {
            return Err(PersistenceError::HistoryCatalogInvariant {
                invariant: "unsupported catalog schema version",
            });
        }
        for table in [
            "generations",
            "segments",
            "frames",
            "events",
            "cause_edges",
            "effect_edges",
            "evidence",
            "inputs",
            "checkpoints",
            "snapshots",
        ] {
            let exists = self
                .connection
                .query_row(
                    "SELECT 1 FROM sqlite_master WHERE type='table' AND name=?1",
                    [table],
                    |_| Ok(()),
                )
                .optional()
                .map_err(|error| sqlite_error("verify catalog schema", error))?
                .is_some();
            if !exists {
                return Err(PersistenceError::HistoryCatalogInvariant {
                    invariant: "required catalog table is missing",
                });
            }
        }
        Ok(())
    }

    fn namespace(&self) -> Result<[u8; 32], PersistenceError> {
        let bytes: Vec<u8> = self
            .connection
            .query_row(
                "SELECT value FROM metadata WHERE key='namespace'",
                [],
                |row| row.get(0),
            )
            .map_err(|error| sqlite_error("read catalog namespace", error))?;
        bytes
            .try_into()
            .map_err(|_| PersistenceError::HistoryCatalogInvariant {
                invariant: "catalog namespace width",
            })
    }

    fn mode(&self) -> Result<DurabilityMode, PersistenceError> {
        let value: String = self
            .connection
            .query_row(
                "SELECT value FROM metadata WHERE key='durability'",
                [],
                |row| row.get(0),
            )
            .map_err(|error| sqlite_error("read durability mode", error))?;
        match value.as_str() {
            "group" => Ok(DurabilityMode::GroupCommit),
            "every_tick" => Ok(DurabilityMode::EveryTick),
            _ => Err(PersistenceError::HistoryCatalogInvariant {
                invariant: "unsupported durability mode",
            }),
        }
    }

    fn insert_active_segment(
        &mut self,
        sequence: u64,
        relative_path: &Path,
    ) -> Result<(), PersistenceError> {
        validate_active_segment_path(sequence, relative_path)?;
        self.connection
            .execute(
                "INSERT INTO segments(sequence,relative_path,sealed,durable_end,identity) VALUES (?1,?2,0,0,NULL)",
                params![to_i64(sequence)?, path_text(relative_path)?],
            )
            .map_err(|error| sqlite_error("insert active segment", error))?;
        Ok(())
    }

    fn active_segment(&self) -> Result<(u64, PathBuf, u64), PersistenceError> {
        let (sequence, relative_path, durable_end) = self.raw_active_segment()?;
        let expected = active_relative_path(sequence);
        if relative_path != expected {
            return Err(PersistenceError::HistoryCatalogPathUnsafe {
                path: relative_path.display().to_string(),
            });
        }
        let path = resolve_catalog_path(&self.root, &relative_path, "segments", "cfs", true)?;
        let file_length = fs::metadata(&path)
            .map_err(|error| io_error("read active segment metadata", &path, error))?
            .len();
        if durable_end > file_length {
            return Err(io_error(
                "read active segment",
                &path,
                std::io::Error::from(std::io::ErrorKind::UnexpectedEof),
            ));
        }
        Ok((sequence, relative_path, durable_end))
    }

    fn raw_active_segment(&self) -> Result<(u64, PathBuf, u64), PersistenceError> {
        let count: i64 = self
            .connection
            .query_row("SELECT COUNT(*) FROM segments WHERE sealed=0", [], |row| {
                row.get(0)
            })
            .map_err(|error| sqlite_error("count active segments", error))?;
        if count != 1 {
            return Err(PersistenceError::HistoryCatalogInvariant {
                invariant: "catalog must contain exactly one active segment",
            });
        }
        self.connection
            .query_row(
                "SELECT sequence,relative_path,durable_end FROM segments WHERE sealed=0 ORDER BY sequence DESC LIMIT 1",
                [],
                |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?, row.get::<_, i64>(2)?)),
            )
            .map_err(|error| sqlite_error("read active segment", error))
            .and_then(|(sequence, path, end)| Ok((from_i64(sequence)?, PathBuf::from(path), from_i64(end)?)))
    }

    fn next_frame_ordinal(&self) -> Result<u64, PersistenceError> {
        let value: i64 = self
            .connection
            .query_row(
                "SELECT COALESCE(MAX(frame_ordinal)+1,0) FROM frames",
                [],
                |row| row.get(0),
            )
            .map_err(|error| sqlite_error("read next frame ordinal", error))?;
        from_i64(value)
    }

    fn publish(
        &mut self,
        predecessor: u64,
        generation: u64,
        logical_digest: [u8; 32],
        frames: &[PendingFrame],
        registry: &EvidenceDecoderRegistry,
        hooks: &dyn HistoryStoreHooks,
    ) -> Result<(), PersistenceError> {
        let root = self.root.clone();
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| sqlite_error("begin durable generation", error))?;
        let current: i64 = tx
            .query_row("SELECT MAX(id) FROM generations", [], |row| row.get(0))
            .map_err(|error| sqlite_error("read predecessor generation", error))?;
        if from_i64(current)? != predecessor {
            return Err(PersistenceError::HistoryCatalogInvariant {
                invariant: "durable predecessor changed",
            });
        }
        let mut verified = Vec::with_capacity(frames.len());
        for frame in frames {
            let path = resolve_segment_path(
                &root,
                frame.segment_sequence,
                false,
                None,
                &frame.relative_path,
                true,
            )?;
            let bytes = read_exact_range(&path, frame.range.clone(), MAX_STORED_FRAME_BYTES)?;
            let decoded = decode_history_frame(&bytes, registry)?;
            if decoded.header().frame_digest != frame.frame_digest
                || decoded.records() != frame.records
                || decoded.header().fields.catalog_generation != frame.generation
            {
                return Err(PersistenceError::HistoryCatalogInvariant {
                    invariant: "catalog publication does not match referenced frame",
                });
            }
            verified.push(decoded);
        }
        tx.execute(
            "INSERT INTO generations(id,predecessor,logical_digest,checkpoint_generation)
             SELECT ?1,?2,?3,checkpoint_generation FROM generations WHERE id=?2",
            params![
                to_i64(generation)?,
                to_i64(predecessor)?,
                logical_digest.as_slice()
            ],
        )
        .map_err(|error| sqlite_error("insert durable generation", error))?;
        for (frame, decoded) in frames.iter().zip(&verified) {
            insert_frame_and_indexes(&tx, generation, frame, decoded.records())?;
        }
        let durable_end = frames
            .iter()
            .filter(|frame| {
                frame.segment_sequence == frames.last().map_or(0, |last| last.segment_sequence)
            })
            .map(|frame| frame.range.end)
            .max()
            .ok_or(PersistenceError::HistoryCatalogInvariant {
                invariant: "durable publication has no frames",
            })?;
        tx.execute(
            "UPDATE segments SET durable_end=?1 WHERE sequence=?2",
            params![
                to_i64(durable_end)?,
                to_i64(
                    frames
                        .last()
                        .ok_or(PersistenceError::HistoryCatalogInvariant {
                            invariant: "durable publication has no final frame"
                        })?
                        .segment_sequence
                )?
            ],
        )
        .map_err(|error| sqlite_error("publish active segment end", error))?;
        if hooks.should_fail(FailurePoint::DuringCatalogCommit) {
            return Err(PersistenceError::InjectedHistoryFailure {
                point: FailurePoint::DuringCatalogCommit,
            });
        }
        tx.commit()
            .map_err(|error| sqlite_error("commit durable generation", error))?;
        Ok(())
    }

    fn advance_logical_generation(
        &mut self,
        predecessor: u64,
        target_generation: u64,
        logical_digest: [u8; 32],
        hooks: &dyn HistoryStoreHooks,
    ) -> Result<(), PersistenceError> {
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| sqlite_error("begin logical generation advance", error))?;
        let current: i64 = tx
            .query_row("SELECT MAX(id) FROM generations", [], |row| row.get(0))
            .map_err(|error| sqlite_error("read logical generation predecessor", error))?;
        if from_i64(current)? != predecessor {
            return Err(PersistenceError::HistoryCatalogInvariant {
                invariant: "logical generation predecessor changed",
            });
        }
        let mut current_generation = predecessor;
        while current_generation < target_generation {
            let generation = current_generation
                .checked_add(1)
                .ok_or(PersistenceError::HistoryArithmeticOverflow)?;
            let inserted = tx
                .execute(
                    "INSERT INTO generations(id,predecessor,logical_digest,checkpoint_generation)
                     SELECT ?1,?2,?3,checkpoint_generation FROM generations WHERE id=?2",
                    params![
                        to_i64(generation)?,
                        to_i64(current_generation)?,
                        logical_digest.as_slice()
                    ],
                )
                .map_err(|error| sqlite_error("insert logical generation", error))?;
            if inserted != 1 {
                return Err(PersistenceError::HistoryGenerationUnavailable { generation });
            }
            current_generation = generation;
        }
        if hooks.should_fail(FailurePoint::DuringCatalogCommit) {
            return Err(PersistenceError::InjectedHistoryFailure {
                point: FailurePoint::DuringCatalogCommit,
            });
        }
        tx.commit()
            .map_err(|error| sqlite_error("commit logical generation advance", error))?;
        hooks.observe(HistoryStoreObservation::CatalogCommit);
        Ok(())
    }

    fn rollback_to_preimage(
        &mut self,
        predecessor: u64,
        active_sequence: u64,
        active_relative_path: &Path,
        active_end: u64,
    ) -> Result<(), PersistenceError> {
        let current_generation = self.durable_generation()?;
        let (current_sequence, current_path, current_end) = self.raw_active_segment()?;
        if current_generation == predecessor
            && current_sequence == active_sequence
            && current_path == active_relative_path
            && current_end == active_end
        {
            return Ok(());
        }
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| sqlite_error("begin append rollback", error))?;
        let predecessor = to_i64(predecessor)?;
        for table in [
            "effect_edges",
            "cause_edges",
            "evidence",
            "events",
            "inputs",
            "frames",
        ] {
            let sql = format!("DELETE FROM {table} WHERE generation>?1");
            tx.execute(&sql, [predecessor])
                .map_err(|error| sqlite_error("rollback append rows", error))?;
        }
        tx.execute("DELETE FROM generations WHERE id>?1", [predecessor])
            .map_err(|error| sqlite_error("rollback append generations", error))?;
        tx.execute(
            "DELETE FROM segments WHERE sequence>?1",
            [to_i64(active_sequence)?],
        )
        .map_err(|error| sqlite_error("rollback append segments", error))?;
        let updated = tx
            .execute(
                "UPDATE segments SET relative_path=?1,sealed=0,durable_end=?2,identity=NULL
                 WHERE sequence=?3",
                params![
                    path_text(active_relative_path)?,
                    to_i64(active_end)?,
                    to_i64(active_sequence)?
                ],
            )
            .map_err(|error| sqlite_error("rollback append active segment", error))?;
        if updated != 1 {
            return Err(PersistenceError::HistoryCatalogInvariant {
                invariant: "rollback could not restore append preimage",
            });
        }
        tx.commit()
            .map_err(|error| sqlite_error("commit append rollback", error))
    }

    fn publish_checkpoint(
        &mut self,
        generation: u64,
        time: SimulationTime,
        digest: [u8; 32],
        relative_path: &Path,
    ) -> Result<(), PersistenceError> {
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| sqlite_error("begin checkpoint publication", error))?;
        tx.execute(
            "INSERT INTO checkpoints(generation,time,digest,relative_path) VALUES (?1,?2,?3,?4)",
            params![
                to_i64(generation)?,
                key(time.raw()),
                digest.as_slice(),
                path_text(relative_path)?
            ],
        )
        .map_err(|error| sqlite_error("insert checkpoint", error))?;
        let updated = tx
            .execute(
                "UPDATE generations SET checkpoint_generation=?1 WHERE id=?1",
                [to_i64(generation)?],
            )
            .map_err(|error| sqlite_error("link checkpoint generation", error))?;
        if updated != 1 {
            return Err(PersistenceError::HistoryGenerationUnavailable { generation });
        }
        tx.commit()
            .map_err(|error| sqlite_error("commit checkpoint publication", error))
    }

    fn rollback_checkpoint(
        &mut self,
        generation: u64,
        digest: [u8; 32],
        relative_path: &Path,
    ) -> Result<(), PersistenceError> {
        let expected_path = PathBuf::from("checkpoints").join(format!("{}.ckp", hex(&digest)));
        if relative_path != expected_path {
            return Err(PersistenceError::HistoryCatalogPathUnsafe {
                path: relative_path.display().to_string(),
            });
        }
        let generation_value = to_i64(generation)?;
        let relative_text = path_text(relative_path)?;
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| sqlite_error("begin checkpoint rollback", error))?;
        let linked: Option<i64> = tx
            .query_row(
                "SELECT checkpoint_generation FROM generations WHERE id=?1",
                [generation_value],
                |row| row.get(0),
            )
            .map_err(|error| sqlite_error("read checkpoint rollback link", error))?;
        let checkpoint = tx
            .query_row(
                "SELECT digest,relative_path FROM checkpoints WHERE generation=?1",
                [generation_value],
                |row| Ok((row.get::<_, Vec<u8>>(0)?, row.get::<_, String>(1)?)),
            )
            .optional()
            .map_err(|error| sqlite_error("read checkpoint rollback row", error))?;
        match (linked, checkpoint) {
            (None, None) => Ok(()),
            (Some(link), Some((stored_digest, stored_path)))
                if link == generation_value
                    && stored_digest.as_slice() == digest.as_slice()
                    && stored_path == relative_text =>
            {
                let unlinked = tx
                    .execute(
                        "UPDATE generations SET checkpoint_generation=NULL WHERE id=?1 AND checkpoint_generation=?1",
                        [generation_value],
                    )
                    .map_err(|error| sqlite_error("unlink rolled-back checkpoint", error))?;
                if unlinked != 1 {
                    return Err(PersistenceError::HistoryCatalogInvariant {
                        invariant: "checkpoint rollback could not unlink generation",
                    });
                }
                let deleted = tx
                    .execute(
                        "DELETE FROM checkpoints WHERE generation=?1 AND digest=?2 AND relative_path=?3",
                        params![
                            generation_value,
                            digest.as_slice(),
                            relative_text
                        ],
                    )
                    .map_err(|error| sqlite_error("delete rolled-back checkpoint", error))?;
                if deleted != 1 {
                    return Err(PersistenceError::HistoryCatalogInvariant {
                        invariant: "checkpoint rollback could not delete row",
                    });
                }
                tx.commit()
                    .map_err(|error| sqlite_error("commit checkpoint rollback", error))
            }
            _ => Err(PersistenceError::HistoryCatalogInvariant {
                invariant: "checkpoint rollback preimage does not match catalog",
            }),
        }
    }

    fn checkpoint_at_generation(
        &self,
        generation: u64,
    ) -> Result<Option<CatalogCheckpoint>, PersistenceError> {
        self.connection
            .query_row(
                "SELECT generation,time,digest,relative_path FROM checkpoints WHERE generation=?1",
                [to_i64(generation)?],
                checkpoint_from_row,
            )
            .optional()
            .map_err(|error| sqlite_error("read checkpoint generation", error))
            .and_then(parse_checkpoint_row)
    }

    fn read_checkpoint(
        &self,
        checkpoint: &CatalogCheckpoint,
        namespace: [u8; 32],
        registry: &EvidenceDecoderRegistry,
    ) -> Result<CheckpointEnvelope, PersistenceError> {
        validate_checkpoint_path(checkpoint)?;
        let path = resolve_catalog_path(
            &self.root,
            &checkpoint.relative_path,
            "checkpoints",
            "ckp",
            true,
        )?;
        let bytes =
            fs::read(&path).map_err(|error| io_error("read checkpoint file", &path, error))?;
        let envelope = decode_checkpoint_with_generations(
            &bytes,
            checkpoint.generation,
            checkpoint.generation,
        )?;
        if envelope.digest != checkpoint.digest {
            return Err(PersistenceError::CheckpointDigestMismatch);
        }
        if envelope.header.namespace != namespace[..16] {
            return Err(PersistenceError::HistoryCatalogInvariant {
                invariant: "checkpoint namespace does not match catalog",
            });
        }
        let logical_digest: Vec<u8> = self
            .connection
            .query_row(
                "SELECT logical_digest FROM generations WHERE id=?1",
                [to_i64(checkpoint.generation)?],
                |row| row.get(0),
            )
            .map_err(|error| sqlite_error("read checkpoint generation digest", error))?;
        if envelope.header.logical_digest.as_slice() != logical_digest.as_slice() {
            return Err(PersistenceError::LogicalDigestMismatch);
        }
        if envelope.header.storage_root.as_slice() != logical_digest.as_slice() {
            return Err(PersistenceError::HistoryCatalogInvariant {
                invariant: "checkpoint storage root does not match catalog generation",
            });
        }
        let evidence_digest =
            self.evidence_manifest_digest_for_generation(checkpoint.generation, registry)?;
        if envelope.header.evidence_manifest_digest != evidence_digest {
            return Err(PersistenceError::HistoryCatalogInvariant {
                invariant: "checkpoint evidence manifest does not match durable history",
            });
        }
        Ok(envelope)
    }

    fn evidence_manifest_digest_for_generation(
        &self,
        generation: u64,
        registry: &EvidenceDecoderRegistry,
    ) -> Result<[u8; 32], PersistenceError> {
        let mut digest = EvidenceManifestDigest::new();
        self.for_each_record(generation, registry, |record| digest.append(record))?;
        Ok(digest.finish())
    }

    fn for_each_record(
        &self,
        generation: u64,
        registry: &EvidenceDecoderRegistry,
        mut callback: impl FnMut(&HistoryRecord) -> Result<(), PersistenceError>,
    ) -> Result<(), PersistenceError> {
        self.for_each_decoded_frame(generation, registry, |_, records| {
            for record in records {
                callback(record)?;
            }
            Ok(())
        })
    }

    fn verify_checkpoints(
        &self,
        namespace: [u8; 32],
        durable_generation: u64,
        registry: &EvidenceDecoderRegistry,
    ) -> Result<(), PersistenceError> {
        let stale_links: i64 = self
            .connection
            .query_row(
                "SELECT COUNT(*) FROM generations g
                 WHERE g.id<=?1 AND g.checkpoint_generation IS NOT
                       (SELECT MAX(c.generation) FROM checkpoints c WHERE c.generation<=g.id)",
                [to_i64(durable_generation)?],
                |row| row.get(0),
            )
            .map_err(|error| sqlite_error("verify checkpoint generation links", error))?;
        if stale_links != 0 {
            return Err(PersistenceError::HistoryCatalogInvariant {
                invariant: "durable generation checkpoint link is stale",
            });
        }
        let mut statement = self
            .connection
            .prepare(
                "SELECT generation,time,digest,relative_path FROM checkpoints WHERE generation<=?1 ORDER BY generation",
            )
            .map_err(|error| sqlite_error("prepare checkpoint verification", error))?;
        let rows = statement
            .query_map([to_i64(durable_generation)?], checkpoint_from_row)
            .map_err(|error| sqlite_error("query checkpoints", error))?;
        for row in rows {
            let checkpoint = parse_checkpoint_row(Some(
                row.map_err(|error| sqlite_error("read checkpoint row", error))?,
            ))?
            .ok_or(PersistenceError::HistoryCatalogInvariant {
                invariant: "checkpoint query returned no row",
            })?;
            self.read_checkpoint(&checkpoint, namespace, registry)?;
        }
        Ok(())
    }

    fn verify_sealed_segments(&self, namespace: [u8; 32]) -> Result<(), PersistenceError> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT sequence,relative_path,durable_end,identity FROM segments WHERE sealed=1 ORDER BY sequence",
            )
            .map_err(|error| sqlite_error("prepare sealed segment verification", error))?;
        let rows = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, Vec<u8>>(3)?,
                ))
            })
            .map_err(|error| sqlite_error("query sealed segments", error))?;
        for row in rows {
            let (sequence, relative, durable_end, identity) =
                row.map_err(|error| sqlite_error("read sealed segment", error))?;
            let sequence = from_i64(sequence)?;
            let durable_end = from_i64(durable_end)?;
            let expected: [u8; 32] =
                identity
                    .try_into()
                    .map_err(|_| PersistenceError::HistoryCatalogInvariant {
                        invariant: "sealed segment identity width",
                    })?;
            let relative = PathBuf::from(relative);
            let path =
                resolve_segment_path(&self.root, sequence, true, Some(expected), &relative, true)?;
            let file_length = fs::metadata(&path)
                .map_err(|error| io_error("read sealed segment metadata", &path, error))?
                .len();
            if file_length != durable_end {
                return Err(PersistenceError::SegmentDigestMismatch);
            }
            let bytes = read_exact_range(&path, 0..durable_end, max_stored_segment_bytes()?)?;
            if sealed_segment_identity(&segment_identity_header(namespace, sequence), &[&bytes])
                != expected
            {
                return Err(PersistenceError::SegmentDigestMismatch);
            }
        }
        Ok(())
    }

    fn has_generation(&self, generation: u64) -> Result<bool, PersistenceError> {
        self.connection
            .query_row(
                "SELECT 1 FROM generations WHERE id=?1",
                [to_i64(generation)?],
                |_| Ok(()),
            )
            .optional()
            .map(|value| value.is_some())
            .map_err(|error| sqlite_error("find durable generation", error))
    }

    fn existing_ids(
        &self,
        column: &'static str,
        ids: &BTreeSet<u64>,
    ) -> Result<BTreeSet<u64>, PersistenceError> {
        let encoded = ids.iter().copied().map(key).collect::<Vec<_>>();
        let mut existing = BTreeSet::new();
        for batch in encoded.chunks(CATALOG_ID_QUERY_BATCH) {
            #[cfg(test)]
            CAUSAL_VALIDATION_CATALOG_LOOKUPS.fetch_add(1, Ordering::SeqCst);
            let placeholders = std::iter::repeat_n("?", batch.len())
                .collect::<Vec<_>>()
                .join(",");
            let sql = format!("SELECT {column} FROM events WHERE {column} IN ({placeholders})");
            let mut statement = self
                .connection
                .prepare(&sql)
                .map_err(|error| sqlite_error("prepare causal id batch", error))?;
            let rows = statement
                .query_map(
                    params_from_iter(batch.iter().map(|value| value.as_slice())),
                    |row| row.get::<_, Vec<u8>>(0),
                )
                .map_err(|error| sqlite_error("query causal id batch", error))?;
            for row in rows {
                let bytes: [u8; 8] = row
                    .map_err(|error| sqlite_error("read causal id batch", error))?
                    .try_into()
                    .map_err(|_| PersistenceError::HistoryCatalogInvariant {
                        invariant: "catalog causal id width",
                    })?;
                existing.insert(u64::from_be_bytes(bytes));
            }
        }
        Ok(existing)
    }

    fn generation_digests(
        &self,
        generation: u64,
        registry: &EvidenceDecoderRegistry,
    ) -> Result<([u8; 32], [u8; 32]), PersistenceError> {
        let logical: Vec<u8> = self
            .connection
            .query_row(
                "SELECT logical_digest FROM generations WHERE id=?1",
                [to_i64(generation)?],
                |row| row.get(0),
            )
            .map_err(|error| sqlite_error("read generation digest", error))?;
        let logical =
            logical
                .try_into()
                .map_err(|_| PersistenceError::HistoryCatalogInvariant {
                    invariant: "generation digest width",
                })?;
        let evidence = self.evidence_manifest_digest_for_generation(generation, registry)?;
        Ok((logical, evidence))
    }

    fn frontier(&self, generation: u64) -> Result<HistoryFrontier, PersistenceError> {
        let row = self
            .connection
            .query_row(
                "SELECT event_id,trace_id,time FROM events WHERE generation<=?1 ORDER BY trace_id DESC LIMIT 1",
                [to_i64(generation)?],
                |row| {
                    Ok((
                        row.get::<_, Vec<u8>>(0)?,
                        row.get::<_, Vec<u8>>(1)?,
                        row.get::<_, Vec<u8>>(2)?,
                    ))
                },
            )
            .optional()
            .map_err(|error| sqlite_error("read history frontier", error))?;
        let (event_id, trace_id, time): (u64, u64, u64) = row.map_or(
            Ok::<(u64, u64, u64), PersistenceError>((0_u64, 0_u64, 0_u64)),
            |(event_id, trace_id, time)| {
                Ok((
                    u64::from_be_bytes(event_id.try_into().map_err(|_| {
                        PersistenceError::HistoryCatalogInvariant {
                            invariant: "event id width",
                        }
                    })?),
                    u64::from_be_bytes(trace_id.try_into().map_err(|_| {
                        PersistenceError::HistoryCatalogInvariant {
                            invariant: "trace id width",
                        }
                    })?),
                    u64::from_be_bytes(time.try_into().map_err(|_| {
                        PersistenceError::HistoryCatalogInvariant {
                            invariant: "event time width",
                        }
                    })?),
                ))
            },
        )?;
        Ok(HistoryFrontier::new(
            generation,
            EventId::new(event_id),
            TraceId::new(trace_id),
            SimulationTime::new(time),
        ))
    }

    fn seal_segment(
        &mut self,
        sequence: u64,
        sealed_relative: &Path,
        identity: [u8; 32],
        new_sequence: u64,
        new_relative: &Path,
    ) -> Result<(), PersistenceError> {
        validate_sealed_segment_path(identity, sealed_relative)?;
        validate_active_segment_path(new_sequence, new_relative)?;
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| sqlite_error("begin segment seal", error))?;
        let updated = tx
            .execute(
            "UPDATE segments SET relative_path=?1,sealed=1,identity=?2 WHERE sequence=?3 AND sealed=0",
            params![path_text(sealed_relative)?, identity.as_slice(), to_i64(sequence)?],
            )
            .map_err(|error| sqlite_error("publish sealed segment", error))?;
        if updated != 1 {
            return Err(PersistenceError::HistoryCatalogInvariant {
                invariant: "seal could not publish the active segment",
            });
        }
        tx.execute(
            "INSERT INTO segments(sequence,relative_path,sealed,durable_end,identity) VALUES (?1,?2,0,0,NULL)",
            params![to_i64(new_sequence)?, path_text(new_relative)?],
        )
        .map_err(|error| sqlite_error("insert next active segment", error))?;
        tx.commit()
            .map_err(|error| sqlite_error("commit segment seal", error))
    }

    fn rollback_seal(
        &mut self,
        sequence: u64,
        active_relative_path: &Path,
        sealed_relative_path: &Path,
        identity: [u8; 32],
        new_sequence: u64,
        new_relative_path: &Path,
    ) -> Result<(), PersistenceError> {
        validate_active_segment_path(sequence, active_relative_path)?;
        validate_sealed_segment_path(identity, sealed_relative_path)?;
        validate_active_segment_path(new_sequence, new_relative_path)?;
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| sqlite_error("begin segment seal rollback", error))?;
        let old = tx
            .query_row(
                "SELECT relative_path,sealed,identity FROM segments WHERE sequence=?1",
                [to_i64(sequence)?],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, i64>(1)?,
                        row.get::<_, Option<Vec<u8>>>(2)?,
                    ))
                },
            )
            .optional()
            .map_err(|error| sqlite_error("read segment seal rollback preimage", error))?;
        let next = tx
            .query_row(
                "SELECT relative_path,sealed,durable_end,identity FROM segments WHERE sequence=?1",
                [to_i64(new_sequence)?],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, i64>(1)?,
                        row.get::<_, i64>(2)?,
                        row.get::<_, Option<Vec<u8>>>(3)?,
                    ))
                },
            )
            .optional()
            .map_err(|error| sqlite_error("read next segment seal rollback row", error))?;
        let active_path = path_text(active_relative_path)?;
        let sealed_path = path_text(sealed_relative_path)?;
        let new_path = path_text(new_relative_path)?;
        match (old, next) {
            (Some((path, sealed, stored_identity)), None)
                if path == active_path && sealed == 0 && stored_identity.is_none() =>
            {
                Ok(())
            }
            (
                Some((path, sealed, stored_identity)),
                Some((next_path, next_sealed, next_end, next_identity)),
            ) if path == sealed_path
                && sealed == 1
                && stored_identity.as_deref() == Some(identity.as_slice())
                && next_path == new_path
                && next_sealed == 0
                && next_end == 0
                && next_identity.is_none() =>
            {
                let deleted = tx
                    .execute(
                        "DELETE FROM segments WHERE sequence=?1 AND relative_path=?2 AND sealed=0 AND durable_end=0 AND identity IS NULL",
                        params![to_i64(new_sequence)?, new_path],
                    )
                    .map_err(|error| sqlite_error("delete rolled-back active segment", error))?;
                if deleted != 1 {
                    return Err(PersistenceError::HistoryCatalogInvariant {
                        invariant: "seal rollback could not delete the new active segment",
                    });
                }
                let restored = tx
                    .execute(
                        "UPDATE segments SET relative_path=?1,sealed=0,identity=NULL WHERE sequence=?2 AND relative_path=?3 AND sealed=1 AND identity=?4",
                        params![
                            active_path,
                            to_i64(sequence)?,
                            sealed_path,
                            identity.as_slice()
                        ],
                    )
                    .map_err(|error| sqlite_error("restore rolled-back active segment", error))?;
                if restored != 1 {
                    return Err(PersistenceError::HistoryCatalogInvariant {
                        invariant: "seal rollback could not restore the old active segment",
                    });
                }
                tx.commit()
                    .map_err(|error| sqlite_error("commit segment seal rollback", error))
            }
            _ => Err(PersistenceError::HistoryCatalogInvariant {
                invariant: "segment seal rollback preimage does not match catalog",
            }),
        }
    }

    fn rebuild_indexes(
        &mut self,
        registry: &EvidenceDecoderRegistry,
        hooks: &dyn HistoryStoreHooks,
    ) -> Result<(), PersistenceError> {
        let root = self.root.clone();
        let ranges = self.frame_ranges(self.durable_generation()?)?;
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| sqlite_error("begin index rebuild", error))?;
        tx.execute_batch("DELETE FROM cause_edges; DELETE FROM effect_edges; DELETE FROM evidence; DELETE FROM events;")
            .map_err(|error| sqlite_error("delete derived indexes", error))?;
        if hooks.should_fail(FailurePoint::IndexDeletion) {
            return Err(PersistenceError::InjectedHistoryFailure {
                point: FailurePoint::IndexDeletion,
            });
        }
        for range in ranges {
            let path = resolve_catalog_path(&root, &range.relative_path, "segments", "cfs", true)?;
            let bytes = read_exact_range(&path, range.range.clone(), MAX_STORED_FRAME_BYTES)?;
            let decoded = decode_history_frame(&bytes, registry)?;
            insert_indexes(
                &tx,
                range.generation,
                range.frame_ordinal,
                decoded.records(),
            )?;
        }
        tx.commit()
            .map_err(|error| sqlite_error("commit index rebuild", error))
    }

    fn rebuild_indexes_if_incomplete(
        &mut self,
        registry: &EvidenceDecoderRegistry,
        hooks: &dyn HistoryStoreHooks,
    ) -> Result<(), PersistenceError> {
        let expected = self.expected_index_counts(self.durable_generation()?, registry)?;
        let actual = (
            count(&self.connection, "events")?,
            count(&self.connection, "cause_edges")?,
            count(&self.connection, "effect_edges")?,
            count(&self.connection, "evidence")?,
        );
        let generation = self.durable_generation()?;
        let indexes_match = match self.indexes_match_canonical(generation, registry) {
            Ok(value) => value,
            Err(PersistenceError::HistoryCatalogInvariant {
                invariant: "derived catalog index rows do not match canonical frames",
            }) => false,
            Err(error) => return Err(error),
        };
        if actual != expected || !indexes_match {
            self.rebuild_indexes(registry, hooks)?;
        }
        Ok(())
    }

    fn expected_index_counts(
        &self,
        generation: u64,
        registry: &EvidenceDecoderRegistry,
    ) -> Result<(u64, u64, u64, u64), PersistenceError> {
        let mut total = (0_u64, 0_u64, 0_u64, 0_u64);
        self.for_each_decoded_frame(generation, registry, |_, records| {
            let frame = records.iter().try_fold(
                (0_u64, 0_u64, 0_u64, 0_u64),
                |(events, causes, effects, evidence), record| -> Result<_, PersistenceError> {
                    match record {
                        HistoryRecord::Causal(event) => Ok((
                            events
                                .checked_add(1)
                                .ok_or(PersistenceError::HistoryArithmeticOverflow)?,
                            causes
                                .checked_add(u64::try_from(event.causes().len())?)
                                .ok_or(PersistenceError::HistoryArithmeticOverflow)?,
                            effects
                                .checked_add(u64::try_from(event.effects().len())?)
                                .ok_or(PersistenceError::HistoryArithmeticOverflow)?,
                            evidence,
                        )),
                        HistoryRecord::Evidence(_) => Ok((
                            events,
                            causes,
                            effects,
                            evidence
                                .checked_add(1)
                                .ok_or(PersistenceError::HistoryArithmeticOverflow)?,
                        )),
                    }
                },
            )?;
            total.0 = total
                .0
                .checked_add(frame.0)
                .ok_or(PersistenceError::HistoryArithmeticOverflow)?;
            total.1 = total
                .1
                .checked_add(frame.1)
                .ok_or(PersistenceError::HistoryArithmeticOverflow)?;
            total.2 = total
                .2
                .checked_add(frame.2)
                .ok_or(PersistenceError::HistoryArithmeticOverflow)?;
            total.3 = total
                .3
                .checked_add(frame.3)
                .ok_or(PersistenceError::HistoryArithmeticOverflow)?;
            Ok(())
        })?;
        Ok(total)
    }

    fn indexes_match_canonical(
        &self,
        generation: u64,
        registry: &EvidenceDecoderRegistry,
    ) -> Result<bool, PersistenceError> {
        crate::history_store_reader::validate_catalog_indexes(
            &self.connection,
            &self.root,
            registry,
            self.namespace()?,
            generation,
        )
        .map(|()| true)
    }

    fn for_each_decoded_frame(
        &self,
        generation: u64,
        registry: &EvidenceDecoderRegistry,
        mut callback: impl FnMut(u64, &[HistoryRecord]) -> Result<(), PersistenceError>,
    ) -> Result<(), PersistenceError> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT f.generation,f.frame_ordinal,f.segment_sequence,s.relative_path,
                        s.sealed,s.identity,f.start_offset,f.end_offset,f.frame_digest
                 FROM frames f JOIN segments s ON s.sequence=f.segment_sequence
                 WHERE f.generation<=?1 ORDER BY f.frame_ordinal",
            )
            .map_err(|error| sqlite_error("prepare streamed frame query", error))?;
        let rows = statement
            .query_map([to_i64(generation)?], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, i64>(4)?,
                    row.get::<_, Option<Vec<u8>>>(5)?,
                    row.get::<_, i64>(6)?,
                    row.get::<_, i64>(7)?,
                    row.get::<_, Vec<u8>>(8)?,
                ))
            })
            .map_err(|error| sqlite_error("query streamed frames", error))?;
        for row in rows {
            let (_row_generation, ordinal, segment, path, sealed, identity, start, end, digest) =
                row.map_err(|error| sqlite_error("read streamed frame", error))?;
            if sealed != 0 && sealed != 1 {
                return Err(PersistenceError::HistoryCatalogInvariant {
                    invariant: "segment sealed flag is invalid",
                });
            }
            let start = from_i64(start)?;
            let end = from_i64(end)?;
            validate_frame_range(start..end, MAX_STORED_FRAME_BYTES)?;
            let segment = from_i64(segment)?;
            let identity = identity
                .map(|bytes| {
                    bytes
                        .try_into()
                        .map_err(|_| PersistenceError::HistoryCatalogInvariant {
                            invariant: "segment identity width",
                        })
                })
                .transpose()?;
            let path = resolve_segment_path(
                &self.root,
                segment,
                sealed != 0,
                identity,
                Path::new(&path),
                true,
            )?;
            let digest: [u8; 32] =
                digest
                    .try_into()
                    .map_err(|_| PersistenceError::HistoryCatalogInvariant {
                        invariant: "frame digest width",
                    })?;
            let bytes = read_exact_range(&path, start..end, MAX_STORED_FRAME_BYTES)?;
            let decoded = decode_history_frame(&bytes, registry)?;
            if decoded.header().fields.namespace != self.namespace()?
                || decoded.header().frame_digest != digest
            {
                return Err(PersistenceError::HistoryCatalogInvariant {
                    invariant: "catalog frame does not match canonical frame",
                });
            }
            callback(from_i64(ordinal)?, decoded.records())?;
        }
        Ok(())
    }
}

impl HistoryCatalog for SqliteHistoryCatalog {
    fn durable_generation(&self) -> Result<u64, PersistenceError> {
        let value: i64 = self
            .connection
            .query_row("SELECT MAX(id) FROM generations", [], |row| row.get(0))
            .map_err(|error| sqlite_error("read durable generation", error))?;
        from_i64(value)
    }

    fn frame_ranges(&self, generation: u64) -> Result<Vec<CatalogFrameRange>, PersistenceError> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT f.generation,f.frame_ordinal,f.segment_sequence,s.relative_path,s.sealed,s.identity,f.start_offset,f.end_offset,f.frame_digest
                 FROM frames f JOIN segments s ON s.sequence=f.segment_sequence
                 WHERE f.generation<=?1 ORDER BY f.frame_ordinal",
            )
            .map_err(|error| sqlite_error("prepare frame range query", error))?;
        let rows = statement
            .query_map([to_i64(generation)?], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, i64>(4)?,
                    row.get::<_, Option<Vec<u8>>>(5)?,
                    row.get::<_, i64>(6)?,
                    row.get::<_, i64>(7)?,
                    row.get::<_, Vec<u8>>(8)?,
                ))
            })
            .map_err(|error| sqlite_error("query frame ranges", error))?;
        let mut ranges = Vec::new();
        for row in rows {
            let (row_generation, ordinal, segment, path, sealed, identity, start, end, digest) =
                row.map_err(|error| sqlite_error("read frame range", error))?;
            if sealed != 0 && sealed != 1 {
                return Err(PersistenceError::HistoryCatalogInvariant {
                    invariant: "segment sealed flag is invalid",
                });
            }
            let start = from_i64(start)?;
            let end = from_i64(end)?;
            validate_frame_range(start..end, MAX_STORED_FRAME_BYTES)?;
            let segment = from_i64(segment)?;
            let relative_path = PathBuf::from(path);
            let identity = identity
                .map(|value| {
                    value
                        .try_into()
                        .map_err(|_| PersistenceError::HistoryCatalogInvariant {
                            invariant: "segment identity width",
                        })
                })
                .transpose()?;
            if sealed != 0 {
                let identity = identity.ok_or(PersistenceError::HistoryCatalogInvariant {
                    invariant: "sealed segment identity is missing",
                })?;
                resolve_segment_path(
                    &self.root,
                    segment,
                    true,
                    Some(identity),
                    &relative_path,
                    true,
                )?;
            } else {
                if identity.is_some() {
                    return Err(PersistenceError::HistoryCatalogInvariant {
                        invariant: "active segment has a sealed identity",
                    });
                }
                resolve_segment_path(&self.root, segment, false, None, &relative_path, true)?;
            }
            ranges.push(CatalogFrameRange {
                generation: from_i64(row_generation)?,
                frame_ordinal: from_i64(ordinal)?,
                segment_sequence: segment,
                relative_path,
                range: start..end,
                frame_digest: digest.try_into().map_err(|_| {
                    PersistenceError::HistoryCatalogInvariant {
                        invariant: "frame digest width",
                    }
                })?,
            });
        }
        Ok(ranges)
    }

    fn counts(&self) -> Result<CatalogCounts, PersistenceError> {
        Ok(CatalogCounts {
            generations: count(&self.connection, "generations")?,
            segments: count(&self.connection, "segments")?,
            frames: count(&self.connection, "frames")?,
            events: count(&self.connection, "events")?,
            cause_edges: count(&self.connection, "cause_edges")?,
            effect_edges: count(&self.connection, "effect_edges")?,
            evidence: count(&self.connection, "evidence")?,
            inputs: count(&self.connection, "inputs")?,
            checkpoints: count(&self.connection, "checkpoints")?,
            snapshots: count(&self.connection, "snapshots")?,
        })
    }

    fn settings(&self) -> Result<CatalogSettings, PersistenceError> {
        let journal_mode: String = self
            .connection
            .query_row("PRAGMA journal_mode", [], |row| row.get(0))
            .map_err(|error| sqlite_error("read catalog journal mode", error))?;
        let synchronous: u32 = self
            .connection
            .query_row("PRAGMA synchronous", [], |row| row.get(0))
            .map_err(|error| sqlite_error("read catalog synchronous mode", error))?;
        let foreign_keys: u32 = self
            .connection
            .query_row("PRAGMA foreign_keys", [], |row| row.get(0))
            .map_err(|error| sqlite_error("read catalog foreign-key mode", error))?;
        Ok(CatalogSettings {
            journal_mode,
            synchronous,
            foreign_keys: foreign_keys == 1,
        })
    }

    fn checkpoint_for_generation(
        &self,
        generation: u64,
    ) -> Result<Option<CatalogCheckpoint>, PersistenceError> {
        self.connection
            .query_row(
                "SELECT c.generation,c.time,c.digest,c.relative_path
                 FROM generations g
                 JOIN checkpoints c ON c.generation=g.checkpoint_generation
                 WHERE g.id=?1",
                [to_i64(generation)?],
                checkpoint_from_row,
            )
            .optional()
            .map_err(|error| sqlite_error("read linked checkpoint", error))
            .and_then(parse_checkpoint_row)
    }
}

fn insert_frame_and_indexes(
    tx: &Transaction<'_>,
    generation: u64,
    frame: &PendingFrame,
    records: &[HistoryRecord],
) -> Result<(), PersistenceError> {
    tx.execute(
        "INSERT INTO frames(generation,frame_ordinal,segment_sequence,start_offset,end_offset,frame_digest,record_count) VALUES (?1,?2,?3,?4,?5,?6,?7)",
        params![to_i64(generation)?, to_i64(frame.frame_ordinal)?, to_i64(frame.segment_sequence)?, to_i64(frame.range.start)?, to_i64(frame.range.end)?, frame.frame_digest.as_slice(), to_i64(u64::try_from(records.len())?)?],
    )
    .map_err(|error| sqlite_error("insert frame range", error))?;
    insert_indexes(tx, generation, frame.frame_ordinal, records)
}

fn insert_indexes(
    tx: &Transaction<'_>,
    generation: u64,
    frame_ordinal: u64,
    records: &[HistoryRecord],
) -> Result<(), PersistenceError> {
    // One parse per statement per frame; a hydrology tick inserts tens of
    // thousands of rows and `Transaction::execute` would parse the SQL for each.
    let mut insert_event = tx
        .prepare(
            "INSERT INTO events(generation,trace_id,event_id,frame_ordinal,record_ordinal,time) VALUES (?1,?2,?3,?4,?5,?6)",
        )
        .map_err(|error| sqlite_error("prepare event index", error))?;
    let mut insert_cause = tx
        .prepare(
            "INSERT INTO cause_edges(generation,parent_trace,child_trace,ordinal) VALUES (?1,?2,?3,?4)",
        )
        .map_err(|error| sqlite_error("prepare cause edge", error))?;
    let mut insert_effect = tx
        .prepare(
            "INSERT INTO effect_edges(generation,trace_id,ordinal,object_kind,object_id,property_id) VALUES (?1,?2,?3,?4,?5,?6)",
        )
        .map_err(|error| sqlite_error("prepare effect edge", error))?;
    let mut insert_evidence = tx
        .prepare(
            "INSERT INTO evidence(generation,schema_id,canonical_key,frame_ordinal,record_ordinal,payload_digest) VALUES (?1,?2,?3,?4,?5,?6)",
        )
        .map_err(|error| sqlite_error("prepare evidence index", error))?;
    for (record_ordinal, record) in records.iter().enumerate() {
        match record {
            HistoryRecord::Causal(event) => {
                insert_event
                    .execute(params![
                        to_i64(generation)?,
                        key(event.trace_id().raw()),
                        key(event.event_id().raw()),
                        to_i64(frame_ordinal)?,
                        to_i64(u64::try_from(record_ordinal)?)?,
                        key(event.time().raw())
                    ])
                    .map_err(|error| sqlite_error("insert event index", error))?;
                for (ordinal, parent) in event.causes().iter().enumerate() {
                    insert_cause
                        .execute(params![
                            to_i64(generation)?,
                            key(parent.raw()),
                            key(event.trace_id().raw()),
                            to_i64(u64::try_from(ordinal)?)?
                        ])
                        .map_err(|error| sqlite_error("insert cause edge", error))?;
                }
                for (ordinal, effect) in event.effects().iter().enumerate() {
                    let target = effect.target();
                    insert_effect
                        .execute(params![
                            to_i64(generation)?,
                            key(event.trace_id().raw()),
                            to_i64(u64::try_from(ordinal)?)?,
                            key(target.object_kind().raw()),
                            key(target.object_id()),
                            key(target.property().raw())
                        ])
                        .map_err(|error| sqlite_error("insert effect edge", error))?;
                }
            }
            HistoryRecord::Evidence(evidence) => {
                insert_evidence
                    .execute(params![
                        to_i64(generation)?,
                        key(evidence.schema_id().raw()),
                        evidence.canonical_key(),
                        to_i64(frame_ordinal)?,
                        to_i64(u64::try_from(record_ordinal)?)?,
                        evidence.payload_digest().bytes().as_slice()
                    ])
                    .map_err(|error| sqlite_error("insert evidence index", error))?;
            }
        }
    }
    Ok(())
}

fn create_layout(root: &Path) -> Result<(), PersistenceError> {
    fs::create_dir_all(root).map_err(|error| io_error("create bundle root", root, error))?;
    for directory in [
        "segments",
        "staging",
        "checkpoints",
        "snapshots",
        "migration",
    ] {
        let path = root.join(directory);
        fs::create_dir(&path).map_err(|error| io_error("create bundle directory", &path, error))?;
    }
    Ok(())
}

type CheckpointRow = (i64, Vec<u8>, Vec<u8>, String);

fn checkpoint_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<CheckpointRow> {
    Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
}

fn parse_checkpoint_row(
    row: Option<CheckpointRow>,
) -> Result<Option<CatalogCheckpoint>, PersistenceError> {
    row.map(|(generation, time, digest, relative_path)| {
        let time = time
            .try_into()
            .map(u64::from_be_bytes)
            .map(SimulationTime::new)
            .map_err(|_| PersistenceError::HistoryCatalogInvariant {
                invariant: "checkpoint time width",
            })?;
        let digest = digest
            .try_into()
            .map_err(|_| PersistenceError::HistoryCatalogInvariant {
                invariant: "checkpoint digest width",
            })?;
        Ok(CatalogCheckpoint {
            generation: from_i64(generation)?,
            time,
            digest,
            relative_path: PathBuf::from(relative_path),
        })
    })
    .transpose()
}

fn validate_checkpoint_path(checkpoint: &CatalogCheckpoint) -> Result<(), PersistenceError> {
    let expected = PathBuf::from("checkpoints").join(format!("{}.ckp", hex(&checkpoint.digest)));
    if checkpoint.relative_path != expected {
        return Err(PersistenceError::HistoryCatalogInvariant {
            invariant: "checkpoint path is not content-addressed",
        });
    }
    Ok(())
}

fn acquire_writer(root: &Path) -> Result<WriterLease, PersistenceError> {
    let path =
        fs::canonicalize(root).map_err(|error| io_error("resolve bundle root", root, error))?;
    let lock_path = path.join("writer.lock");
    let file = OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(false)
        .open(&lock_path)
        .map_err(|error| io_error("open writer lease", &lock_path, error))?;
    match file.try_lock() {
        Ok(()) => Ok(WriterLease { _file: file }),
        Err(TryLockError::WouldBlock) => Err(PersistenceError::HistoryWriterLocked {
            path: path.display().to_string(),
        }),
        Err(TryLockError::Error(error)) => Err(io_error("acquire writer lease", &lock_path, error)),
    }
}

fn validate_catalog_relative_path(
    relative: &Path,
    directory: &str,
    extension: &str,
) -> Result<(), PersistenceError> {
    let mut components = relative.components();
    let first = components.next();
    let second = components.next();
    if !matches!(
        first,
        Some(std::path::Component::Normal(value)) if value == std::ffi::OsStr::new(directory)
    ) {
        return Err(PersistenceError::HistoryCatalogPathUnsafe {
            path: relative.display().to_string(),
        });
    }
    let Some(std::path::Component::Normal(filename)) = second else {
        return Err(PersistenceError::HistoryCatalogPathUnsafe {
            path: relative.display().to_string(),
        });
    };
    if components.next().is_some()
        || filename.is_empty()
        || Path::new(filename).extension() != Some(std::ffi::OsStr::new(extension))
        || Path::new(filename).file_stem().is_none()
    {
        return Err(PersistenceError::HistoryCatalogPathUnsafe {
            path: relative.display().to_string(),
        });
    }
    Ok(())
}

fn validate_active_segment_path(sequence: u64, relative: &Path) -> Result<(), PersistenceError> {
    if relative != active_relative_path(sequence) {
        return Err(PersistenceError::HistoryCatalogPathUnsafe {
            path: relative.display().to_string(),
        });
    }
    Ok(())
}

fn validate_sealed_segment_path(
    identity: [u8; 32],
    relative: &Path,
) -> Result<(), PersistenceError> {
    let expected = PathBuf::from("segments").join(format!("{}.cfs", hex(&identity)));
    if relative != expected {
        return Err(PersistenceError::HistoryCatalogPathUnsafe {
            path: relative.display().to_string(),
        });
    }
    Ok(())
}

pub(crate) fn resolve_segment_path(
    root: &Path,
    sequence: u64,
    sealed: bool,
    identity: Option<[u8; 32]>,
    relative: &Path,
    require_existing: bool,
) -> Result<PathBuf, PersistenceError> {
    if sealed {
        let identity = identity.ok_or(PersistenceError::HistoryCatalogInvariant {
            invariant: "sealed segment identity is missing",
        })?;
        validate_sealed_segment_path(identity, relative)?;
    } else {
        if identity.is_some() {
            return Err(PersistenceError::HistoryCatalogInvariant {
                invariant: "active segment has a sealed identity",
            });
        }
        validate_active_segment_path(sequence, relative)?;
    }
    resolve_catalog_path(root, relative, "segments", "cfs", require_existing)
}

fn resolve_catalog_path(
    root: &Path,
    relative: &Path,
    directory: &str,
    extension: &str,
    require_existing: bool,
) -> Result<PathBuf, PersistenceError> {
    validate_catalog_relative_path(relative, directory, extension)?;
    let canonical_root =
        fs::canonicalize(root).map_err(|error| io_error("resolve bundle root", root, error))?;
    let candidate = canonical_root.join(relative);
    let parent = candidate
        .parent()
        .ok_or(PersistenceError::HistoryCatalogPathUnsafe {
            path: relative.display().to_string(),
        })?;
    let canonical_parent = fs::canonicalize(parent).map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            PersistenceError::HistoryCatalogPathMissing {
                path: relative.display().to_string(),
            }
        } else {
            io_error("resolve catalog path parent", parent, error)
        }
    })?;
    if !canonical_parent.starts_with(&canonical_root) {
        return Err(PersistenceError::HistoryCatalogPathUnsafe {
            path: relative.display().to_string(),
        });
    }
    match fs::symlink_metadata(&candidate) {
        Ok(metadata) => {
            if metadata.file_type().is_symlink() {
                return Err(PersistenceError::HistoryCatalogPathUnsafe {
                    path: relative.display().to_string(),
                });
            }
            let canonical_candidate = fs::canonicalize(&candidate)
                .map_err(|error| io_error("resolve catalog path", &candidate, error))?;
            if !canonical_candidate.starts_with(&canonical_root) {
                return Err(PersistenceError::HistoryCatalogPathUnsafe {
                    path: relative.display().to_string(),
                });
            }
            if !metadata.is_file() {
                return Err(PersistenceError::HistoryCatalogPathUnsafe {
                    path: relative.display().to_string(),
                });
            }
            Ok(candidate)
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound && !require_existing => {
            Ok(candidate)
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            Err(PersistenceError::HistoryCatalogPathMissing {
                path: relative.display().to_string(),
            })
        }
        Err(error) => Err(io_error("inspect catalog path", &candidate, error)),
    }
}

fn open_existing_active(path: &Path) -> Result<File, PersistenceError> {
    OpenOptions::new()
        .read(true)
        .write(true)
        .truncate(false)
        .open(path)
        .map_err(|error| io_error("open active segment", path, error))
}

fn open_new_active(root: &Path, relative: &Path) -> Result<File, PersistenceError> {
    let path = resolve_catalog_path(root, relative, "segments", "cfs", false)?;
    OpenOptions::new()
        .create_new(true)
        .read(true)
        .write(true)
        .truncate(false)
        .open(&path)
        .map_err(|error| io_error("create active segment", &path, error))
}

fn rollback_seal_files(
    sealed_path: &Path,
    new_path: &Path,
    old_active_path: &Path,
    hooks: &dyn HistoryStoreHooks,
) -> Result<(), PersistenceError> {
    let restore = match fs::symlink_metadata(sealed_path) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_file() => {
            Err(PersistenceError::HistoryCatalogPathUnsafe {
                path: sealed_path.display().to_string(),
            })
        }
        Ok(_) => match fs::symlink_metadata(old_active_path) {
            Ok(_) => Err(PersistenceError::HistoryCatalogInvariant {
                invariant: "seal rollback would overwrite the active segment",
            }),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                fs::rename(sealed_path, old_active_path).map_err(|error| {
                    io_error("restore active segment after seal", old_active_path, error)
                })
            }
            Err(error) => Err(io_error(
                "inspect active segment during seal rollback",
                old_active_path,
                error,
            )),
        },
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(io_error(
            "inspect sealed segment rollback",
            sealed_path,
            error,
        )),
    };
    let new_cleanup = remove_owned_seal_file(new_path);
    let directory_sync = sync_directory(&bundle_directory(sealed_path)?, hooks);
    restore?;
    new_cleanup?;
    directory_sync
}

fn ensure_absent_seal_path(path: &Path) -> Result<(), PersistenceError> {
    match fs::symlink_metadata(path) {
        Ok(_) => Err(PersistenceError::HistoryCatalogInvariant {
            invariant: "seal destination already exists",
        }),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(io_error("inspect seal destination", path, error)),
    }
}

fn remove_owned_seal_file(path: &Path) -> Result<(), PersistenceError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_file() => {
            Err(PersistenceError::HistoryCatalogPathUnsafe {
                path: path.display().to_string(),
            })
        }
        Ok(_) => fs::remove_file(path)
            .map_err(|error| io_error("remove rolled-back seal file", path, error)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(io_error("inspect rolled-back seal file", path, error)),
    }
}

fn bundle_directory(path: &Path) -> Result<PathBuf, PersistenceError> {
    path.parent()
        .map(Path::to_path_buf)
        .ok_or(PersistenceError::HistoryCatalogInvariant {
            invariant: "bundle path has no parent directory",
        })
}

fn remove_checkpoint_file(
    path: &Path,
    hooks: &dyn HistoryStoreHooks,
) -> Result<(), PersistenceError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_file() => {
            Err(PersistenceError::HistoryCatalogPathUnsafe {
                path: path.display().to_string(),
            })
        }
        Ok(_) => {
            fs::remove_file(path)
                .map_err(|error| io_error("remove rolled-back checkpoint file", path, error))?;
            if let Some(parent) = path.parent() {
                sync_directory(parent, hooks)?;
            }
            Ok(())
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(io_error("inspect rolled-back checkpoint file", path, error)),
    }
}

fn sync_directory(path: &Path, hooks: &dyn HistoryStoreHooks) -> Result<(), PersistenceError> {
    File::open(path)
        .and_then(|directory| directory.sync_all())
        .map_err(|error| io_error("sync bundle directory", path, error))?;
    hooks.observe(HistoryStoreObservation::DirectorySync);
    Ok(())
}

fn validate_frame_range(range: Range<u64>, max_bytes: u64) -> Result<u64, PersistenceError> {
    let length = range
        .end
        .checked_sub(range.start)
        .ok_or(PersistenceError::HistoryArithmeticOverflow)?;
    if length == 0 {
        return Err(PersistenceError::HistoryCatalogInvariant {
            invariant: "catalog frame range is empty",
        });
    }
    if length > max_bytes {
        return Err(PersistenceError::HistoryFrameSizeExceeded {
            size: length,
            max: usize::try_from(max_bytes)?,
        });
    }
    Ok(length)
}

fn max_stored_segment_bytes() -> Result<u64, PersistenceError> {
    let group_frame_bytes = u64::try_from(GROUP_MAX_TICKS)
        .map_err(|_| PersistenceError::HistoryArithmeticOverflow)?
        .checked_mul(MAX_STORED_FRAME_BYTES)
        .ok_or(PersistenceError::HistoryArithmeticOverflow)?;
    SEGMENT_TARGET_BYTES
        .checked_add(group_frame_bytes)
        .ok_or(PersistenceError::HistoryArithmeticOverflow)
}

pub(crate) fn read_exact_range(
    path: &Path,
    range: Range<u64>,
    max_bytes: u64,
) -> Result<Vec<u8>, PersistenceError> {
    let length = validate_frame_range(range.clone(), max_bytes)?;
    let max_size = usize::try_from(max_bytes)?;

    let mut file = File::open(path).map_err(|error| io_error("open frame range", path, error))?;
    let file_length = file
        .metadata()
        .map_err(|error| io_error("read frame range metadata", path, error))?
        .len();
    let available = file_length.checked_sub(range.start).ok_or_else(|| {
        io_error(
            "read frame range",
            path,
            std::io::Error::from(std::io::ErrorKind::UnexpectedEof),
        )
    })?;
    if length > available {
        return Err(io_error(
            "read frame range",
            path,
            std::io::Error::from(std::io::ErrorKind::UnexpectedEof),
        ));
    }
    let capacity = usize::try_from(length)?;
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(capacity)
        .map_err(|_| PersistenceError::HistoryFrameSizeExceeded {
            size: length,
            max: max_size,
        })?;
    bytes.resize(capacity, 0);
    file.seek(SeekFrom::Start(range.start))
        .and_then(|_| file.read_exact(&mut bytes))
        .map_err(|error| io_error("read frame range", path, error))?;
    Ok(bytes)
}

fn causal_records(records: &[HistoryRecord]) -> Vec<OwnedCausalRecord> {
    records
        .iter()
        .filter_map(|record| match record {
            HistoryRecord::Causal(event) => Some(event.clone()),
            HistoryRecord::Evidence(_) => None,
        })
        .collect()
}

fn validate_causal_batch_against_catalog(
    catalog: &SqliteHistoryCatalog,
    records: &[OwnedCausalRecord],
    pending: &[PendingFrame],
) -> Result<(), PersistenceError> {
    let event_ids = records
        .iter()
        .map(|record| record.event_id().raw())
        .collect::<BTreeSet<_>>();
    let mut trace_ids = records
        .iter()
        .map(|record| record.trace_id().raw())
        .collect::<BTreeSet<_>>();
    trace_ids.extend(
        records
            .iter()
            .flat_map(|record| record.causes().iter().map(|parent| parent.raw())),
    );
    let catalog_events = catalog.existing_ids("event_id", &event_ids)?;
    let catalog_traces = catalog.existing_ids("trace_id", &trace_ids)?;
    let mut pending_events = BTreeSet::new();
    let mut pending_traces = BTreeSet::new();
    for record in pending.iter().flat_map(|frame| frame.records.iter()) {
        if let HistoryRecord::Causal(record) = record {
            pending_events.insert(record.event_id().raw());
            pending_traces.insert(record.trace_id().raw());
        }
    }
    let mut events = BTreeSet::new();
    let mut traces = BTreeSet::new();
    let mut known_batch_traces = BTreeSet::new();
    for (index, record) in records.iter().enumerate() {
        let duplicate_event = catalog_events.contains(&record.event_id().raw())
            || pending_events.contains(&record.event_id().raw())
            || !events.insert(record.event_id());
        if duplicate_event {
            return Err(HistoryError::DuplicateEventId(record.event_id()).into());
        }
        let duplicate_trace = catalog_traces.contains(&record.trace_id().raw())
            || pending_traces.contains(&record.trace_id().raw())
            || !traces.insert(record.trace_id());
        if duplicate_trace {
            return Err(HistoryError::DuplicateTraceId(record.trace_id()).into());
        }
        if index > 0 && records[index - 1] >= *record {
            return Err(HistoryError::NonCanonicalOrder { index }.into());
        }
        for parent in record.causes() {
            if !catalog_traces.contains(&parent.raw())
                && !pending_traces.contains(&parent.raw())
                && !known_batch_traces.contains(parent)
            {
                return Err(HistoryError::UnknownParent {
                    trace_id: record.trace_id(),
                    parent: *parent,
                }
                .into());
            }
        }
        known_batch_traces.insert(record.trace_id());
    }
    Ok(())
}

fn causal_ranges(records: &[HistoryRecord]) -> (u64, u64, u64, u64) {
    let causal = causal_records(records);
    causal
        .first()
        .zip(causal.last())
        .map_or((0, 0, 0, 0), |(first, last)| {
            (
                first.event_id().raw(),
                last.event_id().raw(),
                first.trace_id().raw(),
                last.trace_id().raw(),
            )
        })
}

fn segment_identity_header(namespace: [u8; 32], sequence: u64) -> Vec<u8> {
    let mut header = Vec::with_capacity(46);
    header.extend_from_slice(SEGMENT_IDENTITY_MAGIC);
    header.extend_from_slice(&1_u16.to_le_bytes());
    header.extend_from_slice(&namespace);
    header.extend_from_slice(&sequence.to_le_bytes());
    header
}

fn active_relative_path(sequence: u64) -> PathBuf {
    PathBuf::from("segments").join(format!("active-{sequence}.cfs"))
}

fn path_text(path: &Path) -> Result<&str, PersistenceError> {
    path.to_str()
        .ok_or(PersistenceError::HistoryCatalogInvariant {
            invariant: "bundle relative path is not UTF-8",
        })
}

fn io_error(operation: &'static str, path: &Path, error: std::io::Error) -> PersistenceError {
    PersistenceError::HistoryIo {
        operation,
        path: path.display().to_string(),
        kind: error.kind(),
    }
}

fn sqlite_error(operation: &'static str, error: rusqlite::Error) -> PersistenceError {
    let code = match &error {
        rusqlite::Error::SqliteFailure(failure, _) => Some(failure.extended_code),
        _ => None,
    };
    PersistenceError::HistoryCatalog {
        operation,
        code,
        message: error.to_string(),
    }
}

fn to_i64(value: u64) -> Result<i64, PersistenceError> {
    i64::try_from(value).map_err(|_| PersistenceError::HistoryArithmeticOverflow)
}

pub(crate) fn from_i64(value: i64) -> Result<u64, PersistenceError> {
    u64::try_from(value).map_err(|_| PersistenceError::HistoryCatalogInvariant {
        invariant: "catalog contains a negative unsigned value",
    })
}

pub(crate) fn key(value: u64) -> [u8; 8] {
    value.to_be_bytes()
}

fn count(connection: &Connection, table: &'static str) -> Result<u64, PersistenceError> {
    let sql = format!("SELECT COUNT(*) FROM {table}");
    let value: i64 = connection
        .query_row(&sql, [], |row| row.get(0))
        .map_err(|error| sqlite_error("count catalog rows", error))?;
    from_i64(value)
}

fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(char::from(DIGITS[usize::from(byte >> 4)]));
        output.push(char::from(DIGITS[usize::from(byte & 0x0f)]));
    }
    output
}

impl From<std::num::TryFromIntError> for PersistenceError {
    fn from(_: std::num::TryFromIntError) -> Self {
        Self::HistoryArithmeticOverflow
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use causafera_core::{CausalEffect, CausalTarget, Phase, StateFingerprint};
    use causafera_types::{EventKindId, StateObjectKindId, StatePropertyId};

    struct TestBundle(PathBuf);

    impl TestBundle {
        fn new(name: &str) -> Self {
            let path = std::env::temp_dir().join(format!(
                "causafera-history-validation-{name}-{}",
                std::process::id()
            ));
            fs::create_dir(&path).expect("unique test bundle creates");
            Self(path)
        }
    }

    impl Drop for TestBundle {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).expect("test bundle removes");
        }
    }

    fn causal(sequence: u64, causes: Vec<TraceId>) -> OwnedCausalRecord {
        OwnedCausalRecord::new(
            sequence,
            EventId::new(sequence),
            TraceId::new(sequence),
            SimulationTime::new(sequence),
            Phase::Physics,
            EventKindId::new(1),
            causes,
            vec![
                CausalEffect::new(
                    CausalTarget::new(StateObjectKindId::new(1), sequence, StatePropertyId::new(1)),
                    StateFingerprint::new([1; 32]),
                    StateFingerprint::new([2; 32]),
                )
                .expect("effect changes state"),
            ],
        )
        .expect("causal record is valid")
    }

    #[test]
    fn causal_batch_validation_uses_bounded_catalog_queries() {
        // Given: a real catalog with one durable root and a large valid child chain.
        let bundle = TestBundle::new("bounded-query");
        let mut store = HistoryStore::create(&bundle.0, [0xA5; 32], DurabilityMode::EveryTick)
            .expect("history store creates");
        store
            .append_tick(vec![HistoryRecord::Causal(causal(0, Vec::new()))])
            .expect("root persists");
        let records = (1_u64..=4_096)
            .map(|sequence| causal(sequence, vec![TraceId::new(sequence - 1)]))
            .collect::<Vec<_>>();
        CAUSAL_VALIDATION_CATALOG_LOOKUPS.store(0, Ordering::SeqCst);

        // When: the append-boundary validator checks the batch against real SQLite state.
        validate_causal_batch_against_catalog(&store.catalog, &records, &store.pending)
            .expect("valid causal batch");

        // Then: catalog work is batched rather than one query per parent record.
        let lookups = CAUSAL_VALIDATION_CATALOG_LOOKUPS.load(Ordering::SeqCst);
        println!(
            "causal_validation records={} catalog_queries={lookups}",
            records.len()
        );
        assert!(lookups <= 10, "catalog query count must be batch-bounded");
    }

    #[test]
    fn causal_batch_validation_fails_closed_for_invalid_references() {
        // Given: a real catalog containing trace and event zero.
        let bundle = TestBundle::new("fail-closed");
        let mut store = HistoryStore::create(&bundle.0, [0x5A; 32], DurabilityMode::EveryTick)
            .expect("history store creates");
        store
            .append_tick(vec![HistoryRecord::Causal(causal(0, Vec::new()))])
            .expect("root persists");
        let cases = [
            ("duplicate-event", causal(0, vec![TraceId::new(0)])),
            ("duplicate-trace", {
                let mut record = causal(1, Vec::new());
                record = OwnedCausalRecord::new(
                    record.commit_sequence(),
                    record.event_id(),
                    TraceId::new(0),
                    record.time(),
                    record.phase(),
                    record.kind(),
                    record.causes().to_vec(),
                    record.effects().to_vec(),
                )
                .expect("duplicate trace fixture is structurally valid");
                record
            }),
            ("unknown-parent", causal(1, vec![TraceId::new(99)])),
        ];

        // When: each malformed batch crosses the append-boundary validator.
        let failures = cases.map(|(name, record)| {
            let error =
                validate_causal_batch_against_catalog(&store.catalog, &[record], &store.pending)
                    .expect_err("malformed causal batch must fail closed");
            (name, error)
        });

        // Then: duplicate and unknown-parent inputs retain typed hard errors.
        for (name, error) in failures {
            println!("causal_validation typed_failure[{name}]={error:?}");
            assert!(matches!(
                (name, error),
                (
                    "duplicate-event",
                    PersistenceError::History(HistoryError::DuplicateEventId(_))
                ) | (
                    "duplicate-trace",
                    PersistenceError::History(HistoryError::DuplicateTraceId(_))
                ) | (
                    "unknown-parent",
                    PersistenceError::History(HistoryError::UnknownParent { .. })
                )
            ));
        }
    }
}
