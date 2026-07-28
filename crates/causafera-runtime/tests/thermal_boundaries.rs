use std::collections::{BTreeMap, BTreeSet};

use causafera_domains::{
    THERMAL_SCALE, ThermalActiveRegion, ThermalBoundaryBehavior, ThermalCellKey, ThermalEnergy,
    ThermalEvolutionRequest, ThermalField, ThermalFieldSet, ThermalInjectionProposal,
    ThermalParameters, ThermalReservoir, ThermalReservoirId, ThermalReservoirSchedule,
};
use causafera_runtime::RuntimeError;
use causafera_types::{ChartChunkCoord, ChunkCoord, SpatialChartId, TraceId};

fn chunk(x: i32) -> ChartChunkCoord {
    ChartChunkCoord::new(SpatialChartId::new(1), ChunkCoord::new(x, 0, 0))
}

#[test]
fn active_neighbor_missing() {
    // Given: two chunks in the static active region but only one resident thermal field.
    let field = ThermalField::new(chunk(0), 3, TraceId::new(1)).expect("field must be valid");
    let fields =
        ThermalFieldSet::new(vec![field], TraceId::new(1)).expect("field set must be valid");
    let active = BTreeSet::from([chunk(0), chunk(1)]);
    let region = ThermalActiveRegion::new(active.clone(), active).expect("region must be valid");

    // When: runtime-facing evolution validates residency before arithmetic.
    let result = fields.propose_evolution(ThermalEvolutionRequest {
        tick: 1,
        parameters: ThermalParameters::new(128, THERMAL_SCALE, THERMAL_SCALE, 0, 1)
            .expect("parameters must be valid"),
        active_region: &region,
        boundary_behavior: ThermalBoundaryBehavior::NoFluxOutsideActiveRegion,
        reservoirs: &[],
        injections: &[],
        materials: &BTreeMap::new(),
    });

    // Then: the domain failure maps to the dedicated runtime error without changing the fields.
    let error = result.expect_err("missing active field must reject");
    assert_eq!(
        RuntimeError::from(error),
        RuntimeError::ThermalRegionIncomplete
    );
    assert_eq!(fields.batch_sequence(), 0);
    assert!(fields.field(chunk(0)).is_some());
}

#[test]
fn real_arithmetic() {
    // Given: a nearly saturated cell and a bounded neighboring field.
    let mut energy = vec![ThermalEnergy::ZERO; 27];
    energy[13] = ThermalEnergy::new(ThermalEnergy::MAX.get() - 1)
        .expect("near-maximum energy must be valid");
    let field = ThermalField::from_energy(chunk(0), 3, energy, TraceId::new(1))
        .expect("field must be valid");
    let fields =
        ThermalFieldSet::new(vec![field], TraceId::new(1)).expect("field set must be valid");
    let active = BTreeSet::from([chunk(0)]);
    let region = ThermalActiveRegion::new(active.clone(), active).expect("region must be valid");

    // When: checked i128 arithmetic evolves the field.
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
        .expect("bounded near-maximum evolution must succeed");

    // Then: every after-state cell remains representable and conservation is exact.
    assert!(proposal.after_state().fields().values().all(|field| {
        field
            .energy()
            .iter()
            .all(|energy| energy.get() <= ThermalEnergy::MAX.get())
    }));
    assert_eq!(proposal.conservation_receipt().residual, 0);
}

#[test]
fn outside_active_region_boundary_record() {
    // Given: an edge cell whose +X face leaves the active region after a finite injection.
    let mut energy = vec![ThermalEnergy::ZERO; 27];
    energy[14] = ThermalEnergy::new(100).expect("fixture energy must be valid");
    let field = ThermalField::from_energy(chunk(0), 3, energy, TraceId::new(1))
        .expect("field must be valid");
    let fields =
        ThermalFieldSet::new(vec![field], TraceId::new(1)).expect("field set must be valid");
    let source = ThermalCellKey::new(chunk(0), 14);
    let reservoir = ThermalReservoir {
        id: ThermalReservoirId::new(1),
        target: source,
        budget: ThermalEnergy::new(60).expect("reservoir budget must be valid"),
        schedule: ThermalReservoirSchedule::OneShot,
        bootstrap_trace: TraceId::new(2),
        last_change: TraceId::new(2),
    };
    let injection = ThermalInjectionProposal {
        reservoir_id: reservoir.id,
        target: source,
        scheduled_amount: reservoir.budget,
    };
    let active = BTreeSet::from([chunk(0)]);
    let region = ThermalActiveRegion::new(active.clone(), active).expect("region must be valid");

    // When: evolution computes loaded faces while leaving the inactive +X face unrepresented.
    let proposal = fields
        .propose_evolution(ThermalEvolutionRequest {
            tick: 1,
            parameters: ThermalParameters::new(128, THERMAL_SCALE, THERMAL_SCALE, 0, 1)
                .expect("parameters must be valid"),
            active_region: &region,
            boundary_behavior: ThermalBoundaryBehavior::NoFluxOutsideActiveRegion,
            reservoirs: &[reservoir],
            injections: &[injection],
            materials: &BTreeMap::new(),
        })
        .expect("outside-active-region evolution must succeed");

    // Then: five loaded faces evolve, the missing face is recorded, and energy remains exact.
    let receipt = proposal
        .transfer_receipts()
        .iter()
        .find(|receipt| receipt.cell == source)
        .expect("source receipt must exist");
    assert_eq!(receipt.pre_state.get(), 160);
    assert_eq!(receipt.post_state.get(), 60);
    assert_eq!(receipt.faces.len(), 5);
    assert!(
        receipt
            .faces
            .iter()
            .all(|face| face.neighbor.chunk == chunk(0))
    );

    let expected_neighbor = ThermalCellKey::new(chunk(1), 12);
    let boundary_records = proposal.boundary_records();
    assert!(boundary_records.windows(2).all(|records| {
        (records[0].cell, records[0].neighbor) < (records[1].cell, records[1].neighbor)
    }));
    let matching = boundary_records
        .iter()
        .filter(|record| record.cell == source && record.neighbor == expected_neighbor)
        .collect::<Vec<_>>();
    assert_eq!(matching.len(), 1);
    assert_eq!(matching[0].cell_pre_state.get(), 160);

    let total_after = proposal
        .after_state()
        .fields()
        .values()
        .flat_map(|field| field.energy())
        .map(|energy| i128::from(energy.get()))
        .sum::<i128>();
    assert_eq!(total_after, 160);
    assert_eq!(proposal.conservation_receipt().residual, 0);
}
