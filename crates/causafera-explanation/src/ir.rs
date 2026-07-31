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
pub const MATERIAL_SURFACE_THERMAL_EXCHANGE_SCHEMA: ExplanationClaimSchemaId =
    ExplanationClaimSchemaId::new(17);
pub const HISTORICAL_BOOTSTRAP_RECORD_SCHEMA: ExplanationClaimSchemaId =
    ExplanationClaimSchemaId::new(18);
pub const HISTORICAL_BOOTSTRAP_WINDOW_SCHEMA: ExplanationClaimSchemaId =
    ExplanationClaimSchemaId::new(19);
/// The smallest and largest per-carrier volume in a scope, as two claims.
///
/// Two claims rather than one `Range`: `Range` is `i64` on both ends and a water
/// volume is a `u64`, so a range would silently lose the upper half of what a
/// carrier can hold. Each bound travels as `Ratio { volume, 1 }`, which is
/// lossless without widening `NumericClaimValue` or its wire schema.
pub const HYDROLOGY_STORAGE_MINIMUM_SCHEMA: ExplanationClaimSchemaId =
    ExplanationClaimSchemaId::new(20);
pub const HYDROLOGY_STORAGE_MAXIMUM_SCHEMA: ExplanationClaimSchemaId =
    ExplanationClaimSchemaId::new(21);
/// A scope's whole storage total. Insufficiency above `u64::MAX`, never a
/// narrowed number.
pub const HYDROLOGY_STORAGE_TOTAL_SCHEMA: ExplanationClaimSchemaId =
    ExplanationClaimSchemaId::new(22);
/// Water-table elevation in millimetres, which is signed and therefore a
/// genuine `Range`.
pub const HYDROLOGY_WATER_TABLE_RANGE_SCHEMA: ExplanationClaimSchemaId =
    ExplanationClaimSchemaId::new(23);
pub const HYDROLOGY_FORCING_ACCEPTED_SCHEMA: ExplanationClaimSchemaId =
    ExplanationClaimSchemaId::new(24);
pub const HYDROLOGY_FORCING_UNMET_SCHEMA: ExplanationClaimSchemaId =
    ExplanationClaimSchemaId::new(25);
pub const HYDROLOGY_TRANSFER_ACCEPTED_SCHEMA: ExplanationClaimSchemaId =
    ExplanationClaimSchemaId::new(26);
/// What a limiter refused. Emitted at zero as well: a bound that did not engage
/// is evidence, and its absence would make "nothing was refused" and "nothing
/// was asked" the same answer.
pub const HYDROLOGY_TRANSFER_LIMITED_SCHEMA: ExplanationClaimSchemaId =
    ExplanationClaimSchemaId::new(27);
pub const HYDROLOGY_CONSERVATION_RESIDUAL_SCHEMA: ExplanationClaimSchemaId =
    ExplanationClaimSchemaId::new(28);
pub const HYDROLOGY_BOUNDARY_EXPORT_SCHEMA: ExplanationClaimSchemaId =
    ExplanationClaimSchemaId::new(29);

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

/// That the current initial state has a validated canonical bootstrap record.
///
/// The claim reports how many stages the plan declares, how many terminal
/// receipts closed them, and the bounded canonical window they span; its
/// evidence is the receipts' completion traces and their declared dependency
/// anchors. It deliberately does not render the opaque process schema IDs, name
/// what a stage was for, or infer why it exists — a downstream reader follows
/// the trace anchors to the committed effects, which is where each stage's
/// result identity actually lives. A fingerprint is an equality identity, not a
/// magnitude, so it is never reported as a numeric claim value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HistoricalBootstrapRecordClaim {
    pub stage_count: u32,
    pub receipt_count: u32,
    pub observation_start: SimulationTime,
    pub observation_end: SimulationTime,
    pub completion_traces: Vec<TraceId>,
    pub dependency_traces: Vec<TraceId>,
}

impl HistoricalBootstrapRecordClaim {
    /// Whether the record is complete enough to support a claim at all.
    pub const fn is_supported(&self) -> bool {
        self.stage_count > 0
            && self.receipt_count == self.stage_count
            && self.completion_traces.len() == self.receipt_count as usize
    }

