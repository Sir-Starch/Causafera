//! Conservative hydrology resolution: block addressing, constitutive grouping,
//! and the capped largest-remainder reducer that returns coarse deltas to fine
//! members.
//!
//! Fine state stays canonical at every level. A coarse level changes how much
//! work one tick does, never what exists: nothing is deleted on demotion and
//! nothing is synthesised on promotion. See `plans/hydrology.md` §9.

use std::collections::BTreeMap;

use causafera_geography::{
    HydraulicSubstrateCell, HydraulicSubstrateKey, HydrologyBoundaryCondition,
    HydrologyBoundaryMap, HydrologyCellKey, HydrologyExteriorFaceKey, HydrologyFieldSet,
    HydrologyGridMetric, HydrologyResolutionState,
};
use causafera_types::{
    CHUNK_SIZE, ChartChunkCoord, SpatialChartId, WaterAccumulator, WaterVolumeError,
};

use super::HydrologyError;

/// One coarse unit: an ordered chart-grid block of cells at one level.
///
/// Membership is computed from global terrain-cell coordinates, never from chunk
/// extent — a block may straddle a chunk seam and a chunk may hold many blocks,
/// because chunks are addressing and a block is a unit of work (INV-043).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HydrologyBlockKey {
    chart: SpatialChartId,
    /// The lattice this block belongs to. The hydrology surface is
    /// two-dimensional, so blocks group in x and y and never across `z`.
    plane: i32,
    level: u8,
    block_x: i64,
    block_y: i64,
}

impl HydrologyBlockKey {
    /// The block one cell belongs to at one level.
    pub fn of(cell: HydrologyCellKey, level: u8) -> Result<Self, HydrologyError> {
        let edge = i64::from(block_edge(level));
        let (global_x, global_y) = global_cell(cell);
        Ok(Self {
            chart: cell.chart(),
            plane: cell.chunk().chunk.z,
            level,
            block_x: global_x.div_euclid(edge),
            block_y: global_y.div_euclid(edge),
        })
    }

    pub const fn level(self) -> u8 {
        self.level
    }

    pub const fn block_x(self) -> i64 {
        self.block_x
    }

    pub const fn block_y(self) -> i64 {
        self.block_y
    }

    /// Canonical bytes, for the coarse-input and coarse-process fingerprints.
    pub fn encode(self) -> Vec<u8> {
        let mut out = Vec::with_capacity(8 + 4 + 1 + 8 + 8);
        out.extend_from_slice(&self.chart.raw().to_be_bytes());
        out.extend_from_slice(&self.plane.to_be_bytes());
        out.push(self.level);
        out.extend_from_slice(&self.block_x.to_be_bytes());
        out.extend_from_slice(&self.block_y.to_be_bytes());
        out
    }
}

/// Cells along one edge of a block at `level`, which is `2^min(level, 4)`.
pub const fn block_edge(level: u8) -> u32 {
    let capped = if level > 4 { 4 } else { level };
    1_u32 << capped
}

/// One cell's position in its chart's global terrain-cell grid.
fn global_cell(cell: HydrologyCellKey) -> (i64, i64) {
    let (x, y) = cell.local();
    let span = i64::from(CHUNK_SIZE);
    (
        i64::from(cell.chunk().chunk.x) * span + i64::from(x),
        i64::from(cell.chunk().chunk.y) * span + i64::from(y),
    )
}

/// The exact `(metric, substrate, boundary-kind)` identity two cells must share
/// to be evaluated as one coarse member.
///
/// Exact, not similar. Aggregating cells that differ in any parameter would
/// invent an averaged cell that none of them is, and running a process against
/// that average would then allocate results back onto cells it was never
/// computed for. Heterogeneous ground therefore produces more groups and less
/// work reduction, which is the honest outcome rather than a hidden
/// approximation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HydrologyConstitutiveKey {
    metric: [u8; 26],
    substrate: HydraulicSubstrateKey,
    faces: [FaceKind; 4],
}

/// One face's contribution to a constitutive identity.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct FaceKind {
    /// `0` interior, `1` exterior with a boundary record.
    presence: u8,
    surface_kind: u8,
    surface_head_mm: i64,
    surface_conductance: u64,
    groundwater_kind: u8,
    groundwater_head_mm: i64,
    groundwater_conductance: u64,
}

