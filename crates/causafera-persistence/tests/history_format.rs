use causafera_core::{
    CausalEffect, CausalTarget, Criticality, DomainEvidenceRecord, EvidenceDecoderRegistry,
    EvidenceSchemaId, EvidenceSchemaVersion, HistoryError, HistoryRecord,
    MAX_HISTORY_RECORDS_PER_FRAME, OwnedCausalRecord, Phase, StateFingerprint,
};
use causafera_persistence::{
    HISTORY_FORMAT_MAJOR, HISTORY_FORMAT_MINOR, HISTORY_FRAME_DOMAIN, HISTORY_FRAME_HEADER_LEN,
    HISTORY_FRAME_MAGIC, HistoryFrameHeaderFields, PersistenceError, decode_history_frame,
    encode_history_frame, logical_history_digest, sealed_segment_identity, sealed_segment_root,
    validate_checkpoint_length, verify_logical_history_digest, verify_sealed_segment_identity,
};
use causafera_types::{
    EventId, EventKindId, SimulationTime, StateObjectKindId, StatePropertyId, TraceId,
};

fn effect(object_id: u64, before: u8, after: u8) -> CausalEffect {
    CausalEffect::new(
        CausalTarget::new(
            StateObjectKindId::new(7),
            object_id,
            StatePropertyId::new(9),
        ),
        StateFingerprint::new([before; 32]),
        StateFingerprint::new([after; 32]),
    )
    .expect("fixture effect changes state")
}

fn causal(commit_sequence: u64, event: u64, trace: u64, causes: Vec<TraceId>) -> HistoryRecord {
    HistoryRecord::Causal(
        OwnedCausalRecord::new(
            commit_sequence,
            EventId::new(event),
            TraceId::new(trace),
            SimulationTime::new(11),
            Phase::Physics,
            EventKindId::new(13),
            causes,
            vec![effect(trace, 0x21, 0x22)],
        )
        .expect("fixture causal record is valid"),
    )
}

fn evidence(commit_sequence: u64, key: &[u8], criticality: Criticality) -> HistoryRecord {
    let payload = b"rain=17".to_vec();
    HistoryRecord::Evidence(
        DomainEvidenceRecord::new(
            commit_sequence,
            EvidenceSchemaId::new(0xABCD),
            EvidenceSchemaVersion::new(2, 3),
            criticality,
            SimulationTime::new(11),
            vec![TraceId::new(20)],
            key.to_vec(),
            payload.clone(),
            StateFingerprint::new(*blake3::hash(&payload).as_bytes()),
        )
        .expect("fixture evidence is valid"),
    )
}

fn header(
    first_sequence: u64,
    last_sequence: u64,
    first_trace: u64,
    last_trace: u64,
) -> HistoryFrameHeaderFields {
    HistoryFrameHeaderFields {
        namespace: [0x10; 32],
        catalog_generation: 5,
        first_record_sequence: first_sequence,
        last_record_sequence: last_sequence,
        first_event_id: first_trace,
        last_event_id: last_trace,
        first_trace_id: first_trace,
        last_trace_id: last_trace,
        prior_logical_history_digest: [0x20; 32],
    }
}

fn independent_causal_record_bytes() -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.push(1);
    bytes.extend_from_slice(&1_u16.to_le_bytes());
    bytes.extend_from_slice(&0_u16.to_le_bytes());
    for value in [1_u64, 20, 20, 11, 3, 13, 0, 1, 7, 20, 9] {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes.extend_from_slice(&[0x21; 32]);
    bytes.extend_from_slice(&[0x22; 32]);
    bytes
}

fn independent_frame_bytes() -> Vec<u8> {
    let record = independent_causal_record_bytes();
    let mut payload = Vec::new();
    payload.extend_from_slice(&1_u64.to_le_bytes());
    payload.extend_from_slice(&(record.len() as u64).to_le_bytes());
    payload.extend_from_slice(&record);

    let mut without_digest = Vec::new();
    without_digest.extend_from_slice(b"CFHF");
    without_digest.extend_from_slice(&1_u16.to_le_bytes());
    without_digest.extend_from_slice(&0_u16.to_le_bytes());
    without_digest.extend_from_slice(&[0x10; 32]);
    for value in [5_u64, 1, 1, 20, 20, 20, 20] {
        without_digest.extend_from_slice(&value.to_le_bytes());
    }
    without_digest.extend_from_slice(&[0x20; 32]);
    without_digest.extend_from_slice(&(payload.len() as u64).to_le_bytes());
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"causafera/history-frame/v1\0");
    hasher.update(&without_digest);
    hasher.update(&payload);

    let mut frame = without_digest;
    frame.extend_from_slice(hasher.finalize().as_bytes());
    frame.extend_from_slice(&payload);
    frame
}

