//! Canonical causal-history storage format version 1.
//!
//! Frames use CFHF, a 168-byte little-endian header, and a body containing a
//! u64 record count followed by u64-length-delimited records. The length covers
//! the record tag, schema version, and payload. Causal payload fields retain the
//! core canonical order and fingerprints remain four raw u64 words each.
//! Separate versioned BLAKE3 domains identify frames, segments, segment roots,
//! and the segment-boundary-independent logical record stream.

use std::{cell::Cell, collections::BTreeSet};

use causafera_core::{
    CausalEffect, CausalTarget, Criticality, DomainEvidenceRecord, EvidenceDecoderRegistry,
    EvidenceResolution, EvidenceSchemaId, EvidenceSchemaVersion, HistoryError, HistoryRecord,
    MAX_CAUSES_PER_EVENT, MAX_CHECKPOINT_DECODED_BYTES, MAX_EFFECTS_PER_EVENT,
    MAX_FRAME_DECODED_BYTES, OwnedCausalRecord, PhaseId, StateFingerprint,
    validate_checkpoint_decoded_size, validate_frame_shape,
};
use causafera_types::{
    EventId, EventKindId, SimulationTime, StateObjectKindId, StatePropertyId, TraceId,
};

use crate::PersistenceError;

pub const HISTORY_FRAME_MAGIC: [u8; 4] = *b"CFHF";
pub const HISTORY_FORMAT_MAJOR: u16 = 1;
pub const HISTORY_FORMAT_MINOR: u16 = 0;
/// Current authoritative digest schema shared by runtime, checkpoints, and
/// bounded snapshots. Legacy schema 8 remains readable at the snapshot
/// migration boundary but is never emitted by current writers.
pub const CURRENT_DIGEST_SCHEMA_VERSION: u16 = 9;
pub const LEGACY_DIGEST_SCHEMA_VERSION: u16 = 8;
pub const HISTORY_FRAME_HEADER_LEN: usize = 168;
pub const HISTORY_FRAME_DOMAIN: &[u8] = b"causafera/history-frame/v1\0";
pub const HISTORY_SEGMENT_DOMAIN: &[u8] = b"causafera/history-segment/v1\0";
pub const HISTORY_SEGMENT_ROOT_DOMAIN: &[u8] = b"causafera/history-segment-root/v1\0";
pub const LOGICAL_HISTORY_DOMAIN: &[u8] = b"causafera/logical-history/v2-incremental\0";
pub const EVIDENCE_MANIFEST_DOMAIN: &[u8] = b"causafera/evidence-manifest/v2-incremental\0";
pub const EVIDENCE_MANIFEST_MAJOR: u16 = 2;
pub const EVIDENCE_MANIFEST_MINOR: u16 = 0;
pub const EMPTY_EVIDENCE_MANIFEST_DIGEST: [u8; 32] = [
    0xeb, 0x4f, 0xbc, 0xe1, 0xd6, 0xb8, 0x7d, 0x48, 0xd3, 0xb9, 0x53, 0xe4, 0x48, 0x2d, 0x9a, 0x52,
    0x0c, 0x66, 0xd9, 0xb7, 0x52, 0xa1, 0x41, 0xe9, 0x89, 0x5e, 0xbd, 0xbe, 0x61, 0x1a, 0xc0, 0xb2,
];

