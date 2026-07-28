use std::collections::{BTreeMap, BTreeSet};

use causafera_domains::{
    THERMAL_SCALE, ThermalActiveRegion, ThermalBoundaryBehavior, ThermalEnergy,
    ThermalEvolutionRequest, ThermalField, ThermalFieldSet, ThermalParameters,
};
use causafera_types::{ChartChunkCoord, ChunkCoord, SpatialChartId, TraceId};

#[test]
fn face_reconstruct() {
    // Given: a center cell with asymmetric energies on each incident face.
    let chunk = ChartChunkCoord::new(SpatialChartId::new(1), ChunkCoord::new(0, 0, 0));
    let mut energy = vec![ThermalEnergy::ZERO; 27];
    for (index, raw) in [
        (13, 800),
        (12, 0),
        (14, 80),
        (10, 160),
        (16, 240),
        (4, 320),
        (22, 400),
    ] {
        energy[index] = ThermalEnergy::new(raw).expect("fixture energy must be valid");
    }
    let field =
        ThermalField::from_energy(chunk, 3, energy, TraceId::new(1)).expect("field must be valid");
    let fields =
        ThermalFieldSet::new(vec![field], TraceId::new(1)).expect("field set must be valid");
    let active = BTreeSet::from([chunk]);
    let region = ThermalActiveRegion::new(active.clone(), active).expect("region must be valid");

    // When: one conservative evolution proposal is produced.
    let proposal = fields
        .propose_evolution(ThermalEvolutionRequest {
            tick: 1,
            parameters: ThermalParameters::new(128, THERMAL_SCALE, THERMAL_SCALE, 0, 1)
                .expect("parameters must be valid"),
            active_region: &region,
            boundary_behavior: ThermalBoundaryBehavior::NoFluxOutsideActiveRegion,
            reservoirs: &[],
            injections: &[],
            materials: &BTreeMap::new(),
        })
        .expect("evolution must succeed");
    let receipt = proposal
        .transfer_receipts()
        .iter()
        .find(|receipt| receipt.cell.cell_index == 13)
        .expect("center receipt must exist");

    // Then: all six signed face contributions reconstruct the center's net transition.
    assert_eq!(receipt.faces.len(), 6);
    let net_outflow = receipt
        .faces
        .iter()
        .map(|face| i128::from(face.signed_flux))
        .sum::<i128>();
    assert_eq!(
        i128::from(receipt.pre_state.get()) - net_outflow,
        i128::from(receipt.post_state.get())
    );
}
