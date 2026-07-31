//! The runtime's hydrology configuration: disabled by default, and bounded and
//! explicit when enabled.
//!
//! Nothing is defaulted into an existing session. A world that did not ask for
//! water gets no field, no edge, no boundary, and no rainfall — and its canonical
//! recipe encoding says so explicitly rather than by omission, because "the
//! section was absent" and "the section recorded a disabled domain" are different
//! claims about what a snapshot contains.
//!
//! See `plans/hydrology.md` §4.

use std::collections::BTreeMap;
use std::num::{NonZeroU32, NonZeroU64};

use causafera_domains::HydrologyResolutionPolicy;
use causafera_geography::{
    FaceDirection, HydrologyBoundaryCondition, HydrologyCellKey, HydrologyGridMetric,
    MAX_HYDROLOGY_CELL_OVERRIDES, MAX_HYDROLOGY_CHART_OVERRIDES, MAX_HYDROLOGY_CHARTS,
    MAX_HYDROLOGY_FORCING_HORIZON_TICKS, MAX_HYDROLOGY_FORCING_RECORDS,
    MAX_HYDROLOGY_TARGETS_PER_FORCING, MAX_HYDROLOGY_TOTAL_FORCING_MEMBERS,
};
use causafera_types::{SpatialChartId, WaterVolume};

use crate::RuntimeError;

/// The schema version of the bounds this configuration validates against.
pub const HYDROLOGY_LIMITS_SCHEMA_V1: u16 = 1;

/// The schema version of the bootstrap parameter block.
pub const HYDROLOGY_BOOTSTRAP_PARAMETERS_SCHEMA_V1: u16 = 1;

/// Everything a session configures about hydrology.
///
/// `enabled` is not a hint. A disabled configuration must carry no bootstrap
/// parameters and no collections at all; anything else is noncanonical and
/// rejects, so a half-configured domain cannot reach a snapshot and claim to be
/// off.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HydrologyConfig {
    pub enabled: bool,
    /// Explicit per-chart grid metrics. Never inferred from `chunk_extent`,
    /// containment, observer zoom, or UI scale.
    pub grid_metrics: BTreeMap<SpatialChartId, HydrologyGridMetric>,
    pub bootstrap_parameters: Option<HydrologyBootstrapParameters>,
    /// Ordered and unique by `(scheduled_tick, forcing_id)`.
    pub forcing_schedule: Vec<HydrologyForcingSpec>,
    pub resolution_policy: HydrologyResolutionPolicy,
    pub limits_schema: u16,
}

