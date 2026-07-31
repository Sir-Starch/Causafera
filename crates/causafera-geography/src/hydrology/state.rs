use std::collections::{BTreeMap, BTreeSet};

use causafera_core::StateFingerprint;
use causafera_types::{
    CHUNK_SIZE, ChartChunkCoord, ChunkCoord, SpatialChartId, TraceId, WaterAccumulator, WaterVolume,
};

use super::{
    HydraulicSubstrateCell, HydrologyGridMetrics, HydrologyStateError,
    MAX_HYDROLOGY_BOUNDARY_RECORDS, MAX_HYDROLOGY_CELLS, MAX_HYDROLOGY_CHARTS,
    MAX_HYDROLOGY_CHUNKS, MAX_HYDROLOGY_EDGES, SURFACE_CELL_COUNT,
};

/// The carrier-key encoding this build reads and writes.
pub const HYDROLOGY_CARRIER_KEY_VERSION: u8 = 1;

/// The coarsest hydrology resolution level. Block edge is `2^min(level, 4)`.
pub const MAX_HYDROLOGY_RESOLUTION_LEVEL: u8 = 4;

/// Bytes in one encoded cell body, without its variant tag.
const CELL_BODY_LEN: usize = 22;

const VARIANT_CELL: u8 = 0x01;
const VARIANT_EDGE: u8 = 0x02;
const VARIANT_EXTERIOR_FACE: u8 = 0x03;
const VARIANT_FORCING_RECORD: u8 = 0x04;
const VARIANT_RESOLUTION_CHUNK: u8 = 0x05;
const VARIANT_BATCH_NODE: u8 = 0x06;

// ---------------------------------------------------------------------------
// Lattice addressing
// ---------------------------------------------------------------------------

/// One cell of the terrain-aligned hydrology surface lattice.
///
/// Ordinals are row-major `y * CHUNK_SIZE + x` over the same `CHUNK_SIZE²`
/// surface a terrain chunk carries, so a hydrology cell and a terrain cell with
/// the same address are the same piece of ground. Adjacency is four-face; there
/// are no vertical faces, because the vertical direction is modelled as the
/// surface/soil/groundwater bucket stack rather than as geometry.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HydrologyCellKey {
    chunk: ChartChunkCoord,
    cell_ordinal: u16,
}

impl HydrologyCellKey {
    pub fn new(chunk: ChartChunkCoord, cell_ordinal: u16) -> Result<Self, HydrologyStateError> {
        if usize::from(cell_ordinal) >= SURFACE_CELL_COUNT {
            return Err(HydrologyStateError::CellOrdinalOutOfRange {
                ordinal: cell_ordinal,
                count: SURFACE_CELL_COUNT,
            });
        }
        Ok(Self {
            chunk,
            cell_ordinal,
        })
    }

    /// Address a cell by its local surface position. Infallible: both
    /// coordinates are already narrower than `CHUNK_SIZE`.
    pub fn from_local(chunk: ChartChunkCoord, x: u8, y: u8) -> Result<Self, HydrologyStateError> {
        if x >= CHUNK_SIZE || y >= CHUNK_SIZE {
            return Err(HydrologyStateError::CellOrdinalOutOfRange {
                ordinal: u16::from(y) * u16::from(CHUNK_SIZE) + u16::from(x),
                count: SURFACE_CELL_COUNT,
            });
        }
        Self::new(chunk, u16::from(y) * u16::from(CHUNK_SIZE) + u16::from(x))
    }

    pub const fn chunk(self) -> ChartChunkCoord {
        self.chunk
    }

    pub const fn chart(self) -> SpatialChartId {
        self.chunk.chart
    }

    pub const fn cell_ordinal(self) -> u16 {
        self.cell_ordinal
    }

    /// Local `(x, y)` within the owning chunk's surface.
    pub const fn local(self) -> (u8, u8) {
        (
            (self.cell_ordinal % CHUNK_SIZE as u16) as u8,
            (self.cell_ordinal / CHUNK_SIZE as u16) as u8,
        )
    }

    /// The neighbouring cell across one orthogonal face.
    ///
    /// A face on the chunk edge resolves into the adjacent chunk of the *same
    /// chart*, preserving the orthogonal coordinate, so a chunk seam is an
    /// ordinary interior face and never a wall (INV-043). `None` means the
    /// neighbouring chunk address is not representable — the chart runs out of
    /// `i32` — which callers treat exactly like a missing resident neighbour:
    /// an exterior face needing an explicit boundary record.
    pub fn neighbor(self, direction: FaceDirection) -> Option<Self> {
        let (x, y) = self.local();
        let last = CHUNK_SIZE - 1;
        let (chunk, nx, ny) = match direction {
            FaceDirection::NegX if x == 0 => (checked_neighbor(self.chunk, -1, 0)?, last, y),
            FaceDirection::NegX => (self.chunk, x - 1, y),
            FaceDirection::PosX if x == last => (checked_neighbor(self.chunk, 1, 0)?, 0, y),
            FaceDirection::PosX => (self.chunk, x + 1, y),
            FaceDirection::NegY if y == 0 => (checked_neighbor(self.chunk, 0, -1)?, x, last),
            FaceDirection::NegY => (self.chunk, x, y - 1),
            FaceDirection::PosY if y == last => (checked_neighbor(self.chunk, 0, 1)?, x, 0),
            FaceDirection::PosY => (self.chunk, x, y + 1),
        };
        Self::from_local(chunk, nx, ny).ok()
    }

    /// The face direction from `self` to `other`, or `None` if they are not
    /// orthogonally adjacent within one chart.
    pub fn adjacency(self, other: Self) -> Option<FaceDirection> {
        FaceDirection::ALL
            .into_iter()
            .find(|&direction| self.neighbor(direction) == Some(other))
    }

    fn write_body(self, out: &mut Vec<u8>) {
        out.extend_from_slice(&self.chunk.chart.raw().to_be_bytes());
        out.extend_from_slice(&self.chunk.chunk.x.to_be_bytes());
        out.extend_from_slice(&self.chunk.chunk.y.to_be_bytes());
        out.extend_from_slice(&self.chunk.chunk.z.to_be_bytes());
        out.extend_from_slice(&self.cell_ordinal.to_be_bytes());
    }

    fn read_body(bytes: &[u8]) -> Result<Self, HydrologyStateError> {
        debug_assert_eq!(bytes.len(), CELL_BODY_LEN);
        let chart = SpatialChartId::new(u64::from_be_bytes(
            bytes[0..8].try_into().expect("eight bytes"),
        ));
        let x = i32::from_be_bytes(bytes[8..12].try_into().expect("four bytes"));
        let y = i32::from_be_bytes(bytes[12..16].try_into().expect("four bytes"));
        let z = i32::from_be_bytes(bytes[16..20].try_into().expect("four bytes"));
        let ordinal = u16::from_be_bytes(bytes[20..22].try_into().expect("two bytes"));
        Self::new(
            ChartChunkCoord::new(chart, ChunkCoord::new(x, y, z)),
            ordinal,
        )
    }
}

fn checked_neighbor(chunk: ChartChunkCoord, dx: i32, dy: i32) -> Option<ChartChunkCoord> {
    Some(ChartChunkCoord::new(
        chunk.chart,
        ChunkCoord::new(
            chunk.chunk.x.checked_add(dx)?,
            chunk.chunk.y.checked_add(dy)?,
            chunk.chunk.z,
        ),
    ))
}

/// One of the four orthogonal surface faces.
///
/// The discriminants are the wire encoding (`0 = -X, 1 = +X, 2 = -Y, 3 = +Y`)
/// and are part of the carrier-key contract, not an implementation detail.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FaceDirection {
    NegX = 0,
    PosX = 1,
    NegY = 2,
    PosY = 3,
}

impl FaceDirection {
    pub const ALL: [Self; 4] = [Self::NegX, Self::PosX, Self::NegY, Self::PosY];

    pub const fn code(self) -> u8 {
        self as u8
    }

    pub const fn from_code(code: u8) -> Result<Self, HydrologyStateError> {
        match code {
            0 => Ok(Self::NegX),
            1 => Ok(Self::PosX),
            2 => Ok(Self::NegY),
            3 => Ok(Self::PosY),
            other => Err(HydrologyStateError::UnknownFaceDirection(other)),
        }
    }

    pub const fn opposite(self) -> Self {
        match self {
            Self::NegX => Self::PosX,
            Self::PosX => Self::NegX,
            Self::NegY => Self::PosY,
            Self::PosY => Self::NegY,
        }
    }
}

/// The canonical identity of one interior face: its two endpoints in ascending
/// key order, so the same face has one name whichever side names it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HydrologyEdgeKey {
    low: HydrologyCellKey,
    high: HydrologyCellKey,
}

impl HydrologyEdgeKey {
    pub fn new(a: HydrologyCellKey, b: HydrologyCellKey) -> Result<Self, HydrologyStateError> {
        if a == b {
            return Err(HydrologyStateError::DegenerateEdge);
        }
        Ok(if a < b {
            Self { low: a, high: b }
        } else {
            Self { low: b, high: a }
        })
    }

    pub const fn low(self) -> HydrologyCellKey {
        self.low
    }

    pub const fn high(self) -> HydrologyCellKey {
        self.high
    }

    pub fn contains(self, cell: HydrologyCellKey) -> bool {
        self.low == cell || self.high == cell
    }

