//! Runtime-side hydrology: configuration validation and canonical recipe
//! encoding.
//!
//! Covers the configuration half of `plans/hydrology.md` §4 and the Stage 6 work
//! item "add and validate `HydrologyConfig`, with disabled default and bounded
//! explicit enablement".

use std::collections::BTreeMap;
use std::num::{NonZeroU32, NonZeroU64};

use causafera_domains::HydrologyResolutionPolicy;
use causafera_geography::{
    FluxBoundary, HydrologyBoundaryCondition, HydrologyCellKey, HydrologyGridMetric,
};
use causafera_runtime::snapshot_sections::{
    assemble_envelope, decode_runtime_recipe_section, encode_runtime_recipe_section,
};
use causafera_runtime::{
    BOOTSTRAP_STAGE_COUNT, HYDROLOGY_BOOTSTRAP_BOUNDARIES_PROPERTY,
    HYDROLOGY_BOOTSTRAP_EDGES_PROPERTY, HYDROLOGY_BOOTSTRAP_EVENT_KIND,
    HYDROLOGY_BOOTSTRAP_FORCING_PROPERTY, HYDROLOGY_BOOTSTRAP_METRICS_PROPERTY,
    HYDROLOGY_BOOTSTRAP_OBJECT_KIND, HYDROLOGY_BOOTSTRAP_PARAMETERS_SCHEMA_V1,
    HYDROLOGY_BOOTSTRAP_RESOLUTION_PROPERTY, HYDROLOGY_BOOTSTRAP_STAGE_COUNT,
    HYDROLOGY_BOOTSTRAP_STORAGE_PROPERTY, HYDROLOGY_BOOTSTRAP_SUBSTRATE_PROPERTY,
    HYDROLOGY_LIMITS_SCHEMA_V1, HYDROLOGY_PROCESS_SCHEMA, HYDROLOGY_STAGE,
    HydrologyBootstrapOverride, HydrologyBootstrapParameters, HydrologyConfig,
    HydrologyForcingSpec, Runtime, RuntimeConfig, RuntimeError, THERMAL_RESERVOIR_STAGE,
};
use causafera_types::HistoricalStageId;
use causafera_types::{ChartChunkCoord, ChunkCoord, SpatialChartId, WaterVolume};

fn chart() -> SpatialChartId {
    SpatialChartId::new(1)
}

fn cell(ordinal: u16) -> HydrologyCellKey {
    HydrologyCellKey::new(
        ChartChunkCoord::new(chart(), ChunkCoord::new(0, 0, 0)),
        ordinal,
    )
    .expect("the ordinal is in range")
}

fn nz32(value: u32) -> NonZeroU32 {
    NonZeroU32::new(value).expect("the value is positive")
}

fn nz64(value: u64) -> NonZeroU64 {
    NonZeroU64::new(value).expect("the value is positive")
}

fn metric() -> HydrologyGridMetric {
    HydrologyGridMetric::new(nz64(1_000_000), nz64(1_000), nz64(1_000))
}

fn parameters() -> HydrologyBootstrapParameters {
    HydrologyBootstrapParameters {
        schema_version: HYDROLOGY_BOOTSTRAP_PARAMETERS_SCHEMA_V1,
        default_surface_capacity: WaterVolume::new(1_000_000),
        default_soil_capacity: WaterVolume::new(500_000),
        default_groundwater_capacity: WaterVolume::new(2_000_000),
        initial_surface: WaterVolume::new(1_000),
        initial_soil: WaterVolume::new(2_000),
        initial_groundwater: WaterVolume::new(3_000),
        infiltration_rate_mm_per_second: 4,
        percolation_fraction_num: 1,
        percolation_fraction_den: nz32(4),
        specific_yield_num: 1,
        specific_yield_den: nz32(5),
        aquifer_base_offset_mm: -2_500,
        baseflow_threshold: WaterVolume::new(500),
        baseflow_fraction_num: 1,
        baseflow_fraction_den: nz32(8),
        base_surface_transmissivity_mm3_per_second: 7,
        base_groundwater_transmissivity_mm3_per_second: 3,
        roughness_reference_mm: nz64(50),
        conveyance_capacity: WaterVolume::new(100_000),
        conveyance_initial_storage: WaterVolume::new(1_000),
        conveyance_inlet_capacity_per_tick: WaterVolume::new(10_000),
        conveyance_release_fraction_num: 1,
        conveyance_release_fraction_den: nz32(4),
        default_boundary: HydrologyBoundaryCondition::CLOSED,
        chart_overrides: BTreeMap::new(),
        cell_overrides: BTreeMap::new(),
    }
}

fn enabled_config() -> HydrologyConfig {
    HydrologyConfig {
        enabled: true,
        grid_metrics: [(chart(), metric())].into_iter().collect(),
        bootstrap_parameters: Some(parameters()),
        forcing_schedule: vec![HydrologyForcingSpec {
            forcing_id: 1,
            scheduled_tick: 3,
            targets: vec![(cell(0), nz64(1)), (cell(1), nz64(3))],
            precipitation_volume: WaterVolume::new(9_000),
            potential_et_volume: WaterVolume::new(1_200),
            external_inflow_volume: WaterVolume::ZERO,
        }],
        // Level four, not two: the engine's resolution field drives hydrology's
        // level and its default policy reaches level three, which a lower
        // hydrology policy refuses rather than clamps. See
        // `an_engine_level_above_the_policy_refuses_the_tick`.
        resolution_policy: HydrologyResolutionPolicy::enabled(4)
            .expect("level four is the maximum"),
        limits_schema: HYDROLOGY_LIMITS_SCHEMA_V1,
    }
}

/// Every drop the world holds, cells and conveyance together.
fn total_water(state: &causafera_runtime::HydrologyRuntimeState) -> i128 {
    let cells = state
        .fields
        .total_storage()
        .expect("totals stay in range")
        .get();
    let edges: i128 = state
        .conveyance
        .edges()
        .values()
        .map(|edge| edge.storage().as_i128())
        .sum();
    cells + edges
}

fn config_with(hydrology: HydrologyConfig) -> RuntimeConfig {
    let mut config = RuntimeConfig::new(20_260_730);
    config.hydrology = hydrology;
    config
}

// ---------------------------------------------------------------------------
// The disabled default
// ---------------------------------------------------------------------------

#[test]
fn a_new_runtime_config_has_hydrology_disabled_and_empty() {
    // No field, no edge, no boundary, and no rainfall is defaulted into an
    // existing session.
    let config = RuntimeConfig::new(1);
    assert!(!config.hydrology.enabled);
    assert!(config.hydrology.grid_metrics.is_empty());
    assert!(config.hydrology.bootstrap_parameters.is_none());
    assert!(config.hydrology.forcing_schedule.is_empty());
    assert_eq!(
        config.hydrology.resolution_policy,
        HydrologyResolutionPolicy::DISABLED
    );
    assert_eq!(config.hydrology.limits_schema, HYDROLOGY_LIMITS_SCHEMA_V1);
    assert_eq!(config.hydrology, HydrologyConfig::disabled());
    Runtime::new(config).expect("a disabled hydrology session must construct");
}

