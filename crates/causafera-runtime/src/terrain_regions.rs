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
//! Chebyshev-2 is the provably sufficient search radius, not the more common
//! 3x3: jitter is confined within its own coarse cell (range `[0,
//! MATERIAL_REGION_SIZE)`), so the own cell's feature alone bounds the best
//! candidate at at most `sqrt(2) * MATERIAL_REGION_SIZE` (the cell's
//! diagonal), which is less than `2 * MATERIAL_REGION_SIZE` — the minimum
//! possible distance from any point in the query's own cell to any point in
//! a Chebyshev-3 cell. A 3x3 search is therefore not exact for full-cell
//! jitter, only Chebyshev-2 (5x5) is.
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
/// material the way 32 or 64 do. Measured same-material neighbour rate at 16
/// is 93.15% interior against 93.75% across a chunk boundary — the same
/// order, evidence of no boundary artifact. See `plans/coherent-surface-material-regions.md`.
const MATERIAL_REGION_SIZE: i64 = 16;
/// Matches the sixteen material identifiers `terrain_cells` has always
/// produced (`MaterialId::new(1..=16)`); this module changes how spatially
/// clustered an assignment is, not how many distinct materials exist.
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

/// The surface material at one chart-global cell position.
pub(crate) fn region_material(chart_seed: u64, global_x: i64, global_y: i64) -> MaterialId {
    let coarse_x = global_x.div_euclid(MATERIAL_REGION_SIZE);
    let coarse_y = global_y.div_euclid(MATERIAL_REGION_SIZE);
    (-2..=2)
        .flat_map(|dx| (-2..=2).map(move |dy| (dx, dy)))
        .map(|(dx, dy)| feature_point(chart_seed, coarse_x + dx, coarse_y + dy))
        .min_by_key(|&(fx, fy, _)| {
            let dx = global_x - fx;
            let dy = global_y - fy;
            dx * dx + dy * dy
        })
        .map(|(_, _, material)| material)
        .expect("5x5 neighbourhood is never empty")
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
