//! Shared hydrology fixtures for the domain-level integration tests.
//!
//! These build validated value objects directly and use a stub trace resolver.
//! Per `plans/hydrology.md`'s Verification preamble that is legitimate for
//! constructor and reducer coverage, and explicitly *not* evidence about
//! replay, bootstrap, persistence, provenance, or maturity — those need
//! engine-produced state and live in the runtime suite.
//!
//! Each integration test binary compiles this module separately and uses a
//! different part of it, so an unused builder here is a fact about one binary
//! rather than about the fixture.
#![allow(dead_code)]

use std::collections::BTreeSet;
use std::num::{NonZeroU32, NonZeroU64};

use causafera_core::StateFingerprint;
use causafera_domains::{HydrologyEvolutionLimits, HydrologyEvolutionRequest};
use causafera_geography::{
    HydraulicFraction, HydraulicSubstrateCell, HydraulicSubstrateParts, HydrologyActiveRegion,
    HydrologyBoundaryMap, HydrologyCellKey, HydrologyCellState, HydrologyCellStorage,
    HydrologyConveyanceGraph, HydrologyField, HydrologyFieldSet, HydrologyForcingMember,
    HydrologyForcingParts, HydrologyForcingRecord, HydrologyGridMetric, HydrologyGridMetrics,
    SURFACE_CELL_COUNT,
};
use causafera_types::{ChartChunkCoord, ChunkCoord, SpatialChartId, TraceId, WaterVolume};

pub const BOOTSTRAP_TRACE: TraceId = TraceId::new(1);
pub const ORIGIN_TRACE: TraceId = TraceId::new(2);
pub const PREVIOUS_CONSERVATION: TraceId = TraceId::new(3);
pub const POLICY: u64 = causafera_geography::BOOTSTRAP_HYDROLOGY_FORCING_POLICY_V1;

pub fn chart() -> SpatialChartId {
    SpatialChartId::new(1)
}

pub fn chunk(x: i32) -> ChartChunkCoord {
    ChartChunkCoord::new(chart(), ChunkCoord::new(x, 0, 0))
}

pub fn cell(chunk_x: i32, ordinal: u16) -> HydrologyCellKey {
    HydrologyCellKey::new(chunk(chunk_x), ordinal).expect("ordinal is in range")
}

pub fn nz32(value: u32) -> NonZeroU32 {
    NonZeroU32::new(value).expect("denominator is positive")
}

pub fn nz64(value: u64) -> NonZeroU64 {
    NonZeroU64::new(value).expect("value is positive")
}

pub fn fraction(numerator: u32, denominator: u32) -> HydraulicFraction {
    HydraulicFraction::new(numerator, nz32(denominator)).expect("fraction is within [0, 1]")
}

pub fn metrics() -> HydrologyGridMetrics {
    // One square metre of ground, one metre between cell centres, one second per
    // tick. Every derived per-tick coefficient comes from these three numbers.
    HydrologyGridMetrics::new(vec![(
        chart(),
        HydrologyGridMetric::new(nz64(1_000_000), nz64(1_000), nz64(1_000)),
    )])
    .expect("one chart is a valid registry")
}

/// Ground nothing can happen on: no capacity, no limit, no conductance.
///
/// The background of every fixture, so a test's assertions are about the cells
/// it configured rather than about 1 024 incidental ones.
pub fn inert_substrate() -> HydraulicSubstrateCell {
    HydraulicSubstrateCell::new(HydraulicSubstrateParts {
        surface_capacity: WaterVolume::ZERO,
        soil_capacity: WaterVolume::ZERO,
        groundwater_capacity: WaterVolume::ZERO,
        infiltration_limit_per_tick: WaterVolume::ZERO,
        percolation_fraction: HydraulicFraction::ZERO,
        specific_yield: HydraulicFraction::ZERO,
        aquifer_base_elevation_mm: 0,
        baseflow_threshold: WaterVolume::ZERO,
        baseflow_fraction: HydraulicFraction::ZERO,
        surface_conductance_mm2_per_tick: 0,
        groundwater_conductance_mm2_per_tick: 0,
    })
    .expect("inert substrate is valid")
}