const HEADER_PREIMAGE_LEN: usize = 136;
const CAUSAL_TAG: u8 = 1;
const EVIDENCE_TAG: u8 = 2;
const RECORD_MAJOR: u16 = 1;
const RECORD_MINOR: u16 = 0;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HistoryFrameHeaderFields {
    pub namespace: [u8; 32],
    pub catalog_generation: u64,
    pub first_record_sequence: u64,
    pub last_record_sequence: u64,
    pub first_event_id: u64,
    pub last_event_id: u64,
    pub first_trace_id: u64,
    pub last_trace_id: u64,
    pub prior_logical_history_digest: [u8; 32],
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HistoryFrameHeader {
    pub format_major: u16,
    pub format_minor: u16,
    pub fields: HistoryFrameHeaderFields,
    pub payload_length: u64,
    pub frame_digest: [u8; 32],
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DecodedHistoryFrame {
    header: HistoryFrameHeader,
    records: Vec<HistoryRecord>,
}

#[derive(Clone)]
pub struct LogicalHistoryDigest {
    hasher: blake3::Hasher,
}

#[derive(Clone)]
pub struct EvidenceManifestDigest {
    hasher: blake3::Hasher,
    count: u64,
}

impl EvidenceManifestDigest {
    pub fn new() -> Self {
        let mut hasher = blake3::Hasher::new();
        hasher.update(EVIDENCE_MANIFEST_DOMAIN);
        hasher.update(&EVIDENCE_MANIFEST_MAJOR.to_le_bytes());
        hasher.update(&EVIDENCE_MANIFEST_MINOR.to_le_bytes());
        Self { hasher, count: 0 }
    }

    pub fn from_records(records: &[HistoryRecord]) -> Result<Self, PersistenceError> {
        let mut digest = Self::new();
        for record in records {
            if matches!(record, HistoryRecord::Evidence(_)) {
                digest.append(record)?;
            }
        }
        Ok(digest)
    }

    pub fn append(&mut self, record: &HistoryRecord) -> Result<(), PersistenceError> {
        let HistoryRecord::Evidence(record) = record else {
            return Ok(());
        };
        verify_payload(record)?;
        self.hasher.update(&record.schema_id().raw().to_le_bytes());
        self.hasher.update(&record.version().major.to_le_bytes());
        self.hasher.update(&record.version().minor.to_le_bytes());
        self.hasher
            .update(&to_u64(record.canonical_key().len())?.to_le_bytes());
        self.hasher.update(record.canonical_key());
        self.hasher
            .update(&to_u64(record.canonical_payload().len())?.to_le_bytes());
        self.hasher.update(record.canonical_payload());
        self.hasher.update(&record.payload_digest().bytes());
        self.count = self
            .count
            .checked_add(1)
            .ok_or(PersistenceError::HistoryArithmeticOverflow)?;
        Ok(())
    }

    pub fn finish(&self) -> [u8; 32] {
        let mut hasher = self.hasher.clone();
        hasher.update(&self.count.to_le_bytes());
        *hasher.finalize().as_bytes()
    }
}

impl Default for EvidenceManifestDigest {
    fn default() -> Self {
        Self::new()
    }
}

thread_local! {
    static EVIDENCE_MANIFEST_FULL_RESCAN_CALLS: Cell<usize> = const { Cell::new(0) };
}

pub fn reset_evidence_manifest_full_rescan_counter() {
    EVIDENCE_MANIFEST_FULL_RESCAN_CALLS.with(|calls| calls.set(0));
}

pub fn evidence_manifest_full_rescan_count() -> usize {
    EVIDENCE_MANIFEST_FULL_RESCAN_CALLS.with(Cell::get)
}

impl LogicalHistoryDigest {
    pub fn new() -> Self {
        let mut hasher = blake3::Hasher::new();
        hasher.update(LOGICAL_HISTORY_DOMAIN);
        Self { hasher }
    }

    pub fn from_records(records: &[HistoryRecord]) -> Result<Self, PersistenceError> {
        validate_order(records)?;
        let mut digest = Self::new();
        for record in records {
            digest.append(record)?;
        }
        Ok(digest)
    }

    pub fn append(&mut self, record: &HistoryRecord) -> Result<(), PersistenceError> {
        let bytes = encode_record(record)?;
        self.hasher.update(&to_u64(bytes.len())?.to_le_bytes());
        self.hasher.update(&bytes);
        Ok(())
    }

    pub fn finish(&self) -> [u8; 32] {
        *self.hasher.finalize().as_bytes()
    }
}

impl Default for LogicalHistoryDigest {
    fn default() -> Self {
        Self::new()
    }
}

impl DecodedHistoryFrame {
    pub const fn header(&self) -> &HistoryFrameHeader {
        &self.header
    }

    pub fn records(&self) -> &[HistoryRecord] {
        &self.records
    }
}

pub fn encode_history_frame(
    fields: HistoryFrameHeaderFields,
    records: &[HistoryRecord],
) -> Result<Vec<u8>, PersistenceError> {
    validate_records(records, &fields)?;
    let encoded = records
        .iter()
        .map(encode_record)
        .collect::<Result<Vec<_>, _>>()?;
    let payload_len = encoded.iter().try_fold(8_usize, |total, record| {
        total
            .checked_add(8)
            .and_then(|value| value.checked_add(record.len()))
            .ok_or(PersistenceError::HistoryArithmeticOverflow)
    })?;
    validate_frame_shape(records.len(), payload_len)?;
    let mut payload = Vec::with_capacity(payload_len);
    push_u64(&mut payload, to_u64(records.len())?);
    for record in &encoded {
        push_u64(&mut payload, to_u64(record.len())?);
        payload.extend_from_slice(record);
    }
    let mut header = header_preimage(&fields, to_u64(payload.len())?);
    let digest = digest(HISTORY_FRAME_DOMAIN, &[&header, &payload]);
    let total = HISTORY_FRAME_HEADER_LEN
        .checked_add(payload.len())
        .ok_or(PersistenceError::HistoryArithmeticOverflow)?;
    let mut frame = Vec::with_capacity(total);
    frame.append(&mut header);
    frame.extend_from_slice(&digest);
    frame.extend_from_slice(&payload);
    Ok(frame)
}

pub fn decode_history_frame(
    bytes: &[u8],
    registry: &EvidenceDecoderRegistry,
) -> Result<DecodedHistoryFrame, PersistenceError> {
    if bytes.len() < HISTORY_FRAME_HEADER_LEN {
        return Err(length_error(
            0,
            HISTORY_FRAME_HEADER_LEN as u64,
            bytes.len(),
        ));
    }
    let mut cursor = Cursor::new(bytes);
    if cursor.fixed::<4>()? != &HISTORY_FRAME_MAGIC {
        return Err(PersistenceError::HistoryMagicMismatch);
    }
    let major = cursor.u16()?;
    let minor = cursor.u16()?;
    if (major, minor) != (HISTORY_FORMAT_MAJOR, HISTORY_FORMAT_MINOR) {
        return Err(PersistenceError::UnsupportedHistoryFormat { major, minor });
    }
    let fields = HistoryFrameHeaderFields {
        namespace: *cursor.fixed::<32>()?,
        catalog_generation: cursor.u64()?,
        first_record_sequence: cursor.u64()?,
        last_record_sequence: cursor.u64()?,
        first_event_id: cursor.u64()?,
        last_event_id: cursor.u64()?,
        first_trace_id: cursor.u64()?,
        last_trace_id: cursor.u64()?,
        prior_logical_history_digest: *cursor.fixed::<32>()?,
    };
    let payload_length = cursor.u64()?;
    if payload_length > MAX_FRAME_DECODED_BYTES as u64 {
        return Err(PersistenceError::HistoryFrameSizeExceeded {
            size: payload_length,
            max: MAX_FRAME_DECODED_BYTES,
        });
    }
    let expected_digest = *cursor.fixed::<32>()?;
    let payload_len = checked_length(payload_length, cursor.position(), cursor.remaining())?;
    if payload_len < cursor.remaining() {
        return Err(PersistenceError::TrailingHistoryBytes {
            remaining: cursor.remaining() - payload_len,
        });
    }
    let payload = cursor.bytes(payload_len)?;
    let actual = digest(
        HISTORY_FRAME_DOMAIN,
        &[&bytes[..HEADER_PREIMAGE_LEN], payload],
    );
    if actual != expected_digest {
        return Err(PersistenceError::FrameDigestMismatch);
    }
    let records = decode_payload(payload, registry)?;
    validate_records(&records, &fields)?;
    Ok(DecodedHistoryFrame {
        header: HistoryFrameHeader {
            format_major: major,
            format_minor: minor,
            fields,
            payload_length,
            frame_digest: expected_digest,
        },
        records,
    })
}

pub fn logical_history_digest(records: &[HistoryRecord]) -> Result<[u8; 32], PersistenceError> {
    Ok(LogicalHistoryDigest::from_records(records)?.finish())
}

/// Canonical digest of the durable evidence set covered by a history anchor.
///
/// Entries are ordered by their complete manifest tuple rather than storage or
/// map iteration order. Each variable-width field is length-delimited.
pub fn evidence_manifest_digest_full_rescan(
    records: &[HistoryRecord],
) -> Result<[u8; 32], PersistenceError> {
    EVIDENCE_MANIFEST_FULL_RESCAN_CALLS.with(|calls| calls.set(calls.get().saturating_add(1)));
    Ok(EvidenceManifestDigest::from_records(records)?.finish())
}

pub fn sealed_segment_identity(immutable_header: &[u8], frames: &[&[u8]]) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new();
    hasher.update(HISTORY_SEGMENT_DOMAIN);
    hasher.update(immutable_header);
    for frame in frames {
        hasher.update(frame);
    }
    *hasher.finalize().as_bytes()
}

pub fn sealed_segment_root(identities: &[[u8; 32]]) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new();
    hasher.update(HISTORY_SEGMENT_ROOT_DOMAIN);
    hasher.update(&(identities.len() as u64).to_le_bytes());
    for identity in identities {
        hasher.update(identity);
    }
    *hasher.finalize().as_bytes()
}

