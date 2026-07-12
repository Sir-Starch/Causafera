use ontopolis_types::{BodySegmentId, Orientation};
use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;

/// Physical length of a body segment, in millimetres.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SegmentLengthMm(u32);

impl SegmentLengthMm {
    pub const fn new(millimetres: u32) -> Self {
        Self(millimetres)
    }

    pub const fn millimetres(self) -> u32 {
        self.0
    }
}

/// Property-based angular limits for one structural connection.
///
/// Limits are inclusive yaw, pitch, and roll bounds in radians. This is a
/// physical constraint, not a named anatomical joint classification.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Joint {
    lower: Orientation,
    upper: Orientation,
}

impl Joint {
    pub const fn new(lower: Orientation, upper: Orientation) -> Self {
        Self { lower, upper }
    }

    pub const fn lower(self) -> Orientation {
        self.lower
    }

    pub const fn upper(self) -> Orientation {
        self.upper
    }

    fn contains(self, orientation: Orientation) -> bool {
        self.lower.yaw <= orientation.yaw
            && orientation.yaw <= self.upper.yaw
            && self.lower.pitch <= orientation.pitch
            && orientation.pitch <= self.upper.pitch
            && self.lower.roll <= orientation.roll
            && orientation.roll <= self.upper.roll
    }
}

/// Value view of one authoritative body segment.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BodySegment {
    pub id: BodySegmentId,
    pub parent: Option<BodySegmentId>,
    pub joint: Option<Joint>,
    pub length: SegmentLengthMm,
    pub orientation: Orientation,
}

impl BodySegment {
    pub const fn new(
        id: BodySegmentId,
        parent: Option<BodySegmentId>,
        joint: Option<Joint>,
        length: SegmentLengthMm,
        orientation: Orientation,
    ) -> Self {
        Self {
            id,
            parent,
            joint,
            length,
            orientation,
        }
    }
}

/// Invalid biological body-segment structure.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BodyStructureError {
    Empty,
    FieldLengthMismatch {
        ids: usize,
        parents: usize,
        joints: usize,
        lengths: usize,
        orientations: usize,
    },
    DuplicateSegmentId {
        index: usize,
        id: BodySegmentId,
    },
    RootCount {
        actual: usize,
    },
    RootHasJoint {
        index: usize,
    },
    ConnectedSegmentMissingJoint {
        index: usize,
    },
    UnknownOrUnorderedParent {
        index: usize,
        parent: BodySegmentId,
    },
    NonPositiveLength {
        index: usize,
    },
    NonFiniteOrientation {
        index: usize,
    },
    NonFiniteJointLimit {
        index: usize,
    },
    InvertedJointLimit {
        index: usize,
        component: usize,
    },
    OrientationOutsideJointLimits {
        index: usize,
    },
}

impl fmt::Display for BodyStructureError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => write!(
                formatter,
                "body structure must contain at least one segment"
            ),
            Self::FieldLengthMismatch {
                ids,
                parents,
                joints,
                lengths,
                orientations,
            } => write!(
                formatter,
                "body structure fields must have equal lengths (ids {ids}, parents {parents}, joints {joints}, lengths {lengths}, orientations {orientations})"
            ),
            Self::DuplicateSegmentId { index, id } => {
                write!(formatter, "body segment {index} duplicates segment ID {id}")
            }
            Self::RootCount { actual } => {
                write!(
                    formatter,
                    "body structure must contain exactly one root, found {actual}"
                )
            }
            Self::RootHasJoint { index } => {
                write!(
                    formatter,
                    "root body segment {index} cannot have a parent joint"
                )
            }
            Self::ConnectedSegmentMissingJoint { index } => write!(
                formatter,
                "connected body segment {index} must define its parent joint"
            ),
            Self::UnknownOrUnorderedParent { index, parent } => write!(
                formatter,
                "body segment {index} references parent {parent} that does not precede it"
            ),
            Self::NonPositiveLength { index } => {
                write!(formatter, "body segment {index} must have positive length")
            }
            Self::NonFiniteOrientation { index } => {
                write!(
                    formatter,
                    "body segment {index} has a non-finite orientation"
                )
            }
            Self::NonFiniteJointLimit { index } => {
                write!(
                    formatter,
                    "body segment {index} has a non-finite joint limit"
                )
            }
            Self::InvertedJointLimit { index, component } => write!(
                formatter,
                "body segment {index} has inverted joint limit component {component}"
            ),
            Self::OrientationOutsideJointLimits { index } => write!(
                formatter,
                "body segment {index} orientation lies outside its joint limits"
            ),
        }
    }
}

