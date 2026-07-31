//! Malformed and forged hydrology snapshots fail closed.
//!
//! Covers `plans/hydrology.md` verification gate V25. Decoding rebuilds every
//! collection through its validating constructor, so most of these are refused
//! before they can become state; the rest are cross-cutting agreements no single
//! constructor can see.

mod support_hydrology;

use causafera_runtime::snapshot_sections::{
    HYDROLOGY_SECTION_ID, assemble_envelope, decode_hydrology_section,
    decode_runtime_recipe_section, disassemble_envelope, encode_hydrology_section,
    encode_runtime_recipe_section,
};
use causafera_runtime::{Runtime, RuntimeState};
use causafera_types::TraceId;

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

// ---------------------------------------------------------------------------
// Carriers reaching outside the state that holds them
// ---------------------------------------------------------------------------

#[test]
fn a_conveyance_edge_outside_the_field_set_is_refused() {
    // The graph's own constructor checks that an edge joins two orthogonally
    // adjacent cells and that its outlet is one of them. It holds no field set,
    // so it cannot ask the remaining question: whether those cells exist. Routing
    // settles a release straight into the outlet's storage, so an edge over
    // ground this session does not hold is a write with nowhere to land — and the
    // solver would abort the process rather than refuse the snapshot.
    let mut runtime = Runtime::new(wet_runtime_config()).expect("construction");
    runtime.run_ticks(2).expect("the run must commit");
    let mut snapshot = runtime.export_snapshot().expect("export");

    let absent = causafera_types::ChartChunkCoord::new(
        support_hydrology::chart(),
        causafera_types::ChunkCoord::new(31, 31, 0),
    );
    assert!(
        !snapshot.hydrology.fields.fields().contains_key(&absent),
        "the fixture must not already hold this chunk"
    );
    let low = causafera_geography::HydrologyCellKey::new(absent, 0).expect("ordinal in range");
    let high = causafera_geography::HydrologyCellKey::new(absent, 1).expect("ordinal in range");
    let key = causafera_geography::HydrologyEdgeKey::new(low, high).expect("adjacent cells");

    let template = *snapshot
        .hydrology
        .conveyance
        .edges()
        .values()
        .next()
        .expect("the fixture builds conveyance");
    let offmap = causafera_geography::HydrologyConveyanceEdge::new(
        key,
        key.low(),
        causafera_types::WaterVolume::ZERO,
        template.capacity(),
        template.release(),
        template.inlet_capacity_per_tick(),
        template.last_change(),
        causafera_types::WaterVolume::ZERO,
    )
    .expect("an off-map edge is still a well-formed edge");

    snapshot.hydrology.conveyance =
        causafera_geography::HydrologyConveyanceGraph::new(vec![offmap]).expect("one edge");
    // Rebuilt so the registry still addresses exactly its own carriers; without
    // this the refusal would be the registry's and would prove nothing.
    snapshot.hydrology.registry = causafera_runtime::HydrologyObjectRegistry::assign(
        snapshot.hydrology.registry.cells().keys().copied(),
        [key],
        snapshot
            .hydrology
            .registry
            .forcing()
            .keys()
            .copied()
            .collect::<Vec<_>>(),
        snapshot.hydrology.registry.resolution().keys().copied(),
    );

    let bytes = encode_hydrology_section(&snapshot.hydrology);
    assert!(
        decode_hydrology_section(&bytes).is_ok(),
        "the forgery has to survive decoding, or it tests the decoder instead"
    );
    let error = import_with_section(bytes).expect_err("an off-map edge must be refused");
    assert!(
        error.contains("not resident"),
        "unexpected refusal: {error}"
    );
}

// ---------------------------------------------------------------------------
// Provenance
// ---------------------------------------------------------------------------