pub fn verify_sealed_segment_identity(
    expected: [u8; 32],
    immutable_header: &[u8],
    frames: &[&[u8]],
) -> Result<(), PersistenceError> {
    if sealed_segment_identity(immutable_header, frames) != expected {
        return Err(PersistenceError::SegmentDigestMismatch);
    }
    Ok(())
}

pub fn verify_logical_history_digest(
    expected: [u8; 32],
    records: &[HistoryRecord],
) -> Result<(), PersistenceError> {
    if logical_history_digest(records)? != expected {
        return Err(PersistenceError::LogicalDigestMismatch);
    }
    Ok(())
}

pub fn validate_checkpoint_bytes(bytes: &[u8]) -> Result<(), PersistenceError> {
    validate_checkpoint_length(to_u64(bytes.len())?)
}

pub fn validate_checkpoint_length(decoded_length: u64) -> Result<(), PersistenceError> {
    if decoded_length > MAX_CHECKPOINT_DECODED_BYTES as u64 {
        return Err(HistoryError::DecodedSizeLimitExceeded {
            kind: "checkpoint",
            size: MAX_CHECKPOINT_DECODED_BYTES.saturating_add(1),
            max: MAX_CHECKPOINT_DECODED_BYTES,
        }
        .into());
    }
    let length =
        usize::try_from(decoded_length).map_err(|_| PersistenceError::HistoryArithmeticOverflow)?;
    validate_checkpoint_decoded_size(length).map_err(PersistenceError::from)
}

