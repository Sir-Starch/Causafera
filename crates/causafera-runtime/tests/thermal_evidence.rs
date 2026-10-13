use std::collections::BTreeSet;

use causafera_core::{
    Criticality, DomainEvidenceRecord, HistoryQuery, HistoryRecord, StateFingerprint,
};
use causafera_persistence::logical_history_digest;
use causafera_runtime::{Runtime, RuntimeConfig};
use causafera_types::TraceId;

const TRANSFER_SCHEMA: u64 = 0x0014_1701;
const CONSERVATION_SCHEMA: u64 = 0x0014_1702;
const BOUNDARY_SCHEMA: u64 = 0x0014_1703;

fn all_history_records(runtime: &Runtime) -> Vec<HistoryRecord> {
    let reader = runtime
        .history_reader()
        .expect("history reader must be available");
    let mut offset = 0;
    let mut records = Vec::new();
    loop {
        let page = reader
            .indexed_range(HistoryQuery::new(offset, 256, 4 * 1024 * 1024).expect("bounded query"))
            .expect("committed history page must be readable");
        records.extend_from_slice(page.records());
        match page.next_offset() {
            Some(next) => offset = next,
            None => return records,
        }
    }
}

fn thermal_evidence(runtime: &Runtime) -> Vec<DomainEvidenceRecord> {
    all_history_records(runtime)
        .into_iter()
        .filter_map(|record| match record {
            HistoryRecord::Causal(_) => None,
            HistoryRecord::Evidence(evidence) => Some(evidence),
        })
        .filter(|record| {
            matches!(
                record.schema_id().raw(),
                TRANSFER_SCHEMA | CONSERVATION_SCHEMA | BOUNDARY_SCHEMA
            )
        })
        .collect()
}

#[test]
fn committed_thermal_batch_has_canonical_history_evidence() {
    // Given: a production runtime whose first tick executes a real thermal batch.
    let mut runtime = Runtime::new(RuntimeConfig::new(1_703)).expect("runtime must bootstrap");
    let before = runtime.history_status().expect("initial history status");

    // When: the enclosing tick transaction commits and the public reader pages that generation.
    runtime.tick().expect("thermal batch must commit");
    let after = runtime.history_status().expect("committed history status");
    let evidence = thermal_evidence(&runtime);

    // Then: that one enclosing commit exposes all thermal schemas with canonical metadata.
    assert_eq!(after.logical_generation, before.logical_generation + 1);
    assert!(
        !evidence.is_empty(),
        "committed thermal batch has no canonical HistoryRecord::Evidence"
    );
    assert_eq!(
        evidence
            .iter()
            .map(|record| record.schema_id().raw())
            .collect::<BTreeSet<_>>(),
        BTreeSet::from([TRANSFER_SCHEMA, CONSERVATION_SCHEMA, BOUNDARY_SCHEMA])
    );
    for record in &evidence {
        assert_eq!((record.version().major, record.version().minor), (1, 0));
        assert_eq!(record.criticality(), Criticality::ExplanationOnly);
        assert!(!record.causal_anchors().is_empty());
        assert!(
            record
                .causal_anchors()
                .windows(2)
                .all(|window| window[0] < window[1])
        );
        assert!(!record.canonical_key().is_empty());
        assert!(!record.canonical_payload().is_empty());
        assert_eq!(
            record.payload_digest().bytes(),
            *blake3::hash(record.canonical_payload()).as_bytes()
        );
    }
    println!(
        "MARKER:thermal_evidence same_frame=true schemas=3 records={} criticality=ExplanationOnly",
        evidence.len()
    );
}

#[test]
fn evicted_thermal_projection_remains_queryable_from_cold_history() {
    // Given: the first committed transfer record and its still-hot typed projection.
    let mut runtime = Runtime::new(RuntimeConfig::new(1_704)).expect("runtime must bootstrap");
    runtime.tick().expect("first thermal batch must commit");
    let first = thermal_evidence(&runtime)
        .into_iter()
        .find(|record| record.schema_id().raw() == TRANSFER_SCHEMA)
        .expect("first batch must emit transfer evidence");
    let first_batch = TraceId::new(first.commit_sequence());

    // When: enough later batches commit to evict the first typed projection and flush its group.
    runtime
        .run_ticks(65)
        .expect("later thermal batches must commit");
    let snapshot = runtime.export_snapshot().expect("snapshot must export");
    let status = runtime.history_status().expect("history status");
    let cold = runtime
        .history_reader()
        .expect("history reader")
        .evidence(
            first.schema_id(),
            first.canonical_key(),
            HistoryQuery::new(0, 4, 4 * 1024 * 1024).expect("bounded query"),
        )
        .expect("cold evidence query must succeed");

    // Then: the bounded snapshot no longer carries that batch, while the indexed record is exact.
    assert_eq!(snapshot.thermal.receipt_batches.len(), 64);
    assert!(!snapshot.thermal.receipt_batches.contains(&first_batch));
    assert!(status.durable_generation >= 32);
    assert_eq!(cold.records(), std::slice::from_ref(&first));
    println!(
        "MARKER:thermal_evidence cold_after_eviction=true projection_batches={} cold_records={}",
        snapshot.thermal.receipt_batches.len(),
        cold.records().len()
    );
}

#[test]
fn authoritative_thermal_payload_changes_logical_digest() {
    // Given: one committed canonical thermal conservation record.
    let mut runtime = Runtime::new(RuntimeConfig::new(1_705)).expect("runtime must bootstrap");
    runtime.tick().expect("thermal batch must commit");
    let original = thermal_evidence(&runtime)
        .into_iter()
        .find(|record| record.schema_id().raw() == CONSERVATION_SCHEMA)
        .expect("batch must emit conservation evidence");

    // When: one authoritative payload byte changes with a correspondingly valid payload digest.
    let mut changed_payload = original.canonical_payload().to_vec();
    changed_payload[0] ^= 1;
    let changed = DomainEvidenceRecord::new(
        original.commit_sequence(),
        original.schema_id(),
        original.version(),
        original.criticality(),
        original.time(),
        original.causal_anchors().to_vec(),
        original.canonical_key().to_vec(),
        changed_payload.clone(),
        StateFingerprint::new(*blake3::hash(&changed_payload).as_bytes()),
    )
    .expect("changed record remains structurally canonical");
    let original_digest =
        logical_history_digest(&[HistoryRecord::Evidence(original)]).expect("original digest");
    let changed_digest =
        logical_history_digest(&[HistoryRecord::Evidence(changed)]).expect("changed digest");

    // Then: the canonical logical identity is payload-sensitive.
    assert_ne!(original_digest, changed_digest);
    println!("MARKER:thermal_evidence payload_digest_sensitive=true");
}
