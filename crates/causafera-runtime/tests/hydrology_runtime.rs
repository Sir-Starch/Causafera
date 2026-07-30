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
    HYDROLOGY_BOOTSTRAP_PARAMETERS_SCHEMA_V1, HYDROLOGY_LIMITS_SCHEMA_V1,
    HydrologyBootstrapOverride, HydrologyBootstrapParameters, HydrologyConfig,
    HydrologyForcingSpec, Runtime, RuntimeConfig, RuntimeError,
};
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
        resolution_policy: HydrologyResolutionPolicy::enabled(2).expect("level two is valid"),
        limits_schema: HYDROLOGY_LIMITS_SCHEMA_V1,
    }
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
