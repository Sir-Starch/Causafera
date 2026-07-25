use causafera_domains::ThermalConservationReceipt;
use causafera_types::{ExperimentId, SimulationTime, TraceId};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ExplanationClaimSchemaId(u64);

impl ExplanationClaimSchemaId {
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    pub const fn raw(self) -> u64 {
        self.0
    }
}

pub const MATERIAL_SURFACE_LOOP_CLAIM_SCHEMA: ExplanationClaimSchemaId =
    ExplanationClaimSchemaId::new(10);
pub const MATERIAL_SURFACE_LOOP_WINDOW_SCHEMA: ExplanationClaimSchemaId =
    ExplanationClaimSchemaId::new(11);
pub const MATERIAL_SURFACE_LOOP_CONTROL_SCHEMA: ExplanationClaimSchemaId =
    ExplanationClaimSchemaId::new(12);
pub const MATERIAL_SURFACE_LOOP_MANA_SCHEMA: ExplanationClaimSchemaId =
    ExplanationClaimSchemaId::new(13);
pub const MATERIAL_SURFACE_LOOP_MANA_TRANSITION_SCHEMA: ExplanationClaimSchemaId =
    ExplanationClaimSchemaId::new(14);
pub const MATERIAL_SURFACE_LOOP_LOCAL_MANA_TRANSITION_SCHEMA: ExplanationClaimSchemaId =
    ExplanationClaimSchemaId::new(15);
pub const THERMAL_CARRIER_CONSERVATION_SCHEMA: ExplanationClaimSchemaId =
    ExplanationClaimSchemaId::new(16);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ComparisonCohortId(u64);

impl ComparisonCohortId {
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    pub const fn raw(self) -> u64 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct ClaimConfidence(f64);

impl ClaimConfidence {
    pub const ZERO: Self = Self(0.0);
    pub const ONE: Self = Self(1.0);

    pub fn new(value: f64) -> Result<Self, ExplanationIrError> {
        if !(0.0..=1.0).contains(&value) || value.is_nan() {
            return Err(ExplanationIrError::InvalidConfidence { value });
        }
        Ok(Self(value))
    }

    pub const fn raw(self) -> f64 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum NumericClaimValue {
    Scalar { value: i64 },
    Range { start: i64, end: i64 },
    Ratio { numerator: u64, denominator: u64 },
}

impl NumericClaimValue {
    pub const fn scalar(value: i64) -> Self {
        Self::Scalar { value }
    }

    pub fn range(start: i64, end: i64) -> Result<Self, ExplanationIrError> {
        if start > end {
            return Err(ExplanationIrError::InvalidRange { start, end });
        }
        Ok(Self::Range { start, end })
    }

