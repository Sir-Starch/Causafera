use ontopolis_types::ConceptId;
use std::collections::HashMap;

pub struct MemoryStore {
    pub associations: HashMap<ConceptId, f32>,
}

impl MemoryStore {
    pub fn new() -> Self {
        Self {
            associations: HashMap::new(),
        }
    }
}

impl Default for MemoryStore {
    fn default() -> Self {
        Self::new()
    }
}