/// A substrate builder with sensible defaults each test overrides in one place.
#[derive(Clone, Copy, Debug)]
pub struct Ground {
    pub surface_capacity: u64,
    pub soil_capacity: u64,
    pub groundwater_capacity: u64,
    pub infiltration_limit: u64,
    pub percolation: (u32, u32),
    pub specific_yield: (u32, u32),
    pub aquifer_base_mm: i64,
    pub baseflow_threshold: u64,
    pub baseflow: (u32, u32),
    pub surface_conductance: u64,
    pub groundwater_conductance: u64,
}

impl Default for Ground {
    fn default() -> Self {
        Self {
            surface_capacity: 1_000_000,
            soil_capacity: 1_000_000,
            groundwater_capacity: 1_000_000,
            infiltration_limit: 1_000,
            percolation: (1, 4),
            specific_yield: (1, 5),
            aquifer_base_mm: 0,
            baseflow_threshold: 0,
            baseflow: (0, 1),
            surface_conductance: 0,
            groundwater_conductance: 0,
        }
    }
}

impl Ground {
    pub fn build(self) -> HydraulicSubstrateCell {
        HydraulicSubstrateCell::new(HydraulicSubstrateParts {
            surface_capacity: WaterVolume::new(self.surface_capacity),
            soil_capacity: WaterVolume::new(self.soil_capacity),
            groundwater_capacity: WaterVolume::new(self.groundwater_capacity),
            infiltration_limit_per_tick: WaterVolume::new(self.infiltration_limit),
            percolation_fraction: fraction(self.percolation.0, self.percolation.1),
            specific_yield: fraction(self.specific_yield.0, self.specific_yield.1),
            aquifer_base_elevation_mm: self.aquifer_base_mm,
            baseflow_threshold: WaterVolume::new(self.baseflow_threshold),
            baseflow_fraction: fraction(self.baseflow.0, self.baseflow.1),
            surface_conductance_mm2_per_tick: self.surface_conductance,
            groundwater_conductance_mm2_per_tick: self.groundwater_conductance,
        })
        .expect("configured substrate is valid")
    }
}

pub fn storage(surface: u64, soil: u64, groundwater: u64) -> HydrologyCellStorage {
    HydrologyCellStorage::new(
        WaterVolume::new(surface),
        WaterVolume::new(soil),
        WaterVolume::new(groundwater),
    )
}

/// One chunk whose cells are inert except at the ordinals given.
pub struct ChunkBuilder {
    chunk: ChartChunkCoord,
    cells: Vec<HydrologyCellState>,
    substrate: Vec<HydraulicSubstrateCell>,
}

impl ChunkBuilder {
    pub fn new(chunk_x: i32) -> Self {
        Self {
            chunk: chunk(chunk_x),
            cells: vec![
                HydrologyCellState::initial(
                    HydrologyCellStorage::ZERO,
                    BOOTSTRAP_TRACE,
                    StateFingerprint::new([0; 32]),
                );
                SURFACE_CELL_COUNT
            ],
            substrate: vec![inert_substrate(); SURFACE_CELL_COUNT],
        }
    }

    pub fn with(
        mut self,
        ordinal: u16,
        ground: HydraulicSubstrateCell,
        storage: HydrologyCellStorage,
    ) -> Self {
        let index = usize::from(ordinal);
        self.substrate[index] = ground;
        self.cells[index] =
            HydrologyCellState::initial(storage, BOOTSTRAP_TRACE, StateFingerprint::new([0; 32]));
        self
    }

    /// Distinct pre-tick trace anchors per bucket, so a test can tell which
    /// bucket's prior trace an event actually cited.
    pub fn with_traces(
        mut self,
        ordinal: u16,
        surface: TraceId,
        soil: TraceId,
        groundwater: TraceId,
    ) -> Self {
        let index = usize::from(ordinal);
        let existing = self.cells[index];
        self.cells[index] = HydrologyCellState::from_parts(
            existing.storage(),
            surface,
            soil,
            groundwater,
            existing.forcing_input_fingerprint(),
            existing.forcing_last_change(),
            existing.last_change_before(),
        );
        self
    }

