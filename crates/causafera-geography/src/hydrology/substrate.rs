use std::num::NonZeroU32;

use causafera_types::{WaterVolume, checked_water_div_floor, checked_water_mul};

use super::HydrologyStateError;

/// A validated fraction in `[0, 1]`, carried as the exact `num/den` pair.
///
/// The plan specifies each of the four hydraulic fractions as a separate
/// `_num: u32` / `_den: NonZeroU32` pair with the same `0 <= num <= den`
/// check. Carrying them as one type is the same wire and persistence contract
/// — `numerator()` and `denominator()` are what gets encoded — with the
/// invariant enforced in one place rather than repeated at four construction
/// sites and every override merge. A fraction above one is not a strong
/// process, it is a source of water, so the check is load-bearing.
///
/// Exact rational, never a float: `apply_floor` is the quantisation the ledger
/// is reconciled against, and it must be reproducible bit for bit on every
/// machine that replays the run.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HydraulicFraction {
    numerator: u32,
    denominator: NonZeroU32,
}

impl HydraulicFraction {
    /// The fraction that passes nothing. Used where a process is configured off.
    pub const ZERO: Self = Self {
        numerator: 0,
        denominator: NonZeroU32::new(1).expect("one is not zero"),
    };

    /// The fraction that passes everything.
    pub const ONE: Self = Self {
        numerator: 1,
        denominator: NonZeroU32::new(1).expect("one is not zero"),
    };

    pub const fn new(numerator: u32, denominator: NonZeroU32) -> Result<Self, HydrologyStateError> {
        if numerator > denominator.get() {
            return Err(HydrologyStateError::FractionOutOfRange {
                numerator,
                denominator: denominator.get(),
            });
        }
        Ok(Self {
            numerator,
            denominator,
        })
    }

    /// Rebuild from persisted parts, rejecting a zero denominator.
    pub const fn from_parts(numerator: u32, denominator: u32) -> Result<Self, HydrologyStateError> {
        match NonZeroU32::new(denominator) {
            Some(denominator) => Self::new(numerator, denominator),
            None => Err(HydrologyStateError::FractionOutOfRange {
                numerator,
                denominator,
            }),
        }
    }

    pub const fn numerator(self) -> u32 {
        self.numerator
    }

    pub const fn denominator(self) -> NonZeroU32 {
        self.denominator
    }

    pub const fn is_zero(self) -> bool {
        self.numerator == 0
    }

    /// `floor(value * numerator / denominator)` in the checked `i128` domain.
    pub fn apply_floor(self, value: i128) -> Result<i128, HydrologyStateError> {
        let scaled = checked_water_mul(value, i128::from(self.numerator))?;
        Ok(checked_water_div_floor(
            scaled,
            i128::from(self.denominator.get()),
        )?)
    }

    /// `apply_floor` over a carrier volume, narrowed back to a carrier volume.
    ///
    /// Cannot exceed the input, because the fraction cannot exceed one, so the
    /// narrowing here can only fail if the input itself was out of range.
    pub fn apply_to_volume(self, value: WaterVolume) -> Result<WaterVolume, HydrologyStateError> {
        Ok(WaterVolume::from_i128(self.apply_floor(value.as_i128())?)?)
    }
}

/// The hydraulic properties of the ground under one hydrology cell.
///
/// Every field is a measured physical quantity. There is no soil class, no
/// biome, no material name, and no permeability table keyed on a semantic
/// label: a future geology tranche may *generate* these numbers, but what
/// hydrology consumes is the numbers.
///
/// All three storage buckets have explicit capacity. Excess is never clamped
/// away — it is routed to an accounted destination or the whole proposal is
/// rejected — because a clamp is a hidden sink that the conservation receipt
/// would happily close over.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HydraulicSubstrateCell {
    surface_capacity: WaterVolume,
    soil_capacity: WaterVolume,
    groundwater_capacity: WaterVolume,
    infiltration_limit_per_tick: WaterVolume,
    percolation_fraction: HydraulicFraction,
    specific_yield: HydraulicFraction,
    aquifer_base_elevation_mm: i64,
    baseflow_threshold: WaterVolume,
    baseflow_fraction: HydraulicFraction,
    surface_conductance_mm2_per_tick: u64,
    groundwater_conductance_mm2_per_tick: u64,
}

