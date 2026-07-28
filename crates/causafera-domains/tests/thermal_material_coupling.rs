use std::collections::{BTreeMap, BTreeSet};

use causafera_domains::{
    ThermalActiveRegion, ThermalBoundaryBehavior, ThermalCellKey, ThermalCommittedTraces,
    ThermalEnergy, ThermalEvolutionRequest, ThermalField, ThermalFieldSet, ThermalMaterialSite,
    ThermalParameters,
};
use causafera_types::{ChartChunkCoord, ChunkCoord, SpatialChartId, TraceId};

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

fn site(retained_before: i64) -> ThermalMaterialSite {
    ThermalMaterialSite {
        retained_before: energy(retained_before),
        last_exchange: None,
    }
}

fn evolve(
    fields: &ThermalFieldSet,
    parameters: ThermalParameters,
    materials: &BTreeMap<ThermalCellKey, ThermalMaterialSite>,
) -> causafera_domains::ThermalEvolutionProposal {
    let chunks = fields.fields().keys().copied().collect::<BTreeSet<_>>();
    let active_region = ThermalActiveRegion::new(chunks.clone(), chunks).unwrap();
    fields
        .propose_evolution(ThermalEvolutionRequest {
            tick: 1,
            parameters,
            active_region: &active_region,
            boundary_behavior: ThermalBoundaryBehavior::NoFluxOutsideActiveRegion,
            reservoirs: &[],
            injections: &[],
            materials,
        })
        .unwrap()
}

#[test]
fn symmetric_heating() {
    // Given: an isolated hot cell and a cold co-located material well below capacity.
    let fields = field_set(vec![field(chunk(0), 1, vec![100])]);
    let cell = ThermalCellKey::new(chunk(0), 0);
    let materials = BTreeMap::from([(cell, site(0))]);
    let parameters = ThermalParameters::new(1, 1, 60, 10, 1_000).unwrap();

    // When: one tick exchanges energy between the cell and its material.
    let proposal = evolve(&fields, parameters, &materials);

    // Then: the material gains exactly floor(100 * 10 / 60) and the cell loses the same amount.
    let expected_flux = 100 * 10 / 60;
    assert_eq!(
        *proposal.material_retained_after().get(&cell).unwrap(),
        energy(expected_flux)
    );
    assert_eq!(
        proposal.after_state().field(chunk(0)).unwrap().energy()[0],
        energy(100 - expected_flux)
    );
    assert_eq!(proposal.conservation_receipt().residual, 0);
    assert_eq!(
        proposal
            .conservation_receipt()
            .total_material_retained_after,
        i128::from(expected_flux)
    );
}

#[test]
fn symmetric_cooling() {
    // Given: an isolated cold cell and a hot co-located material.
    let fields = field_set(vec![field(chunk(0), 1, vec![0])]);
    let cell = ThermalCellKey::new(chunk(0), 0);
    let materials = BTreeMap::from([(cell, site(50))]);
    let parameters = ThermalParameters::new(1, 1, 60, 10, 1_000).unwrap();

    // When: one tick lets the material give heat back to its colder cell.
    let proposal = evolve(&fields, parameters, &materials);

    // Then: the transfer magnitude never exceeds the material's retained energy.
    let expected_flux = 50 * 10 / 60;
    assert!(expected_flux <= 50);
    assert_eq!(
        *proposal.material_retained_after().get(&cell).unwrap(),
        energy(50 - expected_flux)
    );
    assert_eq!(
        proposal.after_state().field(chunk(0)).unwrap().energy()[0],
        energy(expected_flux)
    );
    assert_eq!(proposal.conservation_receipt().residual, 0);
}

#[test]
fn capacity_limited_heating_leaves_the_remainder_in_the_cell() {
    // Given: a very hot cell and a material already near its small capacity.
    let fields = field_set(vec![field(chunk(0), 1, vec![1_000])]);
    let cell = ThermalCellKey::new(chunk(0), 0);
    let materials = BTreeMap::from([(cell, site(95))]);
    let parameters = ThermalParameters::new(1, 1, 60, 10, 100).unwrap();

    // When: the candidate flux (150) would exceed the five units of remaining headroom.
    let proposal = evolve(&fields, parameters, &materials);

    // Then: the material saturates exactly at capacity and the rejected remainder stays in the cell.
    assert_eq!(
        *proposal.material_retained_after().get(&cell).unwrap(),
        energy(100)
    );
    assert_eq!(
        proposal.after_state().field(chunk(0)).unwrap().energy()[0],
        energy(1_000 - 5)
    );
    assert_eq!(proposal.conservation_receipt().residual, 0);
}

#[test]
fn seven_way_outflow_stays_non_negative() {
    // Given: a hot center cell with six cold neighbors and a cold co-located material sink,
    // parameterized exactly at the widened coefficient bound (6 * 10 + 40 == 100).
    let mut values = vec![0; 27];
    let center = 13;
    values[center] = 50;
    let fields = field_set(vec![field(chunk(0), 3, values)]);
    let cell = ThermalCellKey::new(chunk(0), center as u16);
    let materials = BTreeMap::from([(cell, site(0))]);
    let parameters = ThermalParameters::new(10, 1, 100, 40, 1_000).unwrap();

    // When: every one of the seven simultaneous outflows is realized in the same tick.
    let proposal = evolve(&fields, parameters, &materials);
    let after = proposal.after_state().field(chunk(0)).unwrap().energy();

    // Then: the center drains to exactly zero (not negative) and the total is conserved.
    let neighbors = [12, 14, 10, 16, 4, 22];
    assert_eq!(after[center], energy(0));
    assert!(neighbors.iter().all(|index| after[*index] == energy(5)));
    assert_eq!(
        *proposal.material_retained_after().get(&cell).unwrap(),
        energy(20)
    );
    assert_eq!(proposal.conservation_receipt().residual, 0);
}

