//! Malformed and forged hydrology snapshots fail closed.
//!
//! Covers `plans/hydrology.md` verification gate V25. Decoding rebuilds every
//! collection through its validating constructor, so most of these are refused
//! before they can become state; the rest are cross-cutting agreements no single
//! constructor can see.

mod support_hydrology;

use causafera_runtime::snapshot_sections::{
    HYDROLOGY_SECTION_ID, assemble_envelope, decode_hydrology_section, disassemble_envelope,
    encode_hydrology_section,
};
use causafera_runtime::{Runtime, RuntimeState};

use support_hydrology::{enabled_runtime_config, wet_runtime_config};

fn evolved_bytes(ticks: u64) -> (Vec<u8>, causafera_runtime::RuntimeSnapshotData) {
    let mut runtime = Runtime::new(wet_runtime_config()).expect("construction");
    runtime.run_ticks(ticks).expect("the run must commit");
    let snapshot = runtime.export_snapshot().expect("export");
    (encode_hydrology_section(&snapshot.hydrology), snapshot)
}

/// Import a snapshot whose hydrology section has been replaced with `bytes`.
fn import_with_section(bytes: Vec<u8>) -> Result<(), String> {
    let mut runtime = Runtime::new(wet_runtime_config()).expect("construction");
    runtime.run_ticks(3).expect("the run must commit");
    let mut envelope =
        assemble_envelope(&runtime.export_snapshot().expect("export")).expect("assemble");
    envelope
        .sections
        .get_mut(&u64::from(HYDROLOGY_SECTION_ID))
        .expect("the section exists")
        .bytes = bytes;
    let data = disassemble_envelope(&envelope).map_err(|error| error.to_string())?;
    RuntimeState::import_snapshot(data)
        .map(|_| ())
        .map_err(|error| error.to_string())
}

// ---------------------------------------------------------------------------
// Malformed bytes
// ---------------------------------------------------------------------------

#[test]
fn trailing_bytes_after_a_complete_section_are_refused() {
    // A decoder that stopped at the last field it recognised would accept a
    // payload carrying anything after it.
    let (mut bytes, _) = evolved_bytes(2);
    bytes.push(0);
    assert!(decode_hydrology_section(&bytes).is_err());
}

#[test]
fn a_truncated_section_is_refused_rather_than_partially_read() {
    let (bytes, _) = evolved_bytes(2);
    for cut in [1_usize, bytes.len() / 3, bytes.len() / 2, bytes.len() - 1] {
        assert!(
            decode_hydrology_section(&bytes[..cut]).is_err(),
            "a section truncated at {cut} must not decode"
        );
    }
}

#[test]
fn a_disabled_flag_followed_by_a_payload_is_refused() {
    // "Disabled" has one canonical encoding: the flag and nothing else.
    assert!(decode_hydrology_section(&[0, 1, 2, 3]).is_err());
    assert!(
        !decode_hydrology_section(&[0])
            .expect("a lone flag decodes")
            .enabled
    );
}

#[test]
fn an_unsupported_resolution_policy_schema_is_refused() {
    let (mut bytes, _) = evolved_bytes(1);
    // The policy schema is the u16 immediately after the enabled flag.
    bytes[1] = 9;
    bytes[2] = 0;
    assert!(decode_hydrology_section(&bytes).is_err());
}

#[test]
fn a_forged_count_is_refused_before_it_can_reserve_memory() {
    // The metric count sits after the flag, the policy, the node counter, the batch
    // sequence, and the conservation trace: 1 + 2 + 1 + 1 + 8 + 8 + 8 = 29.
    let (mut bytes, _) = evolved_bytes(1);
    let offset = 1 + 2 + 1 + 1 + 8 + 8 + 8;
    bytes[offset..offset + 8].copy_from_slice(&u64::MAX.to_le_bytes());
    assert!(decode_hydrology_section(&bytes).is_err());
}

// ---------------------------------------------------------------------------
// Forged state that decodes but must not import
// ---------------------------------------------------------------------------

#[test]
fn an_unmodified_section_still_imports() {
    // The control. Every rejection below has to be attributable to the mutation
    // and not to the harness.
    let (bytes, _) = evolved_bytes(3);
    import_with_section(bytes).expect("an unmodified section must import");
}