impl FaceKind {
    const INTERIOR: Self = Self {
        presence: 0,
        surface_kind: 0,
        surface_head_mm: 0,
        surface_conductance: 0,
        groundwater_kind: 0,
        groundwater_head_mm: 0,
        groundwater_conductance: 0,
    };

    fn exterior(condition: &HydrologyBoundaryCondition) -> Self {
        let (
            surface_kind,
            surface_head_mm,
            surface_conductance,
            groundwater_kind,
            groundwater_head_mm,
            groundwater_conductance,
        ) = condition.constitutive_kind();
        Self {
            presence: 1,
            surface_kind,
            surface_head_mm,
            surface_conductance,
            groundwater_kind,
            groundwater_head_mm,
            groundwater_conductance,
        }
    }

    fn write(self, out: &mut Vec<u8>) {
        out.push(self.presence);
        out.push(self.surface_kind);
        out.extend_from_slice(&(self.surface_head_mm as u64 ^ (1 << 63)).to_be_bytes());
        out.extend_from_slice(&self.surface_conductance.to_be_bytes());
        out.push(self.groundwater_kind);
        out.extend_from_slice(&(self.groundwater_head_mm as u64 ^ (1 << 63)).to_be_bytes());
        out.extend_from_slice(&self.groundwater_conductance.to_be_bytes());
    }
}

impl HydrologyConstitutiveKey {
    /// The identity of one cell, given the state its faces resolve against.
    pub fn of(
        cell: HydrologyCellKey,
        metric: HydrologyGridMetric,
        ground: &HydraulicSubstrateCell,
        state: &HydrologyFieldSet,
        boundaries: &HydrologyBoundaryMap,
    ) -> Result<Self, HydrologyError> {
        let mut faces = [FaceKind::INTERIOR; 4];
        for direction in causafera_geography::FaceDirection::ALL {
            let resident = cell
                .neighbor(direction)
                .is_some_and(|neighbor| state.is_resident(neighbor));
            if resident {
                continue;
            }
            let condition = boundaries
                .get(HydrologyExteriorFaceKey::new(cell, direction))
                .ok_or(HydrologyError::UnspecifiedBoundaryFace)?;
            faces[usize::from(direction.code())] = FaceKind::exterior(&condition);
        }
        Ok(Self {
            metric: encode_metric(metric),
            substrate: ground.constitutive_key(),
            faces,
        })
    }

    /// Canonical bytes, for the coarse-input and coarse-process fingerprints.
    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(26 + 88 + 4 * 36);
        out.extend_from_slice(&self.metric);
        out.extend_from_slice(self.substrate.bytes());
        for face in self.faces {
            face.write(&mut out);
        }
        out
    }
}

fn encode_metric(metric: HydrologyGridMetric) -> [u8; 26] {
    let mut out = [0_u8; 26];
    out[0..2].copy_from_slice(&metric.schema_version().to_be_bytes());
    out[2..10].copy_from_slice(&metric.cell_area_mm2().get().to_be_bytes());
    out[10..18].copy_from_slice(&metric.orthogonal_edge_length_mm().get().to_be_bytes());
    out[18..26].copy_from_slice(&metric.timestep_millis().get().to_be_bytes());
    out
}

/// Which cells one tick evaluates coarsely, and how they group.
///
/// Level zero keeps the fine path: a one-cell block would produce the same state
/// through a different causal DAG, and one representation of one tick is the
/// point of keeping fine state canonical.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HydrologyResolutionPlan {
    levels: BTreeMap<ChartChunkCoord, u8>,
    groups: BTreeMap<(HydrologyBlockKey, HydrologyConstitutiveKey), Vec<HydrologyCellKey>>,
    /// The reverse index. Searching `groups` for a cell would be linear in the
    /// number of groups on a path that runs once per targeted cell.
    membership: BTreeMap<HydrologyCellKey, (HydrologyBlockKey, HydrologyConstitutiveKey)>,
}