    /// The completeness claim, or the existing unknown state with zero
    /// confidence when the record is incomplete or unevidenced.
    pub fn to_explanation_claim(&self) -> Result<ExplanationClaim, ExplanationIrError> {
        let value = NumericClaimValue::scalar(i64::from(self.receipt_count));
        if !self.is_supported() {
            return ExplanationClaim::unknown(
                HISTORICAL_BOOTSTRAP_RECORD_SCHEMA,
                value,
                ComparisonContext::None,
            );
        }
        let mut evidence_traces = self.completion_traces.clone();
        evidence_traces.extend_from_slice(&self.dependency_traces);
        evidence_traces.sort_unstable();
        evidence_traces.dedup();
        ExplanationClaim::new(
            HISTORICAL_BOOTSTRAP_RECORD_SCHEMA,
            value,
            ClaimConfidence::ONE,
            evidence_traces,
            ComparisonContext::None,
            ClaimEvidenceState::Supported,
        )
    }

    /// The bounded canonical window the record spans.
    pub fn to_window_claim(&self) -> Result<ExplanationClaim, ExplanationIrError> {
        let start = i64::try_from(self.observation_start.raw())
            .map_err(|_| ExplanationIrError::ObservationTimeOverflow)?;
        let end = i64::try_from(self.observation_end.raw())
            .map_err(|_| ExplanationIrError::ObservationTimeOverflow)?;
        let value = NumericClaimValue::range(start.min(end), start.max(end))?;
        if !self.is_supported() {
            return ExplanationClaim::unknown(
                HISTORICAL_BOOTSTRAP_WINDOW_SCHEMA,
                value,
                ComparisonContext::None,
            );
        }
        ExplanationClaim::new(
            HISTORICAL_BOOTSTRAP_WINDOW_SCHEMA,
            value,
            ClaimConfidence::ONE,
            self.completion_traces.clone(),
            ComparisonContext::None,
            ClaimEvidenceState::Supported,
        )
    }
}

/// A material surface's retained-heat exchange with its co-located thermal cell
/// (`TODO-THERMAL-002`), scoped by `MaterialSurfaceId` and independent of the surface's
/// mana/contact history: thermal exchange runs every tick regardless of whether an actor has
/// ever touched the surface.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MaterialSurfaceThermalExchangeClaim {
    pub before_retained: i64,
    pub after_retained: i64,
    pub transition_trace: TraceId,
    pub cell_trace: TraceId,
}

impl MaterialSurfaceThermalExchangeClaim {
    pub fn to_explanation_claim(self) -> Result<ExplanationClaim, ExplanationIrError> {
        ExplanationClaim::new(
            MATERIAL_SURFACE_THERMAL_EXCHANGE_SCHEMA,
            NumericClaimValue::range(
                self.before_retained.min(self.after_retained),
                self.before_retained.max(self.after_retained),
            )?,
            ClaimConfidence::ONE,
            vec![self.transition_trace, self.cell_trace],
            ComparisonContext::None,
            ClaimEvidenceState::Supported,
        )
    }

    /// The claim for a surface with no exchange evidence in the bounded transition history,
    /// carrying its current retained energy as the honest "here is the state, here is why there
    /// is no transition evidence" value.
    pub fn unknown(retained_energy: i64) -> Result<ExplanationClaim, ExplanationIrError> {
        ExplanationClaim::unknown(
            MATERIAL_SURFACE_THERMAL_EXCHANGE_SCHEMA,
            NumericClaimValue::scalar(retained_energy),
            ComparisonContext::None,
        )
    }
}

/* --------------------------------------------------------------- hydrology -- */

/// One exact water volume, as a lossless ratio over one.
///
/// `NumericClaimValue` has no unsigned variant and does not gain one here:
/// widening it would change `explanation.proto`, the Rust codec, and the
/// TypeScript codec at once, for a quantity the existing `Ratio` already carries
/// exactly. A denominator of one is not a fraction standing in for a number — it
/// is the number, in the one variant of this schema whose numerator is a `u64`.
pub fn exact_volume(volume: u64) -> Result<NumericClaimValue, ExplanationIrError> {
    NumericClaimValue::ratio(volume, 1)
}

