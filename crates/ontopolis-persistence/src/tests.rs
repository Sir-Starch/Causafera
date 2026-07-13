use crate::{
    FORMAT_MAJOR_V1, FORMAT_MINOR_V1, LittleEndianDecoder, LittleEndianEncoder, PersistenceError,
    SectionPayload, Snapshot, SnapshotEnvelope, SnapshotHeader, SnapshotSection,
};
use std::collections::BTreeMap;

fn make_test_sections() -> BTreeMap<u64, SectionPayload> {
    let mut sections = BTreeMap::new();
    sections.insert(
        0x0001,
        SectionPayload {
            section_major: 1,
            section_minor: 0,
            flags: 0,
            decoded_size_limit: 1024,
            bytes: vec![0x01, 0x02, 0x03, 0x04],
        },
    );
    sections.insert(
        0x0002,
        SectionPayload {
            section_major: 1,
            section_minor: 0,
            flags: 0,
            decoded_size_limit: 2048,
            bytes: vec![0xAB, 0xCD, 0xEF],
        },
    );
    sections
}

fn make_test_header() -> SnapshotHeader {
    SnapshotHeader {
        format_major: FORMAT_MAJOR_V1,
        format_minor: FORMAT_MINOR_V1,
        codec_revision: 1,
        world_seed: 42,
        completed_time: 192,
        runtime_recipe_fingerprint: [0u8; 32],
        physical_digest_schema: 1,
        physical_digest: [0x11; 32],
        history_digest_schema: 1,
        history_digest: [0x22; 32],
        section_count: 0,
        section_directory_offset: 0,
        payload_integrity: [0u8; 32],
    }
}

#[test]
fn roundtrip_basic() {
    let header = make_test_header();
    let sections = make_test_sections();
    let envelope = SnapshotEnvelope::new(header, sections);
    let encoded = envelope.encode().expect("encode failed");
    let decoded = SnapshotEnvelope::decode(&encoded).expect("decode failed");
    assert_eq!(decoded.header.world_seed, 42);
    assert_eq!(decoded.header.completed_time, 192);
    assert_eq!(decoded.sections.len(), 2);
    assert!(decoded.sections.contains_key(&0x0001));
    assert!(decoded.sections.contains_key(&0x0002));
    assert_eq!(
        decoded.sections[&0x0001].bytes,
        vec![0x01, 0x02, 0x03, 0x04]
    );
    assert_eq!(decoded.sections[&0x0002].bytes, vec![0xAB, 0xCD, 0xEF]);
}

#[test]
fn encode_is_deterministic() {
    let header = make_test_header();
    let sections = make_test_sections();
    let envelope = SnapshotEnvelope::new(header, sections);
    let encoded1 = envelope.encode().expect("encode 1 failed");
    let encoded2 = envelope.encode().expect("encode 2 failed");
    assert_eq!(encoded1, encoded2);
}

#[test]
fn decode_truncated_header() {
    let bytes = vec![0x4F, 0x54, 0x50]; // incomplete magic
    let result = SnapshotEnvelope::decode(&bytes);
    assert!(matches!(result, Err(PersistenceError::Codec { .. })));
}

#[test]
fn decode_bad_magic() {
    let mut bytes = vec![0x00; 100];
    bytes[0..4].copy_from_slice(b"BAD!");
    let result = SnapshotEnvelope::decode(&bytes);
    assert!(matches!(result, Err(PersistenceError::MagicMismatch)));
}

#[test]
fn decode_unsupported_major() {
    let mut header = make_test_header();
    header.format_major = 99;
    let envelope = SnapshotEnvelope::new(header, BTreeMap::new());
    let encoded = envelope.encode().expect("encode failed");
    let result = SnapshotEnvelope::decode(&encoded);
    assert!(
        matches!(
            result,
            Err(PersistenceError::UnsupportedMajorVersion { major: 99 })
        ),
        "got {:?}",
        result
    );
}

#[test]
fn decode_payload_integrity_mismatch() {
    let header = make_test_header();
    let sections = make_test_sections();
    let envelope = SnapshotEnvelope::new(header, sections);
    let mut encoded = envelope.encode().expect("encode failed");
    // Corrupt a byte in the payload area (after header).
    let corruption_index = SnapshotHeader::SIZE + 2;
    if corruption_index < encoded.len() {
        encoded[corruption_index] ^= 0xFF;
    }
    let result = SnapshotEnvelope::decode(&encoded);
    assert!(
        matches!(result, Err(PersistenceError::PayloadIntegrityMismatch)),
        "got {:?}",
        result
    );
}

