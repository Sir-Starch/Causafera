use ontopolis_types::ConceptId;

/// Explanation Intermediate Representation.
pub struct PhenomenonExplanation {
    pub subject: ConceptId,
    pub classification: String,
    pub display_label: String,
    pub origin: String,
    pub key_associations: Vec<String>,
    pub historical_transitions: Vec<String>,
    pub confidence: f64,
}