    /// The endpoint that is not `cell`, or `None` if `cell` is not an endpoint.
    pub fn other(self, cell: HydrologyCellKey) -> Option<HydrologyCellKey> {
        if self.low == cell {
            Some(self.high)
        } else if self.high == cell {
            Some(self.low)
        } else {
            None
        }
    }
}

/// One exterior face: a cell and the direction with no resident neighbour.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HydrologyExteriorFaceKey {
    cell: HydrologyCellKey,
    direction: FaceDirection,
}

impl HydrologyExteriorFaceKey {
    pub const fn new(cell: HydrologyCellKey, direction: FaceDirection) -> Self {
        Self { cell, direction }
    }

    pub const fn cell(self) -> HydrologyCellKey {
        self.cell
    }

    pub const fn direction(self) -> FaceDirection {
        self.direction
    }
}

// ---------------------------------------------------------------------------
// Boundaries
// ---------------------------------------------------------------------------

/// What one channel of one exterior face does with water reaching it.
///
/// There is no third case. A face is either closed or it is open with a stated
/// external head and conductance — never "no record, so assume something".
/// A missing resident neighbour without a boundary record is a validation
/// failure, because silently exporting and silently blocking are both a
/// physical claim nobody made (verification gate V13).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FluxBoundary {
    NoFlux,
    Open {
        external_head_mm: i64,
        conductance_mm2_per_tick: u64,
    },
}

impl FluxBoundary {
    pub const fn is_open(self) -> bool {
        matches!(self, Self::Open { .. })
    }
}

/// Surface and groundwater cross an exterior face independently.
///
/// A face can be open to a water body at the surface while the aquifer beneath
/// it is closed, or the reverse. Collapsing the two into one condition would
/// make that unrepresentable.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HydrologyBoundaryCondition {
    pub surface: FluxBoundary,
    pub groundwater: FluxBoundary,
}

impl HydrologyBoundaryCondition {
    pub const CLOSED: Self = Self {
        surface: FluxBoundary::NoFlux,
        groundwater: FluxBoundary::NoFlux,
    };

    pub const fn new(surface: FluxBoundary, groundwater: FluxBoundary) -> Self {
        Self {
            surface,
            groundwater,
        }
    }

    /// The tuple two cells' faces must share to sit in one constitutive group.
    ///
    /// Openness and its parameters both participate: two faces open onto
    /// different external heads are not interchangeable, and averaging them
    /// would invent a boundary neither cell has.
    pub const fn constitutive_kind(&self) -> (u8, i64, u64, u8, i64, u64) {
        let (surface_kind, surface_head, surface_conductance) = match self.surface {
            FluxBoundary::NoFlux => (0, 0, 0),
            FluxBoundary::Open {
                external_head_mm,
                conductance_mm2_per_tick,
            } => (1, external_head_mm, conductance_mm2_per_tick),
        };
        let (ground_kind, ground_head, ground_conductance) = match self.groundwater {
            FluxBoundary::NoFlux => (0, 0, 0),
            FluxBoundary::Open {
                external_head_mm,
                conductance_mm2_per_tick,
            } => (1, external_head_mm, conductance_mm2_per_tick),
        };
        (
            surface_kind,
            surface_head,
            surface_conductance,
            ground_kind,
            ground_head,
            ground_conductance,
        )
    }
}

/// Every exterior face's explicit condition.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HydrologyBoundaryMap {
    records: BTreeMap<HydrologyExteriorFaceKey, HydrologyBoundaryCondition>,
}

impl HydrologyBoundaryMap {
    pub fn new(
        records: Vec<(HydrologyExteriorFaceKey, HydrologyBoundaryCondition)>,
    ) -> Result<Self, HydrologyStateError> {
        if records.len() > MAX_HYDROLOGY_BOUNDARY_RECORDS {
            return Err(HydrologyStateError::BoundaryCountExceeded {
                count: records.len(),
                max: MAX_HYDROLOGY_BOUNDARY_RECORDS,
            });
        }
        let mut map = BTreeMap::new();
        for (face, condition) in records {
            if map.insert(face, condition).is_some() {
                return Err(HydrologyStateError::DuplicateBoundaryFace);
            }
        }
        Ok(Self { records: map })
    }

    pub fn get(&self, face: HydrologyExteriorFaceKey) -> Option<HydrologyBoundaryCondition> {
        self.records.get(&face).copied()
    }

    pub fn records(&self) -> &BTreeMap<HydrologyExteriorFaceKey, HydrologyBoundaryCondition> {
        &self.records
    }

    pub fn len(&self) -> usize {
        self.records.len()
    }

    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }
}

// ---------------------------------------------------------------------------
// Cell state
// ---------------------------------------------------------------------------

/// The three storage buckets of one cell.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HydrologyCellStorage {
    pub surface: WaterVolume,
    pub soil: WaterVolume,
    pub groundwater: WaterVolume,
}

impl HydrologyCellStorage {
    pub const ZERO: Self = Self {
        surface: WaterVolume::ZERO,
        soil: WaterVolume::ZERO,
        groundwater: WaterVolume::ZERO,
    };

    pub const fn new(surface: WaterVolume, soil: WaterVolume, groundwater: WaterVolume) -> Self {
        Self {
            surface,
            soil,
            groundwater,
        }
    }

    /// Exact total in the accumulation domain. Never narrowed to `u64`: three
    /// whole-range buckets do not fit one, and the conservation ledger sums
    /// this over every cell in the world.
    pub fn total(self) -> Result<WaterAccumulator, HydrologyStateError> {
        Ok(WaterAccumulator::ZERO
            .add_volume(self.surface)?
            .add_volume(self.soil)?
            .add_volume(self.groundwater)?)
    }
}

/// One cell's authoritative water state and its trace anchors.
///
/// Each bucket carries its own `last_change`, because the three move for
/// different reasons in different substages and a shared anchor would make a
/// soil change look like a surface one. `forcing_input_fingerprint` and
/// `forcing_last_change` are the cell's durable record of what forcing it was
/// handed, and they are written even when the accepted source and accepted ET
/// both come out zero: "the record targeted this cell and nothing fit" is
/// evidence, and dropping it would make a rejected input indistinguishable from
/// an absent one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HydrologyCellState {
    storage: HydrologyCellStorage,
    surface_last_change: TraceId,
    soil_last_change: TraceId,
    groundwater_last_change: TraceId,
    forcing_input_fingerprint: StateFingerprint,
    forcing_last_change: TraceId,
    last_change_before: HydrologyCellStorage,
}

impl HydrologyCellState {
    /// The state a causally initialized cell starts in: every anchor pointing
    /// at the bootstrap event that created it, and no prior storage to remember.
    pub fn initial(
        storage: HydrologyCellStorage,
        bootstrap_trace: TraceId,
        forcing_input_fingerprint: StateFingerprint,
    ) -> Self {
        Self {
            storage,
            surface_last_change: bootstrap_trace,
            soil_last_change: bootstrap_trace,
            groundwater_last_change: bootstrap_trace,
            forcing_input_fingerprint,
            forcing_last_change: bootstrap_trace,
            last_change_before: storage,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub const fn from_parts(
        storage: HydrologyCellStorage,
        surface_last_change: TraceId,
        soil_last_change: TraceId,
        groundwater_last_change: TraceId,
        forcing_input_fingerprint: StateFingerprint,
        forcing_last_change: TraceId,
        last_change_before: HydrologyCellStorage,
    ) -> Self {
        Self {
            storage,
            surface_last_change,
            soil_last_change,
            groundwater_last_change,
            forcing_input_fingerprint,
            forcing_last_change,
            last_change_before,
        }
    }

    pub const fn storage(&self) -> HydrologyCellStorage {
        self.storage
    }

    pub const fn surface_water(&self) -> WaterVolume {
        self.storage.surface
    }

    pub const fn soil_water(&self) -> WaterVolume {
        self.storage.soil
    }

    pub const fn groundwater(&self) -> WaterVolume {
        self.storage.groundwater
    }

    pub const fn surface_last_change(&self) -> TraceId {
        self.surface_last_change
    }

    pub const fn soil_last_change(&self) -> TraceId {
        self.soil_last_change
    }

    pub const fn groundwater_last_change(&self) -> TraceId {
        self.groundwater_last_change
    }

    pub const fn forcing_input_fingerprint(&self) -> StateFingerprint {
        self.forcing_input_fingerprint
    }

    pub const fn forcing_last_change(&self) -> TraceId {
        self.forcing_last_change
    }

    pub const fn last_change_before(&self) -> HydrologyCellStorage {
        self.last_change_before
    }
}

/// One chunk's dense hydrology state, aligned cell-for-cell with its substrate.
///
/// Both arrays are exactly `SURFACE_CELL_COUNT` long and share an index, so a
/// cell and the ground under it cannot drift apart. Runtime `chunk_extent` is
/// not a parameter here and never will be: it sizes the mana volume, and
/// letting it size this lattice would make chunk configuration a physical
/// quantity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HydrologyField {
    chunk: ChartChunkCoord,
    cells: Vec<HydrologyCellState>,
    substrate: Vec<HydraulicSubstrateCell>,
}

impl HydrologyField {
    pub fn from_parts(
        chunk: ChartChunkCoord,
        cells: Vec<HydrologyCellState>,
        substrate: Vec<HydraulicSubstrateCell>,
    ) -> Result<Self, HydrologyStateError> {
        if cells.len() != SURFACE_CELL_COUNT {
            return Err(HydrologyStateError::InvalidFieldLength {
                expected: SURFACE_CELL_COUNT,
                actual: cells.len(),
            });
        }
        if substrate.len() != SURFACE_CELL_COUNT {
            return Err(HydrologyStateError::InvalidFieldLength {
                expected: SURFACE_CELL_COUNT,
                actual: substrate.len(),
            });
        }
        for (cell, ground) in cells.iter().zip(&substrate) {
            // Storage above its own capacity is not a state the solver can
            // reach — every process is capacity-bounded — so it can only arrive
            // by construction or by import, and both are rejected here.
            if cell.storage.surface > ground.surface_capacity()
                || cell.storage.soil > ground.soil_capacity()
                || cell.storage.groundwater > ground.groundwater_capacity()
            {
                return Err(HydrologyStateError::StorageExceedsCapacity);
            }
        }
        Ok(Self {
            chunk,
            cells,
            substrate,
        })
    }

