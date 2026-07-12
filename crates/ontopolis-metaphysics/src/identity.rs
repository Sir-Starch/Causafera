use ontopolis_types::AgentId;

/// Identity persistence model.
pub struct IdentityState {
    pub agent: AgentId,
    pub persistence_pattern: Vec<u8>,
}