fn decode_payload(
    payload: &[u8],
    registry: &EvidenceDecoderRegistry,
) -> Result<Vec<HistoryRecord>, PersistenceError> {
    let mut cursor = Cursor::new(payload);
    let count = checked_count(cursor.u64()?, causafera_core::MAX_HISTORY_RECORDS_PER_FRAME)?;
    validate_frame_shape(count, payload.len())?;
    if count == 0 {
        return Err(PersistenceError::EmptyHistoryFrame);
    }
    let mut records = Vec::with_capacity(count);
    for _ in 0..count {
        let offset = cursor.position();
        let length = checked_length(cursor.u64()?, offset, cursor.remaining())?;
        records.push(decode_record(cursor.bytes(length)?, registry)?);
    }
    if !cursor.is_empty() {
        return Err(PersistenceError::TrailingHistoryBytes {
            remaining: cursor.remaining(),
        });
    }
    Ok(records)
}

fn header_preimage(fields: &HistoryFrameHeaderFields, payload_length: u64) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(HEADER_PREIMAGE_LEN);
    bytes.extend_from_slice(&HISTORY_FRAME_MAGIC);
    push_u16(&mut bytes, HISTORY_FORMAT_MAJOR);
    push_u16(&mut bytes, HISTORY_FORMAT_MINOR);
    bytes.extend_from_slice(&fields.namespace);
    for value in [
        fields.catalog_generation,
        fields.first_record_sequence,
        fields.last_record_sequence,
        fields.first_event_id,
        fields.last_event_id,
        fields.first_trace_id,
        fields.last_trace_id,
    ] {
        push_u64(&mut bytes, value);
    }
    bytes.extend_from_slice(&fields.prior_logical_history_digest);
    push_u64(&mut bytes, payload_length);
    bytes
}

