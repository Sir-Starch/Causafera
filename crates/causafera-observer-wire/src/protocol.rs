use std::collections::BTreeMap;

use causafera_explanation::{
    ClaimConfidence, ClaimEvidenceState, ComparisonCohortId, ComparisonContext, ExplanationClaim,
    ExplanationClaimSchemaId, ExplanationFrame, ExplanationReport, FrameAssessment,
    NumericClaimValue,
};
use causafera_observer_api::{
    BOOTSTRAP_SUMMARY_SCHEMA_V1, FieldRasterKind, FieldRasterRequest,
    MATERIAL_SURFACE_DELTA_SCHEMA_V3, MATERIAL_SURFACE_DELTA_SCHEMA_V4,
    MAX_BOOTSTRAP_RECEIPT_DEPENDENCIES, MAX_BOOTSTRAP_RECEIPT_SUMMARIES,
    MAX_MATERIAL_SURFACE_DELTAS, MAX_THERMAL_DELTAS, MaterialSurfaceDelta,
    MaterialSurfaceGateDelta, MaterialSurfaceThermalDelta, OBSERVER_PROTOCOL_V1,
    ObserverBootstrapReceipt, ObserverBootstrapSummary, ObserverChunkSummary, ObserverFieldRaster,
    ObserverQuery, ObserverResponse, ObserverSnapshot, ObserverWorldSnapshot, QueryKind,
    QueryStatus, StreamEnvelope, StreamKind, THERMAL_DELTA_SCHEMA_V1, ThermalFieldDelta,
};
use causafera_types::{ChunkId, ExperimentId, SimulationTime, TraceId};
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
                QueryKind::FieldRaster as u32,
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

/// Answer one query that carries parameters, which the payload cache cannot
/// serve because its answer depends on what was asked rather than on when.
pub fn encode_query_response(request_id: u64, status: QueryStatus, payload: Vec<u8>) -> Vec<u8> {
    encode_response(&ObserverResponse {
        request_id,
        protocol_version: OBSERVER_PROTOCOL_V1,
        status,
        payload,
    })
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
    field_bytes(
        &mut out,
        24,
        &encode_i128_zigzag(snapshot.thermal_total_cell_energy),
    );
    field_bytes(
        &mut out,
        25,
        &encode_i128_zigzag(snapshot.thermal_total_reservoir_budget),
    );
    field_varint(&mut out, 26, u64::from(snapshot.thermal_active_chunk_count));
    field_varint(&mut out, 27, u64::from(snapshot.thermal_active_cell_count));
    // Fields 28 onwards are additive. Fields 1..=27 keep the meaning they had, so
    // a reader written against them decodes this payload unchanged and simply
    // skips what it does not know.
    let bootstrap = &snapshot.bootstrap;
    field_varint(&mut out, 28, u64::from(bootstrap.schema_version));
    field_varint(&mut out, 29, bootstrap.plan_id);
    field_varint(&mut out, 30, bootstrap.world_seed);
    field_varint(&mut out, 31, u64::from(bootstrap.stage_count));
    field_varint(&mut out, 32, u64::from(bootstrap.complete));
    field_varint(&mut out, 33, bootstrap.configured_population);
    field_varint(
        &mut out,
        34,
        u64::from(bootstrap.configured_promotion_limit),
    );
    for receipt in bootstrap
        .receipts
        .iter()
        .take(MAX_BOOTSTRAP_RECEIPT_SUMMARIES)
    {
        let mut nested = Vec::with_capacity(64);
        field_varint(&mut nested, 1, receipt.stage);
        field_varint(&mut nested, 2, receipt.completed_at.raw());
        field_bytes(&mut nested, 3, &receipt.result);
        field_varint(&mut nested, 4, receipt.completion_trace.raw());
        for dependency in receipt
            .dependency_traces
            .iter()
            .take(MAX_BOOTSTRAP_RECEIPT_DEPENDENCIES)
        {
            field_varint(&mut nested, 5, dependency.raw());
        }
        field_bytes(&mut out, 35, &nested);
    }
    out
}

/// Decode the bounded bootstrap summary as one atomic optional group.
///
/// Fields 28..=35 are additive, so a payload that carries none of them was
/// written before the summary existed and decodes to the explicit absent schema
/// — that is the only tolerated incompleteness. A payload that carries *part* of
/// the group is not an older peer, it is a contradiction, and every way of being
/// partially present fails closed here rather than being filled in with zeroes:
/// a schema field with no plan behind it, receipts with no schema to interpret
/// them, an unknown schema version, a stage count the receipts do not match, or
/// a completeness flag that disagrees with both.
fn decode_bootstrap_summary(
    values: &[u64; 34],
    present: &[bool; 34],
    receipts: Vec<ObserverBootstrapReceipt>,
) -> Result<ObserverBootstrapSummary, WireError> {
    const GROUP: std::ops::RangeInclusive<usize> = 28..=34;
    let declared = GROUP.clone().any(|field| present[field - 1]);
    if !declared {
        if !receipts.is_empty() {
            // Receipts with no summary to interpret them would otherwise be
            // parsed and then silently dropped.
            return Err(WireError::MissingField(28));
        }
        return Ok(ObserverBootstrapSummary::default());
    }
    for field in GROUP {
        if !present[field - 1] {
            return Err(WireError::MissingField(field as u32));
        }
    }

    let schema_version = to_u32(values[27])?;
    if schema_version != BOOTSTRAP_SUMMARY_SCHEMA_V1 {
        return Err(WireError::UnexpectedFieldForSchema(schema_version));
    }
    let stage_count = to_u32(values[30])?;
    if stage_count as usize > MAX_BOOTSTRAP_RECEIPT_SUMMARIES {
        return Err(WireError::PayloadTooLarge);
    }
    let complete = match values[31] {
        0 => false,
        1 => true,
        other => return Err(WireError::InvalidBoolean(other)),
    };
    if receipts
        .windows(2)
        .any(|pair| pair[0].stage >= pair[1].stage)
    {
        return Err(WireError::NonCanonicalOrder);
    }
    if receipts.len() != stage_count as usize {
        return Err(WireError::MissingField(35));
    }
    // A record is complete exactly when it closed every stage it declared. The
    // producer writes the two together; a payload where they disagree describes
    // no state this runtime can be in.
    if complete != (stage_count > 0) {
        return Err(WireError::UnexpectedFieldForSchema(schema_version));
    }
    Ok(ObserverBootstrapSummary {
        schema_version,
        plan_id: values[28],
        world_seed: values[29],
        stage_count,
        complete,
        configured_population: values[32],
        configured_promotion_limit: to_u32(values[33])?,
        receipts,
    })
}

fn decode_bootstrap_receipt(bytes: &[u8]) -> Result<ObserverBootstrapReceipt, WireError> {
    let mut c = Cursor::new(bytes);
    let mut values = [0_u64; 4];
    let mut present = [false; 4];
    let mut result = None;
    let mut dependency_traces = Vec::new();
    while !c.is_empty() {
        let (field, wire) = c.key()?;
        match (field, wire) {
            (3, WIRE_LEN) => {
                if result.is_some() {
                    return Err(WireError::DuplicateField(3));
                }
                result = Some(array32(c.bytes()?)?);
            }
            (5, WIRE_VARINT) => {
                if dependency_traces.len() == MAX_BOOTSTRAP_RECEIPT_DEPENDENCIES {
                    return Err(WireError::PayloadTooLarge);
                }
                dependency_traces.push(TraceId::new(c.varint()?));
            }
            // A receipt's scalars are single-valued, exactly like the summary's.
            // Two stages or two result fingerprints in one receipt is a
            // contradiction, not a later value winning.
            (1..=4, WIRE_VARINT) => {
                if present[field as usize - 1] {
                    return Err(WireError::DuplicateField(field));
                }
                values[field as usize - 1] = c.varint()?;
                present[field as usize - 1] = true;
            }
            // A known field arriving on the wrong wire type is a malformed
            // receipt, not an unknown field to skip past.
            (1..=5, _) => return Err(WireError::UnexpectedFieldForSchema(field)),
            _ => c.skip(wire)?,
        }
    }
    for field in [1_usize, 2, 4] {
        if !present[field - 1] {
            return Err(WireError::MissingField(field as u32));
        }
    }
    if dependency_traces.windows(2).any(|pair| pair[0] >= pair[1]) {
        return Err(WireError::NonCanonicalOrder);
    }
    Ok(ObserverBootstrapReceipt {
        stage: values[0],
        completed_at: SimulationTime::new(values[1]),
        result: result.ok_or(WireError::MissingField(3))?,
        completion_trace: TraceId::new(values[3]),
        dependency_traces,
    })
}

