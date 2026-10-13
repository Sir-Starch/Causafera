use std::collections::{BTreeSet, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

use causafera_core::{
    CausalHistoryReader, CausalTarget, CausalTraceStore, DomainEvidenceRecord,
    EvidenceDecoderRegistry, EvidenceSchemaId, HistoryAnchor, HistoryAnchorFields, HistoryError,
    HistoryFrontier, HistoryPage, HistoryPageItem, HistoryQuery, HistoryRecord,
    MAX_FRAME_DECODED_BYTES, MAX_HISTORY_RECORDS_PER_FRAME, OwnedCausalRecord,
    canonical_history_records, validate_frame_shape,
};
#[cfg(test)]
use causafera_persistence::FailurePoint;
use causafera_persistence::{
    CheckpointMetadata, CheckpointPolicy, CheckpointThreshold, DurabilityMode, HistoryStore,
    HistoryStoreReader, HistoryStoreStatus, should_checkpoint,
};
use causafera_types::{EventId, TraceId};

use crate::history_reader::RuntimeHistoryReader;
use crate::{CanonicalDigest, RuntimeError};

const MAX_PENDING_HISTORY_TICKS: usize = 32;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HistoryCommitPhase {
    BeforeFramePublication,
    AfterFrameValidation,
    AfterLogicalPublication,
    AfterStatePublication,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct FrameSummary {
    generation: u64,
    first_trace: Option<TraceId>,
    last_trace: Option<TraceId>,
    records: usize,
    frame: Vec<HistoryRecord>,
}

/// Where a session's durable history lives.
#[derive(Clone, Debug)]
pub(crate) enum HistoryTarget {
    /// A scratch bundle removed when the session ends: `Runtime::new`, tests,
    /// and every session that was not asked to keep its history.
    Temporary,
    /// A named bundle that outlives the session.
    Bundle { root: PathBuf, mode: DurabilityMode },
}

pub(crate) struct RuntimeHistoryCoordinator {
    reader: Arc<RuntimeHistoryReader>,
    store: Arc<Mutex<HistoryStore>>,
    durable_reader: Arc<SharedHistoryReader>,
    cleanup_root: Option<Arc<TemporaryHistoryRoot>>,
    logical_generation: u64,
    durable_generation: u64,
    visible_head: TraceId,
    /// The logical history digest after the last published frame: a BLAKE3
    /// fold of every causal and evidence record in stream order, maintained by
    /// the store one record at a time. Cached here so reading it is infallible
    /// and constant-time; the cold oracle is [`Self::logical_digest_full_rescan`].
    logical_digest: [u8; 32],
    pending: VecDeque<FrameSummary>,
    handed_off_generation: u64,
    failure: Option<HistoryCommitPhase>,
    force_storage_flush: bool,
    /// The named bundle this history is, if it is one that outlives the session.
    bundle: Option<PathBuf>,
    /// Set when the session is dropped to simulate a crash: shutdown then skips
    /// the flush a clean exit performs.
    abandoned: bool,
    /// How far the durable history has run past the latest checkpoint.
    cadence: CadenceCounters,
    /// Set while the session re-executes frames the bundle already holds.
    replay: Option<ReplayCursor>,
    /// The failure of the last automatic checkpoint, if it has not succeeded since.
    checkpoint_error: Option<RuntimeError>,
    /// Set when this coordinator came from a bounded snapshot's owned history,
    /// not from importing a complete causal-trace snapshot. A complete import is
    /// validated against the whole history it carries; an owned resume is
    /// validated by indexed reads and never materialises its history.
    owned_resume: bool,
}

#[derive(Debug)]
struct TemporaryHistoryRoot(PathBuf);

impl Drop for TemporaryHistoryRoot {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[derive(Clone)]
pub struct RuntimeHistoryResume {
    coordinator: RuntimeHistoryCoordinator,
}

/// What a reopened bundle needs re-executed: the frames after its checkpoint.
#[derive(Clone, Copy, Debug)]
pub(crate) struct ReplayPlan {
    pub checkpoint_generation: u64,
    pub first_ordinal: u64,
    pub frames: u64,
}

/// How far a bundle's durable history has run past its latest checkpoint.
#[derive(Clone, Copy, Debug)]
struct CadenceCounters {
    policy: crate::CheckpointCadence,
    ticks: u64,
    bytes: u64,
}

impl CadenceCounters {
    const fn new(policy: crate::CheckpointCadence) -> Self {
        Self {
            policy,
            ticks: 0,
            bytes: 0,
        }
    }

    fn due(&self) -> bool {
        should_checkpoint(
            self.ticks,
            self.bytes,
            &CheckpointPolicy {
                threshold: CheckpointThreshold::Coordinator {
                    ticks: self.policy.ticks,
                    bytes: self.policy.frame_bytes,
                },
                force: false,
            },
        )
    }
}

/// Frames a reopened bundle holds past its latest checkpoint, which the session
/// re-executes (and checks) instead of appending.
#[derive(Clone, Copy, Debug)]
struct ReplayCursor {
    next_ordinal: u64,
    remaining: u64,
    durable_digest: [u8; 32],
    evidence_manifest_digest: [u8; 32],
}

/// What a bundle save published.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct BundleSaveReceipt {
    pub generation: u64,
    pub checkpoint_digest: [u8; 32],
    pub snapshot_digest: [u8; 32],
    pub snapshot_path: PathBuf,
}

pub(crate) struct HistoryTxn {
    reader: Arc<RuntimeHistoryReader>,
    generation: u64,
    first_trace: u64,
    first_event: u64,
    staged: Vec<OwnedCausalRecord>,
    evidence: Vec<DomainEvidenceRecord>,
}

pub(crate) struct PreparedHistoryFrame {
    generation: u64,
    records: Vec<HistoryRecord>,
}

pub(crate) struct HistoryPublishReceipt {
    previous_generation: u64,
    previous_digest: CanonicalDigest,
    previous_digest_bytes: [u8; 32],
    previous_frontier: HistoryFrontier,
    frame_records: usize,
    /// The replay position before this frame, restored if the frame is rolled back.
    replay: Option<(u64, u64)>,
}

static NEXT_TEMP_STORE: AtomicU64 = AtomicU64::new(0);

#[cfg(test)]
thread_local! {
    /// Makes a replay see a stored frame that is one record short, so a test can
    /// show that a re-executed tick which differs from the bundle is refused.
    static REPLAY_TAMPER: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

#[cfg(test)]
pub(crate) fn tamper_replayed_stored_frames(enabled: bool) {
    REPLAY_TAMPER.with(|tamper| tamper.set(enabled));
}

pub(crate) fn runtime_history_namespace(world_seed: u64) -> [u8; 32] {
    let mut namespace = [0_u8; 32];
    namespace[..8].copy_from_slice(&world_seed.to_le_bytes());
    namespace[8..16].copy_from_slice(&world_seed.rotate_left(29).to_le_bytes());
    namespace
}

pub(crate) fn runtime_history_decoder_registry()
-> Result<Arc<EvidenceDecoderRegistry>, RuntimeError> {
    let mut registry = EvidenceDecoderRegistry::new();
    crate::register_hydrology_evidence_decoders(&mut registry)?;
    Ok(Arc::new(registry))
}

pub(crate) fn temporary_history_store_with_registry(
    world_seed: u64,
    registry: Arc<EvidenceDecoderRegistry>,
) -> Result<(HistoryStore, PathBuf), RuntimeError> {
    let root = (0..1024)
        .find_map(|_| {
            let attempt = NEXT_TEMP_STORE.fetch_add(1, Ordering::Relaxed);
            let candidate = std::env::temp_dir().join(format!(
                "causafera-runtime-history-p{}-attempt-{attempt}",
                std::process::id()
            ));
            match std::fs::create_dir(&candidate) {
                Ok(()) => Some(Ok(candidate)),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => None,
                Err(error) => Some(Err(RuntimeError::SnapshotAssembly(
                    causafera_persistence::PersistenceError::Codec {
                        message: format!("create temporary history store: {error}"),
                    },
                ))),
            }
        })
        .ok_or_else(|| {
            RuntimeError::SnapshotAssembly(causafera_persistence::PersistenceError::Codec {
                message: "exhausted temporary history store attempts".into(),
            })
        })??;
    match HistoryStore::create_with_registry(
        &root,
        runtime_history_namespace(world_seed),
        DurabilityMode::GroupCommit,
        registry,
    ) {
        Ok(store) => Ok((store, root)),
        Err(error) => {
            let _ = std::fs::remove_dir_all(&root);
            Err(RuntimeError::SnapshotAssembly(error))
        }
    }
}

#[derive(Debug)]
struct SharedHistoryReader {
    inner: Mutex<HistoryStoreReader>,
}

impl SharedHistoryReader {
    fn new(reader: HistoryStoreReader) -> Self {
        Self {
            inner: Mutex::new(reader),
        }
    }

    fn append_durable(
        &self,
        records: &[HistoryRecord],
        anchor: HistoryAnchor,
    ) -> Result<(), RuntimeError> {
        self.inner
            .lock()
            .map_err(|_| RuntimeError::InvalidSnapshot("history reader lock poisoned"))?
            .append_durable(records, anchor)
            .map_err(RuntimeError::SnapshotAssembly)
    }

    fn with_reader<T>(
        &self,
        operation: impl FnOnce(&HistoryStoreReader) -> Result<T, HistoryError>,
    ) -> Result<T, HistoryError> {
        let reader = self
            .inner
            .lock()
            .map_err(|_| HistoryError::ReaderOperationUnavailable)?;
        operation(&reader)
    }
}

impl CausalHistoryReader for SharedHistoryReader {
    fn pin_snapshot(&self) -> Result<Arc<dyn CausalHistoryReader>, HistoryError> {
        self.with_reader(|reader| reader.pin_snapshot())
    }

    fn all_records_for_oracle(&self) -> Result<Vec<OwnedCausalRecord>, HistoryError> {
        self.with_reader(|reader| reader.all_records_for_oracle())
    }

    fn anchor(&self) -> Result<HistoryAnchor, HistoryError> {
        self.with_reader(CausalHistoryReader::anchor)
    }

    fn event(&self, trace_id: TraceId) -> Result<Option<OwnedCausalRecord>, HistoryError> {
        self.with_reader(|reader| reader.event(trace_id))
    }

    fn event_by_id(&self, event_id: EventId) -> Result<Option<OwnedCausalRecord>, HistoryError> {
        self.with_reader(|reader| reader.event_by_id(event_id))
    }

    fn records(&self, query: HistoryQuery) -> Result<HistoryPage<HistoryRecord>, HistoryError> {
        self.with_reader(|reader| reader.records(query))
    }

    fn indexed_range(
        &self,
        query: HistoryQuery,
    ) -> Result<HistoryPage<HistoryRecord>, HistoryError> {
        self.with_reader(|reader| reader.indexed_range(query))
    }

    fn children(
        &self,
        trace_id: TraceId,
        query: HistoryQuery,
    ) -> Result<HistoryPage<TraceId>, HistoryError> {
        self.with_reader(|reader| reader.children(trace_id, query))
    }

    fn child_count(&self, trace_id: TraceId) -> Result<u64, HistoryError> {
        self.with_reader(|reader| reader.child_count(trace_id))
    }

    fn latest_matching_event_before(
        &self,
        target: CausalTarget,
        trace_id: TraceId,
    ) -> Result<Option<OwnedCausalRecord>, HistoryError> {
        self.with_reader(|reader| reader.latest_matching_event_before(target, trace_id))
    }

    fn evidence(
        &self,
        schema_id: EvidenceSchemaId,
        canonical_key: &[u8],
        query: HistoryQuery,
    ) -> Result<HistoryPage<DomainEvidenceRecord>, HistoryError> {
        self.with_reader(|reader| reader.evidence(schema_id, canonical_key, query))
    }

    fn evidence_count(
        &self,
        schema_id: EvidenceSchemaId,
        canonical_key: &[u8],
    ) -> Result<u64, HistoryError> {
        self.with_reader(|reader| reader.evidence_count(schema_id, canonical_key))
    }

    fn evidence_by_anchor(
        &self,
        schema_id: causafera_core::EvidenceSchemaId,
        anchor: TraceId,
        query: HistoryQuery,
    ) -> Result<causafera_core::HistoryPage<causafera_core::DomainEvidenceRecord>, HistoryError>
    {
        self.with_reader(|reader| reader.evidence_by_anchor(schema_id, anchor, query))
    }

    fn evidence_by_anchor_count(
        &self,
        schema_id: causafera_core::EvidenceSchemaId,
        anchor: TraceId,
    ) -> Result<u64, HistoryError> {
        self.with_reader(|reader| reader.evidence_by_anchor_count(schema_id, anchor))
    }

    fn evidence_by_key_prefix(
        &self,
        schema_id: causafera_core::EvidenceSchemaId,
        canonical_key_prefix: &[u8],
        query: HistoryQuery,
    ) -> Result<causafera_core::HistoryPage<causafera_core::DomainEvidenceRecord>, HistoryError>
    {
        self.with_reader(|reader| {
            reader.evidence_by_key_prefix(schema_id, canonical_key_prefix, query)
        })
    }

    fn evidence_by_key_prefix_count(
        &self,
        schema_id: causafera_core::EvidenceSchemaId,
        canonical_key_prefix: &[u8],
    ) -> Result<u64, HistoryError> {
        self.with_reader(|reader| {
            reader.evidence_by_key_prefix_count(schema_id, canonical_key_prefix)
        })
    }

    fn checkpoint_at(
        &self,
        time: causafera_types::SimulationTime,
    ) -> Result<Option<causafera_core::HistoryCheckpoint>, HistoryError> {
        self.with_reader(|reader| reader.checkpoint_at(time))
    }
}

impl Clone for RuntimeHistoryCoordinator {
    fn clone(&self) -> Self {
        Self {
            reader: self.reader.clone(),
            store: self.store.clone(),
            durable_reader: self.durable_reader.clone(),
            cleanup_root: self.cleanup_root.clone(),
            logical_generation: self.logical_generation,
            durable_generation: self.durable_generation,
            visible_head: self.visible_head,
            logical_digest: self.logical_digest,
            pending: self.pending.clone(),
            handed_off_generation: self.handed_off_generation,
            failure: self.failure,
            force_storage_flush: self.force_storage_flush,
            bundle: self.bundle.clone(),
            abandoned: self.abandoned,
            cadence: self.cadence,
            replay: self.replay,
            checkpoint_error: self.checkpoint_error.clone(),
            owned_resume: self.owned_resume,
        }
    }
}

impl RuntimeHistoryResume {
    pub(crate) fn reader(&self) -> Arc<dyn CausalHistoryReader> {
        self.coordinator.reader()
    }

    pub(crate) fn install(
        self,
        hot_budget: usize,
    ) -> Result<(RuntimeHistoryCoordinator, CausalTraceStore), RuntimeError> {
        let traces = CausalTraceStore::with_cold_reader(
            self.coordinator.reader.clone(),
            hot_budget,
            Vec::new(),
        )?;
        let mut coordinator = self.coordinator;
        coordinator.owned_resume = true;
        Ok((coordinator, traces))
    }
}

impl RuntimeHistoryCoordinator {
    pub(crate) fn register_migration_proof(
        &self,
        reference: causafera_persistence::MigrationProofReference,
    ) -> Result<(), RuntimeError> {
        self.store
            .lock()
            .map_err(|_| RuntimeError::InvalidSnapshot("history store lock poisoned"))?
            .register_migration_proof(reference)?;
        Ok(())
    }
    pub(crate) fn reader(&self) -> Arc<dyn CausalHistoryReader> {
        self.reader.clone()
    }

    /// Whether this history was handed over by a bounded snapshot rather than
    /// rebuilt from a complete causal-trace snapshot.
    pub(crate) const fn is_owned_resume(&self) -> bool {
        self.owned_resume
    }

    pub(crate) fn resume(&self) -> Result<RuntimeHistoryResume, RuntimeError> {
        if self.logical_generation != self.durable_generation || !self.pending.is_empty() {
            return Err(RuntimeError::InvalidSnapshot(
                "history resume requires a forced durable generation",
            ));
        }
        Ok(RuntimeHistoryResume {
            coordinator: self.clone(),
        })
    }

    fn lock_store(&self) -> Result<MutexGuard<'_, HistoryStore>, RuntimeError> {
        self.store
            .lock()
            .map_err(|_| RuntimeError::InvalidSnapshot("history store lock poisoned"))
    }

    pub(crate) fn bootstrap(
        traces: &CausalTraceStore,
        world_seed: u64,
        registry: Arc<EvidenceDecoderRegistry>,
    ) -> Result<(Self, CausalTraceStore), RuntimeError> {
        Self::bootstrap_with_registry_at_generation(
            traces,
            world_seed,
            traces.hot_budget(),
            None,
            registry,
        )
    }

    pub(crate) fn bootstrap_with_registry_at_generation(
        traces: &CausalTraceStore,
        world_seed: u64,
        hot_budget: usize,
        target_generation: Option<u64>,
        registry: Arc<EvidenceDecoderRegistry>,
    ) -> Result<(Self, CausalTraceStore), RuntimeError> {
        let (store, cleanup_root) = temporary_history_store_with_registry(world_seed, registry)?;
        Self::bootstrap_with_store_and_cleanup(
            traces,
            world_seed,
            hot_budget,
            store,
            Some(cleanup_root),
            target_generation,
        )
    }

    #[cfg(test)]
    pub(crate) fn bootstrap_with_store(
        traces: &CausalTraceStore,
        world_seed: u64,
        hot_budget: usize,
        store: HistoryStore,
    ) -> Result<(Self, CausalTraceStore), RuntimeError> {
        Self::bootstrap_with_store_and_cleanup(traces, world_seed, hot_budget, store, None, None)
    }

    fn bootstrap_with_store_and_cleanup(
        traces: &CausalTraceStore,
        world_seed: u64,
        hot_budget: usize,
        mut store: HistoryStore,
        cleanup_root: Option<PathBuf>,
        target_generation: Option<u64>,
    ) -> Result<(Self, CausalTraceStore), RuntimeError> {
        if store.namespace() != &runtime_history_namespace(world_seed) {
            return Err(RuntimeError::InvalidSnapshot(
                "history store namespace does not match runtime world seed",
            ));
        }
        let records = owned_hot_records(traces)?;
        let current_frontier = store.logical_reader()?.anchor()?.frontier();
        let existing_events = store.catalog().counts()?.events;
        let first_trace = if existing_events == 0 {
            0
        } else {
            current_frontier
                .trace_id
                .raw()
                .checked_add(1)
                .ok_or(HistoryError::ArithmeticOverflow)?
        };
        let delta = records
            .iter()
            .filter(|record| record.trace_id().raw() >= first_trace)
            .cloned()
            .map(HistoryRecord::Causal)
            .collect::<Vec<_>>();
        if !delta.is_empty() {
            let mut frame_start = 0_usize;
            let mut frame_bytes = 0_usize;
            for (index, record) in delta.iter().enumerate() {
                let record_bytes = record.decoded_size();
                let over_count = index.saturating_sub(frame_start) >= MAX_HISTORY_RECORDS_PER_FRAME;
                let over_bytes = match frame_bytes.checked_add(record_bytes) {
                    Some(bytes) => bytes > MAX_FRAME_DECODED_BYTES,
                    None => true,
                };
                if index > frame_start && (over_count || over_bytes) {
                    store.append_tick(delta[frame_start..index].to_vec())?;
                    frame_start = index;
                    frame_bytes = 0;
                }
                frame_bytes = frame_bytes
                    .checked_add(record_bytes)
                    .ok_or(HistoryError::ArithmeticOverflow)?;
            }
            if frame_start < delta.len() {
                store.append_tick(delta[frame_start..].to_vec())?;
            }
            store.flush_durable()?;
        }
        if let Some(target_generation) = target_generation {
            let current_generation = store.logical_generation();
            let missing = target_generation.checked_sub(current_generation).ok_or(
                RuntimeError::InvalidSnapshot(
                    "bounded snapshot history generation predates imported frames",
                ),
            )?;
            if missing > crate::MAX_RUNTIME_TICKS.saturating_add(1) {
                return Err(RuntimeError::InvalidSnapshot(
                    "bounded snapshot history generation exceeds runtime bounds",
                ));
            }
            if missing != 0 {
                store.advance_logical_generation(target_generation)?;
            }
        }
        let durable_reader = store.durable_reader()?;
        let durable_anchor = durable_reader.anchor()?;
        let counts = store.catalog().counts()?;
        let durable_record_count = counts
            .events
            .checked_add(counts.evidence)
            .and_then(|count| usize::try_from(count).ok())
            .ok_or(HistoryError::ArithmeticOverflow)?;
        let shared_reader = Arc::new(SharedHistoryReader::new(durable_reader));
        let cold_reader: Arc<dyn CausalHistoryReader> = shared_reader.clone();
        let durable_causal_record_count =
            usize::try_from(counts.events).map_err(|_| HistoryError::ArithmeticOverflow)?;
        let reader = Arc::new(RuntimeHistoryReader::new(
            cold_reader.clone(),
            durable_record_count,
            durable_causal_record_count,
        )?);
        let visible_head = durable_anchor.frontier().trace_id;
        let bounded = CausalTraceStore::with_cold_reader(reader.clone(), hot_budget, records)?;
        Ok((
            Self {
                reader,
                store: Arc::new(Mutex::new(store)),
                durable_reader: shared_reader,
                cleanup_root: cleanup_root.map(|root| Arc::new(TemporaryHistoryRoot(root))),
                logical_generation: durable_anchor.logical_generation(),
                durable_generation: durable_anchor.durable_generation(),
                visible_head,
                logical_digest: *durable_anchor.logical_digest(),
                pending: VecDeque::new(),
                handed_off_generation: durable_anchor.logical_generation(),
                failure: None,
                force_storage_flush: false,
                bundle: None,
                abandoned: false,
                cadence: CadenceCounters::new(crate::CheckpointCadence::default()),
                replay: None,
                checkpoint_error: None,
                owned_resume: false,
            },
            bounded,
        ))
    }

    /// A coordinator over `target`, bootstrapped from the genesis records in
    /// `traces`.
    pub(crate) fn bootstrap_in(
        traces: &CausalTraceStore,
        world_seed: u64,
        registry: Arc<EvidenceDecoderRegistry>,
        target: &HistoryTarget,
    ) -> Result<(Self, CausalTraceStore), RuntimeError> {
        match target {
            HistoryTarget::Temporary => Self::bootstrap(traces, world_seed, registry),
            HistoryTarget::Bundle { root, mode } => {
                let store = HistoryStore::create_with_registry(
                    root,
                    runtime_history_namespace(world_seed),
                    *mode,
                    registry,
                )?;
                let (mut coordinator, bounded) = Self::bootstrap_with_store_and_cleanup(
                    traces,
                    world_seed,
                    traces.hot_budget(),
                    store,
                    None,
                    None,
                )?;
                coordinator.bundle = Some(root.clone());
                Ok((coordinator, bounded))
            }
        }
    }

    /// A resume handle over a bundle that was opened from disk.
    ///
    /// Without a replay plan the handle is at the bundle's durable head and
    /// nothing is appended: the history is exactly what the bundle holds. With
    /// one it is pinned at the checkpoint the session restores from, and the
    /// durable frames after it are re-executed rather than appended.
    pub(crate) fn resume_open_bundle(
        store: HistoryStore,
        root: &Path,
        cadence: crate::CheckpointCadence,
        replay: Option<ReplayPlan>,
    ) -> Result<RuntimeHistoryResume, RuntimeError> {
        let (durable_reader, events, evidence, cursor) = match replay {
            None => {
                let counts = store.catalog().counts()?;
                (
                    store.durable_reader()?,
                    counts.events,
                    counts.evidence,
                    None,
                )
            }
            Some(plan) => {
                let (events, evidence) = store.record_counts_at(plan.checkpoint_generation)?;
                let status = store.status()?;
                (
                    store.reader_at_durable_generation(plan.checkpoint_generation)?,
                    events,
                    evidence,
                    Some(ReplayCursor {
                        next_ordinal: plan.first_ordinal,
                        remaining: plan.frames,
                        durable_digest: status.durable_digest,
                        evidence_manifest_digest: status.durable_evidence_manifest_digest,
                    }),
                )
            }
        };
        let durable_anchor = durable_reader.anchor()?;
        let record_count = events
            .checked_add(evidence)
            .and_then(|count| usize::try_from(count).ok())
            .ok_or(HistoryError::ArithmeticOverflow)?;
        let causal_count = usize::try_from(events).map_err(|_| HistoryError::ArithmeticOverflow)?;
        let shared_reader = Arc::new(SharedHistoryReader::new(durable_reader));
        let cold_reader: Arc<dyn CausalHistoryReader> = shared_reader.clone();
        let reader = Arc::new(RuntimeHistoryReader::new(
            cold_reader,
            record_count,
            causal_count,
        )?);
        Ok(RuntimeHistoryResume {
            coordinator: Self {
                reader,
                store: Arc::new(Mutex::new(store)),
                durable_reader: shared_reader,
                cleanup_root: None,
                logical_generation: durable_anchor.logical_generation(),
                durable_generation: durable_anchor.durable_generation(),
                visible_head: durable_anchor.frontier().trace_id,
                logical_digest: *durable_anchor.logical_digest(),
                pending: VecDeque::new(),
                handed_off_generation: durable_anchor.logical_generation(),
                failure: None,
                force_storage_flush: false,
                bundle: Some(root.to_path_buf()),
                abandoned: false,
                cadence: CadenceCounters::new(cadence),
                replay: cursor,
                checkpoint_error: None,
                owned_resume: true,
            },
        })
    }

    /// End a replay: the session is at the bundle's durable head.
    ///
    /// Every replayed frame matched, so the history the session now holds is
    /// the history the bundle recorded; this only moves the reader and the
    /// durable bookkeeping from the checkpoint's generation to that head.
    pub(crate) fn finish_replay(&mut self) -> Result<(), RuntimeError> {
        let Some(cursor) = self.replay else {
            return Ok(());
        };
        if cursor.remaining != 0 {
            return Err(RuntimeError::ReplayDiverged {
                generation: self.logical_generation,
                reason: "the replay ended before the bundle's durable head",
            });
        }
        self.replay = None;
        self.reader.set_digests(
            cursor.durable_digest,
            cursor.durable_digest,
            cursor.evidence_manifest_digest,
        )?;
        self.logical_digest = cursor.durable_digest;
        self.install_durable_state()
    }

    /// One replayed frame: check it against the stored frame it re-executes
    /// and advance the session's view of its history as if it had been appended.
    fn publish_replayed(
        &mut self,
        frame: &PreparedHistoryFrame,
    ) -> Result<HistoryPublishReceipt, RuntimeError> {
        let Some(cursor) = self.replay else {
            return Err(RuntimeError::InvalidSnapshot("no replay is in progress"));
        };
        self.reader
            .validate_append(frame.generation, &frame.records)?;
        #[allow(unused_mut, reason = "only a test corrupts the stored frame")]
        let mut stored = self.lock_store()?.read_frame(cursor.next_ordinal)?.ok_or(
            RuntimeError::ReplayDiverged {
                generation: frame.generation,
                reason: "the bundle has no stored frame for this tick",
            },
        )?;
        #[cfg(test)]
        if REPLAY_TAMPER.with(std::cell::Cell::get) {
            stored.records.pop();
        }
        if stored.generation != frame.generation {
            return Err(RuntimeError::ReplayDiverged {
                generation: frame.generation,
                reason: "the stored frame belongs to a different generation",
            });
        }
        if stored.records != frame.records {
            return Err(RuntimeError::ReplayDiverged {
                generation: frame.generation,
                reason: "the re-executed tick produced different records than the bundle holds",
            });
        }
        let (previous_generation, previous_digest, previous_frontier) =
            self.reader.publication_preimage()?;
        let previous_digest_bytes = self.reader.logical_digest_bytes()?;
        let anchor = self.reader.anchor()?;
        self.reader.append(
            frame.generation,
            &frame.records,
            *anchor.logical_digest(),
            *anchor.storage_root(),
            *anchor.evidence_manifest_digest(),
        )?;
        self.pending.push_back(FrameSummary {
            generation: frame.generation,
            first_trace: frame.first_trace(),
            last_trace: frame.last_trace(),
            records: frame.records.len(),
            frame: frame.records.clone(),
        });
        self.logical_generation = frame.generation;
        self.handed_off_generation = frame.generation;
        if let Some(last) = frame.last_trace() {
            self.visible_head = last;
        }
        self.replay = Some(ReplayCursor {
            next_ordinal: cursor.next_ordinal + 1,
            remaining: cursor.remaining - 1,
            ..cursor
        });
        Ok(HistoryPublishReceipt {
            previous_generation,
            previous_digest,
            previous_digest_bytes,
            previous_frontier,
            frame_records: frame.records.len(),
            replay: Some((cursor.next_ordinal, cursor.remaining)),
        })
    }

    pub(crate) fn set_cadence(&mut self, cadence: crate::CheckpointCadence) {
        self.cadence = CadenceCounters::new(cadence);
    }

    /// Whether the durable history has run far enough past its latest checkpoint
    /// that the next quiescent durable frontier should be checkpointed.
    pub(crate) fn checkpoint_due(&self) -> bool {
        self.bundle.is_some()
            && self.replay.is_none()
            && self.pending.is_empty()
            && self.logical_generation == self.durable_generation
            && self.cadence.due()
    }

    pub(crate) fn note_checkpoint_failure(&mut self, error: RuntimeError) {
        self.checkpoint_error = Some(error);
    }

    pub(crate) fn checkpoint_error(&self) -> Option<RuntimeError> {
        self.checkpoint_error.clone()
    }

    /// Publish a checkpoint (no portable snapshot) of the durable frontier, the
    /// automatic kind taken when the history has run far past the last one.
    pub(crate) fn checkpoint_only(
        &mut self,
        bytes: &[u8],
        anchor: &HistoryAnchor,
        metadata: &CheckpointMetadata,
    ) -> Result<(), RuntimeError> {
        let mut store = self.lock_store()?;
        let generation = store.durable_generation();
        let existing = store
            .checkpoint_at_durable_generation(generation)?
            .filter(|existing| existing.header.durable_generation == generation);
        let digest = match existing {
            Some(existing) => existing.digest,
            None => *store.publish_checkpoint(anchor, metadata, bytes)?.digest(),
        };
        drop(store);
        self.reader.set_checkpoint_digest(digest)?;
        self.cadence.ticks = 0;
        self.cadence.bytes = 0;
        self.checkpoint_error = None;
        Ok(())
    }

    /// The scratch directory a temporary history lives in, if it is one.
    #[cfg(test)]
    pub(crate) fn temporary_root(&self) -> Option<&Path> {
        self.cleanup_root.as_ref().map(|root| root.0.as_path())
    }

    /// The root of the named bundle this history is, if it is one.
    pub(crate) fn bundle_root(&self) -> Option<&Path> {
        self.bundle.as_deref()
    }

    /// Mark the session crashed: dropping it will not run the clean-shutdown flush.
    pub(crate) fn abandon(&mut self) {
        self.abandoned = true;
    }

    pub(crate) const fn is_abandoned(&self) -> bool {
        self.abandoned
    }

    /// Force the pending group, then publish a checkpoint and a portable
    /// snapshot of the durable frontier.
    ///
    /// `bytes` is the bounded snapshot exported after the group was forced, so
    /// its anchor names exactly this generation. The checkpoint is skipped when
    /// the generation already has one (a second save with no tick between).
    pub(crate) fn save(
        &mut self,
        bytes: &[u8],
        anchor: &HistoryAnchor,
        metadata: &CheckpointMetadata,
    ) -> Result<BundleSaveReceipt, RuntimeError> {
        if self.bundle.is_none() {
            return Err(RuntimeError::NotABundle);
        }
        let mut store = self.lock_store()?;
        let generation = store.durable_generation();
        if anchor.durable_generation() != generation || anchor.logical_generation() != generation {
            return Err(RuntimeError::BundleInvalid {
                reason: "the snapshot anchor is not at the bundle's durable frontier",
            });
        }
        // `checkpoint_at_durable_generation` answers with the latest checkpoint
        // *linked* to the generation, which is an earlier one unless this
        // generation has its own; only an exact match is already saved.
        let (checkpoint_digest, snapshot) = match store
            .checkpoint_at_durable_generation(generation)?
            .filter(|existing| existing.header.durable_generation == generation)
        {
            Some(existing) => {
                let snapshot = store.publish_snapshot(generation, &existing.payload)?;
                (existing.digest, snapshot)
            }
            None => {
                let digest = *store.publish_checkpoint(anchor, metadata, bytes)?.digest();
                (digest, store.publish_snapshot(generation, bytes)?)
            }
        };
        drop(store);
        self.reader.set_checkpoint_digest(checkpoint_digest)?;
        self.cadence.ticks = 0;
        self.cadence.bytes = 0;
        self.checkpoint_error = None;
        Ok(BundleSaveReceipt {
            generation,
            checkpoint_digest,
            snapshot_digest: *snapshot.digest(),
            snapshot_path: snapshot.path().to_path_buf(),
        })
    }

    pub(crate) fn begin(&self) -> Result<HistoryTxn, RuntimeError> {
        let frontier = self.reader.anchor()?.frontier();
        let first_event = if self.reader.event(frontier.trace_id)?.is_some() {
            frontier
                .event_id
                .raw()
                .checked_add(1)
                .ok_or(HistoryError::ArithmeticOverflow)?
        } else {
            0
        };
        Ok(HistoryTxn {
            reader: self.reader.clone(),
            generation: self
                .logical_generation
                .checked_add(1)
                .ok_or(HistoryError::ArithmeticOverflow)?,
            first_trace: self
                .visible_head
                .raw()
                .checked_add(1)
                .ok_or(HistoryError::ArithmeticOverflow)?,
            first_event,
            staged: Vec::new(),
            evidence: Vec::new(),
        })
    }

    pub(crate) fn publish(
        &mut self,
        frame: &PreparedHistoryFrame,
    ) -> Result<HistoryPublishReceipt, RuntimeError> {
        if self.replay.is_some() {
            return self.publish_replayed(frame);
        }
        self.reject(HistoryCommitPhase::BeforeFramePublication)?;
        self.reader
            .validate_append(frame.generation, &frame.records)?;
        self.reject(HistoryCommitPhase::AfterFrameValidation)?;
        let (previous_generation, previous_digest, previous_frontier) =
            self.reader.publication_preimage()?;
        let previous_digest_bytes = self.reader.logical_digest_bytes()?;
        self.reject(HistoryCommitPhase::AfterLogicalPublication)?;
        self.reject(HistoryCommitPhase::AfterStatePublication)?;
        let force_storage_flush = self.force_storage_flush;
        let (persisted, persisted_durable_generation) = if force_storage_flush {
            let mut store = self.lock_store()?;
            let persisted = store.append_tick_durable(frame.records.clone())?;
            let durable_generation = store.durable_generation();
            drop(store);
            self.force_storage_flush = false;
            (persisted, durable_generation)
        } else {
            let mut store = self.lock_store()?;
            let persisted = store.append_tick(frame.records.clone())?;
            let durable_generation = store.durable_generation();
            (persisted, durable_generation)
        };
        if persisted.logical_generation() != frame.generation {
            return Err(RuntimeError::InvalidSnapshot(
                "history backend generation disagrees with runtime transaction",
            ));
        }
        self.reader.append(
            frame.generation,
            &frame.records,
            *persisted.logical_digest(),
            *persisted.logical_digest(),
            *persisted.evidence_manifest_digest(),
        )?;
        let summary = FrameSummary {
            generation: frame.generation,
            first_trace: frame.first_trace(),
            last_trace: frame.last_trace(),
            records: frame.records.len(),
            frame: frame.records.clone(),
        };
        self.logical_generation = persisted.logical_generation();
        self.logical_digest = *persisted.logical_digest();
        self.handed_off_generation = persisted.logical_generation();
        if let Some(last) = frame.last_trace() {
            self.visible_head = last;
        }
        let receipt = HistoryPublishReceipt {
            previous_generation,
            previous_digest,
            previous_digest_bytes,
            previous_frontier,
            frame_records: frame.records.len(),
            replay: None,
        };
        let range = persisted.range();
        self.cadence.ticks += 1;
        self.cadence.bytes += range.end.saturating_sub(range.start);
        self.pending.push_back(summary);
        if persisted_durable_generation == persisted.logical_generation() {
            self.install_durable_state()?
        } else {
            if self.pending.len() > MAX_PENDING_HISTORY_TICKS {
                self.flush_durable()?
            }
            self.durable_generation = persisted_durable_generation;
        }
        debug_assert_eq!(receipt.frame_records, frame.records.len());
        Ok(receipt)
    }

    pub(crate) fn finish_state_publication(&self) -> Result<(), RuntimeError> {
        Ok(())
    }

    pub(crate) fn flush_durable(&mut self) -> Result<(), RuntimeError> {
        self.lock_store()?.flush_durable()?;
        self.install_durable_state()
    }

    pub(crate) fn backend_status(&self) -> Result<HistoryStoreStatus, RuntimeError> {
        self.lock_store()?
            .status()
            .map_err(RuntimeError::SnapshotAssembly)
    }

    #[cfg(test)]
    pub(crate) fn inject_storage_failure(
        &mut self,
        point: Option<FailurePoint>,
    ) -> Result<(), RuntimeError> {
        self.lock_store()?.inject_failure(point);
        if point.is_some() {
            self.force_storage_flush = true;
        }
        Ok(())
    }

    fn install_durable_state(&mut self) -> Result<(), RuntimeError> {
        let records = self
            .pending
            .iter()
            .flat_map(|summary| summary.frame.iter().cloned())
            .collect::<Vec<_>>();
        let current = self.reader.anchor()?;
        let (durable_generation, counts) = {
            let store = self.lock_store()?;
            (store.durable_generation(), store.catalog().counts()?)
        };
        let anchor = HistoryAnchor::new(HistoryAnchorFields {
            namespace: *current.namespace(),
            logical_generation: current.logical_generation(),
            durable_generation,
            checkpoint_digest: *current.checkpoint_digest(),
            frontier: current.frontier(),
            logical_digest: *current.logical_digest(),
            storage_root: *current.storage_root(),
            physical_digest_schema: current.physical_digest_schema(),
            history_digest_schema: current.history_digest_schema(),
            evidence_manifest_digest: *current.evidence_manifest_digest(),
            durability: current.durability(),
        })?;
        self.durable_reader
            .append_durable(&records, anchor.clone())?;
        let record_count = counts
            .events
            .checked_add(counts.evidence)
            .and_then(|count| usize::try_from(count).ok())
            .ok_or(HistoryError::ArithmeticOverflow)?;
        self.reader.install_durable(
            self.durable_reader.clone(),
            record_count,
            usize::try_from(counts.events).map_err(|_| HistoryError::ArithmeticOverflow)?,
            anchor.clone(),
        )?;
        self.durable_generation = anchor.durable_generation();
        self.pending.clear();
        Ok(())
    }

    pub(crate) fn rollback(
        &mut self,
        frame: &PreparedHistoryFrame,
        receipt: HistoryPublishReceipt,
    ) -> Result<(), RuntimeError> {
        self.reader.rollback(
            receipt.previous_generation,
            &frame.records,
            receipt.previous_digest,
            receipt.previous_digest_bytes,
            receipt.previous_frontier,
        )?;
        let _ = self.pending.pop_back();
        self.logical_generation = receipt.previous_generation;
        self.logical_digest = receipt.previous_digest_bytes;
        self.visible_head = receipt.previous_frontier.trace_id;
        if let (Some((next_ordinal, remaining)), Some(cursor)) = (receipt.replay, self.replay) {
            self.replay = Some(ReplayCursor {
                next_ordinal,
                remaining,
                ..cursor
            });
        }
        Ok(())
    }

    pub(crate) const fn logical_generation(&self) -> u64 {
        self.logical_generation
    }

    pub(crate) const fn durable_generation(&self) -> u64 {
        self.durable_generation
    }

    pub(crate) const fn visible_head(&self) -> TraceId {
        self.visible_head
    }

    pub(crate) fn pending_ticks(&self) -> usize {
        self.pending.len()
    }

    pub(crate) fn handed_off_generation(&self) -> u64 {
        self.handed_off_generation
    }

    pub(crate) fn pending_records(&self) -> usize {
        self.pending.iter().map(|frame| frame.records).sum()
    }

    pub(crate) fn pending_trace_range(&self) -> (Option<TraceId>, Option<TraceId>) {
        (
            self.pending.front().and_then(|frame| frame.first_trace),
            self.pending.back().and_then(|frame| frame.last_trace),
        )
    }

    /// The history digest of the stream published so far.
    pub(crate) const fn logical_digest(&self) -> [u8; 32] {
        self.logical_digest
    }

    /// The same digest recomputed by decoding every frame from storage.
    ///
    /// The cold oracle `logical_digest` is checked against. Reads the whole
    /// history, so it is never on a tick.
    pub(crate) fn logical_digest_full_rescan(&self) -> Result<[u8; 32], RuntimeError> {
        #[cfg(test)]
        WHOLE_HISTORY_SCANS.with(|scans| scans.set(scans.get() + 1));
        Ok(self.lock_store()?.logical_digest_full_rescan()?)
    }

    pub(crate) fn records_for_cold_oracle(&self) -> Result<Vec<OwnedCausalRecord>, RuntimeError> {
        #[cfg(test)]
        WHOLE_HISTORY_SCANS.with(|scans| scans.set(scans.get() + 1));
        Ok(self.reader.all_records_for_oracle()?)
    }

    #[cfg(test)]
    pub(crate) fn inject_failure(&mut self, phase: Option<HistoryCommitPhase>) {
        self.failure = phase;
    }

    fn reject(&self, phase: HistoryCommitPhase) -> Result<(), RuntimeError> {
        if self.failure == Some(phase) {
            return Err(RuntimeError::HistoryCommitRejected { phase });
        }
        Ok(())
    }
}

impl HistoryTxn {
    /// Index of `trace_id` among this transaction's staged records.
    ///
    /// `stage_causal_records` only accepts records whose trace ids run
    /// consecutively from `first_trace`, so a staged lookup is a subtraction,
    /// not a scan of everything staged so far.
    fn staged_position(&self, trace_id: TraceId) -> Option<usize> {
        let offset = trace_id.raw().checked_sub(self.first_trace)?;
        let position = usize::try_from(offset).ok()?;
        (position < self.staged.len()).then_some(position)
    }

    #[cfg(test)]
    pub(crate) fn staged_event(
        &self,
        trace_id: TraceId,
    ) -> Result<Option<OwnedCausalRecord>, RuntimeError> {
        match self.staged_position(trace_id) {
            Some(position) => Ok(self.staged.get(position).cloned()),
            None => Ok(self.reader.event(trace_id)?),
        }
    }

    /// Whether the event is staged here or already in history, without
    /// cloning it.
    fn has_event(&self, trace_id: TraceId) -> Result<bool, RuntimeError> {
        if self.staged_position(trace_id).is_some() {
            return Ok(true);
        }
        Ok(self.reader.event(trace_id)?.is_some())
    }

    pub(crate) fn stage_causal_records(
        &mut self,
        records: &[OwnedCausalRecord],
    ) -> Result<(), RuntimeError> {
        let bytes = self
            .staged
            .iter()
            .chain(records)
            .try_fold(0_usize, |total, record| {
                total
                    .checked_add(record.canonical_field_size()?)
                    .ok_or(HistoryError::ArithmeticOverflow)
            })?;
        let overlay_len = self
            .staged
            .len()
            .checked_add(records.len())
            .ok_or(HistoryError::ArithmeticOverflow)?;
        validate_frame_shape(overlay_len, bytes)?;
        let base_event = self.first_event;
        let base_trace = self.first_trace;
        let mut seen_events: BTreeSet<_> = self
            .staged
            .iter()
            .map(OwnedCausalRecord::event_id)
            .collect();
        let mut seen_traces: BTreeSet<_> = self
            .staged
            .iter()
            .map(OwnedCausalRecord::trace_id)
            .collect();
        let mut known_traces = seen_traces.clone();
        for (index, record) in records.iter().enumerate() {
            let position = self
                .staged
                .len()
                .checked_add(index)
                .ok_or(HistoryError::ArithmeticOverflow)?;
            let may_be_published =
                record.trace_id().raw() < base_trace || record.event_id().raw() < base_event;
            if (may_be_published && self.reader.event(record.trace_id())?.is_some())
                || !seen_traces.insert(record.trace_id())
            {
                return Err(HistoryError::DuplicateTraceId(record.trace_id()).into());
            }
            if (may_be_published && self.reader.event_by_id(record.event_id())?.is_some())
                || !seen_events.insert(record.event_id())
            {
                return Err(HistoryError::DuplicateEventId(record.event_id()).into());
            }
            let expected_event = base_event
                .checked_add(u64::try_from(position).map_err(|_| HistoryError::ArithmeticOverflow)?)
                .ok_or(HistoryError::ArithmeticOverflow)?;
            let expected_trace = base_trace
                .checked_add(u64::try_from(position).map_err(|_| HistoryError::ArithmeticOverflow)?)
                .ok_or(HistoryError::ArithmeticOverflow)?;
            if record.event_id().raw() != expected_event
                || record.trace_id().raw() != expected_trace
            {
                return Err(HistoryError::NonCanonicalOrder { index: position }.into());
            }
            if position > 0 {
                let prior = if index > 0 {
                    records
                        .get(index - 1)
                        .ok_or(HistoryError::ArithmeticOverflow)?
                } else {
                    self.staged.last().ok_or(HistoryError::ArithmeticOverflow)?
                };
                if prior >= record {
                    return Err(HistoryError::NonCanonicalOrder { index: position }.into());
                }
            }
            for parent in record.causes() {
                if !known_traces.contains(parent)
                    && parent.raw() >= base_trace
                    && !self.has_event(*parent)?
                {
                    return Err(HistoryError::UnknownParent {
                        trace_id: record.trace_id(),
                        parent: *parent,
                    }
                    .into());
                }
            }
            known_traces.insert(record.trace_id());
        }
        self.staged.extend_from_slice(records);
        Ok(())
    }

    pub(crate) fn stage_evidence(
        &mut self,
        records: &[DomainEvidenceRecord],
    ) -> Result<(), RuntimeError> {
        let mut seen_keys: BTreeSet<_> = self
            .evidence
            .iter()
            .map(|record| (record.schema_id(), record.canonical_key().to_vec()))
            .collect();
        for record in records {
            if blake3::hash(record.canonical_payload()).as_bytes()
                != &record.payload_digest().bytes()
            {
                return Err(
                    causafera_persistence::PersistenceError::EvidencePayloadDigestMismatch {
                        schema_id: record.schema_id().raw(),
                    }
                    .into(),
                );
            }
            if !seen_keys.insert((record.schema_id(), record.canonical_key().to_vec())) {
                return Err(HistoryError::DuplicateRecordKey.into());
            }
            let query = HistoryQuery::new(0, 1, 4 * 1024 * 1024)?;
            if !self
                .reader
                .evidence(record.schema_id(), record.canonical_key(), query)?
                .records()
                .is_empty()
            {
                return Err(HistoryError::DuplicateRecordKey.into());
            }
            for anchor in record.causal_anchors() {
                if !self.has_event(*anchor)? {
                    return Err(HistoryError::UnknownEvidenceAnchor { anchor: *anchor }.into());
                }
            }
        }
        self.evidence.extend_from_slice(records);
        Ok(())
    }

    pub(crate) fn stage_tick_evidence(
        &mut self,
        traces: &CausalTraceStore,
        records: &[DomainEvidenceRecord],
    ) -> Result<(), RuntimeError> {
        self.stage_unstaged_causal_records(traces)?;
        self.stage_evidence(records)
    }

    #[cfg(test)]
    pub(crate) fn staged_evidence(
        &self,
        schema_id: causafera_core::EvidenceSchemaId,
        canonical_key: &[u8],
        query: HistoryQuery,
    ) -> Result<HistoryPage<DomainEvidenceRecord>, RuntimeError> {
        let mut values = self
            .reader
            .evidence(schema_id, canonical_key, query)?
            .records()
            .to_vec();
        values.extend(
            self.evidence
                .iter()
                .filter(|record| {
                    record.schema_id() == schema_id && record.canonical_key() == canonical_key
                })
                .cloned(),
        );
        page_values(&values, query).map_err(Into::into)
    }

    pub(crate) fn prepare(
        mut self,
        traces: &CausalTraceStore,
    ) -> Result<PreparedHistoryFrame, RuntimeError> {
        self.stage_unstaged_causal_records(traces)?;
        let mut frame = self
            .staged
            .into_iter()
            .map(HistoryRecord::Causal)
            .chain(self.evidence.into_iter().map(HistoryRecord::Evidence))
            .collect::<Vec<_>>();
        frame = canonical_history_records(frame)?;
        let bytes = frame.iter().try_fold(0_usize, |total, record| {
            total
                .checked_add(record.decoded_size())
                .ok_or(HistoryError::ArithmeticOverflow)
        })?;
        validate_frame_shape(frame.len(), bytes)?;
        Ok(PreparedHistoryFrame {
            generation: self.generation,
            records: frame,
        })
    }

    fn stage_unstaged_causal_records(
        &mut self,
        traces: &CausalTraceStore,
    ) -> Result<(), RuntimeError> {
        let staged_count =
            u64::try_from(self.staged.len()).map_err(|_| HistoryError::ArithmeticOverflow)?;
        let first_unstaged = self
            .first_trace
            .checked_add(staged_count)
            .ok_or(HistoryError::ArithmeticOverflow)?;
        let records = traces
            .iter_hot()
            .filter(|event| event.trace_id.raw() >= first_unstaged)
            .map(|event| {
                OwnedCausalRecord::new(
                    event.commit_sequence,
                    event.event_id,
                    event.trace_id,
                    event.time,
                    event.phase,
                    event.kind,
                    event.causes.to_vec(),
                    event.effects.to_vec(),
                )
            })
            .collect::<Result<Vec<_>, _>>()?;
        self.stage_causal_records(&records)
    }
}

impl PreparedHistoryFrame {
    pub(crate) fn records(&self) -> &[HistoryRecord] {
        &self.records
    }

    fn first_trace(&self) -> Option<TraceId> {
        self.records.iter().find_map(|record| match record {
            HistoryRecord::Causal(record) => Some(record.trace_id()),
            HistoryRecord::Evidence(_) => None,
        })
    }

    fn last_trace(&self) -> Option<TraceId> {
        self.records.iter().rev().find_map(|record| match record {
            HistoryRecord::Causal(record) => Some(record.trace_id()),
            HistoryRecord::Evidence(_) => None,
        })
    }
}

#[cfg(test)]
fn page_values<T: HistoryPageItem + Clone>(
    values: &[T],
    query: HistoryQuery,
) -> Result<HistoryPage<T>, HistoryError> {
    let offset = usize::try_from(query.offset()).map_err(|_| HistoryError::ArithmeticOverflow)?;
    let selected = values
        .iter()
        .skip(offset)
        .take(query.max_records())
        .cloned()
        .collect::<Vec<_>>();
    let next = (offset + selected.len() < values.len())
        .then(|| u64::try_from(offset + selected.len()))
        .transpose()
        .map_err(|_| HistoryError::ArithmeticOverflow)?;
    HistoryPage::new(selected, next, query)
}

fn owned_hot_records(traces: &CausalTraceStore) -> Result<Vec<OwnedCausalRecord>, HistoryError> {
    traces
        .iter_hot()
        .map(|event| {
            OwnedCausalRecord::new(
                event.commit_sequence,
                event.event_id,
                event.trace_id,
                event.time,
                event.phase,
                event.kind,
                event.causes.to_vec(),
                event.effects.to_vec(),
            )
        })
        .collect()
}

// Per-thread so a test that counts the scans of its own ticks is not disturbed by
// parallel tests that run the cold oracle on purpose.
#[cfg(test)]
thread_local! {
    static WHOLE_HISTORY_SCANS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

#[cfg(test)]
pub(crate) fn reset_whole_history_scans() {
    WHOLE_HISTORY_SCANS.with(|scans| scans.set(0));
}

#[cfg(test)]
pub(crate) fn whole_history_scans() -> u64 {
    WHOLE_HISTORY_SCANS.with(std::cell::Cell::get)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    };

    use causafera_core::{
        CausalCommitError, CausalEffect, CausalEventProposal, CausalTarget, Criticality,
        DomainEvidenceRecord, EventProposalKey, EvidenceDecoder, EvidenceResolution,
        EvidenceSchemaId, EvidenceSchemaVersion, HistoryReader, HistoryRecord, OwnedCausalRecord,
        Phase, StateFingerprint,
    };
    use causafera_persistence::{DurabilityMode, HistoryStore};
    use causafera_types::{
        EventId, EventKindId, SimulationTime, StateObjectKindId, StatePropertyId, TraceId,
    };

    use super::*;
    use crate::{Runtime, RuntimeConfig};

    #[derive(Debug)]
    struct RuntimeSyntheticDecoder;

    impl EvidenceDecoder for RuntimeSyntheticDecoder {
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

    #[test]
    fn runtime_registry_resolves_hydrology_and_keeps_unknown_evidence_fail_closed() {
        // Given: the registry used by every runtime-owned temporary HistoryStore.
        let registry = runtime_history_decoder_registry().expect("hydrology registry");
        let record = |schema_id, criticality| {
            let payload = vec![0_u8];
            DomainEvidenceRecord::new(
                1,
                schema_id,
                crate::HYDROLOGY_EVIDENCE_VERSION,
                criticality,
                SimulationTime::new(1),
                Vec::new(),
                b"runtime/test".to_vec(),
                payload.clone(),
                StateFingerprint::new(*blake3::hash(&payload).as_bytes()),
            )
            .expect("structurally valid evidence")
        };
        let malformed_hydrology = record(
            crate::HYDROLOGY_BOOTSTRAP_EVIDENCE_SCHEMA,
            Criticality::ReplayRequired,
        );
        let unknown_required = record(
            EvidenceSchemaId::new(0x0014_18ff),
            Criticality::ReplayRequired,
        );
        let unknown_optional = record(
            EvidenceSchemaId::new(0x0014_18ff),
            Criticality::ExplanationOnly,
        );

        // When: known and unknown records cross the live bootstrap registry.
        let known_result = registry.resolve(&malformed_hydrology);
        let required_result = registry.resolve(&unknown_required);
        let optional_result = registry.resolve(&unknown_optional);

        // Then: hydrology reaches its decoder; unknown required fails; optional stays raw.
        assert!(matches!(
            known_result,
            Err(HistoryError::InvalidOffset { .. })
        ));
        assert!(matches!(
            required_result,
            Err(HistoryError::UnsupportedReplaySchema { .. })
        ));
        assert_eq!(
            optional_result,
            Ok(EvidenceResolution::Raw(unknown_optional))
        );
    }

    #[test]
    fn history_decoder_registry_is_injected_before_temporary_store_append() {
        // Given: runtime construction receives a caller-owned typed decoder registry.
        let mut registry = EvidenceDecoderRegistry::new();
        registry
            .register_typed(RuntimeSyntheticDecoder)
            .expect("unique runtime synthetic decoder");
        let registry = Arc::new(registry);
        let (mut store, root) = temporary_history_store_with_registry(29, Arc::clone(&registry))
            .expect("temporary store with decoder registry");
        let _cleanup = TemporaryHistoryRoot(root.clone());
        let payload = b"runtime-required".to_vec();
        let record = HistoryRecord::Evidence(
            DomainEvidenceRecord::new(
                1,
                EvidenceSchemaId::new(0xDEC0DE),
                EvidenceSchemaVersion::new(1, 0),
                Criticality::ReplayRequired,
                SimulationTime::new(1),
                Vec::new(),
                b"runtime/test".to_vec(),
                payload.clone(),
                StateFingerprint::new(*blake3::hash(&payload).as_bytes()),
            )
            .expect("valid runtime replay-required evidence"),
        );

        // When: append, durability, reopen, and indexed read all use that registry.
        store
            .append_tick(vec![record.clone()])
            .expect("registered append");
        store.flush_durable().expect("registered durable flush");
        drop(store);
        let reopened =
            HistoryStore::open_with_registry(&root, registry).expect("registered runtime reopen");
        let page = reopened
            .durable_reader()
            .expect("registered runtime reader")
            .records(HistoryQuery::new(0, 4, 4096).expect("bounded query"))
            .expect("registered runtime read");

        // Then: no second registration transaction is needed after construction.
        assert_eq!(page.records(), std::slice::from_ref(&record));
    }

    #[test]
    fn history_txn_reads_staged_causal_records_before_publication() {
        let effect = CausalEffect::new(
            CausalTarget::new(StateObjectKindId::new(8), 9, StatePropertyId::new(10)),
            StateFingerprint::new([1; 32]),
            StateFingerprint::new([2; 32]),
        )
        .expect("effect is valid");
        let root = OwnedCausalRecord::new(
            0,
            EventId::new(0),
            TraceId::new(0),
            SimulationTime::new(0),
            Phase::Physics,
            EventKindId::new(6),
            Vec::new(),
            vec![effect],
        )
        .expect("root is valid");
        let record = OwnedCausalRecord::new(
            1,
            EventId::new(1),
            TraceId::new(1),
            SimulationTime::new(1),
            Phase::Physics,
            EventKindId::new(7),
            vec![TraceId::new(0)],
            vec![effect],
        )
        .expect("record is valid");
        let reader = Arc::new(
            RuntimeHistoryReader::new_for_test([0; 16], vec![root]).expect("reader is valid"),
        );
        let mut transaction = HistoryTxn {
            reader,
            generation: 1,
            first_trace: 1,
            first_event: 1,
            staged: Vec::new(),
            evidence: Vec::new(),
        };
        transaction
            .stage_causal_records(std::slice::from_ref(&record))
            .expect("staging is valid");
        assert_eq!(
            transaction.staged_event(record.trace_id()).expect("lookup"),
            Some(record)
        );
    }

    #[test]
    fn history_txn_stages_typed_evidence_and_emits_an_evidence_frame() {
        let effect = CausalEffect::new(
            CausalTarget::new(StateObjectKindId::new(8), 9, StatePropertyId::new(10)),
            StateFingerprint::new([1; 32]),
            StateFingerprint::new([2; 32]),
        )
        .expect("effect is valid");
        let root = OwnedCausalRecord::new(
            0,
            EventId::new(0),
            TraceId::new(0),
            SimulationTime::new(0),
            Phase::Physics,
            EventKindId::new(6),
            Vec::new(),
            vec![effect],
        )
        .expect("root is valid");
        let payload = b"typed-evidence".to_vec();
        let evidence = DomainEvidenceRecord::new(
            1,
            EvidenceSchemaId::new(77),
            EvidenceSchemaVersion::new(1, 0),
            Criticality::ReplayRequired,
            SimulationTime::new(1),
            vec![root.trace_id()],
            b"hydrology/cell/0".to_vec(),
            payload.clone(),
            StateFingerprint::new(*blake3::hash(&payload).as_bytes()),
        )
        .expect("evidence is valid");
        let reader = Arc::new(
            RuntimeHistoryReader::new_for_test([0; 16], vec![root]).expect("reader is valid"),
        );
        let mut transaction = HistoryTxn {
            reader,
            generation: 1,
            first_trace: 1,
            first_event: 1,
            staged: Vec::new(),
            evidence: Vec::new(),
        };
        transaction
            .stage_evidence(std::slice::from_ref(&evidence))
            .expect("evidence staging is valid");
        let page = transaction
            .staged_evidence(
                evidence.schema_id(),
                evidence.canonical_key(),
                HistoryQuery::new(0, 4, 4096).expect("query is valid"),
            )
            .expect("staged evidence lookup is valid");
        let frame = transaction
            .prepare(&CausalTraceStore::new())
            .expect("evidence frame is valid");
        assert_eq!(page.records(), std::slice::from_ref(&evidence));
        assert!(matches!(
            frame.records(),
            [HistoryRecord::Evidence(record)] if record == &evidence
        ));
        println!(
            "GENERIC_EVIDENCE_TXN staged_visible_before_publish=true frame_records={} canonical_key={:?}",
            frame.records().len(),
            evidence.canonical_key()
        );
    }

    #[test]
    fn history_txn_rejects_duplicate_evidence_keys_within_one_stage_call() {
        // Given: two otherwise-valid records with the same schema/key transaction identity.
        let payload = b"canonical".to_vec();
        let record = DomainEvidenceRecord::new(
            0,
            EvidenceSchemaId::new(77),
            EvidenceSchemaVersion::new(1, 0),
            Criticality::ExplanationOnly,
            SimulationTime::new(0),
            Vec::new(),
            b"shared-key".to_vec(),
            payload.clone(),
            StateFingerprint::new(*blake3::hash(&payload).as_bytes()),
        )
        .expect("evidence is valid");
        let reader = Arc::new(
            RuntimeHistoryReader::new_for_test([0; 16], Vec::new()).expect("reader is valid"),
        );
        let mut transaction = HistoryTxn {
            reader,
            generation: 1,
            first_trace: 0,
            first_event: 0,
            staged: Vec::new(),
            evidence: Vec::new(),
        };

        // When: both duplicate identities are staged atomically in one call.
        let error = transaction
            .stage_evidence(&[record.clone(), record])
            .expect_err("duplicate evidence keys must fail closed");

        // Then: the entire call is rejected and publishes no transaction-local write.
        assert!(matches!(
            error,
            RuntimeError::CausalCommit(CausalCommitError::HistoryRead(
                HistoryError::DuplicateRecordKey
            ))
        ));
        assert!(transaction.evidence.is_empty());
        println!("GENERIC_EVIDENCE_TXN duplicate_batch=rejected atomic=true");
    }

    #[test]
    fn runtime_history_handoff_publishes_to_a_real_store_and_flushes_its_reader() {
        let effect = CausalEffect::new(
            CausalTarget::new(StateObjectKindId::new(8), 9, StatePropertyId::new(10)),
            StateFingerprint::new([1; 32]),
            StateFingerprint::new([2; 32]),
        )
        .expect("effect is valid");
        let mut traces = CausalTraceStore::new();
        traces
            .commit_batch(
                SimulationTime::new(0),
                Phase::Physics,
                vec![
                    CausalEventProposal::new(
                        EventProposalKey::new(0, 0, 0),
                        EventKindId::new(6),
                        Vec::new(),
                        vec![effect],
                    )
                    .expect("root proposal is valid"),
                ],
            )
            .expect("root commits");
        let bundle = TestBundle::new("handoff");
        let store = HistoryStore::create(
            &bundle.path,
            runtime_history_namespace(7),
            DurabilityMode::GroupCommit,
        )
        .expect("history store creates");
        let (mut coordinator, _) =
            RuntimeHistoryCoordinator::bootstrap_with_store(&traces, 7, traces.hot_budget(), store)
                .expect("coordinator handoff creates");
        traces
            .commit_batch(
                SimulationTime::new(1),
                Phase::Physics,
                vec![
                    CausalEventProposal::new(
                        EventProposalKey::new(0, 0, 1),
                        EventKindId::new(7),
                        vec![TraceId::new(0)],
                        vec![
                            CausalEffect::new(
                                CausalTarget::new(
                                    StateObjectKindId::new(8),
                                    9,
                                    StatePropertyId::new(10),
                                ),
                                StateFingerprint::new([2; 32]),
                                StateFingerprint::new([3; 32]),
                            )
                            .expect("child effect"),
                        ],
                    )
                    .expect("child proposal"),
                ],
            )
            .expect("child commits");
        let transaction = coordinator.begin().expect("transaction begins");
        let frame = transaction.prepare(&traces).expect("frame is valid");
        let _ = coordinator
            .publish(&frame)
            .expect("logical publish succeeds");
        let before_flush = coordinator.durable_generation();
        assert_eq!(coordinator.pending_ticks(), 1);
        coordinator.flush_durable().expect("group flush succeeds");
        assert_eq!(before_flush + 1, coordinator.logical_generation());
        assert_eq!(
            coordinator.durable_generation(),
            coordinator.logical_generation()
        );
        assert_eq!(coordinator.pending_ticks(), 0);
        assert_eq!(
            coordinator.reader.event(TraceId::new(1)).expect("reader"),
            traces.event(TraceId::new(1)).expect("trace event")
        );
        assert!(coordinator.visible_head().raw() > 0);
    }

    #[test]
    fn runtime_history_reader_pin_survives_a_durable_generation_flush() {
        let mut runtime = Runtime::new(RuntimeConfig::new(0xC0FFEE)).expect("runtime bootstraps");
        let live_reader = runtime.history_reader().expect("live reader");
        let initial_anchor = live_reader.anchor().expect("initial anchor");
        let pinned = live_reader.pin_snapshot().expect("production pin succeeds");
        assert_eq!(
            pinned.anchor().expect("pinned anchor").logical_generation(),
            initial_anchor.logical_generation()
        );

        runtime.run_ticks(1).expect("runtime advances one tick");
        let post_tick_head = runtime
            .history_status()
            .expect("post-tick history status")
            .visible_head;
        runtime
            .export_bounded_envelope()
            .expect("force a durable history generation");

        let updated_reader = runtime.history_reader().expect("updated reader");
        let updated_anchor = updated_reader.anchor().expect("updated anchor");
        assert!(
            updated_anchor.logical_generation() > initial_anchor.logical_generation(),
            "a tick must publish a new logical generation"
        );
        assert!(
            updated_anchor.durable_generation() > initial_anchor.durable_generation(),
            "bounded export must force the new durable generation"
        );
        assert_eq!(
            pinned.anchor().expect("pinned anchor remains fixed"),
            initial_anchor
        );
        assert_eq!(
            pinned.event(post_tick_head).expect("pinned lookup"),
            None,
            "pinned generation must not expose the post-pin event"
        );
        assert!(
            updated_reader
                .event(post_tick_head)
                .expect("updated lookup")
                .is_some(),
            "live reader must expose the flushed event"
        );
    }

    #[test]
    fn bounded_bootstrap_advances_generation_without_empty_frame() {
        // Given
        let effect = CausalEffect::new(
            CausalTarget::new(StateObjectKindId::new(8), 9, StatePropertyId::new(10)),
            StateFingerprint::new([1; 32]),
            StateFingerprint::new([2; 32]),
        )
        .expect("effect is valid");
        let mut traces = CausalTraceStore::new();
        traces
            .commit_batch(
                SimulationTime::new(0),
                Phase::Physics,
                vec![
                    CausalEventProposal::new(
                        EventProposalKey::new(0, 0, 0),
                        EventKindId::new(6),
                        Vec::new(),
                        vec![effect],
                    )
                    .expect("root proposal is valid"),
                ],
            )
            .expect("root commits");
        let bundle = TestBundle::new("bounded-generation-gap");
        let store = HistoryStore::create(
            &bundle.path,
            runtime_history_namespace(17),
            DurabilityMode::EveryTick,
        )
        .expect("history store creates");

        // When
        let (coordinator, _) = RuntimeHistoryCoordinator::bootstrap_with_store_and_cleanup(
            &traces,
            17,
            traces.hot_budget(),
            store,
            None,
            Some(3),
        )
        .expect("bounded generation gap advances without an empty frame");

        // Then
        let status = coordinator.backend_status().expect("store status");
        assert_eq!(status.logical_generation, 3);
        assert_eq!(status.durable_generation, 3);
        assert_eq!(status.catalog_counts.frames, 1);
        assert_eq!(status.logical_records, 1);
        assert_eq!(status.durable_records, 1);
    }

    struct TestBundle {
        path: PathBuf,
    }

    impl TestBundle {
        fn new(name: &str) -> Self {
            static NEXT: AtomicU64 = AtomicU64::new(0);
            let ordinal = NEXT.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "causafera-runtime-history-{name}-{}-{ordinal}",
                std::process::id()
            ));
            std::fs::create_dir(&path).expect("test bundle creates");
            Self { path }
        }
    }

    impl Drop for TestBundle {
        fn drop(&mut self) {
            std::fs::remove_dir_all(&self.path).expect("test bundle removes");
        }
    }
}
