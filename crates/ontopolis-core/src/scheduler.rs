use ontopolis_types::SimulationTime;

/// Simulation scheduler.
pub struct Scheduler {
    current_time: SimulationTime,
}

impl Scheduler {
    pub fn new() -> Self {
        Self {
            current_time: SimulationTime::new(0),
        }
    }

    pub fn current_time(&self) -> SimulationTime {
        self.current_time
    }

    pub fn tick(&mut self) {
        self.current_time = self.current_time.tick();
    }
}

impl Default for Scheduler {
    fn default() -> Self {
        Self::new()
    }
}