/// A snapshot after `ticks`, and one trace that exists but is neither the
/// hydrology bootstrap origin nor a conservation event: the very first event the
/// world committed, which is the terrain stage's.
fn evolved_with_a_foreign_trace(
    config: causafera_runtime::RuntimeConfig,
    ticks: u64,
) -> (causafera_runtime::RuntimeSnapshotData, TraceId) {
    let mut runtime = Runtime::new(config).expect("construction");
    runtime.run_ticks(ticks).expect("the run must commit");
    let snapshot = runtime.export_snapshot().expect("export");
    let foreign = snapshot.traces.events[0].trace_id;
    assert!(
        !snapshot.hydrology.retained_batches.contains(&foreign),
        "the first committed event predates every hydrology batch"
    );
    (snapshot, foreign)
}

#[test]
fn a_forcing_record_naming_an_unknown_origin_is_refused() {
    let (mut snapshot, _) = evolved_with_a_foreign_trace(enabled_runtime_config(), 4);
    let record = &snapshot.hydrology.forcing[0];
    snapshot.hydrology.forcing = vec![rewrite_origin(record, TraceId::new(u64::MAX))];

    let error = reimport(&snapshot).expect_err("an unknown origin must be refused");
    assert!(
        error.contains("does not hold"),
        "unexpected refusal: {error}"
    );
}

#[test]
fn a_forcing_record_naming_a_trace_that_is_not_a_bootstrap_origin_is_refused() {
    // Existence alone is not ancestry. A record pointed at some other committed
    // event would answer "where did this water come from" with a real trace that
    // has nothing to do with water.
    let (mut snapshot, foreign) = evolved_with_a_foreign_trace(enabled_runtime_config(), 4);
    let record = &snapshot.hydrology.forcing[0];
    snapshot.hydrology.forcing = vec![rewrite_origin(record, foreign)];

    let error = reimport(&snapshot).expect_err("a foreign origin must be refused");
    assert!(error.contains("ancestry"), "unexpected refusal: {error}");
}

#[test]
fn a_forcing_record_declaring_its_own_producer_policy_cannot_be_constructed() {
    // Configuration cannot choose its producer policy — that would let a session
    // declare itself an authorized producer — and neither can a snapshot: the
    // record's own constructor knows the one policy the bootstrap stage applies,
    // so there is no section shape that carries another. Asserted at the
    // constructor rather than through a snapshot, because a test that had to
    // build one first would be testing the harness.
    let (snapshot, _) = evolved_with_a_foreign_trace(enabled_runtime_config(), 4);
    let record = &snapshot.hydrology.forcing[0];
    assert_eq!(
        record.producer_policy_schema(),
        causafera_geography::BOOTSTRAP_HYDROLOGY_FORCING_POLICY_V1
    );
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
                producer_policy_schema: 999,
                applied_at: record.applied_at(),
            },
        )
        .is_err(),
        "a self-declared producer policy is not a record"
    );
}

#[test]
fn a_bucket_anchored_to_an_unknown_trace_is_refused() {
    // An anchor is the whole of a bucket's provenance. One that resolves to
    // nothing leaves the cell describing a settlement that never happened, and
    // every later query would report it as fact.
    let mut runtime = Runtime::new(wet_runtime_config()).expect("construction");
    runtime.run_ticks(2).expect("the run must commit");
    let mut snapshot = runtime.export_snapshot().expect("export");

    let chunk = *snapshot
        .hydrology
        .fields
        .fields()
        .keys()
        .next()
        .expect("the fixture is resident somewhere");
    let cell = causafera_geography::HydrologyCellKey::new(chunk, 0).expect("ordinal in range");
    snapshot
        .hydrology
        .fields
        .install_surface_trace(cell, TraceId::new(u64::MAX))
        .expect("installing an anchor does not check the store");

    let bytes = encode_hydrology_section(&snapshot.hydrology);
    let error = import_with_section(bytes).expect_err("an unknown anchor must be refused");
    assert!(
        error.contains("does not hold"),
        "unexpected refusal: {error}"
    );
}