#[test]
fn a_disabled_configuration_carrying_state_it_cannot_use_is_refused() {
    // "Disabled" has one canonical shape. Each of these is a configuration whose
    // author believed something the runtime does not, and each is rejected rather
    // than silently ignored.
    let mut with_parameters = HydrologyConfig::disabled();
    with_parameters.bootstrap_parameters = Some(parameters());

    let mut with_metrics = HydrologyConfig::disabled();
    with_metrics.grid_metrics = [(chart(), metric())].into_iter().collect();

    let mut with_forcing = HydrologyConfig::disabled();
    with_forcing.forcing_schedule = enabled_config().forcing_schedule;

    let mut with_resolution = HydrologyConfig::disabled();
    with_resolution.resolution_policy =
        HydrologyResolutionPolicy::enabled(1).expect("level one is valid");

    for candidate in [with_parameters, with_metrics, with_forcing, with_resolution] {
        assert!(
            matches!(
                Runtime::new(config_with(candidate)),
                Err(RuntimeError::HydrologyDisabledConfigNotCanonical)
            ),
            "a disabled configuration must be canonically empty"
        );
    }
}

// ---------------------------------------------------------------------------
// Bounded explicit enablement
// ---------------------------------------------------------------------------

#[test]
fn an_enabled_configuration_requires_parameters_and_an_explicit_metric() {
    let mut without_parameters = enabled_config();
    without_parameters.bootstrap_parameters = None;
    assert!(matches!(
        Runtime::new(config_with(without_parameters)),
        Err(RuntimeError::HydrologyEnabledWithoutParameters)
    ));

    // The metric is registered, never inferred from `chunk_extent`, containment,
    // observer zoom, or UI scale — so an enabled session without one cannot run.
    let mut without_metric = enabled_config();
    without_metric.grid_metrics.clear();
    assert!(matches!(
        Runtime::new(config_with(without_metric)),
        Err(RuntimeError::HydrologyMetricMissing)
    ));

    Runtime::new(config_with(enabled_config()))
        .expect("a complete hydrology configuration must construct");
}

#[test]
fn an_unknown_schema_version_is_refused_rather_than_assumed() {
    let mut limits = enabled_config();
    limits.limits_schema = 2;
    assert!(matches!(
        Runtime::new(config_with(limits)),
        Err(RuntimeError::HydrologyLimitsSchemaUnknown { schema: 2 })
    ));

    let mut bootstrap = enabled_config();
    bootstrap.bootstrap_parameters = Some(HydrologyBootstrapParameters {
        schema_version: 9,
        ..parameters()
    });
    assert!(matches!(
        Runtime::new(config_with(bootstrap)),
        Err(RuntimeError::HydrologyBootstrapParametersSchemaUnknown { schema: 9 })
    ));
}

#[test]
fn a_fraction_outside_the_unit_interval_is_refused() {
    for mutate in [
        (|p: &mut HydrologyBootstrapParameters| p.percolation_fraction_num = 5)
            as fn(&mut HydrologyBootstrapParameters),
        |p: &mut HydrologyBootstrapParameters| p.specific_yield_num = 6,
        |p: &mut HydrologyBootstrapParameters| p.baseflow_fraction_num = 9,
        |p: &mut HydrologyBootstrapParameters| p.conveyance_release_fraction_num = 5,
    ] {
        let mut broken = parameters();
        mutate(&mut broken);
        let mut config = enabled_config();
        config.bootstrap_parameters = Some(broken);
        assert!(matches!(
            Runtime::new(config_with(config)),
            Err(RuntimeError::HydrologyFractionOutOfRange { .. })
        ));
    }
}

#[test]
fn initial_storage_above_its_own_capacity_is_refused_not_clamped() {
    // A state no process could produce, so it can only arrive by configuration.
    // Clamping would silently discard the difference.
    for mutate in [
        (|p: &mut HydrologyBootstrapParameters| {
            p.initial_surface = WaterVolume::new(p.default_surface_capacity.get() + 1);
        }) as fn(&mut HydrologyBootstrapParameters),
        |p: &mut HydrologyBootstrapParameters| {
            p.initial_soil = WaterVolume::new(p.default_soil_capacity.get() + 1);
        },
        |p: &mut HydrologyBootstrapParameters| {
            p.initial_groundwater = WaterVolume::new(p.default_groundwater_capacity.get() + 1);
        },
        |p: &mut HydrologyBootstrapParameters| {
            p.conveyance_initial_storage = WaterVolume::new(p.conveyance_capacity.get() + 1);
        },
    ] {
        let mut broken = parameters();
        mutate(&mut broken);
        let mut config = enabled_config();
        config.bootstrap_parameters = Some(broken);
        assert!(matches!(
            Runtime::new(config_with(config)),
            Err(RuntimeError::HydrologyInitialStorageExceedsCapacity)
        ));
    }
}

#[test]
fn groundwater_capacity_without_a_specific_yield_is_refused() {
    // Saturated depth divides by `cell_area * specific_yield_num`, so an aquifer
    // without a yield has no water table.
    let mut broken = parameters();
    broken.specific_yield_num = 0;
    let mut config = enabled_config();
    config.bootstrap_parameters = Some(broken);
    assert!(matches!(
        Runtime::new(config_with(config)),
        Err(RuntimeError::HydrologyZeroSpecificYield)
    ));
}

// ---------------------------------------------------------------------------
// The forcing schedule
// ---------------------------------------------------------------------------

#[test]
fn a_forcing_record_scheduled_at_or_before_bootstrap_is_refused() {
    // Production bootstrap completes before the first tick, so a record at tick
    // zero is not in the future. Checked subtraction is what keeps a record
    // "before bootstrap" from wrapping around into a near-term one.
    let mut config = enabled_config();
    config.forcing_schedule[0].scheduled_tick = 0;
    assert!(
        matches!(
            Runtime::new(config_with(config)),
            Err(RuntimeError::HydrologyForcingScheduledTooEarly {
                scheduled_tick: 0,
                bootstrap_tick: 0
            })
        ),
        "tick zero is not after bootstrap"
    );

    // And tick one is, so the boundary is exact.
    let mut edge = enabled_config();
    edge.forcing_schedule[0].scheduled_tick = 1;
    Runtime::new(config_with(edge)).expect("the first tick after bootstrap is admissible");
}

#[test]
fn a_forcing_record_beyond_the_horizon_is_refused() {
    let mut config = enabled_config();
    config.forcing_schedule[0].scheduled_tick =
        causafera_geography::MAX_HYDROLOGY_FORCING_HORIZON_TICKS + 1;
    assert!(matches!(
        Runtime::new(config_with(config)),
        Err(RuntimeError::HydrologyForcingBeyondHorizon { .. })
    ));

    // And the last admissible tick is admitted, so the bound is exact rather
    // than approximately right.
    let mut edge = enabled_config();
    edge.forcing_schedule[0].scheduled_tick =
        causafera_geography::MAX_HYDROLOGY_FORCING_HORIZON_TICKS;
    Runtime::new(config_with(edge)).expect("the horizon itself is inside the horizon");
}

