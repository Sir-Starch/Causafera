//! Digest schema 9 at the snapshot boundary.
//!
//! A bounded snapshot is resumable only if everything its header and anchor
//! claim is recomputed and agrees before the runtime is handed back. Each case
//! below alters exactly one claim and asserts a typed rejection; the intact
//! snapshot still resumes afterwards, so none of the rejections consumed or
//! damaged the history handle.
//!
//! See `plans/causal-history-storage.md` Todo 7.

use causafera_core::{HistoryAnchor, HistoryAnchorFields};
use causafera_persistence::{
    PersistenceError, SnapshotEnvelope, SnapshotHeader, encode_history_anchor,
};
use causafera_runtime::{
    CURRENT_DIGEST_SCHEMA_VERSION, HISTORY_ANCHOR_SECTION_MAJOR, Runtime, RuntimeConfig,
    RuntimeError, RuntimeHistoryResume, SECTION_HISTORY_ANCHOR, SECTION_MANA_FIELDS,
    history_anchor_from_bounded_envelope, runtime_recipe_fingerprint,
};

const SECTION_CAUSAL_TRACES: u64 = 0x000A;

fn fields_of(anchor: &HistoryAnchor) -> HistoryAnchorFields {
    HistoryAnchorFields {
        namespace: *anchor.namespace(),
        logical_generation: anchor.logical_generation(),
        durable_generation: anchor.durable_generation(),
        checkpoint_digest: *anchor.checkpoint_digest(),
        frontier: anchor.frontier(),
        logical_digest: *anchor.logical_digest(),
        storage_root: *anchor.storage_root(),
        physical_digest_schema: anchor.physical_digest_schema(),
        history_digest_schema: anchor.history_digest_schema(),
        evidence_manifest_digest: *anchor.evidence_manifest_digest(),
        durability: anchor.durability(),
    }
}

/// Re-encode `envelope` after `edit`, so the container's own integrity check
/// passes and only the claim under test is false.
fn tampered(
    envelope: &SnapshotEnvelope,
    edit: impl FnOnce(&mut SnapshotHeader, &mut HistoryAnchorFields),
) -> Vec<u8> {
    let mut header = envelope.header;
    let mut sections = envelope.sections.clone();
    let mut fields = fields_of(&history_anchor_from_bounded_envelope(envelope).expect("anchor"));
    edit(&mut header, &mut fields);
    let anchor = HistoryAnchor::new(fields).expect("the tampered anchor is still well formed");
    sections
        .get_mut(&u64::from(SECTION_HISTORY_ANCHOR))
        .expect("anchor section")
        .bytes = encode_history_anchor(&anchor).expect("anchor encodes");
    SnapshotEnvelope::new(header, sections)
        .encode()
        .expect("tampered envelope encodes")
}

fn rejection(bytes: &[u8], resume: &RuntimeHistoryResume) -> RuntimeError {
    match Runtime::from_bounded_snapshot_bytes(bytes, resume.clone()) {
        Ok(_) => panic!("a tampered bounded snapshot resumed"),
        Err(error) => error,
    }
}

fn flip(bytes: &mut [u8; 32]) {
    bytes[0] ^= 1;
}

fn saved_session() -> (SnapshotEnvelope, Vec<u8>, RuntimeHistoryResume) {
    let mut runtime = Runtime::new(RuntimeConfig::new(9_009)).expect("session constructs");
    runtime.run_ticks(12).expect("session runs");
    let envelope = runtime.export_bounded_envelope().expect("bounded envelope");
    let bytes = envelope.encode().expect("bounded bytes");
    let resume = runtime.into_history_resume().expect("history resume");
    (envelope, bytes, resume)
}

#[test]
fn a_schema_9_snapshot_carries_the_anchor_and_no_trace_section() {
    let (envelope, _, _) = saved_session();

    assert_eq!(
        envelope.header.physical_digest_schema,
        CURRENT_DIGEST_SCHEMA_VERSION.raw()
    );
    assert_eq!(
        envelope.header.history_digest_schema,
        CURRENT_DIGEST_SCHEMA_VERSION.raw()
    );
    assert!(
        envelope
            .sections
            .contains_key(&u64::from(SECTION_HISTORY_ANCHOR))
    );
    assert!(!envelope.sections.contains_key(&SECTION_CAUSAL_TRACES));
    assert_eq!(
        envelope.header.runtime_recipe_fingerprint,
        runtime_recipe_fingerprint(&RuntimeConfig::new(9_009)),
        "the header names the recipe instead of a zero placeholder"
    );
    assert_ne!(envelope.header.runtime_recipe_fingerprint, [0; 32]);
    let anchor = history_anchor_from_bounded_envelope(&envelope).expect("anchor");
    assert_eq!(envelope.header.history_digest, *anchor.logical_digest());
    println!(
        "MARKER:task7 schema=9 anchor_section=0x{:04x} anchor_major={} trace_section_0x000A=false \
         recipe_fingerprint_set=true",
        SECTION_HISTORY_ANCHOR, HISTORY_ANCHOR_SECTION_MAJOR
    );
}

