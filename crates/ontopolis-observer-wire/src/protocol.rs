use ontopolis_observer_api::ObserverQuery;

/// Wire protocol handler.
pub struct ProtocolHandler;

impl Default for ProtocolHandler {
    fn default() -> Self {
        Self
    }
}

impl ProtocolHandler {
    pub fn handle_query(&self, _query: ObserverQuery) -> Vec<u8> {
        Vec::new()
    }
}
