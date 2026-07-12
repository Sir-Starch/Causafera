use ontopolis_cognition::CognitiveWeight;
use ontopolis_types::{
    CapabilityResourceSchemaId, CrossWorldTransferId, ImportedPriorId, MaterialId, PracticeId,
    QuantitySchemaId, SubjectivePatternId, SubjectiveSourceId, TraceId,
};
use thiserror::Error;

pub const MAX_IMPORTED_PRIORS: usize = 64;
pub const MAX_REQUIREMENTS: usize = 64;

/// Subjective residue of transfer. It is neither objective truth nor capability evidence.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ImportedPrior {
    id: ImportedPriorId,
    pattern: SubjectivePatternId,
    initial_weight: CognitiveWeight,
    source: SubjectiveSourceId,
}
impl ImportedPrior {
    pub const fn new(
        id: ImportedPriorId,
        pattern: SubjectivePatternId,
        initial_weight: CognitiveWeight,
        source: SubjectiveSourceId,
    ) -> Self {
        Self {
            id,
            pattern,
            initial_weight,
            source,
        }
    }
    pub const fn id(self) -> ImportedPriorId {
        self.id
    }
    pub const fn pattern(self) -> SubjectivePatternId {
        self.pattern
    }
    pub const fn initial_weight(self) -> CognitiveWeight {
        self.initial_weight
    }
    pub const fn source(self) -> SubjectiveSourceId {
        self.source
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImportedPriorBundle {
    transfer: CrossWorldTransferId,
    priors: Vec<ImportedPrior>,
    transfer_trace: TraceId,
}
impl ImportedPriorBundle {
    pub fn new(
        transfer: CrossWorldTransferId,
        mut priors: Vec<ImportedPrior>,
        transfer_trace: TraceId,
    ) -> Result<Self, PriorError> {
        canonicalize(
            &mut priors,
            MAX_IMPORTED_PRIORS,
            PriorError::PriorCapacity,
            PriorError::DuplicatePrior,
        )?;
        if priors.is_empty() {
            return Err(PriorError::NoPriors);
        }
        Ok(Self {
            transfer,
            priors,
            transfer_trace,
        })
    }
    pub const fn transfer(&self) -> CrossWorldTransferId {
        self.transfer
    }
    pub fn priors(&self) -> &[ImportedPrior] {
        &self.priors
    }
    pub const fn transfer_trace(&self) -> TraceId {
        self.transfer_trace
    }
}

/// Independently satisfiable physical and procedural prerequisites.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ReproductionRequirements {
    practices: Vec<PracticeId>,
    materials: Vec<MaterialId>,
    resources: Vec<CapabilityResourceSchemaId>,
    measurements: Vec<QuantitySchemaId>,
}
impl ReproductionRequirements {
    pub fn new(
        mut practices: Vec<PracticeId>,
        mut materials: Vec<MaterialId>,
        mut resources: Vec<CapabilityResourceSchemaId>,
        mut measurements: Vec<QuantitySchemaId>,
    ) -> Result<Self, PriorError> {
        canonicalize(
            &mut practices,
            MAX_REQUIREMENTS,
            PriorError::RequirementCapacity,
            PriorError::DuplicateRequirement,
        )?;
        canonicalize(
            &mut materials,
            MAX_REQUIREMENTS,
            PriorError::RequirementCapacity,
            PriorError::DuplicateRequirement,
        )?;
        canonicalize(
            &mut resources,
            MAX_REQUIREMENTS,
            PriorError::RequirementCapacity,
            PriorError::DuplicateRequirement,
        )?;
        canonicalize(
            &mut measurements,
            MAX_REQUIREMENTS,
            PriorError::RequirementCapacity,
            PriorError::DuplicateRequirement,
        )?;
        Ok(Self {
            practices,
            materials,
            resources,
            measurements,
        })
    }
    pub fn assess<'a>(&'a self, available: &'a CapabilityEvidence) -> CapabilityGap<'a> {
        CapabilityGap {
            practices: missing(&self.practices, &available.practices),
            materials: missing(&self.materials, &available.materials),
            resources: missing(&self.resources, &available.resources),
            measurements: missing(&self.measurements, &available.measurements),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CapabilityEvidence {
    practices: Vec<PracticeId>,
    materials: Vec<MaterialId>,
    resources: Vec<CapabilityResourceSchemaId>,
    measurements: Vec<QuantitySchemaId>,
    causes: Vec<TraceId>,
}
impl CapabilityEvidence {
    pub fn new(
        mut practices: Vec<PracticeId>,
        mut materials: Vec<MaterialId>,
        mut resources: Vec<CapabilityResourceSchemaId>,
        mut measurements: Vec<QuantitySchemaId>,
        mut causes: Vec<TraceId>,
    ) -> Result<Self, PriorError> {
        canonicalize(
            &mut practices,
            MAX_REQUIREMENTS,
            PriorError::RequirementCapacity,
            PriorError::DuplicateRequirement,
        )?;
        canonicalize(
            &mut materials,
            MAX_REQUIREMENTS,
            PriorError::RequirementCapacity,
            PriorError::DuplicateRequirement,
        )?;
        canonicalize(
            &mut resources,
            MAX_REQUIREMENTS,
            PriorError::RequirementCapacity,
            PriorError::DuplicateRequirement,
        )?;
        canonicalize(
            &mut measurements,
            MAX_REQUIREMENTS,
            PriorError::RequirementCapacity,
            PriorError::DuplicateRequirement,
        )?;
        canonicalize(
            &mut causes,
            MAX_REQUIREMENTS,
            PriorError::RequirementCapacity,
            PriorError::DuplicateRequirement,
        )?;
        Ok(Self {
            practices,
            materials,
            resources,
            measurements,
            causes,
        })
    }
    pub fn causes(&self) -> &[TraceId] {
        &self.causes
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CapabilityGap<'a> {
    pub practices: Vec<&'a PracticeId>,
    pub materials: Vec<&'a MaterialId>,
    pub resources: Vec<&'a CapabilityResourceSchemaId>,
    pub measurements: Vec<&'a QuantitySchemaId>,
}
impl CapabilityGap<'_> {
    pub fn is_empty(&self) -> bool {
        self.practices.is_empty()
            && self.materials.is_empty()
            && self.resources.is_empty()
            && self.measurements.is_empty()
    }
}

#[derive(Clone, Copy, Debug, Error, PartialEq, Eq)]
pub enum PriorError {
    #[error("imported prior bundle is empty")]
    NoPriors,
    #[error("imported prior capacity exceeded")]
    PriorCapacity,
    #[error("duplicate imported prior")]
    DuplicatePrior,
    #[error("capability requirement capacity exceeded")]
    RequirementCapacity,
    #[error("duplicate capability requirement or evidence")]
    DuplicateRequirement,
}

fn canonicalize<T: Ord>(
    values: &mut [T],
    cap: usize,
    full: PriorError,
    duplicate: PriorError,
) -> Result<(), PriorError> {
    if values.len() > cap {
        return Err(full);
    }
    values.sort_unstable();
    if values.windows(2).any(|w| w[0] == w[1]) {
        return Err(duplicate);
    }
    Ok(())
}
fn missing<'a, T: Ord>(required: &'a [T], available: &[T]) -> Vec<&'a T> {
    required
        .iter()
        .filter(|item| available.binary_search(item).is_err())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use ontopolis_cognition::CognitiveWeight;
    #[test]
    fn imported_prior_does_not_satisfy_capability_requirements() {
        let _bundle = ImportedPriorBundle::new(
            CrossWorldTransferId::new(1),
            vec![ImportedPrior::new(
                ImportedPriorId::new(1),
                SubjectivePatternId::new(9),
                CognitiveWeight::new(500).unwrap(),
                SubjectiveSourceId::new(3),
            )],
            TraceId::new(4),
        )
        .unwrap();
        let requirements = ReproductionRequirements::new(
            vec![PracticeId::new(1)],
            vec![MaterialId::new(2)],
            vec![CapabilityResourceSchemaId::new(3)],
            vec![QuantitySchemaId::new(4)],
        )
        .unwrap();
        let evidence =
            CapabilityEvidence::new(vec![], vec![], vec![], vec![], vec![TraceId::new(4)]).unwrap();
        assert!(!requirements.assess(&evidence).is_empty());
    }
    #[test]
    fn only_independent_evidence_closes_the_gap() {
        let requirements = ReproductionRequirements::new(
            vec![PracticeId::new(1)],
            vec![MaterialId::new(2)],
            vec![CapabilityResourceSchemaId::new(3)],
            vec![QuantitySchemaId::new(4)],
        )
        .unwrap();
        let evidence = CapabilityEvidence::new(
            vec![PracticeId::new(1)],
            vec![MaterialId::new(2)],
            vec![CapabilityResourceSchemaId::new(3)],
            vec![QuantitySchemaId::new(4)],
            vec![TraceId::new(8)],
        )
        .unwrap();
        assert!(requirements.assess(&evidence).is_empty());
    }
}
