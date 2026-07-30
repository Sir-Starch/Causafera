//! Hydrology persistence: section `0x000F`, digest schema 8, and save/resume
//! equivalence.
//!
//! Covers `plans/hydrology.md` verification gates V24 and V26.

mod support_hydrology;

use causafera_runtime::snapshot_sections::{
    HYDROLOGY_SECTION_ID, HYDROLOGY_SECTION_MAJOR, assemble_envelope, decode_hydrology_section,
    disassemble_envelope, encode_hydrology_section,
};
use causafera_runtime::{CURRENT_DIGEST_SCHEMA_VERSION, Runtime, RuntimeConfig, RuntimeState};

use support_hydrology::{enabled_runtime_config, wet_runtime_config};

fn evolved(ticks: u64) -> Runtime {
    let mut runtime = Runtime::new(wet_runtime_config()).expect("construction");
    runtime.run_ticks(ticks).expect("the run must commit");
    runtime
}

#[test]
fn the_hydrology_section_is_required_and_versioned() {
    let snapshot = Runtime::new(RuntimeConfig::new(5))
        .expect("construction")
        .export_snapshot()
        .expect("export");
    let envelope = assemble_envelope(&snapshot).expect("assemble");
    let section = envelope
        .sections
        .get(&u64::from(HYDROLOGY_SECTION_ID))
        .expect("the section is required even for a disabled domain");
    assert_eq!(section.section_major, HYDROLOGY_SECTION_MAJOR);

    // A wrong major fails closed rather than being read as major 1.
    let mut wrong = envelope.clone();
    wrong
        .sections
        .get_mut(&u64::from(HYDROLOGY_SECTION_ID))
        .expect("the section exists")
        .section_major = HYDROLOGY_SECTION_MAJOR + 1;
    assert!(disassemble_envelope(&wrong).is_err());

    // And so does its absence, which is what distinguishes "no water" from
    // "this snapshot predates hydrology".
    let mut missing = envelope.clone();
    missing.sections.remove(&u64::from(HYDROLOGY_SECTION_ID));
    assert!(disassemble_envelope(&missing).is_err());
}

#[test]
fn a_disabled_domain_encodes_exactly_one_byte() {
    let state = causafera_runtime::HydrologyRuntimeState::disabled();
    let bytes = encode_hydrology_section(&state);
    assert_eq!(bytes, vec![0]);
    assert_eq!(decode_hydrology_section(&bytes).expect("decode"), state);
}

#[test]
fn an_evolved_hydrology_state_round_trips_byte_for_byte() {
    let runtime = evolved(4);
    let state = runtime.hydrology_state();
    assert!(state.enabled);
    assert!(!state.retained_batches.is_empty());

    let bytes = encode_hydrology_section(&state);
    let decoded = decode_hydrology_section(&bytes).expect("the section must decode");
    assert_eq!(
        decoded, state,
        "every field survives the canonical encoding"
    );

    // One state has one representation, so re-encoding is byte-identical.
    assert_eq!(encode_hydrology_section(&decoded), bytes);
}

#[test]
fn the_digest_schema_is_eight_and_covers_hydrology() {
    let disabled = Runtime::new(RuntimeConfig::new(77))
        .expect("construction")
        .snapshot()
        .expect("digest");
    assert_eq!(CURRENT_DIGEST_SCHEMA_VERSION.raw(), 8);
    assert_eq!(disabled.physical_state_digest.schema_version.raw(), 8);

    // Two sessions differing only in their water do not share a physical digest.
    let a = evolved(3).snapshot().expect("digest");
    let mut altered = wet_runtime_config();
    altered
        .hydrology
        .bootstrap_parameters
        .as_mut()
        .expect("enabled")
        .initial_surface = causafera_types::WaterVolume::new(20_000_001);
    let mut runtime = Runtime::new(altered).expect("construction");
    runtime.run_ticks(3).expect("the run must commit");
    let b = runtime.snapshot().expect("digest");
    assert_ne!(
        a.physical_state_digest.bytes(),
        b.physical_state_digest.bytes(),
        "hydrology state is authoritative equality input under schema 8"
    );
}

#[test]
fn an_enabled_session_resumes_byte_identically() {
    // V26: exporting, importing, and re-exporting reproduces the same envelope,
    // and the resumed runtime continues into the same state a run that never
    // stopped would have reached.
    let runtime = evolved(5);
    let exported = runtime.export_snapshot().expect("export");
    let envelope = assemble_envelope(&exported).expect("assemble");
    let bytes = envelope.encode().expect("encode");

    let decoded = disassemble_envelope(&envelope).expect("disassemble");
    let resumed = RuntimeState::import_snapshot(decoded).expect("import");
    let reexported = resumed.export_snapshot();

    assert_eq!(reexported.hydrology, exported.hydrology);
    assert_eq!(
        assemble_envelope(&reexported)
            .expect("assemble")
            .encode()
            .expect("encode"),
        bytes,
        "export, import, export is byte-identical"
    );
}

