use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use causafera_core::{
    CausalDagBatchLimits, CausalDagCommitError, CausalEffect, CausalEventDagCause,
    CausalEventDagProposal, CausalEventProposalKey, CausalHistoryReader, CausalHistoryTxnReader,
    CausalTarget, CausalTraceStore, Criticality, DecodedEvidence, DomainEvidenceRecord,
    EvidenceDecoder, EvidenceResolution, EvidenceSchemaId, EvidenceSchemaVersion, HistoryAnchor,
    HistoryAnchorFields, HistoryDurability, HistoryError, HistoryFrontier, HistoryPage,
    HistoryPageItem, HistoryQuery, HistoryRecord, MAX_CAUSES_PER_EVENT,
    MAX_CHECKPOINT_DECODED_BYTES, MAX_EFFECTS_PER_EVENT, MAX_FRAME_DECODED_BYTES,
    MAX_HISTORY_RECORDS_PER_FRAME, MAX_QUERY_PAGE_BYTES, MAX_QUERY_PAGE_RECORDS, OwnedCausalRecord,
    Phase, StateFingerprint, canonical_history_records, checked_history_range,
    ensure_history_consumed, validate_causal_batch, validate_checkpoint_decoded_size,
    validate_frame_shape,
};
use causafera_types::{
    EventId, EventKindId, SimulationTime, StateObjectKindId, StatePropertyId, TraceId,
};

fn effect(object_id: u64) -> CausalEffect {
    CausalEffect::new(
        CausalTarget::new(
            StateObjectKindId::new(3),
            object_id,
            StatePropertyId::new(5),
        ),
        StateFingerprint::new([7; 32]),
        StateFingerprint::new([9; 32]),
    )
    .unwrap()
}

#[test]
fn provenance_same_tick_dag_is_visible_in_canonical_order_after_commit() {
    // Given: two same-tick events where the second names the first before either has an ID.
    let parent_key = CausalEventProposalKey::new(1, 9, b"parent", 0).unwrap();
    let child_key = CausalEventProposalKey::new(2, 9, b"child", 0).unwrap();
    let parent = CausalEventDagProposal::new(
        parent_key.clone(),
        EventKindId::new(13),
        vec![],
        vec![effect(1)],
    )
    .unwrap();
    let child = CausalEventDagProposal::new(
        child_key.clone(),
        EventKindId::new(13),
        vec![CausalEventDagCause::Local(parent_key.clone())],
        vec![effect(2)],
    )
    .unwrap();
    let mut store = CausalTraceStore::new();

    // When: producer insertion order is the reverse of causal order.
    let resolved = store
        .commit_dag_batch(
            SimulationTime::new(11),
            Phase::Physics,
            vec![child, parent],
            CausalDagBatchLimits {
                max_causes_per_event: 4,
                max_effects_per_event: 4,
            },
        )
        .unwrap();

    // Then: deterministic allocation and immediate reads expose the resolved edge.
    let parent_trace = resolved[&parent_key];
    let child_trace = resolved[&child_key];
    println!(
        "deterministic order marker: parent={} child={}; read-your-writes marker",
        parent_trace.raw(),
        child_trace.raw()
    );
    assert_eq!(
        (parent_trace, child_trace),
        (TraceId::new(0), TraceId::new(1))
    );
    assert_eq!(
        store.event(child_trace).unwrap().unwrap().causes(),
        &[parent_trace]
    );
    assert_eq!(
        store
            .children(
                parent_trace,
                HistoryQuery::new(0, MAX_QUERY_PAGE_RECORDS, MAX_QUERY_PAGE_BYTES).unwrap(),
            )
            .unwrap()
            .records(),
        &[child_trace]
    );
}

fn causal(sequence: u64, event_id: u64, trace_id: u64, causes: Vec<TraceId>) -> OwnedCausalRecord {
    OwnedCausalRecord::new(
        sequence,
        EventId::new(event_id),
        TraceId::new(trace_id),
        SimulationTime::new(11),
        Phase::Physics,
        EventKindId::new(13),
        causes,
        vec![effect(event_id)],
    )
    .unwrap()
}

fn evidence(
    sequence: u64,
    schema: u64,
    version: EvidenceSchemaVersion,
    criticality: Criticality,
    key: &[u8],
    payload: &[u8],
) -> DomainEvidenceRecord {
    DomainEvidenceRecord::new(
        sequence,
        EvidenceSchemaId::new(schema),
        version,
        criticality,
        SimulationTime::new(11),
        vec![TraceId::new(2)],
        key.to_vec(),
        payload.to_vec(),
        StateFingerprint::new([schema as u8; 32]),
    )
    .unwrap()
}

#[test]
fn provenance_records_have_deterministic_order_regardless_of_input_order() {
    let earlier_evidence = HistoryRecord::Evidence(evidence(
        3,
        21,
        EvidenceSchemaVersion::new(1, 0),
        Criticality::ExplanationOnly,
        b"earlier",
        b"earlier evidence",
    ));
    let event = HistoryRecord::Causal(causal(4, 2, 2, vec![]));
    let first = HistoryRecord::Evidence(evidence(
        4,
        20,
        EvidenceSchemaVersion::new(1, 0),
        Criticality::ExplanationOnly,
        b"a",
        b"first",
    ));
    let second = HistoryRecord::Evidence(evidence(
        4,
        20,
        EvidenceSchemaVersion::new(1, 0),
        Criticality::ExplanationOnly,
        b"b",
        b"second",
    ));

    let ordered = canonical_history_records(vec![
        second.clone(),
        event.clone(),
        earlier_evidence.clone(),
        first.clone(),
    ])
    .unwrap();
    let reversed = canonical_history_records(vec![
        first.clone(),
        earlier_evidence.clone(),
        event.clone(),
        second.clone(),
    ])
    .unwrap();

    assert_eq!(ordered, vec![earlier_evidence, event, first, second]);
    assert_eq!(ordered, reversed);
}

