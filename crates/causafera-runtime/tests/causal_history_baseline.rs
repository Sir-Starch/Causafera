mod support_hydrology;

use causafera_persistence::{PersistenceError, SnapshotEnvelope, SnapshotHeader};
use causafera_runtime::snapshot_sections::{
    HYDROLOGY_SECTION_ID, SECTION_POPULATION_BOOTSTRAP, SECTION_RUNTIME_RECIPE,
    assemble_envelope_for_digest_schema, disassemble_envelope,
};
use causafera_runtime::{
    CURRENT_DIGEST_SCHEMA_VERSION, LEGACY_DIGEST_SCHEMA_VERSION, Runtime, RuntimeConfig,
    RuntimeError,
};

const FIXTURE: &[u8] = include_bytes!("fixtures/causal-history/schema-8-baseline.otps");
const FIXTURE_SHA256: &str = "e4a2be7c3daf86bf0539c139a62a24adbec429d517c8face279ee7a6d9b62766";
const BASELINE_TICKS: u64 = 4;
const BASELINE_PHYSICAL_DIGEST: &str =
    "1cab3de3a202057558da26d7c61cede281edf6957757fe3cdef5e1957095c2b9";
const BASELINE_HISTORY_DIGEST: &str =
    "03615361b7d0d1e710b8a400bc4a4db2cce2ac230fe6c29443561a831169f9f9";
const BASELINE_EXPERIMENT_DIGEST: &str =
    "be4c690216fd479583900c1e725208e9e8228ea9531d83f51672515a7984ccc4";
const REQUIRED_SECTIONS: &[(u64, u16)] = &[
    (0x0001, 7),
    (0x0002, 1),
    (0x0003, 2),
    (0x0004, 1),
    (0x0005, 1),
    (0x0006, 3),
    (0x0007, 1),
    (0x0008, 1),
    (0x0009, 2),
    (0x000A, 1),
    (0x000C, 3),
    (0x000D, 1),
    (0x000E, 2),
    (0x000F, 1),
];

fn emitted_schema_8_bytes() -> Vec<u8> {
    let runtime = Runtime::new(RuntimeConfig::new(77)).expect("small runtime constructs");
    assemble_envelope_for_digest_schema(
        &runtime.export_snapshot().expect("baseline exports"),
        LEGACY_DIGEST_SCHEMA_VERSION,
    )
    .expect("baseline envelope assembles")
    .encode()
    .expect("baseline envelope encodes")
}

fn load_schema_8(bytes: &[u8]) -> Result<Runtime, PersistenceError> {
    let envelope = SnapshotEnvelope::decode(bytes)?;
    if envelope.header.codec_revision != 1 {
        return Err(PersistenceError::codec("unsupported codec revision"));
    }
    if envelope.header.physical_digest_schema != LEGACY_DIGEST_SCHEMA_VERSION.raw()
        || envelope.header.history_digest_schema != LEGACY_DIGEST_SCHEMA_VERSION.raw()
    {
        return Err(PersistenceError::codec(
            "unsupported authoritative digest schema",
        ));
    }
    if envelope.sections.len() != REQUIRED_SECTIONS.len()
        || REQUIRED_SECTIONS.iter().any(|(id, major)| {
            envelope.sections.get(id).is_none_or(|section| {
                section.section_major != *major || section.section_minor != 0 || section.flags != 0
            })
        })
    {
        return Err(PersistenceError::codec(
            "unsupported schema-8 section combination",
        ));
    }
    let data = disassemble_envelope(&envelope)?;
    Runtime::from_snapshot(data)
        .map_err(|error| PersistenceError::codec(format!("runtime import failed: {error}")))
}

fn emitted_small_schema_8_bytes() -> Vec<u8> {
    emitted_schema_8_bytes()
}

fn assert_legacy_digest_rejected(case: &str, physical: bool) {
    let mut envelope = SnapshotEnvelope::decode(&emitted_small_schema_8_bytes())
        .expect("small schema-8 envelope decodes");
    if physical {
        envelope.header.physical_digest[0] ^= 1;
    } else {
        envelope.header.history_digest[0] ^= 1;
    }
    let bytes = envelope.encode().expect("mutated small envelope encodes");
    let error = match Runtime::from_snapshot_bytes(&bytes, None) {
        Ok(_) => panic!("{case}: corrupt digest installed"),
        Err(error) => error,
    };
    assert!(
        matches!(error, RuntimeError::InvalidSnapshot(message) if message.ends_with("does not match snapshot")),
        "{case}: unexpected error {error:?}"
    );
    println!("FAILURE_QA case={case} error={error:?} installed=false");
}

fn mutated_envelope(change: impl FnOnce(&mut SnapshotEnvelope)) -> Vec<u8> {
    let mut envelope = SnapshotEnvelope::decode(FIXTURE).expect("fixture decodes before mutation");
    change(&mut envelope);
    envelope.encode().expect("mutated envelope encodes")
}

