use std::collections::BTreeMap;
use std::num::NonZeroU64;

use causafera_types::{
    SpatialChartId, WaterDepthMm, WaterVolume, checked_water_div_floor, checked_water_mul,
};

use super::{HydrologyStateError, MAX_HYDROLOGY_CHARTS};

/// The only grid-metric schema this build accepts.
const HYDROLOGY_GRID_METRIC_SCHEMA_V1: u16 = 1;

/// The registered physical scale of one chart's hydrology lattice.
///
/// This exists because nothing else in the engine carries one. A chunk is an
/// addressing and computation unit; `chunk_extent` sizes the mana volume;
/// containment defines neither adjacency nor distance. Deriving a cell area
/// from any of those would make chunk geometry physical, which INV-036,
/// INV-037, and INV-043 forbid — so the metric is registered explicitly per
/// chart, persisted, and traced from bootstrap.
///
/// It is also what makes cell area, edge length, and timestep *causal* inputs
/// rather than metadata: every per-tick solver coefficient is derived from
/// them, so changing one changes the trajectory (verification gate V31).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HydrologyGridMetric {
    schema_version: u16,
    cell_area_mm2: NonZeroU64,
    orthogonal_edge_length_mm: NonZeroU64,
    timestep_millis: NonZeroU64,
}

impl HydrologyGridMetric {
    pub const SCHEMA_V1: u16 = HYDROLOGY_GRID_METRIC_SCHEMA_V1;

    pub const fn new(
        cell_area_mm2: NonZeroU64,
        orthogonal_edge_length_mm: NonZeroU64,
        timestep_millis: NonZeroU64,
    ) -> Self {
        Self {
            schema_version: HYDROLOGY_GRID_METRIC_SCHEMA_V1,
            cell_area_mm2,
            orthogonal_edge_length_mm,
            timestep_millis,
        }
    }

    /// Rebuild from persisted parts, rejecting an unsupported schema.
    ///
    /// A zero area, edge, or timestep cannot reach this type at all —
    /// `NonZeroU64` refuses it at the boundary rather than leaving a division
    /// to discover it — so decoding is where the raw values are checked.
    pub fn from_parts(
        schema_version: u16,
        cell_area_mm2: u64,
        orthogonal_edge_length_mm: u64,
        timestep_millis: u64,
    ) -> Result<Self, HydrologyStateError> {
        if schema_version != HYDROLOGY_GRID_METRIC_SCHEMA_V1 {
            return Err(HydrologyStateError::UnsupportedMetricSchema(schema_version));
        }
        let cell_area_mm2 = NonZeroU64::new(cell_area_mm2)
            .ok_or(HydrologyStateError::UnsupportedMetricSchema(schema_version))?;
        let orthogonal_edge_length_mm = NonZeroU64::new(orthogonal_edge_length_mm)
            .ok_or(HydrologyStateError::UnsupportedMetricSchema(schema_version))?;
        let timestep_millis = NonZeroU64::new(timestep_millis)
            .ok_or(HydrologyStateError::UnsupportedMetricSchema(schema_version))?;
        Ok(Self::new(
            cell_area_mm2,
            orthogonal_edge_length_mm,
            timestep_millis,
        ))
    }

    pub const fn schema_version(self) -> u16 {
        self.schema_version
    }

    pub const fn cell_area_mm2(self) -> NonZeroU64 {
        self.cell_area_mm2
    }

    pub const fn orthogonal_edge_length_mm(self) -> NonZeroU64 {
        self.orthogonal_edge_length_mm
    }

    pub const fn timestep_millis(self) -> NonZeroU64 {
        self.timestep_millis
    }

    /// Split a volume into whole millimetres of depth and the sub-millimetre
    /// remainder.
    ///
    /// The remainder is returned rather than discarded because it stays in the
    /// donor bucket. Dropping it would be a quantisation sink: small, invisible
    /// per cell per tick, and unbounded over a run — and the conservation
    /// receipt would still close, because nothing would be there to disagree
    /// with. Head comparisons use the whole-millimetre part; the remainder
    /// never becomes head and never leaves storage.
    pub fn split_depth(
        self,
        volume: WaterVolume,
    ) -> Result<HydrologyDepthSplit, HydrologyStateError> {
        let area = i128::from(self.cell_area_mm2.get());
        let depth = checked_water_div_floor(volume.as_i128(), area)?;
        let remainder = volume
            .as_i128()
            .checked_sub(checked_water_mul(depth, area)?)
            .ok_or(causafera_types::WaterVolumeError::Underflow)?;
        Ok(HydrologyDepthSplit {
            depth: WaterDepthMm::from_i128(depth)?,
            remainder: WaterVolume::from_i128(remainder)?,
        })
    }

