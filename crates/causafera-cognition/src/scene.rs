use causafera_types::{
    AttentionTargetId, PerceivedObjectId, PerceptId, SelfAssociationId, SimulationTime,
    SubjectiveBodyPartId,
};
use thiserror::Error;

use crate::AttentionState;

pub const COGNITIVE_WEIGHT_SCALE: u32 = 1_000_000;
pub const MAX_SCENE_CUES: usize = 64;
pub const MAX_TRACKED_OBJECTS: usize = 32;
pub const MAX_SCENE_OBJECTS: usize = 16;
pub const MAX_BODY_SCHEMA_PARTS: usize = 16;
pub const MAX_SELF_ASSOCIATIONS: usize = 16;
pub const MAX_ACTIVE_SELF_ASSOCIATIONS: usize = 4;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CognitiveWeight(u32);

impl CognitiveWeight {
    pub const ZERO: Self = Self(0);

    pub fn new(raw: u32) -> Result<Self, SceneConfigError> {
        if raw > COGNITIVE_WEIGHT_SCALE {
            return Err(SceneConfigError::WeightOutOfRange { value: raw });
        }
        Ok(Self(raw))
    }

    pub const fn raw(self) -> u32 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AppearanceSignature(pub [u16; 4]);

impl AppearanceSignature {
    fn distance(self, other: Self) -> u32 {
        self.0
            .into_iter()
            .zip(other.0)
            .map(|(a, b)| u32::from(a.abs_diff(b)))
            .sum()
    }
}

/// Identity-free cognitive input produced after extractor bookkeeping is stripped.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PerceptualCue {
    pub percept: PerceptId,
    pub attention_target: AttentionTargetId,
    pub appearance: AppearanceSignature,
    pub relative_position: [i32; 3],
    pub strength: CognitiveWeight,
    pub time: SimulationTime,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BodySchemaPart {
    pub id: SubjectiveBodyPartId,
    pub relative_position: [i32; 3],
    pub extent: u32,
    pub mobility: CognitiveWeight,
    pub confidence: CognitiveWeight,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BodySchema {
    parts: [BodySchemaPart; MAX_BODY_SCHEMA_PARTS],
    len: u8,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BodySchemaSnapshot {
    pub parts: Vec<BodySchemaPart>,
}

impl BodySchema {
    pub const fn empty() -> Self {
        Self {
            parts: [BodySchemaPart {
                id: SubjectiveBodyPartId::new(0),
                relative_position: [0; 3],
                extent: 0,
                mobility: CognitiveWeight(0),
                confidence: CognitiveWeight(0),
            }; MAX_BODY_SCHEMA_PARTS],
            len: 0,
        }
    }

    pub fn replace(&mut self, mut parts: Vec<BodySchemaPart>) -> Result<(), SceneUpdateError> {
        if parts.len() > MAX_BODY_SCHEMA_PARTS {
            return Err(SceneUpdateError::BodySchemaCapacity);
        }
        parts.sort_by_key(|part| part.id);
        if parts.windows(2).any(|pair| pair[0].id == pair[1].id) {
            return Err(SceneUpdateError::DuplicateBodyPart);
        }
        self.len = parts.len() as u8;
        self.parts[..parts.len()].copy_from_slice(&parts);
        Ok(())
    }

    pub fn parts(&self) -> &[BodySchemaPart] {
        &self.parts[..self.len as usize]
    }

    pub fn export_snapshot(&self) -> BodySchemaSnapshot {
        BodySchemaSnapshot {
            parts: self.parts().to_vec(),
        }
    }

    pub fn import_snapshot(snapshot: BodySchemaSnapshot) -> Result<Self, SceneUpdateError> {
        let mut schema = Self::empty();
        schema.replace(snapshot.parts)?;
        Ok(schema)
    }
}

impl Default for BodySchema {
    fn default() -> Self {
        Self::empty()
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SelfAssociation {
    pub id: SelfAssociationId,
    pub strength: CognitiveWeight,
    pub supporting_percept: PerceptId,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SelfModel {
    associations: [SelfAssociation; MAX_SELF_ASSOCIATIONS],
    len: u8,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SelfModelSnapshot {
    pub associations: Vec<SelfAssociation>,
}

impl SelfModel {
    pub const fn empty() -> Self {
        Self {
            associations: [SelfAssociation {
                id: SelfAssociationId::new(0),
                strength: CognitiveWeight(0),
                supporting_percept: PerceptId::new(0),
            }; MAX_SELF_ASSOCIATIONS],
            len: 0,
        }
    }

    pub fn revise(&mut self, association: SelfAssociation) -> Result<(), SceneUpdateError> {
        if let Some(existing) = self.associations[..self.len as usize]
            .iter_mut()
            .find(|existing| existing.id == association.id)
        {
            *existing = association;
            return Ok(());
        }
        if self.len as usize == MAX_SELF_ASSOCIATIONS {
            return Err(SceneUpdateError::SelfModelCapacity);
        }
        let index = self.len as usize;
        self.associations[index] = association;
        self.len += 1;
        self.associations[..self.len as usize].sort_by_key(|value| value.id);
        Ok(())
    }

    pub fn associations(&self) -> &[SelfAssociation] {
        &self.associations[..self.len as usize]
    }

    pub fn export_snapshot(&self) -> SelfModelSnapshot {
        SelfModelSnapshot {
            associations: self.associations().to_vec(),
        }
    }

    pub fn import_snapshot(snapshot: SelfModelSnapshot) -> Result<Self, SceneUpdateError> {
        let mut model = Self::empty();
        for association in snapshot.associations {
            model.revise(association)?;
        }
        Ok(model)
    }
}

impl Default for SelfModel {
    fn default() -> Self {
        Self::empty()
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct TrackedObject {
    id: PerceivedObjectId,
    last_target: AttentionTargetId,
    appearance: AppearanceSignature,
    relative_position: [i32; 3],
    confidence: CognitiveWeight,
    last_seen: SimulationTime,
    supporting_percept: PerceptId,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TrackedObjectSnapshot {
    pub id: PerceivedObjectId,
    pub last_target: AttentionTargetId,
    pub appearance: AppearanceSignature,
    pub relative_position: [i32; 3],
    pub confidence: CognitiveWeight,
    pub last_seen: SimulationTime,
    pub supporting_percept: PerceptId,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SceneObject {
    pub id: PerceivedObjectId,
    pub appearance: AppearanceSignature,
    pub relative_position: [i32; 3],
    pub confidence: CognitiveWeight,
    pub supporting_percept: PerceptId,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SubjectiveScene {
    time: SimulationTime,
    objects: [SceneObject; MAX_SCENE_OBJECTS],
    object_len: u8,
    body_schema: BodySchema,
    active_self: [SelfAssociation; MAX_ACTIVE_SELF_ASSOCIATIONS],
    self_len: u8,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SubjectiveSceneSnapshot {
    pub time: SimulationTime,
    pub objects: Vec<SceneObject>,
    pub body_schema: BodySchemaSnapshot,
    pub active_self: Vec<SelfAssociation>,
}

impl SubjectiveScene {
    pub fn objects(&self) -> &[SceneObject] {
        &self.objects[..self.object_len as usize]
    }

    pub const fn time(&self) -> SimulationTime {
        self.time
    }

    pub const fn body_schema(&self) -> &BodySchema {
        &self.body_schema
    }

    pub fn active_self(&self) -> &[SelfAssociation] {
        &self.active_self[..self.self_len as usize]
    }

    pub fn export_snapshot(&self) -> SubjectiveSceneSnapshot {
        SubjectiveSceneSnapshot {
            time: self.time,
            objects: self.objects().to_vec(),
            body_schema: self.body_schema.export_snapshot(),
            active_self: self.active_self().to_vec(),
        }
    }

    pub fn import_snapshot(snapshot: SubjectiveSceneSnapshot) -> Result<Self, SceneUpdateError> {
        if snapshot.objects.len() > MAX_SCENE_OBJECTS
            || snapshot.active_self.len() > MAX_ACTIVE_SELF_ASSOCIATIONS
        {
            return Err(SceneUpdateError::TooManyCues {
                count: snapshot.objects.len(),
            });
        }
        let mut objects = snapshot.objects;
        objects.sort_by_key(|object| object.id);
        if objects.windows(2).any(|pair| pair[0].id == pair[1].id) {
            return Err(SceneUpdateError::DuplicatePercept);
        }
        let mut scene_objects = [SceneObject::default(); MAX_SCENE_OBJECTS];
        scene_objects[..objects.len()].copy_from_slice(&objects);
        let mut active_self = [SelfAssociation::default(); MAX_ACTIVE_SELF_ASSOCIATIONS];
        active_self[..snapshot.active_self.len()].copy_from_slice(&snapshot.active_self);
        Ok(Self {
            time: snapshot.time,
            objects: scene_objects,
            object_len: objects.len() as u8,
            body_schema: BodySchema::import_snapshot(snapshot.body_schema)?,
            active_self,
            self_len: snapshot.active_self.len() as u8,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SceneContinuityState {
    tracked: [TrackedObject; MAX_TRACKED_OBJECTS],
    tracked_len: u8,
    next_object_id: u64,
    last_update: Option<SimulationTime>,
    appearance_tolerance: u32,
    position_tolerance: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SceneContinuitySnapshot {
    pub tracked: Vec<TrackedObjectSnapshot>,
    pub next_object_id: u64,
    pub last_update: Option<SimulationTime>,
    pub appearance_tolerance: u32,
    pub position_tolerance: u32,
}

impl SceneContinuityState {
    pub const fn new(appearance_tolerance: u32, position_tolerance: u32) -> Self {
        Self {
            tracked: [TrackedObject {
                id: PerceivedObjectId::new(0),
                last_target: AttentionTargetId::new(0),
                appearance: AppearanceSignature([0; 4]),
                relative_position: [0; 3],
                confidence: CognitiveWeight(0),
                last_seen: SimulationTime::new(0),
                supporting_percept: PerceptId::new(0),
            }; MAX_TRACKED_OBJECTS],
            tracked_len: 0,
            next_object_id: 1,
            last_update: None,
            appearance_tolerance,
            position_tolerance,
        }
    }

    pub fn export_snapshot(&self) -> SceneContinuitySnapshot {
        SceneContinuitySnapshot {
            tracked: self.tracked[..self.tracked_len as usize]
                .iter()
                .map(|object| TrackedObjectSnapshot {
                    id: object.id,
                    last_target: object.last_target,
                    appearance: object.appearance,
                    relative_position: object.relative_position,
                    confidence: object.confidence,
                    last_seen: object.last_seen,
                    supporting_percept: object.supporting_percept,
                })
                .collect(),
            next_object_id: self.next_object_id,
            last_update: self.last_update,
            appearance_tolerance: self.appearance_tolerance,
            position_tolerance: self.position_tolerance,
        }
    }

    pub fn import_snapshot(snapshot: SceneContinuitySnapshot) -> Result<Self, SceneUpdateError> {
        if snapshot.tracked.len() > MAX_TRACKED_OBJECTS || snapshot.next_object_id == 0 {
            return Err(SceneUpdateError::IdentifierExhausted);
        }
        let mut state = Self::new(snapshot.appearance_tolerance, snapshot.position_tolerance);
        state.tracked_len = snapshot.tracked.len() as u8;
        state.next_object_id = snapshot.next_object_id;
        state.last_update = snapshot.last_update;
        for (index, object) in snapshot.tracked.into_iter().enumerate() {
            state.tracked[index] = TrackedObject {
                id: object.id,
                last_target: object.last_target,
                appearance: object.appearance,
                relative_position: object.relative_position,
                confidence: object.confidence,
                last_seen: object.last_seen,
                supporting_percept: object.supporting_percept,
            };
        }
        Ok(state)
    }

    pub fn reconstruct(
        &mut self,
        time: SimulationTime,
        cues: &[PerceptualCue],
        attention: &AttentionState,
        body_schema: &BodySchema,
        self_model: &SelfModel,
    ) -> Result<SubjectiveScene, SceneUpdateError> {
        if self.last_update.is_some_and(|last| time < last) {
            return Err(SceneUpdateError::TimeRegression);
        }
        if cues.len() > MAX_SCENE_CUES {
            return Err(SceneUpdateError::TooManyCues { count: cues.len() });
        }
        let mut cues = cues.to_vec();
        cues.sort_by_key(|cue| cue.percept);
        if cues
            .windows(2)
            .any(|pair| pair[0].percept == pair[1].percept)
        {
            return Err(SceneUpdateError::DuplicatePercept);
        }
        if cues.iter().any(|cue| cue.time != time) {
            return Err(SceneUpdateError::CueTimeMismatch);
        }

        let mut assignments = Vec::with_capacity(cues.len());
        for cue in cues.iter().copied() {
            let index = match self.best_match(cue) {
                Some(index) => index,
                None => self.allocate(cue)?,
            };
            let tracked = &mut self.tracked[index];
            tracked.last_target = cue.attention_target;
            tracked.appearance = cue.appearance;
            tracked.relative_position = cue.relative_position;
            tracked.confidence = cue.strength;
            tracked.last_seen = time;
            tracked.supporting_percept = cue.percept;
            assignments.push((cue.attention_target, *tracked));
        }

        let mut objects = Vec::new();
        for focus_index in 0..attention.len() {
            let focus = attention
                .focus(focus_index)
                .expect("focus index is bounded");
            if let Some((_, tracked)) = assignments
                .iter()
                .find(|(target, _)| *target == focus.target)
            {
                if !objects
                    .iter()
                    .any(|value: &SceneObject| value.id == tracked.id)
                {
                    objects.push(SceneObject {
                        id: tracked.id,
                        appearance: tracked.appearance,
                        relative_position: tracked.relative_position,
                        confidence: tracked.confidence,
                        supporting_percept: tracked.supporting_percept,
                    });
                }
            }
        }
        objects.sort_by_key(|object| object.id);
        objects.truncate(MAX_SCENE_OBJECTS);
        let mut scene_objects = [SceneObject::default(); MAX_SCENE_OBJECTS];
        scene_objects[..objects.len()].copy_from_slice(&objects);
        let mut selected_self = self_model.associations().to_vec();
        selected_self.sort_by(|a, b| b.strength.cmp(&a.strength).then_with(|| a.id.cmp(&b.id)));
        selected_self.truncate(MAX_ACTIVE_SELF_ASSOCIATIONS);
        let active_self = [SelfAssociation::default(); MAX_ACTIVE_SELF_ASSOCIATIONS];
        let mut scene = SubjectiveScene {
            time,
            objects: scene_objects,
            object_len: objects.len() as u8,
            body_schema: body_schema.clone(),
            active_self,
            self_len: selected_self.len() as u8,
        };
        scene.active_self[..selected_self.len()].copy_from_slice(&selected_self);
        self.last_update = Some(time);
        Ok(scene)
    }

    fn best_match(&self, cue: PerceptualCue) -> Option<usize> {
        self.tracked[..self.tracked_len as usize]
            .iter()
            .enumerate()
            .filter_map(|(index, object)| {
                let appearance = object.appearance.distance(cue.appearance);
                let position = position_distance(object.relative_position, cue.relative_position);
                let same_target = object.last_target == cue.attention_target;
                ((appearance <= self.appearance_tolerance && position <= self.position_tolerance)
                    || (same_target
                        && appearance <= self.appearance_tolerance / 2
                        && position <= self.position_tolerance.saturating_mul(2)))
                .then_some((appearance.saturating_add(position), object.id, index))
            })
            .min()
            .map(|(_, _, index)| index)
    }

    fn allocate(&mut self, cue: PerceptualCue) -> Result<usize, SceneUpdateError> {
        let index = if self.tracked_len as usize == MAX_TRACKED_OBJECTS {
            self.tracked[..self.tracked_len as usize]
                .iter()
                .enumerate()
                .min_by_key(|(_, object)| (object.last_seen, object.confidence, object.id))
                .map(|(index, _)| index)
                .expect("tracker capacity is non-zero")
        } else {
            let index = self.tracked_len as usize;
            self.tracked_len += 1;
            index
        };
        let allocated_id = self.next_object_id;
        self.next_object_id = self
            .next_object_id
            .checked_add(1)
            .ok_or(SceneUpdateError::IdentifierExhausted)?;
        self.tracked[index] = TrackedObject {
            id: PerceivedObjectId::new(allocated_id),
            last_target: cue.attention_target,
            appearance: cue.appearance,
            relative_position: cue.relative_position,
            confidence: cue.strength,
            last_seen: cue.time,
            supporting_percept: cue.percept,
        };
        Ok(index)
    }
}

fn position_distance(a: [i32; 3], b: [i32; 3]) -> u32 {
    a.into_iter().zip(b).fold(0, |distance, (left, right)| {
        distance.saturating_add(left.abs_diff(right))
    })
}

#[derive(Clone, Copy, Debug, Error, PartialEq, Eq)]
pub enum SceneConfigError {
    #[error("cognitive weight {value} exceeds {COGNITIVE_WEIGHT_SCALE}")]
    WeightOutOfRange { value: u32 },
}

#[derive(Clone, Copy, Debug, Error, PartialEq, Eq)]
pub enum SceneUpdateError {
    #[error("scene update time regressed")]
    TimeRegression,
    #[error("scene cue count {count} exceeds {MAX_SCENE_CUES}")]
    TooManyCues { count: usize },
    #[error("percept identifiers must be unique")]
    DuplicatePercept,
    #[error("cue time differs from scene time")]
    CueTimeMismatch,
    #[error("body schema capacity exceeded")]
    BodySchemaCapacity,
    #[error("subjective body part identifiers must be unique")]
    DuplicateBodyPart,
    #[error("self-model capacity exceeded")]
    SelfModelCapacity,
    #[error("perceived-object identifier space is exhausted")]
    IdentifierExhausted,
}

#[cfg(test)]
mod tests {
    use crate::{AttentionCandidate, AttentionConfig, AttentionWeight};

    use super::*;

    fn cue(percept: u64, target: u64, appearance: u16, x: i32, time: u64) -> PerceptualCue {
        PerceptualCue {
            percept: PerceptId::new(percept),
            attention_target: AttentionTargetId::new(target),
            appearance: AppearanceSignature([appearance; 4]),
            relative_position: [x, 0, 0],
            strength: CognitiveWeight::new(800_000).unwrap(),
            time: SimulationTime::new(time),
        }
    }

    fn attention(time: u64, cues: &[PerceptualCue]) -> AttentionState {
        let mut attention = AttentionState::new(
            AttentionConfig::new(
                8,
                AttentionWeight::new(0).unwrap(),
                AttentionWeight::new(0).unwrap(),
            )
            .unwrap(),
        );
        let candidates: Vec<_> = cues
            .iter()
            .map(|cue| {
                AttentionCandidate::new(
                    cue.attention_target,
                    AttentionWeight::new(cue.strength.raw()).unwrap(),
                    cue.percept,
                )
            })
            .collect();
        attention
            .update(SimulationTime::new(time), &candidates)
            .unwrap();
        attention
    }

    #[test]
    fn reconstruction_is_input_order_independent_and_bounded_by_attention() {
        let cues = [cue(2, 20, 10, 2, 1), cue(1, 10, 50, 8, 1)];
        let attention = attention(1, &cues);
        let mut a = SceneContinuityState::new(8, 4);
        let mut b = SceneContinuityState::new(8, 4);
        let scene_a = a
            .reconstruct(
                SimulationTime::new(1),
                &cues,
                &attention,
                &BodySchema::default(),
                &SelfModel::default(),
            )
            .unwrap();
        let scene_b = b
            .reconstruct(
                SimulationTime::new(1),
                &[cues[1], cues[0]],
                &attention,
                &BodySchema::default(),
                &SelfModel::default(),
            )
            .unwrap();
        assert_eq!(scene_a, scene_b);
        assert_eq!(scene_a.objects().len(), 2);
    }

    #[test]
    fn similar_cues_can_merge_and_discontinuous_cues_split() {
        let mut state = SceneContinuityState::new(20, 5);
        let first = cue(1, 1, 10, 0, 1);
        let scene = state
            .reconstruct(
                SimulationTime::new(1),
                &[first],
                &attention(1, &[first]),
                &BodySchema::default(),
                &SelfModel::default(),
            )
            .unwrap();
        let first_id = scene.objects()[0].id;
        let similar = cue(2, 2, 12, 2, 2);
        let scene = state
            .reconstruct(
                SimulationTime::new(2),
                &[similar],
                &attention(2, &[similar]),
                &BodySchema::default(),
                &SelfModel::default(),
            )
            .unwrap();
        assert_eq!(scene.objects()[0].id, first_id);
        let discontinuous = cue(3, 2, 100, 100, 3);
        let scene = state
            .reconstruct(
                SimulationTime::new(3),
                &[discontinuous],
                &attention(3, &[discontinuous]),
                &BodySchema::default(),
                &SelfModel::default(),
            )
            .unwrap();
        assert_ne!(scene.objects()[0].id, first_id);
    }
}