    pub const fn chunk(&self) -> ChartChunkCoord {
        self.chunk
    }

    pub fn cells(&self) -> &[HydrologyCellState] {
        &self.cells
    }

    pub fn substrate(&self) -> &[HydraulicSubstrateCell] {
        &self.substrate
    }

    pub fn cell(&self, ordinal: u16) -> Option<&HydrologyCellState> {
        self.cells.get(usize::from(ordinal))
    }

    pub fn ground(&self, ordinal: u16) -> Option<&HydraulicSubstrateCell> {
        self.substrate.get(usize::from(ordinal))
    }

    pub fn total_storage(&self) -> Result<WaterAccumulator, HydrologyStateError> {
        let mut total = WaterAccumulator::ZERO;
        for cell in &self.cells {
            total = total.add(cell.storage.total()?.get())?;
        }
        Ok(total)
    }
}

/// Every resident hydrology chunk, keyed canonically.
/// `Default` is the empty field set a session without hydrology holds. It is not
/// a usable world — every constructor that builds a real one validates metrics,
/// residency, and capacity — but "no water anywhere" has to be representable so a
/// disabled domain is a state rather than an absence.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HydrologyFieldSet {
    fields: BTreeMap<ChartChunkCoord, HydrologyField>,
    batch_sequence: u64,
    conservation_last_change: TraceId,
}

impl HydrologyFieldSet {
    pub fn new(
        fields: Vec<HydrologyField>,
        metrics: &HydrologyGridMetrics,
        conservation_last_change: TraceId,
    ) -> Result<Self, HydrologyStateError> {
        Self::from_parts(fields, metrics, 0, conservation_last_change)
    }

    pub fn from_parts(
        fields: Vec<HydrologyField>,
        metrics: &HydrologyGridMetrics,
        batch_sequence: u64,
        conservation_last_change: TraceId,
    ) -> Result<Self, HydrologyStateError> {
        if fields.is_empty() {
            return Err(HydrologyStateError::EmptyFieldSet);
        }
        // Counted before insertion so a long duplicate list cannot spend
        // memory on its way to being rejected.
        if fields.len() > MAX_HYDROLOGY_CHUNKS {
            return Err(HydrologyStateError::ChunkCountExceeded {
                count: fields.len(),
                max: MAX_HYDROLOGY_CHUNKS,
            });
        }
        let cell_count = fields.len().saturating_mul(SURFACE_CELL_COUNT);
        if cell_count > MAX_HYDROLOGY_CELLS {
            return Err(HydrologyStateError::CellCountExceeded {
                count: cell_count,
                max: MAX_HYDROLOGY_CELLS,
            });
        }

        let mut ordered = BTreeMap::new();
        let mut charts = BTreeSet::new();
        for field in fields {
            // A chunk in an unregistered chart has no cell area, edge length,
            // or timestep, so nothing about it is computable. Rejected rather
            // than defaulted: a default metric would be an invented geometry.
            if !metrics.contains(field.chunk().chart) {
                return Err(HydrologyStateError::FieldChartWithoutMetric);
            }
            charts.insert(field.chunk().chart);
            // Defence in depth rather than a reachable path today: the metric
            // registry already caps charts at the same bound and the check
            // above rejects an unregistered chart first, so this cannot fire
            // while those two agree. It is here so that it still holds if they
            // ever stop agreeing.
            if charts.len() > MAX_HYDROLOGY_CHARTS {
                return Err(HydrologyStateError::ChartCountExceeded {
                    count: charts.len(),
                    max: MAX_HYDROLOGY_CHARTS,
                });
            }
            if ordered.insert(field.chunk(), field).is_some() {
                return Err(HydrologyStateError::DuplicateFieldChunk);
            }
        }
        Ok(Self {
            fields: ordered,
            batch_sequence,
            conservation_last_change,
        })
    }

    pub fn fields(&self) -> &BTreeMap<ChartChunkCoord, HydrologyField> {
        &self.fields
    }

    pub fn field(&self, chunk: ChartChunkCoord) -> Option<&HydrologyField> {
        self.fields.get(&chunk)
    }

    pub fn cell(&self, key: HydrologyCellKey) -> Option<&HydrologyCellState> {
        self.fields
            .get(&key.chunk())
            .and_then(|field| field.cell(key.cell_ordinal()))
    }

    pub fn ground(&self, key: HydrologyCellKey) -> Option<&HydraulicSubstrateCell> {
        self.fields
            .get(&key.chunk())
            .and_then(|field| field.ground(key.cell_ordinal()))
    }

    pub fn is_resident(&self, key: HydrologyCellKey) -> bool {
        self.fields.contains_key(&key.chunk())
    }

    pub const fn batch_sequence(&self) -> u64 {
        self.batch_sequence
    }

    pub const fn conservation_last_change(&self) -> TraceId {
        self.conservation_last_change
    }

    pub fn cell_count(&self) -> usize {
        self.fields.len() * SURFACE_CELL_COUNT
    }

    /// Install the committed trace of one bucket's settlement.
    ///
    /// Separate methods per bucket rather than one taking a discriminator: the
    /// buckets are named fields of `HydrologyCellState`, and a caller that could
    /// pass the wrong tag would be able to anchor a surface change to soil. The
    /// pre-change storage is written by the solver's after-state, so only the
    /// trace arrives here.
    pub fn install_surface_trace(
        &mut self,
        cell: HydrologyCellKey,
        trace: TraceId,
    ) -> Result<(), HydrologyStateError> {
        self.cell_mut(cell)?.surface_last_change = trace;
        Ok(())
    }

    pub fn install_soil_trace(
        &mut self,
        cell: HydrologyCellKey,
        trace: TraceId,
    ) -> Result<(), HydrologyStateError> {
        self.cell_mut(cell)?.soil_last_change = trace;
        Ok(())
    }

    pub fn install_groundwater_trace(
        &mut self,
        cell: HydrologyCellKey,
        trace: TraceId,
    ) -> Result<(), HydrologyStateError> {
        self.cell_mut(cell)?.groundwater_last_change = trace;
        Ok(())
    }

    pub fn install_forcing_trace(
        &mut self,
        cell: HydrologyCellKey,
        trace: TraceId,
    ) -> Result<(), HydrologyStateError> {
        self.cell_mut(cell)?.forcing_last_change = trace;
        Ok(())
    }

    pub fn install_conservation_trace(&mut self, trace: TraceId) {
        self.conservation_last_change = trace;
    }

    fn cell_mut(
        &mut self,
        cell: HydrologyCellKey,
    ) -> Result<&mut HydrologyCellState, HydrologyStateError> {
        self.fields
            .get_mut(&cell.chunk())
            .and_then(|field| field.cells.get_mut(usize::from(cell.cell_ordinal())))
            .ok_or(HydrologyStateError::CellNotResident)
    }

    pub fn total_storage(&self) -> Result<WaterAccumulator, HydrologyStateError> {
        let mut total = WaterAccumulator::ZERO;
        for field in self.fields.values() {
            total = total.add(field.total_storage()?.get())?;
        }
        Ok(total)
    }
}

// ---------------------------------------------------------------------------
// Conveyance
// ---------------------------------------------------------------------------

/// Physical edge storage between two adjacent cells, directed toward an outlet.
///
/// This is a channel with a volume, a capacity, and a release rule — not a
/// river, a stream, or a canal. Nothing about it is named, and nothing
/// downstream may promote it into a named thing and feed that name back into
/// the simulation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HydrologyConveyanceEdge {
    key: HydrologyEdgeKey,
    outlet: HydrologyCellKey,
    storage: WaterVolume,
    capacity: WaterVolume,
    release: super::HydraulicFraction,
    inlet_capacity_per_tick: WaterVolume,
    last_change: TraceId,
    last_change_before: WaterVolume,
}

