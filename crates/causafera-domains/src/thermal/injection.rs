use std::collections::BTreeMap;

use causafera_types::ThermalEnergy;

use super::{
    ThermalCellKey, ThermalError, ThermalInjectionProposal, ThermalReservoir, ThermalReservoirId,
    ThermalReservoirTransferRecord,
    arithmetic::{check_energy_bounds, energy_from_i128},
};

type CellEnergy = BTreeMap<ThermalCellKey, i128>;
type ReservoirBudgets = BTreeMap<ThermalReservoirId, ThermalEnergy>;
type ReservoirRecords = BTreeMap<ThermalCellKey, Vec<ThermalReservoirTransferRecord>>;

pub(crate) fn index_reservoirs(
    reservoirs: &[ThermalReservoir],
) -> Result<BTreeMap<ThermalReservoirId, ThermalReservoir>, ThermalError> {
    let mut indexed = BTreeMap::new();
    for reservoir in reservoirs {
        if indexed.insert(reservoir.id, *reservoir).is_some() {
            return Err(ThermalError::DuplicateReservoir);
        }
    }
    Ok(indexed)
}

pub(crate) fn accept_injections(
    committed: &CellEnergy,
    reservoirs: &BTreeMap<ThermalReservoirId, ThermalReservoir>,
    injections: &[ThermalInjectionProposal],
) -> Result<(CellEnergy, ReservoirBudgets, ReservoirRecords), ThermalError> {
    let mut ordered = injections.to_vec();
    ordered.sort_unstable_by_key(|proposal| proposal.reservoir_id);
    if ordered
        .windows(2)
        .any(|pair| pair[0].reservoir_id == pair[1].reservoir_id)
    {
        return Err(ThermalError::DuplicateInjectionProposal);
    }
    let mut pre_state = committed.clone();
    let mut budgets = reservoirs
        .iter()
        .map(|(id, reservoir)| (*id, reservoir.budget))
        .collect::<BTreeMap<_, _>>();
    let mut records = BTreeMap::<ThermalCellKey, Vec<ThermalReservoirTransferRecord>>::new();
    for injection in ordered {
        let reservoir = reservoirs
            .get(&injection.reservoir_id)
            .ok_or(ThermalError::UnknownReservoir)?;
        if reservoir.target != injection.target {
            return Err(ThermalError::InjectionTargetMismatch);
        }
        let current = pre_state
            .get(&injection.target)
            .copied()
            .ok_or(ThermalError::PositionOutsideField)?;
        let budget = budgets
            .get(&injection.reservoir_id)
            .copied()
            .ok_or(ThermalError::UnknownReservoir)?;
        let headroom = i128::from(ThermalEnergy::MAX.get())
            .checked_sub(current)
            .ok_or(ThermalError::EnergyOutOfBounds)?;
        let accepted = injection
            .scheduled_amount
            .get()
            .min(budget.get())
            .min(i64::try_from(headroom).map_err(|_| ThermalError::ArithmeticOverflow)?);
        let rejected = injection
            .scheduled_amount
            .get()
            .checked_sub(accepted)
            .ok_or(ThermalError::ArithmeticOverflow)?;
        let next = current
            .checked_add(i128::from(accepted))
            .ok_or(ThermalError::ArithmeticOverflow)?;
        check_energy_bounds(next)?;
        let remaining = budget
            .get()
            .checked_sub(accepted)
            .ok_or(ThermalError::ArithmeticOverflow)?;
        pre_state.insert(injection.target, next);
        budgets.insert(
            injection.reservoir_id,
            energy_from_i128(i128::from(remaining))?,
        );
        records
            .entry(injection.target)
            .or_default()
            .push(ThermalReservoirTransferRecord {
                id: injection.reservoir_id,
                scheduled_injection: injection.scheduled_amount,
                accepted_injection: energy_from_i128(i128::from(accepted))?,
                rejected_injection: energy_from_i128(i128::from(rejected))?,
                transfer_trace_id: None,
            });
    }
    Ok((pre_state, budgets, records))
}
