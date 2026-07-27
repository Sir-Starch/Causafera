//! Spatially coherent surface-material regions (`TODO-GEO-004`).
//!
//! `terrain_cells` (`carrier.rs`) used to derive a cell's material from the
//! same well-mixed per-cell hash that drives elevation, so material was
//! independent noise: 6.5% same-material neighbours against 6.25% expected
//! from chance over sixteen materials. `terrain_structure`'s `material_delta`
//! term is a `.max()` over up to four neighbours, so independent noise makes
//! it nonzero almost everywhere — measured at a mean contribution of 50.0 of
//! 204.8 (24.4%) to `terrain_structure`, a de facto constant floor the
//! carrier's own doc comment says should not exist ("a floor under every cell
//! would make a flat plain drive the field as hard as a ridge does").
//!
//! This module replaces that per-cell hash with a bounded Worley (cellular)
//! partition: a coarse grid of feature points, each owning the region of
//! space nearest to it. A query cell's material is the material of its
//! nearest feature point, found by searching the 5x5 (Chebyshev-2)
//! neighbourhood of coarse cells around it.
//!
//! Chebyshev-2 (5x5) is both necessary and sufficient here, not the more
//! common 3x3, because jitter is confined within its own coarse cell (range
//! `[0, MATERIAL_REGION_SIZE)`, written `R` below) rather than centred on it:
//!
//! - **3x3 is not sufficient.** A query near the edge of its own cell (not a
//!   corner) can sit at distance as little as `R` from a feature in the
//!   *same-row* Chebyshev-2 cell (the query at `x = R - ε`, the feature at
//!   the near edge of the cell starting at `x = 2R`, giving `R + ε`). The
//!   query's own-cell feature can be as far as `sqrt(2) * R` (opposite
//!   corner), and `R < sqrt(2) * R`, so a Chebyshev-2 cell can hold a nearer
//!   feature than the own cell guarantees — it must be searched.
//! - **Chebyshev-3 does not need to be.** The nearest any point in a
//!   Chebyshev-3 cell can be to a query in the centre cell is `2R` (two full
//!   cell-widths), and `2R > sqrt(2) * R`. The own cell's feature is *always*
//!   found within `sqrt(2) * R` in the worst case, so it already beats
//!   anything Chebyshev-3 could offer, for every query position and every
//!   jitter configuration.
//!
//! Chebyshev-2 sits exactly between those two bounds (`R < sqrt(2) * R <
//! 2R`), which is what makes it the minimal provably correct radius.
//!
//! Like `terrain_cells`' elevation, this is a pure function of a cell's
//! position in its chart (`global_x`/`global_y`, not chunk-local coordinates)
//! and a chart-scoped seed, so regions are continuous across chunk
//! boundaries by construction, exactly as `TODO-GEO-005` requires for
//! elevation.

use causafera_types::MaterialId;

use crate::carrier::mix64;

/// Cells per side of one coarse feature-grid cell. Chosen from a sweep
/// against same-material neighbour rate and the mana column footprint
/// (`CHUNK_SIZE / chunk_extent`, 10.7 cells at the production default extent
/// 3): 16 sits close to that footprint (about 1.5x), giving four regions per
/// chunk on average — coherent enough that refining the mana lattice has
/// real region structure to resolve, without collapsing a whole chunk to one
/// material the way 32 or 64 do. Same-*material* neighbour rate overstates
/// coherence slightly, since two different regions can draw the same
/// material by chance (`MATERIAL_COUNT` = 16); the same-*region* rate is the
/// uninflated figure and is what `region_identity`'s test measures. See
/// `plans/coherent-surface-material-regions.md` for the full sweep and both
/// figures.
const MATERIAL_REGION_SIZE: i64 = 16;
/// Matches the sixteen material identifiers `terrain_cells` has always
/// produced (`MaterialId::new(1..=16)`); this module changes how spatially
/// clustered an assignment is, not how many distinct materials exist.
/// Materials are drawn independently per feature point, so two adjacent but
/// distinct regions agree by chance roughly `1 / MATERIAL_COUNT` of the time
/// — see `region_identity` for the uninflated same-*region* rate this causes
/// same-*material* rate to slightly overstate.
const MATERIAL_COUNT: u64 = 16;

/// The feature point that owns one coarse grid cell: its exact position,
/// jittered within the cell so region boundaries are irregular rather than a
/// rigid grid, and the material it seeds. A pure function of the cell's
/// chart-scoped coordinate and `chart_seed`, so two charts never share a
/// region layout and two chunks of the same chart always agree at any
/// coarse cell they both reference.
fn feature_point(chart_seed: u64, coarse_x: i64, coarse_y: i64) -> (i64, i64, MaterialId) {
    let x = mix64((coarse_x as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15));
    let y = mix64((coarse_y as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F));
    let key = mix64(chart_seed ^ x ^ y.rotate_left(29));
    let mask = MATERIAL_REGION_SIZE - 1; // MATERIAL_REGION_SIZE is a power of two
    let jitter_x = (key & 0xFFFF) as i64 & mask;
    let jitter_y = ((key >> 16) & 0xFFFF) as i64 & mask;
    let material = ((key >> 32) % MATERIAL_COUNT) + 1;
    (
        coarse_x * MATERIAL_REGION_SIZE + jitter_x,
        coarse_y * MATERIAL_REGION_SIZE + jitter_y,
        MaterialId::new(material),
    )
}

