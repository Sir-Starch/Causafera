use ontopolis_types::SimulationTime;

use crate::deterministic::DeterministicConfig;
use crate::phases::Phase;
use crate::random::{RandomStream, StreamKey};

/// A simulation system that runs during a specific phase.
///
/// Systems receive a deterministic random stream for the current tick
/// so that every execution is reproducible given the same config and time.
///
/// Systems are `Send` so that the scheduler may eventually execute them
/// on different threads while preserving deterministic ordering in strict mode.
pub trait System: Send {
    fn run(&mut self, stream: &mut RandomStream);
}

/// Type-erased system container for registration.
pub type SystemBox = Box<dyn System>;

/// A registered system with its stable ID.
struct RegisteredSystem {
    id: u64,
    system: SystemBox,
}

/// Per-phase system registry.
#[derive(Default)]
struct PhaseRegistry {
    systems: Vec<RegisteredSystem>,
}

/// Simulation scheduler.
///
/// The scheduler ticks forward in discrete time steps. Each tick,
/// all phases execute in strict order. Within each phase, registered
/// systems run sequentially and each receives an independent random
/// stream keyed by `(world_seed, time, phase, system_id)`.
///
/// # Determinism
///
/// In strict mode, execution is fully deterministic: the same config,
/// the same set of registered systems, and the same starting time always
/// produce the exact same sequence of random stream consumption and
/// therefore the same simulation state.
///
/// # Phase Order
///
/// Phases execute in the order defined by [`Phase::ALL`]:
/// 1. Physics
/// 2. Mana
/// 3. Resolution
/// 4. Perception
/// 5. Cognition
/// 6. Action
///
/// # Example
///
/// ```
/// use ontopolis_core::{DeterministicConfig, Phase, Scheduler, System, RandomStream};
///
/// struct Counter(u64);
/// impl System for Counter {
///     fn run(&mut self, stream: &mut RandomStream) {
///         self.0 += stream.next_u64();
///     }
/// }
///
/// let config = DeterministicConfig::new(42);
/// let mut scheduler = Scheduler::new(config);
/// let mut counter = Counter(0);
/// scheduler.register_system(Phase::Physics, Box::new(counter));
/// scheduler.tick();
/// ```
pub struct Scheduler {
    config: DeterministicConfig,
    current_time: SimulationTime,
    phases: [PhaseRegistry; Phase::COUNT],
    next_system_id: u64,
}

impl Scheduler {
    /// Create a new scheduler with the given deterministic configuration.
    ///
    /// Time starts at [`SimulationTime::new(0)`].
    pub fn new(config: DeterministicConfig) -> Self {
        Self {
            config,
            current_time: SimulationTime::new(0),
            phases: std::array::from_fn(|_| PhaseRegistry::default()),
            next_system_id: 0,
        }
    }

    /// Current simulation time.
    pub fn current_time(&self) -> SimulationTime {
        self.current_time
    }

    /// Reference to the deterministic configuration.
    pub fn config(&self) -> &DeterministicConfig {
        &self.config
    }

    /// Register a system to run during a specific phase.
    ///
    /// Systems are run in registration order within each phase. Each system
    /// receives a unique ID that is used for deterministic stream keying.
    ///
    /// Returns the assigned system ID.
    pub fn register_system(&mut self, phase: Phase, system: SystemBox) -> u64 {
        let id = self.next_system_id;
        self.next_system_id += 1;
        let phase_idx = phase.id().0 as usize;
        self.phases[phase_idx]
            .systems
            .push(RegisteredSystem { id, system });
        id
    }

    /// Advance simulation by one tick, executing all phases.
    ///
    /// Phases run in the order defined by [`Phase::ALL`]. Time is advanced
    /// after all phases complete.
    pub fn tick(&mut self) {
        for &phase in &Phase::ALL {
            self.run_phase(phase);
        }
        self.current_time = self.current_time.tick();
    }

    /// Run a single phase without advancing time.
    ///
    /// This is primarily useful for testing and for the observer layer
    /// to inspect intermediate state. In normal operation, use [`tick`].
    pub fn run_phase(&mut self, phase: Phase) {
        let phase_idx = phase.id().0 as usize;
        for reg in self.phases[phase_idx].systems.iter_mut() {
            let key = StreamKey {
                world_seed: self.config.world_seed,
                time: self.current_time,
                phase,
                system_id: reg.id,
            };
            let mut stream = RandomStream::from_key(key);
            reg.system.run(&mut stream);
        }
    }

