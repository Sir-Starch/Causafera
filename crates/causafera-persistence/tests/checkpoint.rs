use causafera_core::{
    HistoryAnchor, HistoryAnchorFields, HistoryDurability, HistoryFrontier, MAX_CAUSES_PER_EVENT,
};
use causafera_persistence::PersistenceError;
use causafera_persistence::checkpoint::{
    CHECKPOINT_HEADER_FIXED_LEN, CHECKPOINT_MAX_ANCHOR_FIELDS, CHECKPOINT_MAX_PAYLOAD_BYTES,
    CheckpointMetadata, CheckpointPolicy, CheckpointThreshold, decode_checkpoint,
    decode_checkpoint_with_generations, encode_checkpoint, should_checkpoint,
};
use causafera_persistence::{
    CHECKPOINT_DURABLE_FRAME_BYTES, CHECKPOINT_DURABLE_TICKS, CheckpointTrigger, FORMAT_MAJOR_V1,
    FORMAT_MINOR_V1, MAX_TOTAL_SIZE, SectionPayload, SnapshotEnvelope, SnapshotHeader,
    decode_history_anchor, encode_history_anchor, select_checkpoint_trigger,
};
use causafera_types::{EventId, SimulationTime, TraceId};

fn anchor_fixture() -> HistoryAnchor {
    HistoryAnchor::new(HistoryAnchorFields {
        namespace: [0xAB; 16],
        logical_generation: 7,
        durable_generation: 5,
        checkpoint_digest: [0x11; 32],
        frontier: HistoryFrontier::new(
            7,
            EventId::new(42),
            TraceId::new(100),
            SimulationTime::new(1234),
        ),
        logical_digest: [0x22; 32],
        storage_root: [0x33; 32],
        physical_digest_schema: 9,
        history_digest_schema: 9,
        evidence_manifest_digest: [0x44; 32],
        durability: HistoryDurability::GroupCommit {
            max_ticks: 8,
            max_wall_time_ms: 1000,
        },
    })
    .unwrap()
}

fn payload_fixture() -> Vec<u8> {
    b"minimal current-state payload".to_vec()
}

fn metadata_fixture() -> CheckpointMetadata {
    CheckpointMetadata {
        runtime_recipe_fingerprint: [0x55; 32],
        input_fingerprint: [0x66; 32],
        physical_digest: [0x77; 32],
        completed_time: SimulationTime::new(1234),
    }
}

#[test]
fn checkpoint_round_trip_preserves_fields() {
    let anchor = anchor_fixture();
    let payload = payload_fixture();
    let encoded = encode_checkpoint(&anchor, &metadata_fixture(), &payload).unwrap();
    let decoded = decode_checkpoint(&encoded).unwrap();
    assert_eq!(
        decoded.header.logical_generation,
        anchor.logical_generation()
    );
    assert_eq!(
        decoded.header.durable_generation,
        anchor.durable_generation()
    );
    assert_eq!(
        decoded.header.frontier.generation,
        anchor.frontier().generation
    );
    assert_eq!(decoded.header.frontier.event_id, anchor.frontier().event_id);
    assert_eq!(decoded.header.frontier.trace_id, anchor.frontier().trace_id);
    assert_eq!(decoded.header.frontier.time, anchor.frontier().time);
    assert_eq!(decoded.payload, payload);
    assert_eq!(decoded.header.payload_length, payload.len() as u64);
    assert_eq!(decoded.header.namespace, *anchor.namespace());
    assert_eq!(
        decoded.header.runtime_recipe_fingerprint,
        metadata_fixture().runtime_recipe_fingerprint
    );
    assert_eq!(
        decoded.header.input_fingerprint,
        metadata_fixture().input_fingerprint
    );
    assert_eq!(
        decoded.header.physical_digest,
        metadata_fixture().physical_digest
    );
    assert_eq!(
        decoded.header.completed_time,
        metadata_fixture().completed_time
    );
    assert_eq!(decoded.header.durability, anchor.durability());
    assert_eq!(
        decoded.header.evidence_manifest_digest,
        *anchor.evidence_manifest_digest()
    );
}

#[test]
fn checkpoint_digest_mismatches_corrupted_header() {
    let anchor = anchor_fixture();
    let payload = payload_fixture();
    let mut encoded = encode_checkpoint(&anchor, &metadata_fixture(), &payload).unwrap();
    encoded[10] ^= 0xFF;
    let result = decode_checkpoint(&encoded);
    assert!(matches!(
        result,
        Err(PersistenceError::CheckpointDigestMismatch)
    ));
}

