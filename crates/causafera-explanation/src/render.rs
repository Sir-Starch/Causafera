use std::fmt::Write;

use crate::{
    ClaimEvidenceState, ComparisonContext, ExplanationClaim, ExplanationReport, FrameAssessment,
    NumericClaimValue,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObserverLocale {
    En,
    Ru,
}

impl ObserverLocale {
    pub fn parse(locale: &str) -> Self {
        if locale.eq_ignore_ascii_case("ru") || locale.to_ascii_lowercase().starts_with("ru-") {
            Self::Ru
        } else {
            Self::En
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RenderedExplanation {
    pub locale: ObserverLocale,
    pub text: String,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct DeterministicExplanationRenderer;

impl DeterministicExplanationRenderer {
    pub fn render(
        &self,
        report: &ExplanationReport,
        locale: ObserverLocale,
    ) -> RenderedExplanation {
        let mut text = String::new();
        match locale {
            ObserverLocale::En => {
                writeln!(
                    text,
                    "Experiment {}. Assessment: {}.",
                    report.experiment.raw(),
                    assessment(report.overall_assessment, locale)
                )
                .unwrap();
            }
            ObserverLocale::Ru => {
                writeln!(
                    text,
                    "Эксперимент {}. Оценка: {}.",
                    report.experiment.raw(),
                    assessment(report.overall_assessment, locale)
                )
                .unwrap();
            }
        }
        for frame in &report.frames {
            match locale {
                ObserverLocale::En => {
                    writeln!(text, "Tick {}:", frame.checkpoint_time.raw()).unwrap()
                }
                ObserverLocale::Ru => {
                    writeln!(text, "Такт {}:", frame.checkpoint_time.raw()).unwrap()
                }
            }
            for claim in &frame.claims {
                render_claim(&mut text, claim, locale);
            }
        }
        if text.ends_with('\n') {
            text.pop();
        }
        RenderedExplanation { locale, text }
    }
}

fn render_claim(out: &mut String, claim: &ExplanationClaim, locale: ObserverLocale) {
    let name = schema_name(claim.schema.raw(), locale);
    let value = numeric_value(claim.value);
    let evidence = evidence_state(claim.evidence_state, locale);
    let comparison = comparison_context(claim.comparison, locale);
    let confidence = (claim.confidence.raw() * 100.0).round() as u32;
    match locale {
        ObserverLocale::En => writeln!(
            out,
            "- {name}: {value}; evidence {evidence}; confidence {confidence}%; {comparison}; traces {}.",
            claim.evidence_traces.len()
        ).unwrap(),
        ObserverLocale::Ru => writeln!(
            out,
            "- {name}: {value}; свидетельства: {evidence}; уверенность {confidence}%; {comparison}; трасс: {}.",
            claim.evidence_traces.len()
        ).unwrap(),
    }
}

fn schema_name(id: u64, locale: ObserverLocale) -> String {
    let known = match (id, locale) {
        (1, ObserverLocale::En) => "reconstructability ratio",
        (2, ObserverLocale::En) => "path-dependence ratio",
        (3, ObserverLocale::En) => "causal depth",
        (4, ObserverLocale::En) => "temporal span",
        (5, ObserverLocale::En) => "counterfactual state distance",
        (6, ObserverLocale::En) => "recovery distance",
        (7, ObserverLocale::En) => "time to recovery",
        (8, ObserverLocale::En) => "stability under active input",
        (9, ObserverLocale::En) => "stability without active input",
        (10, ObserverLocale::En) => "material-surface loop",
        (11, ObserverLocale::En) => "material-surface observation window",
        (12, ObserverLocale::En) => "material-surface repetition control",
        (13, ObserverLocale::En) => "material-surface mana context",
        (14, ObserverLocale::En) => "material-surface mana transition",
        (1, ObserverLocale::Ru) => "коэффициент реконструируемости",
        (2, ObserverLocale::Ru) => "коэффициент зависимости от истории",
        (3, ObserverLocale::Ru) => "каузальная глубина",
        (4, ObserverLocale::Ru) => "временной диапазон",
        (5, ObserverLocale::Ru) => "контрфактическое расстояние состояний",
        (6, ObserverLocale::Ru) => "расстояние восстановления",
        (7, ObserverLocale::Ru) => "время до восстановления",
        (8, ObserverLocale::Ru) => "стабильность при активном воздействии",
        (9, ObserverLocale::Ru) => "стабильность без активного воздействия",
        (10, ObserverLocale::Ru) => "цикл материальной поверхности",
        (11, ObserverLocale::Ru) => "окно наблюдения материальной поверхности",
        (12, ObserverLocale::Ru) => "контроль повторяемости материальной поверхности",
        (13, ObserverLocale::Ru) => "мана-контекст материальной поверхности",
        (14, ObserverLocale::Ru) => "переход маны материальной поверхности",
        _ => {
            return match locale {
                ObserverLocale::En => format!("claim schema {id} (generic renderer)"),
                ObserverLocale::Ru => format!("схема утверждения {id} (общий шаблон)"),
            };
        }
    };
    known.to_owned()
}

fn numeric_value(value: NumericClaimValue) -> String {
    match value {
        NumericClaimValue::Scalar { value } => value.to_string(),
        NumericClaimValue::Range { start, end } => format!("{start}..{end}"),
        NumericClaimValue::Ratio {
            numerator,
            denominator,
        } => format!("{numerator}/{denominator}"),
    }
}

fn evidence_state(state: ClaimEvidenceState, locale: ObserverLocale) -> &'static str {
    match (state, locale) {
        (ClaimEvidenceState::Supported, ObserverLocale::En) => "supported",
        (ClaimEvidenceState::Unsupported, ObserverLocale::En) => "unsupported",
        (ClaimEvidenceState::Unknown, ObserverLocale::En) => "unknown",
        (ClaimEvidenceState::Supported, ObserverLocale::Ru) => "подтверждено",
        (ClaimEvidenceState::Unsupported, ObserverLocale::Ru) => "не подтверждено",
        (ClaimEvidenceState::Unknown, ObserverLocale::Ru) => "неизвестно",
    }
}

fn assessment(value: FrameAssessment, locale: ObserverLocale) -> &'static str {
    match (value, locale) {
        (FrameAssessment::Supported, ObserverLocale::En) => "supported",
        (FrameAssessment::Partial, ObserverLocale::En) => "partial",
        (FrameAssessment::Unsupported, ObserverLocale::En) => "unsupported",
        (FrameAssessment::Unknown, ObserverLocale::En) => "unknown",
        (FrameAssessment::Supported, ObserverLocale::Ru) => "подтверждено",
        (FrameAssessment::Partial, ObserverLocale::Ru) => "частично подтверждено",
        (FrameAssessment::Unsupported, ObserverLocale::Ru) => "не подтверждено",
        (FrameAssessment::Unknown, ObserverLocale::Ru) => "неизвестно",
    }
}

fn comparison_context(value: ComparisonContext, locale: ObserverLocale) -> String {
    match (value, locale) {
        (ComparisonContext::None, ObserverLocale::En) => "no comparison".into(),
        (ComparisonContext::None, ObserverLocale::Ru) => "без сравнения".into(),
        (ComparisonContext::MatchedCohort { cohort }, ObserverLocale::En) => {
            format!("matched cohort {}", cohort.raw())
        }
        (ComparisonContext::MatchedCohort { cohort }, ObserverLocale::Ru) => {
            format!("сопоставленная когорта {}", cohort.raw())
        }
        (ComparisonContext::Counterfactual { cohort }, ObserverLocale::En) => {
            format!("counterfactual cohort {}", cohort.raw())
        }
        (ComparisonContext::Counterfactual { cohort }, ObserverLocale::Ru) => {
            format!("контрфактическая когорта {}", cohort.raw())
        }
    }
}

#[cfg(test)]
mod tests {
    use causafera_types::{ExperimentId, SimulationTime, TraceId};

    use crate::{
        ClaimConfidence, ExplanationClaimSchemaId, ExplanationFrame,
        MATERIAL_SURFACE_LOOP_CLAIM_SCHEMA, MATERIAL_SURFACE_LOOP_MANA_SCHEMA,
    };

    use super::*;

    fn report() -> ExplanationReport {
        let claim = ExplanationClaim::new(
            ExplanationClaimSchemaId::new(3),
            NumericClaimValue::scalar(12),
            ClaimConfidence::new(0.75).unwrap(),
            vec![TraceId::new(8)],
            ComparisonContext::None,
            ClaimEvidenceState::Supported,
        )
        .unwrap();
        ExplanationReport::new(
            ExperimentId::new(4),
            vec![ExplanationFrame::new(SimulationTime::new(7), vec![claim]).unwrap()],
        )
        .unwrap()
    }

    #[test]
    fn rendering_is_deterministic_and_localized() {
        let renderer = DeterministicExplanationRenderer;
        let report = report();
        let en = renderer.render(&report, ObserverLocale::En);
        let ru = renderer.render(&report, ObserverLocale::Ru);
        assert_eq!(en, renderer.render(&report, ObserverLocale::En));
        assert_ne!(en.text, ru.text);
        assert!(en.text.contains("confidence 75%"));
        assert!(ru.text.contains("уверенность 75%"));
    }

    #[test]
    fn generic_renderer_preserves_unknown_schema_identity() {
        let mut report = report();
        report.frames[0].claims[0].schema = ExplanationClaimSchemaId::new(999);
        let rendered = DeterministicExplanationRenderer.render(&report, ObserverLocale::En);
        assert!(
            rendered
                .text
                .contains("claim schema 999 (generic renderer)")
        );
    }

    #[test]
    fn material_surface_loop_schema_names_are_deterministic_and_localized() {
        // Given: observer-only material-loop schema identities.
        let report = ExplanationReport::new(
            ExperimentId::new(5),
            vec![
                ExplanationFrame::new(
                    SimulationTime::new(8),
                    vec![
                        ExplanationClaim::unknown(
                            MATERIAL_SURFACE_LOOP_CLAIM_SCHEMA,
                            NumericClaimValue::scalar(0),
                            ComparisonContext::None,
                        )
                        .unwrap(),
                        ExplanationClaim::new(
                            MATERIAL_SURFACE_LOOP_MANA_SCHEMA,
                            NumericClaimValue::scalar(12),
                            ClaimConfidence::ONE,
                            vec![TraceId::new(22)],
                            ComparisonContext::None,
                            ClaimEvidenceState::Supported,
                        )
                        .unwrap(),
                    ],
                )
                .unwrap(),
            ],
        )
        .unwrap();

        // When: the same non-authoritative report is rendered in supported locales.
        let english = DeterministicExplanationRenderer.render(&report, ObserverLocale::En);
        let russian = DeterministicExplanationRenderer.render(&report, ObserverLocale::Ru);

        // Then: localized labels change without changing the structured report.
        assert!(english.text.contains("material-surface loop"));
        assert!(russian.text.contains("цикл материальной поверхности"));
        assert_ne!(english.text, russian.text);
    }
}
