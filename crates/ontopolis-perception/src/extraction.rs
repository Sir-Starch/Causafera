use ontopolis_types::{Feature, FeatureId, FeatureRelation, FeatureValue, Persistence, TraceId};
use thiserror::Error;

use crate::SensorySample;

/// Positive magnitude quantum used to assign generic 8-bit bands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MagnitudeQuantum(u64);

impl MagnitudeQuantum {
    pub fn new(raw: u64) -> Result<Self, ExtractionConfigError> {
        if raw == 0 {
            return Err(ExtractionConfigError::ZeroMagnitudeQuantum);
        }
        Ok(Self(raw))
    }

    pub const fn raw(self) -> u64 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Error, PartialEq, Eq)]
pub enum ExtractionConfigError {
    #[error("magnitude quantum must be positive")]
    ZeroMagnitudeQuantum,
}

/// Minimal generic extractor configuration.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GenericFeatureExtractor {
    magnitude_quantum: MagnitudeQuantum,
}

impl GenericFeatureExtractor {
    pub const fn new(magnitude_quantum: MagnitudeQuantum) -> Self {
        Self { magnitude_quantum }
    }

    /// Extract magnitude observations and consecutive changes.
    ///
    /// The type boundary accepts acquired samples, never raw Ground Truth.
    /// Samples are canonicalized before IDs are assigned.
    pub fn extract(
        self,
        first_feature: FeatureId,
        samples: &[SensorySample],
    ) -> Result<ExtractedFeatureBatch, ExtractionError> {
        let mut samples = samples.to_vec();
        samples.sort_by_key(|sample| {
            (
                sample.sensor(),
                sample.source(),
                sample.channel(),
                sample.time(),
                sample.acquisition(),
            )
        });

        let change_count = samples
            .windows(2)
            .filter(|pair| is_later_sample(pair[0], pair[1]))
            .count();
        let feature_count = samples
            .len()
            .checked_add(change_count)
            .ok_or(ExtractionError::CapacityExceeded)?;
        let feature_count_u64 =
            u64::try_from(feature_count).map_err(|_| ExtractionError::IdentifierExhausted)?;
        first_feature
            .raw()
            .checked_add(feature_count_u64)
            .ok_or(ExtractionError::IdentifierExhausted)?;

        let mut batch = ExtractedFeatureBatch::with_capacity(feature_count);
        for (index, sample) in samples.iter().copied().enumerate() {
            let magnitude_band =
                (sample.magnitude().unsigned_abs() / self.magnitude_quantum.raw()).min(255) as u8;
            batch.push(
                Feature {
                    id: FeatureId::new(first_feature.raw() + batch.len() as u64),
                    target_id: sample.source(),
                    relation: FeatureRelation::Magnitude,
                    value: FeatureValue::MagnitudeBand(magnitude_band),
                    persistence: Persistence::Fleeting,
                },
                &[sample.input_trace()],
            )?;

            if index > 0 && is_later_sample(samples[index - 1], sample) {
                let previous = samples[index - 1];
                let delta = sample.magnitude().raw() as f64 - previous.magnitude().raw() as f64;
                let mut traces = [previous.input_trace(), sample.input_trace()];
                traces.sort_unstable();
                let trace_count = if traces[0] == traces[1] { 1 } else { 2 };
                batch.push(
                    Feature {
                        id: FeatureId::new(first_feature.raw() + batch.len() as u64),
                        target_id: sample.source(),
                        relation: FeatureRelation::Change,
                        value: FeatureValue::Scalar(delta),
                        persistence: Persistence::Brief,
                    },
                    &traces[..trace_count],
                )?;
            }
        }
        Ok(batch)
    }
}

/// Flat generic features plus flattened causal-input spans.
#[derive(Clone, Debug, PartialEq)]
pub struct ExtractedFeatureBatch {
    features: Vec<Feature>,
    provenance_offsets: Vec<u32>,
    input_traces: Vec<TraceId>,
}

impl ExtractedFeatureBatch {
    fn with_capacity(features: usize) -> Self {
        let mut provenance_offsets = Vec::with_capacity(features.saturating_add(1));
        provenance_offsets.push(0);
        Self {
            features: Vec::with_capacity(features),
            provenance_offsets,
            input_traces: Vec::with_capacity(features),
        }
    }

    pub fn features(&self) -> &[Feature] {
        &self.features
    }