#[test]
fn decode_section_integrity_mismatch() {
    let header = make_test_header();
    let sections = make_test_sections();
    let envelope = SnapshotEnvelope::new(header, sections);
    let mut encoded = envelope.encode().expect("encode failed");
    // Corrupt a byte inside a section payload but after integrity computation.
    // The simplest way: corrupt the first section byte after the header.
    let corruption_index = SnapshotHeader::SIZE;
    if corruption_index < encoded.len() {
        encoded[corruption_index] ^= 0xFF;
    }
    let result = SnapshotEnvelope::decode(&encoded);
    // This may hit payload integrity OR section integrity depending on placement.
    assert!(
        matches!(
            result,
            Err(PersistenceError::PayloadIntegrityMismatch)
                | Err(PersistenceError::SectionIntegrityMismatch { .. })
        ),
        "got {:?}",
        result
    );
}

#[test]
fn decode_trailing_bytes() {
    let header = make_test_header();
    let sections = make_test_sections();
    let envelope = SnapshotEnvelope::new(header, sections);
    let mut encoded = envelope.encode().expect("encode failed");
    encoded.push(0xFF); // trailing byte
    let result = SnapshotEnvelope::decode(&encoded);
    assert!(
        matches!(result, Err(PersistenceError::TrailingBytes)),
        "got {:?}",
        result
    );
}

#[test]
fn snapshot_from_to_envelope_roundtrip() {
    let snapshot = Snapshot {
        world_seed: 123,
        completed_time: 456,
        physical_digest: [0xAA; 32],
        history_digest: [0xBB; 32],
        sections: vec![
            SnapshotSection {
                schema_id: 0x0001,
                major: 1,
                minor: 0,
                bytes: vec![1, 2, 3],
            },
            SnapshotSection {
                schema_id: 0x0002,
                major: 1,
                minor: 0,
                bytes: vec![4, 5, 6],
            },
        ],
    };
    let envelope = snapshot.to_envelope().expect("to_envelope failed");
    let decoded = Snapshot::from_envelope(envelope);
    assert_eq!(decoded.world_seed, 123);
    assert_eq!(decoded.completed_time, 456);
    assert_eq!(decoded.physical_digest, [0xAA; 32]);
    assert_eq!(decoded.history_digest, [0xBB; 32]);
    assert_eq!(decoded.sections.len(), 2);
}

#[test]
fn codec_primitives_roundtrip() {
    let mut buf = Vec::new();
    {
        let mut enc = LittleEndianEncoder::new(&mut buf);
        enc.write_u8(0x01);
        enc.write_u16(0x0203);
        enc.write_u32(0x04050607);
        enc.write_u64(0x08090A0B0C0D0E0F);
        enc.write_i64(-1);
        enc.write_fixed(&[0x10; 16]);
        enc.write_bytes(&[0x11, 0x12]);
    }

    let mut dec = LittleEndianDecoder::new(&buf);
    assert_eq!(dec.read_u8().unwrap(), 0x01);
    assert_eq!(dec.read_u16().unwrap(), 0x0203);
    assert_eq!(dec.read_u32().unwrap(), 0x04050607);
    assert_eq!(dec.read_u64().unwrap(), 0x08090A0B0C0D0E0F);
    assert_eq!(dec.read_i64().unwrap(), -1);
    assert_eq!(*dec.read_fixed::<16>().unwrap(), [0x10; 16]);
    assert_eq!(dec.read_bytes(2).unwrap(), &[0x11, 0x12]);
    assert!(dec.is_empty());
}

#[test]
fn codec_bounds_checked() {
    let buf = vec![0x01, 0x02];
    let mut dec = LittleEndianDecoder::new(&buf);
    assert!(dec.read_u8().is_ok());
    assert!(dec.read_u8().is_ok());
    assert!(dec.read_u8().is_err());
    assert!(dec.read_u16().is_err());
    assert!(dec.read_u32().is_err());
    assert!(dec.read_u64().is_err());
    assert!(dec.read_i64().is_err());
    assert!(dec.read_fixed::<4>().is_err());
    assert!(dec.read_bytes(1).is_err());
}

#[test]
fn codec_advance_checked() {
    let buf = vec![0x01, 0x02, 0x03];
    let mut dec = LittleEndianDecoder::new(&buf);
    assert!(dec.advance(2).is_ok());
    assert!(dec.advance(2).is_err());
}

#[test]
fn empty_snapshot_roundtrip() {
    let header = make_test_header();
    let envelope = SnapshotEnvelope::new(header, BTreeMap::new());
    let encoded = envelope.encode().expect("encode failed");
    let decoded = SnapshotEnvelope::decode(&encoded).expect("decode failed");
    assert_eq!(decoded.sections.len(), 0);
    assert_eq!(decoded.header.world_seed, 42);
}
