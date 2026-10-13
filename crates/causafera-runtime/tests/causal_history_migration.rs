use std::path::PathBuf;
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};

use causafera_core::EvidenceDecoderRegistry;
use causafera_explanation::{ClaimEvidenceState, HYDROLOGY_CONSERVATION_RESIDUAL_SCHEMA};
use causafera_persistence::{DurabilityMode, HistoryStore, SnapshotEnvelope};
use causafera_runtime::{
    BundleOptions, LegacyMigrationProof, Runtime, SourceCompleteness,
    register_hydrology_evidence_decoders,
};

const LEGACY: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/causal-history/schema-8-baseline.otps"
);

fn hex(bytes: &[u8; 32]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

struct TempDestination(PathBuf);

impl TempDestination {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "causafera-legacy-migration-p{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        Self(path)
    }
}

impl Drop for TempDestination {
    fn drop(&mut self) {
        if self.0.is_dir() {
            let _ = std::fs::remove_dir_all(&self.0);
        } else {
            let _ = std::fs::remove_file(&self.0);
        }
    }
}

#[test]
fn checked_in_schema_eight_migrates_to_a_resumable_bundle() {
    // Given: the byte-pinned legacy snapshot from before the current writer.
    let destination = TempDestination::new();
    let source_before = std::fs::read(LEGACY).expect("legacy fixture");

    // When: migration publishes a bundle and the session advances one tick.
    let mut migrated = Runtime::migrate_legacy_snapshot(
        LEGACY.as_ref(),
        &destination.0,
        BundleOptions {
            durability: DurabilityMode::EveryTick,
            ..BundleOptions::default()
        },
    )
    .expect("legacy migration");
    assert_eq!(migrated.current_time().raw(), 4);
    assert_eq!(
        migrated
            .history_reader()
            .expect("history reader")
            .all_records_for_oracle()
            .expect("complete migrated causal graph")
            .len(),
        37_862
    );
    let proof = LegacyMigrationProof::read(&destination.0)
        .expect("proof read")
        .expect("migration proof");
    assert_eq!(proof.algorithm_version, 1);
    assert_eq!(
        proof.source_envelope_digest,
        *blake3::hash(&source_before).as_bytes()
    );
    assert_eq!(proof.source_causal_events, 37_862);
    assert_eq!(proof.destination_causal_events, 37_862);
    assert_eq!(
        hex(&proof.source_physical_digest_v8),
        "1cab3de3a202057558da26d7c61cede281edf6957757fe3cdef5e1957095c2b9"
    );
    assert_eq!(
        hex(&proof.source_history_digest_v8),
        "03615361b7d0d1e710b8a400bc4a4db2cce2ac230fe6c29443561a831169f9f9"
    );
    assert_eq!(
        hex(&proof.source_experiment_digest_v8),
        "be4c690216fd479583900c1e725208e9e8228ea9531d83f51672515a7984ccc4"
    );
    assert_eq!(proof.coverage.len(), 11);
    let coverage = |field: &str| {
        proof
            .coverage
            .iter()
            .find(|entry| entry.source_field == field)
            .expect("coverage entry")
    };
    assert_eq!(coverage("hydrology.receipts").copied_count, 12_629);
    assert_eq!(coverage("hydrology.conservation_receipts").copied_count, 4);
    assert_eq!(coverage("hydrology.forcing").copied_count, 1);
    assert_eq!(coverage("thermal.transfer_receipts").copied_count, 41);
    assert_eq!(coverage("thermal.conservation_receipts").copied_count, 4);
    assert_eq!(coverage("thermal.boundary_records").copied_count, 54);
    assert_eq!(coverage("material_surfaces.transitions").copied_count, 9);
    assert_eq!(
        coverage("material_surfaces.thermal_transitions").copied_count,
        4
    );
    assert_eq!(coverage("bootstrap.receipts").copied_count, 7);
    assert!(
        !coverage("material_surfaces.transitions")
            .missing_ranges
            .is_empty()
    );
    assert_eq!(
        coverage("material_surfaces.transitions").source_completeness,
        SourceCompleteness::Unknown
    );
    assert_eq!(
        coverage("hydrology.receipts").source_completeness,
        SourceCompleteness::Partial
    );
    assert_eq!(
        coverage("hydrology.forcing").source_completeness,
        SourceCompleteness::Complete
    );
    assert_eq!(
        coverage("hydrology.receipts").first_guaranteed_typed_tick,
        Some(1)
    );
    assert_eq!(
        proof.destination_evidence_records,
        proof
            .coverage
            .iter()
            .map(|entry| entry.copied_count)
            .sum::<u64>()
    );
    let migrated_status = migrated.history_status().expect("history status");
    let legacy_batch = migrated.hydrology_state().retained_batches[0];
    migrated.tick().expect("resumed tick");
    migrated.save_bundle().expect("new snapshot");
    migrated.shutdown().expect("clean shutdown");
    let duplicate =
        Runtime::migrate_legacy_snapshot(LEGACY.as_ref(), &destination.0, BundleOptions::default())
            .expect("same source is idempotent");
    assert_eq!(duplicate.current_time().raw(), 5);
    duplicate.shutdown().expect("duplicate shutdown");
    let (mut reopened, recovery) =
        Runtime::open_bundle(&destination.0).expect("reopen migrated bundle");

    // Then: the old file is untouched and every causal event is in durable history.
    assert_eq!(std::fs::read(LEGACY).expect("source after"), source_before);
    assert_eq!(reopened.current_time().raw(), 5);
    assert_eq!(recovery.lost, None);
    reopened.run_ticks(9).expect("evict legacy typed batch");
    assert!(
        !reopened
            .hydrology_state()
            .retained_batches
            .contains(&legacy_batch)
    );
    let historical = reopened
        .observer_hydrology_explanation_for_batch(legacy_batch, None)
        .expect("historical explanation returns a bounded answer");
    let conservation = historical.frames[0]
        .claims
        .iter()
        .find(|claim| claim.schema == HYDROLOGY_CONSERVATION_RESIDUAL_SCHEMA)
        .expect("conservation claim");
    assert_eq!(conservation.evidence_state, ClaimEvidenceState::Unknown);
    reopened
        .shutdown()
        .expect("shutdown after historical query");
    let mut registry = EvidenceDecoderRegistry::new();
    register_hydrology_evidence_decoders(&mut registry).expect("hydrology decoders");
    let store =
        HistoryStore::open_with_registry(&destination.0, Arc::new(registry)).expect("cold history");
    let counts = store.catalog().counts().expect("catalog counts");
    assert!(counts.events > 37_862);
    assert!(counts.evidence > 0);
    assert!(migrated_status.durable_generation > 0);
    drop(store);
    let reference = destination
        .0
        .join("migration")
        .join(format!("{}.proof", hex(&proof.source_envelope_digest)));
    let proof_bytes = std::fs::read(&reference).expect("proof bytes");
    let mut corrupted = proof_bytes.clone();
    corrupted[0] ^= 1;
    std::fs::write(&reference, &corrupted).expect("corrupt proof");
    assert!(Runtime::open_bundle(&destination.0).is_err());
    std::fs::write(&reference, &proof_bytes).expect("restore proof");
    std::fs::remove_file(&reference).expect("delete proof for negative check");
    assert!(Runtime::open_bundle(&destination.0).is_err());
}