impl Error for BodyStructureError {}

/// Dense, canonically ordered structure-of-arrays body-segment state.
///
/// Segment order is topological: the single root comes first and every parent
/// precedes its children. The order is also the canonical deterministic
/// iteration order.
#[derive(Clone, Debug, PartialEq)]
pub struct BodyStructure {
    ids: Vec<BodySegmentId>,
    parents: Vec<Option<BodySegmentId>>,
    joints: Vec<Option<Joint>>,
    lengths: Vec<SegmentLengthMm>,
    orientations: Vec<Orientation>,
}

impl BodyStructure {
    pub fn from_segments(segments: Vec<BodySegment>) -> Result<Self, BodyStructureError> {
        let mut ids = Vec::with_capacity(segments.len());
        let mut parents = Vec::with_capacity(segments.len());
        let mut joints = Vec::with_capacity(segments.len());
        let mut lengths = Vec::with_capacity(segments.len());
        let mut orientations = Vec::with_capacity(segments.len());

        for segment in segments {
            ids.push(segment.id);
            parents.push(segment.parent);
            joints.push(segment.joint);
            lengths.push(segment.length);
            orientations.push(segment.orientation);
        }

        Self::from_fields(ids, parents, joints, lengths, orientations)
    }

    pub fn from_fields(
        ids: Vec<BodySegmentId>,
        parents: Vec<Option<BodySegmentId>>,
        joints: Vec<Option<Joint>>,
        lengths: Vec<SegmentLengthMm>,
        orientations: Vec<Orientation>,
    ) -> Result<Self, BodyStructureError> {
        let field_length = ids.len();
        if parents.len() != field_length
            || joints.len() != field_length
            || lengths.len() != field_length
            || orientations.len() != field_length
        {
            return Err(BodyStructureError::FieldLengthMismatch {
                ids: field_length,
                parents: parents.len(),
                joints: joints.len(),
                lengths: lengths.len(),
                orientations: orientations.len(),
            });
        }
        if ids.is_empty() {
            return Err(BodyStructureError::Empty);
        }

        let mut prior_ids = BTreeSet::new();
        let mut root_count = 0;

        for index in 0..field_length {
            let id = ids[index];
            if !prior_ids.insert(id) {
                return Err(BodyStructureError::DuplicateSegmentId { index, id });
            }

            match parents[index] {
                None => {
                    root_count += 1;
                    if joints[index].is_some() {
                        return Err(BodyStructureError::RootHasJoint { index });
                    }
                }
                Some(parent) => {
                    if !prior_ids.contains(&parent) {
                        return Err(BodyStructureError::UnknownOrUnorderedParent { index, parent });
                    }
                    if joints[index].is_none() {
                        return Err(BodyStructureError::ConnectedSegmentMissingJoint { index });
                    }
                }
            }

            if lengths[index].millimetres() == 0 {
                return Err(BodyStructureError::NonPositiveLength { index });
            }
            if !orientation_is_finite(orientations[index]) {
                return Err(BodyStructureError::NonFiniteOrientation { index });
            }

            if let Some(joint) = joints[index] {
                validate_joint(index, joint)?;
                if !joint.contains(orientations[index]) {
                    return Err(BodyStructureError::OrientationOutsideJointLimits { index });
                }
            }
        }

        if root_count != 1 {
            return Err(BodyStructureError::RootCount { actual: root_count });
        }

        Ok(Self {
            ids,
            parents,
            joints,
            lengths,
            orientations,
        })
    }

