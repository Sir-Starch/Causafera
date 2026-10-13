use causafera_core::{
    CausalEventSnapshot, HistoryAnchor, HistoryAnchorFields, HistoryDurability, HistoryFrontier,
    Phase,
};
use causafera_persistence::{PersistenceError, SectionDirectoryEntry, SnapshotEnvelope};
use causafera_runtime::{
    BoundedSnapshotMetadata, HISTORY_ANCHOR_SECTION_MAJOR, Runtime, RuntimeSnapshotData,
    SECTION_HISTORY_ANCHOR, assemble_bounded_envelope, history_anchor_from_bounded_envelope,
};
use causafera_types::{EventId, EventKindId, SimulationTime, TraceId};

fn anchor(event_count: usize, completed_time: SimulationTime) -> HistoryAnchor {
    let event_count = u64::try_from(event_count).expect("test event count fits u64");
    HistoryAnchor::new(HistoryAnchorFields {
        namespace: [0x41; 16],
        logical_generation: 1,
        durable_generation: 1,
        checkpoint_digest: [0x42; 32],
        frontier: HistoryFrontier::new(
            1,
            EventId::new(event_count),
            TraceId::new(event_count),
            completed_time,
        ),
        logical_digest: [0x43; 32],
        storage_root: [0x44; 32],
        physical_digest_schema: 9,
        history_digest_schema: 9,
        evidence_manifest_digest: [0x45; 32],
        durability: HistoryDurability::GroupCommit {
            max_ticks: 32,
            max_wall_time_ms: 250,
        },
    })
    .expect("test anchor is valid")
}

fn metadata(event_count: usize, completed_time: SimulationTime) -> BoundedSnapshotMetadata {
    BoundedSnapshotMetadata {
        runtime_recipe_fingerprint: [0x51; 32],
        physical_digest: [0x52; 32],
        anchor: anchor(event_count, completed_time),
    }
}

fn history(event_count: usize) -> Vec<CausalEventSnapshot> {
    (1..=event_count)
        .map(|raw| {
            let raw = u64::try_from(raw).expect("test event ordinal fits u64");
            CausalEventSnapshot {
                event_id: EventId::new(raw),
                trace_id: TraceId::new(raw),
                time: SimulationTime::new(1),
                phase: Phase::Physics,
                kind: EventKindId::new(1),
                causes: Vec::new(),
                effects: Vec::new(),
            }
        })
        .collect()
}

fn snapshot_with_history(event_count: usize) -> RuntimeSnapshotData {
    let mut snapshot = current_snapshot();
    snapshot.traces.next_event_id = u64::try_from(event_count).expect("count fits u64") + 1;
    snapshot.traces.next_trace_id = u64::try_from(event_count).expect("count fits u64") + 1;
    snapshot.traces.events = history(event_count);
    snapshot
}

fn current_snapshot() -> RuntimeSnapshotData {
    let runtime = current_runtime();
    runtime.export_snapshot().expect("snapshot exports")
}

fn current_runtime() -> Runtime {
    let mut runtime = Runtime::from_seed(4_004).expect("runtime constructs");
    runtime.tick().expect("tick commits");
    runtime
}

#[test]
fn snapshot_sections_bounded_revision_2_round_trips_through_runtime_import_api() {
    // Given: a real runtime snapshot and its generation-pinned durable history.
    let runtime = current_runtime();
    let bytes = runtime
        .export_bounded_envelope()
        .expect("bounded envelope")
        .encode()
        .expect("bounded bytes");
    let snapshot = runtime.export_snapshot().expect("snapshot exports");

    // When: revision 2 is resumed through the public runtime byte-import API.
    let history = runtime.into_history_resume().expect("history resume");
    let resumed = Runtime::from_bounded_snapshot_bytes(&bytes, history)
        .expect("bounded runtime import succeeds");
    let round_tripped = resumed.export_snapshot().expect("resumed snapshot exports");

    // Then: the bounded current state and cold causal history reconstruct exactly.
    let mut expected = snapshot;
    expected.traces.events.clear();
    assert_eq!(round_tripped, expected);
    assert!(round_tripped.traces.events.is_empty());
    println!(
        "BOUNDED_RUNTIME_IMPORT codec_revision=2 installed=true current_state_roundtrip=true trace_section_0x000A=false anchor_section_0x0010=1 bytes={}",
        bytes.len()
    );
}

#[test]
fn bounded_revision_2_borrowed_import_rejects_without_history_materialization() {
    let runtime = current_runtime();
    let bytes = runtime
        .export_bounded_envelope()
        .expect("bounded envelope")
        .encode()
        .expect("bounded bytes");
    let reader = runtime.history_reader().expect("history reader");

    let error = match Runtime::from_snapshot_bytes(&bytes, Some(reader.as_ref())) {
        Ok(_) => panic!("bounded import requires owned history resume"),
        Err(error) => error,
    };

    assert_eq!(
        error,
        causafera_runtime::RuntimeError::InvalidSnapshot(
            "bounded revision-2 snapshots require owned history resume"
        )
    );
    println!(
        "BOUNDED_BORROWED_IMPORT accepted=false materialized_history=false error=owned_resume_required"
    );
}

