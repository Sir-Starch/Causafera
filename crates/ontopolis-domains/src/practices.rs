use ontopolis_types::PracticeId;

/// Structured behavioural program.
pub struct Practice {
    pub id: PracticeId,
    pub operations: Vec<PracticeOperation>,
}

pub struct PracticeOperation {
    pub step: u32,
    pub action: String,
}