pub fn decode_observer_snapshot(bytes: &[u8]) -> Result<ObserverSnapshot, WireError> {
    let mut c = Cursor::new(bytes);
    let mut values = [0_u64; 34];
    let mut present = [false; 34];
    let mut physical_digest = None;
    let mut history_digest = None;
    let mut thermal_total_cell_energy = 0;
    let mut thermal_total_reservoir_budget = 0;
    let mut bootstrap_receipts = Vec::new();
    while !c.is_empty() {
        let (field, wire) = c.key()?;
        match (field, wire) {
            (3, WIRE_LEN) => physical_digest = Some(array32(c.bytes()?)?),
            (4, WIRE_LEN) => history_digest = Some(array32(c.bytes()?)?),
            (24, WIRE_LEN) => thermal_total_cell_energy = decode_i128_zigzag(c.bytes()?)?,
            (25, WIRE_LEN) => thermal_total_reservoir_budget = decode_i128_zigzag(c.bytes()?)?,
            (35, WIRE_LEN) => {
                // Bounded before allocation: a payload claiming more receipts
                // than this build's bootstrap can produce is rejected rather
                // than truncated.
                if bootstrap_receipts.len() == MAX_BOOTSTRAP_RECEIPT_SUMMARIES {
                    return Err(WireError::PayloadTooLarge);
                }
                bootstrap_receipts.push(decode_bootstrap_receipt(c.bytes()?)?);
            }
            (28..=34, WIRE_VARINT) => {
                // The summary's scalars are single-valued. Elsewhere in this
                // payload a repeated field silently wins with its last value,
                // which is a decision this group does not inherit: two different
                // stage counts in one payload is a contradiction, not an update.
                if present[field as usize - 1] {
                    return Err(WireError::DuplicateField(field));
                }
                values[field as usize - 1] = c.varint()?;
                present[field as usize - 1] = true;
            }
            (1..=34, WIRE_VARINT) => {
                values[field as usize - 1] = c.varint()?;
                present[field as usize - 1] = true;
            }
            // A bootstrap field arriving on the wrong wire type is malformed,
            // not unknown. Skipping it would let a summary whose every scalar is
            // mistyped fall through to the absent schema, reporting "this reader
            // predates the summary" about a payload that tried to carry one.
            (28..=35, _) => return Err(WireError::UnexpectedFieldForSchema(field)),
            _ => c.skip(wire)?,
        }
    }
    // Fields 13..=22 are written by every encoder here and required by the
    // TypeScript decoder. Defaulting them to zero on this side meant one decoder
    // accepted a payload the other refused, which is the divergence the wire
    // contract cannot afford whichever way it points.
    for field in [
        1_usize, 2, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23,
    ] {
        if !present[field - 1] {
            return Err(WireError::MissingField(field as u32));
        }
    }
    let bootstrap = decode_bootstrap_summary(&values, &present, bootstrap_receipts)?;
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
        thermal_total_cell_energy,
        thermal_total_reservoir_budget,
        thermal_active_chunk_count: to_u32(values[25])?,
        thermal_active_cell_count: to_u32(values[26])?,
        bootstrap,
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
    for delta in snapshot
        .material_surface_deltas
        .iter()
        .take(MAX_MATERIAL_SURFACE_DELTAS)
    {
        field_bytes(
            &mut out,
            3,
            &encode_material_surface_delta(delta, snapshot.material_surface_delta_schema_version),
        );
    }
    if snapshot.material_surface_delta_schema_version != 0 {
        field_varint(
            &mut out,
            4,
            u64::from(snapshot.material_surface_delta_schema_version),
        );
    }
    if snapshot.material_surface_delta_schema_version >= MATERIAL_SURFACE_DELTA_SCHEMA_V3 {
        for delta in snapshot
            .material_surface_gate_deltas
            .iter()
            .take(MAX_MATERIAL_SURFACE_DELTAS)
        {
            field_bytes(&mut out, 5, &encode_material_surface_gate_delta(delta));
        }
    }
    if snapshot.thermal_delta_schema_version >= THERMAL_DELTA_SCHEMA_V1 {
        for delta in snapshot.thermal_deltas.iter().take(MAX_THERMAL_DELTAS) {
            field_bytes(&mut out, 6, &encode_thermal_field_delta(delta));
        }
    }
    if snapshot.thermal_delta_schema_version != 0 {
        field_varint(
            &mut out,
            7,
            u64::from(snapshot.thermal_delta_schema_version),
        );
    }
    if snapshot.material_surface_delta_schema_version >= MATERIAL_SURFACE_DELTA_SCHEMA_V4 {
        for delta in snapshot
            .material_surface_thermal_deltas
            .iter()
            .take(MAX_MATERIAL_SURFACE_DELTAS)
        {
            field_bytes(&mut out, 8, &encode_material_surface_thermal_delta(delta));
        }
    }
    out
}

pub fn decode_world_snapshot(bytes: &[u8]) -> Result<ObserverWorldSnapshot, WireError> {
    let mut cursor = Cursor::new(bytes);
    let mut time = None;
    let mut chunks = Vec::new();
    let mut material_surface_delta_bytes = Vec::new();
    let mut material_surface_gate_deltas = Vec::new();
    let mut material_surface_thermal_deltas = Vec::new();
    let mut thermal_delta_bytes = Vec::new();
    let mut material_surface_delta_schema_version = 0;
    let mut schema_version_seen = false;
    let mut thermal_delta_schema_version = 0;
    let mut thermal_schema_version_seen = false;
    while !cursor.is_empty() {
        let (field, wire) = cursor.key()?;
        match (field, wire) {
            (1, WIRE_VARINT) => time = Some(SimulationTime::new(cursor.varint()?)),
            (2, WIRE_LEN) => chunks.push(decode_chunk_summary(cursor.bytes()?)?),
            (3, WIRE_LEN) if material_surface_delta_bytes.len() < MAX_MATERIAL_SURFACE_DELTAS => {
                material_surface_delta_bytes.push(cursor.bytes()?.to_vec())
            }
            (3, WIRE_LEN) => {
                cursor.bytes()?;
            }
            (4, WIRE_VARINT) if !schema_version_seen => {
                material_surface_delta_schema_version = to_u32(cursor.varint()?)?;
                schema_version_seen = true;
            }
            (4, _) => return Err(WireError::DuplicateField(4)),
            (5, WIRE_LEN)
                if material_surface_delta_schema_version >= MATERIAL_SURFACE_DELTA_SCHEMA_V3
                    && material_surface_gate_deltas.len() < MAX_MATERIAL_SURFACE_DELTAS =>
            {
                material_surface_gate_deltas
                    .push(decode_material_surface_gate_delta(cursor.bytes()?)?)
            }
            (5, WIRE_LEN)
                if material_surface_delta_schema_version >= MATERIAL_SURFACE_DELTA_SCHEMA_V3 =>
            {
                cursor.bytes()?;
            }
            (5, _) => return Err(WireError::UnexpectedFieldForSchema(5)),
            (6, WIRE_LEN) if thermal_delta_bytes.len() < MAX_THERMAL_DELTAS => {
                thermal_delta_bytes.push(cursor.bytes()?.to_vec())
            }
            (6, WIRE_LEN) => {
                cursor.bytes()?;
            }
            (6, _) => return Err(WireError::UnexpectedFieldForSchema(6)),
            (7, WIRE_VARINT) if !thermal_schema_version_seen => {
                thermal_delta_schema_version = to_u32(cursor.varint()?)?;
                thermal_schema_version_seen = true;
            }
            (7, _) => return Err(WireError::DuplicateField(7)),
            (8, WIRE_LEN)
                if material_surface_delta_schema_version >= MATERIAL_SURFACE_DELTA_SCHEMA_V4
                    && material_surface_thermal_deltas.len() < MAX_MATERIAL_SURFACE_DELTAS =>
            {
                material_surface_thermal_deltas
                    .push(decode_material_surface_thermal_delta(cursor.bytes()?)?)
            }
            (8, WIRE_LEN)
                if material_surface_delta_schema_version >= MATERIAL_SURFACE_DELTA_SCHEMA_V4 =>
            {
                cursor.bytes()?;
            }
            (8, _) => return Err(WireError::UnexpectedFieldForSchema(8)),
            _ => cursor.skip(wire)?,
        }
    }
    let material_surface_deltas = material_surface_delta_bytes
        .iter()
        .map(|bytes| decode_material_surface_delta(bytes, material_surface_delta_schema_version))
        .collect::<Result<Vec<_>, _>>()?;
    if !thermal_delta_bytes.is_empty() && thermal_delta_schema_version < THERMAL_DELTA_SCHEMA_V1 {
        return Err(WireError::UnexpectedFieldForSchema(6));
    }
    let thermal_deltas = thermal_delta_bytes
        .iter()
        .map(|bytes| decode_thermal_field_delta(bytes, thermal_delta_schema_version))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(ObserverWorldSnapshot {
        time: time.ok_or(WireError::MissingField(1))?,
        chunks,
        material_surface_delta_schema_version,
        material_surface_deltas,
        material_surface_gate_deltas,
        material_surface_thermal_deltas,
        thermal_delta_schema_version,
        thermal_deltas,
    })
}

