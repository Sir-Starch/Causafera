//! Shared hydrology configurations for the runtime integration tests.
//!
//! Each test binary compiles this separately and uses a different part, so an
//! unused builder here is a fact about one binary rather than about the fixture.
#![allow(dead_code)]

use std::collections::BTreeMap;
use std::num::{NonZeroU32, NonZeroU64};

use causafera_domains::HydrologyResolutionPolicy;
use causafera_geography::{HydrologyBoundaryCondition, HydrologyCellKey, HydrologyGridMetric};
use causafera_runtime::{
    HYDROLOGY_BOOTSTRAP_PARAMETERS_SCHEMA_V1, HYDROLOGY_LIMITS_SCHEMA_V1,
    HydrologyBootstrapParameters, HydrologyConfig, HydrologyForcingSpec, RuntimeConfig,
};
use causafera_types::{ChartChunkCoord, ChunkCoord, SpatialChartId, WaterVolume};

pub const HYDROLOGY_WORLD_SEED: u64 = 20_260_731;

pub fn chart() -> SpatialChartId {
    SpatialChartId::new(1)
}

pub fn cell(ordinal: u16) -> HydrologyCellKey {
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

pub fn parameters() -> HydrologyBootstrapParameters {
    HydrologyBootstrapParameters {
        schema_version: HYDROLOGY_BOOTSTRAP_PARAMETERS_SCHEMA_V1,
        default_surface_capacity: WaterVolume::new(1_000_000_000),
        default_soil_capacity: WaterVolume::new(1_000_000_000),
        default_groundwater_capacity: WaterVolume::new(1_000_000_000),
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

pub fn enabled_hydrology() -> HydrologyConfig {
    HydrologyConfig {
        enabled: true,
        grid_metrics: [(
            chart(),
            HydrologyGridMetric::new(nz64(1_000_000), nz64(1_000), nz64(1_000)),
        )]
        .into_iter()
        .collect(),
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

pub fn enabled_runtime_config() -> RuntimeConfig {
    let mut config = RuntimeConfig::new(HYDROLOGY_WORLD_SEED);
    config.hydrology = enabled_hydrology();
    config
}

/// A world with enough water and conductance that every tick moves some.
pub fn wet_runtime_config() -> RuntimeConfig {
    let mut config = enabled_runtime_config();
    config.hydrology.forcing_schedule.clear();
    let parameters = config
        .hydrology
        .bootstrap_parameters
        .as_mut()
        .expect("the fixture is enabled");
    parameters.initial_surface = WaterVolume::new(20_000_000);
    parameters.initial_soil = WaterVolume::new(100_000);
    parameters.initial_groundwater = WaterVolume::new(50_000);
    parameters.infiltration_rate_mm_per_second = 200;
    parameters.base_surface_transmissivity_mm3_per_second = 5_000_000;
    parameters.base_groundwater_transmissivity_mm3_per_second = 1_000_000;
    config
}
