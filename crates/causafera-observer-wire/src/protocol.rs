use std::collections::{BTreeMap, BTreeSet};

use causafera_explanation::{
    ClaimConfidence, ClaimEvidenceState, ComparisonCohortId, ComparisonContext, ExplanationClaim,
    ExplanationClaimSchemaId, ExplanationFrame, ExplanationReport, FrameAssessment,
    NumericClaimValue,
};
use causafera_observer_api::{
    BOOTSTRAP_SUMMARY_SCHEMA_V1, FieldRasterKind, FieldRasterRequest,
    HYDROLOGY_CONVEYANCE_SUMMARY_SCHEMA_V1, HYDROLOGY_DELTA_SCHEMA_V1,
    HYDROLOGY_RASTER_VALUES_SCHEMA_V1, HYDROLOGY_SUMMARY_SCHEMA_ABSENT,
    HYDROLOGY_SUMMARY_SCHEMA_V1, HYDROLOGY_TRANSFER_SUMMARY_SCHEMA_V1, HydrologyCellDelta,
    HydrologyConveyanceSummary, HydrologyTransferSummary, MATERIAL_SURFACE_DELTA_SCHEMA_V3,
    MATERIAL_SURFACE_DELTA_SCHEMA_V4, MAX_BOOTSTRAP_RECEIPT_DEPENDENCIES,
    MAX_BOOTSTRAP_RECEIPT_SUMMARIES, MAX_HYDROLOGY_CONVEYANCE_SUMMARIES, MAX_HYDROLOGY_DELTAS,
    MAX_HYDROLOGY_TRANSFER_SUMMARIES, MAX_MATERIAL_SURFACE_DELTAS,
    MAX_QUERY_RESPONSE_PAYLOAD_BYTES, MAX_THERMAL_DELTAS, MaterialSurfaceDelta,
    MaterialSurfaceGateDelta, MaterialSurfaceThermalDelta, OBSERVER_PROTOCOL_V1,
    ObserverBootstrapReceipt, ObserverBootstrapSummary, ObserverChunkSummary, ObserverFieldRaster,
    ObserverHydrologyForcing, ObserverHydrologySummary, ObserverQuery, ObserverResponse,
    ObserverSnapshot, ObserverWorldSnapshot, QueryKind, QueryStatus, StreamEnvelope, StreamKind,
    THERMAL_DELTA_SCHEMA_V1, ThermalFieldDelta, validate_hydrology_carrier_key,
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
        encode_response(&response)
    }
}