#[test]
fn a_disabled_section_beside_an_enabled_recipe_is_refused() {
    // The recipe says the session has water and the section says it does not. One
    // of the two is lying and import cannot tell which, so it refuses both.
    let error = match import_with_section(vec![0]) {
        Ok(()) => panic!("the mismatch must be refused"),
        Err(error) => error,
    };
    assert!(error.contains("disagree"), "unexpected refusal: {error}");
}

#[test]
fn a_forged_conservation_residual_is_refused() {
    // A retained ledger that does not close is a record of water appearing or
    // vanishing. Import recomputes the residual rather than reading it, so the
    // forgery has to be in a term — and then the residual is nonzero.
    let mut runtime = Runtime::new(wet_runtime_config()).expect("construction");
    runtime.run_ticks(3).expect("the run must commit");
    let mut snapshot = runtime.export_snapshot().expect("export");

    let trace = *snapshot
        .hydrology
        .retained_batches
        .last()
        .expect("a batch was retained");
    let ledger = snapshot.hydrology.conservation_receipts[&trace];
    let forged = causafera_domains::HydrologyConservationReceipt::new(
        causafera_domains::HydrologyConservationParts {
            tick: ledger.tick(),
            batch_sequence: ledger.batch_sequence(),
            surface_before: ledger.surface_before(),
            soil_before: ledger.soil_before(),
            groundwater_before: ledger.groundwater_before(),
            conveyance_before: ledger.conveyance_before(),
            // One cubic millimetre of invented water.
            surface_after: ledger.surface_after() + 1,
            soil_after: ledger.soil_after(),
            groundwater_after: ledger.groundwater_after(),
            conveyance_after: ledger.conveyance_after(),
            accepted_precipitation: ledger.accepted_precipitation(),
            accepted_external_inflow: ledger.accepted_external_inflow(),
            accepted_evapotranspiration: ledger.accepted_evapotranspiration(),
            boundary_exports: ledger.boundary_exports(),
        },
    )
    .expect("a receipt with a nonzero residual still constructs");
    assert_ne!(forged.residual(), 0);
    snapshot
        .hydrology
        .conservation_receipts
        .insert(trace, forged);

    let bytes = encode_hydrology_section(&snapshot.hydrology);
    let error = import_with_section(bytes).expect_err("an unbalanced ledger must be refused");
    assert!(error.contains("residual"), "unexpected refusal: {error}");
}

#[test]
fn a_retained_batch_missing_its_ledger_is_refused() {
    let mut runtime = Runtime::new(wet_runtime_config()).expect("construction");
    runtime.run_ticks(3).expect("the run must commit");
    let mut snapshot = runtime.export_snapshot().expect("export");
    let trace = *snapshot
        .hydrology
        .retained_batches
        .last()
        .expect("a batch was retained");
    snapshot.hydrology.conservation_receipts.remove(&trace);

    let bytes = encode_hydrology_section(&snapshot.hydrology);
    let error = import_with_section(bytes).expect_err("half a batch must be refused");
    assert!(
        error.contains("retained batch"),
        "unexpected refusal: {error}"
    );
}

#[test]
fn a_registry_that_does_not_address_its_own_carriers_is_refused() {
    // The registry is what every causal target in the store was written against, so
    // an incomplete one would let a carrier have no target — or two carriers share
    // one.
    let mut runtime = Runtime::new(wet_runtime_config()).expect("construction");
    runtime.run_ticks(2).expect("the run must commit");
    let mut snapshot = runtime.export_snapshot().expect("export");

    let mut cells: Vec<_> = snapshot
        .hydrology
        .registry
        .cells()
        .iter()
        .map(|(cell, ordinal)| (*cell, *ordinal))
        .collect();
    cells.pop();
    snapshot.hydrology.registry = causafera_runtime::HydrologyObjectRegistry::from_tables(
        cells,
        snapshot
            .hydrology
            .registry
            .edges()
            .iter()
            .map(|(edge, ordinal)| (*edge, *ordinal))
            .collect(),
        snapshot
            .hydrology
            .registry
            .forcing()
            .iter()
            .map(|(key, ordinal)| (*key, *ordinal))
            .collect(),
        snapshot
            .hydrology
            .registry
            .resolution()
            .iter()
            .map(|(chunk, ordinal)| (*chunk, *ordinal))
            .collect(),
    )
    .expect("dropping the last ordinal keeps the range dense");

    let bytes = encode_hydrology_section(&snapshot.hydrology);
    let error = import_with_section(bytes).expect_err("an incomplete registry must be refused");
    assert!(error.contains("registry"), "unexpected refusal: {error}");
}