impl HydrologyConfig {
    /// The only configuration an existing session gets by default.
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            grid_metrics: BTreeMap::new(),
            bootstrap_parameters: None,
            forcing_schedule: Vec::new(),
            resolution_policy: HydrologyResolutionPolicy::DISABLED,
            limits_schema: HYDROLOGY_LIMITS_SCHEMA_V1,
        }
    }

    /// Validate the configuration against the plan's bounds.
    ///
    /// `bootstrap_tick` is the tick production bootstrap completes on; every
    /// scheduled forcing record must land strictly after it and no further out
    /// than the horizon, with checked subtraction so a wraparound cannot present
    /// itself as a near-term record.
    pub(crate) fn validate(&self, bootstrap_tick: u64) -> Result<(), RuntimeError> {
        if self.limits_schema != HYDROLOGY_LIMITS_SCHEMA_V1 {
            return Err(RuntimeError::HydrologyLimitsSchemaUnknown {
                schema: self.limits_schema,
            });
        }
        if !self.enabled {
            // "Disabled" has one canonical shape. A configuration carrying
            // parameters it will never use is a configuration whose author
            // believed something the runtime does not.
            if self.bootstrap_parameters.is_some()
                || !self.grid_metrics.is_empty()
                || !self.forcing_schedule.is_empty()
                || self.resolution_policy != HydrologyResolutionPolicy::DISABLED
            {
                return Err(RuntimeError::HydrologyDisabledConfigNotCanonical);
            }
            return Ok(());
        }

        let parameters = self
            .bootstrap_parameters
            .as_ref()
            .ok_or(RuntimeError::HydrologyEnabledWithoutParameters)?;
        parameters.validate()?;

        if self.grid_metrics.is_empty() {
            return Err(RuntimeError::HydrologyMetricMissing);
        }
        if self.grid_metrics.len() > MAX_HYDROLOGY_CHARTS {
            return Err(RuntimeError::HydrologyBoundExceeded {
                what: "grid metrics",
                count: self.grid_metrics.len(),
                max: MAX_HYDROLOGY_CHARTS,
            });
        }
        if self.resolution_policy.schema_version != HydrologyResolutionPolicy::SCHEMA_VERSION {
            return Err(RuntimeError::HydrologyLimitsSchemaUnknown {
                schema: self.resolution_policy.schema_version,
            });
        }
        if self.resolution_policy.max_level > causafera_geography::MAX_HYDROLOGY_RESOLUTION_LEVEL {
            return Err(RuntimeError::HydrologyResolutionLevelUnsupported {
                level: self.resolution_policy.max_level,
            });
        }

        self.validate_forcing(bootstrap_tick)
    }

    fn validate_forcing(&self, bootstrap_tick: u64) -> Result<(), RuntimeError> {
        if self.forcing_schedule.len() > MAX_HYDROLOGY_FORCING_RECORDS {
            return Err(RuntimeError::HydrologyBoundExceeded {
                what: "forcing records",
                count: self.forcing_schedule.len(),
                max: MAX_HYDROLOGY_FORCING_RECORDS,
            });
        }
        let mut members_total = 0_usize;
        let mut previous: Option<(u64, u64)> = None;
        for spec in &self.forcing_schedule {
            let key = (spec.scheduled_tick, spec.forcing_id);
            if let Some(last) = previous
                && key <= last
            {
                return Err(RuntimeError::HydrologyForcingScheduleNotCanonical);
            }
            previous = Some(key);

            // Strictly after bootstrap and no further out than the horizon.
            // Checked subtraction, so a record scheduled before bootstrap cannot
            // wrap around and present itself as a near-term one.
            let lead = spec
                .scheduled_tick
                .checked_sub(bootstrap_tick)
                .filter(|lead| *lead > 0)
                .ok_or(RuntimeError::HydrologyForcingScheduledTooEarly {
                    scheduled_tick: spec.scheduled_tick,
                    bootstrap_tick,
                })?;
            if lead > MAX_HYDROLOGY_FORCING_HORIZON_TICKS {
                return Err(RuntimeError::HydrologyForcingBeyondHorizon {
                    scheduled_tick: spec.scheduled_tick,
                    horizon: MAX_HYDROLOGY_FORCING_HORIZON_TICKS,
                });
            }

            if spec.targets.is_empty() {
                return Err(RuntimeError::HydrologyForcingScheduleNotCanonical);
            }
            if spec.targets.len() > MAX_HYDROLOGY_TARGETS_PER_FORCING {
                return Err(RuntimeError::HydrologyBoundExceeded {
                    what: "forcing targets",
                    count: spec.targets.len(),
                    max: MAX_HYDROLOGY_TARGETS_PER_FORCING,
                });
            }
            let mut previous_cell: Option<HydrologyCellKey> = None;
            for (cell, _) in &spec.targets {
                if let Some(last) = previous_cell
                    && *cell <= last
                {
                    return Err(RuntimeError::HydrologyForcingScheduleNotCanonical);
                }
                previous_cell = Some(*cell);
            }
            members_total = members_total.checked_add(spec.targets.len()).ok_or(
                RuntimeError::HydrologyBoundExceeded {
                    what: "forcing members",
                    count: usize::MAX,
                    max: MAX_HYDROLOGY_TOTAL_FORCING_MEMBERS,
                },
            )?;
        }
        if members_total > MAX_HYDROLOGY_TOTAL_FORCING_MEMBERS {
            return Err(RuntimeError::HydrologyBoundExceeded {
                what: "forcing members",
                count: members_total,
                max: MAX_HYDROLOGY_TOTAL_FORCING_MEMBERS,
            });
        }
        Ok(())
    }
}

impl Default for HydrologyConfig {
    fn default() -> Self {
        Self::disabled()
    }
}

/// A scheduled forcing record before bootstrap has an origin to attribute it to.
///
/// Deliberately without `origin_trace` and without a producer policy:
/// configuration cannot name a trace that does not exist yet, and letting it
/// choose its own policy would let a session declare itself an authorized
/// producer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HydrologyForcingSpec {
    pub forcing_id: u64,
    pub scheduled_tick: u64,
    /// `(cell, weight)` pairs, ascending and unique by cell.
    pub targets: Vec<(HydrologyCellKey, NonZeroU64)>,
    pub precipitation_volume: WaterVolume,
    pub potential_et_volume: WaterVolume,
    pub external_inflow_volume: WaterVolume,
}