impl HydrologyConveyanceEdge {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        key: HydrologyEdgeKey,
        outlet: HydrologyCellKey,
        storage: WaterVolume,
        capacity: WaterVolume,
        release: super::HydraulicFraction,
        inlet_capacity_per_tick: WaterVolume,
        last_change: TraceId,
        last_change_before: WaterVolume,
    ) -> Result<Self, HydrologyStateError> {
        if !key.contains(outlet) {
            return Err(HydrologyStateError::OutletNotAnEndpoint);
        }
        // The key type canonicalises any pair; only an orthogonally adjacent
        // pair is a physical face. Without this an edge could join two cells
        // across the map and move water without crossing the ground between.
        if key.low().adjacency(key.high()).is_none() {
            return Err(HydrologyStateError::NonAdjacentEdge);
        }
        if storage > capacity {
            return Err(HydrologyStateError::StorageExceedsCapacity);
        }
        Ok(Self {
            key,
            outlet,
            storage,
            capacity,
            release,
            inlet_capacity_per_tick,
            last_change,
            last_change_before,
        })
    }

    pub const fn key(&self) -> HydrologyEdgeKey {
        self.key
    }

    pub const fn outlet(&self) -> HydrologyCellKey {
        self.outlet
    }

    /// The endpoint water enters from: the one that is not the outlet.
    pub fn source(&self) -> HydrologyCellKey {
        self.key
            .other(self.outlet)
            .expect("the outlet is validated to be an endpoint")
    }

    pub const fn storage(&self) -> WaterVolume {
        self.storage
    }

    pub const fn capacity(&self) -> WaterVolume {
        self.capacity
    }

    pub const fn release(&self) -> super::HydraulicFraction {
        self.release
    }

    pub const fn inlet_capacity_per_tick(&self) -> WaterVolume {
        self.inlet_capacity_per_tick
    }

    pub const fn last_change(&self) -> TraceId {
        self.last_change
    }

    pub const fn last_change_before(&self) -> WaterVolume {
        self.last_change_before
    }

    pub const fn remaining_capacity(&self) -> WaterVolume {
        self.storage.remaining_below(self.capacity)
    }
}

/// Every conveyance edge, plus the per-cell outgoing index the solver needs.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HydrologyConveyanceGraph {
    edges: BTreeMap<HydrologyEdgeKey, HydrologyConveyanceEdge>,
    outgoing: BTreeMap<HydrologyCellKey, HydrologyEdgeKey>,
}

impl HydrologyConveyanceGraph {
    pub fn new(edges: Vec<HydrologyConveyanceEdge>) -> Result<Self, HydrologyStateError> {
        if edges.len() > MAX_HYDROLOGY_EDGES {
            return Err(HydrologyStateError::EdgeCountExceeded {
                count: edges.len(),
                max: MAX_HYDROLOGY_EDGES,
            });
        }
        let mut by_key = BTreeMap::new();
        let mut outgoing: BTreeMap<HydrologyCellKey, HydrologyEdgeKey> = BTreeMap::new();
        for edge in edges {
            // One edge per canonical face. The key is the face, so a second
            // edge on it is a duplicate however its endpoints were ordered.
            if by_key.insert(edge.key(), edge).is_some() {
                return Err(HydrologyStateError::DuplicateEdgeFace);
            }
            // One outgoing edge per cell. Baseflow and conveyance release both
            // ask a cell for "its" outgoing edge, so a second one would make
            // that question ambiguous and the answer order-dependent.
            if outgoing.insert(edge.source(), edge.key()).is_some() {
                return Err(HydrologyStateError::MultipleOutgoingEdges);
            }
        }
        Ok(Self {
            edges: by_key,
            outgoing,
        })
    }

    pub fn edges(&self) -> &BTreeMap<HydrologyEdgeKey, HydrologyConveyanceEdge> {
        &self.edges
    }

    pub fn edge(&self, key: HydrologyEdgeKey) -> Option<&HydrologyConveyanceEdge> {
        self.edges.get(&key)
    }

    /// The one edge water leaves `cell` through, if it has one. A local minimum
    /// has none and retains its water.
    pub fn outgoing(&self, cell: HydrologyCellKey) -> Option<&HydrologyConveyanceEdge> {
        self.outgoing.get(&cell).and_then(|key| self.edges.get(key))
    }

    pub fn len(&self) -> usize {
        self.edges.len()
    }

    pub fn is_empty(&self) -> bool {
        self.edges.is_empty()
    }

    pub fn total_storage(&self) -> Result<WaterAccumulator, HydrologyStateError> {
        let mut total = WaterAccumulator::ZERO;
        for edge in self.edges.values() {
            total = total.add_volume(edge.storage())?;
        }
        Ok(total)
    }

    /// Install one edge's committed settlement trace.
    pub fn install_edge_trace(
        &mut self,
        edge: HydrologyEdgeKey,
        trace: TraceId,
    ) -> Result<(), HydrologyStateError> {
        self.edges
            .get_mut(&edge)
            .ok_or(HydrologyStateError::UnknownEdge)?
            .last_change = trace;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Residency and resolution
// ---------------------------------------------------------------------------

/// Which chunks hydrology holds state for, and which it evaluates.
///
/// Active is a subset of resident, never the reverse: state that is evaluated
/// but not held has nothing to evaluate, and demotion works by narrowing the
/// active set while every fine cell stays resident and canonical.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HydrologyActiveRegion {
    active_chunks: BTreeSet<ChartChunkCoord>,
    resident_chunks: BTreeSet<ChartChunkCoord>,
}

impl HydrologyActiveRegion {
    pub fn new(
        active_chunks: BTreeSet<ChartChunkCoord>,
        resident_chunks: BTreeSet<ChartChunkCoord>,
    ) -> Result<Self, HydrologyStateError> {
        if resident_chunks.len() > MAX_HYDROLOGY_CHUNKS {
            return Err(HydrologyStateError::ChunkCountExceeded {
                count: resident_chunks.len(),
                max: MAX_HYDROLOGY_CHUNKS,
            });
        }
        if !active_chunks.is_subset(&resident_chunks) {
            return Err(HydrologyStateError::ActiveChunkNotResident);
        }
        Ok(Self {
            active_chunks,
            resident_chunks,
        })
    }

    pub fn active_chunks(&self) -> &BTreeSet<ChartChunkCoord> {
        &self.active_chunks
    }

    pub fn resident_chunks(&self) -> &BTreeSet<ChartChunkCoord> {
        &self.resident_chunks
    }
}

/// One chunk's hydrology detail level and the transition that set it.
///
/// Demotion and promotion are causal events like any other state change, so the
/// level carries provenance. No canonical storage is deleted by either: the
/// level selects how the retained fine state is *evaluated*.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HydrologyResolutionState {
    level: u8,
    last_change: TraceId,
}

impl HydrologyResolutionState {
    pub const fn new(level: u8, last_change: TraceId) -> Result<Self, HydrologyStateError> {
        if level > MAX_HYDROLOGY_RESOLUTION_LEVEL {
            return Err(HydrologyStateError::ResolutionLevelExceeded {
                level,
                max: MAX_HYDROLOGY_RESOLUTION_LEVEL,
            });
        }
        Ok(Self { level, last_change })
    }

    pub const fn level(self) -> u8 {
        self.level
    }

    pub const fn last_change(self) -> TraceId {
        self.last_change
    }

    /// Cells along one edge of a coarse block at this level.
    pub const fn block_edge(self) -> u32 {
        1_u32 << self.level
    }
}

// ---------------------------------------------------------------------------
// Carrier keys
// ---------------------------------------------------------------------------

/// The canonical byte identity of anything hydrology addresses causally.
///
/// One encoding serves three consumers that must agree exactly: local DAG
/// proposal keys, the terminal aggregation tree's leaf bytes, and the observer
/// wire. Each variant is fixed-length and big-endian, so a key is comparable as
/// bytes in the same order it compares as a value, and a decoder can reject a
/// wrong length before reading anything.
///
/// `BatchNode` is deliberately not a physical endpoint: it names a synthetic
/// aggregation node, and a transfer whose source or target decoded to one would
/// be a transfer to nowhere.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum HydrologyCarrierKey {
    Cell(HydrologyCellKey),
    Edge(HydrologyEdgeKey),
    ExteriorFace(HydrologyExteriorFaceKey),
    ForcingRecord {
        scheduled_tick: u64,
        forcing_id: u64,
    },
    ResolutionChunk(ChartChunkCoord),
    BatchNode(u64),
}

impl HydrologyCarrierKey {
    pub const fn variant(&self) -> u8 {
        match self {
            Self::Cell(_) => VARIANT_CELL,
            Self::Edge(_) => VARIANT_EDGE,
            Self::ExteriorFace(_) => VARIANT_EXTERIOR_FACE,
            Self::ForcingRecord { .. } => VARIANT_FORCING_RECORD,
            Self::ResolutionChunk(_) => VARIANT_RESOLUTION_CHUNK,
            Self::BatchNode(_) => VARIANT_BATCH_NODE,
        }
    }

