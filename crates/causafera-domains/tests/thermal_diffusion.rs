use std::collections::BTreeSet;

use causafera_domains::{
    ThermalActiveRegion, ThermalBoundaryBehavior, ThermalEnergy, ThermalEvolutionRequest,
    ThermalField, ThermalFieldSet, ThermalInjectionProposal, ThermalParameters, ThermalReservoir,
    ThermalReservoirId, ThermalReservoirSchedule,
};
use causafera_types::{ChartChunkCoord, ChunkCoord, SpatialChartId, TraceId};

const SCALE: i64 = 60;
const TRANSFER_FRACTION: i64 = 10;

fn parameters() -> ThermalParameters {
    ThermalParameters::new(TRANSFER_FRACTION, 1, SCALE).unwrap()
}

fn chunk(x: i32) -> ChartChunkCoord {
    ChartChunkCoord::new(SpatialChartId::new(1), ChunkCoord::new(x, 0, 0))
}

fn energy(value: i64) -> ThermalEnergy {
    ThermalEnergy::new(value).unwrap()
}

fn field(chunk: ChartChunkCoord, extent: u8, values: Vec<i64>) -> ThermalField {
    ThermalField::from_energy(
        chunk,
        extent,
        values.into_iter().map(energy).collect(),
        TraceId::new(1),
    )
    .unwrap()
}

fn field_set(fields: Vec<ThermalField>) -> ThermalFieldSet {
    ThermalFieldSet::new(fields, TraceId::new(2)).unwrap()
}

fn evolve(fields: &ThermalFieldSet) -> causafera_domains::ThermalEvolutionProposal {
    let chunks = fields.fields().keys().copied().collect::<BTreeSet<_>>();
    let active_region = ThermalActiveRegion::new(chunks.clone(), chunks).unwrap();
    fields
        .propose_evolution(ThermalEvolutionRequest {
            tick: 1,
            parameters: parameters(),
            active_region: &active_region,
            boundary_behavior: ThermalBoundaryBehavior::NoFluxOutsideActiveRegion,
            reservoirs: &[],
            injections: &[],
        })
        .unwrap()
}

#[test]
fn hot_cell_six_cold_neighbors() {
    // Given: a 3x3x3 field with only its center energized.
    let mut values = vec![0; 27];
    let center = 13;
    values[center] = 360;
    let fields = field_set(vec![field(chunk(0), 3, values)]);

    // When: one frozen-state conservative step is proposed.
    let proposal = evolve(&fields);
    let after = proposal.after_state().field(chunk(0)).unwrap().energy();

    // Then: all six equal neighbors receive the floor flux and total energy is unchanged.
    let neighbors = [12, 14, 10, 16, 4, 22];
    assert_eq!(after[center], energy(0));
    assert!(neighbors.iter().all(|index| after[*index] == energy(60)));
    assert_eq!(proposal.conservation_receipt().residual, 0);
    assert_eq!(proposal.conservation_receipt().total_cell_energy_after, 360);
}

#[test]
fn simultaneous_six_face_transfers() {
    // Given: a center cell with distinct energies on all six incident faces.
    let mut values = vec![0; 27];
    let center = 13;
    values[center] = 600;
    values[12] = 900;
    values[14] = 0;
    values[10] = 720;
    values[16] = 480;
    values[4] = 780;
    values[22] = 540;
    let fields = field_set(vec![field(chunk(0), 3, values)]);

    // When: every incident face contributes from the same pre-state.
    let proposal = evolve(&fields);
    let after = proposal.after_state().field(chunk(0)).unwrap().energy();
    let receipt = proposal
        .transfer_receipts()
        .iter()
        .find(|receipt| receipt.cell.cell_index == center as u16)
        .unwrap();
    let net_outflow: i64 = receipt.faces.iter().map(|face| face.signed_flux).sum();

    // Then: the center's net transition equals all six signed transfers exactly.
    assert_eq!(receipt.faces.len(), 6);
    assert_eq!(after[center].get(), 600 - net_outflow);
    assert_eq!(proposal.conservation_receipt().residual, 0);
}