impl HydrologyResolutionPlan {
    /// Validate the per-chunk levels and partition every coarse cell.
    ///
    /// A resident chunk with no entry, or an entry above the policy's maximum,
    /// rejects the tick rather than being clamped: a clamp would silently
    /// evaluate a world at a detail nobody asked for. Extra entries for chunks
    /// hydrology does not hold are ignored, because the runtime's resolution
    /// field covers more than one domain.
    pub fn build(
        state: &HydrologyFieldSet,
        boundaries: &HydrologyBoundaryMap,
        metrics: &causafera_geography::HydrologyGridMetrics,
        resolution: &BTreeMap<ChartChunkCoord, HydrologyResolutionState>,
        policy: HydrologyResolutionPolicy,
    ) -> Result<Self, HydrologyError> {
        let mut levels = BTreeMap::new();
        for chunk in state.fields().keys() {
            let level = if policy.enabled {
                let entry = resolution
                    .get(chunk)
                    .ok_or(HydrologyError::ResolutionEntryMissing)?;
                if entry.level() > policy.max_level {
                    return Err(HydrologyError::ResolutionLevelAbovePolicy {
                        level: entry.level(),
                        max: policy.max_level,
                    });
                }
                entry.level()
            } else {
                0
            };
            levels.insert(*chunk, level);
        }

        let mut groups: BTreeMap<
            (HydrologyBlockKey, HydrologyConstitutiveKey),
            Vec<HydrologyCellKey>,
        > = BTreeMap::new();
        let mut membership = BTreeMap::new();
        for (chunk, field) in state.fields() {
            let level = levels[chunk];
            if level == 0 {
                continue;
            }
            let metric = metrics.get(chunk.chart)?;
            for (ordinal, ground) in field.substrate().iter().enumerate() {
                let cell = HydrologyCellKey::new(*chunk, ordinal as u16)?;
                let block = HydrologyBlockKey::of(cell, level)?;
                let constitutive =
                    HydrologyConstitutiveKey::of(cell, metric, ground, state, boundaries)?;
                groups.entry((block, constitutive)).or_default().push(cell);
                membership.insert(cell, (block, constitutive));
            }
        }
        // Members arrive in canonical cell order because the field set and the
        // ordinal walk are both ordered, and every weight, ceiling, and grant
        // downstream is positional.
        for members in groups.values() {
            debug_assert!(members.windows(2).all(|pair| pair[0] < pair[1]));
        }
        Ok(Self {
            levels,
            groups,
            membership,
        })
    }

    /// The block and constitutive group one coarse cell belongs to.
    pub fn group_of(
        &self,
        cell: HydrologyCellKey,
    ) -> Option<(HydrologyBlockKey, HydrologyConstitutiveKey)> {
        self.membership.get(&cell).copied()
    }

    /// The level one chunk is evaluated at this tick.
    pub fn level(&self, chunk: ChartChunkCoord) -> u8 {
        self.levels.get(&chunk).copied().unwrap_or(0)
    }

    /// Whether one cell's vertical processes run coarsely.
    pub fn is_coarse(&self, cell: HydrologyCellKey) -> bool {
        self.level(cell.chunk()) > 0
    }

    /// Whether a face between two resident cells is internal to one block.
    ///
    /// An internal face is not evaluated at coarse resolution. Every other face —
    /// including every face touching a level-zero cell, and every face between
    /// two blocks — stays authoritative and is evaluated from its frozen fine
    /// endpoints, so heterogeneous boundary conductance is never averaged away.
    pub fn is_internal_face(
        &self,
        a: HydrologyCellKey,
        b: HydrologyCellKey,
    ) -> Result<bool, HydrologyError> {
        let level = self.level(a.chunk());
        if level == 0 || level != self.level(b.chunk()) {
            return Ok(false);
        }
        Ok(HydrologyBlockKey::of(a, level)? == HydrologyBlockKey::of(b, level)?)
    }

    pub fn groups(
        &self,
    ) -> &BTreeMap<(HydrologyBlockKey, HydrologyConstitutiveKey), Vec<HydrologyCellKey>> {
        &self.groups
    }

    /// How many constitutive groups this tick evaluates vertical processes over.
    pub fn group_count(&self) -> usize {
        self.groups.len()
    }

    pub fn is_all_fine(&self) -> bool {
        self.groups.is_empty()
    }
}

/// Whether hydrology resolution is active, and how deep it may go.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HydrologyResolutionPolicy {
    pub schema_version: u16,
    pub enabled: bool,
    pub max_level: u8,
}