fn independent_empty_frame_bytes() -> Vec<u8> {
    let payload = 0_u64.to_le_bytes();
    let mut without_digest = Vec::new();
    without_digest.extend_from_slice(b"CFHF");
    without_digest.extend_from_slice(&1_u16.to_le_bytes());
    without_digest.extend_from_slice(&0_u16.to_le_bytes());
    without_digest.extend_from_slice(&[0x10; 32]);
    for value in [5_u64, 1, 0, 0, 0, 0, 0] {
        without_digest.extend_from_slice(&value.to_le_bytes());
    }
    without_digest.extend_from_slice(&[0x20; 32]);
    without_digest.extend_from_slice(&(payload.len() as u64).to_le_bytes());
    let mut hasher = blake3::Hasher::new();
    hasher.update(HISTORY_FRAME_DOMAIN);
    hasher.update(&without_digest);
    hasher.update(&payload);

    let mut frame = without_digest;
    frame.extend_from_slice(hasher.finalize().as_bytes());
    frame.extend_from_slice(&payload);
    frame
}

fn resign_frame(bytes: &mut [u8]) {
    let mut hasher = blake3::Hasher::new();
    hasher.update(HISTORY_FRAME_DOMAIN);
    hasher.update(&bytes[..136]);
    hasher.update(&bytes[168..]);
    bytes[136..168].copy_from_slice(hasher.finalize().as_bytes());
}

#[test]
fn history_format_matches_independent_golden_frame_vector() {
    // Given
    let records = vec![causal(1, 20, 20, vec![])];
    let expected = independent_frame_bytes();

    // When
    let actual = encode_history_frame(header(1, 1, 20, 20), &records).expect("encode frame");

    // Then
    assert_eq!(actual, expected);
    assert_eq!(actual.len(), HISTORY_FRAME_HEADER_LEN + 8 + 8 + 157);
    assert_eq!(HISTORY_FRAME_MAGIC, *b"CFHF");
    assert_eq!((HISTORY_FORMAT_MAJOR, HISTORY_FORMAT_MINOR), (1, 0));
    assert_eq!(HISTORY_FRAME_DOMAIN, b"causafera/history-frame/v1\0");
    assert_eq!(
        &actual[136..168],
        &[
            0xba, 0x30, 0x65, 0xe6, 0x68, 0xcd, 0x41, 0x96, 0x3a, 0x35, 0xe5, 0xf9, 0x52, 0x3e,
            0x7b, 0x2d, 0xb5, 0x4b, 0x11, 0x35, 0x54, 0xd2, 0x72, 0x0b, 0x6b, 0x68, 0x23, 0x7e,
            0x8f, 0x1d, 0xa1, 0x42,
        ]
    );
    println!(
        "golden_frame_bytes={}",
        actual
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    );
    println!(
        "golden_frame_digest={}",
        actual[136..168]
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    );
}

#[test]
fn history_format_rejects_physically_encoded_zero_record_frame() {
    // Given
    let bytes = independent_empty_frame_bytes();

    // When
    let error = decode_history_frame(&bytes, &EvidenceDecoderRegistry::new())
        .expect_err("a zero-record frame must fail closed");

    // Then
    assert!(matches!(error, PersistenceError::EmptyHistoryFrame));
}

#[test]
fn history_format_roundtrips_raw_explanation_evidence() {
    // Given
    let records = vec![
        causal(1, 20, 20, vec![]),
        evidence(1, b"storm/a", Criticality::ExplanationOnly),
    ];
    let bytes = encode_history_frame(header(1, 2, 20, 20), &records).expect("encode frame");

    // When
    let decoded = decode_history_frame(&bytes, &EvidenceDecoderRegistry::new())
        .expect("raw explanation evidence is verifiable");

    // Then
    assert_eq!(decoded.records(), records);
    println!(
        "GENERIC_EVIDENCE_CODEC canonical_roundtrip=true unknown_explanation=Raw payload_digest=verified"
    );
}