#[test]
fn an_unsorted_duplicated_or_empty_forcing_schedule_is_refused() {
    let spec = |forcing_id: u64, scheduled_tick: u64| HydrologyForcingSpec {
        forcing_id,
        scheduled_tick,
        targets: vec![(cell(0), nz64(1))],
        precipitation_volume: WaterVolume::new(10),
        potential_et_volume: WaterVolume::ZERO,
        external_inflow_volume: WaterVolume::ZERO,
    };

    // Unsorted by `(scheduled_tick, forcing_id)`.
    let mut unsorted = enabled_config();
    unsorted.forcing_schedule = vec![spec(1, 5), spec(1, 3)];

    // Duplicated key.
    let mut duplicated = enabled_config();
    duplicated.forcing_schedule = vec![spec(1, 3), spec(1, 3)];

    // A record with no target has no cell to allocate to.
    let mut empty_targets = enabled_config();
    empty_targets.forcing_schedule = vec![HydrologyForcingSpec {
        targets: Vec::new(),
        ..spec(1, 3)
    }];

    // Targets unsorted within one record.
    let mut unsorted_targets = enabled_config();
    unsorted_targets.forcing_schedule = vec![HydrologyForcingSpec {
        targets: vec![(cell(4), nz64(1)), (cell(2), nz64(1))],
        ..spec(1, 3)
    }];

    // Duplicated target within one record.
    let mut duplicated_targets = enabled_config();
    duplicated_targets.forcing_schedule = vec![HydrologyForcingSpec {
        targets: vec![(cell(2), nz64(1)), (cell(2), nz64(1))],
        ..spec(1, 3)
    }];

    for candidate in [
        unsorted,
        duplicated,
        empty_targets,
        unsorted_targets,
        duplicated_targets,
    ] {
        assert!(
            matches!(
                Runtime::new(config_with(candidate)),
                Err(RuntimeError::HydrologyForcingScheduleNotCanonical)
            ),
            "a forcing schedule must be sorted, unique, and non-empty per record"
        );
    }
}

#[test]
fn a_resolution_level_above_the_representable_maximum_is_refused() {
    let mut config = enabled_config();
    config.resolution_policy = HydrologyResolutionPolicy {
        schema_version: HydrologyResolutionPolicy::SCHEMA_VERSION,
        enabled: true,
        max_level: 9,
    };
    assert!(matches!(
        Runtime::new(config_with(config)),
        Err(RuntimeError::HydrologyResolutionLevelUnsupported { level: 9 })
    ));
}

// ---------------------------------------------------------------------------
// The canonical recipe encoding
// ---------------------------------------------------------------------------

#[test]
fn a_disabled_hydrology_recipe_round_trips_through_the_canonical_encoding() {
    let runtime = Runtime::new(RuntimeConfig::new(4_242)).expect("construction");
    let snapshot = runtime.export_snapshot().expect("state must export");
    let encoded = encode_runtime_recipe_section(&snapshot.recipe);
    let decoded = decode_runtime_recipe_section(&encoded).expect("the recipe must decode");
    assert_eq!(decoded.config.hydrology, HydrologyConfig::disabled());
    assert_eq!(decoded.config, snapshot.recipe.config);
}

#[test]
fn an_enabled_hydrology_recipe_round_trips_every_field() {
    // Including the parts most likely to be dropped by an encoder that was
    // written field-by-field: a negative aquifer offset, per-chart and per-cell
    // overrides, an open boundary on one face only, and a non-empty schedule.
    let mut hydrology = enabled_config();
    let parameters = hydrology
        .bootstrap_parameters
        .as_mut()
        .expect("the fixture is enabled");
    parameters.chart_overrides.insert(
        chart(),
        HydrologyBootstrapOverride {
            surface_capacity: Some(WaterVolume::new(777)),
            // Lowered together with the capacity: an override that leaves the
            // inherited initial storage above its new capacity is refused.
            initial_surface: Some(WaterVolume::new(700)),
            aquifer_base_offset_mm: Some(-9_001),
            specific_yield_num: Some(2),
            specific_yield_den: Some(nz32(7)),
            roughness_reference_mm: Some(nz64(11)),
            ..HydrologyBootstrapOverride::default()
        },
    );
    parameters.cell_overrides.insert(
        cell(9),
        HydrologyBootstrapOverride {
            initial_groundwater: Some(WaterVolume::new(42)),
            face_boundaries: [(
                causafera_geography::FaceDirection::PosY,
                HydrologyBoundaryCondition::new(
                    FluxBoundary::Open {
                        external_head_mm: -120,
                        conductance_mm2_per_tick: 33,
                    },
                    FluxBoundary::NoFlux,
                ),
            )]
            .into_iter()
            .collect(),
            ..HydrologyBootstrapOverride::default()
        },
    );

    let runtime = Runtime::new(config_with(hydrology.clone())).expect("construction");
    let snapshot = runtime.export_snapshot().expect("state must export");
    assert_eq!(snapshot.recipe.config.hydrology, hydrology);

    let encoded = encode_runtime_recipe_section(&snapshot.recipe);
    let decoded = decode_runtime_recipe_section(&encoded).expect("the recipe must decode");
    assert_eq!(
        decoded.config.hydrology, hydrology,
        "every configured field survives the canonical encoding"
    );

    // And re-encoding the decoded value is byte-identical, so the encoding has
    // one representation per configuration rather than several.
    assert_eq!(encode_runtime_recipe_section(&decoded), encoded);
}

#[test]
fn an_enabled_hydrology_session_still_assembles_a_complete_envelope() {
    let runtime = Runtime::new(config_with(enabled_config())).expect("construction");
    let snapshot = runtime.export_snapshot().expect("state must export");
    let envelope = assemble_envelope(&snapshot).expect("an enabled session must assemble");
    envelope.encode().expect("the envelope must encode");
}

#[test]
fn enabling_hydrology_changes_the_recipe_payload() {
    // The recipe describes what a session was configured to be, so two sessions
    // that differ only in whether hydrology is on must not share recipe bytes.
    let disabled = Runtime::new(RuntimeConfig::new(11))
        .expect("construction")
        .export_snapshot()
        .expect("export");
    let mut enabled_runtime_config = config_with(enabled_config());
    enabled_runtime_config.deterministic = disabled.recipe.config.deterministic.clone();
    let enabled = Runtime::new(enabled_runtime_config)
        .expect("construction")
        .export_snapshot()
        .expect("export");

    assert_ne!(
        encode_runtime_recipe_section(&disabled.recipe),
        encode_runtime_recipe_section(&enabled.recipe)
    );
}

// ---------------------------------------------------------------------------
// Causal initialization and the committed tick
// ---------------------------------------------------------------------------

