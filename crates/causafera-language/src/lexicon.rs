use causafera_types::{ConceptId, LanguageId, LexemeId, LexemeUseId, PerceptId, SimulationTime};

use crate::phonology::PhonologicalForm;

pub const WEIGHT_SCALE: u16 = 1_000;
pub const MAX_SEMANTIC_ASSOCIATIONS: usize = 8;
pub const MAX_LEXEME_USES: usize = 16;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct LanguageWeight(u16);

impl LanguageWeight {
    pub const ZERO: Self = Self(0);
    pub const FULL: Self = Self(WEIGHT_SCALE);

    pub const fn new(raw: u16) -> Self {
        Self(if raw > WEIGHT_SCALE {
            WEIGHT_SCALE
        } else {
            raw
        })
    }

    pub const fn raw(self) -> u16 {
        self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LexemeLineage {
    pub id: LexemeId,
    pub language_id: LanguageId,
    pub parent: Option<LexemeId>,
    pub form: PhonologicalForm,
    pub created_at: SimulationTime,
    uses: Vec<LexemeUse>,
}

impl LexemeLineage {
    pub fn new(
        id: LexemeId,
        language_id: LanguageId,
        parent: Option<LexemeId>,
        form: PhonologicalForm,
        created_at: SimulationTime,
    ) -> Self {
        Self {
            id,
            language_id,
            parent,
            form,
            created_at,
            uses: Vec::new(),
        }
    }

    pub fn record_use(&mut self, use_record: LexemeUse) {
        if self.uses.len() == MAX_LEXEME_USES {
            self.uses.remove(0);
        }
        self.uses.push(use_record);
        self.uses.sort_unstable_by_key(|item| (item.at, item.id));
    }

    pub fn uses(&self) -> &[LexemeUse] {
        &self.uses
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LexemeUse {
    pub id: LexemeUseId,
    pub at: SimulationTime,
    pub observed_form: PerceptId,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SemanticAssociation {
    pub concept_id: ConceptId,
    pub weight: LanguageWeight,
    pub supporting_percept: PerceptId,
    pub updated_at: SimulationTime,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AgentLexiconEntry {
    pub lexeme_id: LexemeId,
    associations: Vec<SemanticAssociation>,
    pub familiarity: LanguageWeight,
    pub production_probability: LanguageWeight,
}

impl AgentLexiconEntry {
    pub fn new(lexeme_id: LexemeId) -> Self {
        Self {
            lexeme_id,
            associations: Vec::new(),
            familiarity: LanguageWeight::ZERO,
            production_probability: LanguageWeight::ZERO,
        }
    }

    pub fn associations(&self) -> &[SemanticAssociation] {
        &self.associations
    }

    pub fn observe(
        &mut self,
        concept_id: ConceptId,
        evidence: LanguageWeight,
        supporting_percept: PerceptId,
        at: SimulationTime,
    ) {
        if let Some(association) = self
            .associations
            .iter_mut()
            .find(|item| item.concept_id == concept_id)
        {
            let revised = (u32::from(association.weight.raw()) * 3 + u32::from(evidence.raw())) / 4;
            association.weight = LanguageWeight::new(revised as u16);
            association.supporting_percept = supporting_percept;
            association.updated_at = at;
        } else {
            if self.associations.len() == MAX_SEMANTIC_ASSOCIATIONS {
                let remove = self
                    .associations
                    .iter()
                    .enumerate()
                    .min_by_key(|(_, item)| (item.weight, item.concept_id))
                    .map(|(index, _)| index)
                    .unwrap_or(0);
                self.associations.remove(remove);
            }
            self.associations.push(SemanticAssociation {
                concept_id,
                weight: evidence,
                supporting_percept,
                updated_at: at,
            });
        }
        self.associations
            .sort_unstable_by_key(|item| item.concept_id);
        self.familiarity = LanguageWeight::new(self.familiarity.raw().saturating_add(80));
        self.production_probability = LanguageWeight::new(self.familiarity.raw() / 2);
    }
}