#[test]
fn history_format_rejects_unknown_replay_schema() {
    // Given
    let records = vec![
        causal(1, 20, 20, vec![]),
        evidence(1, b"storm/a", Criticality::ReplayRequired),
    ];
    let bytes = encode_history_frame(header(1, 2, 20, 20), &records).expect("encode frame");

    // When
    let error = decode_history_frame(&bytes, &EvidenceDecoderRegistry::new())
        .expect_err("unknown replay schema must fail closed");

    // Then
    assert!(matches!(
        error,
        PersistenceError::History(HistoryError::UnsupportedReplaySchema { .. })
    ));
    println!(
        "GENERIC_EVIDENCE_CODEC unknown_replay=UnsupportedReplaySchema malformed_policy=fail_closed"
    );
}

#[test]
fn history_format_logical_digest_ignores_frame_and_segment_boundaries() {
    // Given
    let records = vec![
        causal(1, 20, 20, vec![]),
        causal(2, 21, 21, vec![TraceId::new(20)]),
    ];
    let one_frame = encode_history_frame(header(1, 2, 20, 21), &records).expect("one frame");
    let first = encode_history_frame(header(1, 1, 20, 20), &records[..1]).expect("first frame");
    let second = encode_history_frame(header(2, 2, 21, 21), &records[1..]).expect("second frame");

    // When
    let logical = logical_history_digest(&records).expect("logical digest");
    let one_segment = sealed_segment_identity(b"segment-header", &[one_frame.as_slice()]);
    let two_segments = [
        sealed_segment_identity(b"segment-header-a", &[first.as_slice()]),
        sealed_segment_identity(b"segment-header-b", &[second.as_slice()]),
    ];

    // Then
    assert_eq!(
        logical,
        logical_history_digest(&records).expect("repeat logical digest")
    );
    assert_eq!(
        logical,
        [
            0x09, 0x5b, 0xdd, 0xc8, 0xc7, 0x92, 0x40, 0xe8, 0xd5, 0x5c, 0xf5, 0x6e, 0x06, 0x8d,
            0x6c, 0x7c, 0x14, 0x00, 0x12, 0x2d, 0x01, 0x62, 0x1f, 0xf3, 0xc7, 0x2c, 0x00, 0x92,
            0x60, 0x4d, 0x63, 0xaf,
        ]
    );
    assert_eq!(
        one_segment,
        [
            0x01, 0xf3, 0xe9, 0x93, 0x51, 0x8a, 0xfc, 0xa8, 0x57, 0x66, 0x4a, 0xc3, 0x9f, 0x1c,
            0x98, 0x61, 0x7e, 0xa0, 0xe1, 0x45, 0xbc, 0x86, 0xaa, 0x2e, 0x5d, 0x03, 0x1c, 0x87,
            0xb1, 0x1e, 0xbe, 0xf5,
        ]
    );
    assert_ne!(
        sealed_segment_root(&[one_segment]),
        sealed_segment_root(&two_segments)
    );
    println!(
        "golden_logical_digest={}",
        logical
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    );
    println!(
        "golden_one_segment_identity={}",
        one_segment
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    );
}

