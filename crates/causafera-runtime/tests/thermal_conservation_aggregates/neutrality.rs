use causafera_runtime::{Runtime, RuntimeState};

use super::support::{FIXTURE_SEED, FIXTURE_TICKS, fixture, runtime_config};

fn hex32(bytes: [u8; 32]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

// Re-recorded for `CURRENT_DIGEST_SCHEMA_VERSION` 7 using the same fixture
// (seed 2026, 7 ticks). The originals were recorded at Wave 1 commit 9ddba30
// against schema 6 and were byte-identical across the thermal-conservation
// aggregate validation change, which is what this gate existed to prove.
//
// `plans/production-bootstrap-receipt-closure.md` moves them deliberately: the
// canonical bootstrap record is now an authoritative `physical_state_digest`
// input, and the terminal stage-completion events are new committed trace events
// that `history_digest` folds in. Both moves are declared contract changes, not
// drift, so the gate keeps its teeth by pinning the new values rather than being
// relaxed to a self-comparison.
const FIXTURE_PHYSICAL_DIGEST: &str =
    "66492e72be44a875b72007a62d548ff14e4bc27eb52fc1edf2382dcbde0ac1a4";
const FIXTURE_HISTORY_DIGEST: &str =
    "66c48e763035e033d7eadd64e8ab04f93c5199b7d16db3fcab00deeea1f2854d";

#[test]
fn aggregate_validation_verdict_is_input_order_independent() {
    // Given: a valid engine-produced snapshot.
    let mut permuted = fixture();

    // Find two transfer receipts with the same conservation trace but different cells.
    let first_index = permuted
        .thermal
        .transfer_receipts
        .iter()
        .enumerate()
        .find(|(_, receipt)| {
            receipt.conservation_trace == permuted.thermal.conservation_receipts[0].trace
        })
        .map(|(index, _)| index)
        .expect("fixture must have a receipt in batch 0");
    let second_index = permuted
        .thermal
        .transfer_receipts
        .iter()
        .enumerate()
        .skip(first_index + 1)
        .find(|(_, receipt)| {
            receipt.conservation_trace == permuted.thermal.conservation_receipts[0].trace
        })
        .map(|(index, _)| index)
        .expect("fixture must have a second receipt in batch 0");

    // When: those two receipts are swapped (same multiset, different order).
    permuted
        .thermal
        .transfer_receipts
        .swap(first_index, second_index);

    // Then: both the original and the permuted snapshot import successfully. The accumulator
    // fold is keyed by conservation trace, so the accept/reject verdict is independent of the
    // receipt encounter order. Physical and history digests intentionally remain order-sensitive
    // because the snapshot format preserves the receipt vector order; the invariants this plan
    // enforces are at the accumulator level and are unit-tested in
    // thermal_conservation_validation::tests.
    assert!(
        RuntimeState::import_snapshot(fixture()).is_ok(),
        "original fixture must import"
    );
    assert!(
        RuntimeState::import_snapshot(permuted).is_ok(),
        "permuted receipts must produce the same import verdict"
    );
}

#[test]
fn save_resume_equivalence_under_aggregate_validation() {
    // Given: a valid engine-produced snapshot.
    let data = fixture();

    // When: the snapshot is imported and re-exported.
    let round_tripped = Runtime::from_snapshot(data.clone())
        .expect("snapshot must resume")
        .export_snapshot()
        .expect("re-exported snapshot must succeed");

    // Then: the re-exported snapshot is byte-identical to the original.
    assert_eq!(
        data, round_tripped,
        "export-import-export must produce identical snapshot data"
    );

    // And: continuing from the resumed state matches an uninterrupted run.
    const CONTINUATION_TICKS: u64 = 3;
    let mut uninterrupted =
        Runtime::new(runtime_config(FIXTURE_SEED)).expect("runtime bootstrap must succeed");
    uninterrupted
        .run_ticks(FIXTURE_TICKS + CONTINUATION_TICKS)
        .expect("uninterrupted run must complete");
    let uninterrupted_snapshot = uninterrupted
        .snapshot()
        .expect("uninterrupted snapshot must succeed");

    let mut resumed = Runtime::from_snapshot(data).expect("snapshot must resume");
    resumed
        .run_ticks(CONTINUATION_TICKS)
        .expect("resumed run must complete");
    let resumed_snapshot = resumed.snapshot().expect("resumed snapshot must succeed");

    assert_eq!(
        uninterrupted_snapshot.physical_state_digest, resumed_snapshot.physical_state_digest,
        "resumed physical state digest must match uninterrupted run"
    );
    assert_eq!(
        uninterrupted_snapshot.history_digest, resumed_snapshot.history_digest,
        "resumed history digest must match uninterrupted run"
    );
    assert_eq!(
        uninterrupted_snapshot.canonical_state, resumed_snapshot.canonical_state,
        "resumed canonical state must match uninterrupted run"
    );
}

#[test]
fn digest_and_section_versions_are_unchanged() {
    // Given: the same fixture that was used to record pre-change digests at Wave 1.
    let mut runtime =
        Runtime::new(runtime_config(FIXTURE_SEED)).expect("runtime bootstrap must succeed");
    runtime
        .run_ticks(FIXTURE_TICKS)
        .expect("fixture ticks must execute");
    let snapshot = runtime.snapshot().expect("snapshot must succeed");

    // Then: the digests match the values recorded before the validation change.
    assert_eq!(
        hex32(snapshot.physical_state_digest.bytes()),
        FIXTURE_PHYSICAL_DIGEST,
        "physical_state_digest must be byte-identical to the pre-change value"
    );
    assert_eq!(
        hex32(snapshot.history_digest.bytes()),
        FIXTURE_HISTORY_DIGEST,
        "history_digest must be byte-identical to the pre-change value"
    );

    // And: importing the exported snapshot reproduces the same digests.
    let data = runtime
        .export_snapshot()
        .expect("fixture snapshot must export");
    let resumed = Runtime::from_snapshot(data).expect("exported fixture must resume");
    let resumed_snapshot = resumed.snapshot().expect("resumed snapshot must succeed");
    assert_eq!(
        snapshot.physical_state_digest, resumed_snapshot.physical_state_digest,
        "imported physical state digest must match the original"
    );
    assert_eq!(
        snapshot.history_digest, resumed_snapshot.history_digest,
        "imported history digest must match the original"
    );
}