/// A world with water on it: real capacity, real conductance, real infiltration.
fn wet_config() -> RuntimeConfig {
    let mut hydrology = enabled_config();
    hydrology.forcing_schedule.clear();
    let parameters = hydrology
        .bootstrap_parameters
        .as_mut()
        .expect("the fixture is enabled");
    parameters.initial_surface = WaterVolume::new(20_000_000);
    parameters.initial_soil = WaterVolume::new(100_000);
    parameters.initial_groundwater = WaterVolume::new(50_000);
    parameters.default_surface_capacity = WaterVolume::new(1_000_000_000);
    parameters.default_soil_capacity = WaterVolume::new(1_000_000_000);
    parameters.default_groundwater_capacity = WaterVolume::new(1_000_000_000);
    parameters.infiltration_rate_mm_per_second = 200;
    parameters.base_surface_transmissivity_mm3_per_second = 5_000_000;
    parameters.base_groundwater_transmissivity_mm3_per_second = 1_000_000;
    config_with(hydrology)
}

#[test]
fn causal_initialization_builds_a_world_from_configured_numbers_and_real_ground() {
    let runtime = Runtime::new(wet_config()).expect("construction");
    let state = runtime.hydrology_state();

    assert!(state.enabled);
    assert!(!state.fields.fields().is_empty(), "chunks are resident");
    assert_eq!(
        state.fields.cell_count(),
        state.fields.fields().len() * 1_024,
        "every resident chunk holds a full surface lattice"
    );
    assert!(!state.metrics.is_empty(), "the metric is registered");

    // Substrate is derived, not copied: the per-tick infiltration limit comes from
    // the configured per-second rate, the cell area, and the timestep.
    let sample = state
        .fields
        .fields()
        .keys()
        .next()
        .copied()
        .expect("a chunk is resident");
    let ground = state
        .fields
        .field(sample)
        .expect("the field exists")
        .ground(0)
        .expect("the cell exists");
    // 200 mm/s over 1 000 000 mm² for 1 000 ms is 200 000 000 mm³ per tick.
    assert_eq!(
        ground.infiltration_limit_per_tick(),
        WaterVolume::new(200_000_000)
    );
    assert!(
        ground.surface_conductance_mm2_per_tick() > 0,
        "roughness-adjusted transmissivity has to survive the conversion"
    );

    // Every exterior face carries an explicit record, or the first tick would be
    // refused rather than defaulting to a wall.
    assert!(!state.boundaries.is_empty());

    // The object registry is a bijection onto a dense range for every key space.
    assert!(state.registry.is_dense());
    assert_eq!(state.registry.cells().len(), state.fields.cell_count());
    assert_eq!(
        state.registry.edges().len(),
        state.conveyance.len(),
        "every conveyance edge is addressable"
    );
}

#[test]
fn conveyance_edges_run_strictly_downhill_and_leave_local_minima_alone() {
    let runtime = Runtime::new(wet_config()).expect("construction");
    let state = runtime.hydrology_state();
    assert!(
        !state.conveyance.is_empty(),
        "real terrain has to produce some outlets"
    );

    for (key, edge) in state.conveyance.edges() {
        let source = edge.source();
        let outlet = edge.outlet();
        assert!(key.contains(source) && key.contains(outlet));
        assert!(
            source.adjacency(outlet).is_some(),
            "an edge joins two orthogonally adjacent cells"
        );
        assert!(
            state.fields.is_resident(outlet),
            "an edge never leaves the resident world, which no boundary record \
             would account for"
        );
    }
    // At most one outgoing edge per cell is a graph invariant, and every edge
    // strictly lowering elevation makes the graph acyclic.
    let mut sources: Vec<_> = state
        .conveyance
        .edges()
        .values()
        .map(|edge| edge.source())
        .collect();
    let total = sources.len();
    sources.sort();
    sources.dedup();
    assert_eq!(sources.len(), total, "one outgoing edge per cell");
}

#[test]
fn a_disabled_session_holds_no_hydrology_state_and_ticks() {
    let mut runtime = Runtime::new(RuntimeConfig::new(9_001)).expect("construction");
    assert!(!runtime.hydrology_state().enabled);
    assert!(runtime.hydrology_state().fields.fields().is_empty());
    runtime.run_ticks(4).expect("a disabled session must tick");
    assert!(
        runtime.hydrology_state().receipts.is_empty(),
        "a disabled domain produces no batch"
    );
}

#[test]
fn an_enabled_session_commits_a_conserved_batch_every_tick() {
    let mut runtime = Runtime::new(wet_config()).expect("construction");
    let opening = runtime
        .hydrology_state()
        .fields
        .total_storage()
        .expect("the opening total is representable")
        .get();

    for tick in 1..=6_u64 {
        runtime.run_ticks(1).unwrap_or_else(|error| {
            panic!("tick {tick} must commit: {error}");
        });
        let state = runtime.hydrology_state();
        assert_eq!(
            state.retained_batches.len(),
            usize::try_from(tick).expect("the tick count is small"),
            "one retained batch per tick"
        );
        let trace = *state
            .retained_batches
            .last()
            .expect("the tick retained a batch");
        let ledger = state
            .conservation_receipts
            .get(&trace)
            .expect("the batch has a conservation receipt");
        assert_eq!(ledger.residual(), 0, "tick {tick} must close exactly");
        assert_eq!(ledger.tick(), tick);
        assert!(
            !state.receipts[&trace].is_empty(),
            "tick {tick} has to have moved water"
        );
    }

    // Closed world: no forcing and every boundary closed, so the total is
    // unchanged after six ticks of real internal transfer.
    let closing = runtime
        .hydrology_state()
        .fields
        .total_storage()
        .expect("the closing total is representable")
        .get();
    let conveyance = runtime
        .hydrology_state()
        .conveyance
        .total_storage()
        .expect("edge storage is representable")
        .get();
    let opening_conveyance = Runtime::new(wet_config())
        .expect("construction")
        .hydrology_state()
        .conveyance
        .total_storage()
        .expect("edge storage is representable")
        .get();
    assert_eq!(
        closing + conveyance,
        opening + opening_conveyance,
        "a closed world conserves across the run, cells and edges together"
    );
}

#[test]
fn a_committed_tick_anchors_every_bucket_it_changed() {
    let mut runtime = Runtime::new(wet_config()).expect("construction");
    let before = runtime.hydrology_state().fields.clone();
    runtime.run_ticks(1).expect("one tick must commit");
    let after = runtime.hydrology_state();

    // Some bucket moved, and every bucket that moved now points at a trace that
    // is not the bootstrap anchor it started with.
    let mut moved = 0_usize;
    for (chunk, field) in after.fields.fields() {
        let previous = before.field(*chunk).expect("the chunk was resident");
        for (ordinal, cell) in field.cells().iter().enumerate() {
            let was = &previous.cells()[ordinal];
            if cell.surface_water() != was.surface_water() {
                moved += 1;
                assert_ne!(
                    cell.surface_last_change(),
                    was.surface_last_change(),
                    "a changed surface bucket must name the event that changed it"
                );
            }
            if cell.soil_water() != was.soil_water() {
                assert_ne!(cell.soil_last_change(), was.soil_last_change());
            }
            if cell.groundwater() != was.groundwater() {
                assert_ne!(
                    cell.groundwater_last_change(),
                    was.groundwater_last_change()
                );
            }
        }
    }
    assert!(moved > 0, "the tick has to have moved water somewhere");
    assert_ne!(
        after.fields.conservation_last_change(),
        before.conservation_last_change(),
        "the tick's conservation event becomes the new anchor"
    );
    assert!(
        after.next_node_id > 1,
        "the terminal aggregation tree consumed synthetic node identifiers"
    );
}

