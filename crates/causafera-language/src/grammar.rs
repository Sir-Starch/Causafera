use causafera_types::{ConceptId, SpeechActId};

use crate::lexicon::LanguageWeight;

/// Speaker-private content. This record is never passed directly to decoding.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CommunicativeIntent {
    pub speech_act: SpeechActId,
    pub referenced_concept: ConceptId,
    pub confidence: LanguageWeight,
}
