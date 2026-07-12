use ontopolis_types::ConceptId;

pub struct Belief {
    pub subject: ConceptId,
    pub confidence: f32,
}
