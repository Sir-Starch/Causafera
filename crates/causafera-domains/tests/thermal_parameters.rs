use causafera_domains::{ThermalError, ThermalParameters};

#[test]
fn invalid_parameters_reject() {
    // Given: parameter sets violating each authoritative thermal bound.
    let invalid = [
        ThermalParameters {
            transfer_fraction: 11,
            heat_capacity: 1,
            scale: 60,
            material_exchange_fraction: 0,
            material_thermal_capacity: 1,
        },
        ThermalParameters {
            transfer_fraction: 1,
            heat_capacity: 0,
            scale: 60,
            material_exchange_fraction: 0,
            material_thermal_capacity: 1,
        },
        ThermalParameters {
            transfer_fraction: 1,
            heat_capacity: 1,
            scale: 0,
            material_exchange_fraction: 0,
            material_thermal_capacity: 1,
        },
        // Negative material exchange fraction.
        ThermalParameters {
            transfer_fraction: 1,
            heat_capacity: 1,
            scale: 60,
            material_exchange_fraction: -1,
            material_thermal_capacity: 1,
        },
        // Non-positive material thermal capacity.
        ThermalParameters {
            transfer_fraction: 1,
            heat_capacity: 1,
            scale: 60,
            material_exchange_fraction: 0,
            material_thermal_capacity: 0,
        },
        // The widened bound subsumes the original six-face-only bound: with no material
        // coupling requested (`material_exchange_fraction == 0`), an over-large
        // `transfer_fraction` alone must still reject exactly as it did before this tranche.
        ThermalParameters {
            transfer_fraction: 11,
            heat_capacity: 1,
            scale: 60,
            material_exchange_fraction: 0,
            material_thermal_capacity: 1,
        },
        // A non-zero material fraction that exhausts the headroom left after six faces.
        ThermalParameters {
            transfer_fraction: 10,
            heat_capacity: 1,
            scale: 60,
            material_exchange_fraction: 1,
            material_thermal_capacity: 1,
        },
    ];

    // When: each set is validated at the domain boundary.
    let results = invalid.map(|parameters| parameters.validate());

    // Then: no invalid coefficient, capacity, or scale is accepted.
    assert!(
        results
            .iter()
            .all(|result| matches!(result, Err(ThermalError::InvalidParameters)))
    );
}

#[test]
fn zero_material_exchange_fraction_disables_material_coupling_without_special_casing() {
    // Given: the widened bound at its exact original boundary (six faces fill the whole scale).
    let parameters = ThermalParameters::new(10, 1, 60, 0, 1).unwrap();

    // Then: construction succeeds — a zero material fraction leaves no headroom requirement.
    assert_eq!(parameters.material_exchange_fraction, 0);
}

#[test]
fn material_exchange_fraction_shares_headroom_with_six_faces() {
    // Given: headroom for exactly one unit of material exchange after six faces at fraction 9.
    let parameters = ThermalParameters::new(9, 1, 60, 6, 1).unwrap();

    assert_eq!(parameters.material_exchange_fraction, 6);
}