#[test]
fn history_format_rejects_corruption() {
    // Given
    let records = vec![causal(1, 20, 20, vec![])];
    let valid = encode_history_frame(header(1, 1, 20, 20), &records).expect("encode frame");
    let registry = EvidenceDecoderRegistry::new();
    let cases = [
        ("truncated", valid[..valid.len() - 1].to_vec()),
        ("unsupported-version", {
            let mut bytes = valid.clone();
            bytes[4..6].copy_from_slice(&99_u16.to_le_bytes());
            bytes
        }),
        ("digest", {
            let mut bytes = valid.clone();
            bytes[136] ^= 1;
            bytes
        }),
        ("trailing", {
            let mut bytes = valid.clone();
            bytes.push(0);
            bytes
        }),
        ("malformed-length", {
            let mut bytes = valid.clone();
            bytes[176..184].copy_from_slice(&u64::MAX.to_le_bytes());
            resign_frame(&mut bytes);
            bytes
        }),
    ];

    // When
    let failures = cases.map(|(name, bytes)| {
        (
            name,
            decode_history_frame(&bytes, &registry).expect_err(name),
        )
    });

    // Then
    for (name, error) in failures {
        println!("typed_failure[{name}]={error:?}");
        assert!(matches!(
            (name, error),
            ("truncated", PersistenceError::MalformedHistoryLength { .. })
                | (
                    "unsupported-version",
                    PersistenceError::UnsupportedHistoryFormat { .. }
                )
                | ("digest", PersistenceError::FrameDigestMismatch)
                | ("trailing", PersistenceError::TrailingHistoryBytes { .. })
                | (
                    "malformed-length",
                    PersistenceError::MalformedHistoryLength { .. }
                )
        ));
        assert_eq!(
            decode_history_frame(&valid, &registry)
                .expect("failed decode does not mutate the valid input")
                .records(),
            records
        );
    }
}

#[test]
fn history_format_rejects_structural_and_bound_failures() {
    // Given
    let duplicate_ids = vec![
        causal(1, 20, 20, vec![]),
        causal(2, 20, 21, vec![TraceId::new(20)]),
    ];
    let unknown_parent = vec![causal(1, 20, 20, vec![TraceId::new(99)])];
    let noncanonical = vec![
        causal(2, 21, 21, vec![TraceId::new(20)]),
        causal(1, 20, 20, vec![]),
    ];
    let duplicate_keys = vec![
        causal(1, 20, 20, vec![]),
        evidence(1, b"same", Criticality::ExplanationOnly),
        evidence(2, b"same", Criticality::ExplanationOnly),
    ];
    let oversized_payload = vec![0x33; causafera_core::MAX_FRAME_DECODED_BYTES];
    let oversized = HistoryRecord::Evidence(
        DomainEvidenceRecord::new(
            1,
            EvidenceSchemaId::new(1),
            EvidenceSchemaVersion::new(1, 0),
            Criticality::ExplanationOnly,
            SimulationTime::new(1),
            vec![],
            vec![],
            oversized_payload.clone(),
            StateFingerprint::new(*blake3::hash(&oversized_payload).as_bytes()),
        )
        .expect("oversized fixture is structurally valid"),
    );

    // When
    let failures = [
        (
            "duplicate-id",
            encode_history_frame(header(1, 2, 20, 21), &duplicate_ids).expect_err("duplicate ID"),
        ),
        (
            "unknown-parent",
            encode_history_frame(header(1, 1, 20, 20), &unknown_parent)
                .expect_err("unknown parent"),
        ),
        (
            "order",
            encode_history_frame(header(1, 2, 21, 20), &noncanonical)
                .expect_err("noncanonical order"),
        ),
        (
            "duplicate-key",
            encode_history_frame(header(1, 3, 20, 20), &duplicate_keys).expect_err("duplicate key"),
        ),
        (
            "oversized",
            encode_history_frame(
                HistoryFrameHeaderFields {
                    first_event_id: 0,
                    last_event_id: 0,
                    first_trace_id: 0,
                    last_trace_id: 0,
                    ..header(1, 1, 0, 0)
                },
                &[oversized],
            )
            .expect_err("oversized frame"),
        ),
    ];

    // Then
    for (name, error) in failures {
        println!("typed_failure[{name}]={error:?}");
        assert!(matches!(
            (name, error),
            (
                "duplicate-id",
                PersistenceError::History(HistoryError::DuplicateEventId(_))
            ) | (
                "unknown-parent",
                PersistenceError::History(HistoryError::UnknownParent { .. })
            ) | (
                "order",
                PersistenceError::History(HistoryError::NonCanonicalOrder { .. })
            ) | (
                "duplicate-key",
                PersistenceError::History(HistoryError::DuplicateRecordKey)
            ) | (
                "oversized",
                PersistenceError::HistoryFrameSizeExceeded { .. }
            )
        ));
    }
}