/* ------------------------------------------------------------ field raster -- */

pub fn encode_field_raster_request(request: &FieldRasterRequest) -> Vec<u8> {
    let mut out = Vec::new();
    field_varint(&mut out, 1, request.chart_id);
    field_varint(&mut out, 2, zigzag(i64::from(request.chunk_x)));
    field_varint(&mut out, 3, zigzag(i64::from(request.chunk_y)));
    field_varint(&mut out, 4, zigzag(i64::from(request.chunk_z)));
    field_varint(&mut out, 5, request.field as u64);
    field_varint(&mut out, 6, u64::from(request.detail_level));
    out
}

pub fn decode_field_raster_request(bytes: &[u8]) -> Result<FieldRasterRequest, WireError> {
    let mut cursor = Cursor::new(bytes);
    let mut chart_id = None;
    let mut chunk_x = 0_i32;
    let mut chunk_y = 0_i32;
    let mut chunk_z = 0_i32;
    let mut field = None;
    let mut detail_level = 0_u8;
    while !cursor.is_empty() {
        let (number, wire) = cursor.key()?;
        match (number, wire) {
            (1, WIRE_VARINT) => chart_id = Some(cursor.varint()?),
            (2, WIRE_VARINT) => chunk_x = to_i32(unzigzag(cursor.varint()?))?,
            (3, WIRE_VARINT) => chunk_y = to_i32(unzigzag(cursor.varint()?))?,
            (4, WIRE_VARINT) => chunk_z = to_i32(unzigzag(cursor.varint()?))?,
            (5, WIRE_VARINT) => field = Some(FieldRasterKind::try_from(to_u32(cursor.varint()?)?)?),
            (6, WIRE_VARINT) => {
                detail_level =
                    u8::try_from(cursor.varint()?).map_err(|_| WireError::IntegerOverflow)?
            }
            _ => cursor.skip(wire)?,
        }
    }
    let request = FieldRasterRequest {
        chart_id: chart_id.ok_or(WireError::MissingField(1))?,
        chunk_x,
        chunk_y,
        chunk_z,
        field: field.ok_or(WireError::MissingField(5))?,
        detail_level,
    };
    request.validate()?;
    Ok(request)
}

/// The measured lattice, delta-encoded along its own scan order.
///
/// Elevation runs about seventy metres across a chunk with a mean neighbour step
/// of 1.6 m, so successive differences are the natural encoding. The difference
/// is taken with wrapping arithmetic and undone the same way, which round-trips
/// every `i64` exactly instead of failing at the extremes.
pub fn encode_field_raster(raster: &ObserverFieldRaster) -> Vec<u8> {
    let mut out = Vec::new();
    field_varint(&mut out, 1, raster.chart_id);
    field_varint(&mut out, 2, zigzag(i64::from(raster.chunk_x)));
    field_varint(&mut out, 3, zigzag(i64::from(raster.chunk_y)));
    field_varint(&mut out, 4, zigzag(i64::from(raster.chunk_z)));
    field_varint(&mut out, 5, raster.field as u64);
    field_varint(&mut out, 6, u64::from(raster.detail_level));
    field_varint(&mut out, 7, u64::from(raster.edge));
    field_varint(&mut out, 8, u64::from(raster.depth));
    field_bytes(&mut out, 9, &encode_delta_band(&raster.values));
    if !raster.auxiliary.is_empty() {
        field_bytes(&mut out, 10, &encode_delta_band(&raster.auxiliary));
    }
    if !raster.cell_traces.is_empty() {
        let mut packed = Vec::new();
        for trace in &raster.cell_traces {
            varint(&mut packed, *trace);
        }
        field_bytes(&mut out, 11, &packed);
    }
    field_varint(&mut out, 12, raster.generation_trace);
    out
}

pub fn decode_field_raster(bytes: &[u8]) -> Result<ObserverFieldRaster, WireError> {
    let mut cursor = Cursor::new(bytes);
    let mut chart_id = None;
    let mut chunk_x = 0_i32;
    let mut chunk_y = 0_i32;
    let mut chunk_z = 0_i32;
    let mut field = None;
    let mut detail_level = 0_u8;
    let mut edge = None;
    let mut depth = None;
    let mut values = Vec::new();
    let mut auxiliary = Vec::new();
    let mut cell_traces = Vec::new();
    let mut generation_trace = 0_u64;
    while !cursor.is_empty() {
        let (number, wire) = cursor.key()?;
        match (number, wire) {
            (1, WIRE_VARINT) => chart_id = Some(cursor.varint()?),
            (2, WIRE_VARINT) => chunk_x = to_i32(unzigzag(cursor.varint()?))?,
            (3, WIRE_VARINT) => chunk_y = to_i32(unzigzag(cursor.varint()?))?,
            (4, WIRE_VARINT) => chunk_z = to_i32(unzigzag(cursor.varint()?))?,
            (5, WIRE_VARINT) => field = Some(FieldRasterKind::try_from(to_u32(cursor.varint()?)?)?),
            (6, WIRE_VARINT) => {
                detail_level =
                    u8::try_from(cursor.varint()?).map_err(|_| WireError::IntegerOverflow)?
            }
            (7, WIRE_VARINT) => edge = Some(to_u32(cursor.varint()?)?),
            (8, WIRE_VARINT) => depth = Some(to_u32(cursor.varint()?)?),
            (9, WIRE_LEN) => values = decode_delta_band(cursor.bytes()?)?,
            (10, WIRE_LEN) => auxiliary = decode_delta_band(cursor.bytes()?)?,
            (11, WIRE_LEN) => {
                let mut packed = Cursor::new(cursor.bytes()?);
                while !packed.is_empty() {
                    cell_traces.push(packed.varint()?);
                }
            }
            (12, WIRE_VARINT) => generation_trace = cursor.varint()?,
            _ => cursor.skip(wire)?,
        }
    }
    let raster = ObserverFieldRaster {
        chart_id: chart_id.ok_or(WireError::MissingField(1))?,
        chunk_x,
        chunk_y,
        chunk_z,
        field: field.ok_or(WireError::MissingField(5))?,
        detail_level,
        edge: edge.ok_or(WireError::MissingField(7))?,
        depth: depth.ok_or(WireError::MissingField(8))?,
        values,
        auxiliary,
        cell_traces,
        generation_trace,
    };
    // A raster whose declared lattice does not match its payload cannot be drawn
    // at real positions, and a renderer must never be left to guess the shape.
    // `edge` and `depth` come off the wire, so dimensions that cannot describe a
    // lattice at all are rejected before anything is compared against them.
    let cell_count = raster
        .cell_count()
        .ok_or(WireError::UnrepresentableFieldRasterLattice {
            edge: raster.edge,
            depth: raster.depth,
        })?;
    if raster.values.len() != cell_count {
        return Err(WireError::InvalidFieldRasterLattice {
            expected: cell_count,
            received: raster.values.len(),
        });
    }
    if !raster.auxiliary.is_empty() && raster.auxiliary.len() != cell_count {
        return Err(WireError::InvalidFieldRasterLattice {
            expected: cell_count,
            received: raster.auxiliary.len(),
        });
    }
    if !raster.cell_traces.is_empty() && raster.cell_traces.len() != cell_count {
        return Err(WireError::InvalidFieldRasterLattice {
            expected: cell_count,
            received: raster.cell_traces.len(),
        });
    }
    Ok(raster)
}

fn encode_delta_band(values: &[i64]) -> Vec<u8> {
    let mut packed = Vec::new();
    let mut previous = 0_i64;
    for value in values {
        varint(&mut packed, zigzag(value.wrapping_sub(previous)));
        previous = *value;
    }
    packed
}

fn decode_delta_band(bytes: &[u8]) -> Result<Vec<i64>, WireError> {
    let mut cursor = Cursor::new(bytes);
    let mut values = Vec::new();
    let mut previous = 0_i64;
    while !cursor.is_empty() {
        previous = previous.wrapping_add(unzigzag(cursor.varint()?));
        values.push(previous);
    }
    Ok(values)
}

