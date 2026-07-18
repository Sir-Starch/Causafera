use causafera_types::{ConceptId, LanguageId, LexemeId, PerceptId, SimulationTime, TransmissionId};

use crate::lexicon::{AgentLexiconEntry, LanguageWeight, LexemeLineage};
use crate::phonology::{PhonemeInventory, mix64};

pub const MAX_PRESSURES: usize = 16;
pub const MAX_TRANSMISSIONS: usize = 32;
pub const COINAGE_THRESHOLD: u16 = 600;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LexicalPressure {
    pub concept_id: ConceptId,
    pub strength: LanguageWeight,
    pub last_need: SimulationTime,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PressureStore {
    pressures: Vec<LexicalPressure>,
}

impl PressureStore {
    pub fn register_need(
        &mut self,
        concept_id: ConceptId,
        unmet: LanguageWeight,
        at: SimulationTime,
    ) -> LexicalPressure {
        if let Some(item) = self
            .pressures
            .iter_mut()
            .find(|item| item.concept_id == concept_id)
        {
            item.strength =
                LanguageWeight::new(item.strength.raw().saturating_add(unmet.raw() / 2));
            item.last_need = at;
        } else {
            if self.pressures.len() == MAX_PRESSURES {
                let index = self
                    .pressures
                    .iter()
                    .enumerate()
                    .min_by_key(|(_, item)| (item.strength, item.concept_id))
                    .map(|(index, _)| index)
                    .unwrap_or(0);
                self.pressures.remove(index);
            }
            self.pressures.push(LexicalPressure {
                concept_id,
                strength: unmet,
                last_need: at,
            });
        }
        self.pressures.sort_unstable_by_key(|item| item.concept_id);
        *self
            .pressures
            .iter()
            .find(|item| item.concept_id == concept_id)
            .expect("registered pressure exists")
    }

    pub fn pressures(&self) -> &[LexicalPressure] {
        &self.pressures
    }
}

pub fn coin(
    seed: u64,
    language_id: LanguageId,
    lexeme_id: LexemeId,
    pressure: LexicalPressure,
    inventory: &PhonemeInventory,
    at: SimulationTime,
) -> Option<LexemeLineage> {
    (pressure.strength.raw() >= COINAGE_THRESHOLD).then(|| {
        let key = mix64(
            seed ^ language_id.raw().rotate_left(7)
                ^ lexeme_id.raw().rotate_left(19)
                ^ pressure.concept_id.raw(),
        );
        LexemeLineage::new(lexeme_id, language_id, None, inventory.generate(key), at)
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TransmissionRecord {
    pub id: TransmissionId,
    pub lexeme_id: LexemeId,
    pub exposure: PerceptId,
    pub at: SimulationTime,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AdoptionHistory {
    records: Vec<TransmissionRecord>,
}

impl AdoptionHistory {
    pub fn adopt(
        &mut self,
        record: TransmissionRecord,
        entry: &mut AgentLexiconEntry,
        concept: ConceptId,
        evidence: LanguageWeight,
    ) {
        entry.observe(concept, evidence, record.exposure, record.at);
        if self.records.len() == MAX_TRANSMISSIONS {
            self.records.remove(0);
        }
        self.records.push(record);
        self.records.sort_unstable_by_key(|item| (item.at, item.id));
    }

    pub fn records(&self) -> &[TransmissionRecord] {
        &self.records
    }
}
