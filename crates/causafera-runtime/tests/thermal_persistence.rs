use causafera_persistence::SectionPayload;
use causafera_runtime::snapshot_sections::{
    THERMAL_SECTION_ID, THERMAL_SECTION_MAJOR, assemble_envelope, decode_thermal_section,
    disassemble_envelope, encode_thermal_section,
};
use causafera_runtime::{
    CURRENT_DIGEST_SCHEMA_VERSION, Runtime, RuntimeConfig, RuntimeError, RuntimeSnapshotData,
    RuntimeState,
};
use causafera_types::{ChartChunkCoord, ChunkCoord, SpatialChartId};

fn runtime_config(seed: u64) -> RuntimeConfig {
    RuntimeConfig::new(seed)
}

fn evolved_snapshot(seed: u64) -> RuntimeSnapshotData {
    let mut runtime = Runtime::new(runtime_config(seed)).expect("runtime bootstrap must succeed");
    runtime.tick().expect("thermal batch must execute");
    runtime.export_snapshot().expect("snapshot must export")
}

fn canonicalize_boundaries(snapshot: &mut RuntimeSnapshotData) {
    snapshot
        .thermal
        .boundary_records
        .sort_unstable_by_key(|record| (record.cell, record.neighbor));
}

fn shift_latest_receipt(snapshot: &mut RuntimeSnapshotData, shift: i64, shift_post_state: bool) {
    let latest_trace = snapshot.thermal.field_set.conservation_last_change;
    let receipt = snapshot
        .thermal
        .transfer_receipts
        .iter_mut()
        .find(|receipt| receipt.conservation_trace == latest_trace)
        .expect("latest batch must contain a transfer receipt");
    let cell = receipt.cell;
    receipt.pre_state = receipt
        .pre_state
        .checked_add(shift)
        .expect("test shift must remain representable");
    if shift_post_state {
        receipt.post_state = receipt
            .post_state
            .checked_add(shift)
            .expect("test shift must remain representable");
    }
    for boundary in snapshot
        .thermal
        .boundary_records
        .iter_mut()
        .filter(|record| record.cell == cell)
    {
        boundary.cell_pre_state = boundary
            .cell_pre_state
            .checked_add(shift)
            .expect("test shift must remain representable");
    }
}

#[test]
fn nonempty_boundary_section_roundtrips_canonically() {
    // Given: a committed thermal batch with current active-region boundary records.
    let snapshot = evolved_snapshot(1_800);
    assert!(!snapshot.thermal.boundary_records.is_empty());

    // When: the V1 thermal section and complete envelope are round-tripped.
    let encoded = encode_thermal_section(&snapshot.thermal);
    let decoded = decode_thermal_section(&encoded).expect("thermal section must decode");
    let envelope = assemble_envelope(&snapshot).expect("envelope must assemble");
    let restored = disassemble_envelope(&envelope).expect("envelope must disassemble");

    // Then: boundaries survive exactly and canonical bytes are stable.
    assert_eq!(decoded, snapshot.thermal);
    assert_eq!(encode_thermal_section(&decoded), encoded);
    assert_eq!(
        restored.thermal.boundary_records,
        snapshot.thermal.boundary_records
    );
}

#[test]
fn boundary_section_rejects_truncated_duplicate_unsorted_and_negative_records() {
    // Given: canonical bootstrap and evolved thermal sections.
    let bootstrap = Runtime::new(runtime_config(1_805))
        .expect("runtime bootstrap must succeed")
        .export_snapshot()
        .expect("snapshot must export");
    let evolved = evolved_snapshot(1_806);
    assert!(evolved.thermal.boundary_records.len() > 1);

    // When: the final count is truncated or records violate scalar/canonical invariants.
    let mut truncated_count = encode_thermal_section(&bootstrap.thermal);
    truncated_count.pop();
    let mut duplicate = evolved.clone();
    duplicate
        .thermal
        .boundary_records
        .push(duplicate.thermal.boundary_records[0]);
    let mut unsorted = evolved.clone();
    unsorted.thermal.boundary_records.swap(0, 1);
    let mut negative = evolved;
    negative.thermal.boundary_records[0].cell_pre_state = -1;

    // Then: decoding fails closed before runtime state can be allocated or installed.
    for malformed in [
        truncated_count,
        encode_thermal_section(&duplicate.thermal),
        encode_thermal_section(&unsorted.thermal),
        encode_thermal_section(&negative.thermal),
    ] {
        assert!(decode_thermal_section(&malformed).is_err());
    }
}