#[test]
fn a_retained_batch_anchored_to_a_non_conservation_event_is_refused() {
    // A batch is keyed by the conservation event that closed it, and only a
    // committed tick produces one. Keyed to anything else, the ledger is
    // attributed to something that never balanced it.
    // Three ticks, because the harness imports the forged section into a store
    // built by a three-tick run of the same configuration; a section from a
    // longer run would carry anchors that store has never heard of, and the
    // refusal would be about the harness rather than about the batch key.
    let (mut snapshot, foreign) = evolved_with_a_foreign_trace(wet_runtime_config(), 3);
    let trace = *snapshot
        .hydrology
        .retained_batches
        .last()
        .expect("a batch was retained");
    let receipts = snapshot
        .hydrology
        .receipts
        .remove(&trace)
        .expect("receipts");
    let ledger = snapshot
        .hydrology
        .conservation_receipts
        .remove(&trace)
        .expect("ledger");
    let last = snapshot.hydrology.retained_batches.len() - 1;
    snapshot.hydrology.retained_batches[last] = foreign;
    snapshot.hydrology.receipts.insert(foreign, receipts);
    snapshot
        .hydrology
        .conservation_receipts
        .insert(foreign, ledger);

    let bytes = encode_hydrology_section(&snapshot.hydrology);
    let error = import_with_section(bytes).expect_err("a foreign batch key must be refused");
    assert!(
        error.contains("conservation event"),
        "unexpected refusal: {error}"
    );
}

fn rewrite_origin(
    record: &causafera_geography::HydrologyForcingRecord,
    origin: TraceId,
) -> causafera_geography::HydrologyForcingRecord {
    causafera_geography::HydrologyForcingRecord::new(causafera_geography::HydrologyForcingParts {
        forcing_id: record.forcing_id(),
        scheduled_tick: record.scheduled_tick(),
        targets: record.targets().to_vec(),
        precipitation_volume: record.precipitation_volume(),
        potential_et_volume: record.potential_et_volume(),
        external_inflow_volume: record.external_inflow_volume(),
        origin_trace: origin,
        producer_policy_schema: record.producer_policy_schema(),
        applied_at: record.applied_at(),
    })
    .expect("the rewritten record is well formed")
}

// ---------------------------------------------------------------------------
// Agreement with the configuration that bootstrapped the state
// ---------------------------------------------------------------------------

#[test]
fn a_grid_metric_that_disagrees_with_the_configuration_is_refused() {
    // The metric decides what every volume in the section means as a depth. A
    // section carrying its own would evaluate a different world than the one this
    // configuration bootstrapped, with the numbers of the one it did not.
    let mut runtime = Runtime::new(wet_runtime_config()).expect("construction");
    runtime.run_ticks(2).expect("the run must commit");
    let mut snapshot = runtime.export_snapshot().expect("export");

    let chart = support_hydrology::chart();
    let metric = snapshot.hydrology.metrics.get(chart).expect("the metric");
    let doubled = causafera_geography::HydrologyGridMetric::new(
        std::num::NonZeroU64::new(metric.cell_area_mm2().get() * 2).expect("positive"),
        metric.orthogonal_edge_length_mm(),
        metric.timestep_millis(),
    );
    snapshot.hydrology.metrics =
        causafera_geography::HydrologyGridMetrics::new(vec![(chart, doubled)]).expect("one chart");

    let bytes = encode_hydrology_section(&snapshot.hydrology);
    let error = import_with_section(bytes).expect_err("a rewritten metric must be refused");
    assert!(
        error.contains("grid metrics"),
        "unexpected refusal: {error}"
    );
}

