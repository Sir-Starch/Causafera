use std::fmt::Write;

use crate::{
    ClaimEvidenceState, ComparisonContext, ExplanationClaim, ExplanationReport, FrameAssessment,
    NumericClaimValue,
};

/// The locales the observer may be rendered in.
///
/// This is presentation identity only. Rendering is deterministic within a locale and no
/// authoritative state depends on which variant is chosen (INV-006, INV-007). English is the
/// fallback for any tag that is not recognised, so an unsupported locale reads as English
/// rather than as an empty or partially translated report.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObserverLocale {
    En,
    Ru,
    ZhHans,
    De,
    Es,
}

impl ObserverLocale {
    /// The order every translation table in this module is written in.
    pub const ORDER: [Self; 5] = [Self::En, Self::Ru, Self::ZhHans, Self::De, Self::Es];

    /// Position in `ORDER`, which is how the tables below are indexed.
    const fn index(self) -> usize {
        match self {
            Self::En => 0,
            Self::Ru => 1,
            Self::ZhHans => 2,
            Self::De => 3,
            Self::Es => 4,
        }
    }

    /// Resolve a locale tag by primary subtag, with the script deciding for Chinese.
    ///
    /// Traditional-script tags do not resolve to `ZhHans`: the project has no traditional
    /// dictionary, and answering a `zh-Hant` request with simplified text would misstate what
    /// the instrument actually covers.
    pub fn parse(locale: &str) -> Self {
        let lowered = locale.trim().to_ascii_lowercase().replace('_', "-");
        let mut parts = lowered.split('-');
        let Some(primary) = parts.next() else {
            return Self::En;
        };
        let subtags: Vec<&str> = parts.collect();
        match primary {
            "ru" => Self::Ru,
            "de" => Self::De,
            "es" => Self::Es,
            "zh" => {
                if subtags
                    .iter()
                    .any(|tag| matches!(*tag, "hant" | "tw" | "hk" | "mo"))
                {
                    Self::En
                } else {
                    Self::ZhHans
                }
            }
            _ => Self::En,
        }
    }
}

