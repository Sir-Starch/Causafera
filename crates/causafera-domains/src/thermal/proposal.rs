use std::collections::BTreeMap;

use causafera_types::ThermalEnergy;

use super::{
    ThermalActiveRegion, ThermalBoundaryBehavior, ThermalBoundaryRecord, ThermalCellChange,
    ThermalCellKey, ThermalCellTransferReceipt, ThermalConservationReceipt, ThermalFieldSet,
    ThermalInjectionProposal, ThermalMaterialSite, ThermalParameters, ThermalReservoir,
    ThermalReservoirId,
};

#[derive(Clone, Copy, Debug)]
pub struct ThermalEvolutionRequest<'a> {
    pub tick: u64,
    pub parameters: ThermalParameters,
    pub active_region: &'a ThermalActiveRegion,
    pub boundary_behavior: ThermalBoundaryBehavior,
    pub reservoirs: &'a [ThermalReservoir],
    pub injections: &'a [ThermalInjectionProposal],
    pub materials: &'a BTreeMap<ThermalCellKey, ThermalMaterialSite>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ThermalEvolutionProposal {
    after_state: ThermalFieldSet,
    cell_changes: Vec<ThermalCellChange>,
    conservation_receipt: ThermalConservationReceipt,
    transfer_receipts: Vec<ThermalCellTransferReceipt>,
    reservoir_budgets_after: BTreeMap<ThermalReservoirId, ThermalEnergy>,
    boundary_records: Vec<ThermalBoundaryRecord>,
    material_retained_after: BTreeMap<ThermalCellKey, ThermalEnergy>,
}

impl ThermalEvolutionProposal {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        after_state: ThermalFieldSet,
        cell_changes: Vec<ThermalCellChange>,
        conservation_receipt: ThermalConservationReceipt,
        transfer_receipts: Vec<ThermalCellTransferReceipt>,
        reservoir_budgets_after: BTreeMap<ThermalReservoirId, ThermalEnergy>,
        boundary_records: Vec<ThermalBoundaryRecord>,
        material_retained_after: BTreeMap<ThermalCellKey, ThermalEnergy>,
    ) -> Self {
        Self {
            after_state,
            cell_changes,
            conservation_receipt,
            transfer_receipts,
            reservoir_budgets_after,
            boundary_records,
            material_retained_after,
        }
    }

    pub fn after_state(&self) -> &ThermalFieldSet {
        &self.after_state
    }

    pub fn cell_changes(&self) -> &[ThermalCellChange] {
        &self.cell_changes
    }

    pub const fn conservation_receipt(&self) -> &ThermalConservationReceipt {
        &self.conservation_receipt
    }

    pub fn transfer_receipts(&self) -> &[ThermalCellTransferReceipt] {
        &self.transfer_receipts
    }

    pub fn reservoir_budgets_after(&self) -> &BTreeMap<ThermalReservoirId, ThermalEnergy> {
        &self.reservoir_budgets_after
    }

    pub fn boundary_records(&self) -> &[ThermalBoundaryRecord] {
        &self.boundary_records
    }

    pub fn material_retained_after(&self) -> &BTreeMap<ThermalCellKey, ThermalEnergy> {
        &self.material_retained_after
    }
}
