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

fn physical_apertures(actor_id: ActorId, actor: &ActorState) -> Vec<PhysicalSensorAperture> {
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

/// The per-actor cue count both cognition-layer caps admit.
///
/// `actor_cognition_step` derives one attention candidate and one perceptual cue
/// from every extracted feature, so a batch has to clear
/// [`causafera_cognition::MAX_ATTENTION_CANDIDATES`] (checked first, inside
/// `Attention::update`) as well as [`causafera_cognition::MAX_SCENE_CUES`]
/// (checked by `SceneContinuityState::reconstruct`). Taking the smaller of the
/// two keeps this faithful if the two caps ever diverge.
pub const MAX_RUNNABLE_SCENE_CUES: usize =
    if causafera_cognition::MAX_ATTENTION_CANDIDATES < causafera_cognition::MAX_SCENE_CUES {
        causafera_cognition::MAX_ATTENTION_CANDIDATES
    } else {
        causafera_cognition::MAX_SCENE_CUES
    };

/// The most material-surface signals `config` can ever present to one aperture.
///
/// `MaterialSurfaceBootstrapStage` creates exactly one surface per active chunk
/// and no other path adds one, so the surfaces holding `contact_count > 0` are
/// at most the active chunk set. Which of them a given run actually reaches
/// depends on where actors move, so this is the only bound a configuration on
/// its own can justify.
pub fn worst_case_contacted_surface_count(config: &RuntimeConfig) -> usize {
    if !config.material_surface_signals_enabled {
        return 0;
    }
    active_chunk_keys(
        config.chart_id,
        config.active_chunk_radius,
        config.active_chunk_shape,
    )
    .len()
}

/// The largest per-actor cue count `config` can produce on any tick of any run.
///
/// This mirrors what [`actor_perception_step`] assembles rather than
/// approximating it, term by term:
///
/// - One cue is produced per extracted feature, and `actor.features.len()`
///   equals `batch.samples().len()` exactly. `GenericFeatureExtractor::extract`
///   appends one extra `Change` feature per adjacent sample pair satisfying
///   `is_later_sample` (`causafera-perception/src/extraction.rs`), which
///   requires `previous.time() < current.time()`. Every sample in a batch
///   carries that acquisition's single `time`, and `acquire_signals`
///   (`causafera-perception/src/access.rs`) discards any signal whose `time`
///   differs from it, so one batch can never hold two samples at different times
///   and the change term is always zero. **This bound stops being sound if
///   either of those two call sites changes.**
/// - `acquire_signals` emits at most one sample per (aperture, signal) pair, and
///   [`physical_apertures`] builds one aperture per configured sensor, so the
///   sample count is at most `sensor_count` times the signal count.
/// - The signal slice is [`physical_signals`] over `actor_objects` — one entry
///   per promoted actor, held at `actor_count` by the promotion guard in
///   `population.rs` — extended with the material-surface signals bounded by
///   [`worst_case_contacted_surface_count`].
///
/// The result is a worst case, not a prediction: a real tick sees fewer cues
/// whenever a signal falls outside a sensor's range or channel, which is why
/// configurations with many sensors run further than this bound admits. It is
/// the tightest bound a `RuntimeConfig` alone can justify, because how far
/// surface contact has spread is a property of a particular run, not of the
/// configuration that started it.
pub fn worst_case_scene_cue_count(config: &RuntimeConfig) -> usize {
    usize::from(config.sensor_count).saturating_mul(
        usize::from(config.actor_count).saturating_add(worst_case_contacted_surface_count(config)),
    )
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
        if let Err(error) = self.execute()
            && let Ok(mut state) = self.state.lock()
        {
            state.failure.get_or_insert(error);
        }
    }

    fn restore_time(&mut self, time: SimulationTime) {
        self.next_time = time;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use causafera_cognition::SceneUpdateError;

    fn boundary_config(
        actor_count: u8,
        sensor_count: u8,
        radius: u8,
        shape: ActiveChunkShape,
    ) -> RuntimeConfig {
        let mut config = RuntimeConfig::new(7);
        config.chunk_extent = 3;
        config.active_chunk_radius = radius;
        config.active_chunk_shape = shape;
        config.actor_count = actor_count;
        config.sensor_count = sensor_count;
        config.bootstrap_population = 512;
        config
    }

    /// Runs the real perception and cognition steps with every active chunk's
    /// surface forced into contact, and reports the largest cue batch produced.
    ///
    /// Waiting for a run to reach full contact does not work: the Wave 1 harness
    /// measured contact staying at three surfaces however many chunks were
    /// active, because which chunks register contact depends on where actors
    /// happen to move. Forcing it is the only way to exercise the case
    /// [`worst_case_scene_cue_count`] claims to bound.
    fn cues_at_full_surface_contact(config: RuntimeConfig) -> Result<usize, SceneUpdateError> {
        let mut runtime = Runtime::new(config).expect("configuration is accepted");
        runtime.run_ticks(4).expect("accepted configuration ticks");
        let time = runtime.current_time().tick();
        let mut state = runtime.state.lock().expect("state is not poisoned");
        for surface in state.material_surfaces.values_mut() {
            surface.contact_count = surface.contact_count.max(1);
        }
        let material_signals = material_surface_physical_signals(&state, time);
        assert_eq!(
            material_signals.len(),
            worst_case_contacted_surface_count(&state.config),
            "every active chunk's surface should be signalling once contact is forced"
        );
        let objects = state.actor_objects.clone();
        assert_eq!(
            objects.len(),
            usize::from(state.config.actor_count),
            "the promotion guard should hold one object per configured actor"
        );
        actor_perception_step(time, &mut state.actors, &objects, &material_signals)
            .expect("perception assembles a batch");
        let observed = state
            .actors
            .values()
            .map(|actor| actor.features.len())
            .max()
            .unwrap_or(0);
        actor_cognition_step(
            time,
            &mut state.actors,
            ActionKindId::new(ACTOR_CONTACT_ACTION_KIND),
        )?;
        Ok(observed)
    }

    #[test]
    fn accepted_configurations_still_run_at_full_surface_contact() {
        for (actor_count, sensor_count, radius, shape) in [
            (63u8, 1u8, 0u8, ActiveChunkShape::Line),
            (31, 2, 0, ActiveChunkShape::Line),
            (7, 8, 0, ActiveChunkShape::Line),
            (3, 16, 0, ActiveChunkShape::Line),
            (8, 2, 1, ActiveChunkShape::Area),
            (12, 2, 2, ActiveChunkShape::Line),
        ] {
            let config = boundary_config(actor_count, sensor_count, radius, shape);
            let bound = worst_case_scene_cue_count(&config);
            assert!(
                bound <= MAX_RUNNABLE_SCENE_CUES,
                "{actor_count} actors with {sensor_count} sensors at radius {radius} should be an \
                 accepted configuration, but its worst case is {bound}"
            );
            let observed = cues_at_full_surface_contact(config)
                .expect("an accepted configuration survives full surface contact");
            assert!(
                observed <= bound,
                "forced full contact produced {observed} cues against a bound of {bound}"
            );
        }
    }

    #[test]
    fn the_bound_is_attained_where_every_signal_clears_every_aperture() {
        // With few sensors at radius 0 every signal is in range and on channel,
        // so the worst case is not merely an upper bound but the count actually
        // produced. This is what makes the validation boundary and the boundary
        // the Wave 1 sweep measured empirically the same number here; beyond
        // roughly eight sensors the apertures stop seeing everything and the
        // bound becomes strictly conservative.
        for (actor_count, sensor_count) in [(63u8, 1u8), (31, 2), (15, 4)] {
            let config = boundary_config(actor_count, sensor_count, 0, ActiveChunkShape::Line);
            let bound = worst_case_scene_cue_count(&config);
            let observed = cues_at_full_surface_contact(config)
                .expect("an accepted configuration survives full surface contact");
            assert_eq!(
                observed, bound,
                "{actor_count} actors with {sensor_count} sensors should attain the bound exactly"
            );
        }
    }

    #[test]
    fn cognition_still_rejects_a_batch_over_the_cue_cap() {
        // The construction-time bound is a usability improvement, not a
        // replacement for the cognition-layer cap. This asserts the backstop
        // still fails closed for a batch that reaches it by any other route.
        let mut runtime = Runtime::new(boundary_config(2, 1, 0, ActiveChunkShape::Line))
            .expect("configuration is accepted");
        runtime.run_ticks(4).expect("accepted configuration ticks");
        let time = runtime.current_time().tick();
        let mut state = runtime.state.lock().expect("state is not poisoned");
        let actor = state
            .actors
            .values_mut()
            .next()
            .expect("a promoted actor exists");
        let template = *actor
            .features
            .first()
            .expect("perception produced at least one feature");
        actor.features.clear();
        for index in 0..=MAX_RUNNABLE_SCENE_CUES {
            let distinct = 1_000_000 + index as u64;
            actor.features.push(GenericFeature {
                percept: PerceptId::new(distinct),
                attention_target: AttentionTargetId::new(distinct),
                time,
                ..template
            });
        }
        let error = actor_cognition_step(
            time,
            &mut state.actors,
            ActionKindId::new(ACTOR_CONTACT_ACTION_KIND),
        )
        .expect_err("cognition rejects a batch over the cap");
        assert!(
            matches!(
                error,
                SceneUpdateError::TooManyCues { count } if count == MAX_RUNNABLE_SCENE_CUES + 1
            ),
            "expected the cue cap to reject 65 cues, got {error}"
        );
    }
}