    pub fn len(&self) -> usize {
        self.ids.len()
    }

    pub fn is_empty(&self) -> bool {
        self.ids.is_empty()
    }

    pub fn ids(&self) -> &[BodySegmentId] {
        &self.ids
    }

    pub fn parents(&self) -> &[Option<BodySegmentId>] {
        &self.parents
    }

    pub fn joints(&self) -> &[Option<Joint>] {
        &self.joints
    }

    pub fn lengths(&self) -> &[SegmentLengthMm] {
        &self.lengths
    }

    pub fn orientations(&self) -> &[Orientation] {
        &self.orientations
    }

    pub fn index_of(&self, id: BodySegmentId) -> Option<usize> {
        self.ids.iter().position(|candidate| *candidate == id)
    }

    pub fn segment(&self, index: usize) -> Option<BodySegment> {
        Some(BodySegment {
            id: *self.ids.get(index)?,
            parent: self.parents[index],
            joint: self.joints[index],
            length: self.lengths[index],
            orientation: self.orientations[index],
        })
    }
}

fn orientation_is_finite(orientation: Orientation) -> bool {
    orientation.yaw.is_finite() && orientation.pitch.is_finite() && orientation.roll.is_finite()
}

fn validate_joint(index: usize, joint: Joint) -> Result<(), BodyStructureError> {
    let lower = joint.lower();
    let upper = joint.upper();
    if !orientation_is_finite(lower) || !orientation_is_finite(upper) {
        return Err(BodyStructureError::NonFiniteJointLimit { index });
    }

    for (component, (lower, upper)) in [
        (lower.yaw, upper.yaw),
        (lower.pitch, upper.pitch),
        (lower.roll, upper.roll),
    ]
    .into_iter()
    .enumerate()
    {
        if lower > upper {
            return Err(BodyStructureError::InvertedJointLimit { index, component });
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn orientation(value: f64) -> Orientation {
        Orientation::new(value, value, value)
    }

    fn joint() -> Joint {
        Joint::new(orientation(-1.0), orientation(1.0))
    }

    fn valid_segments() -> Vec<BodySegment> {
        vec![
            BodySegment::new(
                BodySegmentId::new(10),
                None,
                None,
                SegmentLengthMm::new(500),
                orientation(0.0),
            ),
            BodySegment::new(
                BodySegmentId::new(20),
                Some(BodySegmentId::new(10)),
                Some(joint()),
                SegmentLengthMm::new(300),
                orientation(0.25),
            ),
            BodySegment::new(
                BodySegmentId::new(30),
                Some(BodySegmentId::new(10)),
                Some(joint()),
                SegmentLengthMm::new(200),
                orientation(-0.25),
            ),
        ]
    }

    #[test]
    fn valid_structure_preserves_canonical_order_and_value_access() {
        let structure = BodyStructure::from_segments(valid_segments()).unwrap();

        assert_eq!(structure.len(), 3);
        assert!(!structure.is_empty());
        assert_eq!(structure.index_of(BodySegmentId::new(20)), Some(1));
        assert_eq!(structure.segment(1), Some(valid_segments()[1]));
        assert_eq!(
            structure.ids(),
            &[
                BodySegmentId::new(10),
                BodySegmentId::new(20),
                BodySegmentId::new(30),
            ]
        );
        assert!(structure.segment(3).is_none());
    }

    #[test]
    fn empty_and_mismatched_fields_are_rejected() {
        assert_eq!(
            BodyStructure::from_segments(Vec::new()),
            Err(BodyStructureError::Empty)
        );

        assert_eq!(
            BodyStructure::from_fields(
                vec![BodySegmentId::new(1)],
                Vec::new(),
                vec![None],
                vec![SegmentLengthMm::new(1)],
                vec![orientation(0.0)],
            ),
            Err(BodyStructureError::FieldLengthMismatch {
                ids: 1,
                parents: 0,
                joints: 1,
                lengths: 1,
                orientations: 1,
            })
        );
    }

    #[test]
    fn duplicate_ids_are_rejected() {
        let mut segments = valid_segments();
        segments[1].id = segments[0].id;

        assert_eq!(
            BodyStructure::from_segments(segments),
            Err(BodyStructureError::DuplicateSegmentId {
                index: 1,
                id: BodySegmentId::new(10),
            })
        );
    }

    #[test]
    fn multiple_roots_are_rejected() {
        let mut segments = valid_segments();
        segments[1].parent = None;
        segments[1].joint = None;

        assert_eq!(
            BodyStructure::from_segments(segments),
            Err(BodyStructureError::RootCount { actual: 2 })
        );
    }

    #[test]
    fn parent_joint_consistency_is_enforced() {
        let mut root_joint = valid_segments();
        root_joint[0].joint = Some(joint());
        assert_eq!(
            BodyStructure::from_segments(root_joint),
            Err(BodyStructureError::RootHasJoint { index: 0 })
        );

        let mut missing_joint = valid_segments();
        missing_joint[1].joint = None;
        assert_eq!(
            BodyStructure::from_segments(missing_joint),
            Err(BodyStructureError::ConnectedSegmentMissingJoint { index: 1 })
        );
    }

    #[test]
    fn parents_must_precede_children() {
        let mut segments = valid_segments();
        segments[1].parent = Some(BodySegmentId::new(30));

        assert_eq!(
            BodyStructure::from_segments(segments),
            Err(BodyStructureError::UnknownOrUnorderedParent {
                index: 1,
                parent: BodySegmentId::new(30),
            })
        );
    }

    #[test]
    fn lengths_and_orientations_must_be_physical() {
        let mut zero_length = valid_segments();
        zero_length[1].length = SegmentLengthMm::new(0);
        assert_eq!(
            BodyStructure::from_segments(zero_length),
            Err(BodyStructureError::NonPositiveLength { index: 1 })
        );

        let mut non_finite = valid_segments();
        non_finite[1].orientation = Orientation::new(f64::NAN, 0.0, 0.0);
        assert_eq!(
            BodyStructure::from_segments(non_finite),
            Err(BodyStructureError::NonFiniteOrientation { index: 1 })
        );
    }

    #[test]
    fn joint_limits_must_be_finite_and_ordered() {
        let mut non_finite = valid_segments();
        non_finite[1].joint = Some(Joint::new(
            Orientation::new(f64::NEG_INFINITY, -1.0, -1.0),
            orientation(1.0),
        ));
        assert_eq!(
            BodyStructure::from_segments(non_finite),
            Err(BodyStructureError::NonFiniteJointLimit { index: 1 })
        );

        let mut inverted = valid_segments();
        inverted[1].joint = Some(Joint::new(
            Orientation::new(-1.0, 2.0, -1.0),
            orientation(1.0),
        ));
        assert_eq!(
            BodyStructure::from_segments(inverted),
            Err(BodyStructureError::InvertedJointLimit {
                index: 1,
                component: 1,
            })
        );
    }

    #[test]
    fn connected_orientation_must_be_within_joint_limits() {
        let mut segments = valid_segments();
        segments[1].orientation = orientation(2.0);

        assert_eq!(
            BodyStructure::from_segments(segments),
            Err(BodyStructureError::OrientationOutsideJointLimits { index: 1 })
        );
    }

    #[test]
    fn identical_inputs_produce_identical_structure() {
        let first = BodyStructure::from_segments(valid_segments()).unwrap();
        let second = BodyStructure::from_segments(valid_segments()).unwrap();
        assert_eq!(first, second);
    }
}