#[test]
fn a_forcing_record_that_disagrees_with_the_configuration_is_refused() {
    // Records are installed one-for-one from the configured specs. A section that
    // rains more than its recipe scheduled is a section that added water to a
    // world nobody configured to receive it.
    let mut runtime = Runtime::new(enabled_runtime_config()).expect("construction");
    runtime.run_ticks(2).expect("the run must commit");
    let mut snapshot = runtime.export_snapshot().expect("export");

    let record = &snapshot.hydrology.forcing[0];
    assert!(record.applied_at().is_none(), "still pending at tick two");
    snapshot.hydrology.forcing = vec![
        causafera_geography::HydrologyForcingRecord::new(
            causafera_geography::HydrologyForcingParts {
                forcing_id: record.forcing_id(),
                scheduled_tick: record.scheduled_tick(),
                targets: record.targets().to_vec(),
                precipitation_volume: causafera_types::WaterVolume::new(
                    record.precipitation_volume().get() + 1,
                ),
                potential_et_volume: record.potential_et_volume(),
                external_inflow_volume: record.external_inflow_volume(),
                origin_trace: record.origin_trace(),
                producer_policy_schema: record.producer_policy_schema(),
                applied_at: record.applied_at(),
            },
        )
        .expect("one more cubic millimetre is still a record"),
    ];

    // The harness imports against `wet_runtime_config`, whose schedule is empty,
    // so this test drives the import itself to compare against the recipe that
    // actually produced the state.
    let error = reimport(&snapshot).expect_err("a rewritten record must be refused");
    assert!(
        error.contains("forcing records"),
        "unexpected refusal: {error}"
    );
}

/// Re-import a snapshot through its own envelope, with its own recipe.
fn reimport(snapshot: &causafera_runtime::RuntimeSnapshotData) -> Result<(), String> {
    let envelope = assemble_envelope(snapshot).map_err(|error| error.to_string())?;
    let data = disassemble_envelope(&envelope).map_err(|error| error.to_string())?;
    RuntimeState::import_snapshot(data)
        .map(|_| ())
        .map_err(|error| error.to_string())
}

// ---------------------------------------------------------------------------
// Batch continuity
// ---------------------------------------------------------------------------

#[test]
fn a_gap_in_the_retained_batch_window_is_refused() {
    // Retention pushes the newest batch and evicts whole ones from the front, so
    // the window is contiguous by construction. A missing middle batch is not
    // something eviction can produce — it is a batch removed from the record, and
    // the ledgers either side of it then attest to a storage-before that no
    // retained batch accounts for.
    let mut runtime = Runtime::new(wet_runtime_config()).expect("construction");
    runtime.run_ticks(4).expect("the run must commit");
    let mut snapshot = runtime.export_snapshot().expect("export");
    assert!(
        snapshot.hydrology.retained_batches.len() >= 3,
        "the window has to have a middle to remove"
    );

    let middle = snapshot.hydrology.retained_batches.remove(1);
    snapshot.hydrology.receipts.remove(&middle);
    snapshot.hydrology.conservation_receipts.remove(&middle);

    let bytes = encode_hydrology_section(&snapshot.hydrology);
    let error = import_with_section(bytes).expect_err("a gap must be refused");
    assert!(error.contains("contiguous"), "unexpected refusal: {error}");
}

#[test]
fn a_retained_window_that_does_not_reach_the_current_batch_is_refused() {
    // Dropping the newest batch instead of the oldest keeps the window
    // contiguous, so this is the same rule seen from the other end: the newest
    // retained batch is the one the field set counts.
    let mut runtime = Runtime::new(wet_runtime_config()).expect("construction");
    runtime.run_ticks(4).expect("the run must commit");
    let mut snapshot = runtime.export_snapshot().expect("export");

    let newest = snapshot
        .hydrology
        .retained_batches
        .pop()
        .expect("a batch was retained");
    snapshot.hydrology.receipts.remove(&newest);
    snapshot.hydrology.conservation_receipts.remove(&newest);

    let bytes = encode_hydrology_section(&snapshot.hydrology);
    let error = import_with_section(bytes).expect_err("a truncated window must be refused");
    assert!(error.contains("contiguous"), "unexpected refusal: {error}");
}

#[test]
fn a_transfer_receipt_filed_under_another_batch_is_refused() {
    // The ledger comparison alone cannot catch this: totals are computed from
    // whatever is in the list, so a receipt moved between batches would be
    // recomputed into agreement with the batch it was moved into.
    let mut runtime = Runtime::new(wet_runtime_config()).expect("construction");
    runtime.run_ticks(4).expect("the run must commit");
    let mut snapshot = runtime.export_snapshot().expect("export");

    let newest = *snapshot
        .hydrology
        .retained_batches
        .last()
        .expect("a batch was retained");
    let older = snapshot.hydrology.retained_batches[0];
    let stolen = snapshot.hydrology.receipts[&older][0].clone();
    snapshot
        .hydrology
        .receipts
        .get_mut(&newest)
        .expect("the newest batch")
        .push(stolen);

    let bytes = encode_hydrology_section(&snapshot.hydrology);
    let error = import_with_section(bytes).expect_err("a misfiled receipt must be refused");
    assert!(
        error.contains("does not belong") || error.contains("recompute"),
        "unexpected refusal: {error}"
    );
}

