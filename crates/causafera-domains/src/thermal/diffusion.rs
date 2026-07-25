use std::collections::BTreeMap;

use causafera_types::ThermalEnergy;

use super::{
    ThermalActiveRegion, ThermalBoundaryRecord, ThermalCellKey, ThermalError, ThermalFaceRecord,
    ThermalFieldSet, ThermalParameters,
    arithmetic::{check_energy_bounds, energy_from_i128},
    neighbor::neighbor_keys,
};

type AfterEnergy = BTreeMap<ThermalCellKey, ThermalEnergy>;
type FaceRecords = BTreeMap<ThermalCellKey, Vec<ThermalFaceRecord>>;

pub(crate) fn preflight_faces(
    fields: &ThermalFieldSet,
    active_region: &ThermalActiveRegion,
    pre_state: &BTreeMap<ThermalCellKey, i128>,
    parameters: ThermalParameters,
) -> Result<(AfterEnergy, FaceRecords, Vec<ThermalBoundaryRecord>), ThermalError> {
    let mut deltas = pre_state
        .keys()
        .map(|key| (*key, 0_i128))
        .collect::<BTreeMap<_, _>>();
    let mut faces = BTreeMap::<ThermalCellKey, Vec<ThermalFaceRecord>>::new();
    let mut boundary_records = Vec::new();
    for key in pre_state.keys().copied() {
        let source = *pre_state
            .get(&key)
            .ok_or(ThermalError::PositionOutsideField)?;
        let (neighbors, boundary_neighbors) = neighbor_keys(fields, active_region, key)?;
        let cell_pre_state = energy_from_i128(source)?;
        boundary_records.extend(boundary_neighbors.into_iter().map(|neighbor| {
            ThermalBoundaryRecord {
                cell: key,
                neighbor,
                cell_pre_state,
            }
        }));
        for neighbor in neighbors {
            if key >= neighbor {
                continue;
            }
            let destination = *pre_state
                .get(&neighbor)
                .ok_or(ThermalError::PositionOutsideField)?;
            let flux = signed_flux(source, destination, parameters)?;
            if flux == 0 {
                continue;
            }
            update_delta(pre_state, &mut deltas, key, -flux)?;
            update_delta(pre_state, &mut deltas, neighbor, flux)?;
            let flux_i64 = i64::try_from(flux).map_err(|_| ThermalError::ArithmeticOverflow)?;
            faces.entry(key).or_default().push(ThermalFaceRecord {
                neighbor,
                signed_flux: flux_i64,
                neighbor_pre_state: energy_from_i128(destination)?,
            });
            faces.entry(neighbor).or_default().push(ThermalFaceRecord {
                neighbor: key,
                signed_flux: flux_i64
                    .checked_neg()
                    .ok_or(ThermalError::ArithmeticOverflow)?,
                neighbor_pre_state: energy_from_i128(source)?,
            });
        }
    }
    boundary_records.sort_unstable_by_key(|record| (record.cell, record.neighbor));
    let mut after = BTreeMap::new();
    for (key, before) in pre_state {
        let delta = deltas
            .get(key)
            .copied()
            .ok_or(ThermalError::PositionOutsideField)?;
        after.insert(
            *key,
            energy_from_i128(
                before
                    .checked_add(delta)
                    .ok_or(ThermalError::ArithmeticOverflow)?,
            )?,
        );
    }
    Ok((after, faces, boundary_records))
}

fn signed_flux(
    source: i128,
    destination: i128,
    parameters: ThermalParameters,
) -> Result<i128, ThermalError> {
    let difference = source
        .checked_sub(destination)
        .ok_or(ThermalError::ArithmeticOverflow)?;
    let magnitude = if difference >= 0 {
        difference
    } else {
        -difference
    };
    let scaled = magnitude
        .checked_mul(i128::from(parameters.transfer_fraction))
        .ok_or(ThermalError::ArithmeticOverflow)?
        / i128::from(parameters.scale);
    if difference >= 0 {
        Ok(scaled)
    } else {
        scaled.checked_neg().ok_or(ThermalError::ArithmeticOverflow)
    }
}

fn update_delta(
    pre_state: &BTreeMap<ThermalCellKey, i128>,
    deltas: &mut BTreeMap<ThermalCellKey, i128>,
    key: ThermalCellKey,
    change: i128,
) -> Result<(), ThermalError> {
    let delta = deltas
        .get(&key)
        .copied()
        .ok_or(ThermalError::PositionOutsideField)?;
    let next_delta = delta
        .checked_add(change)
        .ok_or(ThermalError::ArithmeticOverflow)?;
    let base = pre_state
        .get(&key)
        .copied()
        .ok_or(ThermalError::PositionOutsideField)?;
    check_energy_bounds(
        base.checked_add(next_delta)
            .ok_or(ThermalError::ArithmeticOverflow)?,
    )?;
    deltas.insert(key, next_delta);
    Ok(())
}