/// A scope's storage bounds, whole total, and water-table span.
///
/// `carrier_count` is carried so a scope that turned out to be empty can be
/// reported as empty rather than as a set of zero-valued measurements.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HydrologyStorageClaim {
    pub carrier_count: u64,
    pub minimum_volume: u64,
    pub maximum_volume: u64,
    /// The whole-scope total, which is `u128` because it sums `u64` volumes.
    pub total_volume: u128,
    /// Water-table elevation in millimetres. Signed, and a genuine range.
    pub water_table_minimum_mm: i64,
    pub water_table_maximum_mm: i64,
    pub evidence_traces: Vec<TraceId>,
}

impl HydrologyStorageClaim {
    /// Four claims: the two bounds, the total, and the water-table range.
    ///
    /// An empty scope, or one with no trace evidence, yields four `Unknown`
    /// claims rather than four zeroes: "no carrier is in scope" and "every
    /// carrier holds nothing" are different facts.
    pub fn to_explanation_claims(&self) -> Result<Vec<ExplanationClaim>, ExplanationIrError> {
        if self.carrier_count == 0 || self.evidence_traces.is_empty() {
            return Ok(vec![
                ExplanationClaim::unknown(
                    HYDROLOGY_STORAGE_MINIMUM_SCHEMA,
                    exact_volume(0)?,
                    ComparisonContext::None,
                )?,
                ExplanationClaim::unknown(
                    HYDROLOGY_STORAGE_MAXIMUM_SCHEMA,
                    exact_volume(0)?,
                    ComparisonContext::None,
                )?,
                ExplanationClaim::unknown(
                    HYDROLOGY_STORAGE_TOTAL_SCHEMA,
                    exact_volume(0)?,
                    ComparisonContext::None,
                )?,
                ExplanationClaim::unknown(
                    HYDROLOGY_WATER_TABLE_RANGE_SCHEMA,
                    NumericClaimValue::scalar(0),
                    ComparisonContext::None,
                )?,
            ]);
        }
        let supported = |schema, value| {
            ExplanationClaim::new(
                schema,
                value,
                ClaimConfidence::ONE,
                self.evidence_traces.clone(),
                ComparisonContext::None,
                ClaimEvidenceState::Supported,
            )
        };
        // A total wider than `u64` is not reported as a smaller number. The
        // scope is real and the measurement exists; this schema cannot carry it,
        // and saying so is the only honest answer available.
        let total = match u64::try_from(self.total_volume) {
            Ok(total) => supported(HYDROLOGY_STORAGE_TOTAL_SCHEMA, exact_volume(total)?)?,
            Err(_) => ExplanationClaim::unknown(
                HYDROLOGY_STORAGE_TOTAL_SCHEMA,
                exact_volume(0)?,
                ComparisonContext::None,
            )?,
        };
        Ok(vec![
            supported(
                HYDROLOGY_STORAGE_MINIMUM_SCHEMA,
                exact_volume(self.minimum_volume)?,
            )?,
            supported(
                HYDROLOGY_STORAGE_MAXIMUM_SCHEMA,
                exact_volume(self.maximum_volume)?,
            )?,
            total,
            supported(
                HYDROLOGY_WATER_TABLE_RANGE_SCHEMA,
                NumericClaimValue::range(self.water_table_minimum_mm, self.water_table_maximum_mm)?,
            )?,
        ])
    }
}

/// One applied forcing record's accepted and unmet volumes, with its ancestry.
///
/// `origin_trace` is the producer's own event, and it leads the evidence list:
/// a forcing claim whose ancestry stopped at the settlement would say what
/// arrived without saying where it came from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HydrologyForcingClaim {
    pub scheduled_tick: u64,
    pub forcing_id: u64,
    pub accepted_source: u128,
    pub unmet_source: u64,
    pub accepted_evapotranspiration: u64,
    pub unmet_evapotranspiration: u64,
    pub origin_trace: TraceId,
    pub settlement_traces: Vec<TraceId>,
}