/// The complete constructor input, so a caller cannot transpose two of eleven
/// positional arguments of mostly the same type and still compile.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HydraulicSubstrateParts {
    pub surface_capacity: WaterVolume,
    pub soil_capacity: WaterVolume,
    pub groundwater_capacity: WaterVolume,
    pub infiltration_limit_per_tick: WaterVolume,
    pub percolation_fraction: HydraulicFraction,
    pub specific_yield: HydraulicFraction,
    pub aquifer_base_elevation_mm: i64,
    pub baseflow_threshold: WaterVolume,
    pub baseflow_fraction: HydraulicFraction,
    pub surface_conductance_mm2_per_tick: u64,
    pub groundwater_conductance_mm2_per_tick: u64,
}

impl HydraulicSubstrateCell {
    pub fn new(parts: HydraulicSubstrateParts) -> Result<Self, HydrologyStateError> {
        // Groundwater head is `aquifer_base + floor(volume * yield_den /
        // (area * yield_num))`, so a zero yield with real groundwater capacity
        // is a division by zero waiting for the first drop to arrive. Rejected
        // here, where the pairing is visible, rather than mid-substage.
        if parts.specific_yield.is_zero() && !parts.groundwater_capacity.is_zero() {
            return Err(HydrologyStateError::ZeroSpecificYield);
        }
        Ok(Self {
            surface_capacity: parts.surface_capacity,
            soil_capacity: parts.soil_capacity,
            groundwater_capacity: parts.groundwater_capacity,
            infiltration_limit_per_tick: parts.infiltration_limit_per_tick,
            percolation_fraction: parts.percolation_fraction,
            specific_yield: parts.specific_yield,
            aquifer_base_elevation_mm: parts.aquifer_base_elevation_mm,
            baseflow_threshold: parts.baseflow_threshold,
            baseflow_fraction: parts.baseflow_fraction,
            surface_conductance_mm2_per_tick: parts.surface_conductance_mm2_per_tick,
            groundwater_conductance_mm2_per_tick: parts.groundwater_conductance_mm2_per_tick,
        })
    }

    pub const fn surface_capacity(&self) -> WaterVolume {
        self.surface_capacity
    }

    pub const fn soil_capacity(&self) -> WaterVolume {
        self.soil_capacity
    }

    pub const fn groundwater_capacity(&self) -> WaterVolume {
        self.groundwater_capacity
    }

    pub const fn infiltration_limit_per_tick(&self) -> WaterVolume {
        self.infiltration_limit_per_tick
    }

    pub const fn percolation_fraction(&self) -> HydraulicFraction {
        self.percolation_fraction
    }

    pub const fn specific_yield(&self) -> HydraulicFraction {
        self.specific_yield
    }

    pub const fn aquifer_base_elevation_mm(&self) -> i64 {
        self.aquifer_base_elevation_mm
    }

    pub const fn baseflow_threshold(&self) -> WaterVolume {
        self.baseflow_threshold
    }

    pub const fn baseflow_fraction(&self) -> HydraulicFraction {
        self.baseflow_fraction
    }

    pub const fn surface_conductance_mm2_per_tick(&self) -> u64 {
        self.surface_conductance_mm2_per_tick
    }

    pub const fn groundwater_conductance_mm2_per_tick(&self) -> u64 {
        self.groundwater_conductance_mm2_per_tick
    }

    /// The identity two cells must share to sit in the same constitutive group
    /// at coarse resolution.
    ///
    /// Every field participates. Aggregating cells that differ in any of them
    /// would invent an averaged substrate that no cell actually has, and a
    /// coarse process run against that average would then be allocated back
    /// onto cells it was never computed for.
    pub fn constitutive_key(&self) -> HydraulicSubstrateKey {
        let mut bytes = [0_u8; HYDRAULIC_SUBSTRATE_KEY_LEN];
        let mut at = 0_usize;
        let mut put = |value: [u8; 8]| {
            bytes[at..at + 8].copy_from_slice(&value);
            at += 8;
        };
        put(self.surface_capacity.get().to_be_bytes());
        put(self.soil_capacity.get().to_be_bytes());
        put(self.groundwater_capacity.get().to_be_bytes());
        put(self.infiltration_limit_per_tick.get().to_be_bytes());
        put(fraction_bytes(self.percolation_fraction));
        put(fraction_bytes(self.specific_yield));
        // The sign bit is flipped so that byte order matches numeric order for
        // this one signed field. Without it a negative aquifer base would sort
        // above every positive one, and the group ordering — which decides
        // synthetic node ID allocation — would be surprising rather than wrong.
        put((self.aquifer_base_elevation_mm as u64 ^ (1 << 63)).to_be_bytes());
        put(self.baseflow_threshold.get().to_be_bytes());
        put(fraction_bytes(self.baseflow_fraction));
        put(self.surface_conductance_mm2_per_tick.to_be_bytes());
        put(self.groundwater_conductance_mm2_per_tick.to_be_bytes());
        debug_assert_eq!(at, HYDRAULIC_SUBSTRATE_KEY_LEN);
        HydraulicSubstrateKey(bytes)
    }
}

