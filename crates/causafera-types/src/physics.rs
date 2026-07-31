use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Non-negative fixed-point thermal energy in the carrier's base unit.
///
/// `i64::MAX` is the maximum so every cell value remains representable while
/// transfer arithmetic can use `i128` without narrowing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ThermalEnergy(i64);

impl ThermalEnergy {
    pub const ZERO: Self = Self(0);
    pub const MAX: Self = Self(i64::MAX);

    pub const fn new(value: i64) -> Result<Self, ThermalEnergyError> {
        if value < 0 {
            Err(ThermalEnergyError::NegativeValue(value))
        } else {
            Ok(Self(value))
        }
    }

    pub const fn get(self) -> i64 {
        self.0
    }
}

impl std::fmt::Display for ThermalEnergy {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}", self.0)
    }
}

#[derive(Clone, Copy, Debug, Error, PartialEq, Eq)]
pub enum ThermalEnergyError {
    #[error("thermal energy cannot be negative: {0}")]
    NegativeValue(i64),
}

/// Non-negative fixed-point water volume in cubic millimetres (`1 mm³ = 1 μL`).
///
/// Millimetres are the unit terrain already measures elevation in, so a depth
/// and an elevation can be added without a scale conversion sitting between
/// them. The full `u64` range is valid: unlike thermal energy this carrier has
/// no signed counterpart, and every operation that could leave the range is
/// checked rather than saturating.
///
/// Exactness here is computational, not physical. Fixed point buys replay
/// equality and a ledger that closes to zero; it does not make the routing
/// model a scientifically exact one.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WaterVolume(u64);

impl WaterVolume {
    pub const ZERO: Self = Self(0);
    pub const MAX: Self = Self(u64::MAX);

    pub const fn new(cubic_millimetres: u64) -> Self {
        Self(cubic_millimetres)
    }

    pub const fn get(self) -> u64 {
        self.0
    }

    pub const fn as_i128(self) -> i128 {
        self.0 as i128
    }

    pub const fn is_zero(self) -> bool {
        self.0 == 0
    }

    /// Narrow an accumulated `i128` back to a carrier value.
    ///
    /// This is the only way back from the accumulation domain, so it is the one
    /// place a negative or over-wide intermediate can be caught. Saturating
    /// here would manufacture water at the ceiling and destroy it at the floor.
    pub const fn from_i128(value: i128) -> Result<Self, WaterVolumeError> {
        if value < 0 || value > u64::MAX as i128 {
            return Err(WaterVolumeError::OutOfRange(value));
        }
        Ok(Self(value as u64))
    }

    pub const fn checked_add(self, other: Self) -> Result<Self, WaterVolumeError> {
        match self.0.checked_add(other.0) {
            Some(sum) => Ok(Self(sum)),
            None => Err(WaterVolumeError::Overflow),
        }
    }

    pub const fn checked_sub(self, other: Self) -> Result<Self, WaterVolumeError> {
        match self.0.checked_sub(other.0) {
            Some(difference) => Ok(Self(difference)),
            None => Err(WaterVolumeError::Underflow),
        }
    }

    /// Remaining headroom below `capacity`, or zero when already at or above it.
    pub const fn remaining_below(self, capacity: Self) -> Self {
        if capacity.0 > self.0 {
            Self(capacity.0 - self.0)
        } else {
            Self(0)
        }
    }

    pub const fn min(self, other: Self) -> Self {
        if self.0 < other.0 { self } else { other }
    }
}

impl std::fmt::Display for WaterVolume {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}", self.0)
    }
}

/// Non-negative water depth in millimetres, aligned with terrain elevation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WaterDepthMm(u64);

impl WaterDepthMm {
    pub const ZERO: Self = Self(0);

    pub const fn new(millimetres: u64) -> Self {
        Self(millimetres)
    }

    pub const fn millimetres(self) -> u64 {
        self.0
    }

    pub const fn as_i128(self) -> i128 {
        self.0 as i128
    }

    pub const fn from_i128(value: i128) -> Result<Self, WaterVolumeError> {
        if value < 0 || value > u64::MAX as i128 {
            return Err(WaterVolumeError::OutOfRange(value));
        }
        Ok(Self(value as u64))
    }
}

impl std::fmt::Display for WaterDepthMm {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}", self.0)
    }
}