fn encode_record(record: &HistoryRecord) -> Result<Vec<u8>, PersistenceError> {
    match record {
        HistoryRecord::Causal(record) => {
            let fields = record.canonical_field_bytes()?;
            let mut bytes = Vec::with_capacity(
                13_usize
                    .checked_add(fields.len())
                    .ok_or(PersistenceError::HistoryArithmeticOverflow)?,
            );
            record_prefix(&mut bytes, CAUSAL_TAG);
            push_u64(&mut bytes, record.commit_sequence());
            bytes.extend_from_slice(&fields);
            Ok(bytes)
        }
        HistoryRecord::Evidence(record) => encode_evidence(record),
    }
}

fn encode_evidence(record: &DomainEvidenceRecord) -> Result<Vec<u8>, PersistenceError> {
    verify_payload(record)?;
    let estimated = 70_usize
        .checked_add(
            record
                .causal_anchors()
                .len()
                .checked_mul(8)
                .ok_or(PersistenceError::HistoryArithmeticOverflow)?,
        )
        .and_then(|value| value.checked_add(record.canonical_key().len()))
        .and_then(|value| value.checked_add(record.canonical_payload().len()))
        .ok_or(PersistenceError::HistoryArithmeticOverflow)?;
    if estimated > MAX_FRAME_DECODED_BYTES {
        return Err(PersistenceError::HistoryFrameSizeExceeded {
            size: to_u64(estimated)?,
            max: MAX_FRAME_DECODED_BYTES,
        });
    }
    let mut bytes = Vec::with_capacity(estimated);
    record_prefix(&mut bytes, EVIDENCE_TAG);
    push_u64(&mut bytes, record.commit_sequence());
    push_u64(&mut bytes, record.schema_id().raw());
    push_u16(&mut bytes, record.version().major);
    push_u16(&mut bytes, record.version().minor);
    bytes.push(match record.criticality() {
        Criticality::ReplayRequired => 1,
        Criticality::ExplanationOnly => 2,
    });
    push_u64(&mut bytes, record.time().raw());
    push_u64(&mut bytes, to_u64(record.causal_anchors().len())?);
    for anchor in record.causal_anchors() {
        push_u64(&mut bytes, anchor.raw());
    }
    write_slice(&mut bytes, record.canonical_key())?;
    write_slice(&mut bytes, record.canonical_payload())?;
    bytes.extend_from_slice(&record.payload_digest().bytes());
    Ok(bytes)
}

fn decode_record(
    bytes: &[u8],
    registry: &EvidenceDecoderRegistry,
) -> Result<HistoryRecord, PersistenceError> {
    let mut cursor = Cursor::new(bytes);
    let tag = cursor.u8()?;
    let major = cursor.u16()?;
    let minor = cursor.u16()?;
    if (major, minor) != (RECORD_MAJOR, RECORD_MINOR) {
        return Err(PersistenceError::UnsupportedHistoryRecord { tag, major, minor });
    }
    let record = match tag {
        CAUSAL_TAG => HistoryRecord::Causal(decode_causal(&mut cursor)?),
        EVIDENCE_TAG => HistoryRecord::Evidence(decode_evidence(&mut cursor, registry)?),
        _ => return Err(PersistenceError::UnknownHistoryRecordTag { tag }),
    };
    if !cursor.is_empty() {
        return Err(PersistenceError::TrailingHistoryBytes {
            remaining: cursor.remaining(),
        });
    }
    Ok(record)
}