fn fraction_bytes(fraction: HydraulicFraction) -> [u8; 8] {
    let mut out = [0_u8; 8];
    out[0..4].copy_from_slice(&fraction.numerator().to_be_bytes());
    out[4..8].copy_from_slice(&fraction.denominator().get().to_be_bytes());
    out
}

/// Bytes in one canonical substrate identity: eleven eight-byte fields.
pub const HYDRAULIC_SUBSTRATE_KEY_LEN: usize = 88;

/// Ordered, hashable canonical identity of one exact substrate parameterisation.
///
/// Big-endian and fixed-width, so the key compares as bytes in the same order
/// it compares as values and can be length-prefixed straight into a
/// constitutive-group fingerprint without a second encoding that could drift
/// from this one.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HydraulicSubstrateKey([u8; HYDRAULIC_SUBSTRATE_KEY_LEN]);

impl HydraulicSubstrateKey {
    pub const fn bytes(&self) -> &[u8; HYDRAULIC_SUBSTRATE_KEY_LEN] {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn nz(value: u32) -> NonZeroU32 {
        NonZeroU32::new(value).expect("test denominators are positive")
    }

    fn parts() -> HydraulicSubstrateParts {
        HydraulicSubstrateParts {
            surface_capacity: WaterVolume::new(1_000),
            soil_capacity: WaterVolume::new(2_000),
            groundwater_capacity: WaterVolume::new(4_000),
            infiltration_limit_per_tick: WaterVolume::new(100),
            percolation_fraction: HydraulicFraction::new(1, nz(4)).unwrap(),
            specific_yield: HydraulicFraction::new(1, nz(5)).unwrap(),
            aquifer_base_elevation_mm: -3_000,
            baseflow_threshold: WaterVolume::new(500),
            baseflow_fraction: HydraulicFraction::new(1, nz(10)).unwrap(),
            surface_conductance_mm2_per_tick: 64,
            groundwater_conductance_mm2_per_tick: 8,
        }
    }

    #[test]
    fn a_fraction_above_one_is_rejected() {
        // A fraction above one does not model a strong process; it models a
        // source of water that no receipt accounts for.
        assert_eq!(
            HydraulicFraction::new(5, nz(4)),
            Err(HydrologyStateError::FractionOutOfRange {
                numerator: 5,
                denominator: 4,
            })
        );
        assert!(HydraulicFraction::new(4, nz(4)).is_ok());
        assert!(HydraulicFraction::new(0, nz(4)).is_ok());
    }

    #[test]
    fn a_zero_denominator_cannot_be_decoded() {
        assert_eq!(
            HydraulicFraction::from_parts(0, 0),
            Err(HydrologyStateError::FractionOutOfRange {
                numerator: 0,
                denominator: 0,
            })
        );
        assert_eq!(
            HydraulicFraction::from_parts(3, 4),
            Ok(HydraulicFraction::new(3, nz(4)).unwrap())
        );
    }

    #[test]
    fn fractions_floor_exactly_and_never_exceed_their_input() {
        let third = HydraulicFraction::new(1, nz(3)).unwrap();
        assert_eq!(third.apply_floor(10).unwrap(), 3);
        assert_eq!(third.apply_floor(0).unwrap(), 0);
        assert_eq!(third.apply_floor(2).unwrap(), 0);
        // Signed inputs floor rather than truncate, matching the solver's
        // specification everywhere a head difference can be negative.
        assert_eq!(third.apply_floor(-10).unwrap(), -4);

        assert_eq!(
            HydraulicFraction::ONE
                .apply_to_volume(WaterVolume::new(u64::MAX))
                .unwrap(),
            WaterVolume::new(u64::MAX)
        );
        assert_eq!(
            HydraulicFraction::ZERO
                .apply_to_volume(WaterVolume::new(u64::MAX))
                .unwrap(),
            WaterVolume::ZERO
        );
    }

    #[test]
    fn a_whole_range_volume_times_a_whole_range_denominator_stays_checked() {
        // `value * numerator` is where a fraction can overflow `i128`; the
        // multiplication is checked rather than assumed safe.
        let steep = HydraulicFraction::new(u32::MAX, nz(u32::MAX)).unwrap();
        assert_eq!(
            steep.apply_to_volume(WaterVolume::MAX).unwrap(),
            WaterVolume::MAX
        );
    }

    #[test]
    fn groundwater_capacity_without_specific_yield_is_rejected() {
        // Given: substrate that can hold groundwater but reports no yield.
        let mut broken = parts();
        broken.specific_yield = HydraulicFraction::ZERO;

        // Then: construction refuses, because the head equation divides by the
        // yield numerator and would fail on the first drop instead.
        assert_eq!(
            HydraulicSubstrateCell::new(broken),
            Err(HydrologyStateError::ZeroSpecificYield)
        );

        // But: a cell with no groundwater capacity at all may report no yield,
        // since the head equation is never reached for it.
        let mut dry = parts();
        dry.specific_yield = HydraulicFraction::ZERO;
        dry.groundwater_capacity = WaterVolume::ZERO;
        assert!(HydraulicSubstrateCell::new(dry).is_ok());
    }

    #[test]
    fn the_constitutive_key_separates_cells_that_differ_in_any_field() {
        // Coarse execution runs one process per constitutive group, so two
        // cells sharing a key must be genuinely interchangeable for it. Any
        // field that did not participate would let an averaged substrate in.
        let base = HydraulicSubstrateCell::new(parts()).unwrap();
        assert_eq!(base.constitutive_key(), base.constitutive_key());

        let variants: Vec<HydraulicSubstrateParts> = vec![
            HydraulicSubstrateParts {
                surface_capacity: WaterVolume::new(1_001),
                ..parts()
            },
            HydraulicSubstrateParts {
                soil_capacity: WaterVolume::new(2_001),
                ..parts()
            },
            HydraulicSubstrateParts {
                groundwater_capacity: WaterVolume::new(4_001),
                ..parts()
            },
            HydraulicSubstrateParts {
                infiltration_limit_per_tick: WaterVolume::new(101),
                ..parts()
            },
            HydraulicSubstrateParts {
                percolation_fraction: HydraulicFraction::new(1, nz(5)).unwrap(),
                ..parts()
            },
            HydraulicSubstrateParts {
                specific_yield: HydraulicFraction::new(1, nz(6)).unwrap(),
                ..parts()
            },
            HydraulicSubstrateParts {
                aquifer_base_elevation_mm: -2_999,
                ..parts()
            },
            HydraulicSubstrateParts {
                baseflow_threshold: WaterVolume::new(501),
                ..parts()
            },
            HydraulicSubstrateParts {
                baseflow_fraction: HydraulicFraction::new(1, nz(11)).unwrap(),
                ..parts()
            },
            HydraulicSubstrateParts {
                surface_conductance_mm2_per_tick: 65,
                ..parts()
            },
            HydraulicSubstrateParts {
                groundwater_conductance_mm2_per_tick: 9,
                ..parts()
            },
        ];
        assert_eq!(variants.len(), 11, "every substrate field must be covered");
        for variant in variants {
            let changed = HydraulicSubstrateCell::new(variant).unwrap();
            assert_ne!(base.constitutive_key(), changed.constitutive_key());
        }
    }

    #[test]
    fn the_constitutive_key_orders_and_hashes_consistently() {
        use std::collections::BTreeSet;

        let low = HydraulicSubstrateCell::new(parts()).unwrap();
        let high = HydraulicSubstrateCell::new(HydraulicSubstrateParts {
            surface_capacity: WaterVolume::new(9_999),
            ..parts()
        })
        .unwrap();
        assert!(low.constitutive_key() < high.constitutive_key());

        let mut set = BTreeSet::new();
        assert!(set.insert(low.constitutive_key()));
        assert!(!set.insert(low.constitutive_key()));
        assert!(set.insert(high.constitutive_key()));
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn every_accessor_returns_what_was_configured() {
        let cell = HydraulicSubstrateCell::new(parts()).unwrap();
        assert_eq!(cell.surface_capacity(), WaterVolume::new(1_000));
        assert_eq!(cell.soil_capacity(), WaterVolume::new(2_000));
        assert_eq!(cell.groundwater_capacity(), WaterVolume::new(4_000));
        assert_eq!(cell.infiltration_limit_per_tick(), WaterVolume::new(100));
        assert_eq!(cell.percolation_fraction().numerator(), 1);
        assert_eq!(cell.specific_yield().denominator().get(), 5);
        assert_eq!(cell.aquifer_base_elevation_mm(), -3_000);
        assert_eq!(cell.baseflow_threshold(), WaterVolume::new(500));
        assert_eq!(cell.baseflow_fraction().denominator().get(), 10);
        assert_eq!(cell.surface_conductance_mm2_per_tick(), 64);
        assert_eq!(cell.groundwater_conductance_mm2_per_tick(), 8);
    }
}