#[test]
fn provenance_evidence_order_uses_canonical_tuple_and_rejects_noncanonical_causal_input() {
    let key_a = HistoryRecord::Evidence(
        DomainEvidenceRecord::new(
            9,
            EvidenceSchemaId::new(99),
            EvidenceSchemaVersion::new(u16::MAX, u16::MAX),
            Criticality::ReplayRequired,
            SimulationTime::new(u64::MAX),
            vec![TraceId::new(99)],
            b"a".to_vec(),
            vec![u8::MAX],
            StateFingerprint::new([u8::MAX; 32]),
        )
        .unwrap(),
    );
    let key_b = HistoryRecord::Evidence(
        DomainEvidenceRecord::new(
            9,
            EvidenceSchemaId::new(1),
            EvidenceSchemaVersion::new(0, 0),
            Criticality::ExplanationOnly,
            SimulationTime::new(0),
            vec![TraceId::new(1)],
            b"b".to_vec(),
            vec![0],
            StateFingerprint::new([0; 32]),
        )
        .unwrap(),
    );
    let low_digest = HistoryRecord::Evidence(
        DomainEvidenceRecord::new(
            9,
            EvidenceSchemaId::new(7),
            EvidenceSchemaVersion::new(9, 9),
            Criticality::ExplanationOnly,
            SimulationTime::new(900),
            vec![TraceId::new(90)],
            b"same-key".to_vec(),
            vec![u8::MAX],
            StateFingerprint::new([1; 32]),
        )
        .unwrap(),
    );
    let high_digest = HistoryRecord::Evidence(
        DomainEvidenceRecord::new(
            9,
            EvidenceSchemaId::new(7),
            EvidenceSchemaVersion::new(1, 0),
            Criticality::ReplayRequired,
            SimulationTime::new(1),
            vec![TraceId::new(1)],
            b"same-key".to_vec(),
            vec![0],
            StateFingerprint::new([2; 32]),
        )
        .unwrap(),
    );

    let ordered = canonical_history_records(vec![
        high_digest.clone(),
        key_b.clone(),
        low_digest.clone(),
        key_a.clone(),
    ])
    .unwrap();
    assert_eq!(
        ordered,
        vec![
            key_a.clone(),
            key_b,
            low_digest.clone(),
            high_digest.clone()
        ]
    );
    assert_eq!(
        canonical_history_records(vec![key_a.clone(), low_digest.clone(), high_digest.clone()])
            .unwrap(),
        vec![key_a.clone(), low_digest, high_digest]
    );
    assert_eq!(
        canonical_history_records(vec![key_a.clone(), key_a]),
        Err(HistoryError::DuplicateRecordKey)
    );

    let noncanonical = [causal(2, 2, 2, vec![]), causal(1, 1, 1, vec![])];
    assert_eq!(
        validate_causal_batch(&noncanonical, &[]),
        Err(HistoryError::NonCanonicalOrder { index: 1 })
    );
}

#[test]
fn provenance_causal_fields_match_write_trace_event_fingerprint_words_without_length_words() {
    let record = causal(8, 1, 2, vec![TraceId::new(0)]);
    let fields = record.canonical_field_bytes().unwrap();
    let expected_words = [
        1_u64,
        2,
        11,
        3,
        13,
        1,
        0,
        1,
        3,
        1,
        5,
        0x0707_0707_0707_0707,
        0x0707_0707_0707_0707,
        0x0707_0707_0707_0707,
        0x0707_0707_0707_0707,
        0x0909_0909_0909_0909,
        0x0909_0909_0909_0909,
        0x0909_0909_0909_0909,
        0x0909_0909_0909_0909,
    ];
    let mut expected = Vec::with_capacity(expected_words.len() * 8);
    for word in expected_words {
        expected.extend_from_slice(&word.to_le_bytes());
    }
    println!("write_trace_event u64 sequence: {expected_words:?}");

    assert_eq!(fields, expected);
}

#[test]
fn provenance_rejects_bad_cause_effect_and_parent_shapes() {
    assert_eq!(
        OwnedCausalRecord::new(
            0,
            EventId::new(0),
            TraceId::new(0),
            SimulationTime::new(0),
            Phase::Physics,
            EventKindId::new(1),
            vec![],
            vec![],
        ),
        Err(HistoryError::NoEffects)
    );
    assert_eq!(
        OwnedCausalRecord::new(
            0,
            EventId::new(0),
            TraceId::new(0),
            SimulationTime::new(0),
            Phase::Physics,
            EventKindId::new(1),
            vec![TraceId::new(1), TraceId::new(1)],
            vec![effect(0)],
        ),
        Err(HistoryError::CausesNotStrictlyOrdered { index: 1 })
    );
    assert_eq!(
        OwnedCausalRecord::new(
            0,
            EventId::new(0),
            TraceId::new(0),
            SimulationTime::new(0),
            Phase::Physics,
            EventKindId::new(1),
            vec![],
            vec![effect(1), effect(1)],
        ),
        Err(HistoryError::EffectsNotStrictlyOrdered { index: 1 })
    );
    assert!(matches!(
        OwnedCausalRecord::new(
            0,
            EventId::new(0),
            TraceId::new(0),
            SimulationTime::new(0),
            Phase::Physics,
            EventKindId::new(1),
            vec![TraceId::new(2), TraceId::new(1)],
            vec![effect(0)],
        ),
        Err(HistoryError::CausesNotStrictlyOrdered { .. })
    ));
    assert!(matches!(
        OwnedCausalRecord::new(
            0,
            EventId::new(0),
            TraceId::new(0),
            SimulationTime::new(0),
            Phase::Physics,
            EventKindId::new(1),
            vec![],
            vec![effect(2), effect(1)],
        ),
        Err(HistoryError::EffectsNotStrictlyOrdered { .. })
    ));
    assert_eq!(
        validate_causal_batch(&[causal(0, 0, 1, vec![TraceId::new(9)])], &[]),
        Err(HistoryError::UnknownParent {
            trace_id: TraceId::new(1),
            parent: TraceId::new(9),
        })
    );
}