#[test]
fn runtime_import_rejects_malformed_boundary_records() {
    // Given: a valid committed boundary set and its latest transfer-receipt coverage.
    let valid = evolved_snapshot(1_807);
    let first = valid.thermal.boundary_records[0];
    let extent = valid.recipe.config.chunk_extent;
    let volume = u16::from(extent).pow(3);
    let latest_trace = valid.thermal.field_set.conservation_last_change;
    let receipt_cell = valid
        .thermal
        .transfer_receipts
        .iter()
        .find(|receipt| receipt.conservation_trace == latest_trace)
        .and_then(|receipt| {
            valid
                .thermal
                .boundary_records
                .iter()
                .any(|record| record.cell == receipt.cell)
                .then_some(receipt.cell)
        })
        .expect("latest batch must have a boundary source receipt");
    let unchanged_cell =
        valid
            .thermal
            .boundary_records
            .iter()
            .map(|record| record.cell)
            .find(|cell| {
                !valid.thermal.transfer_receipts.iter().any(|receipt| {
                    receipt.conservation_trace == latest_trace && receipt.cell == *cell
                })
            })
            .expect("latest batch must have an unchanged boundary source");

    // When: records violate batch, ordering, source, geometry, completeness, or pre-state rules.
    let mut batch_zero = Runtime::new(runtime_config(1_808))
        .expect("runtime bootstrap must succeed")
        .export_snapshot()
        .expect("snapshot must export");
    batch_zero.thermal.boundary_records.push(first);
    let mut duplicate = valid.clone();
    duplicate.thermal.boundary_records.insert(1, first);
    let mut unsorted = valid.clone();
    unsorted.thermal.boundary_records.swap(0, 1);
    let mut invalid_source = valid.clone();
    invalid_source.thermal.boundary_records[0].cell.cell_index = volume;
    canonicalize_boundaries(&mut invalid_source);
    let mut active_neighbor = valid.clone();
    active_neighbor.thermal.boundary_records[0].neighbor = first.cell;
    canonicalize_boundaries(&mut active_neighbor);
    let mut cross_chart = valid.clone();
    cross_chart.thermal.boundary_records[0].neighbor.chunk.chart =
        SpatialChartId::new(first.neighbor.chunk.chart.raw() + 1);
    canonicalize_boundaries(&mut cross_chart);
    let mut nonadjacent = valid.clone();
    nonadjacent.thermal.boundary_records[0].neighbor.chunk = ChartChunkCoord::new(
        first.neighbor.chunk.chart,
        ChunkCoord::new(
            first.neighbor.chunk.chunk.x.saturating_add(2),
            first.neighbor.chunk.chunk.y,
            first.neighbor.chunk.chunk.z,
        ),
    );
    canonicalize_boundaries(&mut nonadjacent);
    let mut wrong_face = valid.clone();
    wrong_face.thermal.boundary_records[0].neighbor.cell_index =
        (first.neighbor.cell_index + 1) % volume;
    canonicalize_boundaries(&mut wrong_face);
    let mut missing = valid.clone();
    missing.thermal.boundary_records.remove(0);
    let mut receipt_pre_state = valid.clone();
    receipt_pre_state
        .thermal
        .boundary_records
        .iter_mut()
        .find(|record| record.cell == receipt_cell)
        .expect("receipt-backed boundary must exist")
        .cell_pre_state += 1;
    let mut unchanged_pre_state = valid;
    unchanged_pre_state
        .thermal
        .boundary_records
        .iter_mut()
        .find(|record| record.cell == unchanged_cell)
        .expect("unchanged boundary must exist")
        .cell_pre_state += 1;

    // Then: every malformed authoritative set is rejected before installation.
    for malformed in [
        batch_zero,
        duplicate,
        unsorted,
        invalid_source,
        active_neighbor,
        cross_chart,
        nonadjacent,
        wrong_face,
        missing,
        receipt_pre_state,
        unchanged_pre_state,
    ] {
        assert!(matches!(
            RuntimeState::import_snapshot(malformed),
            Err(RuntimeError::InvalidSnapshot(_))
        ));
    }
}

#[test]
fn runtime_import_rejects_receipt_flux_transition_forgery() {
    // Given: a valid latest receipt and a matching boundary record.
    let mut forged = evolved_snapshot(1_809);
    shift_latest_receipt(&mut forged, 1, false);

    // When: the receipt pre-state is changed without changing its post-state or fluxes.
    let imported = RuntimeState::import_snapshot(forged);

    // Then: the internal signed-flux transition equation rejects the coordinated forgery.
    assert!(matches!(imported, Err(RuntimeError::InvalidSnapshot(_))));
}

