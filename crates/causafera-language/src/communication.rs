use causafera_types::{ConceptId, LanguageId, PerceptId, SimulationTime, UtteranceId};

use crate::grammar::CommunicativeIntent;
use crate::lexicon::{AgentLexiconEntry, LanguageWeight};
use crate::phonology::PhonologicalForm;

pub const MAX_INTERPRETATION_CANDIDATES: usize = 8;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PhysicalUtterance {
    pub id: UtteranceId,
    pub form: PhonologicalForm,
    pub emitted_at: SimulationTime,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InterpretationCandidate {
    pub concept_id: ConceptId,
    pub confidence: LanguageWeight,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ListenerInterpretation {
    pub utterance_id: UtteranceId,
    pub heard_as: PerceptId,
    candidates: Vec<InterpretationCandidate>,
}

impl ListenerInterpretation {
    pub fn candidates(&self) -> &[InterpretationCandidate] {
        &self.candidates
    }
}

pub fn encode(
    intent: CommunicativeIntent,
    entry: &AgentLexiconEntry,
    form: PhonologicalForm,
    id: UtteranceId,
    at: SimulationTime,
) -> Option<PhysicalUtterance> {
    entry
        .associations()
        .iter()
        .any(|item| item.concept_id == intent.referenced_concept && item.weight.raw() > 0)
        .then_some(PhysicalUtterance {
            id,
            form,
            emitted_at: at,
        })
}

pub fn decode(
    utterance: &PhysicalUtterance,
    heard_as: PerceptId,
    entry: Option<&AgentLexiconEntry>,
    context: &[InterpretationCandidate],
) -> ListenerInterpretation {
    let mut candidates = Vec::new();
    if let Some(entry) = entry {
        candidates.extend(
            entry
                .associations()
                .iter()
                .map(|item| InterpretationCandidate {
                    concept_id: item.concept_id,
                    confidence: item.weight,
                }),
        );
    }
    for contextual in context {
        if let Some(existing) = candidates
            .iter_mut()
            .find(|item| item.concept_id == contextual.concept_id)
        {
            existing.confidence = LanguageWeight::new(
                existing
                    .confidence
                    .raw()
                    .saturating_add(contextual.confidence.raw() / 2),
            );
        } else {
            candidates.push(*contextual);
        }
    }
    candidates.sort_unstable_by(|a, b| {
        b.confidence
            .cmp(&a.confidence)
            .then_with(|| a.concept_id.cmp(&b.concept_id))
    });
    candidates.truncate(MAX_INTERPRETATION_CANDIDATES);
    ListenerInterpretation {
        utterance_id: utterance.id,
        heard_as,
        candidates,
    }
}

pub fn recognize<'a>(
    language_id: LanguageId,
    form: &PhonologicalForm,
    lexemes: &'a [crate::lexicon::LexemeLineage],
) -> Option<&'a crate::lexicon::LexemeLineage> {
    lexemes
        .iter()
        .filter(|item| item.language_id == language_id && &item.form == form)
        .min_by_key(|item| item.id)
}