#[test]
fn accumulated_delta_bounds() {
    // Given: a target near the maximum and a finite reservoir with more scheduled energy.
    let fields = field_set(vec![field(chunk(0), 1, vec![ThermalEnergy::MAX.get() - 2])]);
    let reservoir = ThermalReservoir {
        id: ThermalReservoirId::new(7),
        target: causafera_domains::ThermalCellKey::new(chunk(0), 0),
        budget: energy(10),
        schedule: ThermalReservoirSchedule::PerTick(energy(10)),
        bootstrap_trace: TraceId::new(3),
        last_change: TraceId::new(3),
    };
    let chunks = BTreeSet::from([chunk(0)]);
    let active_region = ThermalActiveRegion::new(chunks.clone(), chunks).unwrap();

    // When: the scheduled injection is preflighted against the target headroom.
    let proposal = fields
        .propose_evolution(ThermalEvolutionRequest {
            tick: 1,
            parameters: parameters(),
            active_region: &active_region,
            boundary_behavior: ThermalBoundaryBehavior::NoFluxOutsideActiveRegion,
            reservoirs: &[reservoir],
            injections: &[ThermalInjectionProposal {
                reservoir_id: reservoir.id,
                target: reservoir.target,
                scheduled_amount: energy(10),
            }],
        })
        .unwrap();

    // Then: only headroom is accepted and conservation remains exact.
    let receipt = &proposal.transfer_receipts()[0].reservoirs[0];
    assert_eq!(receipt.accepted_injection, energy(2));
    assert_eq!(receipt.rejected_injection, energy(8));
    assert_eq!(
        proposal.after_state().field(chunk(0)).unwrap().energy()[0],
        ThermalEnergy::MAX
    );
    assert_eq!(proposal.conservation_receipt().residual, 0);
}

#[test]
fn upper_bound_six_hot_neighbors() {
    // Given: six maximum-energy neighbors surrounding a nearly full center cell.
    let mut values = vec![0; 27];
    let center = 13;
    values[center] = ThermalEnergy::MAX.get() - 6;
    for index in [12, 14, 10, 16, 4, 22] {
        values[index] = ThermalEnergy::MAX.get();
    }
    let fields = field_set(vec![field(chunk(0), 3, values)]);

    // When: all six permitted transfers are accumulated during preflight.
    let proposal = evolve(&fields);

    // Then: the receiver reaches but cannot exceed the fixed-point maximum.
    assert_eq!(
        proposal.after_state().field(chunk(0)).unwrap().energy()[center],
        ThermalEnergy::MAX
    );
    assert_eq!(proposal.conservation_receipt().residual, 0);
}

#[test]
fn canonical_face_order_invariant() {
    // Given: the same neighboring fields inserted in opposite construction orders.
    let left = field(chunk(0), 1, vec![360]);
    let right = field(chunk(1), 1, vec![0]);
    let first = field_set(vec![left.clone(), right.clone()]);
    let second = field_set(vec![right, left]);

    // When: both field sets enumerate their single shared face.
    let first_after = evolve(&first).after_state().clone();
    let second_after = evolve(&second).after_state().clone();

    // Then: lexicographic ownership produces one identical transfer in either order.
    assert_eq!(first_after, second_after);
    assert_eq!(
        first_after.field(chunk(0)).unwrap().energy()[0],
        energy(300)
    );
    assert_eq!(first_after.field(chunk(1)).unwrap().energy()[0], energy(60));
}

#[test]
fn frozen_state_no_cascade() {
    // Given: a three-cell chain with a hot source and cold downstream cells.
    let fields = field_set(vec![
        field(chunk(0), 1, vec![360]),
        field(chunk(1), 1, vec![0]),
        field(chunk(2), 1, vec![0]),
    ]);

    // When: one step computes every face from its committed pre-state.
    let proposal = evolve(&fields);
    let after_state = proposal.after_state();

    // Then: A's inflow to B does not cascade from B to C until a later tick.
    assert_eq!(
        after_state.field(chunk(0)).unwrap().energy()[0],
        energy(300)
    );
    assert_eq!(after_state.field(chunk(1)).unwrap().energy()[0], energy(60));
    assert_eq!(after_state.field(chunk(2)).unwrap().energy()[0], energy(0));
}