fn encode_thermal_field_delta(delta: &ThermalFieldDelta) -> Vec<u8> {
    let mut nested = Vec::new();
    field_varint(&mut nested, 1, delta.chart_id);
    field_varint(&mut nested, 2, zigzag(i64::from(delta.chunk_x)));
    field_varint(&mut nested, 3, zigzag(i64::from(delta.chunk_y)));
    field_varint(&mut nested, 4, zigzag(i64::from(delta.chunk_z)));
    field_varint(&mut nested, 5, u64::from(delta.cell_ordinal));
    field_varint(&mut nested, 6, zigzag(delta.pre_state_energy));
    field_varint(&mut nested, 7, zigzag(delta.post_state_energy));
    field_varint(&mut nested, 8, zigzag(delta.reservoir_scheduled_injection));
    field_varint(&mut nested, 9, zigzag(delta.reservoir_accepted_injection));
    field_varint(&mut nested, 10, zigzag(delta.reservoir_rejected_injection));
    field_varint(&mut nested, 11, zigzag(delta.net_face_flux));
    field_varint(&mut nested, 12, u64::from(delta.face_count));
    nested
}

fn decode_thermal_field_delta(
    bytes: &[u8],
    schema_version: u32,
) -> Result<ThermalFieldDelta, WireError> {
    if schema_version < THERMAL_DELTA_SCHEMA_V1 {
        return Err(WireError::UnexpectedFieldForSchema(6));
    }
    let mut cursor = Cursor::new(bytes);
    let mut values = [None; 12];
    while !cursor.is_empty() {
        let (field, wire) = cursor.key()?;
        if (1..=12).contains(&field) && wire == WIRE_VARINT {
            values[field as usize - 1] = Some(cursor.varint()?);
        } else {
            cursor.skip(wire)?;
        }
    }
    let value = |field: usize| values[field - 1].ok_or(WireError::MissingField(field as u32));
    Ok(ThermalFieldDelta {
        chart_id: value(1)?,
        chunk_x: to_i32(unzigzag(value(2)?))?,
        chunk_y: to_i32(unzigzag(value(3)?))?,
        chunk_z: to_i32(unzigzag(value(4)?))?,
        cell_ordinal: u16::try_from(value(5)?).map_err(|_| WireError::IntegerOverflow)?,
        pre_state_energy: unzigzag(value(6)?),
        post_state_energy: unzigzag(value(7)?),
        reservoir_scheduled_injection: unzigzag(value(8)?),
        reservoir_accepted_injection: unzigzag(value(9)?),
        reservoir_rejected_injection: unzigzag(value(10)?),
        net_face_flux: unzigzag(value(11)?),
        face_count: to_u32(value(12)?)?,
    })
}

fn encode_material_surface_delta(delta: &MaterialSurfaceDelta, schema_version: u32) -> Vec<u8> {
    let mut nested = Vec::new();
    field_varint(&mut nested, 1, delta.chart_id);
    field_varint(&mut nested, 2, zigzag(i64::from(delta.chunk_x)));
    field_varint(&mut nested, 3, zigzag(i64::from(delta.chunk_y)));
    field_varint(&mut nested, 4, zigzag(i64::from(delta.chunk_z)));
    field_varint(&mut nested, 5, u64::from(delta.cell_ordinal));
    field_varint(&mut nested, 6, zigzag(delta.before_condition));
    field_varint(&mut nested, 7, zigzag(delta.after_condition));
    field_varint(&mut nested, 8, zigzag(delta.mana_total));
    if let Some(trace) = delta.contact_trace {
        field_varint(&mut nested, 9, trace.raw());
    }
    if let Some(trace) = delta.mana_effect_trace {
        field_varint(&mut nested, 10, trace.raw());
    }
    field_varint(&mut nested, 11, delta.transition_tick);
    if let Some(trace) = delta.mana_transition_trace {
        field_varint(&mut nested, 12, trace.raw());
    }
    if let Some(value) = delta.mana_before {
        field_varint(&mut nested, 13, zigzag(value));
    }
    if let Some(value) = delta.mana_after {
        field_varint(&mut nested, 14, zigzag(value));
    }
    if schema_version >= MATERIAL_SURFACE_DELTA_SCHEMA_V3 {
        if let Some(value) = delta.local_mana_before {
            field_varint(&mut nested, 15, zigzag(value));
        }
        if let Some(value) = delta.local_mana_after {
            field_varint(&mut nested, 16, zigzag(value));
        }
        if let Some(trace) = delta.local_mana_transition_trace_id {
            field_varint(&mut nested, 17, trace.raw());
        }
    }
    nested
}

fn decode_material_surface_delta(
    bytes: &[u8],
    schema_version: u32,
) -> Result<MaterialSurfaceDelta, WireError> {
    let mut cursor = Cursor::new(bytes);
    let mut values = [None; 17];
    while !cursor.is_empty() {
        let (field, wire) = cursor.key()?;
        if (15..=17).contains(&field) && schema_version < MATERIAL_SURFACE_DELTA_SCHEMA_V3 {
            return Err(WireError::UnexpectedFieldForSchema(field));
        }
        if (1..=17).contains(&field) && wire == WIRE_VARINT {
            values[field as usize - 1] = Some(cursor.varint()?);
        } else {
            cursor.skip(wire)?;
        }
    }
    let value = |field: usize| values[field - 1].ok_or(WireError::MissingField(field as u32));
    Ok(MaterialSurfaceDelta {
        chart_id: value(1)?,
        chunk_x: to_i32(unzigzag(value(2)?))?,
        chunk_y: to_i32(unzigzag(value(3)?))?,
        chunk_z: to_i32(unzigzag(value(4)?))?,
        cell_ordinal: u16::try_from(value(5)?).map_err(|_| WireError::IntegerOverflow)?,
        before_condition: unzigzag(value(6)?),
        after_condition: unzigzag(value(7)?),
        mana_total: unzigzag(value(8)?),
        contact_trace: values[8].map(TraceId::new),
        mana_effect_trace: values[9].map(TraceId::new),
        transition_tick: value(11)?,
        mana_transition_trace: values[11].map(TraceId::new),
        mana_before: values[12].map(unzigzag),
        mana_after: values[13].map(unzigzag),
        local_mana_before: values[14].map(unzigzag),
        local_mana_after: values[15].map(unzigzag),
        local_mana_transition_trace_id: values[16].map(TraceId::new),
    })
}

fn encode_material_surface_gate_delta(delta: &MaterialSurfaceGateDelta) -> Vec<u8> {
    let mut nested = Vec::new();
    field_varint(&mut nested, 1, delta.chart_id);
    field_varint(&mut nested, 2, zigzag(i64::from(delta.chunk_x)));
    field_varint(&mut nested, 3, zigzag(i64::from(delta.chunk_y)));
    field_varint(&mut nested, 4, zigzag(i64::from(delta.chunk_z)));
    field_varint(&mut nested, 5, u64::from(delta.cell_ordinal));
    field_varint(&mut nested, 6, u64::from(delta.before_active));
    field_varint(&mut nested, 7, u64::from(delta.after_active));
    field_varint(&mut nested, 8, zigzag(delta.local_mana_before));
    field_varint(&mut nested, 9, zigzag(delta.local_mana_after));
    field_varint(&mut nested, 10, delta.local_mana_transition_trace_id.raw());
    field_varint(&mut nested, 11, delta.gate_transition_trace_id.raw());
    if let Some(trace) = delta.contact_trace_id {
        field_varint(&mut nested, 12, trace.raw());
    }
    field_varint(&mut nested, 13, delta.transition_tick);
    nested
}

fn decode_material_surface_gate_delta(bytes: &[u8]) -> Result<MaterialSurfaceGateDelta, WireError> {
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
    Ok(MaterialSurfaceGateDelta {
        chart_id: value(1)?,
        chunk_x: to_i32(unzigzag(value(2)?))?,
        chunk_y: to_i32(unzigzag(value(3)?))?,
        chunk_z: to_i32(unzigzag(value(4)?))?,
        cell_ordinal: u16::try_from(value(5)?).map_err(|_| WireError::IntegerOverflow)?,
        before_active: decode_bool(value(6)?)?,
        after_active: decode_bool(value(7)?)?,
        local_mana_before: unzigzag(value(8)?),
        local_mana_after: unzigzag(value(9)?),
        local_mana_transition_trace_id: TraceId::new(value(10)?),
        gate_transition_trace_id: TraceId::new(value(11)?),
        contact_trace_id: values[11].map(TraceId::new),
        transition_tick: value(13)?,
    })
}