    pub fn ratio(numerator: u64, denominator: u64) -> Result<Self, ExplanationIrError> {
        if denominator == 0 {
            return Err(ExplanationIrError::ZeroRatioDenominator);
        }
        Ok(Self::Ratio {
            numerator,
            denominator,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ComparisonContext {
    None,
    MatchedCohort { cohort: ComparisonCohortId },
    Counterfactual { cohort: ComparisonCohortId },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ClaimEvidenceState {
    Supported,
    Unsupported,
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ExplanationClaim {
    pub schema: ExplanationClaimSchemaId,
    pub value: NumericClaimValue,
    pub confidence: ClaimConfidence,
    pub evidence_traces: Vec<TraceId>,
    pub comparison: ComparisonContext,
    pub evidence_state: ClaimEvidenceState,
}

impl ExplanationClaim {
    pub fn new(
        schema: ExplanationClaimSchemaId,
        value: NumericClaimValue,
        confidence: ClaimConfidence,
        evidence_traces: Vec<TraceId>,
        comparison: ComparisonContext,
        evidence_state: ClaimEvidenceState,
    ) -> Result<Self, ExplanationIrError> {
        let mut evidence_traces = evidence_traces;
        evidence_traces.sort_unstable();
        evidence_traces.dedup();
        let confidence = match evidence_state {
            ClaimEvidenceState::Supported => {
                if evidence_traces.is_empty() {
                    return Err(ExplanationIrError::SupportedClaimWithoutEvidence { schema });
                }
                confidence
            }
            ClaimEvidenceState::Unsupported | ClaimEvidenceState::Unknown => ClaimConfidence::ZERO,
        };
        Ok(Self {
            schema,
            value,
            confidence,
            evidence_traces,
            comparison,
            evidence_state,
        })
    }

    pub fn unsupported(
        schema: ExplanationClaimSchemaId,
        value: NumericClaimValue,
        comparison: ComparisonContext,
    ) -> Result<Self, ExplanationIrError> {
        Self::new(
            schema,
            value,
            ClaimConfidence::ZERO,
            Vec::new(),
            comparison,
            ClaimEvidenceState::Unsupported,
        )
    }

    pub fn unknown(
        schema: ExplanationClaimSchemaId,
        value: NumericClaimValue,
        comparison: ComparisonContext,
    ) -> Result<Self, ExplanationIrError> {
        Self::new(
            schema,
            value,
            ClaimConfidence::ZERO,
            Vec::new(),
            comparison,
            ClaimEvidenceState::Unknown,
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MaterialSurfaceLoopClaim {
    pub before_condition: i64,
    pub after_condition: i64,
    pub mana_total: i64,
    pub observation_start: SimulationTime,
    pub observation_end: SimulationTime,
    pub contact_trace: TraceId,
    pub mana_effect_trace: Option<TraceId>,
    pub mana_transition_trace: Option<TraceId>,
    pub mana_before: Option<i64>,
    pub mana_after: Option<i64>,
    pub repeated_structure_observed: bool,
}

impl MaterialSurfaceLoopClaim {
    pub fn to_explanation_claims(self) -> Result<Vec<ExplanationClaim>, ExplanationIrError> {
        let value = NumericClaimValue::range(self.before_condition, self.after_condition)?;
        let window = NumericClaimValue::range(
            i64::try_from(self.observation_start.raw())
                .map_err(|_| ExplanationIrError::ObservationTimeOverflow)?,
            i64::try_from(self.observation_end.raw())
                .map_err(|_| ExplanationIrError::ObservationTimeOverflow)?,
        )?;
        let mut trace_anchors = vec![self.contact_trace];
        if let Some(trace) = self.mana_effect_trace {
            trace_anchors.push(trace);
        }
        let primary = match (self.repeated_structure_observed, self.mana_effect_trace) {
            (true, Some(_)) => ExplanationClaim::new(
                MATERIAL_SURFACE_LOOP_CLAIM_SCHEMA,
                value,
                ClaimConfidence::ONE,
                trace_anchors.clone(),
                ComparisonContext::None,
                ClaimEvidenceState::Supported,
            )?,
            (false, _) | (_, None) => ExplanationClaim::unknown(
                MATERIAL_SURFACE_LOOP_CLAIM_SCHEMA,
                value,
                ComparisonContext::None,
            )?,
        };
        let window_claim = ExplanationClaim::new(
            MATERIAL_SURFACE_LOOP_WINDOW_SCHEMA,
            window,
            ClaimConfidence::ONE,
            trace_anchors.clone(),
            ComparisonContext::None,
            ClaimEvidenceState::Supported,
        )?;
        let control_claim = ExplanationClaim::new(
            MATERIAL_SURFACE_LOOP_CONTROL_SCHEMA,
            NumericClaimValue::ratio(u64::from(self.repeated_structure_observed), 1)?,
            ClaimConfidence::ONE,
            vec![self.contact_trace],
            ComparisonContext::None,
            ClaimEvidenceState::Supported,
        )?;
        let mana_claim = ExplanationClaim::new(
            MATERIAL_SURFACE_LOOP_MANA_SCHEMA,
            NumericClaimValue::scalar(self.mana_total),
            ClaimConfidence::ONE,
            trace_anchors,
            ComparisonContext::None,
            ClaimEvidenceState::Supported,
        )?;
        let mana_transition_claim = match (
            self.mana_transition_trace,
            self.mana_before,
            self.mana_after,
        ) {
            (Some(trace), Some(before), Some(after)) => ExplanationClaim::new(
                MATERIAL_SURFACE_LOOP_MANA_TRANSITION_SCHEMA,
                NumericClaimValue::range(before, after)?,
                ClaimConfidence::ONE,
                vec![trace],
                ComparisonContext::None,
                ClaimEvidenceState::Supported,
            )?,
            _ => ExplanationClaim::unknown(
                MATERIAL_SURFACE_LOOP_MANA_TRANSITION_SCHEMA,
                NumericClaimValue::scalar(self.mana_total),
                ComparisonContext::None,
            )?,
        };
        Ok(vec![
            primary,
            window_claim,
            control_claim,
            mana_claim,
            mana_transition_claim,
        ])
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MaterialSurfaceLocalManaTransitionClaim {
    pub local_mana_before: i64,
    pub local_mana_after: i64,
    pub local_mana_trace: TraceId,
    pub gate_transition_trace: TraceId,
    pub contact_trace: Option<TraceId>,
}

impl MaterialSurfaceLocalManaTransitionClaim {
    pub fn to_explanation_claim(self) -> Result<ExplanationClaim, ExplanationIrError> {
        let mut evidence_traces = vec![self.local_mana_trace, self.gate_transition_trace];
        if let Some(contact_trace) = self.contact_trace {
            evidence_traces.push(contact_trace);
        }
        ExplanationClaim::new(
            MATERIAL_SURFACE_LOOP_LOCAL_MANA_TRANSITION_SCHEMA,
            NumericClaimValue::range(
                self.local_mana_before.min(self.local_mana_after),
                self.local_mana_before.max(self.local_mana_after),
            )?,
            ClaimConfidence::ONE,
            evidence_traces,
            ComparisonContext::None,
            ClaimEvidenceState::Supported,
        )
    }

    pub fn unknown(mana_total: i64) -> Result<ExplanationClaim, ExplanationIrError> {
        ExplanationClaim::unknown(
            MATERIAL_SURFACE_LOOP_LOCAL_MANA_TRANSITION_SCHEMA,
            NumericClaimValue::scalar(mana_total),
            ComparisonContext::None,
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ThermalCarrierConservationClaim {
    pub receipt: ThermalConservationReceipt,
    pub observation_start: SimulationTime,
    pub observation_end: SimulationTime,
    pub conservation_trace: TraceId,
    pub reservoir_transfer_traces: Vec<TraceId>,
    pub neighbor_transfer_traces: Vec<TraceId>,
}

impl ThermalCarrierConservationClaim {
    pub fn to_explanation_claim(&self) -> Result<ExplanationClaim, ExplanationIrError> {
        if self.receipt.residual != 0 {
            return Err(ExplanationIrError::ThermalConservationResidual {
                residual: self.receipt.residual,
            });
        }
        let _window = NumericClaimValue::range(
            i64::try_from(self.observation_start.raw())
                .map_err(|_| ExplanationIrError::ObservationTimeOverflow)?,
            i64::try_from(self.observation_end.raw())
                .map_err(|_| ExplanationIrError::ObservationTimeOverflow)?,
        )?;
        let mut evidence_traces = Vec::with_capacity(
            1 + self.reservoir_transfer_traces.len() + self.neighbor_transfer_traces.len(),
        );
        evidence_traces.push(self.conservation_trace);
        evidence_traces.extend_from_slice(&self.reservoir_transfer_traces);
        evidence_traces.extend_from_slice(&self.neighbor_transfer_traces);
        ExplanationClaim::new(
            THERMAL_CARRIER_CONSERVATION_SCHEMA,
            NumericClaimValue::scalar(0),
            ClaimConfidence::ONE,
            evidence_traces,
            ComparisonContext::None,
            ClaimEvidenceState::Supported,
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum FrameAssessment {
    Supported,
    Partial,
    Unsupported,
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ExplanationFrame {
    pub checkpoint_time: SimulationTime,
    pub claims: Vec<ExplanationClaim>,
    pub overall_assessment: FrameAssessment,
}

impl ExplanationFrame {
    pub fn new(
        checkpoint_time: SimulationTime,
        claims: Vec<ExplanationClaim>,
    ) -> Result<Self, ExplanationIrError> {
        if claims.is_empty() {
            return Err(ExplanationIrError::FrameWithoutClaims { checkpoint_time });
        }
        let mut claims = claims;
        claims.sort_by_key(|claim| (claim.schema, claim.comparison));
        let overall_assessment = assess_claims(&claims);
        Ok(Self {
            checkpoint_time,
            claims,
            overall_assessment,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ExplanationReport {
    pub experiment: ExperimentId,
    pub frames: Vec<ExplanationFrame>,
    pub overall_assessment: FrameAssessment,
}

impl ExplanationReport {
    pub fn new(
        experiment: ExperimentId,
        frames: Vec<ExplanationFrame>,
    ) -> Result<Self, ExplanationIrError> {
        if frames.is_empty() {
            return Err(ExplanationIrError::ReportWithoutFrames { experiment });
        }
        let mut frames = frames;
        frames.sort_by_key(|frame| frame.checkpoint_time);
        let overall_assessment = assess_frames(&frames);
        Ok(Self {
            experiment,
            frames,
            overall_assessment,
        })
    }
}

fn assess_claims(claims: &[ExplanationClaim]) -> FrameAssessment {
    let supported = claims
        .iter()
        .filter(|claim| claim.evidence_state == ClaimEvidenceState::Supported)
        .count();
    if supported == claims.len() {
        return FrameAssessment::Supported;
    }
    if supported > 0 {
        return FrameAssessment::Partial;
    }
    if claims
        .iter()
        .any(|claim| claim.evidence_state == ClaimEvidenceState::Unsupported)
    {
        return FrameAssessment::Unsupported;
    }
    FrameAssessment::Unknown
}

fn assess_frames(frames: &[ExplanationFrame]) -> FrameAssessment {
    let supported = frames
        .iter()
        .filter(|frame| frame.overall_assessment == FrameAssessment::Supported)
        .count();
    if supported == frames.len() {
        return FrameAssessment::Supported;
    }
    if supported > 0
        || frames
            .iter()
            .any(|frame| frame.overall_assessment == FrameAssessment::Partial)
    {
        return FrameAssessment::Partial;
    }
    if frames
        .iter()
        .any(|frame| frame.overall_assessment == FrameAssessment::Unsupported)
    {
        return FrameAssessment::Unsupported;
    }
    FrameAssessment::Unknown
}

#[derive(Clone, Debug, Error, PartialEq)]
pub enum ExplanationIrError {
    #[error("claim confidence must be within [0.0, 1.0], got {value}")]
    InvalidConfidence { value: f64 },
    #[error("numeric claim range must be ordered, got {start}..{end}")]
    InvalidRange { start: i64, end: i64 },
    #[error("numeric claim ratio denominator must be non-zero")]
    ZeroRatioDenominator,
    #[error("supported claim {schema:?} must reference at least one causal trace")]
    SupportedClaimWithoutEvidence { schema: ExplanationClaimSchemaId },
    #[error("explanation frame at {checkpoint_time:?} must contain at least one claim")]
    FrameWithoutClaims { checkpoint_time: SimulationTime },
    #[error("explanation report for {experiment:?} must contain at least one frame")]
    ReportWithoutFrames { experiment: ExperimentId },
    #[error("material-surface observation time exceeds Explanation IR range")]
    ObservationTimeOverflow,
    #[error("thermal conservation receipt residual must be zero, got {residual}")]
    ThermalConservationResidual { residual: i128 },
}

#[cfg(test)]
mod tests {
    use super::*;
    use causafera_domains::ThermalConservationReceipt;

    #[test]
    fn claim_sorts_and_deduplicates_supporting_traces() {
        let claim = ExplanationClaim::new(
            ExplanationClaimSchemaId::new(9),
            NumericClaimValue::scalar(12),
            ClaimConfidence::new(0.8).unwrap(),
            vec![TraceId::new(3), TraceId::new(1), TraceId::new(3)],
            ComparisonContext::None,
            ClaimEvidenceState::Supported,
        )
        .unwrap();

        assert_eq!(
            claim.evidence_traces,
            vec![TraceId::new(1), TraceId::new(3)]
        );
    }

    #[test]
    fn missing_evidence_marks_claim_unsupported_with_zero_confidence() {
        let claim = ExplanationClaim::unsupported(
            ExplanationClaimSchemaId::new(1),
            NumericClaimValue::scalar(0),
            ComparisonContext::None,
        )
        .unwrap();

        assert_eq!(claim.evidence_state, ClaimEvidenceState::Unsupported);
        assert_eq!(claim.confidence, ClaimConfidence::ZERO);
        assert!(claim.evidence_traces.is_empty());
    }

    #[test]
    fn supported_claim_without_evidence_is_rejected() {
        let err = ExplanationClaim::new(
            ExplanationClaimSchemaId::new(1),
            NumericClaimValue::scalar(5),
            ClaimConfidence::new(0.7).unwrap(),
            Vec::new(),
            ComparisonContext::None,
            ClaimEvidenceState::Supported,
        )
        .unwrap_err();

        assert_eq!(
            err,
            ExplanationIrError::SupportedClaimWithoutEvidence {
                schema: ExplanationClaimSchemaId::new(1)
            }
        );
    }

    #[test]
    fn material_surface_loop_claim_emits_typed_window_and_control_claims() {
        // Given: a traced mana-mediated material transition over an observed runtime window.
        let claim = MaterialSurfaceLoopClaim {
            before_condition: 4,
            after_condition: 6,
            mana_total: 12,
            observation_start: SimulationTime::new(3),
            observation_end: SimulationTime::new(8),
            contact_trace: TraceId::new(21),
            mana_effect_trace: Some(TraceId::new(22)),
            mana_transition_trace: Some(TraceId::new(23)),
            mana_before: Some(0),
            mana_after: Some(3),
            repeated_structure_observed: true,
        };

        // When: the non-authoritative input is converted into Explanation IR.
        let claims = claim.to_explanation_claims().unwrap();

        // Then: typed values, window, controls, and causal anchors are available to observers.
        assert_eq!(claims.len(), 5);
        assert_eq!(claims[0].schema, MATERIAL_SURFACE_LOOP_CLAIM_SCHEMA);
        assert_eq!(claims[0].evidence_state, ClaimEvidenceState::Supported);
        assert_eq!(
            claims[0].evidence_traces,
            vec![TraceId::new(21), TraceId::new(22)]
        );
        assert_eq!(claims[1].schema, MATERIAL_SURFACE_LOOP_WINDOW_SCHEMA);
        assert_eq!(
            claims[1].value,
            NumericClaimValue::Range { start: 3, end: 8 }
        );
        assert_eq!(claims[2].schema, MATERIAL_SURFACE_LOOP_CONTROL_SCHEMA);
        assert_eq!(claims[2].value, NumericClaimValue::ratio(1, 1).unwrap());
        assert_eq!(claims[3].schema, MATERIAL_SURFACE_LOOP_MANA_SCHEMA);
        assert_eq!(claims[3].value, NumericClaimValue::scalar(12));
        assert_eq!(
            claims[4].schema,
            MATERIAL_SURFACE_LOOP_MANA_TRANSITION_SCHEMA
        );
        assert_eq!(
            claims[4].value,
            NumericClaimValue::Range { start: 0, end: 3 }
        );
        assert_eq!(claims[4].evidence_state, ClaimEvidenceState::Supported);
        assert_eq!(claims[4].evidence_traces, vec![TraceId::new(23)]);
    }

    #[test]
    fn thermal_conservation_claim_uses_authoritative_zero_residual() {
        // Given: a committed receipt whose residual was checked by authoritative thermal evolution.
        let receipt = ThermalConservationReceipt {
            tick: 12,
            total_cell_energy_before: 100,
            total_cell_energy_after: 160,
            total_reservoir_budget_before: 80,
            total_reservoir_budget_after: 20,
            residual: 0,
        };
        let claim = ThermalCarrierConservationClaim {
            receipt,
            observation_start: SimulationTime::new(11),
            observation_end: SimulationTime::new(12),
            conservation_trace: TraceId::new(31),
            reservoir_transfer_traces: vec![TraceId::new(29)],
            neighbor_transfer_traces: vec![TraceId::new(30)],
        };

        // When: the receipt is converted into a non-authoritative explanation claim.
        let actual = claim.to_explanation_claim().unwrap();

        // Then: the claim preserves the stored zero residual and all causal support.
        assert_eq!(actual.schema, THERMAL_CARRIER_CONSERVATION_SCHEMA);
        assert_eq!(actual.value, NumericClaimValue::scalar(0));
        assert_eq!(actual.evidence_state, ClaimEvidenceState::Supported);
        assert_eq!(
            actual.evidence_traces,
            vec![TraceId::new(29), TraceId::new(30), TraceId::new(31)]
        );
    }
}