#[test]
fn provenance_rejects_duplicate_evidence_anchors_and_history_record_keys() {
    assert_eq!(
        DomainEvidenceRecord::new(
            0,
            EvidenceSchemaId::new(1),
            EvidenceSchemaVersion::new(1, 0),
            Criticality::ReplayRequired,
            SimulationTime::new(0),
            vec![TraceId::new(2), TraceId::new(2)],
            b"key".to_vec(),
            b"payload".to_vec(),
            StateFingerprint::new([1; 32]),
        ),
        Err(HistoryError::AnchorsNotStrictlyOrdered { index: 1 })
    );

    let duplicate = HistoryRecord::Causal(causal(1, 1, 1, vec![]));
    assert_eq!(
        canonical_history_records(vec![duplicate.clone(), duplicate]),
        Err(HistoryError::DuplicateRecordKey)
    );
}

#[test]
fn provenance_rejects_duplicate_ids_and_accepts_prior_or_earlier_parents() {
    let prior = causal(0, 4, 4, vec![]);
    let child = causal(1, 5, 5, vec![TraceId::new(4)]);
    assert_eq!(
        validate_causal_batch(std::slice::from_ref(&child), std::slice::from_ref(&prior)),
        Ok(())
    );
    assert_eq!(validate_causal_batch(&[prior, child], &[]), Ok(()));

    let duplicates = [causal(0, 1, 1, vec![]), causal(1, 1, 2, vec![])];
    assert_eq!(
        validate_causal_batch(&duplicates, &[]),
        Err(HistoryError::DuplicateEventId(EventId::new(1)))
    );
    let duplicate_traces = [causal(0, 1, 1, vec![]), causal(1, 2, 1, vec![])];
    assert_eq!(
        validate_causal_batch(&duplicate_traces, &[]),
        Err(HistoryError::DuplicateTraceId(TraceId::new(1)))
    );
    let noncanonical = [causal(2, 2, 2, vec![]), causal(1, 1, 1, vec![])];
    assert_eq!(
        validate_causal_batch(&noncanonical, &[]),
        Err(HistoryError::NonCanonicalOrder { index: 1 })
    );
}

#[test]
fn provenance_exact_hard_bounds_accept_limit_and_reject_limit_plus_one() {
    assert_eq!(
        validate_frame_shape(MAX_HISTORY_RECORDS_PER_FRAME, MAX_FRAME_DECODED_BYTES),
        Ok(())
    );
    assert!(matches!(
        validate_frame_shape(MAX_HISTORY_RECORDS_PER_FRAME + 1, 0),
        Err(HistoryError::RecordLimitExceeded { .. })
    ));
    assert!(matches!(
        validate_frame_shape(0, MAX_FRAME_DECODED_BYTES + 1),
        Err(HistoryError::DecodedSizeLimitExceeded { .. })
    ));
    assert_eq!(
        validate_checkpoint_decoded_size(MAX_CHECKPOINT_DECODED_BYTES),
        Ok(())
    );
    assert!(validate_checkpoint_decoded_size(MAX_CHECKPOINT_DECODED_BYTES + 1).is_err());

    let causes = (0..MAX_CAUSES_PER_EVENT)
        .map(|id| TraceId::new(id as u64))
        .collect();
    assert!(
        OwnedCausalRecord::new(
            0,
            EventId::new(0),
            TraceId::new(MAX_CAUSES_PER_EVENT as u64),
            SimulationTime::new(0),
            Phase::Physics,
            EventKindId::new(1),
            causes,
            vec![effect(0)],
        )
        .is_ok()
    );
    let too_many_causes = (0..=MAX_CAUSES_PER_EVENT)
        .map(|id| TraceId::new(id as u64))
        .collect();
    assert!(matches!(
        OwnedCausalRecord::new(
            0,
            EventId::new(0),
            TraceId::new((MAX_CAUSES_PER_EVENT + 1) as u64),
            SimulationTime::new(0),
            Phase::Physics,
            EventKindId::new(1),
            too_many_causes,
            vec![effect(0)],
        ),
        Err(HistoryError::CauseLimitExceeded { .. })
    ));

    let effects = (0..MAX_EFFECTS_PER_EVENT)
        .map(|id| effect(id as u64))
        .collect();
    assert!(
        OwnedCausalRecord::new(
            0,
            EventId::new(0),
            TraceId::new(0),
            SimulationTime::new(0),
            Phase::Physics,
            EventKindId::new(1),
            vec![],
            effects,
        )
        .is_ok()
    );
    let too_many_effects = (0..=MAX_EFFECTS_PER_EVENT)
        .map(|id| effect(id as u64))
        .collect();
    assert!(matches!(
        OwnedCausalRecord::new(
            0,
            EventId::new(0),
            TraceId::new(0),
            SimulationTime::new(0),
            Phase::Physics,
            EventKindId::new(1),
            vec![],
            too_many_effects,
        ),
        Err(HistoryError::EffectLimitExceeded { .. })
    ));
}

#[test]
fn provenance_checked_ranges_reject_overflow_offsets_truncation_and_trailing_bytes() {
    assert_eq!(checked_history_range(4, 3, 7), Ok(4..7));
    assert!(matches!(
        checked_history_range(usize::MAX, 2, usize::MAX),
        Err(HistoryError::ArithmeticOverflow)
    ));
    assert!(matches!(
        checked_history_range(5, 3, 7),
        Err(HistoryError::InvalidOffset { .. })
    ));
    assert!(matches!(
        ensure_history_consumed(6, 7),
        Err(HistoryError::TrailingBytes { .. })
    ));
}

#[derive(Debug)]
struct IncrementingDecoder;

impl EvidenceDecoder for IncrementingDecoder {
    fn source_schema(&self) -> (EvidenceSchemaId, EvidenceSchemaVersion) {
        (EvidenceSchemaId::new(70), EvidenceSchemaVersion::new(1, 0))
    }

    fn target_version(&self) -> EvidenceSchemaVersion {
        EvidenceSchemaVersion::new(2, 0)
    }