fn decode_causal(cursor: &mut Cursor<'_>) -> Result<OwnedCausalRecord, PersistenceError> {
    let commit_sequence = cursor.u64()?;
    let event_id = EventId::new(cursor.u64()?);
    let trace_id = TraceId::new(cursor.u64()?);
    let time = SimulationTime::new(cursor.u64()?);
    let phase_raw = cursor.u64()?;
    let phase_byte = u8::try_from(phase_raw)
        .map_err(|_| PersistenceError::UnsupportedPhase { value: phase_raw })?;
    let phase = PhaseId(phase_byte)
        .to_phase()
        .ok_or(PersistenceError::UnsupportedPhase { value: phase_raw })?;
    let kind = EventKindId::new(cursor.u64()?);
    let cause_count = checked_count(cursor.u64()?, MAX_CAUSES_PER_EVENT)?;
    let mut causes = Vec::with_capacity(cause_count);
    for _ in 0..cause_count {
        causes.push(TraceId::new(cursor.u64()?));
    }
    let effect_count = checked_count(cursor.u64()?, MAX_EFFECTS_PER_EVENT)?;
    let mut effects = Vec::with_capacity(effect_count);
    for _ in 0..effect_count {
        let target = CausalTarget::new(
            StateObjectKindId::new(cursor.u64()?),
            cursor.u64()?,
            StatePropertyId::new(cursor.u64()?),
        );
        let before = StateFingerprint::new(*cursor.fixed::<32>()?);
        let after = StateFingerprint::new(*cursor.fixed::<32>()?);
        effects.push(
            CausalEffect::new(target, before, after)
                .map_err(|_| PersistenceError::UnchangedCausalEffect)?,
        );
    }
    OwnedCausalRecord::new(
        commit_sequence,
        event_id,
        trace_id,
        time,
        phase,
        kind,
        causes,
        effects,
    )
    .map_err(PersistenceError::from)
}

fn decode_evidence(
    cursor: &mut Cursor<'_>,
    registry: &EvidenceDecoderRegistry,
) -> Result<DomainEvidenceRecord, PersistenceError> {
    let commit_sequence = cursor.u64()?;
    let schema_id = EvidenceSchemaId::new(cursor.u64()?);
    let version = EvidenceSchemaVersion::new(cursor.u16()?, cursor.u16()?);
    let criticality = match cursor.u8()? {
        1 => Criticality::ReplayRequired,
        2 => Criticality::ExplanationOnly,
        value => return Err(PersistenceError::UnsupportedEvidenceCriticality { value }),
    };
    let time = SimulationTime::new(cursor.u64()?);
    let count = checked_count(cursor.u64()?, MAX_CAUSES_PER_EVENT)?;
    let mut anchors = Vec::with_capacity(count);
    for _ in 0..count {
        anchors.push(TraceId::new(cursor.u64()?));
    }
    let key = cursor.length_delimited()?.to_vec();
    let payload = cursor.length_delimited()?.to_vec();
    let payload_digest = StateFingerprint::new(*cursor.fixed::<32>()?);
    let record = DomainEvidenceRecord::new(
        commit_sequence,
        schema_id,
        version,
        criticality,
        time,
        anchors,
        key,
        payload,
        payload_digest,
    )?;
    verify_payload(&record)?;
    match registry.resolve(&record)? {
        EvidenceResolution::Decoded(_) | EvidenceResolution::Raw(_) => Ok(record),
    }
}

