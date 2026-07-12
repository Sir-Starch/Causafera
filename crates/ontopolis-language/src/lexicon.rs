use ontopolis_types::{ConceptId, LexemeId};
use std::collections::HashMap;

/// Agent-specific lexicon entry.
pub struct AgentLexiconEntry {
    pub lexeme_id: LexemeId,
    pub semantic_associations: HashMap<ConceptId, f32>,
    pub familiarity: f32,
    pub production_probability: f32,
}