impl HydrologyForcingClaim {
    pub fn to_explanation_claims(&self) -> Result<Vec<ExplanationClaim>, ExplanationIrError> {
        let mut evidence = Vec::with_capacity(1 + self.settlement_traces.len());
        evidence.push(self.origin_trace);
        evidence.extend_from_slice(&self.settlement_traces);
        let accepted = match u64::try_from(self.accepted_source) {
            Ok(accepted) => ExplanationClaim::new(
                HYDROLOGY_FORCING_ACCEPTED_SCHEMA,
                exact_volume(accepted)?,
                ClaimConfidence::ONE,
                evidence.clone(),
                ComparisonContext::None,
                ClaimEvidenceState::Supported,
            )?,
            Err(_) => ExplanationClaim::unknown(
                HYDROLOGY_FORCING_ACCEPTED_SCHEMA,
                exact_volume(0)?,
                ComparisonContext::None,
            )?,
        };
        // Unmet source and unmet evapotranspiration are summed into one claim
        // because both answer the same question — what the record asked for and
        // did not get — and the two are already separated by the transfer
        // receipts the evidence points at.
        let unmet = self
            .unmet_source
            .saturating_add(self.unmet_evapotranspiration);
        Ok(vec![
            accepted,
            ExplanationClaim::new(
                HYDROLOGY_FORCING_UNMET_SCHEMA,
                exact_volume(unmet)?,
                ClaimConfidence::ONE,
                evidence,
                ComparisonContext::None,
                ClaimEvidenceState::Supported,
            )?,
        ])
    }

    /// The answer for a record whose typed evidence has been evicted.
    pub fn unknown() -> Result<Vec<ExplanationClaim>, ExplanationIrError> {
        Ok(vec![
            ExplanationClaim::unknown(
                HYDROLOGY_FORCING_ACCEPTED_SCHEMA,
                exact_volume(0)?,
                ComparisonContext::None,
            )?,
            ExplanationClaim::unknown(
                HYDROLOGY_FORCING_UNMET_SCHEMA,
                exact_volume(0)?,
                ComparisonContext::None,
            )?,
        ])
    }
}

/// One transfer's accepted volume and what its limiter refused.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HydrologyTransferPathClaim {
    pub process_kind: u32,
    pub requested_volume: u64,
    pub accepted_volume: u64,
    pub unaccepted_volume: u64,
    pub transfer_trace: TraceId,
    pub conservation_trace: TraceId,
    pub forcing_origin_trace: Option<TraceId>,
}

impl HydrologyTransferPathClaim {
    pub fn to_explanation_claims(&self) -> Result<Vec<ExplanationClaim>, ExplanationIrError> {
        // Accepting more than was requested is not a generous process, it is a
        // source of water no ledger term accounts for; a claim built on one
        // would be evidence for something that did not happen.
        if self
            .requested_volume
            .checked_sub(self.accepted_volume)
            .is_none_or(|remainder| remainder != self.unaccepted_volume)
        {
            return Err(ExplanationIrError::HydrologyTransferDoesNotClose {
                requested: self.requested_volume,
                accepted: self.accepted_volume,
                unaccepted: self.unaccepted_volume,
            });
        }
        let mut evidence = vec![self.transfer_trace, self.conservation_trace];
        evidence.extend(self.forcing_origin_trace);
        let supported = |schema, value| {
            ExplanationClaim::new(
                schema,
                value,
                ClaimConfidence::ONE,
                evidence.clone(),
                ComparisonContext::None,
                ClaimEvidenceState::Supported,
            )
        };
        Ok(vec![
            supported(
                HYDROLOGY_TRANSFER_ACCEPTED_SCHEMA,
                exact_volume(self.accepted_volume)?,
            )?,
            supported(
                HYDROLOGY_TRANSFER_LIMITED_SCHEMA,
                exact_volume(self.unaccepted_volume)?,
            )?,
        ])
    }
}

/// One batch's exact conservation residual and its boundary export.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HydrologyConservationClaim {
    pub residual: i128,
    pub boundary_exports: i128,
    pub conservation_trace: TraceId,
    pub transfer_traces: Vec<TraceId>,
}