#[test]
fn migration_rejects_mutation_and_existing_destination_without_publishing() {
    let source = TempDestination::new();
    let destination = TempDestination::new();
    let mut bytes = std::fs::read(LEGACY).expect("fixture");
    bytes.pop();
    std::fs::write(&source.0, &bytes).expect("truncated source");
    assert!(
        Runtime::migrate_legacy_snapshot(&source.0, &destination.0, BundleOptions::default())
            .is_err()
    );
    assert!(!destination.0.exists());

    let mut envelope =
        SnapshotEnvelope::decode(&std::fs::read(LEGACY).expect("fixture")).expect("envelope");
    envelope.header.physical_digest[0] ^= 1;
    std::fs::write(&source.0, envelope.encode().expect("mutated envelope"))
        .expect("mutated source");
    assert!(
        Runtime::migrate_legacy_snapshot(&source.0, &destination.0, BundleOptions::default())
            .is_err()
    );
    assert!(!destination.0.exists());

    envelope.header.physical_digest[0] ^= 1;
    envelope.header.world_seed ^= 1;
    std::fs::write(
        &source.0,
        envelope.encode().expect("wrong lineage envelope"),
    )
    .expect("wrong lineage source");
    assert!(
        Runtime::migrate_legacy_snapshot(&source.0, &destination.0, BundleOptions::default())
            .is_err()
    );
    assert!(!destination.0.exists());

    envelope.header.world_seed ^= 1;
    envelope
        .sections
        .get_mut(&0x000F)
        .expect("hydrology section")
        .section_major = 2;
    std::fs::write(
        &source.0,
        envelope.encode().expect("unsupported matrix envelope"),
    )
    .expect("unsupported matrix source");
    assert!(
        Runtime::migrate_legacy_snapshot(&source.0, &destination.0, BundleOptions::default())
            .is_err()
    );
    assert!(!destination.0.exists());

    std::fs::write(&destination.0, b"untouched").expect("existing destination");
    assert!(
        Runtime::migrate_legacy_snapshot(LEGACY.as_ref(), &destination.0, BundleOptions::default())
            .is_err()
    );
    assert_eq!(
        std::fs::read(&destination.0).expect("destination"),
        b"untouched"
    );
}
