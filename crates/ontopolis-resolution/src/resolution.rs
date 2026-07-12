use ontopolis_types::ChunkId;

/// Causal resolution configuration for a spatial region.
pub struct ResolutionConfig {
    pub chunk: ChunkId,
    pub resolution_level: u8,
}