#[test]
fn equilibrium_produces_no_material_record_or_change() {
    // Given: a cell and its co-located material already at the same energy.
    let fields = field_set(vec![field(chunk(0), 1, vec![40])]);
    let cell = ThermalCellKey::new(chunk(0), 0);
    let materials = BTreeMap::from([(cell, site(40))]);
    let parameters = ThermalParameters::new(1, 1, 60, 10, 1_000).unwrap();

    // When: one tick is proposed with no energy difference to exchange.
    let proposal = evolve(&fields, parameters, &materials);

    // Then: the material's retained energy is unchanged and no cell-change event is proposed
    // for a material-only zero-flux cell (no face flux either, in this single-cell field).
    assert_eq!(
        *proposal.material_retained_after().get(&cell).unwrap(),
        energy(40)
    );
    assert!(proposal.cell_changes().is_empty());
    assert_eq!(proposal.conservation_receipt().residual, 0);
}

#[test]
fn material_and_face_flux_cancel_leaving_no_cell_change_event() {
    // Given: a face inflow to the material-bearing cell that exactly offsets its material outflow.
    let fields = field_set(vec![
        field(chunk(0), 1, vec![50]),
        field(chunk(1), 1, vec![150]),
    ]);
    let cell = ThermalCellKey::new(chunk(0), 0);
    let materials = BTreeMap::from([(cell, site(0))]);
    let parameters = ThermalParameters::new(40, 1, 1_000, 80, 1_000).unwrap();

    // When: one tick proposes both the face exchange and the material exchange together.
    let proposal = evolve(&fields, parameters, &materials);

    // Then: the cell's net energy is unchanged (no `ThermalCellChange` event fires for it) even
    // though the material moved a nonzero amount and its transfer receipt still exists.
    assert!(
        proposal
            .cell_changes()
            .iter()
            .all(|change| change.cell != cell)
    );
    let receipt = proposal
        .transfer_receipts()
        .iter()
        .find(|receipt| receipt.cell == cell)
        .expect("material-bearing cell must still have a transfer receipt");
    assert_eq!(receipt.pre_state, energy(50));
    assert_eq!(receipt.post_state, energy(50));
    assert!(receipt.material.is_some());
    assert_eq!(
        *proposal.material_retained_after().get(&cell).unwrap(),
        energy(4)
    );
    assert_eq!(proposal.conservation_receipt().residual, 0);
}

#[test]
fn install_committed_traces_preserves_anchor_for_material_only_net_zero_cell() {
    // Given: the same cancelling face/material exchange, whose cell receipt has no accepted
    // reservoir transfer to anchor a new trace to.
    let fields = field_set(vec![
        field(chunk(0), 1, vec![50]),
        field(chunk(1), 1, vec![150]),
    ]);
    let cell = ThermalCellKey::new(chunk(0), 0);
    let materials = BTreeMap::from([(cell, site(0))]);
    let parameters = ThermalParameters::new(40, 1, 1_000, 80, 1_000).unwrap();
    let proposal = evolve(&fields, parameters, &materials);
    assert!(
        proposal
            .cell_changes()
            .iter()
            .all(|change| change.cell != cell)
    );
    let original_trace = fields.field(chunk(0)).unwrap().last_change()[0];

    // When: committed traces are installed with no cell-change and no reservoir trace for it.
    let mut after = proposal.after_state().clone();
    after.install_committed_traces(ThermalCommittedTraces {
        changes: proposal.cell_changes(),
        receipts: proposal.transfer_receipts(),
        cell_traces: &BTreeMap::new(),
        reservoir_traces: &BTreeMap::new(),
        conservation_trace: TraceId::new(99),
    });

    // Then: the cell's field anchor is left exactly as it was, rather than dangling or being
    // overwritten with a trace this tick never produced for it.
    assert_eq!(
        after.field(chunk(0)).unwrap().last_change()[0],
        original_trace
    );
}

#[test]
fn material_site_without_a_matching_field_cell_is_rejected() {
    // Given: a material site keyed to a cell that does not exist in the field set.
    let fields = field_set(vec![field(chunk(0), 1, vec![10])]);
    let phantom_cell = ThermalCellKey::new(chunk(1), 0);
    let materials = BTreeMap::from([(phantom_cell, site(0))]);
    let parameters = ThermalParameters::new(1, 1, 60, 10, 1_000).unwrap();
    let chunks = BTreeSet::from([chunk(0)]);
    let active_region = ThermalActiveRegion::new(chunks.clone(), chunks).unwrap();

    // When: evolution is proposed against a material site outside the field set.
    let result = fields.propose_evolution(ThermalEvolutionRequest {
        tick: 1,
        parameters,
        active_region: &active_region,
        boundary_behavior: ThermalBoundaryBehavior::NoFluxOutsideActiveRegion,
        reservoirs: &[],
        injections: &[],
        materials: &materials,
    });

    // Then: this internal-invariant violation is surfaced as an error, not silently tolerated.
    assert_eq!(
        result.unwrap_err(),
        causafera_domains::ThermalError::PositionOutsideField
    );
}
