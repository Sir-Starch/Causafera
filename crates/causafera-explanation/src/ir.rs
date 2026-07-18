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
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
