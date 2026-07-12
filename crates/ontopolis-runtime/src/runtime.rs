use ontopolis_core::Scheduler;

/// Simulation runtime.
#[allow(dead_code)]
pub struct Runtime {
    scheduler: Scheduler,
}

impl Runtime {
    pub fn new() -> Self {
        Self {
            scheduler: Scheduler::new(),
        }
    }
}

impl Default for Runtime {
    fn default() -> Self {
        Self::new()
    }
}