// ---------------------------------------------------------------------------
// Composed bounds, before allocation
// ---------------------------------------------------------------------------

#[test]
fn a_schedule_past_the_aggregate_member_cap_is_refused_before_allocation() {
    // The per-record target cap and the record-count cap are each satisfied by a
    // schedule 128 times past the aggregate one. The decoder therefore has to
    // carry the running total itself: by the time a constructor could object, the
    // member vectors are already allocated.
    let mut runtime = Runtime::new(wet_runtime_config()).expect("construction");
    runtime.run_ticks(1).expect("the run must commit");
    let mut snapshot = runtime.export_snapshot().expect("export");

    let chunk = *snapshot
        .hydrology
        .fields
        .fields()
        .keys()
        .next()
        .expect("resident somewhere");
    let weight = std::num::NonZeroU64::new(1).expect("positive");
    // Ascending and unique, which the record constructor requires: one member per
    // cell of one chunk, which is as many as a record over this fixture can name.
    let targets: Vec<causafera_geography::HydrologyForcingMember> = (0..1_024_u16)
        .map(|ordinal| {
            causafera_geography::HydrologyForcingMember::new(
                causafera_geography::HydrologyCellKey::new(chunk, ordinal)
                    .expect("ordinal in range"),
                weight,
            )
        })
        .collect();

    let origin = snapshot.hydrology.fields.conservation_last_change();
    let records = (0..257_u64)
        .map(|id| {
            causafera_geography::HydrologyForcingRecord::new(
                causafera_geography::HydrologyForcingParts {
                    forcing_id: id,
                    scheduled_tick: 1_000 + id,
                    targets: targets.clone(),
                    precipitation_volume: causafera_types::WaterVolume::new(1),
                    potential_et_volume: causafera_types::WaterVolume::ZERO,
                    external_inflow_volume: causafera_types::WaterVolume::ZERO,
                    origin_trace: origin,
                    producer_policy_schema:
                        causafera_geography::BOOTSTRAP_HYDROLOGY_FORCING_POLICY_V1,
                    applied_at: None,
                },
            )
            .expect("each record is individually well formed")
        })
        .collect::<Vec<_>>();
    assert_eq!(
        records.len() * targets.len(),
        263_168,
        "257 x 1024 is past the 262_144 aggregate and inside both per-part caps"
    );
    snapshot.hydrology.forcing = records;

    let bytes = encode_hydrology_section(&snapshot.hydrology);
    let error = decode_hydrology_section(&bytes)
        .expect_err("a schedule past the aggregate cap must be refused")
        .to_string();
    assert!(
        error.contains("member count"),
        "unexpected refusal: {error}"
    );
}