#[test]
fn retained_typed_batches_are_evicted_whole_at_the_bound() {
    // Eight batches are retained; the ninth tick evicts the oldest whole batch
    // rather than trimming it, because half a tick's receipts would answer a
    // question about that tick with an incomplete ledger.
    let mut runtime = Runtime::new(wet_config()).expect("construction");
    runtime.run_ticks(12).expect("twelve ticks must commit");
    let state = runtime.hydrology_state();
    assert_eq!(state.retained_batches.len(), 8);
    assert_eq!(state.receipts.len(), 8);
    assert_eq!(state.conservation_receipts.len(), 8);
    for trace in &state.retained_batches {
        assert!(
            state.receipts.contains_key(trace) && state.conservation_receipts.contains_key(trace),
            "a retained batch keeps both its transfer and its conservation receipts"
        );
    }
}

#[test]
fn replaying_the_same_configuration_produces_the_same_hydrology_state() {
    let run = || {
        let mut runtime = Runtime::new(wet_config()).expect("construction");
        runtime.run_ticks(5).expect("five ticks must commit");
        runtime.hydrology_state().clone()
    };
    let first = run();
    let second = run();
    assert_eq!(first.fields, second.fields);
    assert_eq!(first.conveyance, second.conveyance);
    assert_eq!(first.next_node_id, second.next_node_id);
    assert_eq!(first.retained_batches, second.retained_batches);
    assert_eq!(first.conservation_receipts, second.conservation_receipts);
}

#[test]
fn an_override_that_lowers_capacity_below_its_inherited_storage_is_refused() {
    // Precedence is cell, then chart, then default. An override that changes only
    // the capacity still inherits the default initial storage, and the pair has to
    // be consistent — caught here by name rather than as a bare capacity error
    // from whichever cell tripped over it first.
    let mut hydrology = enabled_config();
    let parameters = hydrology
        .bootstrap_parameters
        .as_mut()
        .expect("the fixture is enabled");
    parameters.initial_surface = WaterVolume::new(5_000);
    parameters.chart_overrides.insert(
        chart(),
        HydrologyBootstrapOverride {
            surface_capacity: Some(WaterVolume::new(100)),
            ..HydrologyBootstrapOverride::default()
        },
    );
    assert!(matches!(
        Runtime::new(config_with(hydrology)),
        Err(RuntimeError::HydrologyInitialStorageExceedsCapacity)
    ));
}

#[test]
fn a_cell_override_inherits_from_its_chart_override_before_the_default() {
    // The cell override supplies the capacity and the chart override the storage;
    // resolved together they are consistent, so this configuration is accepted
    // even though neither override is self-sufficient.
    let mut hydrology = enabled_config();
    let parameters = hydrology
        .bootstrap_parameters
        .as_mut()
        .expect("the fixture is enabled");
    parameters.initial_soil = WaterVolume::new(400_000);
    parameters.chart_overrides.insert(
        chart(),
        HydrologyBootstrapOverride {
            initial_soil: Some(WaterVolume::new(90)),
            ..HydrologyBootstrapOverride::default()
        },
    );
    parameters.cell_overrides.insert(
        cell(3),
        HydrologyBootstrapOverride {
            soil_capacity: Some(WaterVolume::new(100)),
            ..HydrologyBootstrapOverride::default()
        },
    );
    Runtime::new(config_with(hydrology)).expect("chart storage inside cell capacity is valid");
}

// ---------------------------------------------------------------------------
// The seventh production bootstrap stage
// ---------------------------------------------------------------------------

#[test]
fn a_disabled_session_keeps_the_legacy_six_stage_plan() {
    // Appending a stage to an enabled session must not change what a disabled one
    // records — that is what keeps every pre-hydrology snapshot comparable (V22).
    let snapshot = Runtime::new(RuntimeConfig::new(31))
        .expect("construction")
        .export_snapshot()
        .expect("export");
    assert_eq!(snapshot.bootstrap.plan.stages.len(), BOOTSTRAP_STAGE_COUNT);
    assert_eq!(snapshot.bootstrap.receipts.len(), BOOTSTRAP_STAGE_COUNT);
}

#[test]
fn an_enabled_session_appends_a_seventh_stage_after_all_six() {
    let snapshot = Runtime::new(config_with(enabled_config()))
        .expect("construction")
        .export_snapshot()
        .expect("export");

    assert_eq!(
        snapshot.bootstrap.plan.stages.len(),
        HYDROLOGY_BOOTSTRAP_STAGE_COUNT
    );
    assert_eq!(
        snapshot.bootstrap.receipts.len(),
        HYDROLOGY_BOOTSTRAP_STAGE_COUNT
    );

    // Every existing stage keeps its ID and its process schema; the appended one
    // is last and depends on the sixth.
    let stages = &snapshot.bootstrap.plan.stages;
    for (index, stage) in stages.iter().enumerate() {
        assert_eq!(stage.stage, HistoricalStageId::new(index as u64 + 1));
    }
    let hydrology = stages.last().expect("the plan is not empty");
    assert_eq!(hydrology.stage, HYDROLOGY_STAGE);
    assert_eq!(hydrology.process, HYDROLOGY_PROCESS_SCHEMA);
    assert_eq!(hydrology.dependencies, vec![THERMAL_RESERVOIR_STAGE]);
}

#[test]
fn the_seventh_stage_commits_one_origin_event_with_seven_aggregate_effects() {
    let runtime = Runtime::new(config_with(enabled_config())).expect("construction");
    let snapshot = runtime.export_snapshot().expect("export");

    // The stage's own effect is exactly one event. Bootstrap initialises up to
    // 131 072 cells and the effect cap is eight, so the payloads are seven
    // aggregates rather than one effect per carrier.
    let origin = snapshot
        .traces
        .events
        .iter()
        .find(|event| event.kind.raw() == HYDROLOGY_BOOTSTRAP_EVENT_KIND)
        .expect("the origin event was committed");
    assert_eq!(origin.effects.len(), 7);

    let mut properties: Vec<u64> = origin
        .effects
        .iter()
        .map(|effect| {
            assert_eq!(
                effect.target().object_kind().raw(),
                HYDROLOGY_BOOTSTRAP_OBJECT_KIND
            );
            assert_eq!(
                effect.target().object_id(),
                0,
                "the seven aggregates share one fixed object"
            );
            assert_ne!(
                effect.before(),
                effect.after(),
                "each aggregate transitions from absent to its canonical digest"
            );
            effect.target().property().raw()
        })
        .collect();
    properties.sort_unstable();
    assert_eq!(
        properties,
        vec![
            HYDROLOGY_BOOTSTRAP_METRICS_PROPERTY,
            HYDROLOGY_BOOTSTRAP_SUBSTRATE_PROPERTY,
            HYDROLOGY_BOOTSTRAP_STORAGE_PROPERTY,
            HYDROLOGY_BOOTSTRAP_EDGES_PROPERTY,
            HYDROLOGY_BOOTSTRAP_RESOLUTION_PROPERTY,
            HYDROLOGY_BOOTSTRAP_FORCING_PROPERTY,
            HYDROLOGY_BOOTSTRAP_BOUNDARIES_PROPERTY,
        ]
        .into_iter()
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>()
    );

    // Its sole cause is the sixth stage's already committed completion.
    assert_eq!(origin.causes.len(), 1);
    let sixth = snapshot
        .bootstrap
        .receipts
        .iter()
        .find(|receipt| receipt.stage == THERMAL_RESERVOIR_STAGE)
        .expect("the sixth stage has a receipt");
    assert_eq!(origin.causes[0], sixth.trace);
}