    /// Return the number of registered systems for a phase.
    pub fn system_count(&self, phase: Phase) -> usize {
        let phase_idx = phase.id().0 as usize;
        self.phases[phase_idx].systems.len()
    }
}

impl Default for Scheduler {
    fn default() -> Self {
        Self::new(DeterministicConfig::new(0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::random::RandomStream;
    use std::sync::{Arc, Mutex};

    /// Test probe that records every value it draws from the stream.
    struct Probe(Arc<Mutex<Vec<u64>>>);

    impl System for Probe {
        fn run(&mut self, stream: &mut RandomStream) {
            self.0.lock().unwrap().push(stream.next_u64());
        }
    }

    #[test]
    fn scheduler_starts_at_time_zero() {
        let scheduler = Scheduler::new(DeterministicConfig::new(0));
        assert_eq!(scheduler.current_time().raw(), 0);
    }

    #[test]
    fn tick_advances_time() {
        let mut scheduler = Scheduler::new(DeterministicConfig::new(0));
        scheduler.tick();
        assert_eq!(scheduler.current_time().raw(), 1);
    }

    #[test]
    fn systems_run_in_registration_order() {
        let mut scheduler = Scheduler::new(DeterministicConfig::new(0));
        let a = Probe(Arc::new(Mutex::new(Vec::new())));
        let b = Probe(Arc::new(Mutex::new(Vec::new())));

        let id_a = scheduler.register_system(Phase::Physics, Box::new(a));
        let id_b = scheduler.register_system(Phase::Physics, Box::new(b));

        assert_eq!(id_a, 0);
        assert_eq!(id_b, 1);
    }

    #[test]
    fn deterministic_replay_produces_same_values() {
        let config = DeterministicConfig::new(12345);

        let probe_a = Arc::new(Mutex::new(Vec::new()));
        let probe_b = Arc::new(Mutex::new(Vec::new()));

        let mut scheduler_a = Scheduler::new(config.clone());
        scheduler_a.register_system(Phase::Cognition, Box::new(Probe(probe_a.clone())));
        scheduler_a.tick();

        let mut scheduler_b = Scheduler::new(config);
        scheduler_b.register_system(Phase::Cognition, Box::new(Probe(probe_b.clone())));
        scheduler_b.tick();

        let values_a = probe_a.lock().unwrap().clone();
        let values_b = probe_b.lock().unwrap().clone();
        assert_eq!(values_a, values_b);
    }

    #[test]
    fn different_seeds_produce_different_values() {
        let probe_a = Arc::new(Mutex::new(Vec::new()));
        let probe_b = Arc::new(Mutex::new(Vec::new()));

        let mut scheduler_a = Scheduler::new(DeterministicConfig::new(1));
        scheduler_a.register_system(Phase::Cognition, Box::new(Probe(probe_a.clone())));
        scheduler_a.tick();

        let mut scheduler_b = Scheduler::new(DeterministicConfig::new(2));
        scheduler_b.register_system(Phase::Cognition, Box::new(Probe(probe_b.clone())));
        scheduler_b.tick();

        let values_a = probe_a.lock().unwrap().clone();
        let values_b = probe_b.lock().unwrap().clone();
        assert_ne!(values_a, values_b);
    }

    #[test]
    fn multiple_ticks_are_deterministic() {
        let config = DeterministicConfig::new(42);
        let probe_a = Arc::new(Mutex::new(Vec::new()));
        let probe_b = Arc::new(Mutex::new(Vec::new()));

        let mut scheduler_a = Scheduler::new(config.clone());
        scheduler_a.register_system(Phase::Physics, Box::new(Probe(probe_a.clone())));
        for _ in 0..5 {
            scheduler_a.tick();
        }

        let mut scheduler_b = Scheduler::new(config);
        scheduler_b.register_system(Phase::Physics, Box::new(Probe(probe_b.clone())));
        for _ in 0..5 {
            scheduler_b.tick();
        }

        let values_a = probe_a.lock().unwrap().clone();
        let values_b = probe_b.lock().unwrap().clone();
        assert_eq!(values_a, values_b);
        assert_eq!(values_a.len(), 5);
    }

    #[test]
    fn phases_run_in_correct_order() {
        let probe = Arc::new(Mutex::new(Vec::new()));
        let mut scheduler = Scheduler::new(DeterministicConfig::new(0));

        for &phase in &Phase::ALL {
            scheduler.register_system(phase, Box::new(Probe(probe.clone())));
        }

        scheduler.tick();

        let values = probe.lock().unwrap();
        assert_eq!(values.len(), Phase::COUNT);
    }
}
