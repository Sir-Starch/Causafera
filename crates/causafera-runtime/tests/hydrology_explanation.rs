//! Typed hydrology Explanation over engine-produced state.
//!
//! Covers `plans/hydrology.md` §12's numeric policy and V29: supported claims
//! cite authoritative traces and exact typed values; unknown scope or missing
//! history returns insufficiency without a fabricated classification; a
//! per-carrier volume above `i64::MAX` round-trips as a ratio over one while a
//! whole-scope total above `u64::MAX` returns insufficiency instead of narrowing.

mod support_hydrology;

use causafera_core::{EvidenceSchemaId, HistoryQuery, HistoryReader, HistoryRecord};
use causafera_explanation::{
    ClaimEvidenceState, DeterministicExplanationRenderer, ExplanationClaim,
    ExplanationClaimSchemaId, FrameAssessment, HYDROLOGY_BOUNDARY_EXPORT_SCHEMA,
    HYDROLOGY_CONSERVATION_RESIDUAL_SCHEMA, HYDROLOGY_FORCING_ACCEPTED_SCHEMA,
    HYDROLOGY_FORCING_UNMET_SCHEMA, HYDROLOGY_STORAGE_MAXIMUM_SCHEMA,
    HYDROLOGY_STORAGE_MINIMUM_SCHEMA, HYDROLOGY_STORAGE_TOTAL_SCHEMA,
    HYDROLOGY_TRANSFER_ACCEPTED_SCHEMA, HYDROLOGY_TRANSFER_LIMITED_SCHEMA,
    HYDROLOGY_WATER_TABLE_RANGE_SCHEMA, HydrologyConservationClaim, HydrologyStorageClaim,
    HydrologyTransferPathClaim, NumericClaimValue, ObserverLocale, exact_volume,
};
use causafera_runtime::{HYDROLOGY_CONSERVATION_EVENT_KIND, Runtime, RuntimeConfig, RuntimeError};
use causafera_types::{ChartChunkCoord, ChunkCoord, SpatialChartId, TraceId, WaterVolume};

use support_hydrology::{enabled_runtime_config, wet_runtime_config};

fn ticked(config: RuntimeConfig, ticks: u64) -> Runtime {
    let mut runtime = Runtime::new(config).expect("the runtime must bootstrap");
    for _ in 0..ticks {
        runtime.tick().expect("a hydrology tick must commit");
    }
    runtime
}

fn sparse_forcing_runtime_config() -> RuntimeConfig {
    let mut config = enabled_runtime_config();
    let parameters = config
        .hydrology
        .bootstrap_parameters
        .as_mut()
        .expect("the fixture is enabled");
    parameters.initial_surface = WaterVolume::ZERO;
    parameters.initial_soil = WaterVolume::ZERO;
    parameters.initial_groundwater = WaterVolume::ZERO;
    parameters.infiltration_rate_mm_per_second = 0;
    parameters.baseflow_threshold = WaterVolume::ZERO;
    parameters.baseflow_fraction_num = 0;
    parameters.base_surface_transmissivity_mm3_per_second = 0;
    parameters.base_groundwater_transmissivity_mm3_per_second = 0;
    parameters.conveyance_initial_storage = WaterVolume::ZERO;
    parameters.conveyance_inlet_capacity_per_tick = WaterVolume::ZERO;
    config.hydrology.forcing_schedule[0].targets.truncate(1);
    config
}

fn claims_of(runtime: &Runtime, scope: Option<ChartChunkCoord>) -> Vec<ExplanationClaim> {
    let report = runtime
        .observer_hydrology_explanation(scope)
        .expect("the hydrology explanation must succeed");
    assert_eq!(report.frames.len(), 1);
    report.frames[0].claims.clone()
}

fn claim(claims: &[ExplanationClaim], schema: ExplanationClaimSchemaId) -> &ExplanationClaim {
    claims
        .iter()
        .find(|claim| claim.schema == schema)
        .unwrap_or_else(|| panic!("schema {} must be present", schema.raw()))
}