#[test]
fn snapshot_sections_corrupt_encoded_bounded_bytes_fail_before_runtime_install() {
    // Given: encoded revision-2 bytes that have a one-bit anchor payload corruption.
    let runtime = current_runtime();
    let mut envelope = runtime.export_bounded_envelope().expect("bounded envelope");
    envelope
        .sections
        .get_mut(&u64::from(SECTION_HISTORY_ANCHOR))
        .expect("anchor section")
        .bytes[40] ^= 0x01;
    let corrupted = envelope.encode().expect("structurally valid corrupt bytes");

    // When: the public runtime byte-import API validates the encoded artifact.
    let history = runtime.into_history_resume().expect("history resume");
    let error = match Runtime::from_bounded_snapshot_bytes(&corrupted, history) {
        Ok(_) => panic!("corrupt bounded bytes must not construct a runtime"),
        Err(error) => error,
    };

    // Then: the typed anchor-integrity error occurs before a runtime can be installed.
    assert!(matches!(
        error,
        causafera_runtime::RuntimeError::SnapshotAssembly(
            PersistenceError::HistoryAnchorDigestMismatch
        )
    ));
    println!(
        "CORRUPT_BOUNDED_RUNTIME_IMPORT encoded=true accepted=false installed=false error=HistoryAnchorDigestMismatch"
    );
}

#[test]
fn snapshot_sections_malformed_encoded_directory_shapes_fail_before_runtime_install() {
    // Given: one valid bounded artifact and its exact durable history reader.
    let runtime = current_runtime();
    let valid = runtime
        .export_bounded_envelope()
        .expect("bounded envelope")
        .encode()
        .expect("bounded bytes");
    let snapshot = runtime.export_snapshot().expect("snapshot exports");
    let decoded = SnapshotEnvelope::decode(&valid).expect("valid directory");
    let directory = usize::try_from(decoded.header.section_directory_offset)
        .expect("directory offset fits usize");

    let mut duplicate = valid.clone();
    let first_id: [u8; 8] = duplicate[directory..directory + 8]
        .try_into()
        .expect("schema ID bytes");
    let second = directory + SectionDirectoryEntry::SIZE;
    duplicate[second..second + 8].copy_from_slice(&first_id);

    let mut noncanonical = valid.clone();
    noncanonical[directory..directory + 8].copy_from_slice(&2_u64.to_le_bytes());
    noncanonical[second..second + 8].copy_from_slice(&1_u64.to_le_bytes());

    let mut oversized = valid.clone();
    let decoded_limit = directory + 32;
    oversized[decoded_limit..decoded_limit + 8].copy_from_slice(&1_u64.to_le_bytes());

    let mut mismatched_metadata = valid.clone();
    let anchor_entry = directory + (decoded.sections.len() - 1) * SectionDirectoryEntry::SIZE + 8;
    mismatched_metadata[anchor_entry..anchor_entry + 2].copy_from_slice(&2_u16.to_le_bytes());

    let mut trailing = valid;
    trailing.push(0);

    // When/Then: every malformed encoded shape is rejected before a Runtime value exists.
    let cases = [
        (
            "duplicate",
            duplicate,
            PersistenceError::DuplicateSection { schema_id: 1 },
        ),
        (
            "noncanonical",
            noncanonical,
            PersistenceError::NonCanonicalSectionOrder {
                previous: 2,
                current: 1,
            },
        ),
        (
            "oversized",
            oversized,
            PersistenceError::DecodedSizeLimitExceeded {
                schema_id: 1,
                limit: 1,
            },
        ),
        ("trailing", trailing, PersistenceError::TrailingBytes),
    ];
    for (case, bytes, expected) in cases {
        let error = match Runtime::from_bounded_snapshot_bytes(
            &bytes,
            current_runtime()
                .into_history_resume()
                .expect("history resume"),
        ) {
            Ok(_) => panic!("malformed {case} bytes must not construct a runtime"),
            Err(error) => error,
        };
        assert_eq!(
            error,
            causafera_runtime::RuntimeError::SnapshotAssembly(expected)
        );
        println!("MALFORMED_BOUNDED_RUNTIME_IMPORT case={case} accepted=false installed=false");
    }
    let metadata_error = match Runtime::from_bounded_snapshot_bytes(
        &mismatched_metadata,
        current_runtime()
            .into_history_resume()
            .expect("history resume"),
    ) {
        Ok(_) => panic!("mismatched metadata must not construct a runtime"),
        Err(error) => error,
    };
    assert!(matches!(
        metadata_error,
        causafera_runtime::RuntimeError::SnapshotAssembly(PersistenceError::Codec { .. })
    ));
    println!(
        "MALFORMED_BOUNDED_RUNTIME_IMPORT case=section_metadata accepted=false installed=false"
    );

    let stale_metadata = BoundedSnapshotMetadata {
        runtime_recipe_fingerprint: [0x51; 32],
        physical_digest: [0x52; 32],
        anchor: anchor(snapshot.traces.events.len(), snapshot.recipe.completed_time),
    };
    let stale_bytes = assemble_bounded_envelope(&snapshot, &stale_metadata)
        .expect("stale bounded envelope")
        .encode()
        .expect("stale bounded bytes");
    let stale_error = match Runtime::from_bounded_snapshot_bytes(
        &stale_bytes,
        current_runtime()
            .into_history_resume()
            .expect("history resume"),
    ) {
        Ok(_) => panic!("stale history reader must not construct a runtime"),
        Err(error) => error,
    };
    assert!(matches!(
        stale_error,
        causafera_runtime::RuntimeError::SnapshotAssembly(
            PersistenceError::InvalidHistoryAnchor { .. }
        )
    ));
    println!(
        "MALFORMED_BOUNDED_RUNTIME_IMPORT case=stale_history_reader accepted=false installed=false"
    );
}