    fn decode_or_upcast(&self, payload: &[u8]) -> Result<Vec<u8>, HistoryError> {
        payload
            .iter()
            .map(|byte| byte.checked_add(1).ok_or(HistoryError::ArithmeticOverflow))
            .collect()
    }
}

#[test]
fn causal_history_unknown_explanation_only_preserves_raw_bytes() {
    let record = evidence(
        0,
        99,
        EvidenceSchemaVersion::new(9, 4),
        Criticality::ExplanationOnly,
        b"raw-key",
        &[0, 1, 255, 7],
    );
    let registry = causafera_core::EvidenceDecoderRegistry::new();

    assert_eq!(
        registry.resolve(&record).unwrap(),
        EvidenceResolution::Raw(record)
    );
    println!("GENERIC_EVIDENCE_UNKNOWN criticality=ExplanationOnly resolution=Raw");
}

#[test]
fn causal_history_unknown_replay_required_fails_closed() {
    let record = evidence(
        0,
        99,
        EvidenceSchemaVersion::new(9, 4),
        Criticality::ReplayRequired,
        b"required",
        &[4, 5],
    );
    let registry = causafera_core::EvidenceDecoderRegistry::new();

    assert_eq!(
        registry.resolve(&record),
        Err(HistoryError::UnsupportedReplaySchema {
            schema_id: EvidenceSchemaId::new(99),
            version: EvidenceSchemaVersion::new(9, 4),
        })
    );
    println!("GENERIC_EVIDENCE_UNKNOWN criticality=ReplayRequired error=UnsupportedReplaySchema");
}

