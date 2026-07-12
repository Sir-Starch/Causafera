use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SimulationTime(u64);

impl SimulationTime {
    pub const fn new(ticks: u64) -> Self {
        Self(ticks)
    }

    pub const fn raw(self) -> u64 {
        self.0
    }

    pub const fn tick(self) -> Self {
        Self(self.0 + 1)
    }
}

impl std::fmt::Debug for SimulationTime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SimulationTime({})", self.0)
    }
}

impl std::fmt::Display for SimulationTime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
