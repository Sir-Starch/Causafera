use causafera_types::{
    AgentId, EntityId, InventoryLotId, LabourContributionId, MaterialId, MaterialTransferId,
    MaterialTransformationId, PlaceId, PracticeId, PropertyClaimId, SimulationTime, TraceId,
};

pub const MAX_INVENTORY_LOTS: usize = 65_536;
pub const MAX_MATERIAL_TRANSFERS: usize = 65_536;
pub const MAX_TRANSFORMATIONS: usize = 32_768;
pub const MAX_LABOUR_CONTRIBUTIONS: usize = 65_536;
pub const MAX_TRANSFORMATION_LOTS: usize = 64;
pub const MAX_OWNERSHIP_CLAIMS: usize = 16;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InventoryLot {
    pub id: InventoryLotId,
    pub material: MaterialId,
    pub holder: EntityId,
    pub location: PlaceId,
    pub quantity: u64,
    pub ownership_claims: Vec<PropertyClaimId>,
    pub recorded_at: SimulationTime,
    pub trace: TraceId,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct MaterialTransfer {
    pub id: MaterialTransferId,
    pub source_lot: InventoryLotId,
    pub destination_lot: InventoryLotId,
    pub quantity: u64,
    pub occurred_at: SimulationTime,
    pub trace: TraceId,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MaterialTransformation {
    pub id: MaterialTransformationId,
    pub inputs: Vec<InventoryLotId>,
    pub outputs: Vec<InventoryLotId>,
    pub practice: Option<PracticeId>,
    pub occurred_at: SimulationTime,
    pub trace: TraceId,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct LabourContribution {
    pub id: LabourContributionId,
    pub transformation: MaterialTransformationId,
    pub actor: AgentId,
    pub duration_ticks: u64,
    pub trace: TraceId,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EconomicState {
    lots: Vec<InventoryLot>,
    transfers: Vec<MaterialTransfer>,
    transformations: Vec<MaterialTransformation>,
    labour: Vec<LabourContribution>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EconomyError {
    CapacityExceeded,
    DuplicateId,
    DuplicateReference,
    EmptyInputs,
    EmptyOutputs,
    SameTransferLot,
    UnknownLot,
    UnknownTransformation,
    ZeroQuantity,
    ZeroDuration,
    MaterialMismatch,
    TransferExceedsSource,
}

impl EconomicState {
    pub fn new(
        mut lots: Vec<InventoryLot>,
        mut transfers: Vec<MaterialTransfer>,
        mut transformations: Vec<MaterialTransformation>,
        mut labour: Vec<LabourContribution>,
    ) -> Result<Self, EconomyError> {
        if lots.len() > MAX_INVENTORY_LOTS
            || transfers.len() > MAX_MATERIAL_TRANSFERS
            || transformations.len() > MAX_TRANSFORMATIONS
            || labour.len() > MAX_LABOUR_CONTRIBUTIONS
        {
            return Err(EconomyError::CapacityExceeded);
        }
        lots.sort_unstable_by_key(|record| record.id);
        transfers.sort_unstable_by_key(|record| record.id);
        transformations.sort_unstable_by_key(|record| record.id);
        labour.sort_unstable_by_key(|record| record.id);
        reject_duplicate_ids(&lots, |record| record.id)?;
        reject_duplicate_ids(&transfers, |record| record.id)?;
        reject_duplicate_ids(&transformations, |record| record.id)?;
        reject_duplicate_ids(&labour, |record| record.id)?;

        for lot in &mut lots {
            if lot.quantity == 0 {
                return Err(EconomyError::ZeroQuantity);
            }
            sort_unique(&mut lot.ownership_claims, MAX_OWNERSHIP_CLAIMS)?;
        }
        for transfer in &transfers {
            if transfer.quantity == 0 {
                return Err(EconomyError::ZeroQuantity);
            }
            if transfer.source_lot == transfer.destination_lot {
                return Err(EconomyError::SameTransferLot);
            }
            let source = find_lot(&lots, transfer.source_lot)?;
            let destination = find_lot(&lots, transfer.destination_lot)?;
            if source.material != destination.material {
                return Err(EconomyError::MaterialMismatch);
            }
            if transfer.quantity > source.quantity {
                return Err(EconomyError::TransferExceedsSource);
            }
        }
        for transformation in &mut transformations {
            if transformation.inputs.is_empty() {
                return Err(EconomyError::EmptyInputs);
            }
            if transformation.outputs.is_empty() {
                return Err(EconomyError::EmptyOutputs);
            }
            sort_unique(&mut transformation.inputs, MAX_TRANSFORMATION_LOTS)?;
            sort_unique(&mut transformation.outputs, MAX_TRANSFORMATION_LOTS)?;
            for lot in transformation.inputs.iter().chain(&transformation.outputs) {
                find_lot(&lots, *lot)?;
            }
        }
        for contribution in &labour {
            if contribution.duration_ticks == 0 {
                return Err(EconomyError::ZeroDuration);
            }
            find_transformation(&transformations, contribution.transformation)?;
        }

        Ok(Self {
            lots,
            transfers,
            transformations,
            labour,
        })
    }

    pub fn lots(&self) -> &[InventoryLot] {
        &self.lots
    }
    pub fn transfers(&self) -> &[MaterialTransfer] {
        &self.transfers
    }
    pub fn transformations(&self) -> &[MaterialTransformation] {
        &self.transformations
    }
    pub fn labour(&self) -> &[LabourContribution] {
        &self.labour
    }
    pub fn lot(&self, id: InventoryLotId) -> Option<&InventoryLot> {
        self.lots
            .binary_search_by_key(&id, |record| record.id)
            .ok()
            .map(|index| &self.lots[index])
    }
}

fn reject_duplicate_ids<T, K: Eq + Copy>(
    values: &[T],
    key: impl Fn(&T) -> K,
) -> Result<(), EconomyError> {
    if values.windows(2).any(|pair| key(&pair[0]) == key(&pair[1])) {
        Err(EconomyError::DuplicateId)
    } else {
        Ok(())
    }
}

fn sort_unique<T: Ord>(values: &mut [T], maximum: usize) -> Result<(), EconomyError> {
    if values.len() > maximum {
        return Err(EconomyError::CapacityExceeded);
    }
    values.sort_unstable();
    if values.windows(2).any(|pair| pair[0] == pair[1]) {
        Err(EconomyError::DuplicateReference)
    } else {
        Ok(())
    }
}

fn find_lot(values: &[InventoryLot], id: InventoryLotId) -> Result<&InventoryLot, EconomyError> {
    values
        .binary_search_by_key(&id, |record| record.id)
        .map(|index| &values[index])
        .map_err(|_| EconomyError::UnknownLot)
}

fn find_transformation(
    values: &[MaterialTransformation],
    id: MaterialTransformationId,
) -> Result<&MaterialTransformation, EconomyError> {
    values
        .binary_search_by_key(&id, |record| record.id)
        .map(|index| &values[index])
        .map_err(|_| EconomyError::UnknownTransformation)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lot(id: u64, material: u64, quantity: u64) -> InventoryLot {
        InventoryLot {
            id: InventoryLotId::new(id),
            material: MaterialId::new(material),
            holder: EntityId::new(id),
            location: PlaceId::new(id),
            quantity,
            ownership_claims: vec![],
            recorded_at: SimulationTime::new(1),
            trace: TraceId::new(id),
        }
    }

    #[test]
    fn state_is_canonical_and_keeps_possession_separate_from_claims() {
        let state = EconomicState::new(
            vec![lot(2, 7, 4), lot(1, 7, 9)],
            vec![MaterialTransfer {
                id: MaterialTransferId::new(1),
                source_lot: InventoryLotId::new(1),
                destination_lot: InventoryLotId::new(2),
                quantity: 4,
                occurred_at: SimulationTime::new(2),
                trace: TraceId::new(3),
            }],
            vec![MaterialTransformation {
                id: MaterialTransformationId::new(1),
                inputs: vec![InventoryLotId::new(1)],
                outputs: vec![InventoryLotId::new(2)],
                practice: Some(PracticeId::new(8)),
                occurred_at: SimulationTime::new(2),
                trace: TraceId::new(4),
            }],
            vec![LabourContribution {
                id: LabourContributionId::new(1),
                transformation: MaterialTransformationId::new(1),
                actor: AgentId::new(4),
                duration_ticks: 3,
                trace: TraceId::new(5),
            }],
        )
        .unwrap();
        assert_eq!(state.lots()[0].id, InventoryLotId::new(1));
        assert!(state.lots()[0].ownership_claims.is_empty());
    }

    #[test]
    fn transfer_requires_same_material_and_available_quantity() {
        let transfer = MaterialTransfer {
            id: MaterialTransferId::new(1),
            source_lot: InventoryLotId::new(1),
            destination_lot: InventoryLotId::new(2),
            quantity: 10,
            occurred_at: SimulationTime::new(2),
            trace: TraceId::new(3),
        };
        assert_eq!(
            EconomicState::new(
                vec![lot(1, 7, 9), lot(2, 8, 4)],
                vec![transfer],
                vec![],
                vec![]
            ),
            Err(EconomyError::MaterialMismatch)
        );
    }

    #[test]
    fn transformation_references_are_bounded_unique_and_known() {
        let transformation = MaterialTransformation {
            id: MaterialTransformationId::new(1),
            inputs: vec![InventoryLotId::new(1), InventoryLotId::new(1)],
            outputs: vec![InventoryLotId::new(2)],
            practice: None,
            occurred_at: SimulationTime::new(2),
            trace: TraceId::new(4),
        };
        assert_eq!(
            EconomicState::new(
                vec![lot(1, 7, 9), lot(2, 8, 4)],
                vec![],
                vec![transformation],
                vec![]
            ),
            Err(EconomyError::DuplicateReference)
        );
    }
}