#[test]
fn causal_history_registry_selects_exact_typed_decoder_and_upcaster() {
    let mut registry = causafera_core::EvidenceDecoderRegistry::new();
    registry.register_typed(IncrementingDecoder).unwrap();
    let record = evidence(
        0,
        70,
        EvidenceSchemaVersion::new(1, 0),
        Criticality::ReplayRequired,
        b"known",
        &[1, 3],
    );

    assert_eq!(
        registry.resolve(&record).unwrap(),
        EvidenceResolution::Decoded(DecodedEvidence {
            schema_id: EvidenceSchemaId::new(70),
            version: EvidenceSchemaVersion::new(2, 0),
            canonical_payload: vec![2, 4],
        })
    );
    assert!(matches!(
        registry.register(Box::new(IncrementingDecoder)),
        Err(HistoryError::DuplicateDecoder { .. })
    ));
    println!("GENERIC_EVIDENCE_REGISTRY registration=typed decode=upcast duplicate=rejected");
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct SizedItem(usize);

impl HistoryPageItem for SizedItem {
    fn decoded_size(&self) -> usize {
        self.0
    }
}

#[test]
fn provenance_query_and_page_enforce_record_and_byte_limits() {
    assert!(HistoryQuery::new(0, MAX_QUERY_PAGE_RECORDS, MAX_QUERY_PAGE_BYTES).is_ok());
    assert!(matches!(
        HistoryQuery::new(0, MAX_QUERY_PAGE_RECORDS + 1, 1),
        Err(HistoryError::QueryRecordLimitExceeded { .. })
    ));
    assert!(matches!(
        HistoryQuery::new(0, 1, MAX_QUERY_PAGE_BYTES + 1),
        Err(HistoryError::QueryByteLimitExceeded { .. })
    ));
    assert_eq!(
        HistoryQuery::new(u64::MAX, 1, 1),
        Err(HistoryError::ArithmeticOverflow)
    );
    let query = HistoryQuery::new(7, 2, 8).unwrap();
    assert!(HistoryPage::new(vec![SizedItem(4), SizedItem(4)], None, query).is_ok());
    assert!(matches!(
        HistoryPage::new(vec![SizedItem(4), SizedItem(5)], None, query),
        Err(HistoryError::QueryByteLimitExceeded { .. })
    ));

    let max_page_query =
        HistoryQuery::new(0, MAX_QUERY_PAGE_RECORDS, MAX_QUERY_PAGE_BYTES).unwrap();
    assert!(
        HistoryPage::new(
            vec![SizedItem(0); MAX_QUERY_PAGE_RECORDS],
            None,
            max_page_query,
        )
        .is_ok()
    );
    assert!(matches!(
        HistoryPage::new(
            vec![SizedItem(0); MAX_QUERY_PAGE_RECORDS + 1],
            None,
            max_page_query,
        ),
        Err(HistoryError::QueryRecordLimitExceeded { .. })
    ));
}

#[derive(Debug)]
struct ReaderProbe {
    anchor: HistoryAnchor,
    record: OwnedCausalRecord,
}

impl CausalHistoryReader for ReaderProbe {
    fn anchor(&self) -> Result<HistoryAnchor, HistoryError> {
        Ok(self.anchor.clone())
    }

    fn event(&self, trace_id: TraceId) -> Result<Option<OwnedCausalRecord>, HistoryError> {
        Ok((trace_id == self.record.trace_id()).then(|| self.record.clone()))
    }

    fn records(&self, query: HistoryQuery) -> Result<HistoryPage<HistoryRecord>, HistoryError> {
        HistoryPage::new(
            vec![HistoryRecord::Causal(self.record.clone())],
            None,
            query,
        )
    }
}

impl CausalHistoryTxnReader for ReaderProbe {
    fn staged_event(&self, trace_id: TraceId) -> Result<Option<OwnedCausalRecord>, HistoryError> {
        self.event(trace_id)
    }
}

#[test]
fn provenance_reader_anchor_page_and_transaction_seams_are_object_safe() {
    let frontier = HistoryFrontier::new(
        12,
        EventId::new(8),
        TraceId::new(9),
        SimulationTime::new(10),
    );
    let fields = HistoryAnchorFields {
        namespace: [1; 16],
        logical_generation: 12,
        durable_generation: 11,
        checkpoint_digest: [2; 32],
        frontier,
        logical_digest: [3; 32],
        storage_root: [4; 32],
        physical_digest_schema: 9,
        history_digest_schema: 9,
        evidence_manifest_digest: [5; 32],
        durability: HistoryDurability::GroupCommit {
            max_ticks: 32,
            max_wall_time_ms: 250,
        },
    };
    let anchor = HistoryAnchor::new(fields.clone()).unwrap();
    let probe = ReaderProbe {
        anchor: anchor.clone(),
        record: causal(1, 8, 9, vec![]),
    };
    let reader: &dyn CausalHistoryReader = &probe;
    let transaction: &dyn CausalHistoryTxnReader = &probe;

    assert_eq!(reader.anchor().unwrap(), anchor);
    assert_eq!(
        reader.event(TraceId::new(9)).unwrap(),
        Some(probe.record.clone())
    );
    assert_eq!(
        transaction.staged_event(TraceId::new(9)).unwrap(),
        Some(probe.record)
    );
    assert_eq!(
        HistoryAnchor::new(HistoryAnchorFields {
            logical_generation: 10,
            durable_generation: 11,
            ..fields
        }),
        Err(HistoryError::DurableGenerationAhead {
            logical: 10,
            durable: 11,
        })
    );
}

#[derive(Debug)]
struct IndexedReaderProbe {
    anchor: HistoryAnchor,
    records: Vec<OwnedCausalRecord>,
}

#[derive(Debug)]
struct BatchedOverlayProbe {
    anchor: HistoryAnchor,
    records: Vec<OwnedCausalRecord>,
    event_calls: Arc<AtomicUsize>,
    batch_calls: Arc<AtomicUsize>,
}

impl CausalHistoryReader for BatchedOverlayProbe {
    fn anchor(&self) -> Result<HistoryAnchor, HistoryError> {
        Ok(self.anchor.clone())
    }

    fn event(&self, trace_id: TraceId) -> Result<Option<OwnedCausalRecord>, HistoryError> {
        self.event_calls.fetch_add(1, Ordering::SeqCst);
        Ok(self
            .records
            .iter()
            .find(|record| record.trace_id() == trace_id)
            .cloned())
    }

    fn records(&self, query: HistoryQuery) -> Result<HistoryPage<HistoryRecord>, HistoryError> {
        let records = self
            .records
            .iter()
            .skip(usize::try_from(query.offset()).map_err(|_| HistoryError::ArithmeticOverflow)?)
            .take(query.max_records())
            .cloned()
            .map(HistoryRecord::Causal)
            .collect();
        HistoryPage::new(records, None, query)
    }

    fn validate_causal_overlay(&self, records: &[OwnedCausalRecord]) -> Result<(), HistoryError> {
        self.batch_calls.fetch_add(1, Ordering::SeqCst);
        for record in records {
            if self
                .records
                .iter()
                .find(|candidate| candidate.trace_id() == record.trace_id())
                != Some(record)
            {
                return Err(HistoryError::ColdRecordMismatch {
                    trace_id: record.trace_id(),
                });
            }
        }
        Ok(())
    }
}

fn batched_overlay_reader(
    records: Vec<OwnedCausalRecord>,
) -> (
    Arc<dyn CausalHistoryReader>,
    Arc<AtomicUsize>,
    Arc<AtomicUsize>,
) {
    let last = records.last().unwrap();
    let anchor = HistoryAnchor::new(HistoryAnchorFields {
        namespace: [1; 16],
        logical_generation: last.commit_sequence(),
        durable_generation: last.commit_sequence(),
        checkpoint_digest: [2; 32],
        frontier: HistoryFrontier::new(
            last.commit_sequence(),
            last.event_id(),
            last.trace_id(),
            last.time(),
        ),
        logical_digest: [3; 32],
        storage_root: [4; 32],
        physical_digest_schema: 9,
        history_digest_schema: 9,
        evidence_manifest_digest: [5; 32],
        durability: HistoryDurability::EveryTick,
    })
    .unwrap();
    let event_calls = Arc::new(AtomicUsize::new(0));
    let batch_calls = Arc::new(AtomicUsize::new(0));
    (
        Arc::new(BatchedOverlayProbe {
            anchor,
            records,
            event_calls: event_calls.clone(),
            batch_calls: batch_calls.clone(),
        }),
        event_calls,
        batch_calls,
    )
}

impl CausalHistoryReader for IndexedReaderProbe {
    fn anchor(&self) -> Result<HistoryAnchor, HistoryError> {
        Ok(self.anchor.clone())
    }

    fn event(&self, trace_id: TraceId) -> Result<Option<OwnedCausalRecord>, HistoryError> {
        Ok(self
            .records
            .iter()
            .find(|record| record.trace_id() == trace_id)
            .cloned())
    }

    fn event_by_id(&self, event_id: EventId) -> Result<Option<OwnedCausalRecord>, HistoryError> {
        Ok(self
            .records
            .iter()
            .find(|record| record.event_id() == event_id)
            .cloned())
    }

    fn records(&self, query: HistoryQuery) -> Result<HistoryPage<HistoryRecord>, HistoryError> {
        let records = self
            .records
            .iter()
            .skip(usize::try_from(query.offset()).map_err(|_| HistoryError::ArithmeticOverflow)?)
            .take(query.max_records())
            .cloned()
            .map(HistoryRecord::Causal)
            .collect();
        HistoryPage::new(records, None, query)
    }

    fn children(
        &self,
        trace_id: TraceId,
        query: HistoryQuery,
    ) -> Result<HistoryPage<TraceId>, HistoryError> {
        let children = self
            .records
            .iter()
            .filter(|record| record.causes().contains(&trace_id))
            .map(OwnedCausalRecord::trace_id)
            .collect();
        HistoryPage::new(children, None, query)
    }

    fn latest_matching_event_before(
        &self,
        target: CausalTarget,
        trace_id: TraceId,
    ) -> Result<Option<OwnedCausalRecord>, HistoryError> {
        Ok(self
            .records
            .iter()
            .rev()
            .find(|record| {
                record.trace_id() < trace_id
                    && record
                        .effects()
                        .iter()
                        .any(|effect| effect.target() == target)
            })
            .cloned())
    }
}

fn indexed_reader(records: Vec<OwnedCausalRecord>) -> Arc<dyn CausalHistoryReader> {
    let last = records.last().unwrap();
    let anchor = HistoryAnchor::new(HistoryAnchorFields {
        namespace: [1; 16],
        logical_generation: last.commit_sequence(),
        durable_generation: last.commit_sequence(),
        checkpoint_digest: [2; 32],
        frontier: HistoryFrontier::new(
            last.commit_sequence(),
            last.event_id(),
            last.trace_id(),
            last.time(),
        ),
        logical_digest: [3; 32],
        storage_root: [4; 32],
        physical_digest_schema: 9,
        history_digest_schema: 9,
        evidence_manifest_digest: [5; 32],
        durability: HistoryDurability::EveryTick,
    })
    .unwrap();
    Arc::new(IndexedReaderProbe { anchor, records })
}

#[test]
fn provenance_cold_overlay_batch_validation_preserves_exact_records() {
    // Given: a cold chain whose effects and parents must remain byte-exact in the hot facade.
    let records = vec![
        causal(1, 20, 30, vec![]),
        causal(2, 21, 31, vec![TraceId::new(30)]),
        causal(3, 22, 32, vec![TraceId::new(31)]),
    ];
    let budget = records.iter().map(HistoryPageItem::decoded_size).sum();
    let (reader, event_calls, batch_calls) = batched_overlay_reader(records.clone());

    // When: the facade validates and installs the complete hot suffix.
    let store = CausalTraceStore::with_cold_reader(reader, budget, records.clone()).unwrap();
    let hot = store
        .iter_hot()
        .map(|record| {
            OwnedCausalRecord::new(
                record.commit_sequence,
                record.event_id,
                record.trace_id,
                record.time,
                record.phase,
                record.kind,
                record.causes.to_vec(),
                record.effects.to_vec(),
            )
            .unwrap()
        })
        .collect::<Vec<_>>();

    // Then: exact records survive, and only the constant frontier lookup uses event().
    assert_eq!(hot, records);
    assert_eq!(event_calls.load(Ordering::SeqCst), 1);
    assert_eq!(batch_calls.load(Ordering::SeqCst), 1);
    println!(
        "cold_overlay exact_records=3 parents_and_effects_equal=true event_calls={} batch_calls={}",
        event_calls.load(Ordering::SeqCst),
        batch_calls.load(Ordering::SeqCst)
    );
}

#[test]
fn provenance_cold_overlay_batch_validation_rejects_mismatch_and_noncanonical_ids() {
    // Given: canonical durable records and one same-ID record with a different effect.
    let records = vec![
        causal(1, 20, 30, vec![]),
        causal(2, 21, 31, vec![TraceId::new(30)]),
    ];
    let mismatched = OwnedCausalRecord::new(
        records[1].commit_sequence(),
        records[1].event_id(),
        records[1].trace_id(),
        records[1].time(),
        records[1].phase(),
        records[1].kind(),
        records[1].causes().to_vec(),
        vec![effect(999)],
    )
    .unwrap();
    let (reader, _, batch_calls) = batched_overlay_reader(records.clone());

    // When: exact cold equality and canonical ID ordering are violated independently.
    let mismatch = CausalTraceStore::with_cold_reader(
        reader.clone(),
        usize::MAX,
        vec![records[0].clone(), mismatched],
    );
    let noncanonical = CausalTraceStore::with_cold_reader(
        reader,
        usize::MAX,
        vec![records[1].clone(), records[0].clone()],
    );

    // Then: both inputs fail closed before any hot facade can be returned.
    assert!(matches!(
        mismatch,
        Err(HistoryError::ColdRecordMismatch { trace_id }) if trace_id == TraceId::new(31)
    ));
    assert_eq!(
        noncanonical,
        Err(HistoryError::NonCanonicalOrder { index: 1 })
    );
    assert_eq!(batch_calls.load(Ordering::SeqCst), 1);
    println!("cold_overlay mismatch_rejected=true noncanonical_rejected=true");
}

#[test]
fn provenance_hot_eviction_keeps_cold_records_queryable_within_the_byte_budget() {
    // Given: three durable records and a cache budget that fits only the newest one.
    let records = vec![
        causal(1, 20, 30, vec![]),
        causal(2, 21, 31, vec![TraceId::new(30)]),
        causal(3, 22, 32, vec![TraceId::new(31)]),
    ];
    let budget = records[2].decoded_size();

    // When: the bounded facade is seeded from the durable reader.
    let store = CausalTraceStore::with_cold_reader(
        indexed_reader(records.clone()),
        budget,
        records.clone(),
    )
    .unwrap();

    // Then: only the newest event remains hot, while evicted records use the cold seam.
    let hot = store.iter_hot().collect::<Vec<_>>();
    println!(
        "hot eviction marker: hot_records={} hot_bytes={} budget={}",
        hot.len(),
        store.hot_bytes(),
        store.hot_budget()
    );
    assert_eq!(hot.len(), 1);
    assert_eq!(hot[0].trace_id, TraceId::new(32));
    assert!(store.hot_bytes() <= store.hot_budget());
    assert_eq!(
        store.event(TraceId::new(30)).unwrap(),
        Some(records[0].clone())
    );
    assert_eq!(
        store.event_by_id(EventId::new(21)).unwrap(),
        Some(records[1].clone())
    );
    assert!(store.cold_reader().is_some());
}

#[test]
fn cold_import_evicts_verified_prefix_without_point_rereads() {
    // Given: an authenticated input larger than the hot cache.
    let records = vec![
        causal(1, 20, 30, vec![]),
        causal(2, 21, 31, vec![TraceId::new(30)]),
        causal(3, 22, 32, vec![TraceId::new(31)]),
    ];
    let budget = records[2].decoded_size();
    let (reader, event_calls, batch_calls) = batched_overlay_reader(records.clone());

    // When: import validates the whole input and retains its bounded suffix.
    let store = CausalTraceStore::with_cold_reader(reader, budget, records).unwrap();

    // Then: eviction reuses the batch proof instead of rereading cold frames.
    assert_eq!(store.len(), 1);
    assert!(store.hot_bytes() <= budget);
    assert_eq!(batch_calls.load(Ordering::SeqCst), 1);
    assert_eq!(
        event_calls.load(Ordering::SeqCst),
        1,
        "only the frontier requires a point lookup"
    );
}

#[test]
fn provenance_facade_delegates_indexed_range_children_and_latest_before_to_cold_reader() {
    // Given: a cold parent/child chain and no seeded hot records.
    let records = vec![
        causal(1, 20, 30, vec![]),
        causal(2, 21, 31, vec![TraceId::new(30)]),
        causal(3, 22, 32, vec![TraceId::new(31)]),
    ];
    let target = records[1].effects()[0].target();
    let store = CausalTraceStore::with_cold_reader(
        indexed_reader(records.clone()),
        records[2].decoded_size(),
        Vec::new(),
    )
    .unwrap();
    let query = HistoryQuery::new(0, 16, 4096).unwrap();

    // When: callers use each explicit indexed operation.
    let range = store.indexed_range(query).unwrap();
    let parents = store.parents(TraceId::new(31), query).unwrap();
    let children = store.children(TraceId::new(30), query).unwrap();
    let latest = store
        .latest_matching_event_before(target, TraceId::new(32))
        .unwrap();

    // Then: the facade returns owned cold records without a whole-history iterator.
    println!(
        "indexed cold-read marker: range={} parents={} children={}",
        range.records().len(),
        parents.records().len(),
        children.records().len()
    );
    assert_eq!(range.records().len(), 3);
    assert_eq!(parents.records(), &[TraceId::new(30)]);
    assert_eq!(children.records(), &[TraceId::new(31)]);
    assert_eq!(latest, Some(records[1].clone()));
    assert_eq!(store.iter_hot().count(), 0);
}

#[test]
fn provenance_transaction_local_dag_writes_read_cold_parents_and_remain_atomic() {
    // Given: a durable parent/child edge and a bounded facade with room for one staged child.
    let parent = causal(8, 40, 50, vec![]);
    let durable_child = causal(9, 41, 51, vec![TraceId::new(50)]);
    let budget = parent.decoded_size() + 8;
    let mut store = CausalTraceStore::with_cold_reader(
        indexed_reader(vec![parent, durable_child]),
        budget,
        Vec::new(),
    )
    .unwrap();
    let child_key = CausalEventProposalKey::new(1, 9, b"child", 0).unwrap();
    let child = CausalEventDagProposal::new(
        child_key.clone(),
        EventKindId::new(13),
        vec![CausalEventDagCause::Existing(TraceId::new(50))],
        vec![effect(41)],
    )
    .unwrap();

    // When: a transaction-local child cites the cold parent.
    let resolved = store
        .commit_dag_batch(
            SimulationTime::new(12),
            Phase::Physics,
            vec![child],
            CausalDagBatchLimits {
                max_causes_per_event: 4,
                max_effects_per_event: 4,
            },
        )
        .unwrap();

    // Then: frontier-derived IDs and the staged edge are immediately visible.
    let child_trace = resolved[&child_key];
    assert_eq!(child_trace, TraceId::new(52));
    assert_eq!(
        store.event(child_trace).unwrap().unwrap().causes(),
        &[TraceId::new(50)]
    );
    assert_eq!(
        store
            .children(TraceId::new(50), HistoryQuery::new(0, 4, 64).unwrap())
            .unwrap()
            .records(),
        &[TraceId::new(51), TraceId::new(52)]
    );
    println!(
        "read-your-writes overlay marker: cold_child=51 staged_child={}",
        child_trace.raw()
    );
    let first = store
        .children(TraceId::new(50), HistoryQuery::new(0, 1, 8).unwrap())
        .unwrap();
    let second = store
        .children(
            TraceId::new(50),
            HistoryQuery::new(first.next_offset().unwrap(), 1, 8).unwrap(),
        )
        .unwrap();
    assert_eq!(first.records(), &[TraceId::new(51)]);
    assert_eq!(second.records(), &[TraceId::new(52)]);
    assert_eq!(second.next_offset(), None);
    assert!(store.hot_bytes() <= store.hot_budget());
}

#[test]
fn provenance_transaction_overflow_is_trimmed_after_the_frame_is_staged() {
    // Given: an unflushed same-tick record and a budget that fits only that record.
    let parent = causal(8, 40, 50, vec![]);
    let prelude = causal(9, 41, 51, vec![]);
    let budget = prelude.canonical_field_size().unwrap();
    let mut store =
        CausalTraceStore::with_cold_reader(indexed_reader(vec![parent]), budget, Vec::new())
            .unwrap();
    let prelude_key = CausalEventProposalKey::new(1, 8, b"prelude", 0).unwrap();
    let prelude_proposal = CausalEventDagProposal::new(
        prelude_key,
        EventKindId::new(13),
        Vec::new(),
        vec![effect(41)],
    )
    .unwrap();
    store
        .commit_dag_batch(
            SimulationTime::new(12),
            Phase::Physics,
            vec![prelude_proposal],
            CausalDagBatchLimits {
                max_causes_per_event: 4,
                max_effects_per_event: 4,
            },
        )
        .unwrap();

    let first_key = CausalEventProposalKey::new(1, 9, b"overflow-a", 0).unwrap();
    let second_key = CausalEventProposalKey::new(1, 10, b"overflow-b", 0).unwrap();
    let proposals = vec![
        CausalEventDagProposal::new(
            first_key,
            EventKindId::new(13),
            Vec::new(),
            vec![effect(42)],
        )
        .unwrap(),
        CausalEventDagProposal::new(
            second_key,
            EventKindId::new(13),
            Vec::new(),
            vec![effect(43)],
        )
        .unwrap(),
    ];

    // When: the transaction-only path accepts the batch before its exact frame is staged.
    store
        .commit_dag_batch_for_transaction(
            SimulationTime::new(12),
            Phase::Physics,
            proposals,
            CausalDagBatchLimits {
                max_causes_per_event: 4,
                max_effects_per_event: 4,
            },
        )
        .unwrap();
    let frame = store
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
            .map(HistoryRecord::Causal)
            .unwrap()
        })
        .collect::<Vec<_>>();
    store.trim_staged_frame_to_hot_budget(&frame).unwrap();

    // Then: copied transaction records can be evicted without a cold-reader round trip.
    println!(
        "transaction-local overflow marker: frame_records={} hot_records={} hot_bytes={} budget={}",
        frame.len(),
        store.iter_hot().len(),
        store.hot_bytes(),
        store.hot_budget()
    );
    assert_eq!(frame.len(), 3);
    assert!(store.hot_bytes() <= store.hot_budget());
    assert_eq!(store.iter_hot().len(), 1);
    assert_eq!(
        store.event(TraceId::new(53)).unwrap().unwrap().event_id(),
        EventId::new(43)
    );
}

