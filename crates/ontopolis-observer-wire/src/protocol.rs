use std::collections::BTreeMap;

use ontopolis_explanation::{
    ClaimEvidenceState, ComparisonContext, ExplanationReport, FrameAssessment, NumericClaimValue,
};
use ontopolis_observer_api::{
    OBSERVER_PROTOCOL_V1, ObserverChunkSummary, ObserverQuery, ObserverResponse, ObserverSnapshot,
    ObserverWorldSnapshot, QueryKind, QueryStatus, StreamEnvelope, StreamKind,
};
use ontopolis_types::{ChunkId, SimulationTime, TraceId};
use thiserror::Error;

const WIRE_VARINT: u8 = 0;
const WIRE_LEN: u8 = 2;
const WIRE_FIXED64: u8 = 1;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConnectRequest {
    pub supported_versions: Vec<u32>,
    pub locale: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConnectResponse {
    pub selected_version: u32,
    pub current_time: SimulationTime,
    pub capabilities: Vec<u32>,
}

#[derive(Clone, Debug, Default)]
pub struct ProtocolHandler {
    payloads: BTreeMap<QueryKind, Vec<u8>>,
    current_time: SimulationTime,
}

impl ProtocolHandler {
    pub fn new(current_time: SimulationTime) -> Self {
        Self {
            payloads: BTreeMap::new(),
            current_time,
        }
    }

    pub fn set_runtime_snapshot(&mut self, snapshot: &ObserverSnapshot) {
        self.current_time = snapshot.time;
        self.payloads.insert(
            QueryKind::RuntimeSummary,
            encode_observer_snapshot(snapshot),
        );
    }

    pub fn set_explanation_payload(&mut self, payload: Vec<u8>) {
        self.payloads.insert(QueryKind::ExplanationIr, payload);
    }

    pub fn set_explanation_report(&mut self, report: &ExplanationReport) {
        self.set_explanation_payload(encode_explanation_report(report));
    }

    pub fn set_world_snapshot(&mut self, snapshot: &ObserverWorldSnapshot) {
        self.payloads
            .insert(QueryKind::WorldChunks, encode_world_snapshot(snapshot));
    }

    pub fn negotiate(&self, request: &ConnectRequest) -> Result<ConnectResponse, WireError> {
        if !request.supported_versions.contains(&OBSERVER_PROTOCOL_V1) {
            return Err(WireError::NoCompatibleProtocolVersion);
        }
        Ok(ConnectResponse {
            selected_version: OBSERVER_PROTOCOL_V1,
            current_time: self.current_time,
            capabilities: vec![
                QueryKind::RuntimeSummary as u32,
                QueryKind::ExplanationIr as u32,
                QueryKind::WorldChunks as u32,
            ],
        })
    }

    pub fn handle_query(&self, query_bytes: &[u8]) -> Result<Vec<u8>, WireError> {
        let query = decode_query(query_bytes)?;
        let response = match query.validate() {
            Ok(()) => match self.payloads.get(&query.kind) {
                Some(payload) => ObserverResponse {
                    request_id: query.request_id,
                    protocol_version: OBSERVER_PROTOCOL_V1,
                    status: QueryStatus::Ok,
                    payload: payload.clone(),
                },
                None => ObserverResponse {
                    request_id: query.request_id,
                    protocol_version: OBSERVER_PROTOCOL_V1,
                    status: QueryStatus::NotAvailable,
                    payload: Vec::new(),
                },
            },
            Err(_) => ObserverResponse {
                request_id: query.request_id,
                protocol_version: OBSERVER_PROTOCOL_V1,
                status: QueryStatus::InvalidRequest,
                payload: Vec::new(),
            },
        };
        Ok(encode_response(&response))
    }
}

pub fn encode_query(query: &ObserverQuery) -> Vec<u8> {
    let mut out = Vec::new();
    field_varint(&mut out, 1, query.request_id);
    field_varint(&mut out, 2, u64::from(query.protocol_version));
    field_varint(&mut out, 3, query.kind as u64);
    if let Some(scope) = query.scope {
        field_varint(&mut out, 4, scope.raw());
    }
    if !query.payload.is_empty() {
        field_bytes(&mut out, 5, &query.payload);
    }
    out
}

pub fn decode_query(bytes: &[u8]) -> Result<ObserverQuery, WireError> {
    let mut cursor = Cursor::new(bytes);
    let mut request_id = None;
    let mut protocol_version = None;
    let mut kind = None;
    let mut scope = None;
    let mut payload = Vec::new();
    while !cursor.is_empty() {
        let (field, wire) = cursor.key()?;
        match (field, wire) {
            (1, WIRE_VARINT) => request_id = Some(cursor.varint()?),
            (2, WIRE_VARINT) => protocol_version = Some(to_u32(cursor.varint()?)?),
            (3, WIRE_VARINT) => kind = Some(QueryKind::try_from(to_u32(cursor.varint()?)?)?),
            (4, WIRE_VARINT) => scope = Some(ChunkId::new(cursor.varint()?)),
            (5, WIRE_LEN) => payload = cursor.bytes()?.to_vec(),
            _ => cursor.skip(wire)?,
        }
    }
    Ok(ObserverQuery {
        request_id: request_id.ok_or(WireError::MissingField(1))?,
        protocol_version: protocol_version.ok_or(WireError::MissingField(2))?,
        kind: kind.ok_or(WireError::MissingField(3))?,
        scope,
        payload,
    })
}

pub fn encode_response(response: &ObserverResponse) -> Vec<u8> {
    let mut out = Vec::new();
    field_varint(&mut out, 1, response.request_id);
    field_varint(&mut out, 2, u64::from(response.protocol_version));
    field_varint(&mut out, 3, response.status as u64);
    if !response.payload.is_empty() {
        field_bytes(&mut out, 4, &response.payload);
    }
    out
}

pub fn decode_response(bytes: &[u8]) -> Result<ObserverResponse, WireError> {
    let mut cursor = Cursor::new(bytes);
    let mut request_id = None;
    let mut protocol_version = None;
    let mut status = None;
    let mut payload = Vec::new();
    while !cursor.is_empty() {
        let (field, wire) = cursor.key()?;
        match (field, wire) {
            (1, WIRE_VARINT) => request_id = Some(cursor.varint()?),
            (2, WIRE_VARINT) => protocol_version = Some(to_u32(cursor.varint()?)?),
            (3, WIRE_VARINT) => {
                status = Some(match cursor.varint()? {
                    1 => QueryStatus::Ok,
                    2 => QueryStatus::InvalidRequest,
                    3 => QueryStatus::Unsupported,
                    4 => QueryStatus::NotAvailable,
                    value => return Err(WireError::UnknownStatus(value)),
                })
            }
            (4, WIRE_LEN) => payload = cursor.bytes()?.to_vec(),
            _ => cursor.skip(wire)?,
        }
    }
    Ok(ObserverResponse {
        request_id: request_id.ok_or(WireError::MissingField(1))?,
        protocol_version: protocol_version.ok_or(WireError::MissingField(2))?,
        status: status.ok_or(WireError::MissingField(3))?,
        payload,
    })
}

pub fn encode_connect_request(request: &ConnectRequest) -> Vec<u8> {
    let mut out = Vec::new();
    for version in &request.supported_versions {
        field_varint(&mut out, 1, u64::from(*version));
    }
    if !request.locale.is_empty() {
        field_bytes(&mut out, 2, request.locale.as_bytes());
    }
    out
}

pub fn decode_connect_request(bytes: &[u8]) -> Result<ConnectRequest, WireError> {
    let mut cursor = Cursor::new(bytes);
    let mut supported_versions = Vec::new();
    let mut locale = String::new();
    while !cursor.is_empty() {
        let (field, wire) = cursor.key()?;
        match (field, wire) {
            (1, WIRE_VARINT) => supported_versions.push(to_u32(cursor.varint()?)?),
            (2, WIRE_LEN) => {
                locale = std::str::from_utf8(cursor.bytes()?)
                    .map_err(|_| WireError::InvalidUtf8)?
                    .to_owned();
            }
            _ => cursor.skip(wire)?,
        }
    }
    Ok(ConnectRequest {
        supported_versions,
        locale,
    })
}

pub fn encode_connect_response(response: &ConnectResponse) -> Vec<u8> {
    let mut out = Vec::new();
    field_varint(&mut out, 1, u64::from(response.selected_version));
    let mut time = Vec::new();
    field_varint(&mut time, 1, response.current_time.raw());
    field_bytes(&mut out, 2, &time);
    for capability in &response.capabilities {
        field_varint(&mut out, 3, u64::from(*capability));
    }
    out
}

pub fn decode_connect_response(bytes: &[u8]) -> Result<ConnectResponse, WireError> {
    let mut cursor = Cursor::new(bytes);
    let mut selected_version = None;
    let mut current_time = None;
    let mut capabilities = Vec::new();
    while !cursor.is_empty() {
        let (field, wire) = cursor.key()?;
        match (field, wire) {
            (1, WIRE_VARINT) => selected_version = Some(to_u32(cursor.varint()?)?),
            (2, WIRE_LEN) => current_time = Some(decode_time(cursor.bytes()?)?),
            (3, WIRE_VARINT) => capabilities.push(to_u32(cursor.varint()?)?),
            _ => cursor.skip(wire)?,
        }
    }
    Ok(ConnectResponse {
        selected_version: selected_version.ok_or(WireError::MissingField(1))?,
        current_time: current_time.ok_or(WireError::MissingField(2))?,
        capabilities,
    })
}

pub fn encode_observer_snapshot(snapshot: &ObserverSnapshot) -> Vec<u8> {
    let mut out = Vec::with_capacity(256);
    field_varint(&mut out, 1, snapshot.time.raw());
    field_varint(&mut out, 2, u64::from(snapshot.digest_schema_version));
    field_bytes(&mut out, 3, &snapshot.physical_digest);
    field_bytes(&mut out, 4, &snapshot.history_digest);
    field_varint(&mut out, 5, snapshot.mana_total as u64);
    field_varint(&mut out, 6, snapshot.mana_maximum as u64);
    field_varint(&mut out, 7, u64::from(snapshot.active_chunk_count));
    field_varint(&mut out, 8, snapshot.resolution_relevance as u64);
    field_varint(&mut out, 9, u64::from(snapshot.resolution_level));
    field_varint(&mut out, 10, snapshot.causal_trace_count);
    field_varint(&mut out, 11, u64::from(snapshot.actor_count));
    field_varint(&mut out, 12, snapshot.population_total);
    for (field, value) in [
        (13, snapshot.physical_events),
        (14, snapshot.mana_cell_changes),
        (15, snapshot.mana_physical_effects),
        (16, snapshot.resolution_transitions),
        (17, snapshot.actor_actions_committed),
        (18, snapshot.actor_actions_rejected),
        (19, snapshot.population_births),
        (20, snapshot.population_deaths),
        (21, snapshot.population_movements),
        (22, snapshot.bytes_per_chunk),
        (23, snapshot.latest_trace.raw()),
    ] {
        field_varint(&mut out, field, value);
    }
    out
}

pub fn decode_observer_snapshot(bytes: &[u8]) -> Result<ObserverSnapshot, WireError> {
    let mut c = Cursor::new(bytes);
    let mut values = [0_u64; 23];
    let mut present = [false; 23];
    let mut physical_digest = None;
    let mut history_digest = None;
    while !c.is_empty() {
        let (field, wire) = c.key()?;
        match (field, wire) {
            (3, WIRE_LEN) => physical_digest = Some(array32(c.bytes()?)?),
            (4, WIRE_LEN) => history_digest = Some(array32(c.bytes()?)?),
            (1..=23, WIRE_VARINT) if field != 3 && field != 4 => {
                values[field as usize - 1] = c.varint()?;
                present[field as usize - 1] = true;
            }
            _ => c.skip(wire)?,
        }
    }
    for field in [1_usize, 2, 5, 6, 7, 8, 9, 10, 11, 12, 23] {
        if !present[field - 1] {
            return Err(WireError::MissingField(field as u32));
        }
    }
    Ok(ObserverSnapshot {
        time: SimulationTime::new(values[0]),
        digest_schema_version: to_u32(values[1])?,
        physical_digest: physical_digest.ok_or(WireError::MissingField(3))?,
        history_digest: history_digest.ok_or(WireError::MissingField(4))?,
        mana_total: values[4] as i64,
        mana_maximum: values[5] as i64,
        active_chunk_count: to_u32(values[6])?,
        resolution_relevance: values[7] as i64,
        resolution_level: to_u32(values[8])?,
        causal_trace_count: values[9],
        actor_count: to_u32(values[10])?,
        population_total: values[11],
        physical_events: values[12],
        mana_cell_changes: values[13],
        mana_physical_effects: values[14],
        resolution_transitions: values[15],
        actor_actions_committed: values[16],
        actor_actions_rejected: values[17],
        population_births: values[18],
        population_deaths: values[19],
        population_movements: values[20],
        bytes_per_chunk: values[21],
        latest_trace: TraceId::new(values[22]),
    })
}

pub fn encode_world_snapshot(snapshot: &ObserverWorldSnapshot) -> Vec<u8> {
    let mut out = Vec::new();
    field_varint(&mut out, 1, snapshot.time.raw());
    for chunk in &snapshot.chunks {
        let mut nested = Vec::new();
        field_varint(&mut nested, 1, chunk.chart_id);
        field_varint(&mut nested, 2, zigzag(i64::from(chunk.chunk_x)));
        field_varint(&mut nested, 3, zigzag(i64::from(chunk.chunk_y)));
        field_varint(&mut nested, 4, zigzag(i64::from(chunk.chunk_z)));
        field_varint(
            &mut nested,
            5,
            zigzag(i64::from(chunk.minimum_elevation_mm)),
        );
        field_varint(
            &mut nested,
            6,
            zigzag(i64::from(chunk.maximum_elevation_mm)),
        );
        field_varint(&mut nested, 7, u64::from(chunk.mean_roughness_mm));
        field_varint(&mut nested, 8, zigzag(chunk.mana_total));
        field_varint(&mut nested, 9, zigzag(chunk.resolution_relevance));
        field_varint(&mut nested, 10, u64::from(chunk.resolution_level));
        field_varint(&mut nested, 11, chunk.population_total);
        field_varint(&mut nested, 12, chunk.causal_event_count);
        field_varint(&mut nested, 13, chunk.latest_trace.raw());
        field_bytes(&mut out, 2, &nested);
    }
    out
}

pub fn decode_world_snapshot(bytes: &[u8]) -> Result<ObserverWorldSnapshot, WireError> {
    let mut cursor = Cursor::new(bytes);
    let mut time = None;
    let mut chunks = Vec::new();
    while !cursor.is_empty() {
        let (field, wire) = cursor.key()?;
        match (field, wire) {
            (1, WIRE_VARINT) => time = Some(SimulationTime::new(cursor.varint()?)),
            (2, WIRE_LEN) => chunks.push(decode_chunk_summary(cursor.bytes()?)?),
            _ => cursor.skip(wire)?,
        }
    }
    Ok(ObserverWorldSnapshot {
        time: time.ok_or(WireError::MissingField(1))?,
        chunks,
    })
}

fn decode_chunk_summary(bytes: &[u8]) -> Result<ObserverChunkSummary, WireError> {
    let mut cursor = Cursor::new(bytes);
    let mut values = [None; 13];
    while !cursor.is_empty() {
        let (field, wire) = cursor.key()?;
        if (1..=13).contains(&field) && wire == WIRE_VARINT {
            values[field as usize - 1] = Some(cursor.varint()?);
        } else {
            cursor.skip(wire)?;
        }
    }
    let value = |field: usize| values[field - 1].ok_or(WireError::MissingField(field as u32));
    Ok(ObserverChunkSummary {
        chart_id: value(1)?,
        chunk_x: to_i32(unzigzag(value(2)?))?,
        chunk_y: to_i32(unzigzag(value(3)?))?,
        chunk_z: to_i32(unzigzag(value(4)?))?,
        minimum_elevation_mm: to_i32(unzigzag(value(5)?))?,
        maximum_elevation_mm: to_i32(unzigzag(value(6)?))?,
        mean_roughness_mm: to_u32(value(7)?)?,
        mana_total: unzigzag(value(8)?),
        resolution_relevance: unzigzag(value(9)?),
        resolution_level: to_u32(value(10)?)?,
        population_total: value(11)?,
        causal_event_count: value(12)?,
        latest_trace: TraceId::new(value(13)?),
    })
}

pub fn encode_stream_envelope(envelope: &StreamEnvelope) -> Vec<u8> {
    let mut header = Vec::new();
    field_varint(&mut header, 1, envelope.header.stream_id);
    field_varint(&mut header, 2, u64::from(envelope.header.schema_version));
    field_varint(&mut header, 3, envelope.header.sequence_number);
    let mut time = Vec::new();
    field_varint(&mut time, 1, envelope.header.simulation_time.raw());
    field_bytes(&mut header, 4, &time);
    for (field, digest) in [
        (5, envelope.header.physical_digest),
        (6, envelope.header.history_digest),
    ] {
        let mut nested = Vec::new();
        field_varint(&mut nested, 1, 1);
        field_bytes(&mut nested, 2, &digest);
        field_bytes(&mut header, field, &nested);
    }
    field_varint(&mut header, 7, u64::from(envelope.header.is_snapshot));

    let mut out = Vec::new();
    field_bytes(&mut out, 1, &header);
    field_varint(&mut out, 2, envelope.scope.kind as u64);
    if let Some(chunk) = envelope.scope.chunk {
        let mut scope = Vec::new();
        field_varint(&mut scope, 2, chunk.raw());
        field_bytes(&mut out, 3, &scope);
    }
    field_bytes(&mut out, 4, &envelope.payload);
    out
}

pub fn decode_stream_envelope(bytes: &[u8]) -> Result<StreamEnvelope, WireError> {
    let mut c = Cursor::new(bytes);
    let mut header = None;
    let mut kind = None;
    let mut chunk = None;
    let mut payload = Vec::new();
    while !c.is_empty() {
        let (field, wire) = c.key()?;
        match (field, wire) {
            (1, WIRE_LEN) => header = Some(decode_stream_header(c.bytes()?)?),
            (2, WIRE_VARINT) => kind = Some(decode_stream_kind(c.varint()?)?),
            (3, WIRE_LEN) => chunk = decode_chunk_scope(c.bytes()?)?,
            (4, WIRE_LEN) => payload = c.bytes()?.to_vec(),
            _ => c.skip(wire)?,
        }
    }
    let header = header.ok_or(WireError::MissingField(1))?;
    let kind = kind.ok_or(WireError::MissingField(2))?;
    Ok(StreamEnvelope {
        header,
        scope: ontopolis_observer_api::StreamScope { kind, chunk },
        payload,
    })
}

fn decode_stream_header(bytes: &[u8]) -> Result<ontopolis_observer_api::StreamHeader, WireError> {
    let mut c = Cursor::new(bytes);
    let (mut id, mut version, mut sequence, mut time) = (None, None, None, None);
    let (mut physical, mut history, mut snapshot) = (None, None, false);
    while !c.is_empty() {
        let (field, wire) = c.key()?;
        match (field, wire) {
            (1, WIRE_VARINT) => id = Some(c.varint()?),
            (2, WIRE_VARINT) => version = Some(to_u32(c.varint()?)?),
            (3, WIRE_VARINT) => sequence = Some(c.varint()?),
            (4, WIRE_LEN) => time = Some(decode_time(c.bytes()?)?),
            (5, WIRE_LEN) => physical = Some(decode_digest(c.bytes()?)?),
            (6, WIRE_LEN) => history = Some(decode_digest(c.bytes()?)?),
            (7, WIRE_VARINT) => snapshot = c.varint()? != 0,
            _ => c.skip(wire)?,
        }
    }
    Ok(ontopolis_observer_api::StreamHeader {
        stream_id: id.ok_or(WireError::MissingField(1))?,
        schema_version: version.ok_or(WireError::MissingField(2))?,
        sequence_number: sequence.ok_or(WireError::MissingField(3))?,
        simulation_time: time.ok_or(WireError::MissingField(4))?,
        physical_digest: physical.ok_or(WireError::MissingField(5))?,
        history_digest: history.ok_or(WireError::MissingField(6))?,
        is_snapshot: snapshot,
    })
}

fn decode_time(bytes: &[u8]) -> Result<SimulationTime, WireError> {
    let mut c = Cursor::new(bytes);
    while !c.is_empty() {
        let (field, wire) = c.key()?;
        if (field, wire) == (1, WIRE_VARINT) {
            return Ok(SimulationTime::new(c.varint()?));
        }
        c.skip(wire)?;
    }
    Err(WireError::MissingField(1))
}

fn decode_digest(bytes: &[u8]) -> Result<[u8; 32], WireError> {
    let mut c = Cursor::new(bytes);
    while !c.is_empty() {
        let (field, wire) = c.key()?;
        if (field, wire) == (2, WIRE_LEN) {
            return array32(c.bytes()?);
        }
        c.skip(wire)?;
    }
    Err(WireError::MissingField(2))
}

fn decode_chunk_scope(bytes: &[u8]) -> Result<Option<ChunkId>, WireError> {
    let mut c = Cursor::new(bytes);
    while !c.is_empty() {
        let (field, wire) = c.key()?;
        if (field, wire) == (2, WIRE_VARINT) {
            return Ok(Some(ChunkId::new(c.varint()?)));
        }
        c.skip(wire)?;
    }
    Ok(None)
}

fn decode_stream_kind(value: u64) -> Result<StreamKind, WireError> {
    match value {
        1 => Ok(StreamKind::RuntimeSummary),
        2 => Ok(StreamKind::Explanation),
        3 => Ok(StreamKind::Metrics),
        value => Err(WireError::UnknownStreamKind(value)),
    }
}

pub fn encode_explanation_report(report: &ExplanationReport) -> Vec<u8> {
    let mut out = Vec::new();
    field_varint(&mut out, 1, report.experiment.raw());
    for frame in &report.frames {
        let mut frame_bytes = Vec::new();
        field_varint(&mut frame_bytes, 1, frame.checkpoint_time.raw());
        for claim in &frame.claims {
            let mut claim_bytes = Vec::new();
            field_varint(&mut claim_bytes, 1, claim.schema.raw());
            field_bytes(&mut claim_bytes, 2, &encode_numeric_value(claim.value));
            field_fixed64(&mut claim_bytes, 3, claim.confidence.raw().to_bits());
            for trace in &claim.evidence_traces {
                field_varint(&mut claim_bytes, 4, trace.raw());
            }
            let mut comparison = Vec::new();
            match claim.comparison {
                ComparisonContext::None => field_varint(&mut comparison, 1, 0),
                ComparisonContext::MatchedCohort { cohort } => {
                    field_varint(&mut comparison, 1, 1);
                    field_varint(&mut comparison, 2, cohort.raw());
                }
                ComparisonContext::Counterfactual { cohort } => {
                    field_varint(&mut comparison, 1, 2);
                    field_varint(&mut comparison, 2, cohort.raw());
                }
            }
            field_bytes(&mut claim_bytes, 5, &comparison);
            field_varint(&mut claim_bytes, 6, evidence_code(claim.evidence_state));
            field_bytes(&mut frame_bytes, 2, &claim_bytes);
        }
        field_varint(
            &mut frame_bytes,
            3,
            assessment_code(frame.overall_assessment),
        );
        field_bytes(&mut out, 2, &frame_bytes);
    }
    field_varint(&mut out, 3, assessment_code(report.overall_assessment));
    out
}

fn encode_numeric_value(value: NumericClaimValue) -> Vec<u8> {
    let mut out = Vec::new();
    match value {
        NumericClaimValue::Scalar { value } => field_varint(&mut out, 1, zigzag(value)),
        NumericClaimValue::Range { start, end } => {
            let mut range = Vec::new();
            field_varint(&mut range, 1, zigzag(start));
            field_varint(&mut range, 2, zigzag(end));
            field_bytes(&mut out, 2, &range);
        }
        NumericClaimValue::Ratio {
            numerator,
            denominator,
        } => {
            let mut ratio = Vec::new();
            field_varint(&mut ratio, 1, numerator);
            field_varint(&mut ratio, 2, denominator);
            field_bytes(&mut out, 3, &ratio);
        }
    }
    out
}

fn evidence_code(value: ClaimEvidenceState) -> u64 {
    match value {
        ClaimEvidenceState::Supported => 1,
        ClaimEvidenceState::Unsupported => 2,
        ClaimEvidenceState::Unknown => 3,
    }
}
fn assessment_code(value: FrameAssessment) -> u64 {
    match value {
        FrameAssessment::Supported => 1,
        FrameAssessment::Partial => 2,
        FrameAssessment::Unsupported => 3,
        FrameAssessment::Unknown => 4,
    }
}
fn zigzag(value: i64) -> u64 {
    ((value << 1) ^ (value >> 63)) as u64
}

fn field_varint(out: &mut Vec<u8>, field: u32, value: u64) {
    varint(out, u64::from(field) << 3);
    varint(out, value);
}

fn field_bytes(out: &mut Vec<u8>, field: u32, value: &[u8]) {
    varint(out, (u64::from(field) << 3) | u64::from(WIRE_LEN));
    varint(out, value.len() as u64);
    out.extend_from_slice(value);
}

fn field_fixed64(out: &mut Vec<u8>, field: u32, value: u64) {
    varint(out, (u64::from(field) << 3) | u64::from(WIRE_FIXED64));
    out.extend_from_slice(&value.to_le_bytes());
}

fn varint(out: &mut Vec<u8>, mut value: u64) {
    while value >= 0x80 {
        out.push((value as u8 & 0x7f) | 0x80);
        value >>= 7;
    }
    out.push(value as u8);
}

fn to_u32(value: u64) -> Result<u32, WireError> {
    u32::try_from(value).map_err(|_| WireError::IntegerOverflow)
}

fn to_i32(value: i64) -> Result<i32, WireError> {
    i32::try_from(value).map_err(|_| WireError::IntegerOverflow)
}

fn unzigzag(value: u64) -> i64 {
    ((value >> 1) as i64) ^ (-((value & 1) as i64))
}

fn array32(bytes: &[u8]) -> Result<[u8; 32], WireError> {
    bytes
        .try_into()
        .map_err(|_| WireError::InvalidDigestLength(bytes.len()))
}

struct Cursor<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl<'a> Cursor<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, at: 0 }
    }
    fn is_empty(&self) -> bool {
        self.at == self.bytes.len()
    }
    fn varint(&mut self) -> Result<u64, WireError> {
        let mut value = 0_u64;
        for shift in (0..=63).step_by(7) {
            let byte = *self.bytes.get(self.at).ok_or(WireError::UnexpectedEof)?;
            self.at += 1;
            value |= u64::from(byte & 0x7f) << shift;
            if byte & 0x80 == 0 {
                return Ok(value);
            }
        }
        Err(WireError::InvalidVarint)
    }
    fn key(&mut self) -> Result<(u32, u8), WireError> {
        let key = self.varint()?;
        let field = to_u32(key >> 3)?;
        if field == 0 {
            return Err(WireError::InvalidFieldNumber);
        }
        Ok((field, (key & 7) as u8))
    }
    fn bytes(&mut self) -> Result<&'a [u8], WireError> {
        let len = usize::try_from(self.varint()?).map_err(|_| WireError::IntegerOverflow)?;
        let end = self.at.checked_add(len).ok_or(WireError::IntegerOverflow)?;
        let result = self
            .bytes
            .get(self.at..end)
            .ok_or(WireError::UnexpectedEof)?;
        self.at = end;
        Ok(result)
    }
    fn skip(&mut self, wire: u8) -> Result<(), WireError> {
        match wire {
            WIRE_VARINT => {
                self.varint()?;
            }
            WIRE_LEN => {
                self.bytes()?;
            }
            WIRE_FIXED64 => {
                self.at = self.at.checked_add(8).ok_or(WireError::IntegerOverflow)?;
                if self.at > self.bytes.len() {
                    return Err(WireError::UnexpectedEof);
                }
            }
            value => return Err(WireError::UnsupportedWireType(value)),
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum WireError {
    #[error("unexpected end of protobuf input")]
    UnexpectedEof,
    #[error("invalid protobuf varint")]
    InvalidVarint,
    #[error("invalid protobuf field number")]
    InvalidFieldNumber,
    #[error("unsupported protobuf wire type {0}")]
    UnsupportedWireType(u8),
    #[error("required protobuf field {0} is missing")]
    MissingField(u32),
    #[error("protobuf integer overflow")]
    IntegerOverflow,
    #[error("unknown query response status {0}")]
    UnknownStatus(u64),
    #[error("unknown observer stream kind {0}")]
    UnknownStreamKind(u64),
    #[error("digest must contain 32 bytes, got {0}")]
    InvalidDigestLength(usize),
    #[error("no compatible observer protocol version")]
    NoCompatibleProtocolVersion,
    #[error("protobuf string is not valid UTF-8")]
    InvalidUtf8,
    #[error(transparent)]
    Api(#[from] ontopolis_observer_api::ObserverApiError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use ontopolis_explanation::{
        ClaimConfidence, ClaimEvidenceState, ComparisonContext, ExplanationClaim,
        ExplanationClaimSchemaId, ExplanationFrame, NumericClaimValue,
    };
    use ontopolis_observer_api::{DeliveryPolicy, ObserverStreamHub, StreamScope};
    use ontopolis_types::ExperimentId;

    fn snapshot() -> ObserverSnapshot {
        ObserverSnapshot {
            time: SimulationTime::new(44),
            digest_schema_version: 1,
            physical_digest: [3; 32],
            history_digest: [4; 32],
            mana_total: -4,
            mana_maximum: 8,
            active_chunk_count: 2,
            resolution_relevance: -9,
            resolution_level: 3,
            causal_trace_count: 77,
            actor_count: 5,
            population_total: 91,
            physical_events: 6,
            mana_cell_changes: 7,
            mana_physical_effects: 8,
            resolution_transitions: 9,
            actor_actions_committed: 10,
            actor_actions_rejected: 11,
            population_births: 12,
            population_deaths: 13,
            population_movements: 14,
            bytes_per_chunk: 15,
            latest_trace: TraceId::new(16),
        }
    }

    #[test]
    fn query_response_roundtrip_returns_derived_snapshot() {
        let expected = snapshot();
        let mut handler = ProtocolHandler::default();
        handler.set_runtime_snapshot(&expected);
        let request = ObserverQuery::runtime_summary(82);
        let bytes = handler.handle_query(&encode_query(&request)).unwrap();
        let response = decode_response(&bytes).unwrap();
        assert_eq!(response.request_id, 82);
        assert_eq!(response.status, QueryStatus::Ok);
        assert_eq!(
            decode_observer_snapshot(&response.payload).unwrap(),
            expected
        );
    }

    #[test]
    fn canonical_encoding_is_stable() {
        let query = ObserverQuery::runtime_summary(3);
        assert_eq!(encode_query(&query), encode_query(&query));
        let bytes = encode_observer_snapshot(&snapshot());
        assert_eq!(decode_observer_snapshot(&bytes).unwrap(), snapshot());
    }

    #[test]
    fn negotiation_rejects_incompatible_clients() {
        let handler = ProtocolHandler::default();
        assert_eq!(
            handler.negotiate(&ConnectRequest {
                supported_versions: vec![2],
                locale: "ru".into()
            }),
            Err(WireError::NoCompatibleProtocolVersion)
        );
    }

    #[test]
    fn negotiation_messages_roundtrip_through_protobuf() {
        let request = ConnectRequest {
            supported_versions: vec![1, 2],
            locale: "ru-RU".into(),
        };
        assert_eq!(
            decode_connect_request(&encode_connect_request(&request)).unwrap(),
            request
        );
        let response = ConnectResponse {
            selected_version: 1,
            current_time: SimulationTime::new(12),
            capabilities: vec![1, 2, 3],
        };
        assert_eq!(
            decode_connect_response(&encode_connect_response(&response)).unwrap(),
            response
        );
    }

    #[test]
    fn world_query_roundtrips_chart_qualified_chunks() {
        let expected = ObserverWorldSnapshot {
            time: SimulationTime::new(7),
            chunks: vec![ObserverChunkSummary {
                chart_id: 5,
                chunk_x: -1,
                chunk_y: 2,
                chunk_z: -3,
                minimum_elevation_mm: -400,
                maximum_elevation_mm: 2_100,
                mean_roughness_mm: 38,
                mana_total: -90,
                resolution_relevance: 120,
                resolution_level: 2,
                population_total: 44,
                causal_event_count: 11,
                latest_trace: TraceId::new(8),
            }],
        };
        let mut handler = ProtocolHandler::default();
        handler.set_world_snapshot(&expected);
        let response = decode_response(
            &handler
                .handle_query(&encode_query(&ObserverQuery::world_chunks(17)))
                .unwrap(),
        )
        .unwrap();
        assert_eq!(response.status, QueryStatus::Ok);
        assert_eq!(decode_world_snapshot(&response.payload).unwrap(), expected);
    }

    #[test]
    fn stream_envelope_roundtrips_through_protobuf() {
        let mut hub = ObserverStreamHub::default();
        hub.subscribe(
            5,
            StreamScope {
                kind: StreamKind::RuntimeSummary,
                chunk: None,
            },
            DeliveryPolicy::LatestStateWins,
            2,
        )
        .unwrap();
        hub.publish(
            5,
            SimulationTime::new(9),
            [7; 32],
            [8; 32],
            true,
            vec![1, 2, 3],
        )
        .unwrap();
        let envelope = hub.pop(5).unwrap().unwrap();
        let bytes = encode_stream_envelope(&envelope);
        assert_eq!(decode_stream_envelope(&bytes).unwrap(), envelope);
    }

    #[test]
    fn explanation_ir_is_a_typed_wire_payload() {
        let claim = ExplanationClaim::new(
            ExplanationClaimSchemaId::new(6),
            NumericClaimValue::scalar(-2),
            ClaimConfidence::new(0.5).unwrap(),
            vec![TraceId::new(3)],
            ComparisonContext::None,
            ClaimEvidenceState::Supported,
        )
        .unwrap();
        let report = ExplanationReport::new(
            ExperimentId::new(2),
            vec![ExplanationFrame::new(SimulationTime::new(4), vec![claim]).unwrap()],
        )
        .unwrap();
        let mut handler = ProtocolHandler::default();
        handler.set_explanation_report(&report);
        let query = ObserverQuery {
            request_id: 11,
            protocol_version: 1,
            kind: QueryKind::ExplanationIr,
            scope: None,
            payload: vec![],
        };
        let response =
            decode_response(&handler.handle_query(&encode_query(&query)).unwrap()).unwrap();
        assert_eq!(response.status, QueryStatus::Ok);
        assert_eq!(response.payload, encode_explanation_report(&report));
        assert!(!response.payload.is_empty());
    }
}