/// The nearest feature point to one chart-global cell position, identified by
/// the coarse cell that owns it. Two positions in the same region always
/// agree here even where `region_material` cannot tell them apart from a
/// different region that happened to draw the same material (see
/// `MATERIAL_COUNT`'s doc comment on collisions).
fn nearest_feature(chart_seed: u64, global_x: i64, global_y: i64) -> (i64, i64, MaterialId) {
    let coarse_x = global_x.div_euclid(MATERIAL_REGION_SIZE);
    let coarse_y = global_y.div_euclid(MATERIAL_REGION_SIZE);
    (-2..=2)
        .flat_map(|dx| (-2..=2).map(move |dy| (dx, dy)))
        .map(|(dx, dy)| {
            let (fx, fy, material) = feature_point(chart_seed, coarse_x + dx, coarse_y + dy);
            (coarse_x + dx, coarse_y + dy, fx, fy, material)
        })
        .min_by_key(|&(_, _, fx, fy, _)| {
            let dx = global_x - fx;
            let dy = global_y - fy;
            dx * dx + dy * dy
        })
        .map(|(coarse_x, coarse_y, _, _, material)| (coarse_x, coarse_y, material))
        .expect("5x5 neighbourhood is never empty")
}

/// The surface material at one chart-global cell position.
pub(crate) fn region_material(chart_seed: u64, global_x: i64, global_y: i64) -> MaterialId {
    nearest_feature(chart_seed, global_x, global_y).2
}

/// The identity of the region owning one chart-global cell position (its
/// feature point's coarse coordinate), distinct from `region_material`: two
/// different regions can draw the same material by chance
/// (`MATERIAL_COUNT` = 16, so roughly 1 in 16 adjacent-region pairs collide),
/// and only this function tells that case apart from two positions actually
/// belonging to the same region.
#[cfg(test)]
pub(crate) fn region_identity(chart_seed: u64, global_x: i64, global_y: i64) -> (i64, i64) {
    let (coarse_x, coarse_y, _) = nearest_feature(chart_seed, global_x, global_y);
    (coarse_x, coarse_y)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adjacent_chart_positions_usually_share_a_material() {
        let mut same = 0u32;
        let mut total = 0u32;
        for y in 0..64i64 {
            for x in 0..64i64 {
                let material = region_material(7, x, y);
                total += 1;
                if region_material(7, x + 1, y) == material {
                    same += 1;
                }
            }
        }
        let rate = f64::from(same) / f64::from(total);
        assert!(
            rate > 0.5,
            "expected substantial coherence, got {rate} same-material neighbours"
        );
    }

    #[test]
    fn same_material_neighbours_are_mostly_the_same_region_not_a_material_collision() {
        // With sixteen materials drawn independently per feature point, two
        // adjacent but distinct regions agree on material by chance about
        // 1/16 of the time -- that collision inflates the same-*material*
        // rate above the true same-*region* rate. Both are measured here so
        // neither is mistaken for the other.
        let mut same_material = 0u32;
        let mut same_region = 0u32;
        let mut total = 0u32;
        for y in 0..64i64 {
            for x in 0..64i64 {
                total += 1;
                if region_material(7, x, y) == region_material(7, x + 1, y) {
                    same_material += 1;
                }
                if region_identity(7, x, y) == region_identity(7, x + 1, y) {
                    same_region += 1;
                }
            }
        }
        let material_rate = f64::from(same_material) / f64::from(total);
        let region_rate = f64::from(same_region) / f64::from(total);
        // Measured once at these coordinates and seed: material_rate 92.1%,
        // region_rate 91.7% -- material collision at a true region boundary
        // (chance 1/16) inflates the material-only reading by well under one
        // point, not by the several points a naive 1/16-of-all-pairs estimate
        // would suggest, because most neighbour pairs are region-interior and
        // cannot be inflated at all.
        assert!(
            region_rate > 0.5,
            "expected substantial region coherence, got {region_rate}"
        );
        assert!(
            material_rate >= region_rate,
            "material rate {material_rate} should never be below the true region rate {region_rate}"
        );
    }

    #[test]
    fn a_region_boundary_does_not_notice_a_chunk_boundary() {
        // The east edge of chart-local chunk (0,0) against the west edge of
        // chunk (1,0), exactly the seam TODO-GEO-005 fixed for elevation.
        // Both edges are read purely from global coordinates, so nothing
        // distinguishes this from any other pair of adjacent columns.
        let mut same = 0u32;
        let mut total = 0u32;
        for y in 0..32i64 {
            let west = region_material(7, 31, y);
            let east = region_material(7, 32, y);
            total += 1;
            if west == east {
                same += 1;
            }
        }
        let boundary_rate = f64::from(same) / f64::from(total);

        let mut interior_same = 0u32;
        let mut interior_total = 0u32;
        for y in 0..32i64 {
            for x in 0..31i64 {
                let material = region_material(7, x, y);
                interior_total += 1;
                if region_material(7, x + 1, y) == material {
                    interior_same += 1;
                }
            }
        }
        let interior_rate = f64::from(interior_same) / f64::from(interior_total);

        assert!(
            (boundary_rate - interior_rate).abs() < 0.25,
            "boundary rate {boundary_rate} should be the same order as interior rate {interior_rate}"
        );
    }

    #[test]
    fn the_same_chart_position_always_yields_the_same_material() {
        assert_eq!(region_material(7, 100, -50), region_material(7, 100, -50));
    }

    #[test]
    fn different_charts_can_disagree_at_the_same_position() {
        let seed_a = mix64(1 ^ mix64(1));
        let seed_b = mix64(1 ^ mix64(2));
        let mut any_difference = false;
        for y in 0..8i64 {
            for x in 0..8i64 {
                if region_material(seed_a, x, y) != region_material(seed_b, x, y) {
                    any_difference = true;
                    break;
                }
            }
        }
        assert!(any_difference, "two charts should not always agree");
    }
}
