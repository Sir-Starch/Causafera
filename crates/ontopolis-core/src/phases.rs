/// Simulation phases.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Perception,
    Cognition,
    Action,
    Physics,
    Mana,
    Resolution,
}
