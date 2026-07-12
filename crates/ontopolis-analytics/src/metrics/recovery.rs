use ontopolis_core::StateFingerprint;
use ontopolis_types::SimulationTime;

use super::{
    AnalyticsCheckpoint, AnalyticsError, MatchedCheckpointAnalysis, MatchedCheckpointDistance,
};

pub fn analyze_recovery(
    control: &[AnalyticsCheckpoint],
    intervention: &[AnalyticsCheckpoint],
    perturbation_from: SimulationTime,
    perturbation_through: SimulationTime,
    tolerance: u64,
) -> Result<MatchedCheckpointAnalysis, AnalyticsError> {
    if control.len() != intervention.len() || control.is_empty() {
        return Err(AnalyticsError::MismatchedCheckpointSeries);
    }
    let mut distances = Vec::with_capacity(control.len());
    for (control_snapshot, intervention_snapshot) in control.iter().zip(intervention) {
        if control_snapshot.time != intervention_snapshot.time {
            return Err(AnalyticsError::MismatchedCheckpointSeries);
        }
        distances.push(MatchedCheckpointDistance {
            checkpoint_time: control_snapshot.time,
            physical_distance: fingerprint_distance(
                control_snapshot.physical_state,
                intervention_snapshot.physical_state,
            ),
            history_diverged: control_snapshot.history_state != intervention_snapshot.history_state,
            control_trace: control_snapshot.latest_trace,
            intervention_trace: intervention_snapshot.latest_trace,
        });
    }
    let baseline = distances
        .iter()
        .rev()
        .find(|distance| distance.checkpoint_time < perturbation_from)
        .ok_or(AnalyticsError::MissingBaselineCheckpoint)?;
    let mut perturbation = distances
        .iter()
        .filter(|distance| {
            distance.checkpoint_time >= perturbation_from
                && distance.checkpoint_time <= perturbation_through
        })
        .peekable();
    if perturbation.peek().is_none() {
        return Err(AnalyticsError::MissingPerturbationCheckpoint);
    }
    let (perturbation_minimum_distance, perturbation_maximum_distance) =
        perturbation.fold((u64::MAX, 0), |(minimum, maximum), distance| {
            (
                minimum.min(distance.physical_distance),
                maximum.max(distance.physical_distance),
            )
        });
    let final_recovery_distance = distances
        .last()
        .map(|distance| distance.physical_distance)
        .ok_or(AnalyticsError::MismatchedCheckpointSeries)?;
    let time_to_recovery = distances
        .iter()
        .find(|distance| {
            distance.checkpoint_time > perturbation_through
                && distance.physical_distance <= tolerance
        })
        .map(|distance| distance.checkpoint_time.raw() - perturbation_through.raw());
    Ok(MatchedCheckpointAnalysis {
        pre_intervention_baseline_distance: baseline.physical_distance,
        perturbation_minimum_distance,
        perturbation_maximum_distance,
        matched_control_distances: distances,
        final_recovery_distance,
        time_to_recovery,
    })
}

pub fn fingerprint_distance(left: StateFingerprint, right: StateFingerprint) -> u64 {
    left.bytes()
        .iter()
        .zip(right.bytes())
        .map(|(left, right)| u64::from(left.abs_diff(right)))
        .sum()
}