fn validate_records(
    records: &[HistoryRecord],
    fields: &HistoryFrameHeaderFields,
) -> Result<(), PersistenceError> {
    if records.is_empty() {
        return Err(PersistenceError::EmptyHistoryFrame);
    }
    validate_frame_shape(records.len(), 0)?;
    validate_order(records)?;
    let expected_last = fields
        .first_record_sequence
        .checked_add(to_u64(records.len() - 1)?)
        .ok_or(PersistenceError::HistoryArithmeticOverflow)?;
    if fields.first_record_sequence > fields.last_record_sequence
        || expected_last != fields.last_record_sequence
    {
        return Err(invalid_range("record_sequence", fields));
    }
    let mut events = BTreeSet::new();
    let mut traces = BTreeSet::new();
    let mut evidence_keys = BTreeSet::new();
    let mut previous_event = None;
    let mut previous_trace = None;
    for (index, record) in records.iter().enumerate() {
        match record {
            HistoryRecord::Causal(record) => {
                if !events.insert(record.event_id()) {
                    return Err(HistoryError::DuplicateEventId(record.event_id()).into());
                }
                if !traces.insert(record.trace_id()) {
                    return Err(HistoryError::DuplicateTraceId(record.trace_id()).into());
                }
                if previous_event.is_some_and(|event| event >= record.event_id())
                    || previous_trace.is_some_and(|trace| trace >= record.trace_id())
                {
                    return Err(HistoryError::NonCanonicalOrder { index }.into());
                }
                for parent in record.causes() {
                    if parent.raw() >= fields.first_trace_id && !traces.contains(parent) {
                        return Err(HistoryError::UnknownParent {
                            trace_id: record.trace_id(),
                            parent: *parent,
                        }
                        .into());
                    }
                }
                previous_event = Some(record.event_id());
                previous_trace = Some(record.trace_id());
            }
            HistoryRecord::Evidence(record) => {
                if !evidence_keys.insert((record.schema_id(), record.canonical_key().to_vec())) {
                    return Err(HistoryError::DuplicateRecordKey.into());
                }
            }
        }
    }
    validate_id_ranges(records, fields)
}

fn validate_id_ranges(
    records: &[HistoryRecord],
    fields: &HistoryFrameHeaderFields,
) -> Result<(), PersistenceError> {
    let mut causal = records.iter().filter_map(|record| match record {
        HistoryRecord::Causal(record) => Some(record),
        HistoryRecord::Evidence(_) => None,
    });
    let first = causal.next();
    let last = causal.next_back().or(first);
    match (first, last) {
        (Some(first), Some(last)) => {
            validate_causal_header_range("event_id", fields.first_event_id, fields.last_event_id)?;
            validate_causal_header_range("trace_id", fields.first_trace_id, fields.last_trace_id)?;
            if (fields.first_event_id, fields.last_event_id)
                == (first.event_id().raw(), last.event_id().raw())
                && (fields.first_trace_id, fields.last_trace_id)
                    == (first.trace_id().raw(), last.trace_id().raw())
            {
                Ok(())
            } else {
                Err(invalid_range("causal_identity", fields))
            }
        }
        (None, None)
            if fields.first_event_id == 0
                && fields.last_event_id == 0
                && fields.first_trace_id == 0
                && fields.last_trace_id == 0 =>
        {
            Ok(())
        }
        _ => Err(invalid_range("causal_identity", fields)),
    }
}

fn validate_causal_header_range(
    field: &'static str,
    first: u64,
    last: u64,
) -> Result<(), PersistenceError> {
    if first > last {
        return Err(PersistenceError::InvalidHistoryRange { field, first, last });
    }
    Ok(())
}

fn validate_order(records: &[HistoryRecord]) -> Result<(), PersistenceError> {
    for index in 1..records.len() {
        if records[index - 1] >= records[index] {
            return Err(HistoryError::NonCanonicalOrder { index }.into());
        }
    }
    Ok(())
}

