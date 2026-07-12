use ontopolis_types::{BodyId, PathogenId, PopulationLineageId, SimulationTime, TraceId};
use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;

/// Integer fraction in parts per million, inclusive of zero and one.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FractionPpm(u32);

impl FractionPpm {
    pub const ONE: u32 = 1_000_000;

    pub const fn new(parts_per_million: u32) -> Result<Self, PathogenValueError> {
        if parts_per_million > Self::ONE {
            return Err(PathogenValueError::FractionOutOfRange { parts_per_million });
        }
        Ok(Self(parts_per_million))
    }

    pub const fn parts_per_million(self) -> u32 {
        self.0
    }
}

/// Strictly positive duration in simulation ticks.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TickDuration(u64);

impl TickDuration {
    pub const fn new(ticks: u64) -> Result<Self, PathogenValueError> {
        if ticks == 0 {
            return Err(PathogenValueError::ZeroDuration);
        }
        Ok(Self(ticks))
    }

    pub const fn ticks(self) -> u64 {
        self.0
    }
}

/// Strictly positive quantity of transmissible pathogen material.
///
/// The unit scale is selected by the physical carrier model. It must remain
/// consistent for all properties and exposures involving one lineage.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PathogenQuantity(u64);

impl PathogenQuantity {
    pub const fn new(units: u64) -> Result<Self, PathogenValueError> {
        if units == 0 {
            return Err(PathogenValueError::ZeroQuantity);
        }
        Ok(Self(units))
    }

    pub const fn units(self) -> u64 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PathogenValueError {
    FractionOutOfRange { parts_per_million: u32 },
    ZeroDuration,
    ZeroQuantity,
}

impl fmt::Display for PathogenValueError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FractionOutOfRange { parts_per_million } => write!(
                formatter,
                "pathogen fraction {parts_per_million} exceeds one million parts per million"
            ),
            Self::ZeroDuration => write!(formatter, "pathogen duration must be positive"),
            Self::ZeroQuantity => write!(formatter, "pathogen material quantity must be positive"),
        }
    }
}

impl Error for PathogenValueError {}

/// Property-only inputs governing transmission and temporal progression.
///
/// These values neither classify the pathogen nor decide whether a specific
/// exposure establishes an infection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PathogenProperties {
    minimum_infectious_dose: PathogenQuantity,
    shedding_per_tick: PathogenQuantity,
    environmental_persistence: FractionPpm,
    mutation_propensity: FractionPpm,
    incubation: TickDuration,
    infectious: TickDuration,
}

impl PathogenProperties {
    pub const fn new(
        minimum_infectious_dose: PathogenQuantity,
        shedding_per_tick: PathogenQuantity,
        environmental_persistence: FractionPpm,
        mutation_propensity: FractionPpm,
        incubation: TickDuration,
        infectious: TickDuration,
    ) -> Self {
        Self {
            minimum_infectious_dose,
            shedding_per_tick,
            environmental_persistence,
            mutation_propensity,
            incubation,
            infectious,
        }
    }

    pub const fn minimum_infectious_dose(self) -> PathogenQuantity {
        self.minimum_infectious_dose
    }

    pub const fn shedding_per_tick(self) -> PathogenQuantity {
        self.shedding_per_tick
    }

    pub const fn environmental_persistence(self) -> FractionPpm {
        self.environmental_persistence
    }

    pub const fn mutation_propensity(self) -> FractionPpm {
        self.mutation_propensity
    }

    pub const fn incubation(self) -> TickDuration {
        self.incubation
    }

    pub const fn infectious(self) -> TickDuration {
        self.infectious
    }
}

/// Objective interaction properties for one host population lineage.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HostInteraction {
    host_lineage: PopulationLineageId,
    susceptibility: FractionPpm,
    replication_compatibility: FractionPpm,
    damage_response: FractionPpm,
}

impl HostInteraction {
    pub const fn new(
        host_lineage: PopulationLineageId,
        susceptibility: FractionPpm,
        replication_compatibility: FractionPpm,
        damage_response: FractionPpm,
    ) -> Self {
        Self {
            host_lineage,
            susceptibility,
            replication_compatibility,
            damage_response,
        }
    }

    pub const fn host_lineage(self) -> PopulationLineageId {
        self.host_lineage
    }

    pub const fn susceptibility(self) -> FractionPpm {
        self.susceptibility
    }

    pub const fn replication_compatibility(self) -> FractionPpm {
        self.replication_compatibility
    }

    pub const fn damage_response(self) -> FractionPpm {
        self.damage_response
    }
}

