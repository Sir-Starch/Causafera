use ontopolis_types::ChunkId;

/// Observer query.
pub struct ObserverQuery {
    pub scope: ChunkId,
    pub query_type: QueryType,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QueryType {
    Entity,
    Spatial,
    Causal,
    Language,
}