#[test]
fn a_non_dense_registry_is_refused_at_decode() {
    // Sparse ordinals never become state at all: the constructor the decoder uses
    // rejects them.
    let cell = support_hydrology::cell(0);
    let other = support_hydrology::cell(1);
    assert!(
        causafera_runtime::HydrologyObjectRegistry::from_tables(
            vec![(cell, 0), (other, 7)],
            Vec::new(),
            Vec::new(),
            Vec::new(),
        )
        .is_err(),
        "an ordinal outside the dense range must be refused"
    );
}

#[test]
fn a_pending_record_scheduled_in_the_past_is_refused() {
    // It would never apply, which makes it a record of rain that cannot fall.
    let mut config = enabled_runtime_config();
    config.hydrology.forcing_schedule[0].scheduled_tick = 9;
    let mut runtime = Runtime::new(config).expect("construction");
    runtime.run_ticks(4).expect("the run must commit");
    let mut snapshot = runtime.export_snapshot().expect("export");

    // Rewrite the pending record to a tick the runtime has already passed.
    let record = &snapshot.hydrology.forcing[0];
    let rewritten = causafera_geography::HydrologyForcingRecord::new(
        causafera_geography::HydrologyForcingParts {
            forcing_id: record.forcing_id(),
            scheduled_tick: 2,
            targets: record.targets().to_vec(),
            precipitation_volume: record.precipitation_volume(),
            potential_et_volume: record.potential_et_volume(),
            external_inflow_volume: record.external_inflow_volume(),
            origin_trace: record.origin_trace(),
            producer_policy_schema: record.producer_policy_schema(),
            applied_at: None,
        },
    )
    .expect("the rewritten record is well formed");
    snapshot.hydrology.forcing = vec![rewritten];
    snapshot.hydrology.registry = causafera_runtime::HydrologyObjectRegistry::assign(
        snapshot.hydrology.registry.cells().keys().copied(),
        snapshot.hydrology.registry.edges().keys().copied(),
        [(2, snapshot.hydrology.forcing[0].forcing_id())],
        snapshot.hydrology.registry.resolution().keys().copied(),
    );

    // The state is already invalid in memory, so the refusal lands at the first
    // boundary that validates it — assembling the envelope, which digests the state
    // it is about to write. Asserting the message rather than the boundary is what
    // keeps this a test about the rule.
    let error = match assemble_envelope(&snapshot) {
        Ok(_) => panic!("a record that missed its tick must be refused"),
        Err(error) => error.to_string(),
    };
    assert!(
        error.contains("scheduled in the past"),
        "unexpected: {error}"
    );
}

#[test]
fn an_applied_record_whose_timestamp_is_not_its_tick_cannot_be_constructed() {
    // The forgery is refused at the record's own constructor, which import inherits
    // — so there is no snapshot shape that carries it. Asserted here rather than
    // through a snapshot, because a test that had to build one first would be
    // testing the harness.
    let mut runtime = Runtime::new(enabled_runtime_config()).expect("construction");
    runtime.run_ticks(4).expect("the run must commit");
    let snapshot = runtime.export_snapshot().expect("export");
    let record = &snapshot.hydrology.forcing[0];
    assert_eq!(record.applied_at(), Some(record.scheduled_tick()));

    assert!(
        causafera_geography::HydrologyForcingRecord::new(
            causafera_geography::HydrologyForcingParts {
                forcing_id: record.forcing_id(),
                scheduled_tick: record.scheduled_tick(),
                targets: record.targets().to_vec(),
                precipitation_volume: record.precipitation_volume(),
                potential_et_volume: record.potential_et_volume(),
                external_inflow_volume: record.external_inflow_volume(),
                origin_trace: record.origin_trace(),
                producer_policy_schema: record.producer_policy_schema(),
                applied_at: Some(record.scheduled_tick() + 1),
            },
        )
        .is_err(),
        "an applied timestamp that is not the scheduled tick is not a record"
    );
}
