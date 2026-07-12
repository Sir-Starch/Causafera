use ontopolis_types::ChunkId;

/// World hierarchy node.
#[allow(dead_code)]
pub struct World {
    root: ChunkId,
}

impl World {
    pub fn new() -> Self {
        Self {
            root: ChunkId::new(0),
        }
    }
}

impl Default for World {
    fn default() -> Self {
        Self::new()
    }
}