/// The purely numeric block production bootstrap builds hydrology from.
///
/// Every field is a measured quantity or a ratio. There is no soil class, no
/// biome, no named water body, and no language string, because a production
/// constructor that could name a river would be authoring content rather than
/// initialising physics.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HydrologyBootstrapParameters {
    pub schema_version: u16,
    pub default_surface_capacity: WaterVolume,
    pub default_soil_capacity: WaterVolume,
    pub default_groundwater_capacity: WaterVolume,
    pub initial_surface: WaterVolume,
    pub initial_soil: WaterVolume,
    pub initial_groundwater: WaterVolume,
    pub infiltration_rate_mm_per_second: u64,
    pub percolation_fraction_num: u32,
    pub percolation_fraction_den: NonZeroU32,
    pub specific_yield_num: u32,
    pub specific_yield_den: NonZeroU32,
    pub aquifer_base_offset_mm: i64,
    pub baseflow_threshold: WaterVolume,
    pub baseflow_fraction_num: u32,
    pub baseflow_fraction_den: NonZeroU32,
    pub base_surface_transmissivity_mm3_per_second: u64,
    pub base_groundwater_transmissivity_mm3_per_second: u64,
    pub roughness_reference_mm: NonZeroU64,
    pub conveyance_capacity: WaterVolume,
    pub conveyance_initial_storage: WaterVolume,
    pub conveyance_inlet_capacity_per_tick: WaterVolume,
    pub conveyance_release_fraction_num: u32,
    pub conveyance_release_fraction_den: NonZeroU32,
    pub default_boundary: HydrologyBoundaryCondition,
    pub chart_overrides: BTreeMap<SpatialChartId, HydrologyBootstrapOverride>,
    pub cell_overrides: BTreeMap<HydrologyCellKey, HydrologyBootstrapOverride>,
}

impl HydrologyBootstrapParameters {
    fn validate(&self) -> Result<(), RuntimeError> {
        if self.schema_version != HYDROLOGY_BOOTSTRAP_PARAMETERS_SCHEMA_V1 {
            return Err(RuntimeError::HydrologyBootstrapParametersSchemaUnknown {
                schema: self.schema_version,
            });
        }
        if self.chart_overrides.len() > MAX_HYDROLOGY_CHART_OVERRIDES {
            return Err(RuntimeError::HydrologyBoundExceeded {
                what: "chart overrides",
                count: self.chart_overrides.len(),
                max: MAX_HYDROLOGY_CHART_OVERRIDES,
            });
        }
        if self.cell_overrides.len() > MAX_HYDROLOGY_CELL_OVERRIDES {
            return Err(RuntimeError::HydrologyBoundExceeded {
                what: "cell overrides",
                count: self.cell_overrides.len(),
                max: MAX_HYDROLOGY_CELL_OVERRIDES,
            });
        }
        check_fraction(self.percolation_fraction_num, self.percolation_fraction_den)?;
        check_fraction(self.specific_yield_num, self.specific_yield_den)?;
        check_fraction(self.baseflow_fraction_num, self.baseflow_fraction_den)?;
        check_fraction(
            self.conveyance_release_fraction_num,
            self.conveyance_release_fraction_den,
        )?;
        // Initial storage above its own capacity is a state no process could
        // produce, so it can only arrive by configuration — and it is rejected
        // here rather than clamped, which would silently discard the difference.
        if self.initial_surface > self.default_surface_capacity
            || self.initial_soil > self.default_soil_capacity
            || self.initial_groundwater > self.default_groundwater_capacity
            || self.conveyance_initial_storage > self.conveyance_capacity
        {
            return Err(RuntimeError::HydrologyInitialStorageExceedsCapacity);
        }
        if !self.default_groundwater_capacity.is_zero() && self.specific_yield_num == 0 {
            return Err(RuntimeError::HydrologyZeroSpecificYield);
        }
        // Overrides are validated against what they *resolve* to, not only against
        // themselves: an override that lowers a capacity below the initial storage
        // it inherits is a configuration no field can be built from, and catching
        // it here names the problem instead of surfacing a bare capacity error
        // from whichever cell tripped over it first.
        for override_record in self.chart_overrides.values() {
            override_record.validate()?;
            self.check_resolved(override_record, None)?;
        }
        for (cell, override_record) in &self.cell_overrides {
            override_record.validate()?;
            self.check_resolved(override_record, self.chart_overrides.get(&cell.chart()))?;
        }
        Ok(())
    }

