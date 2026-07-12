pub mod bootstrap;
pub mod communication;
pub mod grammar;
pub mod innovation;
pub mod lexicon;
pub mod phonology;

pub use bootstrap::*;
pub use communication::*;
pub use grammar::*;
pub use innovation::*;
pub use lexicon::*;
pub use phonology::*;

#[cfg(test)]
mod tests {
    use ontopolis_types::{
        ConceptId, LanguageId, LexemeId, PerceptId, SimulationTime, SpeechActId, TransmissionId,
        UtteranceId,
    };

    use super::*;

    fn t(value: u64) -> SimulationTime {
        SimulationTime::new(value)
    }

    #[test]
    fn bootstrap_is_seed_deterministic_and_forms_are_phonotactic() {
        let left = LanguageBootstrap::generate(41, 3, 7);
        let right = LanguageBootstrap::generate(41, 3, 7);
        assert_eq!(left, right);
        assert_eq!(left.languages.len(), 3);
        assert!(left.lexemes.iter().all(|lexeme| {
            left.languages
                .iter()
                .find(|language| language.id == lexeme.language_id)
                .expect("lineage exists")
                .inventory
                .accepts(&lexeme.form)
        }));
        assert!(left.languages[1].parent.is_some());
        assert!(
            left.lexemes
                .iter()
                .skip(7)
                .all(|lexeme| lexeme.parent.is_some())
        );
    }

    #[test]
    fn lexeme_lineage_has_no_objective_meaning_and_associations_are_subjective() {
        let mut first = AgentLexiconEntry::new(LexemeId::new(1));
        let mut second = AgentLexiconEntry::new(LexemeId::new(1));
        first.observe(
            ConceptId::new(10),
            LanguageWeight::new(800),
            PerceptId::new(1),
            t(1),
        );
        second.observe(
            ConceptId::new(20),
            LanguageWeight::new(700),
            PerceptId::new(2),
            t(1),
        );
        assert_eq!(first.associations()[0].concept_id, ConceptId::new(10));
        assert_eq!(second.associations()[0].concept_id, ConceptId::new(20));
    }

    #[test]
    fn listener_never_receives_speaker_intent() {
        let bootstrap = LanguageBootstrap::generate(9, 1, 1);
        let lexeme = &bootstrap.lexemes[0];
        let speaker_concept = ConceptId::new(11);
        let listener_concept = ConceptId::new(22);
        let mut speaker = AgentLexiconEntry::new(lexeme.id);
        speaker.observe(
            speaker_concept,
            LanguageWeight::new(900),
            PerceptId::new(1),
            t(1),
        );
        let mut listener = AgentLexiconEntry::new(lexeme.id);
        listener.observe(
            listener_concept,
            LanguageWeight::new(850),
            PerceptId::new(2),
            t(1),
        );
        let intent = CommunicativeIntent {
            speech_act: SpeechActId::new(1),
            referenced_concept: speaker_concept,
            confidence: LanguageWeight::new(900),
        };
        let utterance = encode(
            intent,
            &speaker,
            lexeme.form.clone(),
            UtteranceId::new(1),
            t(2),
        )
        .unwrap();
        let interpretation = decode(&utterance, PerceptId::new(3), Some(&listener), &[]);
        assert_eq!(interpretation.candidates()[0].concept_id, listener_concept);
        assert_ne!(interpretation.candidates()[0].concept_id, speaker_concept);
    }

    #[test]
    fn repeated_unmet_need_enables_deterministic_coinage() {
        let bootstrap = LanguageBootstrap::generate(15, 1, 0);
        let language = &bootstrap.languages[0];
        let mut pressures = PressureStore::default();
        let concept = ConceptId::new(31);
        let first = pressures.register_need(concept, LanguageWeight::new(300), t(1));
        assert!(
            coin(
                15,
                LanguageId::new(1),
                LexemeId::new(8),
                first,
                &language.inventory,
                t(1)
            )
            .is_none()
        );
        let mut current = first;
        for time in 2..=4 {
            current = pressures.register_need(concept, LanguageWeight::new(300), t(time));
        }
        let left = coin(
            15,
            LanguageId::new(1),
            LexemeId::new(8),
            current,
            &language.inventory,
            t(4),
        )
        .unwrap();
        let right = coin(
            15,
            LanguageId::new(1),
            LexemeId::new(8),
            current,
            &language.inventory,
            t(4),
        )
        .unwrap();
        assert_eq!(left, right);
        assert!(language.inventory.accepts(&left.form));
    }

    #[test]
    fn adoption_records_exposure_and_revises_association() {
        let mut entry = AgentLexiconEntry::new(LexemeId::new(5));
        let mut history = AdoptionHistory::default();
        let record = TransmissionRecord {
            id: TransmissionId::new(1),
            lexeme_id: LexemeId::new(5),
            exposure: PerceptId::new(91),
            at: t(7),
        };
        history.adopt(
            record,
            &mut entry,
            ConceptId::new(4),
            LanguageWeight::new(650),
        );
        assert_eq!(history.records(), &[record]);
        assert_eq!(
            entry.associations()[0].supporting_percept,
            PerceptId::new(91)
        );
        assert!(entry.familiarity > LanguageWeight::ZERO);
    }
}