#[test]
fn snapshot_sections_bounded_history_anchor_is_canonical_and_history_independent() {
    // Given: identical current state paired with short and long cold histories.
    let short = snapshot_with_history(100);
    let long = snapshot_with_history(1_000_000);
    let short_metadata = metadata(short.traces.events.len(), short.recipe.completed_time);
    let long_metadata = metadata(long.traces.events.len(), long.recipe.completed_time);

    // When: both states are transported in codec-revision-2 envelopes.
    let short_envelope =
        assemble_bounded_envelope(&short, &short_metadata).expect("short envelope");
    let long_envelope = assemble_bounded_envelope(&long, &long_metadata).expect("long envelope");
    let short_bytes = short_envelope.encode().expect("short bytes");
    let long_bytes = long_envelope.encode().expect("long bytes");
    let decoded = SnapshotEnvelope::decode(&short_bytes).expect("bounded envelope decodes");
    let decoded_anchor =
        history_anchor_from_bounded_envelope(&decoded).expect("history anchor validates");

    // Then: history bytes are absent, the anchor is exact, and size is history-independent.
    assert!(!short_envelope.sections.contains_key(&0x000A));
    assert!(!long_envelope.sections.contains_key(&0x000A));
    assert_eq!(
        short_envelope
            .sections
            .keys()
            .filter(|id| **id == 0x0010)
            .count(),
        1
    );
    assert_eq!(
        long_envelope
            .sections
            .keys()
            .filter(|id| **id == 0x0010)
            .count(),
        1
    );
    assert_eq!(decoded_anchor, short_metadata.anchor);
    assert_eq!(decoded.encode().expect("canonical re-encode"), short_bytes);
    let delta = short_bytes.len().abs_diff(long_bytes.len());
    assert!(delta <= 64 * 1024, "bounded envelope delta was {delta}");
    println!(
        "BOUNDED_HISTORY_ANCHOR short_events=100 short_bytes={} long_events=1000000 long_bytes={} delta_bytes={} limit_bytes=65536 trace_section_0x000A=false anchor_section_0x0010=1 anchor_major={}",
        short_bytes.len(),
        long_bytes.len(),
        delta,
        HISTORY_ANCHOR_SECTION_MAJOR
    );
}

#[test]
fn snapshot_sections_corrupt_history_anchor_is_rejected_before_state_install() {
    // Given: one valid bounded envelope and every contract-bearing corruption class.
    let snapshot = snapshot_with_history(100);
    let metadata = metadata(snapshot.traces.events.len(), snapshot.recipe.completed_time);
    let envelope = assemble_bounded_envelope(&snapshot, &metadata).expect("bounded envelope");
    let anchor_id = u64::from(SECTION_HISTORY_ANCHOR);
    let cases = [
        ("generation", 24_usize),
        ("checkpoint_digest", 40),
        ("frontier", 72),
        ("storage_root", 136),
    ];

    // When/Then: each malformed object fails validation before any runtime installation.
    for (case, offset) in cases {
        let mut corrupted = envelope.clone();
        corrupted
            .sections
            .get_mut(&anchor_id)
            .expect("anchor section")
            .bytes[offset] ^= 0x01;
        let error = history_anchor_from_bounded_envelope(&corrupted)
            .expect_err("corrupt anchor must fail closed");
        assert!(matches!(
            error,
            PersistenceError::HistoryAnchorDigestMismatch
        ));
        println!(
            "CORRUPT_HISTORY_ANCHOR case={case} accepted=false installed=false error={error:?}"
        );
    }

    let mut wrong_major = envelope.clone();
    wrong_major
        .sections
        .get_mut(&anchor_id)
        .expect("anchor section")
        .section_major += 1;
    assert!(history_anchor_from_bounded_envelope(&wrong_major).is_err());
    println!("CORRUPT_HISTORY_ANCHOR case=section_major accepted=false installed=false");

    let mut wrong_length = envelope;
    wrong_length
        .sections
        .get_mut(&anchor_id)
        .expect("anchor section")
        .bytes
        .pop();
    assert!(history_anchor_from_bounded_envelope(&wrong_length).is_err());
    println!("CORRUPT_HISTORY_ANCHOR case=payload_length accepted=false installed=false");
}