#[test]
fn every_false_claim_in_a_bounded_snapshot_is_refused_before_a_runtime_exists() {
    let (envelope, bytes, resume) = saved_session();
    let expect_anchor_mismatch = |error: &RuntimeError| {
        matches!(
            error,
            RuntimeError::SnapshotAssembly(PersistenceError::InvalidHistoryAnchor { .. })
        )
    };
    let mut recorded = Vec::new();
    let mut check = |case: &str, bytes: Vec<u8>, accepts: &dyn Fn(&RuntimeError) -> bool| {
        let error = rejection(&bytes, &resume);
        assert!(
            accepts(&error),
            "{case} was refused with the wrong error: {error}"
        );
        println!(
            "MARKER:task7_failure {{\"case\":\"{case}\",\"outcome\":\"rejected\",\"error\":{:?}}}",
            error.to_string()
        );
        recorded.push(case.to_owned());
    };

    // Header claims about the state the sections describe.
    check(
        "physical_digest_bit_flipped",
        tampered(&envelope, |header, _| flip(&mut header.physical_digest)),
        &|error| {
            matches!(error, RuntimeError::InvalidSnapshot(message)
                if message.contains("physical digest does not match"))
        },
    );
    check(
        "recipe_fingerprint_bit_flipped",
        tampered(&envelope, |header, _| {
            flip(&mut header.runtime_recipe_fingerprint);
        }),
        &|error| {
            matches!(error, RuntimeError::InvalidSnapshot(message)
                if message.contains("recipe fingerprint"))
        },
    );

    // Header claims about the history the anchor names.
    check(
        "history_digest_bit_flipped",
        tampered(&envelope, |header, _| flip(&mut header.history_digest)),
        &expect_anchor_mismatch,
    );
    check(
        "completed_time_advanced",
        tampered(&envelope, |header, _| header.completed_time += 1),
        &|error| {
            expect_anchor_mismatch(error)
                || matches!(
                    error,
                    RuntimeError::SnapshotAssembly(PersistenceError::Codec { .. })
                )
        },
    );

    // The anchor against the history the reader actually holds.
    check(
        "anchor_storage_root_bit_flipped",
        tampered(&envelope, |_, anchor| flip(&mut anchor.storage_root)),
        &expect_anchor_mismatch,
    );
    check(
        "anchor_evidence_manifest_bit_flipped",
        tampered(&envelope, |_, anchor| {
            flip(&mut anchor.evidence_manifest_digest);
        }),
        &expect_anchor_mismatch,
    );
    check(
        "anchor_generation_advanced",
        tampered(&envelope, |_, anchor| {
            anchor.logical_generation += 1;
            anchor.durable_generation += 1;
            anchor.frontier = causafera_core::HistoryFrontier::new(
                anchor.logical_generation,
                anchor.frontier.event_id,
                anchor.frontier.trace_id,
                anchor.frontier.time,
            );
        }),
        &expect_anchor_mismatch,
    );
    check(
        "anchor_frontier_trace_moved",
        tampered(&envelope, |_, anchor| {
            anchor.frontier = causafera_core::HistoryFrontier::new(
                anchor.frontier.generation,
                anchor.frontier.event_id,
                causafera_types::TraceId::new(anchor.frontier.trace_id.raw() + 1),
                anchor.frontier.time,
            );
        }),
        &expect_anchor_mismatch,
    );

    // A current-state section that no longer matches the header's digest.
    let mut sections = envelope.sections.clone();
    let mana = sections
        .get_mut(&u64::from(SECTION_MANA_FIELDS))
        .expect("mana section");
    let last = mana.bytes.len() - 1;
    mana.bytes[last] ^= 1;
    check(
        "current_state_section_altered",
        SnapshotEnvelope::new(envelope.header, sections)
            .encode()
            .expect("altered envelope encodes"),
        &|error| {
            matches!(error, RuntimeError::InvalidSnapshot(message)
                if message.contains("physical digest does not match"))
        },
    );

    // Intact bytes still resume through the same handle after all of that.
    let resumed = Runtime::from_bounded_snapshot_bytes(&bytes, resume.clone())
        .expect("the intact snapshot resumes after every refusal");
    assert_eq!(resumed.current_time().raw(), 12);
    assert_eq!(recorded.len(), 9);
}

#[test]
fn the_checkpoint_pointer_in_an_anchor_is_navigation_and_not_identity() {
    // Given: a snapshot whose anchor points at a different checkpoint.
    let (envelope, _, resume) = saved_session();
    let bytes = tampered(&envelope, |_, anchor| flip(&mut anchor.checkpoint_digest));

    // Then: it still resumes at the runtime boundary, because a snapshot saved
    // into a checkpoint cannot name that checkpoint and so carries the one before
    // it, while the history at the same generation already points at the new one.
    // The pointer is checked against the catalog when a bundle is opened.
    Runtime::from_bounded_snapshot_bytes(&bytes, resume)
        .map(|_| ())
        .expect("the pointer is not part of the history's identity");
}