#[test]
fn a_resumed_session_continues_into_the_same_state() {
    let uninterrupted = {
        let mut runtime = Runtime::new(wet_runtime_config()).expect("construction");
        runtime.run_ticks(8).expect("the run must commit");
        (
            runtime.export_snapshot().expect("export"),
            runtime.snapshot().expect("digest"),
        )
    };
    let (uninterrupted, uninterrupted_digest) = uninterrupted;

    let mut runtime = Runtime::new(wet_runtime_config()).expect("construction");
    runtime.run_ticks(4).expect("the first half must commit");
    let halfway = runtime.export_snapshot().expect("export");
    let envelope = assemble_envelope(&halfway).expect("assemble");
    let mut resumed = Runtime::from_snapshot(disassemble_envelope(&envelope).expect("disassemble"))
        .expect("resume");
    resumed.run_ticks(4).expect("the second half must commit");
    let continued_digest = resumed.snapshot().expect("digest");
    let continued = resumed.export_snapshot().expect("export");

    assert_eq!(continued.hydrology.fields, uninterrupted.hydrology.fields);
    assert_eq!(
        continued.hydrology.conveyance,
        uninterrupted.hydrology.conveyance
    );
    assert_eq!(
        continued.hydrology.next_node_id, uninterrupted.hydrology.next_node_id,
        "the synthetic-node counter survives the interruption"
    );
    assert_eq!(
        continued_digest.physical_state_digest.bytes(),
        uninterrupted_digest.physical_state_digest.bytes(),
        "a resumed run is the same run"
    );
}

#[test]
fn a_pending_forcing_record_survives_export_and_still_applies() {
    let mut config = enabled_runtime_config();
    config.hydrology.forcing_schedule[0].scheduled_tick = 6;
    let mut runtime = Runtime::new(config).expect("construction");
    runtime.run_ticks(2).expect("two ticks must commit");

    let envelope =
        assemble_envelope(&runtime.export_snapshot().expect("export")).expect("assemble");
    let mut resumed = Runtime::from_snapshot(disassemble_envelope(&envelope).expect("disassemble"))
        .expect("resume");
    assert_eq!(
        resumed.hydrology_state().forcing[0].applied_at(),
        None,
        "a pending record is still pending after a reload"
    );
    resumed.run_ticks(4).expect("the run must reach tick six");
    assert_eq!(
        resumed.hydrology_state().forcing[0].applied_at(),
        Some(6),
        "and it still applies at its scheduled tick"
    );
}

#[test]
fn an_applied_forcing_record_stays_persisted_and_is_not_reapplied() {
    let mut config = enabled_runtime_config();
    config.hydrology.forcing_schedule[0].scheduled_tick = 2;
    let mut runtime = Runtime::new(config).expect("construction");
    runtime.run_ticks(3).expect("three ticks must commit");

    let envelope =
        assemble_envelope(&runtime.export_snapshot().expect("export")).expect("assemble");
    let mut resumed = Runtime::from_snapshot(disassemble_envelope(&envelope).expect("disassemble"))
        .expect("resume");
    assert_eq!(resumed.hydrology_state().forcing.len(), 1);
    assert_eq!(resumed.hydrology_state().forcing[0].applied_at(), Some(2));

    resumed.run_ticks(3).expect("later ticks must commit");
    let state = resumed.hydrology_state();
    assert_eq!(state.forcing[0].applied_at(), Some(2));
    let trace = *state.retained_batches.last().expect("a batch was retained");
    assert_eq!(
        state.conservation_receipts[&trace].accepted_precipitation(),
        0,
        "a spent record delivers nothing after a reload either"
    );
}

#[test]
fn retained_typed_batches_survive_export_and_import() {
    let runtime = evolved(3);
    let exported = runtime.export_snapshot().expect("export");
    let envelope = assemble_envelope(&exported).expect("assemble");
    let resumed =
        RuntimeState::import_snapshot(disassemble_envelope(&envelope).expect("disassemble"))
            .expect("import")
            .export_snapshot();

    assert_eq!(
        resumed.hydrology.retained_batches,
        exported.hydrology.retained_batches
    );
    assert_eq!(resumed.hydrology.receipts, exported.hydrology.receipts);
    assert_eq!(
        resumed.hydrology.conservation_receipts,
        exported.hydrology.conservation_receipts
    );
    for trace in &exported.hydrology.retained_batches {
        assert!(
            !exported.hydrology.receipts[trace].is_empty(),
            "the fixture has to have moved water"
        );
    }
}

#[test]
fn a_promoted_chunks_level_and_anchor_survive_a_round_trip() {
    // V20 and V24 over resolution state: the engine promotes chunks on its own
    // schedule, so a long enough run persists a non-zero level and the
    // representation event that anchors it. Both have to come back exactly, or a
    // resumed world would evaluate at a detail nobody chose and cite a change
    // that never happened.
    let runtime = evolved(12);
    let before = runtime.hydrology_state();
    let promoted: Vec<_> = before
        .resolution
        .iter()
        .filter(|(_, entry)| entry.level() > 0)
        .map(|(chunk, entry)| (*chunk, *entry))
        .collect();
    assert!(
        !promoted.is_empty(),
        "the run must reach a promotion, or this proves nothing about resolution"
    );

    let exported = runtime.export_snapshot().expect("export");
    let resumed = RuntimeState::import_snapshot(
        disassemble_envelope(&assemble_envelope(&exported).expect("assemble"))
            .expect("disassemble"),
    )
    .expect("import");

    let restored_state = resumed.export_snapshot().hydrology;
    assert_eq!(restored_state.resolution, before.resolution);
    for (chunk, entry) in promoted {
        let restored = restored_state.resolution[&chunk];
        assert_eq!(restored.level(), entry.level());
        assert_eq!(
            restored.last_change(),
            entry.last_change(),
            "the anchor is the representation event, not a re-derived guess"
        );
    }
}