impl HydrologyResolutionPolicy {
    pub const SCHEMA_VERSION: u16 = 1;

    pub const DISABLED: Self = Self {
        schema_version: Self::SCHEMA_VERSION,
        enabled: false,
        max_level: 0,
    };

    pub fn enabled(max_level: u8) -> Result<Self, HydrologyError> {
        if max_level > causafera_geography::MAX_HYDROLOGY_RESOLUTION_LEVEL {
            return Err(HydrologyError::ResolutionLevelAbovePolicy {
                level: max_level,
                max: causafera_geography::MAX_HYDROLOGY_RESOLUTION_LEVEL,
            });
        }
        Ok(Self {
            schema_version: Self::SCHEMA_VERSION,
            enabled: true,
            max_level,
        })
    }
}

impl Default for HydrologyResolutionPolicy {
    fn default() -> Self {
        Self::DISABLED
    }
}

// ---------------------------------------------------------------------------
// The capped largest-remainder reducer
// ---------------------------------------------------------------------------

/// One member's share of a coarse group delta.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CappedShare {
    pub weight: i128,
    pub ceiling: i128,
    pub granted: i128,
}

/// Return a coarse total to fine members, respecting every member's ceiling.
///
/// `weights` and `ceilings` are positional and already in canonical cell-key
/// order, because the tie-break for equal remainders is ascending cell key.
///
/// The grants sum to exactly `total`. Proportional rounding alone cannot do that
/// once ceilings are involved: a member that rounds above its own room has to
/// hand the excess to someone, and rounds repeat until nobody caps so the excess
/// lands on members that can actually hold it. `total > sum(ceilings)` is an
/// internal error rather than a spill, because the caller is required to have
/// already taken `min(candidate, sum(ceilings))`.
pub fn allocate_capped(
    total: i128,
    weights: &[i128],
    ceilings: &[i128],
) -> Result<Vec<CappedShare>, HydrologyError> {
    if weights.len() != ceilings.len() {
        return Err(HydrologyError::ResolutionShapeMismatch);
    }
    for value in weights.iter().chain(ceilings) {
        if *value < 0 {
            return Err(WaterVolumeError::Underflow.into());
        }
    }
    if total < 0 {
        return Err(WaterVolumeError::Underflow.into());
    }

    let mut room = WaterAccumulator::ZERO;
    for ceiling in ceilings {
        room = room.add(*ceiling)?;
    }
    if total > room.get() {
        return Err(HydrologyError::AllocationExceedsCeilings);
    }

    let mut shares: Vec<CappedShare> = weights
        .iter()
        .zip(ceilings)
        .map(|(weight, ceiling)| CappedShare {
            weight: *weight,
            ceiling: *ceiling,
            granted: 0,
        })
        .collect();
    let mut remaining = total;

    // At most one round per member can cap, because a capped member is full and
    // drops out; the round that caps nobody terminates the loop.
    for _ in 0..=shares.len() {
        if remaining == 0 {
            return Ok(shares);
        }
        let eligible: Vec<usize> = (0..shares.len())
            .filter(|&index| {
                shares[index].weight > 0 && shares[index].granted < shares[index].ceiling
            })
            .collect();
        if eligible.is_empty() {
            // Positive total, and every member that could take a share by weight
            // is full. Room may exist on zero-weight members, but handing them
            // water no process asked them to receive would be inventing a
            // destination.
            return Err(HydrologyError::UnallocatableTotal);
        }
        let mut weight_sum = WaterAccumulator::ZERO;
        for &index in &eligible {
            weight_sum = weight_sum.add(shares[index].weight)?;
        }
        let weight_sum = weight_sum.get();

        let before = remaining;
        let mut capped = false;
        for &index in &eligible {
            let product = causafera_types::checked_water_mul(before, shares[index].weight)?;
            let base = causafera_types::checked_water_div_floor(product, weight_sum)?;
            let room = shares[index].ceiling - shares[index].granted;
            if base >= room {
                shares[index].granted = shares[index].ceiling;
                remaining -= room;
                capped = true;
            } else {
                shares[index].granted += base;
                remaining -= base;
            }
        }
        if capped {
            continue;
        }
        // Nobody capped, so every eligible member has at least one unit of room
        // left and the shortfall is smaller than the number of members. One unit
        // each, by descending remainder then ascending cell key.
        let mut order = eligible.clone();
        order.sort_by(|&left, &right| {
            let left_remainder = causafera_types::checked_water_rem_floor(
                causafera_types::checked_water_mul(before, shares[left].weight)
                    .unwrap_or(i128::MAX),
                weight_sum,
            )
            .unwrap_or(0);
            let right_remainder = causafera_types::checked_water_rem_floor(
                causafera_types::checked_water_mul(before, shares[right].weight)
                    .unwrap_or(i128::MAX),
                weight_sum,
            )
            .unwrap_or(0);
            right_remainder
                .cmp(&left_remainder)
                .then_with(|| left.cmp(&right))
        });
        for index in order {
            if remaining == 0 {
                break;
            }
            debug_assert!(shares[index].granted < shares[index].ceiling);
            shares[index].granted += 1;
            remaining -= 1;
        }
        break;
    }

    if remaining != 0 {
        return Err(HydrologyError::AllocationExceedsCeilings);
    }
    debug_assert_eq!(
        shares.iter().map(|share| share.granted).sum::<i128>(),
        total
    );
    Ok(shares)
}