/// Exact `i128` accumulation domain for water arithmetic.
///
/// Every hydrology sum, product, proportional allocation, and conservation
/// residual runs through here rather than through `u64` or `i64`. Two things
/// follow that the carrier type alone cannot give: an intermediate may be
/// negative (a residual is a difference, and a difference that is *not* zero is
/// exactly what must be reportable), and no step can silently wrap. Nothing
/// leaves this domain without `into_volume` re-checking the range.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WaterAccumulator(i128);

impl WaterAccumulator {
    pub const ZERO: Self = Self(0);

    pub const fn new(value: i128) -> Self {
        Self(value)
    }

    pub const fn get(self) -> i128 {
        self.0
    }

    pub const fn is_zero(self) -> bool {
        self.0 == 0
    }

    pub const fn add(self, value: i128) -> Result<Self, WaterVolumeError> {
        match self.0.checked_add(value) {
            Some(sum) => Ok(Self(sum)),
            None => Err(WaterVolumeError::Overflow),
        }
    }

    pub const fn sub(self, value: i128) -> Result<Self, WaterVolumeError> {
        match self.0.checked_sub(value) {
            Some(difference) => Ok(Self(difference)),
            None => Err(WaterVolumeError::Underflow),
        }
    }

    pub const fn add_volume(self, volume: WaterVolume) -> Result<Self, WaterVolumeError> {
        self.add(volume.as_i128())
    }

    pub const fn sub_volume(self, volume: WaterVolume) -> Result<Self, WaterVolumeError> {
        self.sub(volume.as_i128())
    }

    pub const fn into_volume(self) -> Result<WaterVolume, WaterVolumeError> {
        WaterVolume::from_i128(self.0)
    }
}

/// Checked `a * b` in the accumulation domain.
pub const fn checked_water_mul(a: i128, b: i128) -> Result<i128, WaterVolumeError> {
    match a.checked_mul(b) {
        Some(product) => Ok(product),
        None => Err(WaterVolumeError::Overflow),
    }
}

/// Checked flooring division in the accumulation domain.
///
/// Rust's `/` truncates toward zero, which is not flooring for a negative
/// numerator. Every quantisation in the hydrology solver is specified as
/// `floor`, and a head difference or a residual can be negative, so truncation
/// would move a rounding remainder by one unit in exactly the cases the ledger
/// is least able to absorb it.
pub const fn checked_water_div_floor(
    numerator: i128,
    denominator: i128,
) -> Result<i128, WaterVolumeError> {
    if denominator == 0 {
        return Err(WaterVolumeError::ZeroDenominator);
    }
    let Some(quotient) = numerator.checked_div(denominator) else {
        return Err(WaterVolumeError::Overflow);
    };
    let Some(remainder) = numerator.checked_rem(denominator) else {
        return Err(WaterVolumeError::Overflow);
    };
    if remainder != 0 && ((remainder < 0) != (denominator < 0)) {
        match quotient.checked_sub(1) {
            Some(floored) => Ok(floored),
            None => Err(WaterVolumeError::Underflow),
        }
    } else {
        Ok(quotient)
    }
}

/// Checked Euclidean remainder paired with [`checked_water_div_floor`].
///
/// The result carries the sign of the denominator, so
/// `floor_div * denominator + this == numerator` holds exactly. A largest-
/// remainder allocation orders on this value, so a `%` that could come back
/// negative would order the tie-break wrongly rather than merely oddly.
pub const fn checked_water_rem_floor(
    numerator: i128,
    denominator: i128,
) -> Result<i128, WaterVolumeError> {
    if denominator == 0 {
        return Err(WaterVolumeError::ZeroDenominator);
    }
    let Some(remainder) = numerator.checked_rem(denominator) else {
        return Err(WaterVolumeError::Overflow);
    };
    if remainder != 0 && ((remainder < 0) != (denominator < 0)) {
        match remainder.checked_add(denominator) {
            Some(adjusted) => Ok(adjusted),
            None => Err(WaterVolumeError::Overflow),
        }
    } else {
        Ok(remainder)
    }
}