impl HydrologyConservationClaim {
    pub fn to_explanation_claims(&self) -> Result<Vec<ExplanationClaim>, ExplanationIrError> {
        let mut evidence = Vec::with_capacity(1 + self.transfer_traces.len());
        evidence.push(self.conservation_trace);
        evidence.extend_from_slice(&self.transfer_traces);
        // A committed batch closed exactly, so the residual claim is a literal
        // zero. A nonzero residual is not a smaller claim about a bigger number
        // — it means this batch is not committed evidence — so it answers with
        // insufficiency rather than reporting the discrepancy as a measurement.
        // Thermal errors here instead; hydrology's contract is insufficiency.
        let residual = if self.residual == 0 {
            ExplanationClaim::new(
                HYDROLOGY_CONSERVATION_RESIDUAL_SCHEMA,
                NumericClaimValue::scalar(0),
                ClaimConfidence::ONE,
                evidence.clone(),
                ComparisonContext::None,
                ClaimEvidenceState::Supported,
            )?
        } else {
            ExplanationClaim::unknown(
                HYDROLOGY_CONSERVATION_RESIDUAL_SCHEMA,
                NumericClaimValue::scalar(0),
                ComparisonContext::None,
            )?
        };
        let exported = match u64::try_from(self.boundary_exports) {
            Ok(exported) => ExplanationClaim::new(
                HYDROLOGY_BOUNDARY_EXPORT_SCHEMA,
                exact_volume(exported)?,
                ClaimConfidence::ONE,
                evidence,
                ComparisonContext::None,
                ClaimEvidenceState::Supported,
            )?,
            // Negative or wider than `u64`: an export term that cannot be one.
            Err(_) => ExplanationClaim::unknown(
                HYDROLOGY_BOUNDARY_EXPORT_SCHEMA,
                exact_volume(0)?,
                ComparisonContext::None,
            )?,
        };
        Ok(vec![residual, exported])
    }