/// The largest total the reducer can actually place: `min(candidate, sum of the
/// ceilings of members with a positive weight)`.
///
/// This is where the plan's quantisation case is handled: two one-unit soil cells
/// at a percolation fraction of one half give an aggregate candidate of one while
/// every fine ceiling rounds to zero. Taking the minimum means the group moves
/// nothing rather than moving a unit no member can receive.
///
/// Only positive-weight members count. `plans/hydrology.md` §9 writes
/// `sum(member_ceilings)`, but weight zero means the process never addressed that
/// member — a cell no record rained on, or none asked evapotranspiration of — and
/// the reducer refuses to hand it water. Counting its room would produce a total
/// the reducer then cannot place, which contradicts the same section's promise
/// that "the reducer never receives an unallocatable ordinary candidate". See the
/// Decision log.
pub fn clamp_to_allocatable(
    candidate: i128,
    weights: &[i128],
    ceilings: &[i128],
) -> Result<i128, HydrologyError> {
    if weights.len() != ceilings.len() {
        return Err(HydrologyError::ResolutionShapeMismatch);
    }
    let mut room = WaterAccumulator::ZERO;
    for (weight, ceiling) in weights.iter().zip(ceilings) {
        if *weight > 0 {
            room = room.add(*ceiling)?;
        }
    }
    Ok(candidate.min(room.get()))
}

// ---------------------------------------------------------------------------
// Promotion and demotion
// ---------------------------------------------------------------------------

/// The record and event one chunk's level change commits.
///
/// Committed in `Phase::Resolution`, so it applies from the next Physics tick.
/// The prior anchor is the cause and the new trace becomes the anchor, which is
/// what keeps a level change as inspectable as any other state change — and the
/// only state it transitions is the level itself, because promotion reactivates
/// retained fine state and never synthesises detail.
pub fn representation_change(
    chunk: ChartChunkCoord,
    current: HydrologyResolutionState,
    to_level: u8,
    policy: HydrologyResolutionPolicy,
) -> Result<
    (
        super::HydrologyRepresentationChange,
        super::HydrologyEventPlan,
    ),
    HydrologyError,
