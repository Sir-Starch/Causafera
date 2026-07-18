use causafera_core::StateFingerprint;
use causafera_types::{AttractorExperimentId, AttractorProbeSchemaId, SimulationTime, TraceId};
use thiserror::Error;

pub const MAX_ATTRACTOR_OBSERVATIONS: usize = 4_096;

/// Read-only numeric observation of a physical field trajectory.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct FieldTrajectoryObservation {
    pub observed_at: SimulationTime,
    pub state: StateFingerprint,
    pub total_magnitude: i64,
    pub maximum_magnitude: i64,
    pub changed_components: u32,
    pub supporting_trace: TraceId,
}

impl FieldTrajectoryObservation {
    pub fn new(
        observed_at: SimulationTime,
        state: StateFingerprint,
        total_magnitude: i64,
        maximum_magnitude: i64,
        changed_components: u32,
        supporting_trace: TraceId,
    ) -> Result<Self, AttractorResearchError> {
        if total_magnitude < 0 || maximum_magnitude < 0 || maximum_magnitude > total_magnitude {
            return Err(AttractorResearchError::InvalidMagnitude);
        }
        Ok(Self {
            observed_at,
            state,
            total_magnitude,
            maximum_magnitude,
            changed_components,
            supporting_trace,
        })
    }
}

/// Numeric read-only probe; its schema is opaque and externally registered.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AttractorProbe {
    pub schema: AttractorProbeSchemaId,
    pub stability_tolerance: i64,
    pub recovery_tolerance: i64,
}

impl AttractorProbe {
    pub fn new(
        schema: AttractorProbeSchemaId,
        stability_tolerance: i64,
        recovery_tolerance: i64,
    ) -> Result<Self, AttractorResearchError> {
        if stability_tolerance < 0 || recovery_tolerance < 0 {
            return Err(AttractorResearchError::InvalidTolerance);
        }
        Ok(Self {
            schema,
            stability_tolerance,
            recovery_tolerance,
        })
    }

    pub fn evaluate(
        self,
        experiment: AttractorExperimentId,
        observations: &[FieldTrajectoryObservation],
        perturbation_ended_at: Option<SimulationTime>,
    ) -> Result<AttractorEvidence, AttractorResearchError> {
        if observations.len() < 2 || observations.len() > MAX_ATTRACTOR_OBSERVATIONS {
            return Err(AttractorResearchError::InvalidObservationCount {
                count: observations.len(),
            });
        }
        let mut ordered = observations.to_vec();
        ordered.sort_unstable();
        if ordered
            .windows(2)
            .any(|pair| pair[0].observed_at == pair[1].observed_at)
        {
            return Err(AttractorResearchError::DuplicateObservationTime);
        }

        let stable_transitions = ordered
            .windows(2)
            .filter(|pair| {
                pair[1].total_magnitude.abs_diff(pair[0].total_magnitude)
                    <= self.stability_tolerance as u64
            })
            .count() as u32;
        let recovery_distance = perturbation_ended_at.and_then(|end| {
            let before = ordered.iter().rev().find(|item| item.observed_at < end)?;
            let after = ordered.iter().rev().find(|item| item.observed_at >= end)?;
            Some(after.total_magnitude.abs_diff(before.total_magnitude))
        });
        let recovered_within_tolerance =
            recovery_distance.map(|distance| distance <= self.recovery_tolerance as u64);
        let mut supporting_traces = ordered
            .iter()
            .map(|observation| observation.supporting_trace)
            .collect::<Vec<_>>();
        supporting_traces.sort_unstable();
        supporting_traces.dedup();

        Ok(AttractorEvidence {
            experiment,
            probe_schema: self.schema,
            observed_from: ordered[0].observed_at,
            observed_through: ordered[ordered.len() - 1].observed_at,
            stable_transitions,
            transition_count: (ordered.len() - 1) as u32,
            recovery_distance,
            recovered_within_tolerance,
            supporting_traces,
        })
    }
}

/// Evidence about stability and recovery, not an instantiated attractor or being.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AttractorEvidence {
    pub experiment: AttractorExperimentId,
    pub probe_schema: AttractorProbeSchemaId,
    pub observed_from: SimulationTime,
    pub observed_through: SimulationTime,
    pub stable_transitions: u32,
    pub transition_count: u32,
    pub recovery_distance: Option<u64>,
    pub recovered_within_tolerance: Option<bool>,
    pub supporting_traces: Vec<TraceId>,
}

#[derive(Clone, Copy, Debug, Error, PartialEq, Eq)]
pub enum AttractorResearchError {
    #[error("field trajectory magnitudes are invalid")]
    InvalidMagnitude,
    #[error("attractor probe tolerance must be non-negative")]
    InvalidTolerance,
    #[error("invalid field trajectory observation count: {count}")]
    InvalidObservationCount { count: usize },
    #[error("field trajectory observation times must be unique")]
    DuplicateObservationTime,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn observation(tick: u64, total: i64) -> FieldTrajectoryObservation {
        FieldTrajectoryObservation::new(
            SimulationTime::new(tick),
            StateFingerprint::new([tick as u8; 32]),
            total,
            total,
            1,
            TraceId::new(tick),
        )
        .unwrap()
    }

    #[test]
    fn probe_reports_numeric_evidence_without_creating_an_entity() {
        let probe = AttractorProbe::new(AttractorProbeSchemaId::new(3), 3, 10).unwrap();
        let evidence = probe
            .evaluate(
                AttractorExperimentId::new(8),
                &[
                    observation(1, 100),
                    observation(2, 102),
                    observation(3, 101),
                ],
                None,
            )
            .unwrap();
        assert_eq!(evidence.stable_transitions, 2);
        assert_eq!(evidence.transition_count, 2);
        assert_eq!(evidence.recovered_within_tolerance, None);
    }
}
