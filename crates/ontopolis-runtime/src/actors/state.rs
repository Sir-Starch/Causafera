use std::collections::BTreeMap;

use ontopolis_cognition::{
    AppearanceSignature, AttentionConfig, AttentionState, AttentionWeight, BodySchema,
    BodySchemaPart, BodySchemaSnapshot, CognitiveWeight, SceneContinuitySnapshot,
    SceneContinuityState, SceneObject, SelfAssociation, SelfModel, SelfModelSnapshot,
    SubjectiveScene as CognitionScene, SubjectiveSceneSnapshot as CognitionSceneSnapshot,
};
use ontopolis_types::{
    AngularVelocity, AttentionTargetId, FeatureRelation, FeatureValue, LocalCoord, Orientation,
    PerceivedObjectId, PerceptId, SelfAssociationId, SimulationTime, SubjectiveBodyPartId, TraceId,
    Velocity, WorldCoord,
};
use thiserror::Error;

use super::{ACTOR_BASE_ENERGY, ActionKindId, ActorId, SensorKindId};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MinimalBodyState {
    pub position: WorldCoord,
    pub orientation: Orientation,
    pub velocity: Velocity,
    pub angular_velocity: AngularVelocity,
    pub energy: i64,
}

impl MinimalBodyState {
    pub const fn stationary(position: WorldCoord, energy: i64) -> Self {
        Self {
            position,
            orientation: Orientation::new(0.0, 0.0, 0.0),
            velocity: Velocity::new(0.0, 0.0, 0.0),
            angular_velocity: AngularVelocity::new(0.0, 0.0, 0.0),
            energy,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SensorAperture {
    pub position: LocalCoord,
    pub range: u8,
    pub kind: SensorKindId,
}

impl SensorAperture {
    pub const fn new(position: LocalCoord, range: u8, kind: SensorKindId) -> Self {
        Self {
            position,
            range,
            kind,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GenericFeature {
    pub percept: PerceptId,
    pub attention_target: AttentionTargetId,
    pub relation: FeatureRelation,
    pub value: FeatureValue,
    pub appearance: AppearanceSignature,
    pub relative_position: [i32; 3],
    pub strength: CognitiveWeight,
    pub time: SimulationTime,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PerceivedSelf {
    pub energy_band: u8,
    pub motion_band: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActiveGoal {
    pub action_kind: ActionKindId,
    pub target: SubjectiveTarget,
    pub urgency: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SubjectiveScene {
    pub perceived_self: PerceivedSelf,
    pub objects: Vec<SceneObject>,
    pub body_schema: BodySchema,
    pub active_goals: Vec<ActiveGoal>,
    pub inner: CognitionScene,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SubjectiveTarget {
    SelfBody,
    Object(PerceivedObjectId),
    Relative([i32; 3]),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActionProposal {
    pub action_kind: ActionKindId,
    pub target: SubjectiveTarget,
    pub intensity: i64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActionValidationResult {
    Valid {
        trace: TraceId,
    },
    Invalid {
        cause: ActionRejection,
        trace: TraceId,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActionRejection {
    MissingSubjectiveTarget,
    OutOfBounds,
    InsufficientEnergy,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ActorState {
    pub body: MinimalBodyState,
    pub sensors: Vec<SensorAperture>,
    pub features: Vec<GenericFeature>,
    pub subjective_scene: Option<SubjectiveScene>,
    pub proposals: Vec<ActionProposal>,
    pub validation_results: Vec<ActionValidationResult>,
    pub(super) continuity: SceneContinuityState,
    pub(super) attention: AttentionState,
    pub(super) body_schema: BodySchema,
    pub(super) self_model: SelfModel,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ActorObjectiveSnapshot {
    pub body: MinimalBodyState,
    pub sensors: Vec<SensorAperture>,
    pub features: Vec<GenericFeature>,
    pub proposals: Vec<ActionProposal>,
    pub validation_results: Vec<ActionValidationResult>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActorSubjectiveSnapshot {
    pub subjective_scene: Option<SubjectiveSceneSnapshot>,
    pub continuity: SceneContinuitySnapshot,
    pub attention: ontopolis_cognition::AttentionStateSnapshot,
    pub body_schema: BodySchemaSnapshot,
    pub self_model: SelfModelSnapshot,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SubjectiveSceneSnapshot {
    pub perceived_self: PerceivedSelf,
    pub objects: Vec<SceneObject>,
    pub body_schema: BodySchemaSnapshot,
    pub active_goals: Vec<ActiveGoal>,
    pub inner: CognitionSceneSnapshot,
}

impl ActorState {
    pub fn new(
        body: MinimalBodyState,
        sensors: Vec<SensorAperture>,
    ) -> Result<Self, ActorInitError> {
        let attention = AttentionState::new(AttentionConfig::new(
            4,
            AttentionWeight::new(1)?,
            AttentionWeight::new(50_000)?,
        )?);
        let mut body_schema = BodySchema::default();
        body_schema.replace(vec![BodySchemaPart {
            id: SubjectiveBodyPartId::new(1),
            relative_position: [0, 0, 0],
            extent: 1,
            mobility: CognitiveWeight::new(800_000)?,
            confidence: CognitiveWeight::new(900_000)?,
        }])?;
        let mut self_model = SelfModel::default();
        self_model.revise(SelfAssociation {
            id: SelfAssociationId::new(1),
            strength: CognitiveWeight::new(700_000)?,
            supporting_percept: PerceptId::new(0),
        })?;
        Ok(Self {
            body,
            sensors,
            features: Vec::new(),
            subjective_scene: None,
            proposals: Vec::new(),
            validation_results: Vec::new(),
            continuity: SceneContinuityState::new(24, 12),
            attention,
            body_schema,
            self_model,
        })
    }

    pub fn export_objective_snapshot(&self) -> ActorObjectiveSnapshot {
        ActorObjectiveSnapshot {
            body: self.body,
            sensors: self.sensors.clone(),
            features: self.features.clone(),
            proposals: self.proposals.clone(),
            validation_results: self.validation_results.clone(),
        }
    }

    pub fn export_subjective_snapshot(&self) -> ActorSubjectiveSnapshot {
        ActorSubjectiveSnapshot {
            subjective_scene: self
                .subjective_scene
                .as_ref()
                .map(|scene| SubjectiveSceneSnapshot {
                    perceived_self: scene.perceived_self,
                    objects: scene.objects.clone(),
                    body_schema: scene.body_schema.export_snapshot(),
                    active_goals: scene.active_goals.clone(),
                    inner: scene.inner.export_snapshot(),
                }),
            continuity: self.continuity.export_snapshot(),
            attention: self.attention.export_snapshot(),
            body_schema: self.body_schema.export_snapshot(),
            self_model: self.self_model.export_snapshot(),
        }
    }

    pub fn import_snapshots(
        objective: ActorObjectiveSnapshot,
        subjective: ActorSubjectiveSnapshot,
    ) -> Result<Self, ActorInitError> {
        Ok(Self {
            body: objective.body,
            sensors: objective.sensors,
            features: objective.features,
            subjective_scene: match subjective.subjective_scene {
                Some(scene) => Some(SubjectiveScene {
                    perceived_self: scene.perceived_self,
                    objects: scene.objects,
                    body_schema: BodySchema::import_snapshot(scene.body_schema)?,
                    active_goals: scene.active_goals,
                    inner: CognitionScene::import_snapshot(scene.inner)?,
                }),
                None => None,
            },
            proposals: objective.proposals,
            validation_results: objective.validation_results,
            continuity: SceneContinuityState::import_snapshot(subjective.continuity)?,
            attention: AttentionState::import_snapshot(subjective.attention)?,
            body_schema: BodySchema::import_snapshot(subjective.body_schema)?,
            self_model: SelfModel::import_snapshot(subjective.self_model)?,
        })
    }
}

#[derive(Clone, Copy, Debug, Error, PartialEq, Eq)]
pub enum ActorInitError {
    #[error("actor attention configuration failed: {0}")]
    Attention(#[from] ontopolis_cognition::AttentionConfigError),
    #[error("actor scene configuration failed: {0}")]
    SceneConfig(#[from] ontopolis_cognition::SceneConfigError),
    #[error("actor scene initialization failed: {0}")]
    SceneUpdate(#[from] ontopolis_cognition::SceneUpdateError),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActorRuntimeConfig {
    pub actor_count: u8,
    pub sensor_count: u8,
    pub action_bounds: i64,
}

impl ActorRuntimeConfig {
    pub const fn none() -> Self {
        Self {
            actor_count: 0,
            sensor_count: 0,
            action_bounds: 0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActorPhysicalObject {
    pub object_key: u64,
    pub position: WorldCoord,
    pub magnitude: i64,
    pub accessible: bool,
    pub occluded: bool,
    pub trace: TraceId,
}

impl ActorPhysicalObject {
    pub const fn new(
        object_key: u64,
        position: WorldCoord,
        magnitude: i64,
        trace: TraceId,
    ) -> Self {
        Self {
            object_key,
            position,
            magnitude,
            accessible: true,
            occluded: false,
            trace,
        }
    }
}

pub fn fixture_actors(
    config: ActorRuntimeConfig,
) -> Result<BTreeMap<ActorId, ActorState>, ActorInitError> {
    let mut actors = BTreeMap::new();
    for index in 0..config.actor_count {
        let actor = ActorId::new(u64::from(index) + 1);
        actors.insert(
            actor,
            ActorState::new(
                MinimalBodyState::stationary(
                    WorldCoord::new(0, i64::from(index), 0),
                    ACTOR_BASE_ENERGY,
                ),
                fixture_sensors(config.sensor_count),
            )?,
        );
    }
    Ok(actors)
}

pub fn fixture_sensors(sensor_count: u8) -> Vec<SensorAperture> {
    (0..sensor_count)
        .map(|index| SensorAperture::new(LocalCoord::new(index, 0, 0), 8, SensorKindId::new(1)))
        .collect()
}
