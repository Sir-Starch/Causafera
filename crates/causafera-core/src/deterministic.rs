use serde::{Deserialize, Serialize};

/// Deterministic random stream configuration.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeterministicConfig {
    pub world_seed: u64,
    pub strict_mode: bool,
}

impl DeterministicConfig {
    pub fn new(world_seed: u64) -> Self {
        Self {
            world_seed,
            strict_mode: true,
        }
    }
}