#[test]
fn history_format_rejects_decreasing_causal_ids() {
    // Given
    let decreasing_ids = vec![
        causal(1, 20, 20, vec![]),
        causal(2, 19, 19, vec![TraceId::new(20)]),
    ];
    let decreasing_ids_with_interleaved_evidence = vec![
        causal(1, 20, 20, vec![]),
        evidence(1, b"storm/a", Criticality::ExplanationOnly),
        causal(2, 19, 19, vec![TraceId::new(20)]),
    ];
    let decreasing_event_header = HistoryFrameHeaderFields {
        first_event_id: 20,
        last_event_id: 19,
        first_trace_id: 20,
        last_trace_id: 20,
        ..header(1, 1, 20, 20)
    };
    let decreasing_trace_header = HistoryFrameHeaderFields {
        first_event_id: 20,
        last_event_id: 20,
        first_trace_id: 20,
        last_trace_id: 19,
        ..header(1, 1, 20, 20)
    };
    let increasing_with_interleaved_evidence = vec![
        causal(1, 20, 20, vec![]),
        evidence(2, b"storm/a", Criticality::ExplanationOnly),
        causal(3, 21, 21, vec![TraceId::new(20)]),
    ];

    // When
    let decreasing_ids_error =
        encode_history_frame(header(1, 2, 20, 19), &decreasing_ids).expect_err("decreasing IDs");
    let decreasing_ids_with_interleaved_evidence_error = encode_history_frame(
        header(1, 3, 20, 19),
        &decreasing_ids_with_interleaved_evidence,
    )
    .expect_err("decreasing IDs with interleaved evidence");
    let decreasing_event_header_error =
        encode_history_frame(decreasing_event_header, &[causal(1, 20, 20, vec![])])
            .expect_err("decreasing event header range");
    let decreasing_trace_header_error =
        encode_history_frame(decreasing_trace_header, &[causal(1, 20, 20, vec![])])
            .expect_err("decreasing trace header range");
    let increasing_frame =
        encode_history_frame(header(1, 3, 20, 21), &increasing_with_interleaved_evidence)
            .expect("increasing IDs with parent-before-child and interleaved evidence");
    let increasing_decoded =
        decode_history_frame(&increasing_frame, &EvidenceDecoderRegistry::new())
            .expect("valid decode");

    // Then
    println!("typed_failure[decreasing-ids]={decreasing_ids_error:?}");
    println!(
        "typed_failure[decreasing-ids-interleaved]={decreasing_ids_with_interleaved_evidence_error:?}"
    );
    println!("typed_failure[decreasing-event-header]={decreasing_event_header_error:?}");
    println!("typed_failure[decreasing-trace-header]={decreasing_trace_header_error:?}");
    assert!(matches!(
        decreasing_ids_error,
        PersistenceError::History(HistoryError::NonCanonicalOrder { index: 1 })
    ));
    assert!(matches!(
        decreasing_ids_with_interleaved_evidence_error,
        PersistenceError::History(HistoryError::NonCanonicalOrder { index: 2 })
    ));
    assert!(matches!(
        decreasing_event_header_error,
        PersistenceError::InvalidHistoryRange {
            field: "event_id",
            first: 20,
            last: 19,
        }
    ));
    assert!(matches!(
        decreasing_trace_header_error,
        PersistenceError::InvalidHistoryRange {
            field: "trace_id",
            first: 20,
            last: 19,
        }
    ));
    assert_eq!(
        increasing_decoded.records(),
        increasing_with_interleaved_evidence
    );
}