fn conservation_descendant(history: &dyn HistoryReader, origin: TraceId, tick: u64) -> TraceId {
    let mut pending = std::collections::VecDeque::from([origin]);
    let mut visited = std::collections::BTreeSet::from([origin]);
    while let Some(parent) = pending.pop_front() {
        let mut offset = 0;
        loop {
            let page = history
                .children(
                    parent,
                    HistoryQuery::new(offset, 256, 4 * 1024 * 1024).expect("bounded page"),
                )
                .expect("child query");
            for child in page.records() {
                if !visited.insert(*child) {
                    continue;
                }
                let event = history
                    .event(*child)
                    .expect("event query")
                    .expect("child event exists");
                if event.kind().raw() == HYDROLOGY_CONSERVATION_EVENT_KIND
                    && event.time().raw() == tick
                {
                    return *child;
                }
                pending.push_back(*child);
            }
            let Some(next) = page.next_offset() else {
                break;
            };
            offset = next;
        }
    }
    panic!("forcing ancestry must reach its hydrology conservation event")
}

#[test]
fn a_watered_scope_reports_exact_volumes_with_authoritative_traces() {
    // Given: a world that has moved water for several ticks.
    let runtime = ticked(wet_runtime_config(), 3);
    let state = runtime.hydrology_state();

    // When: the whole resident scope is explained.
    let claims = claims_of(&runtime, None);

    // Then: the bounds are exact volumes, not a lossy `Range`, and each cites
    // committed traces.
    let (mut minimum, mut maximum, mut total) = (u64::MAX, 0_u64, 0_u128);
    for field in state.fields.fields().values() {
        for cell in field.cells() {
            for volume in [
                cell.surface_water().get(),
                cell.soil_water().get(),
                cell.groundwater().get(),
            ] {
                minimum = minimum.min(volume);
                maximum = maximum.max(volume);
                total += u128::from(volume);
            }
        }
    }
    let smallest = claim(&claims, HYDROLOGY_STORAGE_MINIMUM_SCHEMA);
    let largest = claim(&claims, HYDROLOGY_STORAGE_MAXIMUM_SCHEMA);
    assert_eq!(smallest.value, exact_volume(minimum).expect("a ratio"));
    assert_eq!(largest.value, exact_volume(maximum).expect("a ratio"));
    assert_eq!(smallest.evidence_state, ClaimEvidenceState::Supported);
    assert!(!smallest.evidence_traces.is_empty());
    assert!(
        smallest
            .evidence_traces
            .iter()
            .all(|trace| trace.raw() != 0),
        "a supported claim cites committed traces"
    );

    // The total fits `u64` for this fixture and is reported exactly.
    let reported_total = claim(&claims, HYDROLOGY_STORAGE_TOTAL_SCHEMA);
    assert_eq!(
        reported_total.value,
        exact_volume(u64::try_from(total).expect("the fixture fits")).expect("a ratio")
    );

    // The water table is signed and therefore a genuine range.
    let table = claim(&claims, HYDROLOGY_WATER_TABLE_RANGE_SCHEMA);
    assert!(matches!(table.value, NumericClaimValue::Range { .. }));
    assert_eq!(table.evidence_state, ClaimEvidenceState::Supported);
}

#[test]
fn a_committed_batch_reports_a_residual_of_exactly_zero() {
    let runtime = ticked(wet_runtime_config(), 3);
    let claims = claims_of(&runtime, None);

    let residual = claim(&claims, HYDROLOGY_CONSERVATION_RESIDUAL_SCHEMA);
    assert_eq!(residual.value, NumericClaimValue::scalar(0));
    assert_eq!(residual.evidence_state, ClaimEvidenceState::Supported);
    assert!(!residual.evidence_traces.is_empty());

    let exported = claim(&claims, HYDROLOGY_BOUNDARY_EXPORT_SCHEMA);
    assert!(matches!(
        exported.value,
        NumericClaimValue::Ratio { denominator: 1, .. }
    ));
}