#[derive(Clone, Copy, Debug, Error, PartialEq, Eq)]
pub enum WaterVolumeError {
    #[error("water arithmetic overflowed")]
    Overflow,
    #[error("water arithmetic underflowed")]
    Underflow,
    #[error("water arithmetic divided by zero")]
    ZeroDenominator,
    #[error("value {0} is outside the water volume range")]
    OutOfRange(i128),
}

/// Temperature in Kelvin.
///
/// This is a physical primitive that exists in Ground Truth regardless of
/// whether any agent recognizes it as "hot" or "cold".
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Temperature {
    pub kelvin: f64,
}

impl Temperature {
    pub const fn new(kelvin: f64) -> Self {
        Self { kelvin }
    }

    /// Returns the Celsius value.
    pub fn celsius(self) -> f64 {
        self.kelvin - 273.15
    }

    /// Absolute zero in Kelvin.
    pub const ABSOLUTE_ZERO: f64 = 0.0;
}

/// 3-D orientation in radians.
///
/// Yaw, pitch, and roll follow the aviation convention:
/// - yaw: rotation around the vertical axis (0 = forward)
/// - pitch: rotation around the lateral axis (0 = level)
/// - roll: rotation around the longitudinal axis (0 = upright)
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Orientation {
    pub yaw: f64,
    pub pitch: f64,
    pub roll: f64,
}

impl Orientation {
    pub const fn new(yaw: f64, pitch: f64, roll: f64) -> Self {
        Self { yaw, pitch, roll }
    }
}

/// 3-D linear velocity vector.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Velocity {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Velocity {
    pub const fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }

    /// Squared magnitude.
    pub fn length_squared(self) -> f64 {
        self.x
            .mul_add(self.x, self.y.mul_add(self.y, self.z * self.z))
    }
}

/// 3-D angular velocity vector.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct AngularVelocity {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl AngularVelocity {
    pub const fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }
}

/// Combined linear and angular motion state.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Motion {
    pub velocity: Velocity,
    pub angular_velocity: AngularVelocity,
}

impl Motion {
    pub const fn new(velocity: Velocity, angular_velocity: AngularVelocity) -> Self {
        Self {
            velocity,
            angular_velocity,
        }
    }
}

/// A material with physical properties, not semantic labels.
///
/// The engine does not contain a taxonomy of materials ("wood", "stone",
/// "iron"). Instead, each material is defined by measurable physical properties.
/// Agents may later construct subjective concepts grouping materials by
/// perceived similarity, but those concepts are emergent, not primitive.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Material {
    /// Globally unique material identifier.
    pub id: crate::MaterialId,
    /// Density in kg/m³.
    pub density: f64,
    /// Thermal conductivity in W/(m·K).
    pub thermal_conductivity: f64,
    /// Specific heat capacity in J/(kg·K).
    pub specific_heat: f64,
    /// Mohs hardness scale (0–10).
    pub hardness: f64,
    /// Porosity fraction (0.0 = solid, 1.0 = entirely void).
    pub porosity: f64,
    /// Component substances and their mass fractions.
    /// Fractions should sum to 1.0; the engine does not enforce this at the type level.
    pub composition: Vec<(crate::SubstanceId, f64)>,
}

impl Material {
    pub fn new(
        id: crate::MaterialId,
        density: f64,
        thermal_conductivity: f64,
        specific_heat: f64,
        hardness: f64,
        porosity: f64,
    ) -> Self {
        Self {
            id,
            density,
            thermal_conductivity,
            specific_heat,
            hardness,
            porosity,
            composition: Vec::new(),
        }
    }

