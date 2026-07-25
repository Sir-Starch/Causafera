use causafera_types::ThermalEnergy;

use super::ThermalError;

pub(crate) fn check_energy_bounds(value: i128) -> Result<(), ThermalError> {
    if !(0..=i128::from(ThermalEnergy::MAX.get())).contains(&value) {
        return Err(ThermalError::EnergyOutOfBounds);
    }
    Ok(())
}

pub(crate) fn energy_from_i128(value: i128) -> Result<ThermalEnergy, ThermalError> {
    check_energy_bounds(value)?;
    let raw = i64::try_from(value).map_err(|_| ThermalError::ArithmeticOverflow)?;
    ThermalEnergy::new(raw).map_err(|_| ThermalError::EnergyOutOfBounds)
}

pub(crate) fn sum_i128(mut values: impl Iterator<Item = i128>) -> Result<i128, ThermalError> {
    values.try_fold(0_i128, |total, value| {
        total
            .checked_add(value)
            .ok_or(ThermalError::ArithmeticOverflow)
    })
}
