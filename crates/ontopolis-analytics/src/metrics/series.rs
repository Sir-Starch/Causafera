use ontopolis_explanation::{
    ClaimConfidence, ClaimEvidenceState, ComparisonContext, ExplanationClaim,
    ExplanationClaimSchemaId, ExplanationFrame, NumericClaimValue,
};
use ontopolis_types::TraceId;

use super::{
    AUTONOMOUS_PERSISTENCE_SCHEMA, AnalyticsCheckpoint, AnalyticsError, CAUSAL_DEPTH_SCHEMA,
    COUNTERFACTUAL_DISTANCE_SCHEMA, DRIVEN_EQUILIBRIUM_SCHEMA, ExperimentAnalytics,
    FieldInputState, PhenomenonMetrics, RECONSTRUCTABILITY_SCHEMA, TEMPORAL_SPAN_SCHEMA,
};
use crate::metrics::recovery::fingerprint_distance;

pub fn reconstructability_from_trace_density(checkpoints: &[AnalyticsCheckpoint]) -> (u64, u64) {
    let span = temporal_span(checkpoints).max(1);
    let traces = checkpoints
        .last()
        .map(|snapshot| snapshot.causal_trace_count)
        .unwrap_or(0);
    (traces.min(span), span)
}

pub fn analyze_checkpoint_series(
    checkpoints: &[AnalyticsCheckpoint],
    input_state: FieldInputState,
) -> Result<ExplanationFrame, AnalyticsError> {
    let checkpoint_time = checkpoints
        .last()
        .map(|snapshot| snapshot.time)
        .ok_or(AnalyticsError::EmptyCheckpointSeries)?;
    let evidence_traces = evidence_traces(checkpoints);
    let reconstructability = reconstructability_claim(checkpoints, &evidence_traces)?;
    let metrics = phenomenon_metrics(checkpoints);
    let claims = vec![
        reconstructability,
        metric_claim(
            CAUSAL_DEPTH_SCHEMA,
            metrics.causal_depth,
            &evidence_traces,
            ComparisonContext::None,
        )?,
        metric_claim(
            TEMPORAL_SPAN_SCHEMA,
            metrics.temporal_span,
            &evidence_traces,
            ComparisonContext::None,
        )?,
        metric_claim(
            COUNTERFACTUAL_DISTANCE_SCHEMA,
            metrics.counterfactual_state_distance,
            &evidence_traces,
            ComparisonContext::None,
        )?,
        stability_claim(checkpoints, input_state, &evidence_traces)?,
    ];
    ExplanationFrame::new(checkpoint_time, claims).map_err(AnalyticsError::Explanation)
}

fn reconstructability_claim(
    checkpoints: &[AnalyticsCheckpoint],
    evidence_traces: &[TraceId],
) -> Result<ExplanationClaim, AnalyticsError> {
    let (numerator, denominator) =
        ExperimentAnalytics::reconstructability_from_trace_density(checkpoints);
    let value =
        NumericClaimValue::ratio(numerator, denominator).map_err(AnalyticsError::Explanation)?;
    if evidence_traces.is_empty() {
        return ExplanationClaim::unsupported(
            RECONSTRUCTABILITY_SCHEMA,
            value,
            ComparisonContext::None,
        )
        .map_err(AnalyticsError::Explanation);
    }
    ExplanationClaim::new(
        RECONSTRUCTABILITY_SCHEMA,
        value,
        trace_density_confidence(numerator, denominator)?,
        evidence_traces.to_vec(),
        ComparisonContext::None,
        ClaimEvidenceState::Supported,
    )
    .map_err(AnalyticsError::Explanation)
}

fn metric_claim(
    schema: ExplanationClaimSchemaId,
    value: u64,
    evidence_traces: &[TraceId],
    comparison: ComparisonContext,
) -> Result<ExplanationClaim, AnalyticsError> {
    let value = NumericClaimValue::scalar(u64_to_i64_saturating(value));
    if evidence_traces.is_empty() {
        return ExplanationClaim::unknown(schema, value, comparison)
            .map_err(AnalyticsError::Explanation);
    }
    ExplanationClaim::new(
        schema,
        value,
        ClaimConfidence::new(0.75).map_err(AnalyticsError::Explanation)?,
        evidence_traces.to_vec(),
        comparison,
        ClaimEvidenceState::Supported,
    )
    .map_err(AnalyticsError::Explanation)
}

fn stability_claim(
    checkpoints: &[AnalyticsCheckpoint],
    input_state: FieldInputState,
    evidence_traces: &[TraceId],
) -> Result<ExplanationClaim, AnalyticsError> {
    let schema = match input_state {
        FieldInputState::ActiveInput => DRIVEN_EQUILIBRIUM_SCHEMA,
        FieldInputState::NoInput => AUTONOMOUS_PERSISTENCE_SCHEMA,
    };
    let Some(stable_transitions) = stable_transition_count(checkpoints) else {
        return ExplanationClaim::unknown(
            schema,
            NumericClaimValue::scalar(0),
            ComparisonContext::None,
        )
        .map_err(AnalyticsError::Explanation);
    };
    metric_claim(
        schema,
        stable_transitions,
        evidence_traces,
        ComparisonContext::None,
    )
}

fn trace_density_confidence(
    numerator: u64,
    denominator: u64,
) -> Result<ClaimConfidence, AnalyticsError> {
    if denominator == 0 {
        return ClaimConfidence::new(0.0).map_err(AnalyticsError::Explanation);
    }
    let confidence = u64_to_f64_saturating(numerator) / u64_to_f64_saturating(denominator);
    ClaimConfidence::new(confidence.min(1.0)).map_err(AnalyticsError::Explanation)
}

fn phenomenon_metrics(checkpoints: &[AnalyticsCheckpoint]) -> PhenomenonMetrics {
    PhenomenonMetrics {
        causal_depth: checkpoints
            .last()
            .map(|snapshot| snapshot.causal_trace_count)
            .unwrap_or(0),
        temporal_span: temporal_span(checkpoints),
        counterfactual_state_distance: checkpoints
            .first()
            .zip(checkpoints.last())
            .map(|(first, last)| fingerprint_distance(first.physical_state, last.physical_state))
            .unwrap_or(0),
    }
}

fn temporal_span(checkpoints: &[AnalyticsCheckpoint]) -> u64 {
    checkpoints
        .first()
        .zip(checkpoints.last())
        .map(|(first, last)| last.time.raw().saturating_sub(first.time.raw()))
        .unwrap_or(0)
}

fn stable_transition_count(checkpoints: &[AnalyticsCheckpoint]) -> Option<u64> {
    if checkpoints.len() < 2 {
        return None;
    }
    Some(
        checkpoints
            .windows(2)
            .filter(|pair| pair[0].mana_total == pair[1].mana_total)
            .fold(0_u64, |count, _| count + 1),
    )
}

fn u64_to_i64_saturating(value: u64) -> i64 {
    match i64::try_from(value) {
        Ok(value) => value,
        Err(_) => i64::MAX,
    }
}

fn u64_to_f64_saturating(value: u64) -> f64 {
    match u32::try_from(value) {
        Ok(value) => f64::from(value),
        Err(_) => f64::from(u32::MAX),
    }
}

fn evidence_traces(checkpoints: &[AnalyticsCheckpoint]) -> Vec<TraceId> {
    let mut traces = checkpoints
        .iter()
        .filter(|snapshot| snapshot.causal_trace_count > 0)
        .map(|snapshot| snapshot.latest_trace)
        .collect::<Vec<_>>();
    traces.sort_unstable();
    traces.dedup();
    traces
}
