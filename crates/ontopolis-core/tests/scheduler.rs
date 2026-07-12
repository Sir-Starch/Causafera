use ontopolis_core::{DeterministicConfig, Phase, RandomStream, Scheduler, System};
use std::sync::{Arc, Mutex};

struct Probe(Arc<Mutex<Vec<u64>>>);

impl System for Probe {
    fn run(&mut self, stream: &mut RandomStream) {
        self.0.lock().unwrap().push(stream.next_u64());
    }
}

#[test]
fn scheduler_phase_order_is_fixed() {
    let probe = Arc::new(Mutex::new(Vec::new()));
    let mut scheduler = Scheduler::new(DeterministicConfig::new(0));

    for &phase in &Phase::ALL {
        scheduler.register_system(phase, Box::new(Probe(probe.clone())));
    }

    scheduler.tick();

    let values = probe.lock().unwrap();
    assert_eq!(values.len(), Phase::COUNT);
    assert_eq!(
        Phase::ALL,
        [
            Phase::Physics,
            Phase::Mana,
            Phase::Resolution,
            Phase::Perception,
            Phase::Cognition,
            Phase::Action,
            Phase::Lifecycle,
        ]
    );
}

#[test]
fn deterministic_scheduler_replay() {
    let config = DeterministicConfig::new(42);

    let probe_a = Arc::new(Mutex::new(Vec::new()));
    let probe_b = Arc::new(Mutex::new(Vec::new()));

    let mut scheduler_a = Scheduler::new(config.clone());
    scheduler_a.register_system(Phase::Physics, Box::new(Probe(probe_a.clone())));
    scheduler_a.tick();

    let mut scheduler_b = Scheduler::new(config);
    scheduler_b.register_system(Phase::Physics, Box::new(Probe(probe_b.clone())));
    scheduler_b.tick();

    assert_eq!(*probe_a.lock().unwrap(), *probe_b.lock().unwrap());
}

#[test]
fn different_seeds_diverge() {
    let probe_a = Arc::new(Mutex::new(Vec::new()));
    let probe_b = Arc::new(Mutex::new(Vec::new()));

    let mut scheduler_a = Scheduler::new(DeterministicConfig::new(1));
    scheduler_a.register_system(Phase::Cognition, Box::new(Probe(probe_a.clone())));
    scheduler_a.tick();

    let mut scheduler_b = Scheduler::new(DeterministicConfig::new(2));
    scheduler_b.register_system(Phase::Cognition, Box::new(Probe(probe_b.clone())));
    scheduler_b.tick();

    assert_ne!(*probe_a.lock().unwrap(), *probe_b.lock().unwrap());
}

#[test]
fn multiple_systems_same_phase_get_independent_streams() {
    let probe_a = Arc::new(Mutex::new(Vec::new()));
    let probe_b = Arc::new(Mutex::new(Vec::new()));

    let mut scheduler = Scheduler::new(DeterministicConfig::new(99));
    scheduler.register_system(Phase::Physics, Box::new(Probe(probe_a.clone())));
    scheduler.register_system(Phase::Physics, Box::new(Probe(probe_b.clone())));
    scheduler.tick();

    let values_a = probe_a.lock().unwrap();
    let values_b = probe_b.lock().unwrap();

    assert_eq!(values_a.len(), 1);
    assert_eq!(values_b.len(), 1);
    assert_ne!(values_a[0], values_b[0]);
}

#[test]
fn tick_advances_time() {
    let mut scheduler = Scheduler::new(DeterministicConfig::new(0));
    assert_eq!(scheduler.current_time().raw(), 0);
    scheduler.tick();
    assert_eq!(scheduler.current_time().raw(), 1);
    scheduler.tick();
    assert_eq!(scheduler.current_time().raw(), 2);
}

#[test]
fn strict_mode_flag_is_stored() {
    let config = DeterministicConfig::new(7);
    let scheduler = Scheduler::new(config);
    assert!(scheduler.config().strict_mode);
    assert_eq!(scheduler.config().world_seed, 7);
}
