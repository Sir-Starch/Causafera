mod recovery;
mod series;

use causafera_core::StateFingerprint;
use causafera_explanation::{ExplanationClaimSchemaId, ExplanationFrame, ExplanationIrError};
use causafera_types::{SimulationTime, TraceId};
use thiserror::Error;

pub const RECONSTRUCTABILITY_SCHEMA: ExplanationClaimSchemaId = ExplanationClaimSchemaId::new(1);
pub const PATH_DEPENDENCE_SCHEMA: ExplanationClaimSchemaId = ExplanationClaimSchemaId::new(2);
pub const CAUSAL_DEPTH_SCHEMA: ExplanationClaimSchemaId = ExplanationClaimSchemaId::new(3);
pub const TEMPORAL_SPAN_SCHEMA: ExplanationClaimSchemaId = ExplanationClaimSchemaId::new(4);
pub const COUNTERFACTUAL_DISTANCE_SCHEMA: ExplanationClaimSchemaId =
    ExplanationClaimSchemaId::new(5);
pub const RECOVERY_DISTANCE_SCHEMA: ExplanationClaimSchemaId = ExplanationClaimSchemaId::new(6);
pub const TIME_TO_RECOVERY_SCHEMA: ExplanationClaimSchemaId = ExplanationClaimSchemaId::new(7);
pub const DRIVEN_EQUILIBRIUM_SCHEMA: ExplanationClaimSchemaId = ExplanationClaimSchemaId::new(8);
pub const AUTONOMOUS_PERSISTENCE_SCHEMA: ExplanationClaimSchemaId =
    ExplanationClaimSchemaId::new(9);

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SimulationMetrics {
    pub ticks_per_second: f64,
    pub active_updates: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PhenomenonMetrics {
    pub causal_depth: u64,
    pub temporal_span: u64,
    pub counterfactual_state_distance: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AnalyticsCheckpoint {
    pub time: SimulationTime,
    pub physical_state: StateFingerprint,
    pub history_state: StateFingerprint,
    pub mana_total: i64,
    pub causal_trace_count: u64,
    pub latest_trace: TraceId,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FieldInputState {
    ActiveInput,
    NoInput,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MatchedCheckpointDistance {
    pub checkpoint_time: SimulationTime,
    pub physical_distance: u64,
    pub history_diverged: bool,
    pub control_trace: TraceId,
    pub intervention_trace: TraceId,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MatchedCheckpointAnalysis {
    pub pre_intervention_baseline_distance: u64,
    pub perturbation_minimum_distance: u64,
    pub perturbation_maximum_distance: u64,
    pub matched_control_distances: Vec<MatchedCheckpointDistance>,
    pub final_recovery_distance: u64,
    pub time_to_recovery: Option<u64>,
}

pub struct ExperimentAnalytics;

impl ExperimentAnalytics {
    pub fn reconstructability_from_trace_density(
        checkpoints: &[AnalyticsCheckpoint],
    ) -> (u64, u64) {
        series::reconstructability_from_trace_density(checkpoints)
    }

    pub fn path_dependence_from_seed_sensitivity(diverged: u64, compared: u64) -> (u64, u64) {
        if compared == 0 {
            return (0, 1);
        }
        (diverged.min(compared), compared)
    }

    pub fn analyze_checkpoint_series(
        checkpoints: &[AnalyticsCheckpoint],
        input_state: FieldInputState,
    ) -> Result<ExplanationFrame, AnalyticsError> {
        series::analyze_checkpoint_series(checkpoints, input_state)
    }

    pub fn analyze_recovery(
        control: &[AnalyticsCheckpoint],
        intervention: &[AnalyticsCheckpoint],
        perturbation_from: SimulationTime,
        perturbation_through: SimulationTime,
        tolerance: u64,
    ) -> Result<MatchedCheckpointAnalysis, AnalyticsError> {
        recovery::analyze_recovery(
            control,
            intervention,
            perturbation_from,
            perturbation_through,
            tolerance,
        )
    }
}

#[derive(Clone, Debug, Error, PartialEq)]
pub enum AnalyticsError {
    #[error("checkpoint series must not be empty")]
    EmptyCheckpointSeries,
    #[error("matched checkpoint series must be non-empty and share identical checkpoint times")]
    MismatchedCheckpointSeries,
    #[error("matched recovery analysis requires a pre-intervention baseline checkpoint")]
    MissingBaselineCheckpoint,
    #[error("matched recovery analysis requires perturbation-window checkpoints")]
    MissingPerturbationCheckpoint,
    #[error(transparent)]
    Explanation(#[from] ExplanationIrError),
}

#[cfg(test)]
mod tests {
    use causafera_core::StateFingerprint;
    use causafera_explanation::{ClaimConfidence, ClaimEvidenceState};

    use super::*;

    fn snapshot(tick: u64, physical: u8, traces: u64, mana_total: i64) -> AnalyticsCheckpoint {
        AnalyticsCheckpoint {
            time: SimulationTime::new(tick),
            physical_state: StateFingerprint::new([physical; 32]),
            history_state: StateFingerprint::new([physical; 32]),
            mana_total,
            causal_trace_count: traces,
            latest_trace: TraceId::new(traces),
        }
    }

    #[test]
    fn identical_checkpoints_produce_identical_explanation_frames() {
        let checkpoints = [snapshot(0, 1, 1, 10), snapshot(10, 2, 2, 10)];

        let first = ExperimentAnalytics::analyze_checkpoint_series(
            &checkpoints,
            FieldInputState::ActiveInput,
        )
        .unwrap();
        let second = ExperimentAnalytics::analyze_checkpoint_series(
            &checkpoints,
            FieldInputState::ActiveInput,
        )
        .unwrap();

        assert_eq!(first, second);
    }

    #[test]
    fn missing_evidence_marks_reconstructability_unsupported() {
        let checkpoints = [snapshot(0, 1, 0, 10), snapshot(10, 2, 0, 10)];

        let frame = ExperimentAnalytics::analyze_checkpoint_series(
            &checkpoints,
            FieldInputState::ActiveInput,
        )
        .unwrap();
        let reconstructability = frame
            .claims
            .iter()
            .find(|claim| claim.schema == RECONSTRUCTABILITY_SCHEMA)
            .unwrap();

        assert_eq!(
            reconstructability.evidence_state,
            ClaimEvidenceState::Unsupported
        );
        assert_eq!(reconstructability.confidence, ClaimConfidence::ZERO);
    }

    #[test]
    fn recovery_distance_is_computed_from_matched_physical_digests() {
        let control = [
            snapshot(0, 1, 1, 10),
            snapshot(10, 1, 2, 10),
            snapshot(20, 1, 3, 10),
        ];
        let intervention = [
            snapshot(0, 1, 1, 10),
            snapshot(10, 3, 2, 10),
            snapshot(20, 2, 3, 10),
        ];

        let analysis = ExperimentAnalytics::analyze_recovery(
            &control,
            &intervention,
            SimulationTime::new(10),
            SimulationTime::new(10),
            32,
        )
        .unwrap();

        assert_eq!(analysis.pre_intervention_baseline_distance, 0);
        assert_eq!(analysis.perturbation_maximum_distance, 64);
        assert_eq!(analysis.final_recovery_distance, 32);
        assert_eq!(analysis.time_to_recovery, Some(10));
    }

    #[test]
    fn active_and_inactive_field_stability_use_distinct_claim_schemas() {
        let checkpoints = [snapshot(0, 1, 1, 10), snapshot(10, 1, 2, 10)];

        let driven = ExperimentAnalytics::analyze_checkpoint_series(
            &checkpoints,
            FieldInputState::ActiveInput,
        )
        .unwrap();
        let autonomous =
            ExperimentAnalytics::analyze_checkpoint_series(&checkpoints, FieldInputState::NoInput)
                .unwrap();

        assert!(
            driven
                .claims
                .iter()
                .any(|claim| claim.schema == DRIVEN_EQUILIBRIUM_SCHEMA)
        );
        assert!(
            autonomous
                .claims
                .iter()
                .any(|claim| claim.schema == AUTONOMOUS_PERSISTENCE_SCHEMA)
        );
    }
}