    pub fn input_traces(&self, feature_index: usize) -> Option<&[TraceId]> {
        if feature_index >= self.features.len() {
            return None;
        }
        let start = self.provenance_offsets[feature_index] as usize;
        let end = self.provenance_offsets[feature_index + 1] as usize;
        Some(&self.input_traces[start..end])
    }

    pub fn len(&self) -> usize {
        self.features.len()
    }

    pub fn is_empty(&self) -> bool {
        self.features.is_empty()
    }

    fn push(&mut self, feature: Feature, traces: &[TraceId]) -> Result<(), ExtractionError> {
        let final_len = self
            .input_traces
            .len()
            .checked_add(traces.len())
            .ok_or(ExtractionError::CapacityExceeded)?;
        let offset = u32::try_from(final_len).map_err(|_| ExtractionError::CapacityExceeded)?;
        self.features.push(feature);
        self.input_traces.extend_from_slice(traces);
        self.provenance_offsets.push(offset);
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Error, PartialEq, Eq)]
pub enum ExtractionError {
    #[error("feature identifier space is exhausted")]
    IdentifierExhausted,
    #[error("flat feature provenance capacity is exceeded")]
    CapacityExceeded,
}

fn same_series(a: SensorySample, b: SensorySample) -> bool {
    a.sensor() == b.sensor() && a.source() == b.source() && a.channel() == b.channel()
}

fn is_later_sample(previous: SensorySample, current: SensorySample) -> bool {
    same_series(previous, current) && previous.time() < current.time()
}

#[cfg(test)]
mod tests {
    use ontopolis_types::{
        AcquisitionId, EntityId, SensorId, SignalChannelId, SimulationTime, TraceId, WorldCoord,
    };

    use super::*;
    use crate::{AccessRange, PhysicalSignal, SensorAperture, SignalMagnitude, acquire_signals};

    fn acquired_samples() -> Vec<SensorySample> {
        let aperture = SensorAperture::new(
            SensorId::new(1),
            EntityId::new(99),
            SignalChannelId::new(4),
            WorldCoord::new(0, 0, 0),
            AccessRange::new(10),
            0,
        );
        let first = PhysicalSignal::new(
            EntityId::new(8),
            SignalChannelId::new(4),
            WorldCoord::new(2, 0, 0),
            SignalMagnitude::new(10),
            SimulationTime::new(2),
            TraceId::new(7),
        );
        let second = PhysicalSignal::new(
            EntityId::new(8),
            SignalChannelId::new(4),
            WorldCoord::new(2, 0, 0),
            SignalMagnitude::new(25),
            SimulationTime::new(3),
            TraceId::new(9),
        );
        let mut samples = acquire_signals(
            SimulationTime::new(2),
            AcquisitionId::new(0),
            &[aperture],
            &[first],
        )
        .unwrap()
        .samples()
        .to_vec();
        samples.extend_from_slice(
            acquire_signals(
                SimulationTime::new(3),
                AcquisitionId::new(1),
                &[aperture],
                &[second],
            )
            .unwrap()
            .samples(),
        );
        samples
    }

    #[test]
    fn quantum_must_be_positive() {
        assert_eq!(
            MagnitudeQuantum::new(0),
            Err(ExtractionConfigError::ZeroMagnitudeQuantum)
        );
    }

    #[test]
    fn extraction_emits_generic_features_with_causal_inputs() {
        let batch = GenericFeatureExtractor::new(MagnitudeQuantum::new(5).unwrap())
            .extract(FeatureId::new(20), &acquired_samples())
            .unwrap();
        assert_eq!(batch.len(), 3);
        assert_eq!(batch.features()[0].relation, FeatureRelation::Magnitude);
        assert_eq!(batch.features()[1].relation, FeatureRelation::Magnitude);
        assert_eq!(batch.features()[2].relation, FeatureRelation::Change);
        assert_eq!(batch.features()[2].value, FeatureValue::Scalar(15.0));
        assert_eq!(
            batch.input_traces(2),
            Some([TraceId::new(7), TraceId::new(9)].as_slice())
        );
    }

    #[test]
    fn extraction_is_independent_of_sample_order() {
        let samples = acquired_samples();
        let extractor = GenericFeatureExtractor::new(MagnitudeQuantum::new(5).unwrap());
        let a = extractor.extract(FeatureId::new(0), &samples).unwrap();
        let b = extractor
            .extract(
                FeatureId::new(0),
                &samples.into_iter().rev().collect::<Vec<_>>(),
            )
            .unwrap();
        assert_eq!(a, b);
    }
}