> {
    if !policy.enabled && to_level != 0 {
        return Err(HydrologyError::ResolutionLevelAbovePolicy {
            level: to_level,
            max: 0,
        });
    }
    if to_level > policy.max_level {
        return Err(HydrologyError::ResolutionLevelAbovePolicy {
            level: to_level,
            max: policy.max_level,
        });
    }
    if to_level == current.level() {
        return Err(HydrologyError::ResolutionUnchanged);
    }
    // Validated here so a caller cannot construct a level the state type would
    // refuse, which would leave the event committed and the state unwritable.
    HydrologyResolutionState::new(to_level, current.last_change())?;

    let before = super::resolution_fingerprint(chunk, current.level());
    let after = super::resolution_fingerprint(chunk, to_level);
    let change = super::HydrologyRepresentationChange {
        chunk,
        from_level: current.level(),
        to_level,
        prior_change: current.last_change(),
        before,
        after,
    };
    let event = super::HydrologyEventPlan {
        key: causafera_core::CausalEventProposalKey::new(
            super::substage::REPRESENTATION,
            super::process::REPRESENTATION,
            &causafera_geography::HydrologyCarrierKey::ResolutionChunk(chunk).encode(),
            0,
        )?,
        kind: super::HydrologyEventKind::Representation,
        coarse_process: None,
        causes: vec![causafera_core::CausalEventDagCause::Existing(
            current.last_change(),
        )],
        effects: vec![super::HydrologyEventEffect {
            carrier: causafera_geography::HydrologyCarrierKey::ResolutionChunk(chunk),
            property: super::HydrologyProperty::Resolution,
            before,
            after,
        }],
    };
    Ok((change, event))
}

#[cfg(test)]
mod tests {
    use super::*;
    use causafera_types::ChunkCoord;

    fn cell(chunk_x: i32, chunk_y: i32, ordinal: u16) -> HydrologyCellKey {
        HydrologyCellKey::new(
            ChartChunkCoord::new(SpatialChartId::new(1), ChunkCoord::new(chunk_x, chunk_y, 0)),
            ordinal,
        )
        .unwrap()
    }

    fn granted(shares: &[CappedShare]) -> Vec<i128> {
        shares.iter().map(|share| share.granted).collect()
    }

    #[test]
    fn block_edges_double_per_level_and_stop_at_sixteen() {
        assert_eq!(
            [0, 1, 2, 3, 4].map(block_edge),
            [1, 2, 4, 8, 16],
            "2^min(L, 4)"
        );
    }

    #[test]
    fn block_membership_uses_global_cell_coordinates_not_chunk_extent() {
        // Cell 31 of chunk 0 and cell 0 of chunk 1 are neighbours across a seam.
        // At level 1 the blocks are two cells wide, so global column 31 and
        // column 32 fall in different blocks — and at level 2, columns 32..35
        // share one. Chunk extent never enters it.
        let last_of_chunk_zero = cell(0, 0, 31);
        let first_of_chunk_one = cell(1, 0, 0);
        assert_ne!(
            HydrologyBlockKey::of(last_of_chunk_zero, 1).unwrap(),
            HydrologyBlockKey::of(first_of_chunk_one, 1).unwrap()
        );
        assert_eq!(
            HydrologyBlockKey::of(first_of_chunk_one, 2).unwrap(),
            HydrologyBlockKey::of(cell(1, 0, 3), 2).unwrap(),
            "global columns 32 and 35 share a four-wide block"
        );
    }

    #[test]
    fn a_block_can_straddle_a_chunk_seam() {
        // Global column 31 and 32 are in one block at level 5-capped-to-4, whose
        // sixteen-wide blocks start at multiples of sixteen: 31 is in block 1,
        // 32 in block 2. At level 4 with an offset chunk they do share one:
        // columns 16..31 are block 1.
        assert_eq!(
            HydrologyBlockKey::of(cell(0, 0, 16), 4).unwrap(),
            HydrologyBlockKey::of(cell(0, 0, 31), 4).unwrap()
        );
    }

    #[test]
    fn negative_chunk_coordinates_floor_rather_than_truncate() {
        // Global column -1 belongs to the block covering -2..-1, not to the one
        // covering 0..1. Truncating division would put it in the wrong block and
        // silently merge two blocks either side of the origin.
        let left = HydrologyBlockKey::of(cell(-1, 0, 31), 1).unwrap();
        let right = HydrologyBlockKey::of(cell(0, 0, 0), 1).unwrap();
        assert_ne!(left, right);
        assert_eq!(left.block_x(), -1);
        assert_eq!(right.block_x(), 0);
    }

    #[test]
    fn blocks_never_group_across_the_lattice_plane() {
        let same_plan_view = HydrologyBlockKey::of(
            HydrologyCellKey::new(
                ChartChunkCoord::new(SpatialChartId::new(1), ChunkCoord::new(0, 0, 1)),
                0,
            )
            .unwrap(),
            2,
        )
        .unwrap();
        assert_ne!(
            same_plan_view,
            HydrologyBlockKey::of(cell(0, 0, 0), 2).unwrap()
        );
    }