/// Answer one query that carries parameters, which the payload cache cannot
/// serve because its answer depends on what was asked rather than on when.
pub fn encode_query_response(
    request_id: u64,
    status: QueryStatus,
    payload: Vec<u8>,
) -> Result<Vec<u8>, WireError> {
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

/// Serialize a response, refusing to emit one past the response cap.
///
/// Fallible where the request encoder is not, because this is the side that
/// produces the bytes: a bounded projection that outgrew its budget is a bug in
/// the producer, and a peer must never be the first to find out about it.
pub fn encode_response(response: &ObserverResponse) -> Result<Vec<u8>, WireError> {
    if response.payload.len() > MAX_QUERY_RESPONSE_PAYLOAD_BYTES {
        return Err(WireError::PayloadTooLarge);
    }
    let mut out = Vec::new();
    field_varint(&mut out, 1, response.request_id);
    field_varint(&mut out, 2, u64::from(response.protocol_version));
    field_varint(&mut out, 3, response.status as u64);
    if !response.payload.is_empty() {
        field_bytes(&mut out, 4, &response.payload);
    }
    Ok(out)
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
            (4, WIRE_LEN) => {
                // Bounded before the copy, not after: `bytes()` only reborrows
                // the input, and the response cap exists so a client never has
                // to allocate a payload it did not agree to receive.
                let bytes = cursor.bytes()?;
                if bytes.len() > MAX_QUERY_RESPONSE_PAYLOAD_BYTES {
                    return Err(WireError::PayloadTooLarge);
                }
                payload = bytes.to_vec();
            }
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
    // Field 48: the appended hydrology stage's receipt, optional and separately
    // bounded. Fields 31, 32, and 35 keep their frozen V1 meanings — a projected
    // six-stage count, six-stage completion, and at most six summaries — so a
    // frozen V1 decoder skips this field and reads exactly what it always did.
    if let Some(receipt) = &bootstrap.stage_seven {
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
        field_bytes(&mut out, 48, &nested);
    }
    encode_hydrology_summary(&mut out, &snapshot.hydrology);
    out
}

/// Write fields 36..=47, or nothing at all.
///
/// Nothing at all only for a summary that decoded from a pre-hydrology payload:
/// a live session writes the whole group even when it holds no water, because
/// "this build has no hydrology" and "this world has none" are different facts
/// and a reader that cannot tell them apart cannot report either.
fn encode_hydrology_summary(out: &mut Vec<u8>, hydrology: &ObserverHydrologySummary) {
    if hydrology.schema_version == HYDROLOGY_SUMMARY_SCHEMA_ABSENT {
        return;
    }
    field_varint(out, 36, u64::from(hydrology.schema_version));
    field_bytes(out, 37, &encode_u128(hydrology.total_surface));
    field_bytes(out, 38, &encode_u128(hydrology.total_soil));
    field_bytes(out, 39, &encode_u128(hydrology.total_groundwater));
    field_bytes(out, 40, &encode_u128(hydrology.total_conveyance));
    field_bytes(out, 41, &encode_i128_zigzag(hydrology.latest_residual));
    field_varint(out, 42, u64::from(hydrology.active_chunk_count));
    if let Some(forcing) = &hydrology.latest_forcing {
        field_varint(out, 43, forcing.tick);
        field_varint(out, 44, forcing.forcing_id);
        field_varint(out, 45, forcing.origin_trace.raw());
        field_bytes(out, 46, &encode_u128(forcing.accepted_source));
        field_bytes(out, 47, &encode_u64(forcing.accepted_et));
    }
}

/// The scalars and byte integers fields 36..=47 carry, before grouping.
///
/// Collected rather than assigned directly so presence can be judged for the
/// group as a whole: half a summary is not a smaller summary, and the decision
/// about what half means belongs in one place.
#[derive(Default)]
struct HydrologyGroupFields {
    schema_version: Option<u64>,
    total_surface: Option<u128>,
    total_soil: Option<u128>,
    total_groundwater: Option<u128>,
    total_conveyance: Option<u128>,
    latest_residual: Option<i128>,
    active_chunk_count: Option<u64>,
    forcing_tick: Option<u64>,
    forcing_id: Option<u64>,
    forcing_origin: Option<u64>,
    accepted_source: Option<u128>,
    accepted_et: Option<u64>,
}

impl HydrologyGroupFields {
    /// Which of fields 36..=47 arrived, in field order.
    fn present(&self) -> [bool; 12] {
        [
            self.schema_version.is_some(),
            self.total_surface.is_some(),
            self.total_soil.is_some(),
            self.total_groundwater.is_some(),
            self.total_conveyance.is_some(),
            self.latest_residual.is_some(),
            self.active_chunk_count.is_some(),
            self.forcing_tick.is_some(),
            self.forcing_id.is_some(),
            self.forcing_origin.is_some(),
            self.accepted_source.is_some(),
            self.accepted_et.is_some(),
        ]
    }
}

/// Decode fields 36..=47 as one required group and one optional subgroup.
///
/// Absence is tolerated only wholesale: a payload carrying none of 36..=47 was
/// written before hydrology existed. Every other incompleteness fails closed,
/// because a partially present group is not an older peer — it is a summary
/// whose missing halves would otherwise be filled in with zeroes and read as
/// measurements.
fn decode_hydrology_summary(
    fields: &HydrologyGroupFields,
) -> Result<ObserverHydrologySummary, WireError> {
    let present = fields.present();
    let required = &present[0..7];
    let forcing = &present[7..12];
    if !required.iter().any(|seen| *seen) {
        if forcing.iter().any(|seen| *seen) {
            // A forcing record with no summary to attribute it to.
            return Err(WireError::MissingField(36));
        }
        return Ok(ObserverHydrologySummary::default());
    }
    for (offset, seen) in required.iter().enumerate() {
        if !seen {
            return Err(WireError::MissingField(36 + offset as u32));
        }
    }
    let schema_version = to_u32(fields.schema_version.unwrap_or_default())?;
    if schema_version != HYDROLOGY_SUMMARY_SCHEMA_V1 {
        return Err(WireError::UnexpectedFieldForSchema(schema_version));
    }
    let latest_forcing = if forcing.iter().all(|seen| *seen) {
        Some(ObserverHydrologyForcing {
            tick: fields.forcing_tick.unwrap_or_default(),
            forcing_id: fields.forcing_id.unwrap_or_default(),
            origin_trace: TraceId::new(fields.forcing_origin.unwrap_or_default()),
            accepted_source: fields.accepted_source.unwrap_or_default(),
            accepted_et: fields.accepted_et.unwrap_or_default(),
        })
    } else if forcing.iter().any(|seen| *seen) {
        let offset = forcing
            .iter()
            .position(|seen| !seen)
            .expect("a partially present group has a missing member");
        return Err(WireError::MissingField(43 + offset as u32));
    } else {
        None
    };
    Ok(ObserverHydrologySummary {
        schema_version,
        total_surface: fields.total_surface.unwrap_or_default(),
        total_soil: fields.total_soil.unwrap_or_default(),
        total_groundwater: fields.total_groundwater.unwrap_or_default(),
        total_conveyance: fields.total_conveyance.unwrap_or_default(),
        latest_residual: fields.latest_residual.unwrap_or_default(),
        active_chunk_count: to_u32(fields.active_chunk_count.unwrap_or_default())?,
        latest_forcing,
    })
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
    stage_seven: Option<ObserverBootstrapReceipt>,
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
        stage_seven,
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
    let mut bootstrap_stage_seven = None;
    let mut hydrology = HydrologyGroupFields::default();
    while !c.is_empty() {
        let (field, wire) = c.key()?;
        match (field, wire) {
            (3, WIRE_LEN) => physical_digest = Some(array32(c.bytes()?)?),
            (4, WIRE_LEN) => history_digest = Some(array32(c.bytes()?)?),
            (24, WIRE_LEN) => thermal_total_cell_energy = decode_i128_zigzag(c.bytes()?)?,
            (25, WIRE_LEN) => thermal_total_reservoir_budget = decode_i128_zigzag(c.bytes()?)?,
            // Fields 36..=47 are the hydrology group. Every one of them is
            // single-valued: two different water totals in one payload is a
            // contradiction rather than a later value winning, exactly as for the
            // bootstrap group above.
            (36, WIRE_VARINT) => set_once(&mut hydrology.schema_version, 36, c.varint()?)?,
            (37, WIRE_LEN) => set_once(&mut hydrology.total_surface, 37, decode_u128(c.bytes()?)?)?,
            (38, WIRE_LEN) => set_once(&mut hydrology.total_soil, 38, decode_u128(c.bytes()?)?)?,
            (39, WIRE_LEN) => set_once(
                &mut hydrology.total_groundwater,
                39,
                decode_u128(c.bytes()?)?,
            )?,
            (40, WIRE_LEN) => set_once(
                &mut hydrology.total_conveyance,
                40,
                decode_u128(c.bytes()?)?,
            )?,
            (41, WIRE_LEN) => set_once(
                &mut hydrology.latest_residual,
                41,
                decode_i128_zigzag_canonical(c.bytes()?)?,
            )?,
            (42, WIRE_VARINT) => set_once(&mut hydrology.active_chunk_count, 42, c.varint()?)?,
            (43, WIRE_VARINT) => set_once(&mut hydrology.forcing_tick, 43, c.varint()?)?,
            (44, WIRE_VARINT) => set_once(&mut hydrology.forcing_id, 44, c.varint()?)?,
            (45, WIRE_VARINT) => set_once(&mut hydrology.forcing_origin, 45, c.varint()?)?,
            (46, WIRE_LEN) => {
                set_once(&mut hydrology.accepted_source, 46, decode_u128(c.bytes()?)?)?
            }
            (47, WIRE_LEN) => set_once(&mut hydrology.accepted_et, 47, decode_u64(c.bytes()?)?)?,
            // A hydrology field on the wrong wire type is malformed, not unknown:
            // skipping it would let a summary whose every member is mistyped fall
            // through to "this payload predates hydrology".
            (36..=47, _) => return Err(WireError::UnexpectedFieldForSchema(field)),
            (35, WIRE_LEN) => {
                // Bounded before allocation: a payload claiming more receipts
                // than this build's bootstrap can produce is rejected rather
                // than truncated.
                if bootstrap_receipts.len() == MAX_BOOTSTRAP_RECEIPT_SUMMARIES {
                    return Err(WireError::PayloadTooLarge);
                }
                bootstrap_receipts.push(decode_bootstrap_receipt(c.bytes()?)?);
            }
            (48, WIRE_LEN) => {
                // Separately bounded: exactly one appended stage, so a second
                // occurrence is a payload describing two seventh stages.
                if bootstrap_stage_seven.is_some() {
                    return Err(WireError::DuplicateField(48));
                }
                bootstrap_stage_seven = Some(decode_bootstrap_receipt(c.bytes()?)?);
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
    let bootstrap =
        decode_bootstrap_summary(&values, &present, bootstrap_receipts, bootstrap_stage_seven)?;
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
        hydrology: decode_hydrology_summary(&hydrology)?,
    })
}

/// Record a single-valued field, refusing a second occurrence.
fn set_once<T>(slot: &mut Option<T>, field: u32, value: T) -> Result<(), WireError> {
    if slot.is_some() {
        return Err(WireError::DuplicateField(field));
    }
    *slot = Some(value);
    Ok(())
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
    if snapshot.hydrology_delta_schema_version >= HYDROLOGY_DELTA_SCHEMA_V1 {
        for delta in snapshot.hydrology_deltas.iter().take(MAX_HYDROLOGY_DELTAS) {
            field_bytes(&mut out, 9, &encode_hydrology_cell_delta(delta));
        }
    }
    if snapshot.hydrology_delta_schema_version != 0 {
        field_varint(
            &mut out,
            10,
            u64::from(snapshot.hydrology_delta_schema_version),
        );
    }
    if snapshot.hydrology_transfer_schema_version >= HYDROLOGY_TRANSFER_SUMMARY_SCHEMA_V1 {
        for summary in snapshot
            .hydrology_transfer_summaries
            .iter()
            .take(MAX_HYDROLOGY_TRANSFER_SUMMARIES)
        {
            field_bytes(&mut out, 11, &encode_hydrology_transfer_summary(summary));
        }
    }
    if snapshot.hydrology_transfer_schema_version != 0 {
        field_varint(
            &mut out,
            12,
            u64::from(snapshot.hydrology_transfer_schema_version),
        );
    }
    if snapshot.hydrology_conveyance_schema_version >= HYDROLOGY_CONVEYANCE_SUMMARY_SCHEMA_V1 {
        for summary in snapshot
            .hydrology_conveyance_summaries
            .iter()
            .take(MAX_HYDROLOGY_CONVEYANCE_SUMMARIES)
        {
            field_bytes(&mut out, 13, &encode_hydrology_conveyance_summary(summary));
        }
    }
    if snapshot.hydrology_conveyance_schema_version != 0 {
        field_varint(
            &mut out,
            14,
            u64::from(snapshot.hydrology_conveyance_schema_version),
        );
    }
    out
}

fn encode_hydrology_cell_delta(delta: &HydrologyCellDelta) -> Vec<u8> {
    let mut nested = Vec::with_capacity(96);
    field_varint(&mut nested, 1, delta.chart_id);
    field_varint(&mut nested, 2, zigzag(i64::from(delta.chunk_x)));
    field_varint(&mut nested, 3, zigzag(i64::from(delta.chunk_y)));
    field_varint(&mut nested, 4, zigzag(i64::from(delta.chunk_z)));
    field_varint(&mut nested, 5, u64::from(delta.cell_ordinal));
    for (field, volume) in [
        (6, delta.surface_before),
        (7, delta.surface_after),
        (8, delta.soil_before),
        (9, delta.soil_after),
        (10, delta.groundwater_before),
        (11, delta.groundwater_after),
    ] {
        field_bytes(&mut nested, field, &encode_u64(volume));
    }
    field_bytes(&mut nested, 12, &encode_i128_zigzag(delta.net_forcing));
    field_bytes(&mut nested, 13, &encode_i128_zigzag(delta.net_lateral_flow));
    field_varint(&mut nested, 14, delta.transition_trace.raw());
    field_varint(&mut nested, 15, delta.conservation_trace.raw());
    field_varint(&mut nested, 16, delta.transition_tick);
    nested
}

fn decode_hydrology_cell_delta(bytes: &[u8]) -> Result<HydrologyCellDelta, WireError> {
    let mut c = Cursor::new(bytes);
    let mut scalars: [Option<u64>; 5] = [None; 5];
    let mut volumes: [Option<u64>; 6] = [None; 6];
    let mut net_forcing = None;
    let mut net_lateral_flow = None;
    let mut traces: [Option<u64>; 3] = [None; 3];
    while !c.is_empty() {
        let (field, wire) = c.key()?;
        match (field, wire) {
            (1..=5, WIRE_VARINT) => set_once(&mut scalars[field as usize - 1], field, c.varint()?)?,
            (6..=11, WIRE_LEN) => set_once(
                &mut volumes[field as usize - 6],
                field,
                decode_u64(c.bytes()?)?,
            )?,
            (12, WIRE_LEN) => set_once(
                &mut net_forcing,
                12,
                decode_i128_zigzag_canonical(c.bytes()?)?,
            )?,
            (13, WIRE_LEN) => set_once(
                &mut net_lateral_flow,
                13,
                decode_i128_zigzag_canonical(c.bytes()?)?,
            )?,
            (14..=16, WIRE_VARINT) => {
                set_once(&mut traces[field as usize - 14], field, c.varint()?)?
            }
            (1..=16, _) => return Err(WireError::UnexpectedFieldForSchema(field)),
            _ => c.skip(wire)?,
        }
    }
    for (index, value) in scalars.iter().enumerate() {
        if value.is_none() {
            return Err(WireError::MissingField(index as u32 + 1));
        }
    }
    for (index, value) in volumes.iter().enumerate() {
        if value.is_none() {
            return Err(WireError::MissingField(index as u32 + 6));
        }
    }
    for (index, value) in traces.iter().enumerate() {
        if value.is_none() {
            return Err(WireError::MissingField(index as u32 + 14));
        }
    }
    let scalar = |index: usize| scalars[index].unwrap_or_default();
    let volume = |index: usize| volumes[index].unwrap_or_default();
    let trace = |index: usize| TraceId::new(traces[index].unwrap_or_default());
    Ok(HydrologyCellDelta {
        chart_id: scalar(0),
        chunk_x: to_i32(unzigzag(scalar(1)))?,
        chunk_y: to_i32(unzigzag(scalar(2)))?,
        chunk_z: to_i32(unzigzag(scalar(3)))?,
        cell_ordinal: u16::try_from(scalar(4)).map_err(|_| WireError::IntegerOverflow)?,
        surface_before: volume(0),
        surface_after: volume(1),
        soil_before: volume(2),
        soil_after: volume(3),
        groundwater_before: volume(4),
        groundwater_after: volume(5),
        net_forcing: net_forcing.ok_or(WireError::MissingField(12))?,
        net_lateral_flow: net_lateral_flow.ok_or(WireError::MissingField(13))?,
        transition_trace: trace(0),
        conservation_trace: trace(1),
        transition_tick: traces[2].unwrap_or_default(),
    })
}

fn encode_hydrology_transfer_summary(summary: &HydrologyTransferSummary) -> Vec<u8> {
    let mut nested = Vec::with_capacity(128);
    field_varint(&mut nested, 1, u64::from(summary.process_kind));
    field_bytes(&mut nested, 2, &summary.source_key);
    field_bytes(&mut nested, 3, &summary.target_key);
    field_bytes(&mut nested, 4, &encode_u64(summary.requested_volume));
    field_bytes(&mut nested, 5, &encode_u64(summary.accepted_volume));
    field_bytes(&mut nested, 6, &encode_u64(summary.unaccepted_volume));
    field_varint(&mut nested, 7, summary.transfer_trace.raw());
    field_varint(&mut nested, 8, summary.conservation_trace.raw());
    field_varint(&mut nested, 9, summary.tick);
    if let Some(origin) = summary.forcing_origin_trace {
        field_varint(&mut nested, 10, origin.raw());
    }
    nested
}

fn decode_hydrology_transfer_summary(bytes: &[u8]) -> Result<HydrologyTransferSummary, WireError> {
    let mut c = Cursor::new(bytes);
    let mut process_kind = None;
    let mut source_key = None;
    let mut target_key = None;
    let mut volumes: [Option<u64>; 3] = [None; 3];
    let mut traces: [Option<u64>; 3] = [None; 3];
    let mut forcing_origin_trace = None;
    while !c.is_empty() {
        let (field, wire) = c.key()?;
        match (field, wire) {
            (1, WIRE_VARINT) => set_once(&mut process_kind, 1, c.varint()?)?,
            (2, WIRE_LEN) => set_once(&mut source_key, 2, c.bytes()?.to_vec())?,
            (3, WIRE_LEN) => set_once(&mut target_key, 3, c.bytes()?.to_vec())?,
            (4..=6, WIRE_LEN) => set_once(
                &mut volumes[field as usize - 4],
                field,
                decode_u64(c.bytes()?)?,
            )?,
            (7..=9, WIRE_VARINT) => set_once(&mut traces[field as usize - 7], field, c.varint()?)?,
            (10, WIRE_VARINT) => set_once(&mut forcing_origin_trace, 10, c.varint()?)?,
            (1..=10, _) => return Err(WireError::UnexpectedFieldForSchema(field)),
            _ => c.skip(wire)?,
        }
    }
    let source_key = source_key.ok_or(WireError::MissingField(2))?;
    let target_key = target_key.ok_or(WireError::MissingField(3))?;
    let source_variant = validate_hydrology_carrier_key(&source_key)?;
    validate_hydrology_carrier_key(&target_key)?;
    // A cell is the one carrier a transfer may name twice: infiltration,
    // percolation, and evapotranspiration move water between buckets *inside*
    // one cell, and the buckets are not part of the key. Every other carrier is
    // a single store or a single face, so naming it as both ends describes a
    // transfer to nowhere. Judging that on the carrier's own structure rather
    // than on a table of process meanings keeps the wire out of the business of
    // classifying simulation processes.
    if source_key == target_key && source_variant != causafera_observer_api::HYDROLOGY_CARRIER_CELL
    {
        return Err(WireError::HydrologyCarrierNotDistinct);
    }
    for (index, value) in volumes.iter().enumerate() {
        if value.is_none() {
            return Err(WireError::MissingField(index as u32 + 4));
        }
    }
    for (index, value) in traces.iter().enumerate() {
        if value.is_none() {
            return Err(WireError::MissingField(index as u32 + 7));
        }
    }
    let requested_volume = volumes[0].unwrap_or_default();
    let accepted_volume = volumes[1].unwrap_or_default();
    let unaccepted_volume = volumes[2].unwrap_or_default();
    // The three volumes are one statement, not three: a payload where they do
    // not close has either invented water or lost some, and the limiter evidence
    // it claims to carry would be a fabrication.
    if requested_volume
        .checked_sub(accepted_volume)
        .is_none_or(|remainder| remainder != unaccepted_volume)
    {
        return Err(WireError::InconsistentHydrologyTransfer);
    }
    Ok(HydrologyTransferSummary {
        process_kind: to_u32(process_kind.ok_or(WireError::MissingField(1))?)?,
        source_key,
        target_key,
        requested_volume,
        accepted_volume,
        unaccepted_volume,
        transfer_trace: TraceId::new(traces[0].unwrap_or_default()),
        conservation_trace: TraceId::new(traces[1].unwrap_or_default()),
        tick: traces[2].unwrap_or_default(),
        forcing_origin_trace: forcing_origin_trace.map(TraceId::new),
    })
}

fn encode_hydrology_conveyance_summary(summary: &HydrologyConveyanceSummary) -> Vec<u8> {
    let mut nested = Vec::with_capacity(96);
    field_bytes(&mut nested, 1, &summary.edge_key);
    field_bytes(&mut nested, 2, &encode_u64(summary.storage));
    field_bytes(&mut nested, 3, &encode_u64(summary.capacity));
    field_bytes(&mut nested, 4, &encode_u64(summary.accepted_inflow));
    field_bytes(&mut nested, 5, &encode_u64(summary.accepted_release));
    field_varint(&mut nested, 6, summary.last_change_trace.raw());
    field_varint(&mut nested, 7, summary.tick);
    nested
}

fn decode_hydrology_conveyance_summary(
    bytes: &[u8],
) -> Result<HydrologyConveyanceSummary, WireError> {
    let mut c = Cursor::new(bytes);
    let mut edge_key = None;
    let mut volumes: [Option<u64>; 4] = [None; 4];
    let mut last_change_trace = None;
    let mut tick = None;
    while !c.is_empty() {
        let (field, wire) = c.key()?;
        match (field, wire) {
            (1, WIRE_LEN) => set_once(&mut edge_key, 1, c.bytes()?.to_vec())?,
            (2..=5, WIRE_LEN) => set_once(
                &mut volumes[field as usize - 2],
                field,
                decode_u64(c.bytes()?)?,
            )?,
            (6, WIRE_VARINT) => set_once(&mut last_change_trace, 6, c.varint()?)?,
            (7, WIRE_VARINT) => set_once(&mut tick, 7, c.varint()?)?,
            (1..=7, _) => return Err(WireError::UnexpectedFieldForSchema(field)),
            _ => c.skip(wire)?,
        }
    }
    let edge_key = edge_key.ok_or(WireError::MissingField(1))?;
    // A conveyance summary describes one edge. Any other carrier here would be
    // a storage-and-discharge report about something that has neither.
    if validate_hydrology_carrier_key(&edge_key)? != causafera_observer_api::HYDROLOGY_CARRIER_EDGE
    {
        return Err(WireError::UnexpectedFieldForSchema(1));
    }
    for (index, value) in volumes.iter().enumerate() {
        if value.is_none() {
            return Err(WireError::MissingField(index as u32 + 2));
        }
    }
    Ok(HydrologyConveyanceSummary {
        edge_key,
        storage: volumes[0].unwrap_or_default(),
        capacity: volumes[1].unwrap_or_default(),
        accepted_inflow: volumes[2].unwrap_or_default(),
        accepted_release: volumes[3].unwrap_or_default(),
        last_change_trace: TraceId::new(last_change_trace.ok_or(WireError::MissingField(6))?),
        tick: tick.ok_or(WireError::MissingField(7))?,
    })
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
    let mut hydrology_delta_bytes: Vec<Vec<u8>> = Vec::new();
    let mut hydrology_transfer_bytes: Vec<Vec<u8>> = Vec::new();
    let mut hydrology_conveyance_bytes: Vec<Vec<u8>> = Vec::new();
    let mut hydrology_delta_schema_version = 0;
    let mut hydrology_delta_schema_seen = false;
    let mut hydrology_transfer_schema_version = 0;
    let mut hydrology_transfer_schema_seen = false;
    let mut hydrology_conveyance_schema_version = 0;
    let mut hydrology_conveyance_schema_seen = false;
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
            // Hydrology's three bounded lists reject at `limit + 1` rather than
            // skipping past it. The older lists above silently drop the excess,
            // which reports a truncated projection as a complete one; a bound
            // that a peer can exceed without being told is not a bound.
            (9, WIRE_LEN) => {
                if hydrology_delta_bytes.len() == MAX_HYDROLOGY_DELTAS {
                    return Err(WireError::PayloadTooLarge);
                }
                hydrology_delta_bytes.push(cursor.bytes()?.to_vec());
            }
            (10, WIRE_VARINT) if !hydrology_delta_schema_seen => {
                hydrology_delta_schema_version = to_u32(cursor.varint()?)?;
                hydrology_delta_schema_seen = true;
            }
            (10, _) => return Err(WireError::DuplicateField(10)),
            (11, WIRE_LEN) => {
                if hydrology_transfer_bytes.len() == MAX_HYDROLOGY_TRANSFER_SUMMARIES {
                    return Err(WireError::PayloadTooLarge);
                }
                hydrology_transfer_bytes.push(cursor.bytes()?.to_vec());
            }
            (12, WIRE_VARINT) if !hydrology_transfer_schema_seen => {
                hydrology_transfer_schema_version = to_u32(cursor.varint()?)?;
                hydrology_transfer_schema_seen = true;
            }
            (12, _) => return Err(WireError::DuplicateField(12)),
            (13, WIRE_LEN) => {
                if hydrology_conveyance_bytes.len() == MAX_HYDROLOGY_CONVEYANCE_SUMMARIES {
                    return Err(WireError::PayloadTooLarge);
                }
                hydrology_conveyance_bytes.push(cursor.bytes()?.to_vec());
            }
            (14, WIRE_VARINT) if !hydrology_conveyance_schema_seen => {
                hydrology_conveyance_schema_version = to_u32(cursor.varint()?)?;
                hydrology_conveyance_schema_seen = true;
            }
            (14, _) => return Err(WireError::DuplicateField(14)),
            (9 | 11 | 13, _) => return Err(WireError::UnexpectedFieldForSchema(field)),
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
    // Entries with no schema to interpret them are parsed and then read under a
    // contract the payload never declared.
    if !hydrology_delta_bytes.is_empty()
        && hydrology_delta_schema_version < HYDROLOGY_DELTA_SCHEMA_V1
    {
        return Err(WireError::UnexpectedFieldForSchema(9));
    }
    if !hydrology_transfer_bytes.is_empty()
        && hydrology_transfer_schema_version < HYDROLOGY_TRANSFER_SUMMARY_SCHEMA_V1
    {
        return Err(WireError::UnexpectedFieldForSchema(11));
    }
    if !hydrology_conveyance_bytes.is_empty()
        && hydrology_conveyance_schema_version < HYDROLOGY_CONVEYANCE_SUMMARY_SCHEMA_V1
    {
        return Err(WireError::UnexpectedFieldForSchema(13));
    }
    let hydrology_deltas = hydrology_delta_bytes
        .iter()
        .map(|bytes| decode_hydrology_cell_delta(bytes))
        .collect::<Result<Vec<_>, _>>()?;
    let hydrology_transfer_summaries = hydrology_transfer_bytes
        .iter()
        .map(|bytes| decode_hydrology_transfer_summary(bytes))
        .collect::<Result<Vec<_>, _>>()?;
    let hydrology_conveyance_summaries = hydrology_conveyance_bytes
        .iter()
        .map(|bytes| decode_hydrology_conveyance_summary(bytes))
        .collect::<Result<Vec<_>, _>>()?;
    require_distinct_hydrology_keys(
        &hydrology_deltas,
        &hydrology_transfer_summaries,
        &hydrology_conveyance_summaries,
    )?;
    Ok(ObserverWorldSnapshot {
        time: time.ok_or(WireError::MissingField(1))?,
        chunks,
        material_surface_delta_schema_version,
        material_surface_deltas,
        material_surface_gate_deltas,
        material_surface_thermal_deltas,
        thermal_delta_schema_version,
        thermal_deltas,
        hydrology_deltas,
        hydrology_delta_schema_version,
        hydrology_transfer_summaries,
        hydrology_transfer_schema_version,
        hydrology_conveyance_summaries,
        hydrology_conveyance_schema_version,
    })
}

/// Refuse a projection that describes the same thing twice.
///
/// A duplicate is not a redundant row. Two deltas for one cell in one tick
/// disagree about what that cell did, and a reader summing accepted volumes
/// over a repeated transfer counts water that moved once as water that moved
/// twice.
fn require_distinct_hydrology_keys(
    deltas: &[HydrologyCellDelta],
    transfers: &[HydrologyTransferSummary],
    conveyance: &[HydrologyConveyanceSummary],
) -> Result<(), WireError> {
    let mut cells = BTreeSet::new();
    for delta in deltas {
        let key = (
            delta.transition_tick,
            delta.chart_id,
            delta.chunk_x,
            delta.chunk_y,
            delta.chunk_z,
            delta.cell_ordinal,
        );
        if !cells.insert(key) {
            return Err(WireError::DuplicateKey);
        }
    }
    let mut keys = BTreeSet::new();
    for summary in transfers {
        if !keys.insert(summary.canonical_key()) {
            return Err(WireError::DuplicateKey);
        }
    }
    let mut edges = BTreeSet::new();
    for summary in conveyance {
        if !edges.insert((summary.tick, summary.edge_key.as_slice())) {
            return Err(WireError::DuplicateKey);
        }
    }
    Ok(())
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
    if raster.unsigned_values_schema_version != 0 {
        let mut packed = Vec::new();
        for value in &raster.unsigned_values {
            varint(&mut packed, *value);
        }
        field_bytes(&mut out, 13, &packed);
        field_varint(
            &mut out,
            14,
            u64::from(raster.unsigned_values_schema_version),
        );
    }
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
    let mut unsigned_values = Vec::new();
    let mut unsigned_values_schema_version = 0_u32;
    let mut unsigned_seen = false;
    let mut unsigned_schema_seen = false;
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
            (13, WIRE_LEN) => {
                if unsigned_seen {
                    return Err(WireError::DuplicateField(13));
                }
                unsigned_seen = true;
                let mut packed = Cursor::new(cursor.bytes()?);
                while !packed.is_empty() {
                    unsigned_values.push(packed.varint_shortest()?);
                }
            }
            (14, WIRE_VARINT) => {
                if unsigned_schema_seen {
                    return Err(WireError::DuplicateField(14));
                }
                unsigned_schema_seen = true;
                unsigned_values_schema_version = to_u32(cursor.varint()?)?;
            }
            (13 | 14, _) => return Err(WireError::UnexpectedFieldForSchema(number)),
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
        unsigned_values,
        unsigned_values_schema_version,
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
    // The two bands are mutually exclusive, and which one a raster carries is
    // decided by its kind rather than by what arrived: a hydrology lattice in the
    // signed band would have wrapped every volume above `i64::MAX`, and any other
    // lattice in the unsigned one would have lost every negative elevation.
    if raster.field.carries_unsigned_values() {
        if unsigned_schema_seen
            && unsigned_values_schema_version != HYDROLOGY_RASTER_VALUES_SCHEMA_V1
        {
            return Err(WireError::UnexpectedFieldForSchema(14));
        }
        if !unsigned_schema_seen {
            return Err(WireError::MissingField(14));
        }
        if !raster.values.is_empty() || !raster.auxiliary.is_empty() {
            return Err(WireError::UnexpectedFieldForSchema(9));
        }
        if raster.unsigned_values.len() != cell_count {
            return Err(WireError::InvalidFieldRasterLattice {
                expected: cell_count,
                received: raster.unsigned_values.len(),
            });
        }
    } else {
        if unsigned_seen || unsigned_schema_seen {
            return Err(WireError::UnexpectedFieldForSchema(13));
        }
        if raster.values.len() != cell_count {
            return Err(WireError::InvalidFieldRasterLattice {
                expected: cell_count,
                received: raster.values.len(),
            });
        }
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

fn encode_u128(value: u128) -> Vec<u8> {
    let mut bytes = Vec::new();
    varint_u128(&mut bytes, value);
    bytes
}

fn encode_u64(value: u64) -> Vec<u8> {
    let mut bytes = Vec::new();
    varint(&mut bytes, value);
    bytes
}

/// Reject a length-delimited LEB128 integer that is not in shortest form.
///
/// A byte integer that admits several encodings admits several byte strings for
/// one payload, and the digest of a payload is an identity. The check is one
/// byte wide: an encoding whose last byte is `0x00` completed one byte earlier,
/// and the only value whose shortest form ends in `0x00` is zero itself.
fn require_shortest_form(bytes: &[u8]) -> Result<(), WireError> {
    match bytes.last() {
        None => Err(WireError::InvalidVarint),
        Some(0) if bytes.len() > 1 => Err(WireError::NonCanonicalInteger),
        Some(byte) if byte & 0x80 != 0 => Err(WireError::InvalidVarint),
        Some(_) => Ok(()),
    }
}

fn decode_u128(bytes: &[u8]) -> Result<u128, WireError> {
    require_shortest_form(bytes)?;
    let mut value = 0_u128;
    for (index, byte) in bytes.iter().copied().enumerate() {
        let shift = index
            .checked_mul(7)
            .and_then(|shift| u32::try_from(shift).ok())
            .ok_or(WireError::IntegerOverflow)?;
        if shift >= 128 {
            return Err(WireError::InvalidVarint);
        }
        let part = u128::from(byte & 0x7f);
        if part > (u128::MAX >> shift) {
            return Err(WireError::IntegerOverflow);
        }
        value |= part << shift;
        if byte & 0x80 == 0 {
            if index + 1 != bytes.len() {
                return Err(WireError::InvalidVarint);
            }
            return Ok(value);
        }
    }
    Err(WireError::InvalidVarint)
}

fn decode_u64(bytes: &[u8]) -> Result<u64, WireError> {
    require_shortest_form(bytes)?;
    let mut value = 0_u64;
    for (index, byte) in bytes.iter().copied().enumerate() {
        let shift = index
            .checked_mul(7)
            .and_then(|shift| u32::try_from(shift).ok())
            .ok_or(WireError::IntegerOverflow)?;
        if shift >= 64 {
            return Err(WireError::InvalidVarint);
        }
        let part = u64::from(byte & 0x7f);
        if part > (u64::MAX >> shift) {
            return Err(WireError::IntegerOverflow);
        }
        value |= part << shift;
        if byte & 0x80 == 0 {
            if index + 1 != bytes.len() {
                return Err(WireError::InvalidVarint);
            }
            return Ok(value);
        }
    }
    Err(WireError::InvalidVarint)
}

/// The thermal totals in fields 24 and 25 predate the shortest-form rule and
/// keep their existing tolerance; every hydrology byte integer decodes through
/// this instead, so a second encoding of the same water total is not accepted.
fn decode_i128_zigzag_canonical(bytes: &[u8]) -> Result<i128, WireError> {
    require_shortest_form(bytes)?;
    decode_i128_zigzag(bytes)
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
    /// A varint that must be in shortest canonical form.
    ///
    /// Used for the packed unsigned raster band, where a redundant continuation
    /// byte would give one lattice more than one byte string and make the same
    /// measurement hash two ways.
    fn varint_shortest(&mut self) -> Result<u64, WireError> {
        let start = self.at;
        let value = self.varint()?;
        let encoded = &self.bytes[start..self.at];
        require_shortest_form(encoded)?;
        Ok(value)
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
    #[error("a byte integer is not in shortest canonical form")]
    NonCanonicalInteger,
    #[error("a bounded observer projection carries the same key twice")]
    DuplicateKey,
    #[error("a hydrology transfer names the same carrier as source and target")]
    HydrologyCarrierNotDistinct,
    #[error("a hydrology transfer's requested, accepted, and unaccepted volumes do not close")]
    InconsistentHydrologyTransfer,
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
                stage_seven: None,
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
            hydrology: ObserverHydrologySummary::default(),
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
            unsigned_values: Vec::new(),
            unsigned_values_schema_version: 0,
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
            hydrology_deltas: Vec::new(),
            hydrology_delta_schema_version: 0,
            hydrology_transfer_summaries: Vec::new(),
            hydrology_transfer_schema_version: 0,
            hydrology_conveyance_summaries: Vec::new(),
            hydrology_conveyance_schema_version: 0,
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
            hydrology_deltas: Vec::new(),
            hydrology_delta_schema_version: 0,
            hydrology_transfer_summaries: Vec::new(),
            hydrology_transfer_schema_version: 0,
            hydrology_conveyance_summaries: Vec::new(),
            hydrology_conveyance_schema_version: 0,
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

    #[test]
    fn the_appended_bootstrap_stage_travels_in_its_own_optional_field() {
        // Field 48 is additive: a payload carrying it keeps fields 31, 32, and 35
        // at their frozen V1 meanings — a projected six-stage count, six-stage
        // completion, and at most six summaries — so a frozen decoder reads
        // exactly what it always did and skips the rest.
        let mut original = snapshot();
        let seventh = ObserverBootstrapReceipt {
            stage: 7,
            completed_at: SimulationTime::new(7),
            result: [9_u8; 32],
            completion_trace: TraceId::new(4_242),
            dependency_traces: vec![TraceId::new(4_241)],
        };
        original.bootstrap.stage_seven = Some(seventh.clone());

        let encoded = encode_observer_snapshot(&original);
        let decoded = decode_observer_snapshot(&encoded).expect("the payload must decode");
        assert_eq!(decoded.bootstrap.stage_seven, Some(seventh));
        assert_eq!(
            decoded.bootstrap.stage_count, original.bootstrap.stage_count,
            "the projected stage count is unchanged"
        );
        assert_eq!(decoded.bootstrap.receipts, original.bootstrap.receipts);
        assert_eq!(decoded, original);

        // A payload without the field decodes to `None` rather than to an
        // invented seventh stage, and its bytes are a strict prefix-free subset:
        // the only difference is the absent field.
        let mut without = original.clone();
        without.bootstrap.stage_seven = None;
        let shorter = encode_observer_snapshot(&without);
        assert!(shorter.len() < encoded.len());
        assert_eq!(
            decode_observer_snapshot(&shorter)
                .expect("the payload must decode")
                .bootstrap
                .stage_seven,
            None
        );
    }

    #[test]
    fn two_appended_bootstrap_stages_in_one_payload_are_refused() {
        // Exactly one appended stage exists, so a repeated field 48 describes two
        // seventh stages.
        let mut original = snapshot();
        original.bootstrap.stage_seven = Some(ObserverBootstrapReceipt {
            stage: 7,
            completed_at: SimulationTime::new(7),
            result: [1_u8; 32],
            completion_trace: TraceId::new(11),
            dependency_traces: Vec::new(),
        });
        let encoded = encode_observer_snapshot(&original);
        let mut without = original.clone();
        without.bootstrap.stage_seven = None;
        let baseline = encode_observer_snapshot(&without);

        let mut doubled = encoded.clone();
        doubled.extend_from_slice(&encoded[baseline.len()..]);
        assert!(matches!(
            decode_observer_snapshot(&doubled),
            Err(WireError::DuplicateField(48))
        ));
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
        hydrology_deltas: Vec::new(),
        hydrology_delta_schema_version: 0,
        hydrology_transfer_summaries: Vec::new(),
        hydrology_transfer_schema_version: 0,
        hydrology_conveyance_summaries: Vec::new(),
        hydrology_conveyance_schema_version: 0,
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
        hydrology_deltas: Vec::new(),
        hydrology_delta_schema_version: 0,
        hydrology_transfer_summaries: Vec::new(),
        hydrology_transfer_schema_version: 0,
        hydrology_conveyance_summaries: Vec::new(),
        hydrology_conveyance_schema_version: 0,
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
        hydrology_deltas: Vec::new(),
        hydrology_delta_schema_version: 0,
        hydrology_transfer_summaries: Vec::new(),
        hydrology_transfer_schema_version: 0,
        hydrology_conveyance_summaries: Vec::new(),
        hydrology_conveyance_schema_version: 0,
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
        hydrology_deltas: Vec::new(),
        hydrology_delta_schema_version: 0,
        hydrology_transfer_summaries: Vec::new(),
        hydrology_transfer_schema_version: 0,
        hydrology_conveyance_summaries: Vec::new(),
        hydrology_conveyance_schema_version: 0,
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
        hydrology_deltas: Vec::new(),
        hydrology_delta_schema_version: 0,
        hydrology_transfer_summaries: Vec::new(),
        hydrology_transfer_schema_version: 0,
        hydrology_conveyance_summaries: Vec::new(),
        hydrology_conveyance_schema_version: 0,
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