#[test]
fn runtime_import_rejects_latest_receipt_post_state_mismatch() {
    // Given: a valid latest receipt whose pre/post states are shifted together.
    let mut forged = evolved_snapshot(1_810);
    shift_latest_receipt(&mut forged, 1, true);

    // When: the latest receipt no longer describes the current field energy.
    let imported = RuntimeState::import_snapshot(forged);

    // Then: the latest batch post-state must bind to the indexed current field cell.
    assert!(matches!(imported, Err(RuntimeError::InvalidSnapshot(_))));
}

#[test]
fn runtime_import_rejects_non_latest_receipt_cell_index_out_of_bounds() {
    // Given: a snapshot spanning two committed batches, so an earlier (non-latest) transfer
    // receipt exists alongside the latest one.
    let mut runtime = Runtime::new(runtime_config(1_813)).expect("runtime bootstrap must succeed");
    runtime.run_ticks(2).expect("thermal batches must execute");
    let mut forged = runtime.export_snapshot().expect("snapshot must export");
    let latest_trace = forged.thermal.field_set.conservation_last_change;
    let volume = u16::from(forged.recipe.config.chunk_extent).pow(3);
    let non_latest_receipt = forged
        .thermal
        .transfer_receipts
        .iter_mut()
        .find(|receipt| receipt.conservation_trace != latest_trace)
        .expect("an earlier batch must have a transfer receipt");
    non_latest_receipt.cell.cell_index = volume;

    // When: a historical, non-latest transfer receipt references a cell index outside the field.
    let imported = RuntimeState::import_snapshot(forged);

    // Then: cell-index bounds validation applies to every batch, not only the latest.
    assert!(matches!(imported, Err(RuntimeError::InvalidSnapshot(_))));
}

#[test]
fn runtime_import_rejects_reservoir_budget_subtraction_overflow() {
    // Given: a valid snapshot with an unrepresentable reservoir budget difference.
    let mut forged = evolved_snapshot(1_811);
    forged.thermal.conservation_receipts[0].total_reservoir_budget_before = i128::MAX;
    forged.thermal.conservation_receipts[0].total_reservoir_budget_after = i128::MIN;

    // When: the crafted snapshot is imported.
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        RuntimeState::import_snapshot(forged)
    }));

    // Then: checked arithmetic returns InvalidSnapshot rather than panicking.
    assert!(result.is_ok(), "snapshot import must not panic on overflow");
    assert!(matches!(
        result.expect("checked import result must exist"),
        Err(RuntimeError::InvalidSnapshot(_))
    ));
}

#[test]
fn malformed_rejects() {
    // Given: a production snapshot with the complete thermal section.
    let runtime = Runtime::new(runtime_config(1_801)).expect("runtime bootstrap must succeed");
    let snapshot = runtime.export_snapshot().expect("snapshot must export");

    // When: independently corrupting authoritative thermal invariants.
    let mut negative_energy = snapshot.clone();
    negative_energy.thermal.field_set.fields[0].energy[0] = -1;
    let mut malformed_extent = snapshot.clone();
    malformed_extent.thermal.field_set.fields[0].extent = 0;
    let mut duplicate_chunk = snapshot.clone();
    duplicate_chunk
        .thermal
        .field_set
        .fields
        .push(duplicate_chunk.thermal.field_set.fields[0].clone());
    let mut active_region_gap = snapshot.clone();
    active_region_gap.thermal.field_set.fields.pop();
    let mut unbalanced_budget = snapshot;
    unbalanced_budget.thermal.reservoirs[0].budget = -1;
    let mut evolved_runtime =
        Runtime::new(runtime_config(1_804)).expect("runtime bootstrap must succeed");
    evolved_runtime.tick().expect("thermal batch must execute");
    let evolved = evolved_runtime
        .export_snapshot()
        .expect("evolved snapshot must export");
    let mut mismatched_anchor = evolved.clone();
    mismatched_anchor.thermal.field_set.fields[0].last_change[0] =
        evolved.traces.events[0].trace_id;
    let mut mismatched_batch_sequence = evolved.clone();
    mismatched_batch_sequence.thermal.field_set.batch_sequence += 1;
    let mut nonzero_residual = evolved.clone();
    nonzero_residual.thermal.conservation_receipts[0].residual = 1;
    let mut receipt_budget_mismatch = evolved;
    receipt_budget_mismatch.thermal.conservation_receipts[0].total_reservoir_budget_after += 1;

    // Then: no malformed thermal state is installed.
    for malformed in [
        negative_energy,
        malformed_extent,
        duplicate_chunk,
        active_region_gap,
        unbalanced_budget,
        mismatched_anchor,
        mismatched_batch_sequence,
        nonzero_residual,
        receipt_budget_mismatch,
    ] {
        assert!(matches!(
            RuntimeState::import_snapshot(malformed),
            Err(RuntimeError::InvalidSnapshot(_))
        ));
    }
}