    /// Check one override's resolved storage against its resolved capacity.
    ///
    /// Precedence is cell, then chart, then default — the same order bootstrap
    /// applies, so what is checked here is what bootstrap will build.
    fn check_resolved(
        &self,
        override_record: &HydrologyBootstrapOverride,
        chart: Option<&HydrologyBootstrapOverride>,
    ) -> Result<(), RuntimeError> {
        let resolve =
            |cell: Option<WaterVolume>, chart_value: Option<WaterVolume>, default: WaterVolume| {
                cell.or(chart_value).unwrap_or(default)
            };
        let groundwater_capacity = resolve(
            override_record.groundwater_capacity,
            chart.and_then(|o| o.groundwater_capacity),
            self.default_groundwater_capacity,
        );
        let pairs = [
            (
                resolve(
                    override_record.initial_surface,
                    chart.and_then(|o| o.initial_surface),
                    self.initial_surface,
                ),
                resolve(
                    override_record.surface_capacity,
                    chart.and_then(|o| o.surface_capacity),
                    self.default_surface_capacity,
                ),
            ),
            (
                resolve(
                    override_record.initial_soil,
                    chart.and_then(|o| o.initial_soil),
                    self.initial_soil,
                ),
                resolve(
                    override_record.soil_capacity,
                    chart.and_then(|o| o.soil_capacity),
                    self.default_soil_capacity,
                ),
            ),
            (
                resolve(
                    override_record.initial_groundwater,
                    chart.and_then(|o| o.initial_groundwater),
                    self.initial_groundwater,
                ),
                groundwater_capacity,
            ),
            (
                resolve(
                    override_record.conveyance_initial_storage,
                    chart.and_then(|o| o.conveyance_initial_storage),
                    self.conveyance_initial_storage,
                ),
                resolve(
                    override_record.conveyance_capacity,
                    chart.and_then(|o| o.conveyance_capacity),
                    self.conveyance_capacity,
                ),
            ),
        ];
        for (initial, capacity) in pairs {
            if initial > capacity {
                return Err(RuntimeError::HydrologyInitialStorageExceedsCapacity);
            }
        }
        // A resolved groundwater capacity still needs a resolved specific yield,
        // for the same reason the defaults do: saturated depth divides by it.
        let yield_numerator = override_record
            .specific_yield_num
            .or(chart.and_then(|o| o.specific_yield_num))
            .unwrap_or(self.specific_yield_num);
        if !groundwater_capacity.is_zero() && yield_numerator == 0 {
            return Err(RuntimeError::HydrologyZeroSpecificYield);
        }
        Ok(())
    }
}

fn check_fraction(numerator: u32, denominator: NonZeroU32) -> Result<(), RuntimeError> {
    if numerator > denominator.get() {
        return Err(RuntimeError::HydrologyFractionOutOfRange {
            numerator,
            denominator: denominator.get(),
        });
    }
    Ok(())
}

/// A per-chart or per-cell override of the numeric defaults.
///
/// Optional fields only, and none of them can override a schema version, either
/// override map, or a material identity — an override is a different number for
/// the same physics, never a different kind of thing.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HydrologyBootstrapOverride {
    pub surface_capacity: Option<WaterVolume>,
    pub soil_capacity: Option<WaterVolume>,
    pub groundwater_capacity: Option<WaterVolume>,
    pub initial_surface: Option<WaterVolume>,
    pub initial_soil: Option<WaterVolume>,
    pub initial_groundwater: Option<WaterVolume>,
    pub infiltration_rate_mm_per_second: Option<u64>,
    pub percolation_fraction_num: Option<u32>,
    pub percolation_fraction_den: Option<NonZeroU32>,
    pub specific_yield_num: Option<u32>,
    pub specific_yield_den: Option<NonZeroU32>,
    pub aquifer_base_offset_mm: Option<i64>,
    pub baseflow_threshold: Option<WaterVolume>,
    pub baseflow_fraction_num: Option<u32>,
    pub baseflow_fraction_den: Option<NonZeroU32>,
    pub base_surface_transmissivity_mm3_per_second: Option<u64>,
    pub base_groundwater_transmissivity_mm3_per_second: Option<u64>,
    pub roughness_reference_mm: Option<NonZeroU64>,
    pub conveyance_capacity: Option<WaterVolume>,
    pub conveyance_initial_storage: Option<WaterVolume>,
    pub conveyance_inlet_capacity_per_tick: Option<WaterVolume>,
    pub conveyance_release_fraction_num: Option<u32>,
    pub conveyance_release_fraction_den: Option<NonZeroU32>,
    /// Conditions for specific exterior faces, overriding the resolved default
    /// for those faces only.
    pub face_boundaries: BTreeMap<FaceDirection, HydrologyBoundaryCondition>,
}

impl HydrologyBootstrapOverride {
    fn validate(&self) -> Result<(), RuntimeError> {
        if self.face_boundaries.len() > FaceDirection::ALL.len() {
            return Err(RuntimeError::HydrologyBoundExceeded {
                what: "override face boundaries",
                count: self.face_boundaries.len(),
                max: FaceDirection::ALL.len(),
            });
        }
        for (numerator, denominator) in [
            (self.percolation_fraction_num, self.percolation_fraction_den),
            (self.specific_yield_num, self.specific_yield_den),
            (self.baseflow_fraction_num, self.baseflow_fraction_den),
            (
                self.conveyance_release_fraction_num,
                self.conveyance_release_fraction_den,
            ),
        ] {
            if let (Some(numerator), Some(denominator)) = (numerator, denominator) {
                check_fraction(numerator, denominator)?;
            }
        }
        Ok(())
    }
}
