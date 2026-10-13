mod support_hydrology;

use causafera_core::{
    Criticality, DomainEvidenceRecord, EvidenceDecoderRegistry, EvidenceResolution,
    EvidenceSchemaId, HistoryError, HistoryQuery, HistoryRecord, StateFingerprint,
};
use causafera_persistence::logical_history_digest;
use causafera_runtime::{
    HYDROLOGY_CONSERVATION_EVIDENCE_SCHEMA, HYDROLOGY_EVIDENCE_VERSION, Runtime,
    hydrology_bootstrap_evidence, register_hydrology_evidence_decoders,
};
use causafera_types::{SimulationTime, TraceId};

use support_hydrology::enabled_runtime_config;

#[test]
fn committed_hydrology_tick_emits_canonical_evidence() {
    // Given: an enabled world with scheduled forcing and physical water.
    let mut runtime = Runtime::new(enabled_runtime_config()).expect("enabled runtime");

    // When: the forcing tick commits through the production transaction.
    runtime.tick().expect("hydrology tick commits");
    let reader = runtime.history_reader().expect("history reader");
    // Then: canonical hydrology evidence, not just hot typed receipts, is visible.
    let mut evidence = Vec::new();
    let mut offset = 0;
    loop {
        let page = reader
            .indexed_range(HistoryQuery::new(offset, 256, 4 * 1024 * 1024).expect("bounded page"))
            .expect("committed page");
        evidence.extend(page.records().iter().filter_map(|record| match record {
            HistoryRecord::Evidence(record)
                if (0x0014_1801..=0x0014_1805).contains(&record.schema_id().raw()) =>
            {
                Some(record.clone())
            }
            HistoryRecord::Causal(_) | HistoryRecord::Evidence(_) => None,
        }));
        match page.next_offset() {
            Some(next) => offset = next,
            None => break,
        }
    }
    assert!(
        !evidence.is_empty(),
        "committed hydrology tick has no canonical evidence"
    );
    for record in evidence {
        assert_eq!(record.version().major, 1);
        assert_eq!(record.criticality(), Criticality::ReplayRequired);
        assert!(!record.causal_anchors().is_empty());
        assert!(!record.canonical_key().is_empty());
        assert!(!record.canonical_payload().is_empty());
    }
}

#[test]
fn bootstrap_decoder_rejects_malformed_required_evidence_and_digest_is_sensitive() {
    // Given: one real bootstrap hydrology payload and the hydrology-owned registry.
    let runtime = Runtime::new(enabled_runtime_config()).expect("enabled runtime");
    let original = hydrology_bootstrap_evidence(
        &runtime.hydrology_state(),
        SimulationTime::new(0),
        TraceId::new(1),
    )
    .expect("bootstrap evidence encoding")
    .expect("enabled hydrology evidence");
    let mut registry = EvidenceDecoderRegistry::new();
    register_hydrology_evidence_decoders(&mut registry).expect("unique decoder registration");

    // When: the canonical record and adversarial variants cross the decoder boundary.
    assert!(matches!(
        registry.resolve(&original),
        Ok(EvidenceResolution::Decoded(_))
    ));
    assert!(matches!(
        EvidenceDecoderRegistry::new().resolve(&original),
        Err(HistoryError::UnsupportedReplaySchema { .. })
    ));
    let altered = |schema: EvidenceSchemaId, anchor: TraceId, payload: Vec<u8>| {
        let digest = StateFingerprint::new(*blake3::hash(&payload).as_bytes());
        DomainEvidenceRecord::new(
            original.commit_sequence(),
            schema,
            original.version(),
            original.criticality(),
            original.time(),
            vec![anchor],
            original.canonical_key().to_vec(),
            payload,
            digest,
        )
        .expect("structurally valid evidence")
    };
    let malformed = altered(original.schema_id(), TraceId::new(1), vec![0]);
    let new_schema = altered(
        EvidenceSchemaId::new(0x0014_18ff),
        TraceId::new(1),
        original.canonical_payload().to_vec(),
    );
    let new_anchor = altered(
        original.schema_id(),
        TraceId::new(2),
        original.canonical_payload().to_vec(),
    );
    let mut new_bytes = original.canonical_payload().to_vec();
    new_bytes[0] ^= 1;
    let new_payload = altered(original.schema_id(), TraceId::new(1), new_bytes);
    let digest = |record: DomainEvidenceRecord| {
        logical_history_digest(&[HistoryRecord::Evidence(record)]).expect("digest")
    };
    let baseline = digest(original.clone());

    // Then: decoding fails closed and all three canonical dimensions change identity.
    assert!(registry.resolve(&malformed).is_err());
    assert!(matches!(
        registry.resolve(&new_schema),
        Err(HistoryError::UnsupportedReplaySchema { .. })
    ));
    assert!(matches!(
        DomainEvidenceRecord::new(
            original.commit_sequence(),
            original.schema_id(),
            original.version(),
            original.criticality(),
            original.time(),
            vec![TraceId::new(1), TraceId::new(1)],
            original.canonical_key().to_vec(),
            original.canonical_payload().to_vec(),
            original.payload_digest(),
        ),
        Err(HistoryError::AnchorsNotStrictlyOrdered { .. })
    ));
    assert_ne!(baseline, digest(new_schema));
    assert_ne!(baseline, digest(new_anchor));
    assert_ne!(baseline, digest(new_payload));
}

#[test]
fn conservation_decoder_rejects_unbalanced_raw_terms() {
    // Given: a structurally valid zero-balance hydrology conservation payload.
    let mut registry = EvidenceDecoderRegistry::new();
    register_hydrology_evidence_decoders(&mut registry).expect("decoder registration");
    let mut payload = vec![0_u8; 208];
    payload[..8].copy_from_slice(&1_u64.to_le_bytes());
    payload[8..16].copy_from_slice(&1_u64.to_le_bytes());
    let record = |payload: Vec<u8>| {
        let digest = StateFingerprint::new(*blake3::hash(&payload).as_bytes());
        DomainEvidenceRecord::new(
            1,
            HYDROLOGY_CONSERVATION_EVIDENCE_SCHEMA,
            HYDROLOGY_EVIDENCE_VERSION,
            Criticality::ReplayRequired,
            SimulationTime::new(1),
            vec![TraceId::new(1)],
            1_u64.to_le_bytes().to_vec(),
            payload,
            digest,
        )
        .expect("canonical record")
    };

    // When: one after-storage term is altered without a balancing source.
    let balanced = record(payload.clone());
    payload[80] = 1;
    let unbalanced = record(payload);

    // Then: the registered decoder accepts the first and rejects the second.
    assert!(matches!(
        registry.resolve(&balanced),
        Ok(EvidenceResolution::Decoded(_))
    ));
    assert!(registry.resolve(&unbalanced).is_err());
}
