use serde::{Deserialize, Serialize};

/// A generic perceptual feature relation extracted from raw Ground Truth state.
///
/// These relations carry no semantic labels. An agent's cognitive system may
/// later group similar features into subjective concepts, but the engine itself
/// never treats `FeatureRelation` as meaning anything beyond a structural pattern.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FeatureRelation {
    /// A scalar value changed over time or across samples.
    Change,
    /// A scalar magnitude was observed.
    Magnitude,
    /// A directional vector was observed.
    Direction,
    /// Variance or spread across a set of values.
    Variance,
    /// Repeating pattern with a characteristic interval.
    Periodicity,
    /// Multiple sources aligned in time or phase.
    Synchrony,
    /// The same pattern appeared again after a gap.
    Recurrence,
    /// A span of time between two events or states.
    Duration,
    /// Spatial positioning relative to another entity.
    SpatialRelation,
    /// Temporal ordering relative to another event.
    TemporalRelation,
    /// Two patterns present at the same time or place.
    CoOccurrence,
    /// Structural shape or topology match.
    StructuralSimilarity,
    /// Scalar difference between two measurements.
    RelativeDifference,
    /// Pattern match across ordered sequences.
    SequenceSimilarity,
}

/// The value associated with a generic feature.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum FeatureValue {
    /// A continuous scalar value.
    Scalar(f64),
    /// A 3-D directional vector.
    Direction(Direction3D),
    /// A binned frequency band index (0–255).
    FrequencyBand(u8),
    /// A binned magnitude band index (0–255).
    MagnitudeBand(u8),
}

/// Persistence of a detected feature.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Persistence {
    /// Detected for a single instant.
    Fleeting,
    /// Lasted a few ticks.
    Brief,
    /// Lasted a noticeable duration.
    Moderate,
    /// Stable over many ticks.
    Persistent,
    /// Highly stable, unlikely to change without external cause.
    High,
}

/// A 3-D unit direction vector.
///
/// Components are stored as `f64` and are expected to be normalized by the
/// producer. The engine does not enforce normalization at the type level.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Direction3D {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Direction3D {
    pub const fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }

    /// Returns the squared Euclidean length.
    pub fn length_squared(self) -> f64 {
        self.x
            .mul_add(self.x, self.y.mul_add(self.y, self.z * self.z))
    }
}

/// A generic perceptual feature detected by an extractor.
///
/// `Feature` ties a `FeatureRelation`, a `FeatureValue`, a `Persistence`, and
/// the identity of the target entity being observed. It carries no semantic
/// label; labels are produced downstream by the Explanation Engine.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Feature {
    pub id: crate::FeatureId,
    pub target_id: crate::EntityId,
    pub relation: FeatureRelation,
    pub value: FeatureValue,
    pub persistence: Persistence,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{EntityId, FeatureId};

    #[test]
    fn feature_relation_roundtrip() {
        let relations = [
            FeatureRelation::Change,
            FeatureRelation::Magnitude,
            FeatureRelation::Direction,
            FeatureRelation::Variance,
            FeatureRelation::Periodicity,
            FeatureRelation::Synchrony,
            FeatureRelation::Recurrence,
            FeatureRelation::Duration,
            FeatureRelation::SpatialRelation,
            FeatureRelation::TemporalRelation,
            FeatureRelation::CoOccurrence,
            FeatureRelation::StructuralSimilarity,
            FeatureRelation::RelativeDifference,
            FeatureRelation::SequenceSimilarity,
        ];
        for r in &relations {
            let serialized = serde_json::to_string(r).unwrap();
            let deserialized: FeatureRelation = serde_json::from_str(&serialized).unwrap();
            assert_eq!(*r, deserialized);
        }
    }

    #[test]
    fn feature_value_roundtrip() {
        let values = [
            FeatureValue::Scalar(std::f64::consts::PI),
            FeatureValue::Direction(Direction3D::new(1.0, 0.0, 0.0)),
            FeatureValue::FrequencyBand(7),
            FeatureValue::MagnitudeBand(42),
        ];
        for v in &values {
            let serialized = serde_json::to_string(v).unwrap();
            let deserialized: FeatureValue = serde_json::from_str(&serialized).unwrap();
            assert_eq!(*v, deserialized);
        }
    }

    #[test]
    fn direction3d_length_squared() {
        let d = Direction3D::new(3.0, 4.0, 0.0);
        assert!((d.length_squared() - 25.0).abs() < f64::EPSILON);
    }

    #[test]
    fn persistence_roundtrip() {
        let pers = [
            Persistence::Fleeting,
            Persistence::Brief,
            Persistence::Moderate,
            Persistence::Persistent,
            Persistence::High,
        ];
        for p in &pers {
            let serialized = serde_json::to_string(p).unwrap();
            let deserialized: Persistence = serde_json::from_str(&serialized).unwrap();
            assert_eq!(*p, deserialized);
        }
    }

    #[test]
    fn feature_struct_creation() {
        let f = Feature {
            id: FeatureId::new(1),
            target_id: EntityId::new(2),
            relation: FeatureRelation::Change,
            value: FeatureValue::Scalar(1.5),
            persistence: Persistence::Brief,
        };
        assert_eq!(f.id, FeatureId::new(1));
        assert_eq!(f.target_id, EntityId::new(2));
        assert_eq!(f.relation, FeatureRelation::Change);
    }
}
