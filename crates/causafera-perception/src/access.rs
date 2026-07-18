use causafera_types::{
    AcquisitionId, EntityId, SensorId, SignalChannelId, SimulationTime, TraceId, WorldCoord,
};
use thiserror::Error;

/// Signed, quantized physical signal magnitude.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SignalMagnitude(i64);

impl SignalMagnitude {
    pub const fn new(raw: i64) -> Self {
        Self(raw)
    }

    pub const fn raw(self) -> i64 {
        self.0
    }

    pub const fn unsigned_abs(self) -> u64 {
        self.0.unsigned_abs()
    }
}

/// Maximum Chebyshev access distance in world-coordinate cells.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AccessRange(u32);

impl AccessRange {
    pub const fn new(cells: u32) -> Self {
        Self(cells)
    }

    pub const fn cells(self) -> u32 {
        self.0
    }
}

/// One physically emitted signal at a Ground Truth position.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PhysicalSignal {
    source: EntityId,
    channel: SignalChannelId,
    position: WorldCoord,
    magnitude: SignalMagnitude,
    time: SimulationTime,
    trace: TraceId,
}

impl PhysicalSignal {
    pub const fn new(
        source: EntityId,
        channel: SignalChannelId,
        position: WorldCoord,
        magnitude: SignalMagnitude,
        time: SimulationTime,
        trace: TraceId,
    ) -> Self {
        Self {
            source,
            channel,
            position,
            magnitude,
            time,
            trace,
        }
    }
}

/// Property-based physical aperture for one signal channel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SensorAperture {
    sensor: SensorId,
    owner: EntityId,
    channel: SignalChannelId,
    position: WorldCoord,
    range: AccessRange,
    minimum_magnitude: u64,
}

impl SensorAperture {
    pub const fn new(
        sensor: SensorId,
        owner: EntityId,
        channel: SignalChannelId,
        position: WorldCoord,
        range: AccessRange,
        minimum_magnitude: u64,
    ) -> Self {
        Self {
            sensor,
            owner,
            channel,
            position,
            range,
            minimum_magnitude,
        }
    }

    pub const fn sensor(self) -> SensorId {
        self.sensor
    }

    pub const fn owner(self) -> EntityId {
        self.owner
    }
}

/// Acquired, relative sample suitable for Ground Truth feature extraction.
///
/// `source` remains authoritative extractor bookkeeping. This record must not
/// enter agent cognition; Phase 9 maps it to a subjective identity hypothesis.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SensorySample {
    acquisition: AcquisitionId,
    sensor: SensorId,
    source: EntityId,
    channel: SignalChannelId,
    relative_position: [i64; 3],
    magnitude: SignalMagnitude,
    time: SimulationTime,
    input_trace: TraceId,
}

impl SensorySample {
    pub const fn acquisition(self) -> AcquisitionId {
        self.acquisition
    }

    pub const fn sensor(self) -> SensorId {
        self.sensor
    }

    pub const fn source(self) -> EntityId {
        self.source
    }

    pub const fn channel(self) -> SignalChannelId {
        self.channel
    }

    pub const fn relative_position(self) -> [i64; 3] {
        self.relative_position
    }

    pub const fn magnitude(self) -> SignalMagnitude {
        self.magnitude
    }

    pub const fn time(self) -> SimulationTime {
        self.time
    }

    pub const fn input_trace(self) -> TraceId {
        self.input_trace
    }
}

/// Canonically ordered acquisition output.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SensoryBatch {
    samples: Vec<SensorySample>,
}

impl SensoryBatch {
    pub fn samples(&self) -> &[SensorySample] {
        &self.samples
    }

    pub fn len(&self) -> usize {
        self.samples.len()
    }

    pub fn is_empty(&self) -> bool {
        self.samples.is_empty()
    }
}

