use causafera_core::{
    Criticality, DomainEvidenceRecord, EvidenceSchemaId, HistoryError, HistoryQuery, HistoryReader,
};
use causafera_domains::{HydrologyConservationParts, HydrologyConservationReceipt, process};
use causafera_explanation::{
    ExplanationClaim, ExplanationIrError, HydrologyConservationClaim, HydrologyForcingClaim,
    HydrologyTransferPathClaim,
};
use causafera_geography::HydrologyCarrierKey;
use causafera_persistence::LittleEndianDecoder;
use causafera_types::{ChartChunkCoord, TraceId};
use thiserror::Error;

use crate::{
    HYDROLOGY_CONSERVATION_EVENT_KIND, HYDROLOGY_CONSERVATION_EVIDENCE_SCHEMA,
    HYDROLOGY_EVIDENCE_VERSION, HYDROLOGY_FORCING_EVIDENCE_SCHEMA,
    HYDROLOGY_TRANSFER_EVIDENCE_SCHEMA, HydrologyRuntimeState,
    hydrology_projection::MAX_EXPLANATION_EVIDENCE_TRACES,
};
use causafera_geography::MAX_HYDROLOGY_TARGETS_PER_FORCING;

const EVIDENCE_PAGE_BYTES: usize = 4 * 1024 * 1024;
const MAX_FORCING_EVIDENCE_RECORDS: usize = 64;