#[test]
fn a_transfer_reports_both_what_moved_and_what_a_bound_refused() {
    let runtime = ticked(wet_runtime_config(), 3);
    let claims = claims_of(&runtime, None);

    // Both claims are present even when nothing was refused: a bound that did
    // not engage is evidence, and its absence would make "nothing was refused"
    // and "nothing was asked" the same answer.
    let accepted = claim(&claims, HYDROLOGY_TRANSFER_ACCEPTED_SCHEMA);
    let limited = claim(&claims, HYDROLOGY_TRANSFER_LIMITED_SCHEMA);
    assert_eq!(accepted.evidence_state, ClaimEvidenceState::Supported);
    assert_eq!(limited.evidence_state, ClaimEvidenceState::Supported);
    assert!(matches!(
        accepted.value,
        NumericClaimValue::Ratio { denominator: 1, .. }
    ));
    assert!(matches!(
        limited.value,
        NumericClaimValue::Ratio { denominator: 1, .. }
    ));
}

#[test]
fn the_applied_forcing_record_is_reported_with_its_ancestry() {
    let runtime = ticked(enabled_runtime_config(), 5);
    let claims = claims_of(&runtime, None);
    let state = runtime.hydrology_state();
    let record = state
        .forcing
        .iter()
        .filter(|record| record.is_applied())
        .max_by_key(|record| record.key())
        .expect("the fixture's record applies at tick three");

    let accepted = claim(&claims, HYDROLOGY_FORCING_ACCEPTED_SCHEMA);
    assert_eq!(accepted.evidence_state, ClaimEvidenceState::Supported);
    // The producer's own event leads the ancestry: a forcing claim that stopped
    // at the settlement would say what arrived without saying where from.
    assert!(accepted.evidence_traces.contains(&record.origin_trace()));
    assert!(
        claims
            .iter()
            .any(|claim| claim.schema == HYDROLOGY_FORCING_UNMET_SCHEMA)
    );
}

#[test]
fn an_unknown_scope_returns_insufficiency_rather_than_an_error() {
    // V29: a chunk that is not resident is answered, not refused.
    let runtime = ticked(wet_runtime_config(), 2);
    let elsewhere = ChartChunkCoord::new(SpatialChartId::new(9), ChunkCoord::new(7, 7, 7));

    let claims = claims_of(&runtime, Some(elsewhere));

    for schema in [
        HYDROLOGY_STORAGE_MINIMUM_SCHEMA,
        HYDROLOGY_STORAGE_MAXIMUM_SCHEMA,
        HYDROLOGY_STORAGE_TOTAL_SCHEMA,
        HYDROLOGY_WATER_TABLE_RANGE_SCHEMA,
    ] {
        let claim = claim(&claims, schema);
        assert_eq!(
            claim.evidence_state,
            ClaimEvidenceState::Unknown,
            "schema {} must report insufficiency for an unknown scope",
            schema.raw()
        );
        assert!(claim.evidence_traces.is_empty());
        assert_eq!(claim.confidence.raw(), 0.0);
    }
    // No transfer touches that chunk, so no transfer claim is fabricated for it.
    assert!(
        !claims
            .iter()
            .any(|claim| claim.schema == HYDROLOGY_TRANSFER_ACCEPTED_SCHEMA)
    );
}

#[test]
fn a_session_without_hydrology_reports_insufficiency_throughout() {
    let runtime = ticked(RuntimeConfig::new(7_007), 2);

    let report = runtime
        .observer_hydrology_explanation(None)
        .expect("a disabled session is answered, not refused");

    assert_eq!(report.overall_assessment, FrameAssessment::Unknown);
    assert!(
        report.frames[0]
            .claims
            .iter()
            .all(|claim| claim.evidence_state == ClaimEvidenceState::Unknown),
        "nothing may be asserted about a domain that never ran"
    );
}