#[test]
fn provenance_active_trim_rollback_preserves_pretransaction_hot_prefix() {
    // Given: a cold-backed facade with one pre-transaction hot record and a
    // budget that cannot hold that record plus a staged child.
    let parent = causal(8, 40, 50, vec![]);
    let prelude = causal(9, 41, 51, vec![TraceId::new(50)]);
    let budget = prelude.canonical_field_size().unwrap();
    let mut store = CausalTraceStore::with_cold_reader(
        indexed_reader(vec![parent, prelude.clone()]),
        budget,
        vec![prelude.clone()],
    )
    .unwrap();
    let before = store.clone();
    let child_key = CausalEventProposalKey::new(1, 9, b"rollback-child", 0).unwrap();
    let child = CausalEventDagProposal::new(
        child_key.clone(),
        EventKindId::new(13),
        vec![CausalEventDagCause::Existing(TraceId::new(51))],
        vec![effect(42)],
    )
    .unwrap();

    // When: the staged child requests hot trimming and the transaction rolls back.
    store.begin_transaction();
    let child_trace = store
        .commit_dag_batch_for_transaction(
            SimulationTime::new(12),
            Phase::Physics,
            vec![child.clone()],
            CausalDagBatchLimits {
                max_causes_per_event: 4,
                max_effects_per_event: 4,
            },
        )
        .unwrap()[&child_key];
    let frame = vec![HistoryRecord::Causal(
        store.event(child_trace).unwrap().unwrap(),
    )];
    store.trim_staged_frame_to_hot_budget(&frame).unwrap();
    assert_eq!(store.iter_hot().count(), 2);
    assert!(store.hot_bytes() > store.hot_budget());
    assert_eq!(
        store
            .children(TraceId::new(51), HistoryQuery::new(0, 4, 64).unwrap())
            .unwrap()
            .records(),
        &[child_trace]
    );
    store.rollback_transaction();

    // Then: the hot prefix, child index, IDs, bytes, and cold overlay are exact;
    // retrying the same valid child reuses the rejected frontier.
    assert_eq!(store, before);
    assert_eq!(store.iter_hot().count(), 1);
    assert_eq!(store.hot_bytes(), budget);
    assert_eq!(store.event(child_trace).unwrap(), None);
    assert_eq!(
        store
            .children(TraceId::new(50), HistoryQuery::new(0, 4, 64).unwrap())
            .unwrap()
            .records(),
        &[TraceId::new(51)]
    );
    let retry = store
        .commit_dag_batch(
            SimulationTime::new(12),
            Phase::Physics,
            vec![child],
            CausalDagBatchLimits {
                max_causes_per_event: 4,
                max_effects_per_event: 4,
            },
        )
        .unwrap();
    assert_eq!(retry[&child_key], child_trace);
    println!(
        "active trim rollback marker: before_hot=1 after_rollback_hot={} retry_trace={} hot_bytes={} budget={}",
        store.iter_hot().count(),
        retry[&child_key].raw(),
        store.hot_bytes(),
        store.hot_budget()
    );
}