#[test]
fn every_initialized_carrier_is_anchored_to_the_origin_event() {
    let runtime = Runtime::new(config_with(enabled_config())).expect("construction");
    let snapshot = runtime.export_snapshot().expect("export");
    let origin = snapshot
        .traces
        .events
        .iter()
        .find(|event| event.kind.raw() == HYDROLOGY_BOOTSTRAP_EVENT_KIND)
        .expect("the origin event was committed")
        .trace_id;

    let state = runtime.hydrology_state();
    assert_eq!(state.fields.conservation_last_change(), origin);
    for field in state.fields.fields().values() {
        for cell in field.cells() {
            assert_eq!(cell.surface_last_change(), origin);
            assert_eq!(cell.soil_last_change(), origin);
            assert_eq!(cell.groundwater_last_change(), origin);
            assert_eq!(cell.forcing_last_change(), origin);
        }
    }
    for edge in state.conveyance.edges().values() {
        assert_eq!(edge.last_change(), origin);
    }
    for entry in state.resolution.values() {
        assert_eq!(entry.last_change(), origin);
    }
    for record in &state.forcing {
        assert_eq!(record.origin_trace(), origin);
    }
}

#[test]
fn installed_forcing_records_carry_the_bootstrap_policy_and_start_pending() {
    // Configuration never contains a trace, and letting it choose its own producer
    // policy would let a session declare itself an authorized producer.
    let runtime = Runtime::new(config_with(enabled_config())).expect("construction");
    let state = runtime.hydrology_state();
    assert_eq!(state.forcing.len(), 1);
    let record = &state.forcing[0];
    assert_eq!(
        record.producer_policy_schema(),
        causafera_geography::BOOTSTRAP_HYDROLOGY_FORCING_POLICY_V1
    );
    assert_eq!(
        record.applied_at(),
        None,
        "a canonical record starts pending"
    );
    assert_eq!(record.scheduled_tick(), 3);
    assert_eq!(record.targets().len(), 2);
}

#[test]
fn a_scheduled_record_is_applied_exactly_once_at_its_tick() {
    let mut hydrology = enabled_config();
    hydrology.forcing_schedule[0].scheduled_tick = 2;
    let mut runtime = Runtime::new(config_with(hydrology)).expect("construction");

    runtime.run_ticks(1).expect("tick one must commit");
    assert_eq!(
        runtime.hydrology_state().forcing[0].applied_at(),
        None,
        "a record scheduled for tick two is untouched at tick one"
    );

    runtime.run_ticks(1).expect("tick two must commit");
    assert_eq!(
        runtime.hydrology_state().forcing[0].applied_at(),
        Some(2),
        "the record transitions in the same tick as the water it delivered"
    );
    let precipitated = {
        let state = runtime.hydrology_state();
        let trace = *state.retained_batches.last().expect("a batch was retained");
        state.conservation_receipts[&trace].accepted_precipitation()
    };
    assert!(precipitated > 0, "the record actually delivered water");

    // And it is not applied again, nor does it keep delivering.
    runtime.run_ticks(3).expect("later ticks must commit");
    assert_eq!(runtime.hydrology_state().forcing[0].applied_at(), Some(2));
    let state = runtime.hydrology_state();
    let trace = *state.retained_batches.last().expect("a batch was retained");
    assert_eq!(
        state.conservation_receipts[&trace].accepted_precipitation(),
        0,
        "a spent record delivers nothing on a later tick"
    );
    assert_eq!(
        state.forcing.len(),
        1,
        "the record stays persisted so its origin and allocations remain inspectable"
    );
}

#[test]
fn two_configurations_differing_only_in_hydrology_have_different_bootstrap_plans() {
    // The stage's parameter fingerprint covers the configured numbers, so two
    // worlds initialised differently cannot share a canonical plan.
    let plan_of = |mutate: fn(&mut HydrologyConfig)| {
        let mut hydrology = enabled_config();
        mutate(&mut hydrology);
        Runtime::new(config_with(hydrology))
            .expect("construction")
            .export_snapshot()
            .expect("export")
            .bootstrap
            .plan
            .id
    };
    let baseline = plan_of(|_| {});
    let altered = plan_of(|hydrology| {
        hydrology
            .bootstrap_parameters
            .as_mut()
            .expect("enabled")
            .infiltration_rate_mm_per_second += 1;
    });
    assert_ne!(baseline, altered);
}

#[test]
fn an_engine_resolution_change_reaches_hydrology_and_is_committed_as_an_event() {
    // V32. The engine's own `ResolutionField` is what decides a chunk's detail
    // level; hydrology reads it through the runtime adapter on the next tick,
    // because `Phase::Resolution` runs after Physics has already completed. What
    // this asserts is the whole path: a level that actually moved, an event that
    // records the move, and water that did not change because of it.
    // A closed world with nothing scheduled: the only thing that can change the
    // total is a leak, so the conservation assertion below is exact rather than
    // approximate.
    let mut hydrology = enabled_config();
    hydrology.forcing_schedule.clear();
    let mut runtime = Runtime::new(config_with(hydrology)).expect("construction");
    let initial = runtime.hydrology_state();
    assert!(
        initial.resolution.values().all(|entry| entry.level() == 0),
        "bootstrap commits no detail level nobody decided"
    );
    let initial_water = total_water(&initial);

    runtime.run_ticks(12).expect("twelve ticks must commit");

    let state = runtime.hydrology_state();
    let promoted: Vec<_> = state
        .resolution
        .iter()
        .filter(|(_, entry)| entry.level() > 0)
        .collect();
    assert!(
        !promoted.is_empty(),
        "the engine's resolution field must have promoted at least one chunk, \
         or this test proves nothing"
    );

    // Every promoted chunk's anchor is a committed representation event whose
    // cause is the anchor it replaced.
    let exported = runtime.export_snapshot().expect("export");
    for (chunk, entry) in &promoted {
        let event = exported
            .traces
            .events
            .iter()
            .find(|event| event.trace_id == entry.last_change())
            .unwrap_or_else(|| panic!("chunk {chunk:?} anchors a trace that does not exist"));
        assert_eq!(
            event.kind.raw(),
            causafera_runtime::HYDROLOGY_REPRESENTATION_EVENT_KIND,
            "a promoted chunk is anchored to its representation event"
        );
        assert_eq!(
            event.causes.len(),
            1,
            "the prior anchor is the cause, and it is the only one"
        );
        assert_eq!(
            event.effects.len(),
            1,
            "a level change transitions the level and nothing else"
        );
    }

    // Detail changed; water did not. The initial storage is still all there,
    // because a representation change moves no water and deletes no state.
    assert_eq!(
        total_water(&state),
        initial_water,
        "a level change is not a source or a sink"
    );
    assert_eq!(
        state.fields.fields().len(),
        initial.fields.fields().len(),
        "and it changes no topology"
    );
    assert_eq!(state.conveyance.len(), initial.conveyance.len());
}

