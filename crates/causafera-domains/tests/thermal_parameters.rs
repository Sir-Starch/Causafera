use causafera_domains::{ThermalError, ThermalParameters};

#[test]
fn invalid_parameters_reject() {
    // Given: parameter sets violating each authoritative thermal bound.
    let invalid = [
        ThermalParameters {
            transfer_fraction: 11,
            heat_capacity: 1,
            scale: 60,
        },
        ThermalParameters {
            transfer_fraction: 1,
            heat_capacity: 0,
            scale: 60,
        },
        ThermalParameters {
            transfer_fraction: 1,
            heat_capacity: 1,
            scale: 0,
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