    /// Whole millimetres of depth `volume` covers at this metric.
    pub fn depth_of(self, volume: WaterVolume) -> Result<WaterDepthMm, HydrologyStateError> {
        Ok(self.split_depth(volume)?.depth)
    }

    /// The volume exactly `depth` millimetres over one cell occupies.
    pub fn volume_of(self, depth: WaterDepthMm) -> Result<WaterVolume, HydrologyStateError> {
        let product = checked_water_mul(depth.as_i128(), i128::from(self.cell_area_mm2.get()))?;
        Ok(WaterVolume::from_i128(product)?)
    }
}

/// A volume's whole-millimetre depth and the remainder that stays in storage.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HydrologyDepthSplit {
    pub depth: WaterDepthMm,
    pub remainder: WaterVolume,
}

/// The registered hydrology grid metric of every participating chart.
///
/// Cross-chart transport is outside this tranche, so charts share nothing but
/// this registry: each has its own area, edge length, and timestep, and a
/// chunk whose chart is not registered is rejected rather than defaulted.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HydrologyGridMetrics {
    metrics: BTreeMap<SpatialChartId, HydrologyGridMetric>,
}

impl HydrologyGridMetrics {
    pub fn new(
        entries: Vec<(SpatialChartId, HydrologyGridMetric)>,
    ) -> Result<Self, HydrologyStateError> {
        if entries.is_empty() {
            return Err(HydrologyStateError::EmptyMetrics);
        }
        // Counted before the map is built, so a caller cannot spend memory on
        // the way to a rejection by handing over a long list of duplicates.
        if entries.len() > MAX_HYDROLOGY_CHARTS {
            return Err(HydrologyStateError::ChartCountExceeded {
                count: entries.len(),
                max: MAX_HYDROLOGY_CHARTS,
            });
        }
        let mut metrics = BTreeMap::new();
        for (chart, metric) in entries {
            if metrics.insert(chart, metric).is_some() {
                return Err(HydrologyStateError::DuplicateMetricChart);
            }
        }
        Ok(Self { metrics })
    }

    pub fn get(&self, chart: SpatialChartId) -> Result<HydrologyGridMetric, HydrologyStateError> {
        self.metrics
            .get(&chart)
            .copied()
            .ok_or(HydrologyStateError::UnknownMetricChart)
    }

    pub fn contains(&self, chart: SpatialChartId) -> bool {
        self.metrics.contains_key(&chart)
    }

    pub fn entries(&self) -> &BTreeMap<SpatialChartId, HydrologyGridMetric> {
        &self.metrics
    }

    pub fn len(&self) -> usize {
        self.metrics.len()
    }