fn encode_material_surface_thermal_delta(delta: &MaterialSurfaceThermalDelta) -> Vec<u8> {
    let mut nested = Vec::new();
    field_varint(&mut nested, 1, delta.chart_id);
    field_varint(&mut nested, 2, zigzag(i64::from(delta.chunk_x)));
    field_varint(&mut nested, 3, zigzag(i64::from(delta.chunk_y)));
    field_varint(&mut nested, 4, zigzag(i64::from(delta.chunk_z)));
    field_varint(&mut nested, 5, u64::from(delta.cell_ordinal));
    field_varint(&mut nested, 6, zigzag(delta.before_retained));
    field_varint(&mut nested, 7, zigzag(delta.after_retained));
    field_varint(&mut nested, 8, zigzag(delta.cell_pre_state));
    field_varint(&mut nested, 9, zigzag(delta.signed_flux));
    field_varint(&mut nested, 10, delta.thermal_exchange_trace_id.raw());
    field_varint(&mut nested, 11, delta.transition_tick);
    nested
}

fn decode_material_surface_thermal_delta(
    bytes: &[u8],
) -> Result<MaterialSurfaceThermalDelta, WireError> {
    let mut cursor = Cursor::new(bytes);
    let mut values = [None; 11];
    while !cursor.is_empty() {
        let (field, wire) = cursor.key()?;
        if (1..=11).contains(&field) && wire == WIRE_VARINT {
            values[field as usize - 1] = Some(cursor.varint()?);
        } else {
            cursor.skip(wire)?;
        }
    }
    let value = |field: usize| values[field - 1].ok_or(WireError::MissingField(field as u32));
    Ok(MaterialSurfaceThermalDelta {
        chart_id: value(1)?,
        chunk_x: to_i32(unzigzag(value(2)?))?,
        chunk_y: to_i32(unzigzag(value(3)?))?,
        chunk_z: to_i32(unzigzag(value(4)?))?,
        cell_ordinal: u16::try_from(value(5)?).map_err(|_| WireError::IntegerOverflow)?,
        before_retained: unzigzag(value(6)?),
        after_retained: unzigzag(value(7)?),
        cell_pre_state: unzigzag(value(8)?),
        signed_flux: unzigzag(value(9)?),
        thermal_exchange_trace_id: TraceId::new(value(10)?),
        transition_tick: value(11)?,
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
        scope: causafera_observer_api::StreamScope { kind, chunk },
        payload,
    })
}