/// One immutable pathogen lineage definition.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PathogenLineage {
    id: PathogenId,
    parent: Option<PathogenId>,
    properties: PathogenProperties,
    host_interactions: Box<[HostInteraction]>,
}

impl PathogenLineage {
    pub fn new(
        id: PathogenId,
        parent: Option<PathogenId>,
        properties: PathogenProperties,
        host_interactions: Vec<HostInteraction>,
    ) -> Result<Self, PathogenLineageError> {
        for (index, pair) in host_interactions.windows(2).enumerate() {
            if pair[0].host_lineage() >= pair[1].host_lineage() {
                return Err(PathogenLineageError::HostInteractionsNotStrictlyOrdered {
                    index: index + 1,
                    previous: pair[0].host_lineage(),
                    actual: pair[1].host_lineage(),
                });
            }
        }

        Ok(Self {
            id,
            parent,
            properties,
            host_interactions: host_interactions.into_boxed_slice(),
        })
    }

    pub const fn id(&self) -> PathogenId {
        self.id
    }

    pub const fn parent(&self) -> Option<PathogenId> {
        self.parent
    }

    pub const fn properties(&self) -> PathogenProperties {
        self.properties
    }

    pub fn host_interactions(&self) -> &[HostInteraction] {
        &self.host_interactions
    }

    pub fn host_interaction(&self, host_lineage: PopulationLineageId) -> Option<HostInteraction> {
        self.host_interactions
            .iter()
            .copied()
            .find(|interaction| interaction.host_lineage() == host_lineage)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PathogenLineageError {
    HostInteractionsNotStrictlyOrdered {
        index: usize,
        previous: PopulationLineageId,
        actual: PopulationLineageId,
    },
    FieldLengthMismatch {
        ids: usize,
        parents: usize,
        properties: usize,
        host_interactions: usize,
    },
    DuplicatePathogenId {
        index: usize,
        id: PathogenId,
    },
    UnknownOrUnorderedParent {
        index: usize,
        parent: PathogenId,
    },
}

impl fmt::Display for PathogenLineageError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::HostInteractionsNotStrictlyOrdered {
                index,
                previous,
                actual,
            } => write!(
                formatter,
                "host interaction {index} lineage {actual} does not follow lineage {previous}"
            ),
            Self::FieldLengthMismatch {
                ids,
                parents,
                properties,
                host_interactions,
            } => write!(
                formatter,
                "pathogen lineage field lengths differ: ids={ids}, parents={parents}, properties={properties}, host_interactions={host_interactions}"
            ),
            Self::DuplicatePathogenId { index, id } => {
                write!(formatter, "pathogen lineage {index} duplicates id {id}")
            }
            Self::UnknownOrUnorderedParent { index, parent } => write!(
                formatter,
                "pathogen lineage {index} has missing or non-preceding parent {parent}"
            ),
        }
    }
}

impl Error for PathogenLineageError {}

/// Dense, canonically ordered structure-of-arrays pathogen-lineage registry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PathogenLineages {
    ids: Vec<PathogenId>,
    parents: Vec<Option<PathogenId>>,
    properties: Vec<PathogenProperties>,
    host_interactions: Vec<Box<[HostInteraction]>>,
}

impl PathogenLineages {
    pub fn from_lineages(lineages: Vec<PathogenLineage>) -> Result<Self, PathogenLineageError> {
        let mut ids = Vec::with_capacity(lineages.len());
        let mut parents = Vec::with_capacity(lineages.len());
        let mut properties = Vec::with_capacity(lineages.len());
        let mut host_interactions = Vec::with_capacity(lineages.len());

        for lineage in lineages {
            ids.push(lineage.id);
            parents.push(lineage.parent);
            properties.push(lineage.properties);
            host_interactions.push(lineage.host_interactions);
        }

        Self::from_fields(ids, parents, properties, host_interactions)
    }

    pub fn from_fields(
        ids: Vec<PathogenId>,
        parents: Vec<Option<PathogenId>>,
        properties: Vec<PathogenProperties>,
        host_interactions: Vec<Box<[HostInteraction]>>,
    ) -> Result<Self, PathogenLineageError> {
        let field_length = ids.len();
        if parents.len() != field_length
            || properties.len() != field_length
            || host_interactions.len() != field_length
        {
            return Err(PathogenLineageError::FieldLengthMismatch {
                ids: field_length,
                parents: parents.len(),
                properties: properties.len(),
                host_interactions: host_interactions.len(),
            });
        }

        let mut prior_ids = BTreeSet::new();
        for (index, id) in ids.iter().copied().enumerate() {
            if !prior_ids.insert(id) {
                return Err(PathogenLineageError::DuplicatePathogenId { index, id });
            }
            if let Some(parent) = parents[index] {
                if !prior_ids.contains(&parent) {
                    return Err(PathogenLineageError::UnknownOrUnorderedParent { index, parent });
                }
            }
            validate_host_interactions(&host_interactions[index])?;
        }

        Ok(Self {
            ids,
            parents,
            properties,
            host_interactions,
        })
    }