    #[test]
    fn an_uncapped_allocation_is_the_ordinary_largest_remainder_rule() {
        // 100 over weights 1, 2, 3 with room to spare: floors 16, 33, 50 leave one
        // unit, and the largest remainder takes it.
        let shares = allocate_capped(100, &[1, 2, 3], &[1_000, 1_000, 1_000]).unwrap();
        assert_eq!(granted(&shares), vec![17, 33, 50]);
        assert_eq!(granted(&shares).iter().sum::<i128>(), 100);
    }

    #[test]
    fn a_member_that_would_round_above_its_room_hands_the_excess_on() {
        // Equal weights want 50 each, but the first can hold only 10. The excess
        // does not vanish and does not overfill: it goes to the member with room.
        let shares = allocate_capped(100, &[1, 1], &[10, 1_000]).unwrap();
        assert_eq!(granted(&shares), vec![10, 90]);
    }

    #[test]
    fn capping_repeats_until_every_unit_has_somewhere_to_go() {
        // Three members, two of them nearly full. Each round fills whoever caps
        // and re-proportions the rest, so the total still lands exactly.
        let shares = allocate_capped(100, &[1, 1, 1], &[5, 7, 1_000]).unwrap();
        assert_eq!(granted(&shares), vec![5, 7, 88]);
        assert_eq!(granted(&shares).iter().sum::<i128>(), 100);
    }

    #[test]
    fn a_zero_weight_member_receives_nothing_even_with_room() {
        // Weight is what the process asked for. Handing water to a member that
        // asked for none would be inventing a destination.
        let shares = allocate_capped(10, &[1, 0], &[1_000, 1_000]).unwrap();
        assert_eq!(granted(&shares), vec![10, 0]);
    }

    #[test]
    fn a_total_above_the_summed_ceilings_is_an_internal_error() {
        // The caller is required to have taken `min(candidate, sum(ceilings))`,
        // so arriving here means the coarse path lost track of its own bounds.
        assert_eq!(
            allocate_capped(21, &[1, 1], &[10, 10]),
            Err(HydrologyError::AllocationExceedsCeilings)
        );
    }

    #[test]
    fn a_positive_total_with_no_eligible_member_rejects() {
        assert_eq!(
            allocate_capped(5, &[0, 0], &[10, 10]),
            Err(HydrologyError::UnallocatableTotal)
        );
    }

    #[test]
    fn zero_allocates_to_nobody_and_succeeds() {
        assert_eq!(
            granted(&allocate_capped(0, &[3, 4], &[1, 1]).unwrap()),
            vec![0, 0]
        );
        assert!(allocate_capped(0, &[], &[]).unwrap().is_empty());
    }

    #[test]
    fn clamping_handles_the_quantisation_case_the_plan_names() {
        // Two one-unit soil cells at a fraction of one half: the aggregate
        // candidate rounds to one, both fine ceilings round to zero, and the
        // group therefore moves nothing rather than moving an unreceivable unit.
        assert_eq!(clamp_to_allocatable(1, &[1, 1], &[0, 0]).unwrap(), 0);
        assert_eq!(clamp_to_allocatable(1, &[1, 1], &[0, 5]).unwrap(), 1);
    }

    #[test]
    fn clamping_ignores_room_on_members_the_process_never_addressed() {
        // A member with weight zero cannot receive, so counting its room would
        // produce a total the reducer must then refuse. The clamp and the reducer
        // therefore agree on who is eligible, and every ordinary candidate is
        // placeable.
        assert_eq!(
            clamp_to_allocatable(600, &[600, 0], &[400, 400]).unwrap(),
            400
        );
        let shares = allocate_capped(400, &[600, 0], &[400, 400]).unwrap();
        assert_eq!(granted(&shares), vec![400, 0]);
    }

    #[test]
    fn equal_remainders_break_by_ascending_member_position() {
        // Position is canonical cell order, so this is the plan's "ascending cell
        // key" tie-break expressed positionally.
        let shares = allocate_capped(5, &[1, 1, 1], &[1_000, 1_000, 1_000]).unwrap();
        assert_eq!(granted(&shares), vec![2, 2, 1]);
    }
}