#[derive(Debug, Error)]
pub(crate) enum HydrologyExplanationError {
    #[error(transparent)]
    History(#[from] HistoryError),
    #[error(transparent)]
    Explanation(#[from] ExplanationIrError),
    #[error("hydrology historical evidence failed integrity check: {0}")]
    Integrity(&'static str),
    #[error("hydrology history query is partial for evidence schema {schema_id:?}")]
    PartialEvidence { schema_id: EvidenceSchemaId },
}

pub(crate) fn historical_claims(
    state: &HydrologyRuntimeState,
    history: &dyn HistoryReader,
    scope: Option<ChartChunkCoord>,
    conservation_trace: Option<TraceId>,
    selected_batch: Option<TraceId>,
) -> Result<Vec<ExplanationClaim>, HydrologyExplanationError> {
    let mut storage = state.storage_claim(scope)?;
    let mut storage_traces = storage.evidence_traces.clone();
    storage_traces.sort_unstable();
    storage_traces.dedup();
    storage_traces.truncate(MAX_EXPLANATION_EVIDENCE_TRACES);
    for trace in &storage_traces {
        if history.event(*trace)?.is_none() {
            storage.evidence_traces.clear();
            break;
        }
    }
    if !storage.evidence_traces.is_empty() {
        storage.evidence_traces = storage_traces;
    }

    let mut claims = storage.to_explanation_claims()?;
    let Some(conservation_trace) = conservation_trace else {
        claims.extend(HydrologyForcingClaim::unknown()?);
        claims.extend(HydrologyConservationClaim::unknown()?);
        return Ok(claims);
    };
    let Some(conservation_event) = history.event(conservation_trace)? else {
        claims.extend(HydrologyForcingClaim::unknown()?);
        claims.extend(HydrologyConservationClaim::unknown()?);
        return Ok(claims);
    };
    if conservation_event.kind().raw() != HYDROLOGY_CONSERVATION_EVENT_KIND {
        return Err(HydrologyExplanationError::Integrity(
            "selected trace is not a hydrology conservation event",
        ));
    }

    let conservation_record = evidence(
        history,
        HYDROLOGY_CONSERVATION_EVIDENCE_SCHEMA,
        &conservation_trace.raw().to_le_bytes(),
    )?;
    let Some(conservation_record) = conservation_record else {
        claims.extend(HydrologyForcingClaim::unknown()?);
        claims.extend(HydrologyConservationClaim::unknown()?);
        return Ok(claims);
    };
    validate_record(&conservation_record, conservation_trace)?;
    validate_claim_trace(history, &conservation_record, conservation_trace)?;
    let conservation = decode_conservation(&conservation_record)?;

    let transfer_records = transfer_records(history, conservation_trace, selected_batch.is_some())?;
    let mut transfer_traces = Vec::with_capacity(transfer_records.len());
    for transfer in &transfer_records {
        transfer_traces.push(transfer.settlement_trace(conservation_trace));
    }
    transfer_traces.sort_unstable();
    transfer_traces.dedup();
    transfer_traces.truncate(MAX_EXPLANATION_EVIDENCE_TRACES - 1);
    let conservation_claims = HydrologyConservationClaim {
        residual: conservation.residual(),
        boundary_exports: conservation.boundary_exports(),
        conservation_trace,
        transfer_traces,
    }
    .to_explanation_claims()?;
    claims.extend(forcing_claims(state, history, selected_batch)?);
    claims.extend(conservation_claims);

    let mut scoped_transfers = 0_usize;
    for transfer in transfer_records
        .iter()
        .take(causafera_observer_api::MAX_HYDROLOGY_TRANSFER_SUMMARIES)
    {
        if scope.is_some_and(|wanted| !transfer.touches(wanted)) {
            continue;
        }
        scoped_transfers = scoped_transfers.saturating_add(1);
        let transfer_trace = transfer.settlement_trace(conservation_trace);
        claims.extend(
            HydrologyTransferPathClaim {
                process_kind: transfer.process_kind,
                requested_volume: transfer.requested,
                accepted_volume: transfer.accepted,
                unaccepted_volume: transfer.unaccepted,
                transfer_trace,
                conservation_trace,
                forcing_origin_trace: transfer.forcing_origin,
            }
            .to_explanation_claims()?,
        );
    }
    if scoped_transfers == 0 && scope.is_none() {
        claims.extend(unknown_transfer_claims()?);
    }
    Ok(claims)
}

fn unknown_transfer_claims() -> Result<Vec<ExplanationClaim>, HydrologyExplanationError> {
    Ok(vec![
        ExplanationClaim::unknown(
            causafera_explanation::HYDROLOGY_TRANSFER_ACCEPTED_SCHEMA,
            causafera_explanation::exact_volume(0)?,
            causafera_explanation::ComparisonContext::None,
        )?,
        ExplanationClaim::unknown(
            causafera_explanation::HYDROLOGY_TRANSFER_LIMITED_SCHEMA,
            causafera_explanation::exact_volume(0)?,
            causafera_explanation::ComparisonContext::None,
        )?,
    ])
}

fn forcing_claims(
    state: &HydrologyRuntimeState,
    history: &dyn HistoryReader,
    selected_batch: Option<TraceId>,
) -> Result<Vec<ExplanationClaim>, HydrologyExplanationError> {
    let mut claims = Vec::new();
    for forcing in state.forcing.iter().filter(|record| record.is_applied()) {
        let query = HistoryQuery::new(0, MAX_FORCING_EVIDENCE_RECORDS, EVIDENCE_PAGE_BYTES)?;
        let page = history.evidence_by_anchor(
            HYDROLOGY_FORCING_EVIDENCE_SCHEMA,
            forcing.origin_trace(),
            query,
        )?;
        if page.next_offset().is_some() {
            return Err(HydrologyExplanationError::PartialEvidence {
                schema_id: HYDROLOGY_FORCING_EVIDENCE_SCHEMA,
            });
        }
        let matching_records = page
            .records()
            .iter()
            .filter_map(
                |record| match forcing_record_identity_matches(record, forcing) {
                    Ok(true) => Some(Ok(record)),
                    Ok(false) => None,
                    Err(error) => Some(Err(error)),
                },
            )
            .collect::<Result<Vec<_>, _>>()?;
        if matching_records.len() > 1 {
            return Err(HydrologyExplanationError::Integrity(
                "forcing identity has multiple indexed evidence records",
            ));
        }
        let Some(record) = matching_records.first().copied() else {
            continue;
        };
        let conservation_trace = TraceId::new(record.commit_sequence());
        if selected_batch.is_some_and(|selected| selected != conservation_trace) {
            continue;
        }
        let Some(event) = history.event(conservation_trace)? else {
            return Err(HydrologyExplanationError::Integrity(
                "forcing evidence references a missing conservation event",
            ));
        };
        if event.kind().raw() != HYDROLOGY_CONSERVATION_EVENT_KIND {
            return Err(HydrologyExplanationError::Integrity(
                "forcing evidence references a non-conservation event",
            ));
        }
        validate_record(record, conservation_trace)?;
        validate_claim_trace(history, record, forcing.origin_trace())?;
        validate_forcing_record(record, forcing)?;
        let conservation_record = evidence(
            history,
            HYDROLOGY_CONSERVATION_EVIDENCE_SCHEMA,
            &conservation_trace.raw().to_le_bytes(),
        )?
        .ok_or(HydrologyExplanationError::Integrity(
            "forcing evidence has no conservation evidence record",
        ))?;
        validate_record(&conservation_record, conservation_trace)?;
        let conservation = decode_conservation(&conservation_record)?;
        if forcing.applied_at() != Some(conservation.tick()) {
            return Err(HydrologyExplanationError::Integrity(
                "forcing tick differs from its conservation batch",
            ));
        }
        let transfer_records =
            transfer_records(history, conservation_trace, selected_batch.is_some())?;
        let carrier = HydrologyCarrierKey::ForcingRecord {
            scheduled_tick: forcing.scheduled_tick(),
            forcing_id: forcing.forcing_id(),
        };
        let mut claim = HydrologyForcingClaim {
            scheduled_tick: forcing.scheduled_tick(),
            forcing_id: forcing.forcing_id(),
            accepted_source: 0,
            unmet_source: 0,
            accepted_evapotranspiration: 0,
            unmet_evapotranspiration: 0,
            origin_trace: forcing.origin_trace(),
            settlement_traces: Vec::new(),
        };
        for transfer in &transfer_records {
            if transfer.source == carrier {
                claim.accepted_source = claim
                    .accepted_source
                    .saturating_add(u128::from(transfer.accepted));
                claim.unmet_source = claim.unmet_source.saturating_add(transfer.unaccepted);
                claim
                    .settlement_traces
                    .push(transfer.settlement_trace(conservation_trace));
            }
            if matches!(
                transfer.process_kind,
                process::EVAPOTRANSPIRATION_SURFACE | process::EVAPOTRANSPIRATION_SOIL
            ) && transfer.forcing_origin == Some(forcing.origin_trace())
            {
                claim.accepted_evapotranspiration = claim
                    .accepted_evapotranspiration
                    .saturating_add(transfer.accepted);
                claim.unmet_evapotranspiration = claim
                    .unmet_evapotranspiration
                    .saturating_add(transfer.unaccepted);
                claim
                    .settlement_traces
                    .push(transfer.settlement_trace(conservation_trace));
            }
        }
        claim.settlement_traces.sort_unstable();
        claim.settlement_traces.dedup();
        claim
            .settlement_traces
            .truncate(MAX_EXPLANATION_EVIDENCE_TRACES - 1);
        claims.extend(claim.to_explanation_claims()?);
    }
    if claims.is_empty() {
        claims = HydrologyForcingClaim::unknown()?;
    }
    Ok(claims)
}

fn evidence(
    history: &dyn HistoryReader,
    schema: causafera_core::EvidenceSchemaId,
    key: &[u8],
) -> Result<Option<DomainEvidenceRecord>, HydrologyExplanationError> {
    let query = HistoryQuery::new(0, 1, EVIDENCE_PAGE_BYTES)?;
    let page = history.evidence(schema, key, query)?;
    if page.next_offset().is_some() {
        return Err(HydrologyExplanationError::Integrity(
            "hydrology evidence key returned multiple records",
        ));
    }
    Ok(page.records().first().cloned())
}

fn validate_record(
    record: &DomainEvidenceRecord,
    conservation_trace: TraceId,
) -> Result<(), HydrologyExplanationError> {
    if record.commit_sequence() != conservation_trace.raw()
        || record.criticality() != Criticality::ReplayRequired
        || record.version() != HYDROLOGY_EVIDENCE_VERSION
    {
        return Err(HydrologyExplanationError::Integrity(
            "hydrology evidence metadata does not match its committed batch",
        ));
    }
    Ok(())
}

fn validate_claim_trace(
    history: &dyn HistoryReader,
    record: &DomainEvidenceRecord,
    trace: TraceId,
) -> Result<(), HydrologyExplanationError> {
    if !record.causal_anchors().contains(&trace) || history.event(trace)?.is_none() {
        return Err(HydrologyExplanationError::Integrity(
            "hydrology claim trace is missing from its evidence or history",
        ));
    }
    Ok(())
}

fn decode_conservation(
    record: &DomainEvidenceRecord,
) -> Result<HydrologyConservationReceipt, HydrologyExplanationError> {
    let mut decoder = LittleEndianDecoder::new(record.canonical_payload());
    let tick = decoder
        .read_u64()
        .map_err(|_| HydrologyExplanationError::Integrity("invalid conservation tick"))?;
    let batch_sequence = decoder
        .read_u64()
        .map_err(|_| HydrologyExplanationError::Integrity("invalid conservation sequence"))?;
    let mut terms = [0_i128; 12];
    for term in &mut terms {
        *term = i128::from_le_bytes(
            *decoder
                .read_fixed::<16>()
                .map_err(|_| HydrologyExplanationError::Integrity("invalid conservation term"))?,
        );
    }
    if !decoder.is_empty() {
        return Err(HydrologyExplanationError::Integrity(
            "conservation evidence has trailing bytes",
        ));
    }
    let receipt = HydrologyConservationReceipt::new(HydrologyConservationParts {
        tick,
        batch_sequence,
        surface_before: terms[0],
        soil_before: terms[1],
        groundwater_before: terms[2],
        conveyance_before: terms[3],
        surface_after: terms[4],
        soil_after: terms[5],
        groundwater_after: terms[6],
        conveyance_after: terms[7],
        accepted_precipitation: terms[8],
        accepted_external_inflow: terms[9],
        accepted_evapotranspiration: terms[10],
        boundary_exports: terms[11],
    })
    .map_err(|_| HydrologyExplanationError::Integrity("invalid conservation evidence"))?;
    receipt
        .require_balanced()
        .map_err(|_| HydrologyExplanationError::Integrity("unbalanced conservation evidence"))?;
    Ok(receipt)
}

fn validate_forcing_record(
    record: &DomainEvidenceRecord,
    forcing: &causafera_geography::HydrologyForcingRecord,
) -> Result<(), HydrologyExplanationError> {
    let mut decoder = LittleEndianDecoder::new(record.canonical_payload());
    let fields = [
        decoder.read_u64(),
        decoder.read_u64(),
        decoder.read_u64(),
        decoder.read_u64(),
        decoder.read_u64(),
        decoder.read_u64(),
        decoder.read_u64(),
        decoder.read_u64(),
    ];
    let values = fields.map(|field| {
        field.map_err(|_| HydrologyExplanationError::Integrity("invalid forcing evidence"))
    });
    let [
        scheduled_tick,
        forcing_id,
        origin,
        policy,
        precipitation,
        potential_et,
        inflow,
        applied_at,
    ] = values;
    if scheduled_tick? != forcing.scheduled_tick()
        || forcing_id? != forcing.forcing_id()
        || origin? != forcing.origin_trace().raw()
        || policy? != forcing.producer_policy_schema()
        || precipitation? != forcing.precipitation_volume().get()
        || potential_et? != forcing.potential_et_volume().get()
        || inflow? != forcing.external_inflow_volume().get()
        || applied_at? != forcing.applied_at().unwrap_or(0)
    {
        return Err(HydrologyExplanationError::Integrity(
            "forcing evidence does not match the persisted forcing record",
        ));
    }
    let count = decoder
        .read_u64()
        .map_err(|_| HydrologyExplanationError::Integrity("invalid forcing members"))?;
    if count == 0 || count > MAX_HYDROLOGY_TARGETS_PER_FORCING as u64 {
        return Err(HydrologyExplanationError::Integrity(
            "forcing evidence member count is out of bounds",
        ));
    }
    let count = usize::try_from(count)
        .map_err(|_| HydrologyExplanationError::Integrity("forcing member count overflow"))?;
    if count != forcing.targets().len() {
        return Err(HydrologyExplanationError::Integrity(
            "forcing evidence member count does not match its schedule",
        ));
    }
    for target in forcing.targets() {
        let encoded = decoder
            .read_fixed::<23>()
            .map_err(|_| HydrologyExplanationError::Integrity("invalid forcing target"))?;
        let carrier = HydrologyCarrierKey::decode(encoded)
            .map_err(|_| HydrologyExplanationError::Integrity("invalid forcing target"))?;
        let weight = decoder
            .read_u64()
            .map_err(|_| HydrologyExplanationError::Integrity("invalid forcing weight"))?;
        if carrier != HydrologyCarrierKey::Cell(target.cell) || weight != target.weight.get() {
            return Err(HydrologyExplanationError::Integrity(
                "forcing evidence members do not match their schedule",
            ));
        }
    }
    if !decoder.is_empty() {
        return Err(HydrologyExplanationError::Integrity(
            "forcing evidence has trailing bytes",
        ));
    }
    Ok(())
}

fn forcing_record_identity_matches(
    record: &DomainEvidenceRecord,
    forcing: &causafera_geography::HydrologyForcingRecord,
) -> Result<bool, HydrologyExplanationError> {
    let mut decoder = LittleEndianDecoder::new(record.canonical_payload());
    let scheduled_tick = decoder
        .read_u64()
        .map_err(|_| HydrologyExplanationError::Integrity("invalid forcing evidence"))?;
    let forcing_id = decoder
        .read_u64()
        .map_err(|_| HydrologyExplanationError::Integrity("invalid forcing evidence"))?;
    let origin = decoder
        .read_u64()
        .map_err(|_| HydrologyExplanationError::Integrity("invalid forcing evidence"))?;
    Ok(scheduled_tick == forcing.scheduled_tick()
        && forcing_id == forcing.forcing_id()
        && origin == forcing.origin_trace().raw())
}

fn transfer_records(
    history: &dyn HistoryReader,
    conservation_trace: TraceId,
    require_complete: bool,
) -> Result<Vec<TransferEvidence>, HydrologyExplanationError> {
    let prefix = conservation_trace.raw().to_le_bytes();
    let query = HistoryQuery::new(
        0,
        causafera_observer_api::MAX_HYDROLOGY_TRANSFER_SUMMARIES,
        EVIDENCE_PAGE_BYTES,
    )?;
    let page =
        history.evidence_by_key_prefix(HYDROLOGY_TRANSFER_EVIDENCE_SCHEMA, &prefix, query)?;
    if require_complete && page.next_offset().is_some() {
        return Err(HydrologyExplanationError::PartialEvidence {
            schema_id: HYDROLOGY_TRANSFER_EVIDENCE_SCHEMA,
        });
    }
    let mut indexed_records = page
        .records()
        .iter()
        .map(|record| indexed_transfer_record(record, conservation_trace))
        .collect::<Result<Vec<_>, _>>()?;
    indexed_records.sort_by_key(|(index, _)| *index);
    let mut records = Vec::with_capacity(indexed_records.len());
    let mut previous_index = None;
    for (index, record) in indexed_records {
        if previous_index.is_some_and(|previous| previous == index) {
            return Err(HydrologyExplanationError::Integrity(
                "hydrology transfer evidence index is duplicated",
            ));
        }
        previous_index = Some(index);
        let decoded = decode_transfer(&record)?;
        validate_claim_trace(
            history,
            &record,
            decoded.settlement_trace(conservation_trace),
        )?;
        if let Some(origin) = decoded.forcing_origin {
            validate_claim_trace(history, &record, origin)?;
        }
        records.push(decoded);
    }
    Ok(records)
}

fn indexed_transfer_record(
    record: &DomainEvidenceRecord,
    conservation_trace: TraceId,
) -> Result<(u64, DomainEvidenceRecord), HydrologyExplanationError> {
    validate_record(record, conservation_trace)?;
    let key = record.canonical_key();
    if key.len() != 16 || key[..8] != conservation_trace.raw().to_le_bytes() {
        return Err(HydrologyExplanationError::Integrity(
            "hydrology transfer evidence key does not match its batch",
        ));
    }
    let index = u64::from_le_bytes(key[8..16].try_into().map_err(|_| {
        HydrologyExplanationError::Integrity("invalid hydrology transfer evidence index")
    })?);
    Ok((index, record.clone()))
}

#[derive(Clone, Debug)]
struct TransferEvidence {
    process_kind: u32,
    source: HydrologyCarrierKey,
    target: HydrologyCarrierKey,
    requested: u64,
    accepted: u64,
    unaccepted: u64,
    forcing_origin: Option<TraceId>,
    transfer_event: Option<TraceId>,
    storage_event: Option<TraceId>,
}

impl TransferEvidence {
    fn settlement_trace(&self, conservation_trace: TraceId) -> TraceId {
        self.storage_event
            .or(self.transfer_event)
            .unwrap_or(conservation_trace)
    }

    fn touches(&self, chunk: ChartChunkCoord) -> bool {
        [self.source, self.target]
            .into_iter()
            .any(|carrier| match carrier {
                HydrologyCarrierKey::Cell(cell) => cell.chunk() == chunk,
                HydrologyCarrierKey::Edge(edge) => {
                    edge.low().chunk() == chunk || edge.high().chunk() == chunk
                }
                HydrologyCarrierKey::ExteriorFace(face) => face.cell().chunk() == chunk,
                HydrologyCarrierKey::ResolutionChunk(coord) => coord == chunk,
                HydrologyCarrierKey::ForcingRecord { .. } | HydrologyCarrierKey::BatchNode(_) => {
                    false
                }
            })
    }
}

fn decode_transfer(
    record: &DomainEvidenceRecord,
) -> Result<TransferEvidence, HydrologyExplanationError> {
    let mut decoder = LittleEndianDecoder::new(record.canonical_payload());
    decoder
        .read_u64()
        .and_then(|_| decoder.read_u64())
        .map_err(|_| HydrologyExplanationError::Integrity("invalid transfer batch header"))?;
    let process_kind = decoder
        .read_u32()
        .map_err(|_| HydrologyExplanationError::Integrity("invalid transfer process"))?;
    let mut carriers = Vec::with_capacity(2);
    for _ in 0..2 {
        let length = decoder
            .read_u8()
            .map_err(|_| HydrologyExplanationError::Integrity("invalid carrier length"))?;
        let bytes = decoder
            .read_bytes(usize::from(length))
            .map_err(|_| HydrologyExplanationError::Integrity("invalid transfer carrier"))?;
        carriers.push(
            HydrologyCarrierKey::decode(bytes)
                .map_err(|_| HydrologyExplanationError::Integrity("invalid transfer carrier"))?,
        );
    }
    decoder
        .read_u8()
        .and_then(|_| decoder.read_u8())
        .map_err(|_| HydrologyExplanationError::Integrity("invalid transfer buckets"))?;
    let mut volumes = [0_u64; 7];
    for volume in &mut volumes {
        *volume = decoder
            .read_u64()
            .map_err(|_| HydrologyExplanationError::Integrity("invalid transfer volume"))?;
    }
    if volumes[0].checked_sub(volumes[1]) != Some(volumes[2]) {
        return Err(HydrologyExplanationError::Integrity(
            "transfer evidence does not conserve its requested volume",
        ));
    }
    let parent_count = decoder
        .read_u64()
        .map_err(|_| HydrologyExplanationError::Integrity("invalid transfer parents"))?;
    if parent_count > 4096 {
        return Err(HydrologyExplanationError::Integrity(
            "transfer parent count is out of bounds",
        ));
    }
    for _ in 0..parent_count {
        decoder
            .read_u64()
            .map_err(|_| HydrologyExplanationError::Integrity("invalid transfer parent"))?;
    }
    let forcing_raw = decoder
        .read_u64()
        .map_err(|_| HydrologyExplanationError::Integrity("invalid forcing trace"))?;
    let mut event_traces = [None, None];
    for trace in &mut event_traces {
        let raw = decoder
            .read_u64()
            .map_err(|_| HydrologyExplanationError::Integrity("invalid transfer event"))?;
        *trace = (raw != 0).then_some(TraceId::new(raw));
    }
    for _ in 0..2 {
        let length = decoder
            .read_u32()
            .map_err(|_| HydrologyExplanationError::Integrity("invalid proposal key"))?;
        decoder
            .read_bytes(usize::try_from(length).map_err(|_| HistoryError::ArithmeticOverflow)?)
            .map_err(|_| HydrologyExplanationError::Integrity("invalid proposal key"))?;
    }
    if !decoder.is_empty() {
        return Err(HydrologyExplanationError::Integrity(
            "transfer evidence has trailing bytes",
        ));
    }
    let [source, target] = carriers.try_into().map_err(|_| {
        HydrologyExplanationError::Integrity("transfer evidence does not contain two carriers")
    })?;
    Ok(TransferEvidence {
        process_kind,
        source,
        target,
        requested: volumes[0],
        accepted: volumes[1],
        unaccepted: volumes[2],
        forcing_origin: (forcing_raw != 0).then_some(TraceId::new(forcing_raw)),
        transfer_event: event_traces[0],
        storage_event: event_traces[1],
    })
}