#[test]
fn checkpoint_digest_mismatches_corrupted_payload() {
    let anchor = anchor_fixture();
    let payload = payload_fixture();
    let mut encoded = encode_checkpoint(&anchor, &metadata_fixture(), &payload).unwrap();
    let last = encoded.len() - 1;
    encoded[last] ^= 0xFF;
    let result = decode_checkpoint(&encoded);
    assert!(matches!(
        result,
        Err(PersistenceError::CheckpointDigestMismatch)
    ));
}

#[test]
fn checkpoint_trailing_bytes_after_payload_are_rejected() {
    let anchor = anchor_fixture();
    let payload = payload_fixture();
    let mut encoded = encode_checkpoint(&anchor, &metadata_fixture(), &payload).unwrap();
    encoded.extend_from_slice(b"trailing");
    let result = decode_checkpoint(&encoded);
    assert!(matches!(
        result,
        Err(PersistenceError::TrailingCheckpointBytes { .. })
    ));
}

#[test]
fn checkpoint_undersized_buffer_is_rejected() {
    let result = decode_checkpoint(&[0x00, 0x01]);
    assert!(matches!(
        result,
        Err(PersistenceError::CheckpointCodec { .. })
    ));
}

#[test]
fn checkpoint_bad_magic_is_rejected() {
    let anchor = anchor_fixture();
    let payload = payload_fixture();
    let mut encoded = encode_checkpoint(&anchor, &metadata_fixture(), &payload).unwrap();
    encoded[0] = b'X';
    encoded[1] = b'X';
    encoded[2] = b'X';
    encoded[3] = b'X';
    let result = decode_checkpoint(&encoded);
    assert!(matches!(
        result,
        Err(PersistenceError::CheckpointMagicMismatch)
    ));
}

#[test]
fn checkpoint_unsupported_version_is_rejected() {
    let anchor = anchor_fixture();
    let payload = payload_fixture();
    let mut encoded = encode_checkpoint(&anchor, &metadata_fixture(), &payload).unwrap();
    encoded[4] = 0xFF;
    encoded[5] = 0xFF;
    let result = decode_checkpoint(&encoded);
    assert!(matches!(
        result,
        Err(PersistenceError::UnsupportedCheckpointVersion { .. })
    ));
}

#[test]
fn checkpoint_oversized_payload_is_rejected() {
    let anchor = anchor_fixture();
    let payload = vec![0u8; CHECKPOINT_MAX_PAYLOAD_BYTES + 1];
    let result = encode_checkpoint(&anchor, &metadata_fixture(), &payload);
    assert!(matches!(
        result,
        Err(PersistenceError::CheckpointPayloadTooLarge { .. })
    ));
}

#[test]
fn checkpoint_decode_oversize_payload_length_is_rejected() {
    let anchor = anchor_fixture();
    let payload = payload_fixture();
    let mut encoded = encode_checkpoint(&anchor, &metadata_fixture(), &payload).unwrap();
    let payload_length_offset = CHECKPOINT_HEADER_FIXED_LEN - 8;
    encoded[payload_length_offset..payload_length_offset + 8]
        .copy_from_slice(&(CHECKPOINT_MAX_PAYLOAD_BYTES as u64 + 1).to_le_bytes());
    let result = decode_checkpoint(&encoded);
    assert!(matches!(
        result,
        Err(PersistenceError::CheckpointPayloadTooLarge { .. })
    ));
}

#[test]
fn checkpoint_generation_mismatch_is_rejected() {
    let anchor = anchor_fixture();
    let payload = payload_fixture();
    let encoded = encode_checkpoint(&anchor, &metadata_fixture(), &payload).unwrap();
    // Decode with a different durable generation than encoded.
    let result = decode_checkpoint_with_generations(&encoded, anchor.logical_generation(), 99);
    assert!(matches!(
        result,
        Err(PersistenceError::CheckpointGenerationMismatch { .. })
    ));
}

#[test]
fn checkpoint_coordinator_threshold_triggers_on_ticks() {
    let policy = CheckpointPolicy {
        threshold: CheckpointThreshold::Coordinator {
            ticks: 10,
            bytes: 1024,
        },
        force: false,
    };
    assert!(should_checkpoint(10, 0, &policy));
    assert!(should_checkpoint(11, 0, &policy));
    assert!(!should_checkpoint(9, 0, &policy));
}

#[test]
fn checkpoint_coordinator_threshold_triggers_on_bytes() {
    let policy = CheckpointPolicy {
        threshold: CheckpointThreshold::Coordinator {
            ticks: 10,
            bytes: 1024,
        },
        force: false,
    };
    assert!(should_checkpoint(0, 1024, &policy));
    assert!(should_checkpoint(0, 1025, &policy));
    assert!(!should_checkpoint(0, 1023, &policy));
}