#[test]
fn an_engine_level_above_the_policy_refuses_the_tick() {
    // V32's third clause. The engine promotes on its own schedule; a hydrology
    // policy that admits less than the engine produces refuses the tick rather
    // than quietly evaluating at a level nobody chose.
    let mut hydrology = enabled_config();
    hydrology.resolution_policy =
        HydrologyResolutionPolicy::enabled(0).expect("level zero is a valid ceiling");
    let mut runtime = Runtime::new(config_with(hydrology)).expect("construction");

    let mut refusal = None;
    for _ in 0..12 {
        if let Err(error) = runtime.tick() {
            refusal = Some(error);
            break;
        }
    }
    let error = refusal.expect("the engine must eventually promote past level zero");
    assert!(
        matches!(
            error,
            causafera_runtime::RuntimeError::HydrologyResolutionLevelUnsupported { .. }
        ),
        "unexpected refusal: {error}"
    );
    // And the refusal is a refusal, not a poisoning: the tick rolled back whole.
    assert!(
        runtime.snapshot().is_ok(),
        "the session stays readable at the tick it completed"
    );
}

#[test]
fn observer_queries_do_not_change_what_the_engine_computes() {
    // V32's last clause and V27's. Reading is not participating: a run watched
    // every tick reaches the same digests as a run nobody looked at.
    let watched = {
        let mut runtime = Runtime::new(config_with(enabled_config())).expect("construction");
        for _ in 0..6 {
            runtime.tick().expect("tick");
            runtime.observer_world_snapshot().expect("observer read");
            runtime.hydrology_state();
        }
        runtime.snapshot().expect("digest")
    };
    let unwatched = {
        let mut runtime = Runtime::new(config_with(enabled_config())).expect("construction");
        runtime.run_ticks(6).expect("six ticks");
        runtime.snapshot().expect("digest")
    };
    assert_eq!(
        watched.physical_state_digest.bytes(),
        unwatched.physical_state_digest.bytes()
    );
    assert_eq!(
        watched.history_digest.bytes(),
        unwatched.history_digest.bytes()
    );
}

// ---------------------------------------------------------------------------
// Metric and parameter counterfactuals (V31)
// ---------------------------------------------------------------------------

/// The substrate one configuration produces for the cell every case reads.
///
/// Engine-produced, not hand-built: these are assertions about what causal
/// initialization derives from the configured numbers, so deriving them a second
/// way in the test would assert nothing about production.
fn derived(
    mutate: impl FnOnce(&mut HydrologyConfig),
) -> causafera_geography::HydraulicSubstrateCell {
    let mut hydrology = conductive_config();
    mutate(&mut hydrology);
    let runtime = Runtime::new(config_with(hydrology)).expect("construction");
    let state = runtime.hydrology_state();
    *state
        .fields
        .ground(cell(0))
        .expect("the fixture chunk is resident")
}

/// The fixture's transmissivities are small enough that a one-second tick over a
/// one-metre face floors every conductance to zero. These cases are about the
/// derivation, so they start from numbers the derivation can show.
fn conductive_config() -> HydrologyConfig {
    let mut hydrology = enabled_config();
    hydrology.forcing_schedule.clear();
    let parameters = hydrology.bootstrap_parameters.as_mut().expect("enabled");
    parameters.base_surface_transmissivity_mm3_per_second = 5_000_000;
    parameters.base_groundwater_transmissivity_mm3_per_second = 1_000_000;
    parameters.roughness_reference_mm = nz64(1_000_000);
    hydrology
}

/// Every cell of the fixture chunk, so a case about terrain can read terrain that
/// actually varies instead of whichever cell ordinal zero happens to be.
fn derived_chunk(
    mutate: impl FnOnce(&mut HydrologyConfig),
) -> Vec<causafera_geography::HydraulicSubstrateCell> {
    let mut hydrology = conductive_config();
    mutate(&mut hydrology);
    let runtime = Runtime::new(config_with(hydrology)).expect("construction");
    let state = runtime.hydrology_state();
    state
        .fields
        .fields()
        .values()
        .next()
        .expect("the fixture holds one chunk")
        .substrate()
        .to_vec()
}

fn with_metric_chunk(
    cell_area: u64,
    edge_length: u64,
    timestep: u64,
) -> Vec<causafera_geography::HydraulicSubstrateCell> {
    derived_chunk(|hydrology| {
        hydrology.grid_metrics = [(
            chart(),
            HydrologyGridMetric::new(nz64(cell_area), nz64(edge_length), nz64(timestep)),
        )]
        .into_iter()
        .collect();
    })
}

fn with_metric(
    cell_area: u64,
    edge_length: u64,
    timestep: u64,
) -> causafera_geography::HydraulicSubstrateCell {
    derived(|hydrology| {
        hydrology.grid_metrics = [(
            chart(),
            HydrologyGridMetric::new(nz64(cell_area), nz64(edge_length), nz64(timestep)),
        )]
        .into_iter()
        .collect();
    })
}

#[test]
fn doubling_the_timestep_doubles_the_derived_per_tick_coefficients() {
    // `infiltration_limit_per_tick = floor(rate * area * millis / 1000)` and both
    // conductances carry `millis` in the numerator, so twice the tick is twice the
    // budget — exactly where the division is exact, and within the one unit the
    // floor can withhold where it is not. This is what makes the timestep a causal
    // input rather than metadata.
    let base = with_metric_chunk(1_000_000, 1_000, 1_000);
    let doubled = with_metric_chunk(1_000_000, 1_000, 2_000);

    let mut exact = 0;
    for (base, doubled) in base.iter().zip(doubled.iter()) {
        assert_eq!(
            doubled.infiltration_limit_per_tick().get(),
            base.infiltration_limit_per_tick().get() * 2,
            "the infiltration equation divides by a constant, so it doubles exactly"
        );
        let expected = base.surface_conductance_mm2_per_tick() * 2;
        let actual = doubled.surface_conductance_mm2_per_tick();
        assert!(
            actual == expected || actual == expected + 1,
            "twice the tick is twice the conductance, up to the floored remainder: \
             {actual} against {expected}"
        );
        exact += u32::from(actual == expected);
        assert_eq!(
            doubled.groundwater_conductance_mm2_per_tick(),
            base.groundwater_conductance_mm2_per_tick() * 2,
            "groundwater conductance is not roughness-adjusted, so it has no remainder to carry"
        );
    }
    assert!(
        base[0].infiltration_limit_per_tick().get() > 0,
        "non-vacuous"
    );
    assert!(
        base[0].surface_conductance_mm2_per_tick() > 0,
        "non-vacuous"
    );
    assert!(
        exact > 0,
        "and the exact case is reached, not merely bounded"
    );
}

