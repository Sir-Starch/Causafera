use ontopolis_core::{DeterministicConfig, Scheduler};

/// Simulation runtime.
#[allow(dead_code)]
pub struct Runtime {
    scheduler: Scheduler,
}

impl Runtime {
    pub fn new(config: DeterministicConfig) -> Self {
        Self {
            scheduler: Scheduler::new(config),
        }
    }
}

impl Default for Runtime {
    fn default() -> Self {
        Self::new(DeterministicConfig::new(0))
    }
}
