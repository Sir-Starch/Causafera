use crate::*;
use causafera_core::*;

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use causafera_cognition::{AppearanceSignature, CognitiveWeight};
use causafera_perception::{
    AccessRange, GenericFeatureExtractor, MagnitudeQuantum, PhysicalSignal,
    SensorAperture as PhysicalSensorAperture, SignalMagnitude, acquire_signals,
};
use causafera_types::{
    AcquisitionId, AttentionTargetId, EntityId, FeatureId, FeatureValue, PerceptId, SensorId,
    SignalChannelId, SimulationTime, WorldCoord,
};

use super::{
    ACTOR_SIGNAL_CHANNEL, ActorId, ActorPhysicalObject, ActorState, GenericFeature, SensorKindId,
};

pub fn actor_perception_step(
    time: SimulationTime,
    actors: &mut BTreeMap<ActorId, ActorState>,
    objects: &BTreeMap<u64, ActorPhysicalObject>,
    material_signals: &[PhysicalSignal],
) -> Result<usize, causafera_perception::AcquisitionError> {
    let extractor = GenericFeatureExtractor::new(
        MagnitudeQuantum::new(1)
            .map_err(|_| causafera_perception::AcquisitionError::IdentifierExhausted)?,
    );
    let mut total = 0;
    for (actor_id, actor) in actors {
        let apertures = physical_apertures(*actor_id, actor);
        let mut signals = physical_signals(time, objects);
        signals.extend_from_slice(material_signals);
        let batch = acquire_signals(
            time,
            AcquisitionId::new(
                actor_id
                    .raw()
                    .saturating_mul(1_000_000)
                    .saturating_add(time.raw()),
            ),
            &apertures,
            &signals,
        )?;
        let extracted = extractor
            .extract(
                FeatureId::new(
                    actor_id
                        .raw()
                        .saturating_mul(1_000_000)
                        .saturating_add(time.raw()),
                ),
                batch.samples(),
            )
            .map_err(|_| causafera_perception::AcquisitionError::IdentifierExhausted)?;
        actor.features = extracted
            .features()
            .iter()
            .zip(batch.samples().iter().cycle())
            .map(|(feature, sample)| {
                generic_feature_from_extracted(time, feature, sample.relative_position())
            })
            .collect();
        total += actor.features.len();
    }
    Ok(total)
}

fn physical_apertures(
    actor_id: ActorId,
    actor: &ActorState,
) -> Vec<PhysicalSensorAperture> {
    actor
        .sensors
        .iter()
        .enumerate()
        .map(|(index, sensor)| {
            PhysicalSensorAperture::new(
                SensorId::new(
                    actor_id
                        .raw()
                        .saturating_mul(1_000)
                        .saturating_add(index as u64),
                ),
                EntityId::new(actor_id.raw()),
                SignalChannelId::new(sensor.kind.raw().saturating_add(ACTOR_SIGNAL_CHANNEL)),
                WorldCoord::new(
                    actor.body.position.x + i64::from(sensor.position.x),
                    actor.body.position.y + i64::from(sensor.position.y),
                    actor.body.position.z + i64::from(sensor.position.z),
                ),
                AccessRange::new(u32::from(sensor.range)),
                1,
            )
        })
        .collect()
}

fn physical_signals(
    time: SimulationTime,
    objects: &BTreeMap<u64, ActorPhysicalObject>,
) -> Vec<PhysicalSignal> {
    objects
        .values()
        .filter(|object| object.accessible && !object.occluded)
        .map(|object| {
            PhysicalSignal::new(
                EntityId::new(object.object_key),
                SignalChannelId::new(
                    SensorKindId::new(1)
                        .raw()
                        .saturating_add(ACTOR_SIGNAL_CHANNEL),
                ),
                object.position,
                SignalMagnitude::new(object.magnitude),
                time,
                object.trace,
            )
        })
        .collect()
}

fn generic_feature_from_extracted(
    time: SimulationTime,
    feature: &causafera_types::Feature,
    relative_position: [i64; 3],
) -> GenericFeature {
    let magnitude = match feature.value {
        FeatureValue::MagnitudeBand(value) => u16::from(value),
        FeatureValue::FrequencyBand(value) => u16::from(value),
        FeatureValue::Scalar(value) => value.abs().min(f64::from(u16::MAX)) as u16,
        FeatureValue::Direction(direction) => {
            direction.length_squared().min(f64::from(u16::MAX)) as u16
        }
    };
    GenericFeature {
        percept: PerceptId::new(feature.id.raw()),
        attention_target: AttentionTargetId::new(feature.id.raw()),
        relation: feature.relation,
        value: feature.value,
        appearance: AppearanceSignature([magnitude, magnitude, magnitude, magnitude]),
        relative_position: [
            relative_position[0].clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32,
            relative_position[1].clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32,
            relative_position[2].clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32,
        ],
        strength: match CognitiveWeight::new(
            u32::from(magnitude).saturating_mul(10_000).min(1_000_000),
        ) {
            Ok(value) => value,
            Err(_) => CognitiveWeight::ZERO,
        },
        time,
    }
}
pub(crate) struct ActorPerceptionSystem {
    pub(crate) state: Arc<Mutex<RuntimeState>>,
    pub(crate) next_time: SimulationTime,
}

impl ActorPerceptionSystem {
    pub(crate) fn new(state: Arc<Mutex<RuntimeState>>) -> Self {
        Self {
            state,
            next_time: SimulationTime::new(1),
        }
    }

    pub(crate) fn execute(&mut self) -> Result<(), RuntimeError> {
        let mut state = self.state.lock().map_err(|_| RuntimeError::StatePoisoned)?;
        let objects = state.actor_objects.clone();
        let material_signals = material_surface_physical_signals(&state, self.next_time);
        let feature_count = actor_perception_step(
            self.next_time,
            &mut state.actors,
            &objects,
            &material_signals,
        )?;
        state.perceived_actor_features = state
            .perceived_actor_features
            .saturating_add(feature_count as u64);
        self.next_time = self.next_time.tick();
        Ok(())
    }
}

impl System for ActorPerceptionSystem {
    fn run(&mut self, _stream: &mut RandomStream) {
        if let Err(error) = self.execute() {
            if let Ok(mut state) = self.state.lock() {
                state.failure.get_or_insert(error);
            }
        }
    }

    fn restore_time(&mut self, time: SimulationTime) {
        self.next_time = time;
    }
}