#[test]
fn history_format_rejects_additional_corruption() {
    // Given
    let records = vec![causal(1, 20, 20, vec![])];
    let valid = encode_history_frame(header(1, 1, 20, 20), &records).expect("valid frame");
    let explanation = vec![evidence(1, b"storm/a", Criticality::ExplanationOnly)];
    let evidence_frame = encode_history_frame(
        HistoryFrameHeaderFields {
            first_event_id: 0,
            last_event_id: 0,
            first_trace_id: 0,
            last_trace_id: 0,
            ..header(1, 1, 0, 0)
        },
        &explanation,
    )
    .expect("evidence frame");
    let registry = EvidenceDecoderRegistry::new();
    let cases = [
        ("record-count", {
            let mut bytes = valid.clone();
            let over_limit = u64::try_from(MAX_HISTORY_RECORDS_PER_FRAME + 1).expect("fits");
            bytes[168..176].copy_from_slice(&over_limit.to_le_bytes());
            resign_frame(&mut bytes);
            bytes
        }),
        ("cause-count", {
            let mut bytes = valid.clone();
            bytes[237..245].copy_from_slice(&4_097_u64.to_le_bytes());
            resign_frame(&mut bytes);
            bytes
        }),
        ("effect-count", {
            let mut bytes = valid.clone();
            bytes[245..253].copy_from_slice(&4_097_u64.to_le_bytes());
            resign_frame(&mut bytes);
            bytes
        }),
        ("record-version", {
            let mut bytes = valid.clone();
            bytes[185..187].copy_from_slice(&2_u16.to_le_bytes());
            resign_frame(&mut bytes);
            bytes
        }),
        ("criticality", {
            let mut bytes = evidence_frame.clone();
            bytes[209] = 99;
            resign_frame(&mut bytes);
            bytes
        }),
        ("evidence-payload-digest", {
            let mut bytes = evidence_frame.clone();
            let last = bytes.len() - 1;
            bytes[last] ^= 1;
            resign_frame(&mut bytes);
            bytes
        }),
        ("payload-frame-digest", {
            let mut bytes = valid.clone();
            let last = bytes.len() - 1;
            bytes[last] ^= 1;
            bytes
        }),
        ("header-frame-digest", {
            let mut bytes = valid.clone();
            bytes[40] ^= 1;
            bytes
        }),
    ];

    // When
    let failures = cases.map(|(name, bytes)| {
        (
            name,
            decode_history_frame(&bytes, &registry).expect_err(name),
        )
    });

    // Then
    for (name, error) in failures {
        println!("typed_failure[{name}]={error:?}");
        assert!(matches!(
            (name, error),
            (
                "record-count",
                PersistenceError::HistoryFrameSizeExceeded { .. }
            ) | (
                "cause-count",
                PersistenceError::HistoryFrameSizeExceeded { .. }
            ) | (
                "effect-count",
                PersistenceError::HistoryFrameSizeExceeded { .. }
            ) | (
                "record-version",
                PersistenceError::UnsupportedHistoryRecord { .. }
            ) | (
                "criticality",
                PersistenceError::UnsupportedEvidenceCriticality { .. }
            ) | (
                "evidence-payload-digest",
                PersistenceError::EvidencePayloadDigestMismatch { .. }
            ) | (
                "payload-frame-digest",
                PersistenceError::FrameDigestMismatch
            ) | ("header-frame-digest", PersistenceError::FrameDigestMismatch)
        ));
    }
}

#[test]
fn history_format_rejects_identity_and_checkpoint_mismatches() {
    // Given
    let records = vec![causal(1, 20, 20, vec![])];
    let frame = encode_history_frame(header(1, 1, 20, 20), &records).expect("valid frame");
    let segment = sealed_segment_identity(b"segment", &[frame.as_slice()]);
    let logical = logical_history_digest(&records).expect("logical digest");

    // When
    let segment_error = verify_sealed_segment_identity([0xFF; 32], b"segment", &[frame.as_slice()])
        .expect_err("altered segment identity");
    let logical_error =
        verify_logical_history_digest([0xFF; 32], &records).expect_err("altered logical digest");
    let checkpoint_error = validate_checkpoint_length(u64::MAX).expect_err("oversized checkpoint");

    // Then
    assert!(matches!(
        segment_error,
        PersistenceError::SegmentDigestMismatch
    ));
    assert!(matches!(
        logical_error,
        PersistenceError::LogicalDigestMismatch
    ));
    assert!(matches!(
        checkpoint_error,
        PersistenceError::History(HistoryError::DecodedSizeLimitExceeded { .. })
    ));
    verify_sealed_segment_identity(segment, b"segment", &[frame.as_slice()])
        .expect("segment verifies");
    verify_logical_history_digest(logical, &records).expect("logical digest verifies");
    println!("typed_failure[segment-digest]={segment_error:?}");
    println!("typed_failure[logical-digest]={logical_error:?}");
    println!("typed_failure[checkpoint-u64-max]={checkpoint_error:?}");
}
