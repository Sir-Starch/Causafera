use serde::{Deserialize, Serialize};

/// Simulation snapshot for persistence.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Snapshot {
    pub version: u32,
    pub world_seed: u64,
}