    /// The answer for a scope with no retained batch to report on.
    pub fn unknown() -> Result<Vec<ExplanationClaim>, ExplanationIrError> {
        Ok(vec![
            ExplanationClaim::unknown(
                HYDROLOGY_CONSERVATION_RESIDUAL_SCHEMA,
                NumericClaimValue::scalar(0),
                ComparisonContext::None,
            )?,
            ExplanationClaim::unknown(
                HYDROLOGY_BOUNDARY_EXPORT_SCHEMA,
                exact_volume(0)?,
                ComparisonContext::None,
            )?,
        ])
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
    /// Unlike a nonzero conservation residual, which answers with insufficiency,
    /// this is a malformed input rather than absent evidence: the three volumes
    /// come from one receipt and a caller that can present them not closing has
    /// built the claim from parts of different transfers.
    #[error(
        "hydrology transfer volumes do not close: requested {requested}, accepted {accepted}, unaccepted {unaccepted}"
    )]
    HydrologyTransferDoesNotClose {
        requested: u64,
        accepted: u64,
        unaccepted: u64,
    },
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
            total_material_retained_before: 0,
            total_material_retained_after: 0,
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

    #[test]
    fn material_surface_thermal_exchange_claim_reports_a_retained_energy_range() {
        // Given: a surface whose retained energy rose during one committed exchange.
        let claim = MaterialSurfaceThermalExchangeClaim {
            before_retained: 20,
            after_retained: 24,
            transition_trace: TraceId::new(41),
            cell_trace: TraceId::new(40),
        };

        // When: the transition is converted into a non-authoritative explanation claim.
        let actual = claim.to_explanation_claim().unwrap();

        // Then: the range is ordered independent of exchange direction and evidence is preserved.
        assert_eq!(actual.schema, MATERIAL_SURFACE_THERMAL_EXCHANGE_SCHEMA);
        assert_eq!(
            actual.value,
            NumericClaimValue::Range { start: 20, end: 24 }
        );
        assert_eq!(actual.evidence_state, ClaimEvidenceState::Supported);
        assert_eq!(
            actual.evidence_traces,
            vec![TraceId::new(40), TraceId::new(41)]
        );
    }

    #[test]
    fn material_surface_thermal_exchange_claim_orders_a_cooling_range() {
        // Given: a surface whose retained energy fell during one committed exchange.
        let claim = MaterialSurfaceThermalExchangeClaim {
            before_retained: 24,
            after_retained: 20,
            transition_trace: TraceId::new(41),
            cell_trace: TraceId::new(40),
        };

        // Then: the range is still ordered start <= end regardless of the exchange direction.
        assert_eq!(
            claim.to_explanation_claim().unwrap().value,
            NumericClaimValue::Range { start: 20, end: 24 }
        );
    }

    #[test]
    fn material_surface_thermal_exchange_unknown_carries_current_retained_energy() {
        // Given: a surface with no exchange evidence in the bounded transition history.
        let claim = MaterialSurfaceThermalExchangeClaim::unknown(7).unwrap();

        // Then: the claim is `Unknown` but still reports the surface's current retained energy.
        assert_eq!(claim.schema, MATERIAL_SURFACE_THERMAL_EXCHANGE_SCHEMA);
        assert_eq!(claim.value, NumericClaimValue::scalar(7));
        assert_eq!(claim.evidence_state, ClaimEvidenceState::Unknown);
        assert_eq!(claim.confidence, ClaimConfidence::ZERO);
        assert!(claim.evidence_traces.is_empty());
    }

    fn bootstrap_claim(
        stage_count: u32,
        receipt_count: u32,
        completion_traces: Vec<TraceId>,
    ) -> HistoricalBootstrapRecordClaim {
        HistoricalBootstrapRecordClaim {
            stage_count,
            receipt_count,
            observation_start: SimulationTime::new(0),
            observation_end: SimulationTime::new(6),
            completion_traces,
            dependency_traces: vec![TraceId::new(40), TraceId::new(41)],
        }
    }

    #[test]
    fn a_complete_bootstrap_record_is_supported_and_anchored_to_its_completions() {
        // Given: six stages closed by six evidenced completion traces.
        let traces = (0..6).map(TraceId::new).collect::<Vec<_>>();
        let claim = bootstrap_claim(6, 6, traces.clone())
            .to_explanation_claim()
            .unwrap();

        // Then: the claim reports the receipt count and carries both the
        // completion traces and their declared dependency anchors.
        assert_eq!(claim.schema, HISTORICAL_BOOTSTRAP_RECORD_SCHEMA);
        assert_eq!(claim.value, NumericClaimValue::scalar(6));
        assert_eq!(claim.evidence_state, ClaimEvidenceState::Supported);
        assert_eq!(claim.confidence, ClaimConfidence::ONE);
        assert_eq!(claim.evidence_traces.len(), traces.len() + 2);
    }

    #[test]
    fn an_incomplete_bootstrap_record_reports_insufficient_evidence() {
        // Given: a plan with six stages but only five receipts, and a record
        // whose completion traces could not all be resolved.
        for claim in [
            bootstrap_claim(6, 5, (0..5).map(TraceId::new).collect()),
            bootstrap_claim(6, 6, (0..5).map(TraceId::new).collect()),
            bootstrap_claim(0, 0, Vec::new()),
        ] {
            // Then: both the completeness and window claims fall back to the
            // existing unknown state with zero confidence and no evidence.
            let record = claim.to_explanation_claim().unwrap();
            assert_eq!(record.evidence_state, ClaimEvidenceState::Unknown);
            assert_eq!(record.confidence, ClaimConfidence::ZERO);
            assert!(record.evidence_traces.is_empty());

            let window = claim.to_window_claim().unwrap();
            assert_eq!(window.schema, HISTORICAL_BOOTSTRAP_WINDOW_SCHEMA);
            assert_eq!(window.evidence_state, ClaimEvidenceState::Unknown);
            assert_eq!(window.confidence, ClaimConfidence::ZERO);
        }
    }

    #[test]
    fn the_bootstrap_window_is_the_canonical_stage_span() {
        // Given: a complete six-stage record spanning [0, 6].
        let claim = bootstrap_claim(6, 6, (0..6).map(TraceId::new).collect());

        // Then: the window claim reports exactly that span, and nothing in the
        // IR names a process — only opaque counts and trace anchors.
        let window = claim.to_window_claim().unwrap();
        assert_eq!(window.value, NumericClaimValue::Range { start: 0, end: 6 });
        assert_eq!(window.evidence_state, ClaimEvidenceState::Supported);
    }
}