#[test]
fn a_recipe_schedule_past_the_aggregate_member_cap_is_refused_before_allocation() {
    // The schedule is persisted twice — as applied state in the hydrology section
    // and as configuration in the recipe — and the aggregate cap binds both. The
    // recipe path has a validator that holds it, but only once the whole schedule
    // is a value to hand it, which is one allocation too late.
    let mut runtime = Runtime::new(wet_runtime_config()).expect("construction");
    runtime.run_ticks(1).expect("the run must commit");
    let mut snapshot = runtime.export_snapshot().expect("export");

    let chunk = *snapshot
        .hydrology
        .fields
        .fields()
        .keys()
        .next()
        .expect("resident somewhere");
    let weight = std::num::NonZeroU64::new(1).expect("positive");
    type Target = (causafera_geography::HydrologyCellKey, std::num::NonZeroU64);
    let targets: Vec<Target> = (0..1_024_u16)
        .map(|ordinal| {
            (
                causafera_geography::HydrologyCellKey::new(chunk, ordinal)
                    .expect("ordinal in range"),
                weight,
            )
        })
        .collect();
    let specs = (0..257_u64)
        .map(|id| causafera_runtime::HydrologyForcingSpec {
            forcing_id: id,
            scheduled_tick: 1_000 + id,
            targets: targets.clone(),
            precipitation_volume: causafera_types::WaterVolume::new(1),
            potential_et_volume: causafera_types::WaterVolume::ZERO,
            external_inflow_volume: causafera_types::WaterVolume::ZERO,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        specs.len() * targets.len(),
        263_168,
        "257 x 1024 is past the 262_144 aggregate and inside both per-part caps"
    );
    snapshot.recipe.config.hydrology.forcing_schedule = specs;

    let bytes = encode_runtime_recipe_section(&snapshot.recipe);
    let error = decode_runtime_recipe_section(&bytes)
        .expect_err("a recipe schedule past the aggregate cap must be refused")
        .to_string();
    assert!(
        error.contains("member count"),
        "unexpected refusal: {error}"
    );
}

#[test]
fn retained_receipts_past_the_retention_cap_are_refused_before_allocation() {
    // Retention bounds the retained receipts across every batch, so the per-batch
    // cap alone would let eight batches declare eight times what a live session
    // may ever hold — two million receipt slots reserved before a single one is
    // read. The forgery is a declared count, patched in the encoded bytes,
    // because a state that genuinely carried them would have to allocate them.
    let mut runtime = Runtime::new(wet_runtime_config()).expect("construction");
    runtime.run_ticks(3).expect("the run must commit");
    let snapshot = runtime.export_snapshot().expect("export");
    let newest = *snapshot
        .hydrology
        .retained_batches
        .last()
        .expect("a batch was retained");
    let declared = snapshot.hydrology.receipts[&newest].len() as u64;
    assert!(declared > 0, "the newest batch has receipts to count");

    let bytes = encode_hydrology_section(&snapshot.hydrology);
    // The batch's own trace immediately precedes its receipt count, and a trace
    // ID is unique within the section, so this locates the count without
    // restating the section layout.
    let mut needle = newest.raw().to_le_bytes().to_vec();
    needle.extend_from_slice(&declared.to_le_bytes());
    let at = bytes
        .windows(needle.len())
        .position(|window| window == needle)
        .expect("the batch header is in the encoded section");
    let mut forged = bytes.clone();
    let count = at + 8;
    forged[count..count + 8].copy_from_slice(
        &(causafera_geography::MAX_HYDROLOGY_PERSISTED_TRANSFER_RECEIPTS as u64).to_le_bytes(),
    );

    let error = decode_hydrology_section(&forged)
        .expect_err("the composed retention cap must refuse this")
        .to_string();
    assert!(
        error.contains("retained receipt count"),
        "unexpected refusal: {error}"
    );
}

#[test]
fn a_resolution_policy_that_disagrees_with_the_configuration_is_refused() {
    // Every imported level is checked against the policy, so a section carrying
    // its own would be the only thing deciding whether its own detail is
    // acceptable. Raising it to fit is the clamp the contract refuses, moved one
    // layer earlier.
    let mut runtime = Runtime::new(wet_runtime_config()).expect("construction");
    runtime.run_ticks(2).expect("the run must commit");
    let mut snapshot = runtime.export_snapshot().expect("export");

    let policy = snapshot.hydrology.resolution_policy;
    assert!(policy.enabled, "the fixture runs with a policy");
    snapshot.hydrology.resolution_policy = causafera_domains::HydrologyResolutionPolicy {
        max_level: policy.max_level - 1,
        ..policy
    };

    let bytes = encode_hydrology_section(&snapshot.hydrology);
    let error = import_with_section(bytes).expect_err("a rewritten policy must be refused");
    assert!(
        error.contains("resolution policy"),
        "unexpected refusal: {error}"
    );
}