#[test]
fn doubling_the_edge_length_halves_the_derived_conductance() {
    // Conductance divides by `orthogonal_edge_length_mm`, and the division floors,
    // so the assertion is exact for a base value that halves exactly and bounded
    // by one unit otherwise. Both conductances share the divisor.
    let base = with_metric(1_000_000, 1_000, 1_000_000);
    let longer = with_metric(1_000_000, 2_000, 1_000_000);

    assert!(base.surface_conductance_mm2_per_tick() > 1, "non-vacuous");
    assert_eq!(
        longer.surface_conductance_mm2_per_tick(),
        base.surface_conductance_mm2_per_tick() / 2
    );
    assert_eq!(
        longer.groundwater_conductance_mm2_per_tick(),
        base.groundwater_conductance_mm2_per_tick() / 2
    );
    // Edge length is not part of the infiltration equation.
    assert_eq!(
        longer.infiltration_limit_per_tick(),
        base.infiltration_limit_per_tick()
    );
}

#[test]
fn cell_area_changes_the_infiltration_volume_and_nothing_else_derived() {
    // `infiltration_limit_per_tick` is a volume, so it scales with the area the
    // rate is applied over. Conductance is an area-per-tick coefficient of the
    // face, not of the cell, and does not.
    let base = with_metric(1_000_000, 1_000, 1_000);
    let wider = with_metric(2_000_000, 1_000, 1_000);

    assert!(base.infiltration_limit_per_tick().get() > 0, "non-vacuous");
    assert_eq!(
        wider.infiltration_limit_per_tick().get(),
        base.infiltration_limit_per_tick().get() * 2
    );
    assert_eq!(
        wider.surface_conductance_mm2_per_tick(),
        base.surface_conductance_mm2_per_tick()
    );
    assert_eq!(
        wider.groundwater_conductance_mm2_per_tick(),
        base.groundwater_conductance_mm2_per_tick()
    );
}

#[test]
fn rougher_ground_conducts_less_and_leaves_groundwater_alone() {
    // `adjusted = floor(base * reference / (reference + cell_roughness))`. The
    // reference is the roughness at which half the base transmissivity survives,
    // so lowering it makes the same terrain weigh far more heavily — every cell
    // conducts at most what it did before, and the rough ones conduct much less.
    // Read across the whole chunk, because terrain roughness varies across it and
    // one ordinal is not evidence about the equation.
    let slick = derived_chunk(|hydrology| {
        hydrology
            .bootstrap_parameters
            .as_mut()
            .expect("enabled")
            .roughness_reference_mm = nz64(1_000_000);
    });
    let rough = derived_chunk(|hydrology| {
        hydrology
            .bootstrap_parameters
            .as_mut()
            .expect("enabled")
            .roughness_reference_mm = nz64(1);
    });

    let total = |cells: &[causafera_geography::HydraulicSubstrateCell]| -> u128 {
        cells
            .iter()
            .map(|cell| u128::from(cell.surface_conductance_mm2_per_tick()))
            .sum()
    };
    assert!(total(&slick) > 0, "non-vacuous");
    assert!(
        total(&rough) < total(&slick),
        "terrain roughness weighs against surface conductance"
    );
    for (slick, rough) in slick.iter().zip(rough.iter()) {
        assert!(
            rough.surface_conductance_mm2_per_tick() <= slick.surface_conductance_mm2_per_tick(),
            "and it does so cell by cell, never the other way"
        );
        assert_eq!(
            rough.groundwater_conductance_mm2_per_tick(),
            slick.groundwater_conductance_mm2_per_tick(),
            "groundwater transmissivity is not roughness-adjusted"
        );
        assert_eq!(
            rough.infiltration_limit_per_tick(),
            slick.infiltration_limit_per_tick(),
            "and roughness is not part of the infiltration equation"
        );
    }
    assert!(
        rough
            .iter()
            .zip(slick.iter())
            .any(|(rough, slick)| rough.surface_conductance_mm2_per_tick()
                < slick.surface_conductance_mm2_per_tick()),
        "the fixture terrain must actually vary in roughness"
    );
}

#[test]
fn doubling_groundwater_transmissivity_doubles_only_its_own_conductance() {
    let base = derived(|_| {});
    let doubled = derived(|hydrology| {
        let parameters = hydrology.bootstrap_parameters.as_mut().expect("enabled");
        parameters.base_groundwater_transmissivity_mm3_per_second *= 2;
    });

    assert!(
        base.groundwater_conductance_mm2_per_tick() > 0,
        "non-vacuous"
    );
    assert_eq!(
        doubled.groundwater_conductance_mm2_per_tick(),
        base.groundwater_conductance_mm2_per_tick() * 2
    );
    assert_eq!(
        doubled.surface_conductance_mm2_per_tick(),
        base.surface_conductance_mm2_per_tick(),
        "the two transmissivities are independent inputs"
    );
}

#[test]
fn an_explicit_bound_engages_before_the_derived_coefficient_passes_it() {
    // "until an explicit bound engages": infiltration is bounded by the receiving
    // soil's remaining room, so a per-tick limit far above that room moves the
    // room and not the limit. The limit itself keeps scaling with the timestep —
    // the bound is a solver constraint, not a silent clamp on the coefficient.
    let generous = with_metric(1_000_000, 1_000, 1_000_000);
    assert!(
        generous.infiltration_limit_per_tick().get() > generous.soil_capacity().get(),
        "the derived limit exceeds the room, or the bound would not be the binding one"
    );

    let mut hydrology = enabled_config();
    hydrology.forcing_schedule.clear();
    hydrology.grid_metrics = [(
        chart(),
        HydrologyGridMetric::new(nz64(1_000_000), nz64(1_000), nz64(1_000_000)),
    )]
    .into_iter()
    .collect();
    let mut runtime = Runtime::new(config_with(hydrology)).expect("construction");
    let before = runtime.hydrology_state();
    let soil_before = before
        .fields
        .cell(cell(0))
        .expect("resident")
        .storage()
        .soil
        .get();
    runtime.run_ticks(1).expect("one tick must commit");
    let after = runtime.hydrology_state();
    let soil_after = after
        .fields
        .cell(cell(0))
        .expect("resident")
        .storage()
        .soil
        .get();
    assert!(
        soil_after
            <= after
                .fields
                .ground(cell(0))
                .expect("resident")
                .soil_capacity()
                .get(),
        "the soil never passes its capacity however large the per-tick limit is"
    );
    assert!(
        soil_after >= soil_before,
        "and the tick moved water into it rather than out"
    );
}
