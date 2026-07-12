/// Simulation phases.
///
/// Phases execute in the order declared here. The discriminant values
/// are fixed and stable; they are used for deterministic stream keying
/// and must not be reordered without a migration plan.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Phase {
    Perception = 0,
    Cognition = 1,
    Action = 2,
    Physics = 3,
    Mana = 4,
    Resolution = 5,
}

impl Phase {
    /// Number of defined phases.
    pub const COUNT: usize = 6;

    /// All phases in execution order.
    pub const ALL: [Phase; Self::COUNT] = [
        Phase::Perception,
        Phase::Cognition,
        Phase::Action,
        Phase::Physics,
        Phase::Mana,
        Phase::Resolution,
    ];

    /// Return the discriminant value as a small integer.
    pub const fn id(self) -> PhaseId {
        PhaseId(self as u8)
    }

    /// Return a human-readable label for debugging and observer UI.
    ///
    /// These labels are **non-authoritative** and belong to the observer
    /// layer only. The simulation engine never uses English strings for
    /// phase logic.
    pub const fn label(self) -> &'static str {
        match self {
            Phase::Perception => "perception",
            Phase::Cognition => "cognition",
            Phase::Action => "action",
            Phase::Physics => "physics",
            Phase::Mana => "mana",
            Phase::Resolution => "resolution",
        }
    }
}

/// Lightweight phase identifier.
///
/// `PhaseId` is a thin wrapper around a `u8` discriminant. It can be
/// stored compactly and converted back to a [`Phase`] when needed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PhaseId(pub u8);

impl PhaseId {
    /// Convert to a [`Phase`] if the value is valid.
    pub const fn to_phase(self) -> Option<Phase> {
        match self.0 {
            0 => Some(Phase::Perception),
            1 => Some(Phase::Cognition),
            2 => Some(Phase::Action),
            3 => Some(Phase::Physics),
            4 => Some(Phase::Mana),
            5 => Some(Phase::Resolution),
            _ => None,
        }
    }
}
