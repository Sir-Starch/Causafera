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
    fn temperature_serde_roundtrip() {
        let t = Temperature::new(300.0);
        let serialized = serde_json::to_string(&t).unwrap();
        let deserialized: Temperature = serde_json::from_str(&serialized).unwrap();
        assert_eq!(t, deserialized);
    }
}