fn assert_rejected(case: &str, bytes: &[u8], expected: impl FnOnce(&PersistenceError) -> bool) {
    let error = match load_schema_8(bytes) {
        Ok(_) => panic!("{case}: mutated fixture installed a runtime"),
        Err(error) => error,
    };
    assert!(expected(&error), "{case}: unexpected error {error:?}");
    println!("FAILURE_QA case={case} error={error:?} installed=false");
}

#[test]
fn checked_in_schema_8_bytes_match_the_explicit_legacy_writer() {
    let generated = emitted_schema_8_bytes();
    let envelope = SnapshotEnvelope::decode(&generated).expect("legacy writer output decodes");
    assert_eq!(envelope.header.physical_digest_schema, 8);
    assert_eq!(envelope.header.history_digest_schema, 8);
    let migrated = Runtime::from_snapshot_bytes(&generated, None)
        .expect("legacy writer output imports through the production migration path");
    let identities = migrated.snapshot().expect("migrated identities project");
    assert_eq!(
        identities.physical_state_digest.schema_version,
        CURRENT_DIGEST_SCHEMA_VERSION
    );
    assert_eq!(
        identities.history_digest.schema_version,
        CURRENT_DIGEST_SCHEMA_VERSION
    );
}

#[test]
fn schema_8_header_sections_digests_and_counts_are_frozen() {
    let given_envelope = SnapshotEnvelope::decode(FIXTURE).expect("fixture decodes");
    let when_data = disassemble_envelope(&given_envelope).expect("fixture disassembles");
    let runtime = Runtime::from_snapshot_bytes(&emitted_small_schema_8_bytes(), None)
        .expect("small schema-8 fixture installs and migrates");
    let identities = runtime.snapshot().expect("fixture identities project");
    let sections: Vec<(u64, u16)> = given_envelope
        .sections
        .iter()
        .map(|(id, section)| (*id, section.section_major))
        .collect();
    let hydrology_transfers: usize = when_data.hydrology.receipts.values().map(Vec::len).sum();

    assert_eq!(given_envelope.header.format_major, 1);
    assert_eq!(hex(&sha256(FIXTURE)), FIXTURE_SHA256);
    assert_eq!(given_envelope.header.format_minor, 0);
    assert_eq!(given_envelope.header.codec_revision, 1);
    assert_eq!(given_envelope.header.world_seed, 20_260_731);
    assert_eq!(given_envelope.header.completed_time, BASELINE_TICKS);
    assert_eq!(given_envelope.header.physical_digest_schema, 8);
    assert_eq!(given_envelope.header.history_digest_schema, 8);
    assert_eq!(
        hex(&given_envelope.header.physical_digest),
        BASELINE_PHYSICAL_DIGEST
    );
    assert_eq!(
        hex(&given_envelope.header.history_digest),
        BASELINE_HISTORY_DIGEST
    );
    assert_eq!(sections, REQUIRED_SECTIONS);
    assert_eq!(
        identities.physical_state_digest.schema_version,
        CURRENT_DIGEST_SCHEMA_VERSION
    );
    assert_eq!(
        identities.history_digest.schema_version,
        CURRENT_DIGEST_SCHEMA_VERSION
    );
    assert_ne!(
        hex(&identities.canonical_state.bytes()),
        BASELINE_EXPERIMENT_DIGEST,
        "migration must not present the legacy canonical identity as current"
    );
    assert_eq!(when_data.traces.events.len(), 37_862);
    assert_eq!(when_data.bootstrap.receipts.len(), 7);
    assert_eq!(when_data.experiment_recipe_mana_source_receipts.len(), 0);
    assert_eq!(when_data.thermal.receipt_batches.len(), 4);
    assert_eq!(when_data.thermal.transfer_receipts.len(), 41);
    assert_eq!(when_data.thermal.conservation_receipts.len(), 4);
    assert_eq!(when_data.thermal.boundary_records.len(), 54);
    assert_eq!(when_data.hydrology.retained_batches.len(), 4);
    assert_eq!(hydrology_transfers, 12_629);
    assert_eq!(when_data.hydrology.conservation_receipts.len(), 4);
    assert_eq!(when_data.hydrology.forcing.len(), 1);
    assert_eq!(when_data.material_surfaces.transitions.len(), 9);
    assert_eq!(when_data.material_surfaces.thermal_transitions.len(), 4);
    println!(
        "BASELINE fixture_sha256={FIXTURE_SHA256} physical_v8={BASELINE_PHYSICAL_DIGEST} history_v8={BASELINE_HISTORY_DIGEST} experiment_v8={BASELINE_EXPERIMENT_DIGEST} events=37862 sections={sections:?}"
    );
}

#[test]
fn malformed_persisted_bytes_fail_closed_before_runtime_installation() {
    let mut payload_bit = FIXTURE.to_vec();
    payload_bit[SnapshotHeader::SIZE] ^= 1;
    assert_rejected("one_bit_payload", &payload_bit, |error| {
        matches!(error, PersistenceError::PayloadIntegrityMismatch)
    });

    let mut trailing = FIXTURE.to_vec();
    trailing.push(0);
    assert_rejected("trailing_byte", &trailing, |error| {
        matches!(error, PersistenceError::TrailingBytes)
    });
}