    /// Add a substance component by mass fraction.
    pub fn add_component(&mut self, substance: crate::SubstanceId, fraction: f64) {
        self.composition.push((substance, fraction));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MaterialId;

    #[test]
    fn temperature_celsius_conversion() {
        let t = Temperature::new(273.15);
        assert!((t.celsius()).abs() < f64::EPSILON * 10.0);
    }

    #[test]
    fn thermal_energy_rejects_negative_values() {
        // Given: a fixed-point value below the conserved carrier's lower bound.
        let negative = -1;

        // When: the value is parsed as authoritative thermal energy.
        let result = ThermalEnergy::new(negative);

        // Then: construction rejects the invalid carrier value.
        assert_eq!(result, Err(ThermalEnergyError::NegativeValue(negative)));
    }

    #[test]
    fn orientation_creation() {
        let o = Orientation::new(1.0, 0.5, -0.2);
        assert_eq!(o.yaw, 1.0);
        assert_eq!(o.pitch, 0.5);
        assert_eq!(o.roll, -0.2);
    }

    #[test]
    fn velocity_length_squared() {
        let v = Velocity::new(3.0, 4.0, 0.0);
        assert!((v.length_squared() - 25.0).abs() < f64::EPSILON);
    }

    #[test]
    fn motion_roundtrip() {
        let m = Motion::new(
            Velocity::new(1.0, 2.0, 3.0),
            AngularVelocity::new(0.1, 0.2, 0.3),
        );
        let serialized = serde_json::to_string(&m).unwrap();
        let deserialized: Motion = serde_json::from_str(&serialized).unwrap();
        assert_eq!(m, deserialized);
    }

    #[test]
    fn material_creation() {
        let mut mat = Material::new(MaterialId::new(1), 2700.0, 237.0, 900.0, 6.5, 0.02);
        mat.add_component(crate::SubstanceId::new(10), 0.6);
        mat.add_component(crate::SubstanceId::new(11), 0.4);
        assert_eq!(mat.id, MaterialId::new(1));
        assert_eq!(mat.composition.len(), 2);
    }

    #[test]
    fn water_volume_arithmetic_never_saturates() {
        // Given: carrier values at both ends of the range.
        let max = WaterVolume::MAX;
        let one = WaterVolume::new(1);

        // When/Then: neither end wraps or clamps. A saturating ceiling would
        // manufacture water; a saturating floor would destroy it, and the
        // conservation receipt would close over the lie either way.
        assert_eq!(max.checked_add(one), Err(WaterVolumeError::Overflow));
        assert_eq!(
            WaterVolume::ZERO.checked_sub(one),
            Err(WaterVolumeError::Underflow)
        );
        assert_eq!(max.checked_sub(max), Ok(WaterVolume::ZERO));
        assert_eq!(
            WaterVolume::new(u64::MAX - 1).checked_add(one),
            Ok(WaterVolume::MAX)
        );
    }

    #[test]
    fn water_volume_conversion_rejects_values_outside_its_range() {
        // Given: accumulated intermediates on either side of the carrier range.
        let above = i128::from(u64::MAX) + 1;
        let below = -1_i128;

        // When/Then: leaving the accumulation domain re-checks the range.
        assert_eq!(
            WaterVolume::from_i128(above),
            Err(WaterVolumeError::OutOfRange(above))
        );
        assert_eq!(
            WaterVolume::from_i128(below),
            Err(WaterVolumeError::OutOfRange(below))
        );
        assert_eq!(
            WaterVolume::from_i128(i128::from(u64::MAX)),
            Ok(WaterVolume::MAX)
        );
        assert_eq!(WaterVolume::MAX.as_i128(), i128::from(u64::MAX));
    }

    #[test]
    fn remaining_headroom_is_saturating_only_at_zero() {
        // `remaining_below` answers "how much more fits", so being over
        // capacity is zero headroom rather than a negative one. It is not a
        // clamp of a stored value: no carrier is written through it.
        assert_eq!(
            WaterVolume::new(3).remaining_below(WaterVolume::new(10)),
            WaterVolume::new(7)
        );
        assert_eq!(
            WaterVolume::new(10).remaining_below(WaterVolume::new(10)),
            WaterVolume::ZERO
        );
        assert_eq!(
            WaterVolume::new(11).remaining_below(WaterVolume::new(10)),
            WaterVolume::ZERO
        );
    }

    #[test]
    fn accumulator_holds_negative_intermediates_and_checks_both_ends() {
        // Given: the accumulation domain a conservation residual is computed in.
        let ledger = WaterAccumulator::ZERO;

        // When: more is withdrawn than deposited.
        let negative = ledger
            .add_volume(WaterVolume::new(5))
            .unwrap()
            .sub_volume(WaterVolume::new(9))
            .unwrap();

        // Then: the intermediate is representable and negative — a residual
        // that cannot be negative cannot report a loss.
        assert_eq!(negative.get(), -4);
        assert_eq!(
            negative.into_volume(),
            Err(WaterVolumeError::OutOfRange(-4))
        );
        assert!(!negative.is_zero());
        assert!(WaterAccumulator::ZERO.is_zero());

        // And: both ends of `i128` are checked rather than wrapped.
        assert_eq!(
            WaterAccumulator::new(i128::MAX).add(1),
            Err(WaterVolumeError::Overflow)
        );
        assert_eq!(
            WaterAccumulator::new(i128::MIN).sub(1),
            Err(WaterVolumeError::Underflow)
        );
    }

    #[test]
    fn division_floors_rather_than_truncating_toward_zero() {
        // Rust's `/` truncates toward zero. Every quantisation in the solver is
        // specified as `floor`, and heads and residuals are signed, so the two
        // disagree exactly where a rounding unit is hardest to account for.
        assert_eq!(checked_water_div_floor(7, 2), Ok(3));
        assert_eq!(checked_water_div_floor(-7, 2), Ok(-4));
        assert_eq!(checked_water_div_floor(7, -2), Ok(-4));
        assert_eq!(checked_water_div_floor(-7, -2), Ok(3));
        assert_eq!(checked_water_div_floor(-6, 2), Ok(-3));
        assert_eq!(-7_i128 / 2, -3, "the truncating operator this replaces");
    }

    #[test]
    fn floor_remainder_pairs_exactly_with_floor_division() {
        // A largest-remainder allocation orders on this value, so it has to be
        // the remainder that reconstructs the numerator, not the one `%` gives.
        for numerator in [-9_i128, -7, -1, 0, 1, 7, 9] {
            for denominator in [-4_i128, -3, 3, 4] {
                let quotient = checked_water_div_floor(numerator, denominator).unwrap();
                let remainder = checked_water_rem_floor(numerator, denominator).unwrap();
                assert_eq!(
                    quotient * denominator + remainder,
                    numerator,
                    "{numerator} / {denominator}"
                );
                assert!(
                    (remainder == 0) || ((remainder < 0) == (denominator < 0)),
                    "{numerator} % {denominator} must carry the denominator's sign"
                );
            }
        }
    }

    #[test]
    fn zero_denominators_and_overflowing_products_are_rejected() {
        assert_eq!(
            checked_water_div_floor(1, 0),
            Err(WaterVolumeError::ZeroDenominator)
        );
        assert_eq!(
            checked_water_rem_floor(1, 0),
            Err(WaterVolumeError::ZeroDenominator)
        );
        assert_eq!(
            checked_water_mul(i128::MAX, 2),
            Err(WaterVolumeError::Overflow)
        );
        // `i128` is not wide enough to hold the product of two whole-range
        // carrier values: `(2^64 - 1)^2` needs 128 unsigned bits and `i128`
        // offers 127. So "accumulate in `i128`" is not on its own a guarantee
        // that a multiplication is safe, and every product in the solver is
        // checked rather than assumed. Squaring `u32::MAX` — the scale a
        // per-tick conductance times a head difference actually reaches — has
        // room to spare.
        assert_eq!(
            checked_water_mul(i128::from(u64::MAX), i128::from(u64::MAX)),
            Err(WaterVolumeError::Overflow)
        );
        assert_eq!(
            checked_water_mul(i128::from(u64::MAX), 2),
            Ok(i128::from(u64::MAX) * 2)
        );
        assert_eq!(
            checked_water_mul(i128::from(u32::MAX), i128::from(u32::MAX)),
            Ok(i128::from(u32::MAX) * i128::from(u32::MAX))
        );
    }

    #[test]
    fn water_depth_shares_the_carrier_range_and_checks() {
        assert_eq!(WaterDepthMm::new(4).millimetres(), 4);
        assert_eq!(WaterDepthMm::ZERO.as_i128(), 0);
        assert_eq!(
            WaterDepthMm::from_i128(-1),
            Err(WaterVolumeError::OutOfRange(-1))
        );
        assert_eq!(
            WaterDepthMm::from_i128(i128::from(u64::MAX)),
            Ok(WaterDepthMm::new(u64::MAX))
        );
    }

    #[test]
    fn temperature_serde_roundtrip() {
        let t = Temperature::new(300.0);
        let serialized = serde_json::to_string(&t).unwrap();
        let deserialized: Temperature = serde_json::from_str(&serialized).unwrap();
        assert_eq!(t, deserialized);
    }
}