fn decode_stream_header(bytes: &[u8]) -> Result<causafera_observer_api::StreamHeader, WireError> {
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
    Ok(causafera_observer_api::StreamHeader {
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

pub fn decode_explanation_report(bytes: &[u8]) -> Result<ExplanationReport, WireError> {
    let mut cursor = Cursor::new(bytes);
    let mut experiment = None;
    let mut frames = Vec::new();
    let mut assessment = None;
    while !cursor.is_empty() {
        let (field, wire) = cursor.key()?;
        match (field, wire) {
            (1, WIRE_VARINT) => experiment = Some(ExperimentId::new(cursor.varint()?)),
            (2, WIRE_LEN) => frames.push(decode_explanation_frame(cursor.bytes()?)?),
            (3, WIRE_VARINT) => assessment = Some(decode_assessment(cursor.varint()?)?),
            _ => cursor.skip(wire)?,
        }
    }
    let report = ExplanationReport::new(experiment.ok_or(WireError::MissingField(1))?, frames)
        .map_err(|_| WireError::InvalidExplanationReport)?;
    if report.overall_assessment != assessment.ok_or(WireError::MissingField(3))? {
        return Err(WireError::InvalidExplanationReport);
    }
    Ok(report)
}

fn decode_explanation_frame(bytes: &[u8]) -> Result<ExplanationFrame, WireError> {
    let mut cursor = Cursor::new(bytes);
    let mut checkpoint_time = None;
    let mut claims = Vec::new();
    let mut assessment = None;
    while !cursor.is_empty() {
        let (field, wire) = cursor.key()?;
        match (field, wire) {
            (1, WIRE_VARINT) => checkpoint_time = Some(SimulationTime::new(cursor.varint()?)),
            (2, WIRE_LEN) => claims.push(decode_explanation_claim(cursor.bytes()?)?),
            (3, WIRE_VARINT) => assessment = Some(decode_assessment(cursor.varint()?)?),
            _ => cursor.skip(wire)?,
        }
    }
    let frame = ExplanationFrame::new(checkpoint_time.ok_or(WireError::MissingField(1))?, claims)
        .map_err(|_| WireError::InvalidExplanationFrame)?;
    if frame.overall_assessment != assessment.ok_or(WireError::MissingField(3))? {
        return Err(WireError::InvalidExplanationFrame);
    }
    Ok(frame)
}

fn decode_explanation_claim(bytes: &[u8]) -> Result<ExplanationClaim, WireError> {
    let mut cursor = Cursor::new(bytes);
    let mut schema = None;
    let mut value = None;
    let mut confidence = None;
    let mut evidence_traces = Vec::new();
    let mut comparison = None;
    let mut evidence_state = None;
    while !cursor.is_empty() {
        let (field, wire) = cursor.key()?;
        match (field, wire) {
            (1, WIRE_VARINT) => schema = Some(ExplanationClaimSchemaId::new(cursor.varint()?)),
            (2, WIRE_LEN) => value = Some(decode_numeric_value(cursor.bytes()?)?),
            (3, WIRE_FIXED64) => {
                confidence = Some(
                    ClaimConfidence::new(f64::from_bits(cursor.fixed64()?))
                        .map_err(|_| WireError::InvalidClaimConfidence)?,
                )
            }
            (4, WIRE_VARINT) => evidence_traces.push(TraceId::new(cursor.varint()?)),
            (5, WIRE_LEN) => comparison = Some(decode_comparison(cursor.bytes()?)?),
            (6, WIRE_VARINT) => evidence_state = Some(decode_evidence_state(cursor.varint()?)?),
            _ => cursor.skip(wire)?,
        }
    }
    ExplanationClaim::new(
        schema.ok_or(WireError::MissingField(1))?,
        value.ok_or(WireError::MissingField(2))?,
        confidence.ok_or(WireError::MissingField(3))?,
        evidence_traces,
        comparison.ok_or(WireError::MissingField(5))?,
        evidence_state.ok_or(WireError::MissingField(6))?,
    )
    .map_err(|_| WireError::InvalidExplanationClaim)
}

fn decode_numeric_value(bytes: &[u8]) -> Result<NumericClaimValue, WireError> {
    let mut cursor = Cursor::new(bytes);
    while !cursor.is_empty() {
        let (field, wire) = cursor.key()?;
        match (field, wire) {
            (1, WIRE_VARINT) => return Ok(NumericClaimValue::scalar(unzigzag(cursor.varint()?))),
            (2, WIRE_LEN) => {
                let (start, end) = decode_numeric_pair(cursor.bytes()?)?;
                return NumericClaimValue::range(unzigzag(start), unzigzag(end))
                    .map_err(|_| WireError::InvalidNumericClaimValue);
            }
            (3, WIRE_LEN) => {
                let (numerator, denominator) = decode_numeric_pair(cursor.bytes()?)?;
                return NumericClaimValue::ratio(numerator, denominator)
                    .map_err(|_| WireError::InvalidNumericClaimValue);
            }
            _ => cursor.skip(wire)?,
        }
    }
    Err(WireError::InvalidNumericClaimValue)
}

fn decode_numeric_pair(bytes: &[u8]) -> Result<(u64, u64), WireError> {
    let mut cursor = Cursor::new(bytes);
    let mut first = None;
    let mut second = None;
    while !cursor.is_empty() {
        let (field, wire) = cursor.key()?;
        match (field, wire) {
            (1, WIRE_VARINT) => first = Some(cursor.varint()?),
            (2, WIRE_VARINT) => second = Some(cursor.varint()?),
            _ => cursor.skip(wire)?,
        }
    }
    Ok((
        first.ok_or(WireError::MissingField(1))?,
        second.ok_or(WireError::MissingField(2))?,
    ))
}

fn decode_comparison(bytes: &[u8]) -> Result<ComparisonContext, WireError> {
    let (kind, cohort) = decode_numeric_pair_optional_second(bytes)?;
    match kind {
        0 => Ok(ComparisonContext::None),
        1 => Ok(ComparisonContext::MatchedCohort {
            cohort: ComparisonCohortId::new(cohort.ok_or(WireError::MissingField(2))?),
        }),
        2 => Ok(ComparisonContext::Counterfactual {
            cohort: ComparisonCohortId::new(cohort.ok_or(WireError::MissingField(2))?),
        }),
        value => Err(WireError::UnknownComparisonContext(value)),
    }
}

fn decode_numeric_pair_optional_second(bytes: &[u8]) -> Result<(u64, Option<u64>), WireError> {
    let mut cursor = Cursor::new(bytes);
    let mut first = None;
    let mut second = None;
    while !cursor.is_empty() {
        let (field, wire) = cursor.key()?;
        match (field, wire) {
            (1, WIRE_VARINT) => first = Some(cursor.varint()?),
            (2, WIRE_VARINT) => second = Some(cursor.varint()?),
            _ => cursor.skip(wire)?,
        }
    }
    Ok((first.ok_or(WireError::MissingField(1))?, second))
}

fn decode_evidence_state(value: u64) -> Result<ClaimEvidenceState, WireError> {
    match value {
        1 => Ok(ClaimEvidenceState::Supported),
        2 => Ok(ClaimEvidenceState::Unsupported),
        3 => Ok(ClaimEvidenceState::Unknown),
        value => Err(WireError::UnknownEvidenceState(value)),
    }
}

fn decode_assessment(value: u64) -> Result<FrameAssessment, WireError> {
    match value {
        1 => Ok(FrameAssessment::Supported),
        2 => Ok(FrameAssessment::Partial),
        3 => Ok(FrameAssessment::Unsupported),
        4 => Ok(FrameAssessment::Unknown),
        value => Err(WireError::UnknownAssessment(value)),
    }
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

fn encode_i128_zigzag(value: i128) -> Vec<u8> {
    let bits = u128::from_ne_bytes(value.to_ne_bytes());
    let encoded = if value.is_negative() {
        ((!bits) << 1) | 1
    } else {
        bits << 1
    };
    let mut bytes = Vec::new();
    varint_u128(&mut bytes, encoded);
    bytes
}

fn decode_i128_zigzag(bytes: &[u8]) -> Result<i128, WireError> {
    let mut encoded = 0_u128;
    for (index, byte) in bytes.iter().copied().enumerate() {
        let shift = u32::try_from(index.checked_mul(7).ok_or(WireError::IntegerOverflow)?)
            .map_err(|_| WireError::IntegerOverflow)?;
        if shift >= 128 {
            return Err(WireError::InvalidVarint);
        }
        let part = u128::from(byte & 0x7f);
        if part > (u128::MAX >> shift) {
            return Err(WireError::IntegerOverflow);
        }
        encoded |= part << shift;
        if byte & 0x80 == 0 {
            if index + 1 != bytes.len() {
                return Err(WireError::InvalidVarint);
            }
            let sign = if encoded & 1 == 0 { 0 } else { u128::MAX };
            let decoded = (encoded >> 1) ^ sign;
            return Ok(i128::from_ne_bytes(decoded.to_ne_bytes()));
        }
    }
    Err(WireError::InvalidVarint)
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

fn varint_u128(out: &mut Vec<u8>, mut value: u128) {
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

fn decode_bool(value: u64) -> Result<bool, WireError> {
    match value {
        0 => Ok(false),
        1 => Ok(true),
        _ => Err(WireError::InvalidBoolean(value)),
    }
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
            // The tenth byte contributes only bit 63, so anything above one in
            // its payload describes a value wider than 64 bits. Shifting it in
            // would drop those bits silently, which is worse than rejecting:
            // the TypeScript decoder accumulates into a bigint and cannot
            // truncate, so the two would disagree on whether the payload is
            // valid rather than merely on what it means.
            if shift == 63 && byte & 0x7f > 1 {
                return Err(WireError::InvalidVarint);
            }
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
    fn fixed64(&mut self) -> Result<u64, WireError> {
        let end = self.at.checked_add(8).ok_or(WireError::IntegerOverflow)?;
        let bytes = self
            .bytes
            .get(self.at..end)
            .ok_or(WireError::UnexpectedEof)?;
        self.at = end;
        Ok(u64::from_le_bytes(
            bytes.try_into().map_err(|_| WireError::UnexpectedEof)?,
        ))
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
    #[error("protobuf boolean must be zero or one, got {0}")]
    InvalidBoolean(u64),
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
    #[error("explanation claim confidence is invalid")]
    InvalidClaimConfidence,
    #[error("explanation numeric claim value is invalid")]
    InvalidNumericClaimValue,
    #[error("explanation claim is invalid")]
    InvalidExplanationClaim,
    #[error("explanation frame is invalid")]
    InvalidExplanationFrame,
    #[error("explanation report is invalid")]
    InvalidExplanationReport,
    #[error("unknown explanation comparison context {0}")]
    UnknownComparisonContext(u64),
    #[error("unknown explanation evidence state {0}")]
    UnknownEvidenceState(u64),
    #[error("unknown explanation assessment {0}")]
    UnknownAssessment(u64),
    #[error("field {0} is not allowed for the current schema version")]
    UnexpectedFieldForSchema(u32),
    #[error("duplicate protobuf field {0}")]
    DuplicateField(u32),
    #[error("field raster declares {expected} cells but carries {received}")]
    InvalidFieldRasterLattice { expected: usize, received: usize },
    /// Distinct from the above because the count is not merely wrong, it does
    /// not exist: reporting `expected: usize::MAX` claimed the raster declared
    /// 18446744073709551615 cells when what it declared cannot be represented
    /// at all.
    #[error(
        "field raster declares a lattice of edge {edge} and depth {depth}, which is not a representable cell count"
    )]
    UnrepresentableFieldRasterLattice { edge: u32, depth: u32 },
    #[error("bounded observer payload exceeds its declared maximum")]
    PayloadTooLarge,
    #[error("bounded observer payload is not in canonical order")]
    NonCanonicalOrder,
    #[error(transparent)]
    Api(#[from] causafera_observer_api::ObserverApiError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use causafera_explanation::{
        ClaimConfidence, ClaimEvidenceState, ComparisonContext, ExplanationClaim,
        ExplanationClaimSchemaId, ExplanationFrame, NumericClaimValue,
    };
    use causafera_observer_api::{DeliveryPolicy, ObserverStreamHub, StreamScope};
    use causafera_types::ExperimentId;

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
            thermal_total_cell_energy: i128::MAX,
            thermal_total_reservoir_budget: 200,
            thermal_active_chunk_count: 2,
            thermal_active_cell_count: 54,
            bootstrap: ObserverBootstrapSummary {
                schema_version: causafera_observer_api::BOOTSTRAP_SUMMARY_SCHEMA_V1,
                plan_id: 0xDEAD_BEEF,
                world_seed: 44,
                stage_count: 2,
                complete: true,
                configured_population: 512,
                configured_promotion_limit: 8,
                receipts: vec![
                    ObserverBootstrapReceipt {
                        stage: 1,
                        completed_at: SimulationTime::new(1),
                        result: [7; 32],
                        completion_trace: TraceId::new(40),
                        dependency_traces: Vec::new(),
                    },
                    ObserverBootstrapReceipt {
                        stage: 2,
                        completed_at: SimulationTime::new(2),
                        result: [8; 32],
                        completion_trace: TraceId::new(41),
                        dependency_traces: vec![TraceId::new(40)],
                    },
                ],
            },
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

    fn raster(values: Vec<i64>, edge: u32, depth: u32) -> ObserverFieldRaster {
        ObserverFieldRaster {
            chart_id: 1,
            chunk_x: -2,
            chunk_y: 3,
            chunk_z: 0,
            field: FieldRasterKind::TerrainElevation,
            detail_level: 0,
            edge,
            depth,
            values,
            auxiliary: Vec::new(),
            cell_traces: Vec::new(),
            generation_trace: 88,
        }
    }

    #[test]
    fn field_raster_request_roundtrips_and_bounds_its_detail_level() {
        let request = FieldRasterRequest {
            chart_id: 1,
            chunk_x: -3,
            chunk_y: 4,
            chunk_z: 0,
            field: FieldRasterKind::ManaIntensity,
            detail_level: 2,
        };

        assert_eq!(
            decode_field_raster_request(&encode_field_raster_request(&request)).unwrap(),
            request
        );

        let beyond = FieldRasterRequest {
            detail_level: 3,
            ..request
        };
        assert!(matches!(
            decode_field_raster_request(&encode_field_raster_request(&beyond)),
            Err(WireError::Api(_))
        ));
    }

    /// The delta band must survive the values it will actually meet, which
    /// includes both `i32` bounds after widening and the `i64` extremes.
    #[test]
    fn field_raster_delta_encoding_roundtrips_at_the_integer_extremes() {
        let values = vec![
            i64::MIN,
            i64::MAX,
            0,
            i64::from(i32::MIN),
            i64::from(i32::MAX),
            -1,
            1,
            i64::MIN,
            i64::MAX,
        ];
        let expected = raster(values, 3, 1);

        assert_eq!(
            decode_field_raster(&encode_field_raster(&expected)).unwrap(),
            expected
        );
    }

    #[test]
    fn field_raster_roundtrips_a_volumetric_lattice_with_per_cell_provenance() {
        let mut expected = raster((0..27).map(|value| value * 31 - 400).collect(), 3, 3);
        expected.field = FieldRasterKind::ManaIntensity;
        expected.auxiliary = (0..27).map(|value| value * 2).collect();
        expected.cell_traces = (0..27).map(|value| value as u64 * 7).collect();

        assert_eq!(
            decode_field_raster(&encode_field_raster(&expected)).unwrap(),
            expected
        );
    }

    #[test]
    fn field_raster_refuses_a_lattice_its_payload_does_not_fill() {
        let mut encoded = raster((0..9).collect(), 3, 1);
        encoded.edge = 4;

        assert!(matches!(
            decode_field_raster(&encode_field_raster(&encoded)),
            Err(WireError::InvalidFieldRasterLattice {
                expected: 16,
                received: 9
            })
        ));
    }

    /// `cell_count` became a checked `Option` on this branch. Before it did, the
    /// lattice product was an unchecked multiply, so a declared lattice whose
    /// `edge² × depth` is exactly 2⁶⁴ wrapped to zero and a release build
    /// accepted the payload with an empty value band. The overflow branch is the
    /// whole point of the change and nothing pinned it.
    #[test]
    fn field_raster_refuses_a_lattice_whose_cell_count_does_not_fit() {
        for (edge, depth) in [(1_u32 << 24, 1_u32 << 16), (1 << 26, 1 << 12)] {
            let mut encoded = raster(Vec::new(), 1, 1);
            encoded.edge = edge;
            encoded.depth = depth;

            assert!(
                matches!(
                    decode_field_raster(&encode_field_raster(&encoded)),
                    Err(WireError::UnrepresentableFieldRasterLattice { .. })
                ),
                "edge {edge} depth {depth} declares more cells than can be represented"
            );
        }
    }

    /// The tenth continuation byte of a 64-bit varint carries a single usable
    /// bit. Accepting more silently truncated, so the two decoders could read
    /// different values from identical bytes; this pins the Rust side of that
    /// bound, which only the Node suite covered.
    #[test]
    fn a_varint_wider_than_sixty_four_bits_is_rejected() {
        // Nine payload bytes of ones covers every shift from 0 to 56, so a
        // mutation of the shift-56 byte is visible here too; an earlier revision
        // zeroed that byte and could not see it.
        let payload = |tenth: u8| {
            let mut bytes = vec![0x08_u8];
            bytes.extend(std::iter::repeat_n(0xFF_u8, 9));
            bytes.push(tenth);
            bytes
        };

        // A tenth byte of 0 or 1 is the only representable continuation. The
        // decoded VALUE is asserted, not merely the absence of a rejection: a
        // bound applied at the wrong shift, or one that drops bit 63, still
        // produces a rejection-free decode of the wrong number, and an earlier
        // revision of this test could not tell those apart.
        let complete = |tenth: u8| {
            let mut raster = raster((0..1).collect(), 1, 1);
            raster.chart_id = 0;
            let encoded = encode_field_raster(&raster);
            // `encode_field_raster` writes field 1 first, as key plus one byte
            // for a chart id of zero.
            let mut out = payload(tenth);
            out.extend_from_slice(&encoded[2..]);
            out
        };
        assert_eq!(
            decode_field_raster(&complete(0x00))
                .expect("a representable varint must decode")
                .chart_id,
            u64::MAX >> 1,
            "nine payload bytes of ones fill bits 0 to 62; the tenth byte is zero"
        );
        assert_eq!(
            decode_field_raster(&complete(0x01))
                .expect("a representable varint must decode")
                .chart_id,
            u64::MAX,
            "the tenth byte's single usable bit is bit 63"
        );

        for tenth in [0x02_u8, 0x7F, 0x80, 0x81, 0xFF] {
            assert!(
                matches!(
                    decode_field_raster(&payload(tenth)),
                    Err(WireError::InvalidVarint)
                ),
                "tenth varint byte {tenth:#04x} does not fit in 64 bits"
            );
        }
    }

    #[test]
    fn negotiation_advertises_the_field_raster_query() {
        let handler = ProtocolHandler::default();

        let response = handler
            .negotiate(&ConnectRequest {
                supported_versions: vec![1],
                locale: "en-US".into(),
            })
            .unwrap();

        assert!(
            response
                .capabilities
                .contains(&(QueryKind::FieldRaster as u32))
        );
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
            material_surface_delta_schema_version: 0,
            material_surface_deltas: Vec::new(),
            material_surface_thermal_deltas: Vec::new(),
            material_surface_gate_deltas: Vec::new(),
            thermal_delta_schema_version: 0,
            thermal_deltas: Vec::new(),
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
    fn world_query_roundtrips_bounded_material_surface_deltas() {
        // Given: a chart-qualified, typed material transition from a live observer read model.
        let expected = ObserverWorldSnapshot {
            time: SimulationTime::new(8),
            chunks: Vec::new(),
            material_surface_delta_schema_version: 3,
            material_surface_deltas: vec![
                MaterialSurfaceDelta {
                    chart_id: 5,
                    chunk_x: -1,
                    chunk_y: 2,
                    chunk_z: -3,
                    cell_ordinal: 7,
                    before_condition: 4,
                    after_condition: 6,
                    mana_total: 12,
                    contact_trace: None,
                    mana_effect_trace: None,
                    transition_tick: 0,
                    mana_transition_trace: None,
                    mana_before: None,
                    mana_after: None,
                    local_mana_before: None,
                    local_mana_after: None,
                    local_mana_transition_trace_id: None,
                },
                MaterialSurfaceDelta {
                    chart_id: 5,
                    chunk_x: -1,
                    chunk_y: 2,
                    chunk_z: -3,
                    cell_ordinal: 8,
                    before_condition: 6,
                    after_condition: 9,
                    mana_total: 12,
                    contact_trace: Some(TraceId::new(0)),
                    mana_effect_trace: Some(TraceId::new(22)),
                    transition_tick: 8,
                    mana_transition_trace: Some(TraceId::new(21)),
                    mana_before: Some(0),
                    mana_after: Some(3),
                    local_mana_before: Some(0),
                    local_mana_after: Some(3),
                    local_mana_transition_trace_id: Some(TraceId::new(21)),
                },
            ],
            material_surface_thermal_deltas: Vec::new(),
            material_surface_gate_deltas: vec![MaterialSurfaceGateDelta {
                chart_id: 5,
                chunk_x: -1,
                chunk_y: 2,
                chunk_z: -3,
                cell_ordinal: 8,
                before_active: true,
                after_active: false,
                local_mana_before: 3,
                local_mana_after: 0,
                local_mana_transition_trace_id: TraceId::new(23),
                gate_transition_trace_id: TraceId::new(24),
                contact_trace_id: None,
                transition_tick: 9,
            }],
            thermal_delta_schema_version: 0,
            thermal_deltas: Vec::new(),
        };
        let mut handler = ProtocolHandler::default();
        handler.set_world_snapshot(&expected);

        // When: an OBSERVER_PROTOCOL_V1 client asks for the existing world-chunk read model.
        let response = decode_response(
            &handler
                .handle_query(&encode_query(&ObserverQuery::world_chunks(18)))
                .unwrap(),
        )
        .unwrap();

        // Then: the additive bounded delta projection round-trips without a new query kind.
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
        assert_eq!(
            decode_explanation_report(&response.payload).unwrap(),
            report
        );
        assert!(!response.payload.is_empty());
    }
}

#[cfg(test)]
#[test]
fn world_query_roundtrips_material_delta_mana_transition_trace() {
    // Given: a material delta with all additive mana-transition evidence fields populated.
    let expected = ObserverWorldSnapshot {
        time: SimulationTime::new(8),
        chunks: Vec::new(),
        material_surface_delta_schema_version: MATERIAL_SURFACE_DELTA_SCHEMA_V3,
        material_surface_deltas: vec![MaterialSurfaceDelta {
            chart_id: 5,
            chunk_x: -1,
            chunk_y: 2,
            chunk_z: -3,
            cell_ordinal: 7,
            before_condition: 4,
            after_condition: 6,
            mana_total: 12,
            contact_trace: Some(TraceId::new(19)),
            mana_effect_trace: Some(TraceId::new(22)),
            transition_tick: 8,
            mana_transition_trace: Some(TraceId::new(21)),
            mana_before: Some(0),
            mana_after: Some(3),
            local_mana_before: Some(0),
            local_mana_after: Some(3),
            local_mana_transition_trace_id: Some(TraceId::new(21)),
        }],
        material_surface_thermal_deltas: Vec::new(),
        material_surface_gate_deltas: Vec::new(),
        thermal_delta_schema_version: 0,
        thermal_deltas: Vec::new(),
    };
    let mut handler = ProtocolHandler::default();
    handler.set_world_snapshot(&expected);

    // When: a world-chunk query crosses the observer wire codec.
    let response = decode_response(
        &handler
            .handle_query(&encode_query(&ObserverQuery::world_chunks(19)))
            .unwrap(),
    )
    .unwrap();

    // Then: fields 11-14 round-trip without changing the query kind or bounded shape.
    assert_eq!(response.status, QueryStatus::Ok);
    assert_eq!(decode_world_snapshot(&response.payload).unwrap(), expected);
}

#[cfg(test)]
#[test]
fn decode_world_snapshot_rejects_gate_deltas_under_v2_schema() {
    let v2 = ObserverWorldSnapshot {
        time: SimulationTime::new(9),
        chunks: Vec::new(),
        material_surface_delta_schema_version: 2,
        material_surface_deltas: vec![MaterialSurfaceDelta {
            chart_id: 1,
            chunk_x: 0,
            chunk_y: 0,
            chunk_z: 0,
            cell_ordinal: 2,
            before_condition: 4,
            after_condition: 5,
            mana_total: 7,
            contact_trace: Some(TraceId::new(10)),
            mana_effect_trace: Some(TraceId::new(12)),
            transition_tick: 9,
            mana_transition_trace: Some(TraceId::new(11)),
            mana_before: Some(0),
            mana_after: Some(3),
            local_mana_before: None,
            local_mana_after: None,
            local_mana_transition_trace_id: None,
        }],
        material_surface_thermal_deltas: Vec::new(),
        material_surface_gate_deltas: Vec::new(),
        thermal_delta_schema_version: 0,
        thermal_deltas: Vec::new(),
    };
    let mut encoded = encode_world_snapshot(&v2);
    assert_eq!(decode_world_snapshot(&encoded).unwrap(), v2);

    let gate_delta = MaterialSurfaceGateDelta {
        chart_id: 1,
        chunk_x: 0,
        chunk_y: 0,
        chunk_z: 0,
        cell_ordinal: 2,
        before_active: true,
        after_active: false,
        local_mana_before: 3,
        local_mana_after: 0,
        local_mana_transition_trace_id: TraceId::new(13),
        gate_transition_trace_id: TraceId::new(14),
        contact_trace_id: None,
        transition_tick: 10,
    };
    field_bytes(
        &mut encoded,
        5,
        &encode_material_surface_gate_delta(&gate_delta),
    );
    assert_eq!(
        decode_world_snapshot(&encoded),
        Err(WireError::UnexpectedFieldForSchema(5))
    );
}

#[cfg(test)]
#[test]
fn decode_material_surface_delta_rejects_v3_fields_under_v2_schema() {
    let delta = MaterialSurfaceDelta {
        chart_id: 1,
        chunk_x: 0,
        chunk_y: 0,
        chunk_z: 0,
        cell_ordinal: 2,
        before_condition: 4,
        after_condition: 5,
        mana_total: 7,
        contact_trace: Some(TraceId::new(10)),
        mana_effect_trace: Some(TraceId::new(12)),
        transition_tick: 9,
        mana_transition_trace: Some(TraceId::new(11)),
        mana_before: Some(0),
        mana_after: Some(3),
        local_mana_before: Some(0),
        local_mana_after: Some(3),
        local_mana_transition_trace_id: Some(TraceId::new(11)),
    };
    let encoded = encode_material_surface_delta(&delta, MATERIAL_SURFACE_DELTA_SCHEMA_V3);
    assert_eq!(
        decode_material_surface_delta(&encoded, 2),
        Err(WireError::UnexpectedFieldForSchema(15))
    );
}

#[cfg(test)]
#[test]
fn decode_world_snapshot_rejects_duplicate_schema_version() {
    let v3 = ObserverWorldSnapshot {
        time: SimulationTime::new(9),
        chunks: Vec::new(),
        material_surface_delta_schema_version: MATERIAL_SURFACE_DELTA_SCHEMA_V3,
        material_surface_deltas: vec![MaterialSurfaceDelta {
            chart_id: 1,
            chunk_x: 0,
            chunk_y: 0,
            chunk_z: 0,
            cell_ordinal: 2,
            before_condition: 4,
            after_condition: 5,
            mana_total: 7,
            contact_trace: Some(TraceId::new(10)),
            mana_effect_trace: Some(TraceId::new(12)),
            transition_tick: 9,
            mana_transition_trace: Some(TraceId::new(11)),
            mana_before: Some(0),
            mana_after: Some(3),
            local_mana_before: Some(0),
            local_mana_after: Some(3),
            local_mana_transition_trace_id: Some(TraceId::new(11)),
        }],
        material_surface_thermal_deltas: Vec::new(),
        material_surface_gate_deltas: vec![MaterialSurfaceGateDelta {
            chart_id: 1,
            chunk_x: 0,
            chunk_y: 0,
            chunk_z: 0,
            cell_ordinal: 2,
            before_active: true,
            after_active: false,
            local_mana_before: 3,
            local_mana_after: 0,
            local_mana_transition_trace_id: TraceId::new(13),
            gate_transition_trace_id: TraceId::new(14),
            contact_trace_id: None,
            transition_tick: 10,
        }],
        thermal_delta_schema_version: 0,
        thermal_deltas: Vec::new(),
    };
    let mut encoded = encode_world_snapshot(&v3);
    assert_eq!(decode_world_snapshot(&encoded).unwrap(), v3);

    // Append a duplicate field 4 that downgrades the schema version to V2.
    field_varint(&mut encoded, 4, 2);
    assert_eq!(
        decode_world_snapshot(&encoded),
        Err(WireError::DuplicateField(4))
    );
}

#[cfg(test)]
#[test]
fn world_query_roundtrips_material_surface_thermal_deltas() {
    // Given: a chart-qualified material thermal exchange from a live observer read model.
    let expected = ObserverWorldSnapshot {
        time: SimulationTime::new(8),
        chunks: Vec::new(),
        material_surface_delta_schema_version: MATERIAL_SURFACE_DELTA_SCHEMA_V4,
        material_surface_deltas: Vec::new(),
        material_surface_gate_deltas: Vec::new(),
        material_surface_thermal_deltas: vec![MaterialSurfaceThermalDelta {
            chart_id: 5,
            chunk_x: -1,
            chunk_y: 2,
            chunk_z: -3,
            cell_ordinal: 7,
            before_retained: 20,
            after_retained: 24,
            cell_pre_state: 60,
            signed_flux: 4,
            thermal_exchange_trace_id: TraceId::new(30),
            transition_tick: 9,
        }],
        thermal_delta_schema_version: 0,
        thermal_deltas: Vec::new(),
    };
    let mut handler = ProtocolHandler::default();
    handler.set_world_snapshot(&expected);

    // When: a world-chunk query crosses the observer wire codec.
    let response = decode_response(
        &handler
            .handle_query(&encode_query(&ObserverQuery::world_chunks(20)))
            .unwrap(),
    )
    .unwrap();

    // Then: the new bounded delta round-trips without changing the query kind or shape.
    assert_eq!(response.status, QueryStatus::Ok);
    assert_eq!(decode_world_snapshot(&response.payload).unwrap(), expected);
}

#[cfg(test)]
#[test]
fn decode_world_snapshot_rejects_thermal_material_deltas_under_v3_schema() {
    let v3 = ObserverWorldSnapshot {
        time: SimulationTime::new(9),
        chunks: Vec::new(),
        material_surface_delta_schema_version: MATERIAL_SURFACE_DELTA_SCHEMA_V3,
        material_surface_deltas: Vec::new(),
        material_surface_gate_deltas: Vec::new(),
        material_surface_thermal_deltas: Vec::new(),
        thermal_delta_schema_version: 0,
        thermal_deltas: Vec::new(),
    };
    let mut encoded = encode_world_snapshot(&v3);
    assert_eq!(decode_world_snapshot(&encoded).unwrap(), v3);

    let thermal_delta = MaterialSurfaceThermalDelta {
        chart_id: 1,
        chunk_x: 0,
        chunk_y: 0,
        chunk_z: 0,
        cell_ordinal: 2,
        before_retained: 4,
        after_retained: 8,
        cell_pre_state: 50,
        signed_flux: 4,
        thermal_exchange_trace_id: TraceId::new(15),
        transition_tick: 10,
    };
    field_bytes(
        &mut encoded,
        8,
        &encode_material_surface_thermal_delta(&thermal_delta),
    );
    assert_eq!(
        decode_world_snapshot(&encoded),
        Err(WireError::UnexpectedFieldForSchema(8))
    );
}