    pub fn is_empty(&self) -> bool {
        self.metrics.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn nz(value: u64) -> NonZeroU64 {
        NonZeroU64::new(value).expect("test metric components are positive")
    }

    fn metric(area: u64) -> HydrologyGridMetric {
        HydrologyGridMetric::new(nz(area), nz(1_000), nz(1_000))
    }

    #[test]
    fn depth_conversion_keeps_its_remainder_in_the_donor_bucket() {
        // Given: a cell of 1000 mm² holding a volume that is not a whole
        // number of millimetres deep.
        let metric = metric(1_000);

        // When: the volume is split into depth and remainder.
        let split = metric.split_depth(WaterVolume::new(2_500)).unwrap();

        // Then: the depth floors and the remainder is handed back rather than
        // dropped. Nothing here writes the remainder anywhere — that is the
        // caller's job, and it can only do it because it is returned.
        assert_eq!(split.depth, WaterDepthMm::new(2));
        assert_eq!(split.remainder, WaterVolume::new(500));
        assert_eq!(
            i128::from(split.depth.millimetres()) * 1_000 + split.remainder.as_i128(),
            2_500,
            "depth and remainder must reconstruct the volume exactly"
        );
    }

    #[test]
    fn depth_and_volume_round_trip_on_exact_multiples() {
        let metric = metric(64);
        for depth in [0_u64, 1, 7, 1_000] {
            let volume = metric.volume_of(WaterDepthMm::new(depth)).unwrap();
            assert_eq!(volume, WaterVolume::new(depth * 64));
            assert_eq!(metric.depth_of(volume).unwrap(), WaterDepthMm::new(depth));
            assert_eq!(
                metric.split_depth(volume).unwrap().remainder,
                WaterVolume::ZERO
            );
        }
    }

    #[test]
    fn a_volume_below_one_cell_millimetre_is_all_remainder() {
        // The interesting quantisation case: real cells are large, so most
        // per-tick movements are fractions of a millimetre. If the remainder
        // were dropped here the loss would be total rather than partial.
        let metric = metric(1_000_000);
        let split = metric.split_depth(WaterVolume::new(999_999)).unwrap();
        assert_eq!(split.depth, WaterDepthMm::ZERO);
        assert_eq!(split.remainder, WaterVolume::new(999_999));
    }

    #[test]
    fn a_zero_metric_component_cannot_be_decoded() {
        // `NonZeroU64` refuses zero at the constructor, so the reachable way in
        // is a decoded snapshot. Each component is checked there.
        for (area, edge, timestep) in [(0, 1, 1), (1, 0, 1), (1, 1, 0)] {
            assert!(matches!(
                HydrologyGridMetric::from_parts(1, area, edge, timestep),
                Err(HydrologyStateError::UnsupportedMetricSchema(1))
            ));
        }
        assert_eq!(
            HydrologyGridMetric::from_parts(1, 8, 4, 2),
            Ok(HydrologyGridMetric::new(nz(8), nz(4), nz(2)))
        );
    }

    #[test]
    fn an_unsupported_metric_schema_is_rejected() {
        for schema in [0_u16, 2, u16::MAX] {
            assert_eq!(
                HydrologyGridMetric::from_parts(schema, 1, 1, 1),
                Err(HydrologyStateError::UnsupportedMetricSchema(schema))
            );
        }
    }

    #[test]
    fn volume_conversion_rejects_an_overflowing_depth() {
        let metric = metric(1_000);
        assert!(matches!(
            metric.volume_of(WaterDepthMm::new(u64::MAX)),
            Err(HydrologyStateError::Arithmetic(_))
        ));
    }

    #[test]
    fn the_metric_registry_enforces_its_bounds_and_ordering() {
        let chart = SpatialChartId::new(1);

        assert_eq!(
            HydrologyGridMetrics::new(Vec::new()),
            Err(HydrologyStateError::EmptyMetrics)
        );
        assert_eq!(
            HydrologyGridMetrics::new(vec![(chart, metric(1)), (chart, metric(2))]),
            Err(HydrologyStateError::DuplicateMetricChart)
        );

        let over = (0..=MAX_HYDROLOGY_CHARTS)
            .map(|index| (SpatialChartId::new(index as u64 + 1), metric(1)))
            .collect::<Vec<_>>();
        assert_eq!(
            HydrologyGridMetrics::new(over),
            Err(HydrologyStateError::ChartCountExceeded {
                count: MAX_HYDROLOGY_CHARTS + 1,
                max: MAX_HYDROLOGY_CHARTS,
            })
        );

        let at_bound = (0..MAX_HYDROLOGY_CHARTS)
            .map(|index| (SpatialChartId::new(index as u64 + 1), metric(1)))
            .collect::<Vec<_>>();
        assert_eq!(
            HydrologyGridMetrics::new(at_bound).unwrap().len(),
            MAX_HYDROLOGY_CHARTS
        );
    }

    #[test]
    fn registry_construction_is_independent_of_input_order() {
        let a = (SpatialChartId::new(7), metric(11));
        let b = (SpatialChartId::new(3), metric(13));
        assert_eq!(
            HydrologyGridMetrics::new(vec![a, b]).unwrap(),
            HydrologyGridMetrics::new(vec![b, a]).unwrap()
        );
    }

    #[test]
    fn an_unregistered_chart_is_rejected_rather_than_defaulted() {
        let metrics = HydrologyGridMetrics::new(vec![(SpatialChartId::new(1), metric(4))]).unwrap();
        assert!(metrics.contains(SpatialChartId::new(1)));
        assert_eq!(
            metrics.get(SpatialChartId::new(2)),
            Err(HydrologyStateError::UnknownMetricChart)
        );
    }
}