#[test]
fn an_evicted_batch_keeps_forcing_claims_supported_by_history() {
    // Given: the fixture's applied forcing receipts have left the retained window,
    // with a control captured while that batch is still retained.
    let control = ticked(sparse_forcing_runtime_config(), 3);
    let control_claims = claims_of(&control, None);
    let runtime = ticked(sparse_forcing_runtime_config(), 14);
    let claims = claims_of(&runtime, None);

    // When: the current hydrology state is explained through its history reader.
    // Then: retained typed evidence continues to support the exact claims.
    assert_eq!(
        claim(&claims, HYDROLOGY_FORCING_ACCEPTED_SCHEMA).evidence_state,
        ClaimEvidenceState::Supported
    );
    assert_eq!(
        claim(&claims, HYDROLOGY_FORCING_UNMET_SCHEMA).evidence_state,
        ClaimEvidenceState::Supported
    );

    let forcing = claim(&claims, HYDROLOGY_FORCING_ACCEPTED_SCHEMA);
    let origin = *forcing
        .evidence_traces
        .first()
        .expect("historical forcing origin trace");
    let history = runtime
        .history_reader()
        .expect("runtime history reader")
        .pin_snapshot()
        .expect("pinned history reader");
    let conservation_trace = conservation_descendant(history.as_ref(), origin, 3);
    let historical = runtime
        .observer_hydrology_explanation_for_batch(conservation_trace, None)
        .expect("evicted batch remains explainable");
    let historical_claims = &historical.frames[0].claims;
    for schema in [
        HYDROLOGY_FORCING_ACCEPTED_SCHEMA,
        HYDROLOGY_FORCING_UNMET_SCHEMA,
        HYDROLOGY_CONSERVATION_RESIDUAL_SCHEMA,
        HYDROLOGY_BOUNDARY_EXPORT_SCHEMA,
        HYDROLOGY_TRANSFER_ACCEPTED_SCHEMA,
        HYDROLOGY_TRANSFER_LIMITED_SCHEMA,
    ] {
        assert_eq!(
            claim(historical_claims, schema),
            claim(&control_claims, schema),
            "historical schema {} matches the retained-batch control",
            schema.raw()
        );
    }
    assert!(
        historical_claims
            .iter()
            .all(|claim| claim.evidence_traces.len() <= 64)
    );
    assert!(
        historical_claims
            .iter()
            .flat_map(|claim| &claim.evidence_traces)
            .all(|trace| history.event(*trace).expect("trace query").is_some())
    );
}

#[test]
fn an_oversize_transfer_batch_is_not_reported_as_complete() {
    let runtime = ticked(wet_runtime_config(), 1);
    let history = runtime
        .history_reader()
        .expect("runtime history reader")
        .pin_snapshot()
        .expect("pinned history reader");
    let mut offset = 0;
    let transfer_record = loop {
        let page = history
            .indexed_range(HistoryQuery::new(offset, 256, 4 * 1024 * 1024).expect("history page"))
            .expect("history range");
        if let Some(record) = page.records().iter().find_map(|record| match record {
            HistoryRecord::Evidence(record)
                if record.schema_id() == EvidenceSchemaId::new(0x0014_1803) =>
            {
                Some(record.clone())
            }
            _ => None,
        }) {
            break record;
        }
        offset = page
            .next_offset()
            .expect("transfer evidence exists in history");
    };
    let conservation_trace = TraceId::new(u64::from_le_bytes(
        transfer_record.canonical_key()[..8]
            .try_into()
            .expect("batch prefix is eight bytes"),
    ));
    let prefix = conservation_trace.raw().to_le_bytes();
    let page = history
        .evidence_by_key_prefix(
            EvidenceSchemaId::new(0x0014_1803),
            &prefix,
            HistoryQuery::new(0, 256, 4 * 1024 * 1024).expect("bounded history page"),
        )
        .expect("indexed transfer evidence page");
    assert_eq!(page.records().len(), 256);
    assert!(
        page.next_offset().is_some(),
        "batch must contain over 256 records"
    );
    assert!(matches!(
        runtime.observer_hydrology_explanation_for_batch(conservation_trace, None),
        Err(RuntimeError::HistoryQueryPartial {
            schema_id: 0x0014_1803
        })
    ));
}

