use ontopolis_types::{LanguageId, LexemeId, PhonologicalUnitId, SimulationTime};

use crate::lexicon::LexemeLineage;
use crate::phonology::{PhonemeInventory, mix64};

pub const MAX_BOOTSTRAP_LANGUAGES: usize = 8;
pub const MAX_BOOTSTRAP_LEXEMES: usize = 32;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LanguageLineage {
    pub id: LanguageId,
    pub parent: Option<LanguageId>,
    pub inventory: PhonemeInventory,
    pub formed_at: SimulationTime,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LanguageBootstrap {
    pub languages: Vec<LanguageLineage>,
    pub lexemes: Vec<LexemeLineage>,
}

impl LanguageBootstrap {
    pub fn generate(seed: u64, language_count: usize, lexemes_per_language: usize) -> Self {
        let language_count = language_count.clamp(1, MAX_BOOTSTRAP_LANGUAGES);
        let lexemes_per_language = lexemes_per_language.min(MAX_BOOTSTRAP_LEXEMES);
        let mut languages = Vec::with_capacity(language_count);
        let mut lexemes = Vec::with_capacity(language_count * lexemes_per_language);
        let mut next_lexeme = 1_u64;
        for ordinal in 0..language_count {
            let language_id = LanguageId::new((ordinal + 1) as u64);
            let parent = (ordinal > 0).then(|| LanguageId::new(((ordinal - 1) / 2 + 1) as u64));
            let base = mix64(seed ^ language_id.raw());
            let units = (0..12)
                .map(|offset| PhonologicalUnitId::new((base & 0xffff) + offset + 1))
                .collect();
            let inventory = PhonemeInventory::new(language_id, units, 5, 4, 3)
                .expect("generated inventory is valid");
            for form_ordinal in 0..lexemes_per_language {
                let id = LexemeId::new(next_lexeme);
                next_lexeme += 1;
                let key = mix64(base ^ form_ordinal as u64);
                let form = inventory.generate(key);
                let parent_lexeme = parent.map(|parent_language| {
                    (form_ordinal < lexemes_per_language)
                        .then(|| {
                            let parent_offset =
                                (parent_language.raw() - 1) * lexemes_per_language as u64;
                            LexemeId::new(parent_offset + form_ordinal as u64 + 1)
                        })
                        .expect("bootstrap uses the same lexeme count per language")
                });
                lexemes.push(LexemeLineage::new(
                    id,
                    language_id,
                    parent_lexeme,
                    form,
                    SimulationTime::new(ordinal as u64),
                ));
            }
            languages.push(LanguageLineage {
                id: language_id,
                parent,
                inventory,
                formed_at: SimulationTime::new(ordinal as u64),
            });
        }
        Self { languages, lexemes }
    }
}