/// Acquire signals visible through the supplied apertures at `time`.
///
/// Input order has no effect. Output is ordered by sensor, source, channel,
/// trace, relative position, and magnitude before sequential IDs are assigned.
pub fn acquire_signals(
    time: SimulationTime,
    first_acquisition: AcquisitionId,
    apertures: &[SensorAperture],
    signals: &[PhysicalSignal],
) -> Result<SensoryBatch, AcquisitionError> {
    let mut apertures = apertures.to_vec();
    apertures.sort_by_key(|aperture| aperture.sensor);
    for index in 1..apertures.len() {
        if apertures[index - 1].sensor == apertures[index].sensor {
            return Err(AcquisitionError::DuplicateSensor {
                sensor: apertures[index].sensor,
            });
        }
    }

    let mut visible = Vec::new();
    for aperture in apertures {
        for signal in signals {
            if signal.time != time
                || signal.channel != aperture.channel
                || signal.magnitude.unsigned_abs() < aperture.minimum_magnitude
                || chebyshev_distance(signal.position, aperture.position)
                    > u64::from(aperture.range.cells())
            {
                continue;
            }
            let relative_position = [
                signal.position.x.checked_sub(aperture.position.x),
                signal.position.y.checked_sub(aperture.position.y),
                signal.position.z.checked_sub(aperture.position.z),
            ];
            let [Some(x), Some(y), Some(z)] = relative_position else {
                return Err(AcquisitionError::CoordinateDifferenceOverflow);
            };
            visible.push((
                aperture.sensor,
                signal.source,
                signal.channel,
                signal.trace,
                [x, y, z],
                signal.magnitude,
            ));
        }
    }
    visible.sort_by_key(|(sensor, source, channel, trace, position, magnitude)| {
        (*sensor, *source, *channel, *trace, *position, *magnitude)
    });
    visible.dedup();

    let count = u64::try_from(visible.len()).map_err(|_| AcquisitionError::IdentifierExhausted)?;
    first_acquisition
        .raw()
        .checked_add(count)
        .ok_or(AcquisitionError::IdentifierExhausted)?;
    let samples = visible
        .into_iter()
        .enumerate()
        .map(
            |(index, (sensor, source, channel, trace, relative_position, magnitude))| {
                SensorySample {
                    acquisition: AcquisitionId::new(first_acquisition.raw() + index as u64),
                    sensor,
                    source,
                    channel,
                    relative_position,
                    magnitude,
                    time,
                    input_trace: trace,
                }
            },
        )
        .collect();
    Ok(SensoryBatch { samples })
}

#[derive(Clone, Copy, Debug, Error, PartialEq, Eq)]
pub enum AcquisitionError {
    #[error("sensor {sensor} has more than one aperture in an acquisition batch")]
    DuplicateSensor { sensor: SensorId },
    #[error("relative coordinate calculation overflowed")]
    CoordinateDifferenceOverflow,
    #[error("acquisition identifier space is exhausted")]
    IdentifierExhausted,
}

fn chebyshev_distance(a: WorldCoord, b: WorldCoord) -> u64 {
    a.x.abs_diff(b.x)
        .max(a.y.abs_diff(b.y))
        .max(a.z.abs_diff(b.z))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn aperture(sensor: u64, channel: u64, x: i64, range: u32, threshold: u64) -> SensorAperture {
        SensorAperture::new(
            SensorId::new(sensor),
            EntityId::new(900 + sensor),
            SignalChannelId::new(channel),
            WorldCoord::new(x, 0, 0),
            AccessRange::new(range),
            threshold,
        )
    }

    fn signal(source: u64, channel: u64, x: i64, magnitude: i64, trace: u64) -> PhysicalSignal {
        PhysicalSignal::new(
            EntityId::new(source),
            SignalChannelId::new(channel),
            WorldCoord::new(x, 0, 0),
            SignalMagnitude::new(magnitude),
            SimulationTime::new(4),
            TraceId::new(trace),
        )
    }

    #[test]
    fn acquisition_filters_channel_range_threshold_and_time() {
        let signals = [
            signal(1, 5, 2, 10, 1),
            signal(2, 6, 2, 10, 2),
            signal(3, 5, 9, 10, 3),
            signal(4, 5, 2, 3, 4),
            PhysicalSignal::new(
                EntityId::new(5),
                SignalChannelId::new(5),
                WorldCoord::new(2, 0, 0),
                SignalMagnitude::new(10),
                SimulationTime::new(3),
                TraceId::new(5),
            ),
        ];
        let batch = acquire_signals(
            SimulationTime::new(4),
            AcquisitionId::new(10),
            &[aperture(7, 5, 0, 3, 5)],
            &signals,
        )
        .unwrap();
        assert_eq!(batch.len(), 1);
        assert_eq!(batch.samples()[0].source(), EntityId::new(1));
        assert_eq!(batch.samples()[0].relative_position(), [2, 0, 0]);
        assert_eq!(batch.samples()[0].input_trace(), TraceId::new(1));
    }

    #[test]
    fn acquisition_is_independent_of_input_order() {
        let apertures = vec![aperture(2, 5, 0, 9, 0), aperture(1, 5, 0, 9, 0)];
        let signals = vec![signal(8, 5, 2, 4, 8), signal(3, 5, 1, 9, 3)];
        let a = acquire_signals(
            SimulationTime::new(4),
            AcquisitionId::new(0),
            &apertures,
            &signals,
        )
        .unwrap();
        let b = acquire_signals(
            SimulationTime::new(4),
            AcquisitionId::new(0),
            &apertures.into_iter().rev().collect::<Vec<_>>(),
            &signals.into_iter().rev().collect::<Vec<_>>(),
        )
        .unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn duplicate_sensor_configuration_is_rejected() {
        let result = acquire_signals(
            SimulationTime::new(4),
            AcquisitionId::new(0),
            &[aperture(1, 5, 0, 1, 0), aperture(1, 6, 0, 1, 0)],
            &[],
        );
        assert_eq!(
            result,
            Err(AcquisitionError::DuplicateSensor {
                sensor: SensorId::new(1)
            })
        );
    }
}
