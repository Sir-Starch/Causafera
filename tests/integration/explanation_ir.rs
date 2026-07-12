use ontopolis_explanation::ir::PhenomenonExplanation;
use ontopolis_types::ConceptId;

#[test]
fn explanation_ir_can_be_constructed() {
    let _explanation = PhenomenonExplanation {
        subject: ConceptId::new(1),
        classification: "test".to_string(),
        display_label: "Test".to_string(),
        origin: "origin".to_string(),
        key_associations: vec![],
        historical_transitions: vec![],
        confidence: 0.5,
    };
}