    /// The exact encoded length of a variant, checked before decoding.
    pub const fn encoded_len(variant: u8) -> Result<usize, HydrologyStateError> {
        match variant {
            VARIANT_CELL => Ok(1 + CELL_BODY_LEN),
            VARIANT_EDGE => Ok(1 + 2 * CELL_BODY_LEN),
            VARIANT_EXTERIOR_FACE => Ok(1 + CELL_BODY_LEN + 1),
            VARIANT_FORCING_RECORD => Ok(1 + 8 + 8),
            VARIANT_RESOLUTION_CHUNK => Ok(1 + 8 + 12),
            VARIANT_BATCH_NODE => Ok(1 + 8),
            other => Err(HydrologyStateError::UnknownCarrierKeyVariant(other)),
        }
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(
            Self::encoded_len(self.variant()).expect("every constructed variant is known"),
        );
        out.push(self.variant());
        match *self {
            Self::Cell(cell) => cell.write_body(&mut out),
            Self::Edge(edge) => {
                edge.low().write_body(&mut out);
                edge.high().write_body(&mut out);
            }
            Self::ExteriorFace(face) => {
                face.cell().write_body(&mut out);
                out.push(face.direction().code());
            }
            Self::ForcingRecord {
                scheduled_tick,
                forcing_id,
            } => {
                out.extend_from_slice(&scheduled_tick.to_be_bytes());
                out.extend_from_slice(&forcing_id.to_be_bytes());
            }
            Self::ResolutionChunk(chunk) => {
                out.extend_from_slice(&chunk.chart.raw().to_be_bytes());
                out.extend_from_slice(&chunk.chunk.x.to_be_bytes());
                out.extend_from_slice(&chunk.chunk.y.to_be_bytes());
                out.extend_from_slice(&chunk.chunk.z.to_be_bytes());
            }
            Self::BatchNode(id) => out.extend_from_slice(&id.to_be_bytes()),
        }
        debug_assert_eq!(
            out.len(),
            Self::encoded_len(self.variant()).expect("every constructed variant is known")
        );
        out
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, HydrologyStateError> {
        let Some(&variant) = bytes.first() else {
            return Err(HydrologyStateError::UnknownCarrierKeyVariant(0));
        };
        let expected = Self::encoded_len(variant)?;
        if bytes.len() != expected {
            return Err(HydrologyStateError::InvalidCarrierKeyLength {
                variant,
                expected,
                actual: bytes.len(),
            });
        }
        match variant {
            VARIANT_CELL => Ok(Self::Cell(HydrologyCellKey::read_body(&bytes[1..23])?)),
            VARIANT_EDGE => {
                let low = HydrologyCellKey::read_body(&bytes[1..23])?;
                let high = HydrologyCellKey::read_body(&bytes[23..45])?;
                // Canonical order is the identity, so accepting the reverse
                // would give one face two names and let a receipt be counted
                // twice under keys that compare unequal.
                if low >= high {
                    return Err(HydrologyStateError::NoncanonicalCarrierKeyOrder);
                }
                Ok(Self::Edge(HydrologyEdgeKey::new(low, high)?))
            }
            VARIANT_EXTERIOR_FACE => {
                let cell = HydrologyCellKey::read_body(&bytes[1..23])?;
                let direction = FaceDirection::from_code(bytes[23])?;
                Ok(Self::ExteriorFace(HydrologyExteriorFaceKey::new(
                    cell, direction,
                )))
            }
            VARIANT_FORCING_RECORD => Ok(Self::ForcingRecord {
                scheduled_tick: u64::from_be_bytes(bytes[1..9].try_into().expect("eight bytes")),
                forcing_id: u64::from_be_bytes(bytes[9..17].try_into().expect("eight bytes")),
            }),
            VARIANT_RESOLUTION_CHUNK => {
                let chart = SpatialChartId::new(u64::from_be_bytes(
                    bytes[1..9].try_into().expect("eight bytes"),
                ));
                let x = i32::from_be_bytes(bytes[9..13].try_into().expect("four bytes"));
                let y = i32::from_be_bytes(bytes[13..17].try_into().expect("four bytes"));
                let z = i32::from_be_bytes(bytes[17..21].try_into().expect("four bytes"));
                Ok(Self::ResolutionChunk(ChartChunkCoord::new(
                    chart,
                    ChunkCoord::new(x, y, z),
                )))
            }
            VARIANT_BATCH_NODE => Ok(Self::BatchNode(u64::from_be_bytes(
                bytes[1..9].try_into().expect("eight bytes"),
            ))),
            other => Err(HydrologyStateError::UnknownCarrierKeyVariant(other)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::{HydraulicFraction, HydraulicSubstrateParts, HydrologyGridMetric};
    use super::*;
    use std::num::{NonZeroU32, NonZeroU64};

    fn chart() -> SpatialChartId {
        SpatialChartId::new(1)
    }

    fn chunk(x: i32, y: i32) -> ChartChunkCoord {
        ChartChunkCoord::new(chart(), ChunkCoord::new(x, y, 0))
    }

    fn cell(x: i32, y: i32, local_x: u8, local_y: u8) -> HydrologyCellKey {
        HydrologyCellKey::from_local(chunk(x, y), local_x, local_y).expect("local cell is in range")
    }

    fn metrics() -> HydrologyGridMetrics {
        HydrologyGridMetrics::new(vec![(
            chart(),
            HydrologyGridMetric::new(
                NonZeroU64::new(1_000).unwrap(),
                NonZeroU64::new(1_000).unwrap(),
                NonZeroU64::new(1_000).unwrap(),
            ),
        )])
        .expect("one chart is a valid registry")
    }

    fn ground() -> HydraulicSubstrateCell {
        HydraulicSubstrateCell::new(HydraulicSubstrateParts {
            surface_capacity: WaterVolume::new(1_000),
            soil_capacity: WaterVolume::new(1_000),
            groundwater_capacity: WaterVolume::new(1_000),
            infiltration_limit_per_tick: WaterVolume::new(10),
            percolation_fraction: HydraulicFraction::new(1, NonZeroU32::new(4).unwrap()).unwrap(),
            specific_yield: HydraulicFraction::new(1, NonZeroU32::new(5).unwrap()).unwrap(),
            aquifer_base_elevation_mm: 0,
            baseflow_threshold: WaterVolume::new(100),
            baseflow_fraction: HydraulicFraction::new(1, NonZeroU32::new(10).unwrap()).unwrap(),
            surface_conductance_mm2_per_tick: 8,
            groundwater_conductance_mm2_per_tick: 4,
        })
        .expect("test substrate is valid")
    }

    fn field(at: ChartChunkCoord, storage: HydrologyCellStorage) -> HydrologyField {
        HydrologyField::from_parts(
            at,
            vec![
                HydrologyCellState::initial(
                    storage,
                    TraceId::new(1),
                    StateFingerprint::new([0; 32])
                );
                SURFACE_CELL_COUNT
            ],
            vec![ground(); SURFACE_CELL_COUNT],
        )
        .expect("a full field is valid")
    }

    // -- lattice -----------------------------------------------------------

    #[test]
    fn ordinals_are_row_major_over_the_terrain_surface() {
        // The same row-major `y * CHUNK_SIZE + x` order `TerrainChunk::cell`
        // uses, so a hydrology cell and the ground under it share an index.
        assert_eq!(cell(0, 0, 0, 0).cell_ordinal(), 0);
        assert_eq!(cell(0, 0, 1, 0).cell_ordinal(), 1);
        assert_eq!(cell(0, 0, 0, 1).cell_ordinal(), u16::from(CHUNK_SIZE));
        assert_eq!(
            cell(0, 0, CHUNK_SIZE - 1, CHUNK_SIZE - 1).cell_ordinal(),
            (SURFACE_CELL_COUNT - 1) as u16
        );
        for ordinal in [0_u16, 1, 33, 1_023] {
            let key = HydrologyCellKey::new(chunk(0, 0), ordinal).unwrap();
            let (x, y) = key.local();
            assert_eq!(
                HydrologyCellKey::from_local(chunk(0, 0), x, y).unwrap(),
                key
            );
        }
    }

    #[test]
    fn an_ordinal_past_the_surface_lattice_is_rejected() {
        assert_eq!(
            HydrologyCellKey::new(chunk(0, 0), SURFACE_CELL_COUNT as u16),
            Err(HydrologyStateError::CellOrdinalOutOfRange {
                ordinal: SURFACE_CELL_COUNT as u16,
                count: SURFACE_CELL_COUNT,
            })
        );
        assert!(HydrologyCellKey::from_local(chunk(0, 0), CHUNK_SIZE, 0).is_err());
        assert!(HydrologyCellKey::from_local(chunk(0, 0), 0, CHUNK_SIZE).is_err());
    }

    #[test]
    fn interior_neighbours_move_one_cell_in_each_direction() {
        let centre = cell(0, 0, 5, 7);
        assert_eq!(centre.neighbor(FaceDirection::NegX), Some(cell(0, 0, 4, 7)));
        assert_eq!(centre.neighbor(FaceDirection::PosX), Some(cell(0, 0, 6, 7)));
        assert_eq!(centre.neighbor(FaceDirection::NegY), Some(cell(0, 0, 5, 6)));
        assert_eq!(centre.neighbor(FaceDirection::PosY), Some(cell(0, 0, 5, 8)));
    }

    #[test]
    fn a_chunk_seam_is_an_ordinary_face_that_preserves_its_orthogonal_coordinate() {
        // Given: cells on both sides of the +X seam between chunks 0 and 1.
        let east_edge = cell(0, 0, CHUNK_SIZE - 1, 9);
        let west_edge = cell(1, 0, 0, 9);

        // Then: each is the other's neighbour, the row is preserved, and the
        // relationship is symmetric — a seam that lost the orthogonal
        // coordinate would silently shear the chart along it.
        assert_eq!(east_edge.neighbor(FaceDirection::PosX), Some(west_edge));
        assert_eq!(west_edge.neighbor(FaceDirection::NegX), Some(east_edge));
        assert_eq!(east_edge.local().1, west_edge.local().1);

        let north_edge = cell(0, 0, 12, CHUNK_SIZE - 1);
        let south_edge = cell(0, 1, 12, 0);
        assert_eq!(north_edge.neighbor(FaceDirection::PosY), Some(south_edge));
        assert_eq!(south_edge.neighbor(FaceDirection::NegY), Some(north_edge));
        assert_eq!(north_edge.local().0, south_edge.local().0);
    }

    #[test]
    fn every_neighbour_relation_is_symmetric_across_the_whole_seam() {
        for row in 0..CHUNK_SIZE {
            let east = cell(0, 0, CHUNK_SIZE - 1, row);
            let west = east.neighbor(FaceDirection::PosX).unwrap();
            assert_eq!(west.neighbor(FaceDirection::NegX), Some(east));
            assert_eq!(west.chunk(), chunk(1, 0));
        }
    }

    #[test]
    fn a_neighbour_whose_chunk_address_does_not_exist_is_absent_not_wrapped() {
        // A chart cannot run past `i32`. Wrapping would connect the two ends of
        // the world through arithmetic nobody modelled.
        let edge_of_representable = HydrologyCellKey::from_local(
            ChartChunkCoord::new(chart(), ChunkCoord::new(i32::MAX, 0, 0)),
            CHUNK_SIZE - 1,
            0,
        )
        .unwrap();
        assert_eq!(edge_of_representable.neighbor(FaceDirection::PosX), None);
        assert!(
            edge_of_representable
                .neighbor(FaceDirection::NegX)
                .is_some()
        );
    }

    #[test]
    fn adjacency_finds_the_direction_and_refuses_distant_pairs() {
        let a = cell(0, 0, 4, 4);
        assert_eq!(
            a.adjacency(cell(0, 0, 5, 4)),
            Some(FaceDirection::PosX),
            "an interior face"
        );
        assert_eq!(
            cell(0, 0, CHUNK_SIZE - 1, 3).adjacency(cell(1, 0, 0, 3)),
            Some(FaceDirection::PosX),
            "a seam face"
        );
        assert_eq!(a.adjacency(cell(0, 0, 6, 4)), None, "two cells apart");
        assert_eq!(a.adjacency(cell(0, 0, 5, 5)), None, "diagonal");
        assert_eq!(a.adjacency(a), None, "itself");
        assert_eq!(
            a.adjacency(
                HydrologyCellKey::from_local(
                    ChartChunkCoord::new(SpatialChartId::new(2), ChunkCoord::new(0, 0, 0)),
                    5,
                    4
                )
                .unwrap()
            ),
            None,
            "another chart"
        );
    }

    #[test]
    fn face_direction_codes_are_the_wire_contract() {
        assert_eq!(
            FaceDirection::ALL.map(FaceDirection::code),
            [0, 1, 2, 3],
            "-X, +X, -Y, +Y"
        );
        for direction in FaceDirection::ALL {
            assert_eq!(FaceDirection::from_code(direction.code()), Ok(direction));
            assert_eq!(direction.opposite().opposite(), direction);
            assert_ne!(direction.opposite(), direction);
        }
        assert_eq!(
            FaceDirection::from_code(4),
            Err(HydrologyStateError::UnknownFaceDirection(4))
        );
    }

    // -- edge keys ---------------------------------------------------------

    #[test]
    fn an_edge_key_is_the_same_whichever_endpoint_names_it() {
        let a = cell(0, 0, 1, 1);
        let b = cell(0, 0, 2, 1);
        let forward = HydrologyEdgeKey::new(a, b).unwrap();
        let reverse = HydrologyEdgeKey::new(b, a).unwrap();
        assert_eq!(forward, reverse);
        assert_eq!(forward.low(), a.min(b));
        assert_eq!(forward.high(), a.max(b));
        assert_eq!(forward.other(a), Some(b));
        assert_eq!(forward.other(b), Some(a));
        assert_eq!(forward.other(cell(0, 0, 9, 9)), None);
        assert!(forward.contains(a));
        assert_eq!(
            HydrologyEdgeKey::new(a, a),
            Err(HydrologyStateError::DegenerateEdge)
        );
    }

    // -- boundaries --------------------------------------------------------

    #[test]
    fn boundary_channels_are_independent_and_bounded() {
        let face = HydrologyExteriorFaceKey::new(cell(0, 0, 0, 0), FaceDirection::NegX);
        let open_surface = HydrologyBoundaryCondition::new(
            FluxBoundary::Open {
                external_head_mm: -500,
                conductance_mm2_per_tick: 32,
            },
            FluxBoundary::NoFlux,
        );
        assert!(open_surface.surface.is_open());
        assert!(!open_surface.groundwater.is_open());
        assert!(!HydrologyBoundaryCondition::CLOSED.surface.is_open());

        let map = HydrologyBoundaryMap::new(vec![(face, open_surface)]).unwrap();
        assert_eq!(map.get(face), Some(open_surface));
        assert_eq!(map.len(), 1);
        assert!(!map.is_empty());
        assert_eq!(
            map.get(HydrologyExteriorFaceKey::new(
                cell(0, 0, 0, 0),
                FaceDirection::PosX
            )),
            None
        );

        assert_eq!(
            HydrologyBoundaryMap::new(vec![(face, open_surface), (face, open_surface)]),
            Err(HydrologyStateError::DuplicateBoundaryFace)
        );
    }

    #[test]
    fn boundary_constitutive_kind_separates_every_parameter() {
        let closed = HydrologyBoundaryCondition::CLOSED;
        let open = HydrologyBoundaryCondition::new(
            FluxBoundary::Open {
                external_head_mm: 10,
                conductance_mm2_per_tick: 20,
            },
            FluxBoundary::NoFlux,
        );
        let other_head = HydrologyBoundaryCondition::new(
            FluxBoundary::Open {
                external_head_mm: 11,
                conductance_mm2_per_tick: 20,
            },
            FluxBoundary::NoFlux,
        );
        let other_conductance = HydrologyBoundaryCondition::new(
            FluxBoundary::Open {
                external_head_mm: 10,
                conductance_mm2_per_tick: 21,
            },
            FluxBoundary::NoFlux,
        );
        let ground_open = HydrologyBoundaryCondition::new(
            FluxBoundary::Open {
                external_head_mm: 10,
                conductance_mm2_per_tick: 20,
            },
            FluxBoundary::Open {
                external_head_mm: 10,
                conductance_mm2_per_tick: 20,
            },
        );
        let kinds = [closed, open, other_head, other_conductance, ground_open]
            .map(|condition| condition.constitutive_kind());
        for (index, left) in kinds.iter().enumerate() {
            for right in &kinds[index + 1..] {
                assert_ne!(left, right, "boundary kinds must not collide");
            }
        }
    }

    // -- fields ------------------------------------------------------------

    #[test]
    fn a_field_must_be_exactly_the_surface_lattice_on_both_arrays() {
        let full = vec![
            HydrologyCellState::initial(
                HydrologyCellStorage::ZERO,
                TraceId::new(1),
                StateFingerprint::new([0; 32])
            );
            SURFACE_CELL_COUNT
        ];
        assert!(
            HydrologyField::from_parts(
                chunk(0, 0),
                full.clone(),
                vec![ground(); SURFACE_CELL_COUNT]
            )
            .is_ok()
        );
        assert_eq!(
            HydrologyField::from_parts(chunk(0, 0), Vec::new(), vec![ground(); SURFACE_CELL_COUNT]),
            Err(HydrologyStateError::InvalidFieldLength {
                expected: SURFACE_CELL_COUNT,
                actual: 0,
            })
        );
        assert_eq!(
            HydrologyField::from_parts(chunk(0, 0), full, vec![ground(); SURFACE_CELL_COUNT - 1]),
            Err(HydrologyStateError::InvalidFieldLength {
                expected: SURFACE_CELL_COUNT,
                actual: SURFACE_CELL_COUNT - 1,
            })
        );
    }

    #[test]
    fn storage_above_its_own_capacity_cannot_be_constructed_or_imported() {
        // No solver path reaches this state, so it can only arrive by
        // construction or by a forged snapshot. Both are refused here.
        for storage in [
            HydrologyCellStorage::new(
                WaterVolume::new(1_001),
                WaterVolume::ZERO,
                WaterVolume::ZERO,
            ),
            HydrologyCellStorage::new(
                WaterVolume::ZERO,
                WaterVolume::new(1_001),
                WaterVolume::ZERO,
            ),
            HydrologyCellStorage::new(
                WaterVolume::ZERO,
                WaterVolume::ZERO,
                WaterVolume::new(1_001),
            ),
        ] {
            assert_eq!(
                HydrologyField::from_parts(
                    chunk(0, 0),
                    vec![
                        HydrologyCellState::initial(
                            storage,
                            TraceId::new(1),
                            StateFingerprint::new([0; 32])
                        );
                        SURFACE_CELL_COUNT
                    ],
                    vec![ground(); SURFACE_CELL_COUNT],
                ),
                Err(HydrologyStateError::StorageExceedsCapacity)
            );
        }
    }

    #[test]
    fn a_field_set_is_ordered_bounded_and_independent_of_input_order() {
        let metrics = metrics();
        let a = field(chunk(0, 0), HydrologyCellStorage::ZERO);
        let b = field(chunk(-1, 0), HydrologyCellStorage::ZERO);

        let forward =
            HydrologyFieldSet::new(vec![a.clone(), b.clone()], &metrics, TraceId::new(1)).unwrap();
        let reverse = HydrologyFieldSet::new(vec![b, a], &metrics, TraceId::new(1)).unwrap();
        assert_eq!(forward, reverse);
        assert_eq!(
            forward.fields().keys().copied().collect::<Vec<_>>(),
            vec![chunk(-1, 0), chunk(0, 0)],
            "canonical chunk order, not insertion order"
        );
        assert_eq!(forward.cell_count(), 2 * SURFACE_CELL_COUNT);
        assert_eq!(forward.batch_sequence(), 0);
        assert_eq!(forward.conservation_last_change(), TraceId::new(1));

        assert_eq!(
            HydrologyFieldSet::new(Vec::new(), &metrics, TraceId::new(1)),
            Err(HydrologyStateError::EmptyFieldSet)
        );
        assert_eq!(
            HydrologyFieldSet::new(
                vec![
                    field(chunk(0, 0), HydrologyCellStorage::ZERO),
                    field(chunk(0, 0), HydrologyCellStorage::ZERO)
                ],
                &metrics,
                TraceId::new(1)
            ),
            Err(HydrologyStateError::DuplicateFieldChunk)
        );
    }

    #[test]
    fn a_field_set_rejects_one_chunk_past_its_bound() {
        let metrics = metrics();
        let at_bound = (0..MAX_HYDROLOGY_CHUNKS)
            .map(|index| field(chunk(index as i32, 0), HydrologyCellStorage::ZERO))
            .collect::<Vec<_>>();
        assert!(HydrologyFieldSet::new(at_bound, &metrics, TraceId::new(1)).is_ok());

        let over = (0..=MAX_HYDROLOGY_CHUNKS)
            .map(|index| field(chunk(index as i32, 0), HydrologyCellStorage::ZERO))
            .collect::<Vec<_>>();
        assert_eq!(
            HydrologyFieldSet::new(over, &metrics, TraceId::new(1)),
            Err(HydrologyStateError::ChunkCountExceeded {
                count: MAX_HYDROLOGY_CHUNKS + 1,
                max: MAX_HYDROLOGY_CHUNKS,
            })
        );
    }

    #[test]
    fn the_chunk_and_cell_bounds_agree_with_the_lattice_size() {
        // These two constants are stated independently in the plan; if they
        // ever disagree, one of them is silently unreachable.
        assert_eq!(
            MAX_HYDROLOGY_CHUNKS * SURFACE_CELL_COUNT,
            MAX_HYDROLOGY_CELLS
        );
    }

    #[test]
    fn a_chunk_in_an_unregistered_chart_is_rejected_rather_than_defaulted() {
        let elsewhere = ChartChunkCoord::new(SpatialChartId::new(99), ChunkCoord::new(0, 0, 0));
        assert_eq!(
            HydrologyFieldSet::new(
                vec![field(elsewhere, HydrologyCellStorage::ZERO)],
                &metrics(),
                TraceId::new(1)
            ),
            Err(HydrologyStateError::FieldChartWithoutMetric)
        );
    }

    #[test]
    fn field_set_totals_sum_every_bucket_of_every_cell_exactly() {
        let storage = HydrologyCellStorage::new(
            WaterVolume::new(3),
            WaterVolume::new(5),
            WaterVolume::new(7),
        );
        let set = HydrologyFieldSet::new(
            vec![field(chunk(0, 0), storage), field(chunk(1, 0), storage)],
            &metrics(),
            TraceId::new(1),
        )
        .unwrap();
        assert_eq!(
            set.total_storage().unwrap().get(),
            2 * SURFACE_CELL_COUNT as i128 * 15
        );
        assert_eq!(
            set.cell(cell(0, 0, 3, 4)).unwrap().storage(),
            storage,
            "a resident cell resolves through the set"
        );
        assert!(set.ground(cell(0, 0, 3, 4)).is_some());
        assert!(set.is_resident(cell(1, 0, 0, 0)));
        assert!(!set.is_resident(cell(5, 5, 0, 0)));
        assert_eq!(set.cell(cell(5, 5, 0, 0)), None);
    }

    // -- conveyance --------------------------------------------------------

    fn edge(
        a: HydrologyCellKey,
        b: HydrologyCellKey,
        outlet: HydrologyCellKey,
        storage: u64,
        capacity: u64,
    ) -> Result<HydrologyConveyanceEdge, HydrologyStateError> {
        HydrologyConveyanceEdge::new(
            HydrologyEdgeKey::new(a, b)?,
            outlet,
            WaterVolume::new(storage),
            WaterVolume::new(capacity),
            HydraulicFraction::new(1, NonZeroU32::new(2).unwrap())?,
            WaterVolume::new(50),
            TraceId::new(1),
            WaterVolume::ZERO,
        )
    }

    #[test]
    fn a_conveyance_edge_must_be_a_real_face_with_a_real_outlet() {
        let a = cell(0, 0, 1, 1);
        let b = cell(0, 0, 2, 1);
        let distant = cell(0, 0, 8, 8);

        let valid = edge(a, b, b, 10, 100).unwrap();
        assert_eq!(valid.source(), a);
        assert_eq!(valid.outlet(), b);
        assert_eq!(valid.remaining_capacity(), WaterVolume::new(90));

        assert_eq!(
            edge(a, b, distant, 0, 100),
            Err(HydrologyStateError::OutletNotAnEndpoint)
        );
        assert_eq!(
            edge(a, distant, distant, 0, 100),
            Err(HydrologyStateError::NonAdjacentEdge)
        );
        assert_eq!(
            edge(a, b, b, 101, 100),
            Err(HydrologyStateError::StorageExceedsCapacity)
        );
    }

    #[test]
    fn a_seam_face_carries_a_conveyance_edge_exactly_like_an_interior_one() {
        let east = cell(0, 0, CHUNK_SIZE - 1, 4);
        let west = cell(1, 0, 0, 4);
        let seam = edge(east, west, west, 5, 50).unwrap();
        assert_eq!(seam.source(), east);
        assert_eq!(seam.outlet(), west);
    }

    #[test]
    fn a_face_carries_one_edge_and_a_cell_one_outgoing_edge() {
        let a = cell(0, 0, 1, 1);
        let b = cell(0, 0, 2, 1);
        let c = cell(0, 0, 1, 2);

        let graph = HydrologyConveyanceGraph::new(vec![edge(a, b, b, 4, 100).unwrap()]).unwrap();
        assert_eq!(graph.len(), 1);
        assert!(!graph.is_empty());
        assert_eq!(graph.outgoing(a).map(|e| e.outlet()), Some(b));
        assert_eq!(
            graph.outgoing(b).map(|e| e.outlet()),
            None,
            "the outlet is not itself a source"
        );
        assert_eq!(graph.total_storage().unwrap().get(), 4);

        // The same face named from the other side is the same face.
        assert_eq!(
            HydrologyConveyanceGraph::new(vec![
                edge(a, b, b, 0, 100).unwrap(),
                edge(b, a, a, 0, 100).unwrap(),
            ]),
            Err(HydrologyStateError::DuplicateEdgeFace)
        );

        // Two different faces, both leaving `a`: baseflow and release both ask
        // for "the" outgoing edge, so two would make that order-dependent.
        assert_eq!(
            HydrologyConveyanceGraph::new(vec![
                edge(a, b, b, 0, 100).unwrap(),
                edge(a, c, c, 0, 100).unwrap(),
            ]),
            Err(HydrologyStateError::MultipleOutgoingEdges)
        );
    }

    /// `count` edges with pairwise distinct faces and distinct sources, laid
    /// out as disjoint horizontal pairs so neither graph invariant trips before
    /// the bound under test does.
    fn distinct_edges(count: usize) -> Vec<HydrologyConveyanceEdge> {
        let per_chunk = (CHUNK_SIZE as usize / 2) * CHUNK_SIZE as usize;
        (0..count)
            .map(|index| {
                let chunk_index = (index / per_chunk) as i32;
                let within = index % per_chunk;
                let row = (within / (CHUNK_SIZE as usize / 2)) as u8;
                let pair = (within % (CHUNK_SIZE as usize / 2)) as u8;
                let left = cell(chunk_index, 0, pair * 2, row);
                let right = cell(chunk_index, 0, pair * 2 + 1, row);
                edge(left, right, right, 0, 1).expect("a horizontal pair is a valid edge")
            })
            .collect()
    }

    #[test]
    fn the_conveyance_graph_rejects_one_edge_past_its_bound() {
        // The count is checked before the maps are built, so `limit + 1` is
        // refused rather than inserted and then noticed.
        assert!(HydrologyConveyanceGraph::new(distinct_edges(MAX_HYDROLOGY_EDGES)).is_ok());
        assert_eq!(
            HydrologyConveyanceGraph::new(distinct_edges(MAX_HYDROLOGY_EDGES + 1)),
            Err(HydrologyStateError::EdgeCountExceeded {
                count: MAX_HYDROLOGY_EDGES + 1,
                max: MAX_HYDROLOGY_EDGES,
            })
        );
    }

    #[test]
    fn the_boundary_map_rejects_one_record_past_its_bound() {
        let faces = |count: usize| {
            (0..count)
                .map(|index| {
                    let chunk_index = (index / (SURFACE_CELL_COUNT * 4)) as i32;
                    let within = index % (SURFACE_CELL_COUNT * 4);
                    let ordinal = (within / 4) as u16;
                    let direction = FaceDirection::from_code((within % 4) as u8)
                        .expect("four directions cycle");
                    (
                        HydrologyExteriorFaceKey::new(
                            HydrologyCellKey::new(chunk(chunk_index, 0), ordinal)
                                .expect("ordinal is in range"),
                            direction,
                        ),
                        HydrologyBoundaryCondition::CLOSED,
                    )
                })
                .collect::<Vec<_>>()
        };
        assert!(HydrologyBoundaryMap::new(faces(MAX_HYDROLOGY_BOUNDARY_RECORDS)).is_ok());
        assert_eq!(
            HydrologyBoundaryMap::new(faces(MAX_HYDROLOGY_BOUNDARY_RECORDS + 1)),
            Err(HydrologyStateError::BoundaryCountExceeded {
                count: MAX_HYDROLOGY_BOUNDARY_RECORDS + 1,
                max: MAX_HYDROLOGY_BOUNDARY_RECORDS,
            })
        );
    }

    #[test]
    fn conveyance_graph_construction_is_independent_of_input_order() {
        let first = edge(cell(0, 0, 1, 1), cell(0, 0, 2, 1), cell(0, 0, 2, 1), 3, 10).unwrap();
        let second = edge(cell(0, 0, 5, 5), cell(0, 0, 6, 5), cell(0, 0, 6, 5), 7, 20).unwrap();
        assert_eq!(
            HydrologyConveyanceGraph::new(vec![first, second]).unwrap(),
            HydrologyConveyanceGraph::new(vec![second, first]).unwrap()
        );
    }

    // -- residency and resolution -----------------------------------------

    #[test]
    fn active_chunks_must_be_resident() {
        let resident: BTreeSet<_> = [chunk(0, 0), chunk(1, 0)].into_iter().collect();
        let active: BTreeSet<_> = [chunk(0, 0)].into_iter().collect();
        let region = HydrologyActiveRegion::new(active.clone(), resident.clone()).unwrap();
        assert_eq!(region.active_chunks(), &active);
        assert_eq!(region.resident_chunks(), &resident);

        let stray: BTreeSet<_> = [chunk(9, 9)].into_iter().collect();
        assert_eq!(
            HydrologyActiveRegion::new(stray, resident),
            Err(HydrologyStateError::ActiveChunkNotResident)
        );
    }

    #[test]
    fn resolution_levels_are_bounded_and_map_to_block_edges() {
        for level in 0..=MAX_HYDROLOGY_RESOLUTION_LEVEL {
            let state = HydrologyResolutionState::new(level, TraceId::new(2)).unwrap();
            assert_eq!(state.level(), level);
            assert_eq!(state.last_change(), TraceId::new(2));
            assert_eq!(state.block_edge(), 1_u32 << level);
        }
        assert_eq!(
            HydrologyResolutionState::new(MAX_HYDROLOGY_RESOLUTION_LEVEL + 1, TraceId::new(2)),
            Err(HydrologyStateError::ResolutionLevelExceeded {
                level: MAX_HYDROLOGY_RESOLUTION_LEVEL + 1,
                max: MAX_HYDROLOGY_RESOLUTION_LEVEL,
            })
        );
    }

    // -- carrier keys ------------------------------------------------------

    fn keys() -> Vec<(HydrologyCarrierKey, usize)> {
        vec![
            (HydrologyCarrierKey::Cell(cell(-3, 7, 11, 13)), 23),
            (
                HydrologyCarrierKey::Edge(
                    HydrologyEdgeKey::new(cell(0, 0, 1, 1), cell(0, 0, 2, 1)).unwrap(),
                ),
                45,
            ),
            (
                HydrologyCarrierKey::ExteriorFace(HydrologyExteriorFaceKey::new(
                    cell(2, -5, 0, 31),
                    FaceDirection::PosY,
                )),
                24,
            ),
            (
                HydrologyCarrierKey::ForcingRecord {
                    scheduled_tick: u64::MAX,
                    forcing_id: 7,
                },
                17,
            ),
            (HydrologyCarrierKey::ResolutionChunk(chunk(-1, 4)), 21),
            (HydrologyCarrierKey::BatchNode(u64::MAX), 9),
        ]
    }

    #[test]
    fn every_carrier_key_variant_round_trips_at_its_exact_declared_length() {
        assert_eq!(HYDROLOGY_CARRIER_KEY_VERSION, 1);
        let cases = keys();
        assert_eq!(cases.len(), 6, "every variant must be covered");
        for (key, length) in cases {
            let encoded = key.encode();
            assert_eq!(encoded.len(), length, "{key:?}");
            assert_eq!(
                HydrologyCarrierKey::encoded_len(key.variant()),
                Ok(length),
                "{key:?}"
            );
            assert_eq!(HydrologyCarrierKey::decode(&encoded), Ok(key));
        }
    }

    #[test]
    fn carrier_keys_are_big_endian_so_bytes_order_as_values_do() {
        // The aggregation tree's ready-set tie-break is byte order, so a
        // little-endian field would order proposal keys differently from the
        // values they name and make the committed DAG shape surprising.
        let low = HydrologyCarrierKey::BatchNode(1).encode();
        let high = HydrologyCarrierKey::BatchNode(2).encode();
        assert!(low < high);
        assert_eq!(&low[1..], &[0, 0, 0, 0, 0, 0, 0, 1]);

        let early = HydrologyCarrierKey::ForcingRecord {
            scheduled_tick: 1,
            forcing_id: 9,
        }
        .encode();
        let late = HydrologyCarrierKey::ForcingRecord {
            scheduled_tick: 2,
            forcing_id: 0,
        }
        .encode();
        assert!(early < late, "tick must dominate id in byte order");
    }

    #[test]
    fn a_negative_chunk_coordinate_survives_encoding() {
        let key = HydrologyCarrierKey::Cell(cell(-1, -2_000_000, 3, 4));
        assert_eq!(HydrologyCarrierKey::decode(&key.encode()), Ok(key));
    }

    #[test]
    fn carrier_key_decoding_rejects_wrong_lengths_and_unknown_variants() {
        for (key, length) in keys() {
            let encoded = key.encode();
            let mut short = encoded.clone();
            short.pop();
            assert_eq!(
                HydrologyCarrierKey::decode(&short),
                Err(HydrologyStateError::InvalidCarrierKeyLength {
                    variant: key.variant(),
                    expected: length,
                    actual: length - 1,
                })
            );
            let mut long = encoded;
            long.push(0);
            assert_eq!(
                HydrologyCarrierKey::decode(&long),
                Err(HydrologyStateError::InvalidCarrierKeyLength {
                    variant: key.variant(),
                    expected: length,
                    actual: length + 1,
                }),
                "trailing bytes are not ignored"
            );
        }

        assert_eq!(
            HydrologyCarrierKey::decode(&[]),
            Err(HydrologyStateError::UnknownCarrierKeyVariant(0))
        );
        for variant in [0x00_u8, 0x07, 0xff] {
            assert_eq!(
                HydrologyCarrierKey::decode(&[variant]),
                Err(HydrologyStateError::UnknownCarrierKeyVariant(variant))
            );
        }
    }

    #[test]
    fn a_reversed_edge_carrier_key_is_rejected_rather_than_silently_canonicalised() {
        // Accepting the reverse would give one face two byte identities, and a
        // receipt keyed on bytes would then be counted under both.
        let low = cell(0, 0, 1, 1);
        let high = cell(0, 0, 2, 1);
        let mut reversed = vec![VARIANT_EDGE];
        high.write_body(&mut reversed);
        low.write_body(&mut reversed);
        assert_eq!(
            HydrologyCarrierKey::decode(&reversed),
            Err(HydrologyStateError::NoncanonicalCarrierKeyOrder)
        );

        let mut degenerate = vec![VARIANT_EDGE];
        low.write_body(&mut degenerate);
        low.write_body(&mut degenerate);
        assert_eq!(
            HydrologyCarrierKey::decode(&degenerate),
            Err(HydrologyStateError::NoncanonicalCarrierKeyOrder)
        );
    }

    #[test]
    fn a_carrier_key_carrying_an_out_of_range_ordinal_is_rejected() {
        let mut bytes = vec![VARIANT_CELL];
        cell(0, 0, 0, 0).write_body(&mut bytes);
        let last = bytes.len() - 2;
        bytes[last..].copy_from_slice(&(SURFACE_CELL_COUNT as u16).to_be_bytes());
        assert_eq!(
            HydrologyCarrierKey::decode(&bytes),
            Err(HydrologyStateError::CellOrdinalOutOfRange {
                ordinal: SURFACE_CELL_COUNT as u16,
                count: SURFACE_CELL_COUNT,
            })
        );
    }

    #[test]
    fn a_carrier_key_carrying_an_unknown_face_direction_is_rejected() {
        let mut bytes = vec![VARIANT_EXTERIOR_FACE];
        cell(0, 0, 0, 0).write_body(&mut bytes);
        bytes.push(4);
        assert_eq!(
            HydrologyCarrierKey::decode(&bytes),
            Err(HydrologyStateError::UnknownFaceDirection(4))
        );
    }
}