#[test]
fn unsupported_header_and_digest_combinations_fail_closed() {
    let mut format_major = FIXTURE.to_vec();
    format_major[4..6].copy_from_slice(&2_u16.to_le_bytes());
    assert_rejected("format_major_2", &format_major, |error| {
        matches!(
            error,
            PersistenceError::UnsupportedMajorVersion { major: 2 }
        )
    });

    let codec = mutated_envelope(|envelope| envelope.header.codec_revision = 2);
    assert_rejected(
        "codec_revision_2",
        &codec,
        |error| matches!(error, PersistenceError::Codec { message } if message == "unsupported codec revision"),
    );

    for (case, physical, history) in [
        ("physical_digest_schema_9", 9, 8),
        ("history_digest_schema_9", 8, 9),
    ] {
        let bytes = mutated_envelope(|envelope| {
            envelope.header.physical_digest_schema = physical;
            envelope.header.history_digest_schema = history;
        });
        assert_rejected(
            case,
            &bytes,
            |error| matches!(error, PersistenceError::Codec { message } if message == "unsupported authoritative digest schema"),
        );
    }

    assert_legacy_digest_rejected("physical_digest_bit", true);
    assert_legacy_digest_rejected("history_digest_bit", false);
}

#[test]
fn missing_or_unsupported_sections_fail_closed() {
    let missing_hydrology = mutated_envelope(|envelope| {
        envelope.sections.remove(&u64::from(HYDROLOGY_SECTION_ID));
    });
    assert_rejected(
        "missing_hydrology_0x000f",
        &missing_hydrology,
        |error| matches!(error, PersistenceError::Codec { message } if message == "unsupported schema-8 section combination"),
    );

    for (case, id) in [
        ("runtime_recipe_major_8", u64::from(SECTION_RUNTIME_RECIPE)),
        (
            "population_bootstrap_major_3",
            u64::from(SECTION_POPULATION_BOOTSTRAP),
        ),
    ] {
        let bytes = mutated_envelope(|envelope| {
            envelope
                .sections
                .get_mut(&id)
                .expect("required section exists")
                .section_major += 1;
        });
        assert_rejected(
            case,
            &bytes,
            |error| matches!(error, PersistenceError::Codec { message } if message == "unsupported schema-8 section combination"),
        );
    }

    for &(id, major) in REQUIRED_SECTIONS {
        let bytes = mutated_envelope(|envelope| {
            envelope
                .sections
                .get_mut(&id)
                .expect("required section exists")
                .section_major = major + 1;
        });
        assert_rejected(
            &format!("section_{id:#06x}_major_{}", major + 1),
            &bytes,
            |error| matches!(error, PersistenceError::Codec { message } if message == "unsupported schema-8 section combination"),
        );
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn sha256(bytes: &[u8]) -> [u8; 32] {
    const INITIAL: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];
    const ROUND: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];
    let bit_len = (bytes.len() as u64).wrapping_mul(8);
    let mut padded = bytes.to_vec();
    padded.push(0x80);
    while padded.len() % 64 != 56 {
        padded.push(0);
    }
    padded.extend_from_slice(&bit_len.to_be_bytes());
    let mut state = INITIAL;
    let (chunks, remainder) = padded.as_chunks::<64>();
    debug_assert!(remainder.is_empty());
    for chunk in chunks {
        let mut schedule = [0_u32; 64];
        for (index, word) in chunk.as_chunks::<4>().0.iter().enumerate() {
            schedule[index] = u32::from_be_bytes([word[0], word[1], word[2], word[3]]);
        }
        for index in 16..64 {
            let s0 = schedule[index - 15].rotate_right(7)
                ^ schedule[index - 15].rotate_right(18)
                ^ (schedule[index - 15] >> 3);
            let s1 = schedule[index - 2].rotate_right(17)
                ^ schedule[index - 2].rotate_right(19)
                ^ (schedule[index - 2] >> 10);
            schedule[index] = schedule[index - 16]
                .wrapping_add(s0)
                .wrapping_add(schedule[index - 7])
                .wrapping_add(s1);
        }
        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h] = state;
        for index in 0..64 {
            let sigma1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let choice = (e & f) ^ (!e & g);
            let temp1 = h
                .wrapping_add(sigma1)
                .wrapping_add(choice)
                .wrapping_add(ROUND[index])
                .wrapping_add(schedule[index]);
            let sigma0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let majority = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = sigma0.wrapping_add(majority);
            (h, g, f, e, d, c, b, a) = (
                g,
                f,
                e,
                d.wrapping_add(temp1),
                c,
                b,
                a,
                temp1.wrapping_add(temp2),
            );
        }
        for (slot, value) in state.iter_mut().zip([a, b, c, d, e, f, g, h]) {
            *slot = slot.wrapping_add(value);
        }
    }
    let mut digest = [0_u8; 32];
    for (output, word) in digest.as_chunks_mut::<4>().0.iter_mut().zip(state) {
        output.copy_from_slice(&word.to_be_bytes());
    }
    digest
}