    pub fn len(&self) -> usize {
        self.ids.len()
    }

    pub fn is_empty(&self) -> bool {
        self.ids.is_empty()
    }

    pub fn ids(&self) -> &[PathogenId] {
        &self.ids
    }

    pub fn parents(&self) -> &[Option<PathogenId>] {
        &self.parents
    }

    pub fn properties(&self) -> &[PathogenProperties] {
        &self.properties
    }

    pub fn host_interactions(&self, index: usize) -> Option<&[HostInteraction]> {
        self.host_interactions.get(index).map(Box::as_ref)
    }

    pub fn index_of(&self, id: PathogenId) -> Option<usize> {
        self.ids.iter().position(|candidate| *candidate == id)
    }

    pub fn lineage(&self, index: usize) -> Option<PathogenLineage> {
        Some(PathogenLineage {
            id: *self.ids.get(index)?,
            parent: self.parents[index],
            properties: self.properties[index],
            host_interactions: self.host_interactions[index].clone(),
        })
    }
}

fn validate_host_interactions(
    host_interactions: &[HostInteraction],
) -> Result<(), PathogenLineageError> {
    for (index, pair) in host_interactions.windows(2).enumerate() {
        if pair[0].host_lineage() >= pair[1].host_lineage() {
            return Err(PathogenLineageError::HostInteractionsNotStrictlyOrdered {
                index: index + 1,
                previous: pair[0].host_lineage(),
                actual: pair[1].host_lineage(),
            });
        }
    }
    Ok(())
}

/// Causally referenced physical transmission opportunity.
///
/// An exposure is not proof of infection and does not mutate host state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PathogenExposure {
    pathogen: PathogenId,
    source: Option<BodyId>,
    target: BodyId,
    dose: PathogenQuantity,
    time: SimulationTime,
    trace: TraceId,
}

impl PathogenExposure {
    pub const fn new(
        pathogen: PathogenId,
        source: Option<BodyId>,
        target: BodyId,
        dose: PathogenQuantity,
        time: SimulationTime,
        trace: TraceId,
    ) -> Self {
        Self {
            pathogen,
            source,
            target,
            dose,
            time,
            trace,
        }
    }

    pub const fn pathogen(self) -> PathogenId {
        self.pathogen
    }

    pub const fn source(self) -> Option<BodyId> {
        self.source
    }

    pub const fn target(self) -> BodyId {
        self.target
    }

    pub const fn dose(self) -> PathogenQuantity {
        self.dose
    }

    pub const fn time(self) -> SimulationTime {
        self.time
    }