    pub fn build(self) -> HydrologyField {
        HydrologyField::from_parts(self.chunk, self.cells, self.substrate)
            .expect("a full field is valid")
    }
}

pub fn field_set(fields: Vec<HydrologyField>) -> HydrologyFieldSet {
    let metrics = metrics();
    HydrologyFieldSet::new(fields, &metrics, PREVIOUS_CONSERVATION).expect("field set is valid")
}

pub fn active(chunks: &[i32]) -> HydrologyActiveRegion {
    let resident: BTreeSet<_> = chunks.iter().map(|&x| chunk(x)).collect();
    HydrologyActiveRegion::new(resident.clone(), resident).expect("active region is valid")
}

/// A forcing record builder.
#[derive(Clone, Debug)]
pub struct Forcing {
    pub id: u64,
    pub tick: u64,
    pub targets: Vec<(HydrologyCellKey, u64)>,
    pub precipitation: u64,
    pub potential_et: u64,
    pub external_inflow: u64,
    pub origin: TraceId,
}

impl Forcing {
    pub fn new(id: u64, tick: u64) -> Self {
        Self {
            id,
            tick,
            targets: Vec::new(),
            precipitation: 0,
            potential_et: 0,
            external_inflow: 0,
            origin: ORIGIN_TRACE,
        }
    }

    pub fn target(mut self, cell: HydrologyCellKey, weight: u64) -> Self {
        self.targets.push((cell, weight));
        self
    }

    pub fn precipitation(mut self, volume: u64) -> Self {
        self.precipitation = volume;
        self
    }

    pub fn potential_et(mut self, volume: u64) -> Self {
        self.potential_et = volume;
        self
    }

    pub fn external_inflow(mut self, volume: u64) -> Self {
        self.external_inflow = volume;
        self
    }

    pub fn origin(mut self, origin: TraceId) -> Self {
        self.origin = origin;
        self
    }

    /// Build the record, sorting targets so a test can list them readably.
    /// Canonical order is a *contract* on the record — this fixture satisfies it
    /// rather than exercising it; `hydrology::forcing`'s own unit tests cover
    /// the rejection of an unsorted or duplicated member list.
    pub fn build(mut self) -> HydrologyForcingRecord {
        self.targets.sort_by_key(|(cell, _)| *cell);
        HydrologyForcingRecord::new(HydrologyForcingParts {
            forcing_id: self.id,
            scheduled_tick: self.tick,
            targets: self
                .targets
                .into_iter()
                .map(|(cell, weight)| HydrologyForcingMember::new(cell, nz64(weight)))
                .collect(),
            precipitation_volume: WaterVolume::new(self.precipitation),
            potential_et_volume: WaterVolume::new(self.potential_et),
            external_inflow_volume: WaterVolume::new(self.external_inflow),
            origin_trace: self.origin,
            producer_policy_schema: POLICY,
            applied_at: None,
        })
        .expect("configured record is valid")
    }
}

/// A request over the given state with no conveyance and no boundaries — the
/// shape Stage 3's vertical cycle runs in.
pub struct Scenario {
    pub metrics: HydrologyGridMetrics,
    pub active: HydrologyActiveRegion,
    pub conveyance: HydrologyConveyanceGraph,
    pub boundaries: HydrologyBoundaryMap,
    pub forcing: Vec<HydrologyForcingRecord>,
}

impl Scenario {
    pub fn new(chunks: &[i32]) -> Self {
        Self {
            metrics: metrics(),
            active: active(chunks),
            conveyance: HydrologyConveyanceGraph::default(),
            boundaries: HydrologyBoundaryMap::default(),
            forcing: Vec::new(),
        }
    }

    pub fn with_forcing(mut self, records: Vec<HydrologyForcingRecord>) -> Self {
        self.forcing = records;
        self
    }

    pub fn request(&self, tick: u64) -> HydrologyEvolutionRequest<'_> {
        HydrologyEvolutionRequest {
            tick,
            metrics: &self.metrics,
            active: &self.active,
            conveyance: &self.conveyance,
            boundaries: &self.boundaries,
            forcing: &self.forcing,
            previous_conservation: PREVIOUS_CONSERVATION,
            limits: HydrologyEvolutionLimits::default(),
        }
    }
}