fn invalid_range(field: &'static str, fields: &HistoryFrameHeaderFields) -> PersistenceError {
    PersistenceError::InvalidHistoryRange {
        field,
        first: fields.first_record_sequence,
        last: fields.last_record_sequence,
    }
}

fn verify_payload(record: &DomainEvidenceRecord) -> Result<(), PersistenceError> {
    if blake3::hash(record.canonical_payload()).as_bytes() != &record.payload_digest().bytes() {
        return Err(PersistenceError::EvidencePayloadDigestMismatch {
            schema_id: record.schema_id().raw(),
        });
    }
    Ok(())
}

fn record_prefix(bytes: &mut Vec<u8>, tag: u8) {
    bytes.push(tag);
    push_u16(bytes, RECORD_MAJOR);
    push_u16(bytes, RECORD_MINOR);
}

fn write_slice(bytes: &mut Vec<u8>, value: &[u8]) -> Result<(), PersistenceError> {
    push_u64(bytes, to_u64(value.len())?);
    bytes.extend_from_slice(value);
    Ok(())
}

fn checked_count(value: u64, max: usize) -> Result<usize, PersistenceError> {
    if value > max as u64 {
        return Err(PersistenceError::HistoryFrameSizeExceeded { size: value, max });
    }
    usize::try_from(value).map_err(|_| PersistenceError::HistoryArithmeticOverflow)
}

fn checked_length(value: u64, offset: usize, remaining: usize) -> Result<usize, PersistenceError> {
    let length = usize::try_from(value).map_err(|_| length_error(offset, value, remaining))?;
    if length > remaining {
        return Err(length_error(offset, value, remaining));
    }
    Ok(length)
}

fn length_error(offset: usize, declared: u64, remaining: usize) -> PersistenceError {
    PersistenceError::MalformedHistoryLength {
        offset,
        declared,
        remaining,
    }
}

fn to_u64(value: usize) -> Result<u64, PersistenceError> {
    u64::try_from(value).map_err(|_| PersistenceError::HistoryArithmeticOverflow)
}

fn digest(domain: &[u8], parts: &[&[u8]]) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new();
    hasher.update(domain);
    for part in parts {
        hasher.update(part);
    }
    *hasher.finalize().as_bytes()
}

fn push_u16(bytes: &mut Vec<u8>, value: u16) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn push_u64(bytes: &mut Vec<u8>, value: u64) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

struct Cursor<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> Cursor<'a> {
    const fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    const fn position(&self) -> usize {
        self.offset
    }

    const fn remaining(&self) -> usize {
        self.bytes.len().saturating_sub(self.offset)
    }

    const fn is_empty(&self) -> bool {
        self.remaining() == 0
    }

    fn u8(&mut self) -> Result<u8, PersistenceError> {
        Ok(self.bytes(1)?[0])
    }

    fn u16(&mut self) -> Result<u16, PersistenceError> {
        Ok(u16::from_le_bytes(*self.fixed::<2>()?))
    }

    fn u64(&mut self) -> Result<u64, PersistenceError> {
        Ok(u64::from_le_bytes(*self.fixed::<8>()?))
    }

    fn fixed<const N: usize>(&mut self) -> Result<&'a [u8; N], PersistenceError> {
        let start = self.offset;
        self.bytes(N)?
            .try_into()
            .map_err(|_| length_error(start, N as u64, self.remaining()))
    }

    fn bytes(&mut self, length: usize) -> Result<&'a [u8], PersistenceError> {
        let start = self.offset;
        let end = start
            .checked_add(length)
            .ok_or(PersistenceError::HistoryArithmeticOverflow)?;
        if end > self.bytes.len() {
            return Err(length_error(start, length as u64, self.remaining()));
        }
        self.offset = end;
        Ok(&self.bytes[start..end])
    }

    fn length_delimited(&mut self) -> Result<&'a [u8], PersistenceError> {
        let offset = self.position();
        let raw = self.u64()?;
        let length = checked_length(raw, offset, self.remaining())?;
        self.bytes(length)
    }
}