#[test]
fn provenance_hot_budget_rejection_is_typed_atomic_and_repeatable() {
    // Given: a cold parent and a cache budget one byte smaller than its staged child.
    let parent = causal(8, 40, 50, vec![]);
    let child_size = causal(12, 41, 51, vec![TraceId::new(50)]).decoded_size();
    let mut store = CausalTraceStore::with_cold_reader(
        indexed_reader(vec![parent]),
        child_size - 1,
        Vec::new(),
    )
    .unwrap();
    let before = store.clone();
    let child_key = CausalEventProposalKey::new(1, 9, b"child", 0).unwrap();
    let child = CausalEventDagProposal::new(
        child_key,
        EventKindId::new(13),
        vec![CausalEventDagCause::Existing(TraceId::new(50))],
        vec![effect(41)],
    )
    .unwrap();

    // When: the same oversized staged write is attempted after an interruption boundary.
    let first = store.commit_dag_batch(
        SimulationTime::new(12),
        Phase::Physics,
        vec![child.clone()],
        CausalDagBatchLimits {
            max_causes_per_event: 4,
            max_effects_per_event: 4,
        },
    );
    let second = store.commit_dag_batch(
        SimulationTime::new(12),
        Phase::Physics,
        vec![child],
        CausalDagBatchLimits {
            max_causes_per_event: 4,
            max_effects_per_event: 4,
        },
    );

    // Then: both attempts have the same typed marker and leave bytes, IDs, and cache unchanged.
    println!("typed rejection marker: {first:?}; repeated interruption marker: {second:?}");
    assert!(matches!(
        first,
        Err(CausalDagCommitError::HistoryRead(
            HistoryError::HotBudgetExceeded { .. }
        ))
    ));
    assert_eq!(first, second);
    assert_eq!(store, before);
}