/// Pick a locale's entry out of a table written in `ObserverLocale::ORDER`.
const fn pick(table: &[&'static str; 5], locale: ObserverLocale) -> &'static str {
    table[locale.index()]
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
        writeln!(
            text,
            "{}",
            HEADING[locale.index()]
                .replace("{experiment}", &report.experiment.raw().to_string())
                .replace(
                    "{assessment}",
                    assessment(report.overall_assessment, locale)
                )
        )
        .unwrap();
        for frame in &report.frames {
            writeln!(
                text,
                "{}",
                TICK_HEADING[locale.index()]
                    .replace("{tick}", &frame.checkpoint_time.raw().to_string())
            )
            .unwrap();
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

/// The report's opening line. `{experiment}` and `{assessment}` are substituted in order.
const HEADING: [&str; 5] = [
    "Experiment {experiment}. Assessment: {assessment}.",
    "Эксперимент {experiment}. Оценка: {assessment}.",
    "实验 {experiment}。评估：{assessment}。",
    "Experiment {experiment}. Bewertung: {assessment}.",
    "Experimento {experiment}. Valoración: {assessment}.",
];

const TICK_HEADING: [&str; 5] = [
    "Tick {tick}:",
    "Такт {tick}:",
    "第 {tick} 刻：",
    "Takt {tick}:",
    "Tic {tick}:",
];

/// One claim line. Every placeholder appears exactly once in every locale.
const CLAIM_LINE: [&str; 5] = [
    "- {name}: {value}; evidence {evidence}; confidence {confidence}%; {comparison}; traces {traces}.",
    "- {name}: {value}; свидетельства: {evidence}; уверенность {confidence}%; {comparison}; трасс: {traces}.",
    "- {name}：{value}；证据：{evidence}；置信度 {confidence}%；{comparison}；迹线 {traces} 条。",
    "- {name}: {value}; Belege: {evidence}; Konfidenz {confidence} %; {comparison}; Spuren {traces}.",
    "- {name}: {value}; evidencia: {evidence}; confianza {confidence} %; {comparison}; trazas {traces}.",
];

fn render_claim(out: &mut String, claim: &ExplanationClaim, locale: ObserverLocale) {
    let name = schema_name(claim.schema.raw(), locale);
    let value = numeric_value(claim.value);
    let evidence = evidence_state(claim.evidence_state, locale);
    let comparison = comparison_context(claim.comparison, locale);
    let confidence = (claim.confidence.raw() * 100.0).round() as u32;
    let line = CLAIM_LINE[locale.index()]
        .replace("{name}", &name)
        .replace("{value}", &value)
        .replace("{evidence}", evidence)
        .replace("{confidence}", &confidence.to_string())
        .replace("{comparison}", &comparison)
        .replace("{traces}", &claim.evidence_traces.len().to_string());
    writeln!(out, "{line}").unwrap();
}

/// Registered claim schemas, in `ObserverLocale::ORDER`. An identifier absent from this table
/// is rendered generically rather than hidden, so a new schema appears the moment it is emitted.
const SCHEMA_NAMES: [(u64, [&str; 5]); 15] = [
    (
        1,
        [
            "reconstructability ratio",
            "коэффициент реконструируемости",
            "可重构性比率",
            "Rekonstruierbarkeitsquotient",
            "razón de reconstruibilidad",
        ],
    ),
    (
        2,
        [
            "path-dependence ratio",
            "коэффициент зависимости от истории",
            "路径依赖比率",
            "Pfadabhängigkeitsquotient",
            "razón de dependencia de trayectoria",
        ],
    ),
    (
        3,
        [
            "causal depth",
            "каузальная глубина",
            "因果深度",
            "Kausaltiefe",
            "profundidad causal",
        ],
    ),
    (
        4,
        [
            "temporal span",
            "временной диапазон",
            "时间跨度",
            "zeitliche Spanne",
            "extensión temporal",
        ],
    ),
    (
        5,
        [
            "counterfactual state distance",
            "контрфактическое расстояние состояний",
            "反事实状态距离",
            "kontrafaktischer Zustandsabstand",
            "distancia de estado contrafactual",
        ],
    ),
    (
        6,
        [
            "recovery distance",
            "расстояние восстановления",
            "恢复距离",
            "Erholungsabstand",
            "distancia de recuperación",
        ],
    ),
    (
        7,
        [
            "time to recovery",
            "время до восстановления",
            "恢复所需时间",
            "Zeit bis zur Erholung",
            "tiempo hasta la recuperación",
        ],
    ),
    (
        8,
        [
            "stability under active input",
            "стабильность при активном воздействии",
            "主动输入下的稳定性",
            "Stabilität unter aktiver Einwirkung",
            "estabilidad bajo entrada activa",
        ],
    ),
    (
        9,
        [
            "stability without active input",
            "стабильность без активного воздействия",
            "无主动输入时的稳定性",
            "Stabilität ohne aktive Einwirkung",
            "estabilidad sin entrada activa",
        ],
    ),
    (
        10,
        [
            "material-surface loop",
            "цикл материальной поверхности",
            "物质表面回路",
            "materieller Oberflächenkreis",
            "ciclo de superficie material",
        ],
    ),
    (
        11,
        [
            "material-surface observation window",
            "окно наблюдения материальной поверхности",
            "物质表面观测窗口",
            "Beobachtungsfenster der materiellen Oberfläche",
            "ventana de observación de superficie material",
        ],
    ),
    (
        12,
        [
            "material-surface repetition control",
            "контроль повторяемости материальной поверхности",
            "物质表面重复性对照",
            "Wiederholungskontrolle der materiellen Oberfläche",
            "control de repetición de superficie material",
        ],
    ),
    (
        13,
        [
            "material-surface mana context",
            "мана-контекст материальной поверхности",
            "物质表面魔力背景",
            "Mana-Kontext der materiellen Oberfläche",
            "contexto de maná de superficie material",
        ],
    ),
    (
        14,
        [
            "material-surface mana transition",
            "переход маны материальной поверхности",
            "物质表面魔力转变",
            "Mana-Übergang der materiellen Oberfläche",
            "transición de maná de superficie material",
        ],
    ),
    (
        15,
        [
            "material-surface local mana coupling",
            "локальная связь маны материальной поверхности",
            "物质表面局部魔力耦合",
            "Kopplung des lokalen Mana an der materiellen Oberfläche",
            "acoplamiento de maná local de superficie material",
        ],
    ),
];

/// The generic fallback. `{id}` keeps the schema identity visible in every locale.
const GENERIC_SCHEMA: [&str; 5] = [
    "claim schema {id} (generic renderer)",
    "схема утверждения {id} (общий шаблон)",
    "断言模式 {id}（通用渲染器）",
    "Aussageschema {id} (generischer Renderer)",
    "esquema de afirmación {id} (renderizador genérico)",
];

fn schema_name(id: u64, locale: ObserverLocale) -> String {
    for (schema, names) in &SCHEMA_NAMES {
        if *schema == id {
            return pick(names, locale).to_owned();
        }
    }
    GENERIC_SCHEMA[locale.index()].replace("{id}", &id.to_string())
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

const EVIDENCE_SUPPORTED: [&str; 5] = [
    "supported",
    "подтверждено",
    "有支持",
    "gestützt",
    "respaldada",
];

const EVIDENCE_UNSUPPORTED: [&str; 5] = [
    "unsupported",
    "не подтверждено",
    "无支持",
    "nicht gestützt",
    "sin respaldo",
];

const EVIDENCE_UNKNOWN: [&str; 5] = ["unknown", "неизвестно", "未知", "unbekannt", "desconocida"];

fn evidence_state(state: ClaimEvidenceState, locale: ObserverLocale) -> &'static str {
    let table = match state {
        ClaimEvidenceState::Supported => &EVIDENCE_SUPPORTED,
        ClaimEvidenceState::Unsupported => &EVIDENCE_UNSUPPORTED,
        ClaimEvidenceState::Unknown => &EVIDENCE_UNKNOWN,
    };
    pick(table, locale)
}

const ASSESSMENT_SUPPORTED: [&str; 5] = [
    "supported",
    "подтверждено",
    "有支持",
    "gestützt",
    "respaldada",
];

const ASSESSMENT_PARTIAL: [&str; 5] = [
    "partial",
    "частично подтверждено",
    "部分支持",
    "teilweise gestützt",
    "parcialmente respaldada",
];

const ASSESSMENT_UNSUPPORTED: [&str; 5] = [
    "unsupported",
    "не подтверждено",
    "无支持",
    "nicht gestützt",
    "sin respaldo",
];

const ASSESSMENT_UNKNOWN: [&str; 5] = ["unknown", "неизвестно", "未知", "unbekannt", "desconocida"];

fn assessment(value: FrameAssessment, locale: ObserverLocale) -> &'static str {
    let table = match value {
        FrameAssessment::Supported => &ASSESSMENT_SUPPORTED,
        FrameAssessment::Partial => &ASSESSMENT_PARTIAL,
        FrameAssessment::Unsupported => &ASSESSMENT_UNSUPPORTED,
        FrameAssessment::Unknown => &ASSESSMENT_UNKNOWN,
    };
    pick(table, locale)
}

const COMPARISON_NONE: [&str; 5] = [
    "no comparison",
    "без сравнения",
    "无对照",
    "kein Vergleich",
    "sin comparación",
];

/// `{cohort}` carries the cohort identity, which is data and never translated.
const COMPARISON_MATCHED: [&str; 5] = [
    "matched cohort {cohort}",
    "сопоставленная когорта {cohort}",
    "匹配队列 {cohort}",
    "gepaarte Kohorte {cohort}",
    "cohorte emparejada {cohort}",
];

const COMPARISON_COUNTERFACTUAL: [&str; 5] = [
    "counterfactual cohort {cohort}",
    "контрфактическая когорта {cohort}",
    "反事实队列 {cohort}",
    "kontrafaktische Kohorte {cohort}",
    "cohorte contrafactual {cohort}",
];

fn comparison_context(value: ComparisonContext, locale: ObserverLocale) -> String {
    match value {
        ComparisonContext::None => pick(&COMPARISON_NONE, locale).to_owned(),
        ComparisonContext::MatchedCohort { cohort } => {
            pick(&COMPARISON_MATCHED, locale).replace("{cohort}", &cohort.raw().to_string())
        }
        ComparisonContext::Counterfactual { cohort } => {
            pick(&COMPARISON_COUNTERFACTUAL, locale).replace("{cohort}", &cohort.raw().to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use causafera_types::{ExperimentId, SimulationTime, TraceId};

    use crate::{
        ClaimConfidence, ExplanationClaimSchemaId, ExplanationFrame,
        MATERIAL_SURFACE_LOOP_CLAIM_SCHEMA, MATERIAL_SURFACE_LOOP_LOCAL_MANA_TRANSITION_SCHEMA,
        MATERIAL_SURFACE_LOOP_MANA_SCHEMA,
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
    fn every_locale_renders_deterministically_and_distinctly() {
        // Given: one report and the full set of supported locales.
        let renderer = DeterministicExplanationRenderer;
        let report = report();

        // When: each locale renders it twice.
        let mut rendered = Vec::new();
        for locale in ObserverLocale::ORDER {
            let first = renderer.render(&report, locale);
            let second = renderer.render(&report, locale);

            // Then: rendering is a function of the report and the locale, nothing else.
            assert_eq!(first, second, "{locale:?} did not render deterministically");
            assert!(!first.text.is_empty(), "{locale:?} rendered nothing");
            assert!(
                !first.text.contains('{'),
                "{locale:?} left an unsubstituted placeholder: {}",
                first.text
            );
            rendered.push(first.text);
        }

        // Then: no two locales produce the same text, so none is silently untranslated.
        for (index, text) in rendered.iter().enumerate() {
            for (other_index, other) in rendered.iter().enumerate().skip(index + 1) {
                assert_ne!(
                    text,
                    other,
                    "locales {:?} and {:?} rendered identically",
                    ObserverLocale::ORDER[index],
                    ObserverLocale::ORDER[other_index]
                );
            }
        }
    }

    #[test]
    fn locale_tags_resolve_by_primary_subtag_and_script() {
        // Given: the tags the observer front end sends, plus bare and irregular forms.
        for (tag, expected) in [
            ("en-US", ObserverLocale::En),
            ("en", ObserverLocale::En),
            ("ru-RU", ObserverLocale::Ru),
            ("ru", ObserverLocale::Ru),
            ("zh-Hans", ObserverLocale::ZhHans),
            ("zh", ObserverLocale::ZhHans),
            ("zh-CN", ObserverLocale::ZhHans),
            ("zh_hans_cn", ObserverLocale::ZhHans),
            ("de-DE", ObserverLocale::De),
            ("de", ObserverLocale::De),
            ("es-ES", ObserverLocale::Es),
            ("es-MX", ObserverLocale::Es),
        ] {
            assert_eq!(
                ObserverLocale::parse(tag),
                expected,
                "tag {tag} misresolved"
            );
        }

        // Then: traditional script and unknown tags fall back rather than claiming coverage.
        for tag in ["zh-Hant", "zh-TW", "zh-HK", "fr-FR", "", "   ", "nonsense"] {
            assert_eq!(
                ObserverLocale::parse(tag),
                ObserverLocale::En,
                "tag {tag} should have fallen back to English"
            );
        }
    }

    #[test]
    fn an_unregistered_schema_keeps_its_identity_in_every_locale() {
        // Given: a claim carrying a schema the renderer does not know.
        let mut report = report();
        report.frames[0].claims[0].schema = ExplanationClaimSchemaId::new(4242);

        // When: every locale renders it.
        for locale in ObserverLocale::ORDER {
            let rendered = DeterministicExplanationRenderer.render(&report, locale);

            // Then: the numeric identity survives translation, so the schema stays traceable.
            assert!(
                rendered.text.contains("4242"),
                "{locale:?} lost the schema identity: {}",
                rendered.text
            );
        }
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
                        ExplanationClaim::new(
                            MATERIAL_SURFACE_LOOP_LOCAL_MANA_TRANSITION_SCHEMA,
                            NumericClaimValue::range(0, 3).unwrap(),
                            ClaimConfidence::ONE,
                            vec![TraceId::new(23)],
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
        assert!(
            english
                .text
                .contains("material-surface local mana coupling")
        );
        assert!(russian.text.contains("цикл материальной поверхности"));
        assert!(
            russian
                .text
                .contains("локальная связь маны материальной поверхности")
        );
        assert_ne!(english.text, russian.text);
    }
}