    pub const fn trace(self) -> TraceId {
        self.trace
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fraction(value: u32) -> FractionPpm {
        FractionPpm::new(value).unwrap()
    }

    fn duration(value: u64) -> TickDuration {
        TickDuration::new(value).unwrap()
    }

    fn quantity(value: u64) -> PathogenQuantity {
        PathogenQuantity::new(value).unwrap()
    }

    fn properties() -> PathogenProperties {
        PathogenProperties::new(
            quantity(25),
            quantity(4),
            fraction(300_000),
            fraction(250),
            duration(8),
            duration(20),
        )
    }

    fn interactions() -> Vec<HostInteraction> {
        vec![
            HostInteraction::new(
                PopulationLineageId::new(10),
                fraction(800_000),
                fraction(700_000),
                fraction(100_000),
            ),
            HostInteraction::new(
                PopulationLineageId::new(20),
                fraction(200_000),
                fraction(50_000),
                fraction(10_000),
            ),
        ]
    }

    #[test]
    fn numeric_primitives_enforce_physical_bounds() {
        assert_eq!(fraction(0).parts_per_million(), 0);
        assert_eq!(fraction(FractionPpm::ONE).parts_per_million(), 1_000_000);
        assert_eq!(duration(9).ticks(), 9);
        assert_eq!(quantity(7).units(), 7);
        assert_eq!(
            FractionPpm::new(1_000_001),
            Err(PathogenValueError::FractionOutOfRange {
                parts_per_million: 1_000_001
            })
        );
        assert_eq!(TickDuration::new(0), Err(PathogenValueError::ZeroDuration));
        assert_eq!(
            PathogenQuantity::new(0),
            Err(PathogenValueError::ZeroQuantity)
        );
    }

    #[test]
    fn lineage_preserves_properties_and_canonical_host_lookup() {
        let lineage = PathogenLineage::new(
            PathogenId::new(3),
            Some(PathogenId::new(1)),
            properties(),
            interactions(),
        )
        .unwrap();

        assert_eq!(lineage.id(), PathogenId::new(3));
        assert_eq!(lineage.parent(), Some(PathogenId::new(1)));
        assert_eq!(lineage.properties(), properties());
        assert_eq!(lineage.host_interactions().len(), 2);
        assert_eq!(
            lineage
                .host_interaction(PopulationLineageId::new(20))
                .unwrap()
                .replication_compatibility(),
            fraction(50_000)
        );
        assert_eq!(lineage.host_interaction(PopulationLineageId::new(30)), None);
    }

    #[test]
    fn host_profiles_must_be_strictly_ordered_and_unique() {
        let duplicate = vec![interactions()[0], interactions()[0]];
        assert!(matches!(
            PathogenLineage::new(PathogenId::new(1), None, properties(), duplicate),
            Err(PathogenLineageError::HostInteractionsNotStrictlyOrdered { index: 1, .. })
        ));

        let reversed = vec![interactions()[1], interactions()[0]];
        assert!(matches!(
            PathogenLineage::new(PathogenId::new(1), None, properties(), reversed),
            Err(PathogenLineageError::HostInteractionsNotStrictlyOrdered { index: 1, .. })
        ));
    }

    #[test]
    fn registry_requires_unique_ids_and_parent_before_child_order() {
        let root =
            PathogenLineage::new(PathogenId::new(1), None, properties(), interactions()).unwrap();
        let child = PathogenLineage::new(
            PathogenId::new(2),
            Some(PathogenId::new(1)),
            properties(),
            Vec::new(),
        )
        .unwrap();

        let registry = PathogenLineages::from_lineages(vec![root.clone(), child.clone()]).unwrap();
        assert_eq!(registry.ids(), &[PathogenId::new(1), PathogenId::new(2)]);
        assert_eq!(registry.index_of(PathogenId::new(2)), Some(1));
        assert_eq!(registry.lineage(1), Some(child));

        assert!(matches!(
            PathogenLineages::from_lineages(vec![root.clone(), root]),
            Err(PathogenLineageError::DuplicatePathogenId { index: 1, .. })
        ));
        assert!(matches!(
            PathogenLineages::from_lineages(vec![
                PathogenLineage::new(
                    PathogenId::new(2),
                    Some(PathogenId::new(1)),
                    properties(),
                    Vec::new(),
                )
                .unwrap(),
            ]),
            Err(PathogenLineageError::UnknownOrUnorderedParent { index: 0, .. })
        ));
    }

    #[test]
    fn registry_rejects_mismatched_fields_and_accepts_empty_state() {
        assert!(matches!(
            PathogenLineages::from_fields(
                vec![PathogenId::new(1)],
                Vec::new(),
                vec![properties()],
                vec![interactions().into_boxed_slice()],
            ),
            Err(PathogenLineageError::FieldLengthMismatch { .. })
        ));
        assert!(
            PathogenLineages::from_fields(Vec::new(), Vec::new(), Vec::new(), Vec::new())
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn exposure_is_an_immutable_traced_physical_record() {
        let exposure = PathogenExposure::new(
            PathogenId::new(4),
            Some(BodyId::new(5)),
            BodyId::new(6),
            quantity(11),
            SimulationTime::new(7),
            TraceId::new(8),
        );

        assert_eq!(exposure.pathogen(), PathogenId::new(4));
        assert_eq!(exposure.source(), Some(BodyId::new(5)));
        assert_eq!(exposure.target(), BodyId::new(6));
        assert_eq!(exposure.dose(), quantity(11));
        assert_eq!(exposure.time(), SimulationTime::new(7));
        assert_eq!(exposure.trace(), TraceId::new(8));
    }

    #[test]
    fn identical_inputs_produce_identical_lineage_state() {
        let first =
            PathogenLineage::new(PathogenId::new(1), None, properties(), interactions()).unwrap();
        let second =
            PathogenLineage::new(PathogenId::new(1), None, properties(), interactions()).unwrap();
        assert_eq!(first, second);
    }
}