#[test]
fn unknown_thermal_section_or_version_rejects() {
    // Given: an envelope containing the required thermal section.
    let runtime = Runtime::new(runtime_config(1_802)).expect("runtime bootstrap must succeed");
    let snapshot = runtime.export_snapshot().expect("snapshot must export");
    let envelope = assemble_envelope(&snapshot).expect("envelope must assemble");

    // When: its version is changed or an unknown authoritative section is injected.
    let mut incompatible = envelope.clone();
    incompatible
        .sections
        .get_mut(&u64::from(THERMAL_SECTION_ID))
        .expect("thermal section must exist")
        .section_major = 2;
    let mut unknown = envelope;
    unknown.sections.insert(
        0xFFFF,
        SectionPayload {
            section_major: 1,
            section_minor: 0,
            flags: 0,
            decoded_size_limit: 0,
            bytes: Vec::new(),
        },
    );

    // Then: both envelopes fail closed before runtime restoration.
    assert!(disassemble_envelope(&incompatible).is_err());
    assert!(disassemble_envelope(&unknown).is_err());
}

#[test]
fn thermal_persistence_literal_version_contract() {
    // Given: the current production snapshot and its assembled envelope.
    let runtime = Runtime::new(runtime_config(1_812)).expect("runtime bootstrap must succeed");
    let snapshot = runtime.export_snapshot().expect("snapshot must export");
    let envelope = assemble_envelope(&snapshot).expect("envelope must assemble");

    // Then: thermal persistence and digest versions retain their literal wire contract.
    assert_eq!(THERMAL_SECTION_ID, 0x000E);
    assert_eq!(THERMAL_SECTION_MAJOR, 1);
    assert_eq!(CURRENT_DIGEST_SCHEMA_VERSION.raw(), 5);
    assert_eq!(envelope.sections[&u64::from(0x000E_u16)].section_major, 1);
    assert_eq!(envelope.header.physical_digest_schema, 5);
    assert_eq!(envelope.header.history_digest_schema, 5);
}

#[test]
fn save_resume_equivalence() {
    // Given: equivalent continuous and checkpointed production runtimes.
    let config = runtime_config(1_803);
    let mut continuous = Runtime::new(config.clone()).expect("continuous runtime must bootstrap");
    let mut checkpointed = Runtime::new(config).expect("checkpointed runtime must bootstrap");

    // When: one runtime runs continuously and the other resumes from tick fifty.
    continuous
        .run_ticks(100)
        .expect("continuous runtime must execute");
    checkpointed
        .run_ticks(50)
        .expect("checkpoint runtime must execute");
    let checkpoint = checkpointed
        .export_snapshot()
        .expect("checkpoint snapshot must export");
    let mut resumed = Runtime::from_snapshot(checkpoint).expect("checkpoint must resume");
    resumed.run_ticks(50).expect("resumed runtime must execute");

    // Then: persisted thermal state and canonical identities match the uninterrupted run.
    let resumed_export = resumed
        .export_snapshot()
        .expect("resumed snapshot must export");
    let continuous_export = continuous
        .export_snapshot()
        .expect("continuous snapshot must export");
    assert!(!resumed_export.thermal.boundary_records.is_empty());
    assert_eq!(
        resumed_export.thermal.boundary_records,
        continuous_export.thermal.boundary_records
    );
    assert_eq!(resumed_export.thermal, continuous_export.thermal);
    assert_eq!(resumed_export, continuous_export);
    assert_eq!(resumed.current_time(), continuous.current_time());
    assert_eq!(
        resumed
            .snapshot()
            .expect("resumed snapshot must be available")
            .physical_state_digest,
        continuous
            .snapshot()
            .expect("continuous snapshot must be available")
            .physical_state_digest
    );
    assert_eq!(
        resumed
            .snapshot()
            .expect("resumed snapshot must be available")
            .history_digest,
        continuous
            .snapshot()
            .expect("continuous snapshot must be available")
            .history_digest
    );
}