#[test]
fn checkpoint_force_overrides_threshold() {
    let policy = CheckpointPolicy {
        threshold: CheckpointThreshold::Coordinator {
            ticks: 10,
            bytes: 1024,
        },
        force: true,
    };
    assert!(should_checkpoint(0, 0, &policy));
}

#[test]
fn checkpoint_constant_4096_is_anchor_field_limit() {
    assert_eq!(CHECKPOINT_MAX_ANCHOR_FIELDS, 4096);
}

#[test]
fn checkpoint_constant_67108864_is_payload_limit() {
    assert_eq!(CHECKPOINT_MAX_PAYLOAD_BYTES, 67_108_864);
}

#[test]
fn checkpoint_max_causes_per_event_is_4096() {
    assert_eq!(MAX_CAUSES_PER_EVENT, 4096);
}

#[test]
fn checkpoint_decode_repeated_or_stale_generation_is_rejected() {
    let anchor = anchor_fixture();
    let payload = payload_fixture();
    let encoded = encode_checkpoint(&anchor, &metadata_fixture(), &payload).unwrap();
    // Same durable generation as logical is accepted (already durable boundary), but
    // stale durable generation ahead of logical is rejected.
    let result = decode_checkpoint_with_generations(&encoded, 5, 7);
    assert!(matches!(
        result,
        Err(PersistenceError::CheckpointGenerationMismatch { .. })
    ));
}

#[test]
fn checkpoint_stale_logical_generation_is_rejected_by_coordinator() {
    let anchor = anchor_fixture();
    let payload = payload_fixture();
    let encoded = encode_checkpoint(&anchor, &metadata_fixture(), &payload).unwrap();
    // Caller expects a newer logical generation than the checkpoint records.
    let result = decode_checkpoint_with_generations(
        &encoded,
        anchor.logical_generation() + 1,
        anchor.durable_generation(),
    );
    assert!(matches!(
        result,
        Err(PersistenceError::CheckpointGenerationMismatch { .. })
    ));
}

#[test]
fn checkpoint_dirty_header_field_is_rejected() {
    let anchor = anchor_fixture();
    let payload = payload_fixture();
    let mut encoded = encode_checkpoint(&anchor, &metadata_fixture(), &payload).unwrap();
    // Logical generation field starts at offset 4 (magic) + 4 (version) + 2 (anchor count) + 16 (namespace) = 26.
    let logical_generation_offset = 4 + 4 + 2 + 16;
    encoded[logical_generation_offset] ^= 0xFF;
    let result = decode_checkpoint(&encoded);
    assert!(matches!(
        result,
        Err(PersistenceError::CheckpointDigestMismatch)
    ));
}

#[test]
fn checkpoint_misleading_valid_header_with_invalid_digest_is_rejected() {
    let anchor = anchor_fixture();
    let payload = payload_fixture();
    let mut encoded = encode_checkpoint(&anchor, &metadata_fixture(), &payload).unwrap();
    let payload_start = encoded.len() - payload.len();
    encoded[payload_start] ^= 0xFF;
    let result = decode_checkpoint(&encoded);
    assert!(matches!(
        result,
        Err(PersistenceError::CheckpointDigestMismatch)
    ));
}

#[test]
fn checkpoint_repeated_decode_is_idempotent() {
    let anchor = anchor_fixture();
    let payload = payload_fixture();
    let encoded = encode_checkpoint(&anchor, &metadata_fixture(), &payload).unwrap();
    let first = decode_checkpoint(&encoded).unwrap();
    let second = decode_checkpoint(&encoded).unwrap();
    assert_eq!(first, second);
    assert_eq!(first.digest, second.digest);
}

#[test]
fn checkpoint_long_payload_at_limit_round_trips() {
    let anchor = anchor_fixture();
    let payload = vec![0xDBu8; CHECKPOINT_MAX_PAYLOAD_BYTES];
    let encoded = encode_checkpoint(&anchor, &metadata_fixture(), &payload).unwrap();
    let decoded = decode_checkpoint(&encoded).unwrap();
    assert_eq!(decoded.payload.len(), CHECKPOINT_MAX_PAYLOAD_BYTES);
    assert_eq!(decoded.payload, payload);
}

#[test]
fn checkpoint_zero_byte_payload_round_trips() {
    let anchor = anchor_fixture();
    let payload = Vec::new();
    let encoded = encode_checkpoint(&anchor, &metadata_fixture(), &payload).unwrap();
    let decoded = decode_checkpoint(&encoded).unwrap();
    assert_eq!(decoded.payload, payload);
    assert_eq!(decoded.header.payload_length, 0);
}

#[test]
fn checkpoint_every_tick_threshold_always_triggers() {
    let policy = CheckpointPolicy {
        threshold: CheckpointThreshold::EveryTick,
        force: false,
    };
    assert!(should_checkpoint(0, 0, &policy));
}