#[test]
fn a_per_carrier_volume_above_the_signed_ceiling_survives_as_a_ratio() {
    // §12: exact per-carrier `u64` volumes travel as `Ratio { v, 1 }` rather
    // than through a widened claim-value variant.
    let volume = u64::MAX;
    let claim = HydrologyStorageClaim {
        carrier_count: 1,
        minimum_volume: volume,
        maximum_volume: volume,
        total_volume: u128::from(volume),
        water_table_minimum_mm: -1_000,
        water_table_maximum_mm: 1_000,
        evidence_traces: vec![TraceId::new(11)],
    }
    .to_explanation_claims()
    .expect("the claim must build");

    let maximum = claim
        .iter()
        .find(|claim| claim.schema == HYDROLOGY_STORAGE_MAXIMUM_SCHEMA)
        .expect("the maximum is present");
    assert_eq!(
        maximum.value,
        NumericClaimValue::Ratio {
            numerator: u64::MAX,
            denominator: 1
        }
    );
    assert_eq!(maximum.evidence_state, ClaimEvidenceState::Supported);
}

#[test]
fn a_whole_scope_total_above_the_unsigned_ceiling_returns_insufficiency() {
    // §12: never narrowed. The scope is real and the measurement exists; this
    // schema cannot carry it, and saying so is the only honest answer.
    let claims = HydrologyStorageClaim {
        carrier_count: 3,
        minimum_volume: 1,
        maximum_volume: u64::MAX,
        total_volume: u128::from(u64::MAX) + 1,
        water_table_minimum_mm: 0,
        water_table_maximum_mm: 10,
        evidence_traces: vec![TraceId::new(12)],
    }
    .to_explanation_claims()
    .expect("the claim must build");

    let total = claims
        .iter()
        .find(|claim| claim.schema == HYDROLOGY_STORAGE_TOTAL_SCHEMA)
        .expect("the total is present");
    assert_eq!(total.evidence_state, ClaimEvidenceState::Unknown);
    assert!(total.evidence_traces.is_empty());
    // The bounds are unaffected: one unrepresentable total does not make the
    // per-carrier measurements unknown.
    assert_eq!(
        claims
            .iter()
            .find(|claim| claim.schema == HYDROLOGY_STORAGE_MAXIMUM_SCHEMA)
            .expect("the maximum is present")
            .evidence_state,
        ClaimEvidenceState::Supported
    );
}

#[test]
fn a_nonzero_residual_is_insufficiency_rather_than_a_reported_discrepancy() {
    // A nonzero residual means this batch is not committed evidence. Reporting
    // the number would present an uncommitted state as a measurement.
    let claims = HydrologyConservationClaim {
        residual: 17,
        boundary_exports: 4,
        conservation_trace: TraceId::new(21),
        transfer_traces: vec![TraceId::new(22)],
    }
    .to_explanation_claims()
    .expect("the claim must build");

    let residual = claims
        .iter()
        .find(|claim| claim.schema == HYDROLOGY_CONSERVATION_RESIDUAL_SCHEMA)
        .expect("the residual is present");
    assert_eq!(residual.evidence_state, ClaimEvidenceState::Unknown);
    assert_eq!(residual.value, NumericClaimValue::scalar(0));
    assert!(residual.evidence_traces.is_empty());
}

#[test]
fn a_transfer_whose_volumes_do_not_close_is_refused_at_construction() {
    // Distinct from a nonzero residual: the three volumes come from one receipt,
    // so a caller presenting them not closing has built the claim from parts of
    // different transfers.
    let malformed = HydrologyTransferPathClaim {
        process_kind: 7,
        requested_volume: 100,
        accepted_volume: 40,
        unaccepted_volume: 50,
        transfer_trace: TraceId::new(31),
        conservation_trace: TraceId::new(32),
        forcing_origin_trace: None,
    }
    .to_explanation_claims();

    assert!(malformed.is_err());
}

#[test]
fn rendering_is_locale_dependent_and_the_claims_are_not() {
    // The observer may render a process name; the simulation may not carry one.
    // Every locale renders the same claims, so wording is downstream of evidence.
    let runtime = ticked(wet_runtime_config(), 2);
    let report = runtime
        .observer_hydrology_explanation(None)
        .expect("the hydrology explanation must succeed");

    let renderer = DeterministicExplanationRenderer;
    let english = renderer.render(&report, ObserverLocale::En).text;
    let russian = renderer.render(&report, ObserverLocale::Ru).text;
    assert_ne!(english, russian);
    assert!(english.contains("hydrology conservation residual"));
    assert!(russian.contains("невязка водного баланса"));
}