#[test]
fn checkpoint_ticks_only_threshold_respects_bytes() {
    let policy = CheckpointPolicy {
        threshold: CheckpointThreshold::Ticks(5),
        force: false,
    };
    assert!(!should_checkpoint(0, 1_000_000, &policy));
    assert!(should_checkpoint(5, 0, &policy));
}

#[test]
fn checkpoint_bytes_only_threshold_respects_ticks() {
    let policy = CheckpointPolicy {
        threshold: CheckpointThreshold::Bytes(1_000_000),
        force: false,
    };
    assert!(!should_checkpoint(1_000_000, 0, &policy));
    assert!(should_checkpoint(0, 1_000_000, &policy));
}

#[test]
fn checkpoint_todo4_threshold_selects_the_first_durable_bound_or_explicit_save() {
    assert_eq!(
        select_checkpoint_trigger(
            CHECKPOINT_DURABLE_TICKS - 1,
            CHECKPOINT_DURABLE_FRAME_BYTES - 1,
            false
        ),
        None
    );
    assert_eq!(
        select_checkpoint_trigger(CHECKPOINT_DURABLE_TICKS, 0, false),
        Some(CheckpointTrigger::DurableTicks)
    );
    assert_eq!(
        select_checkpoint_trigger(0, CHECKPOINT_DURABLE_FRAME_BYTES, false),
        Some(CheckpointTrigger::DurableFrameBytes)
    );
    assert_eq!(
        select_checkpoint_trigger(0, 0, true),
        Some(CheckpointTrigger::ExplicitSave)
    );
    println!(
        "CHECKPOINT_THRESHOLD durable_ticks={} durable_frame_bytes={} explicit_save_forces=true",
        CHECKPOINT_DURABLE_TICKS, CHECKPOINT_DURABLE_FRAME_BYTES
    );
}

#[test]
fn checkpoint_duplicate_anchor_field_group_is_rejected() {
    let anchor = anchor_fixture();
    let payload = payload_fixture();
    let mut encoded = encode_checkpoint(&anchor, &metadata_fixture(), &payload).unwrap();
    encoded[8..10].copy_from_slice(&2_u16.to_le_bytes());
    let error = decode_checkpoint(&encoded).expect_err("duplicate anchor group must fail");
    assert!(matches!(error, PersistenceError::CheckpointCodec { .. }));
    println!("CHECKPOINT_REJECTION case=duplicate_anchor_group accepted=false error={error:?}");
}

#[test]
fn checkpoint_snapshot_envelope_encode_enforces_256_mib_cap() {
    let header = SnapshotHeader {
        format_major: FORMAT_MAJOR_V1,
        format_minor: FORMAT_MINOR_V1,
        codec_revision: 2,
        world_seed: 1,
        completed_time: 1,
        runtime_recipe_fingerprint: [0; 32],
        physical_digest_schema: 9,
        physical_digest: [0; 32],
        history_digest_schema: 9,
        history_digest: [0; 32],
        section_count: 0,
        section_directory_offset: 0,
        payload_integrity: [0; 32],
    };
    let mut sections = std::collections::BTreeMap::new();
    sections.insert(
        1,
        SectionPayload {
            section_major: 1,
            section_minor: 0,
            flags: 0,
            decoded_size_limit: 0,
            bytes: vec![0; usize::try_from(MAX_TOTAL_SIZE).unwrap()],
        },
    );
    let error = SnapshotEnvelope::new(header, sections)
        .encode()
        .expect_err("oversized envelope must fail");
    assert!(matches!(error, PersistenceError::TotalSizeExceeded { .. }));
    println!("CHECKPOINT_REJECTION case=envelope_over_256_mib accepted=false error={error:?}");
}

#[test]
fn checkpoint_history_anchor_codec_round_trips_canonically_and_rejects_corruption() {
    let anchor = anchor_fixture();
    let first = encode_history_anchor(&anchor).expect("anchor encodes");
    let second = encode_history_anchor(&anchor).expect("anchor re-encodes");
    assert_eq!(first, second);
    assert_eq!(
        decode_history_anchor(&first).expect("anchor decodes"),
        anchor
    );

    let mut corrupted = first;
    corrupted[40] ^= 0x01;
    let error = decode_history_anchor(&corrupted).expect_err("corrupt anchor must fail");
    assert!(matches!(
        error,
        PersistenceError::HistoryAnchorDigestMismatch
    ));
    println!(
        "CHECKPOINT_HISTORY_ANCHOR canonical_bytes={} coverage_manifest_digest={:?} corruption=CheckpointDigest accepted=false",
        second.len(),
        anchor.evidence_manifest_digest()
    );
}
