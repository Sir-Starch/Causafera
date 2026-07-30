//! Groundwater head and lateral flow, baseflow into conveyance, and conveyance
//! storage-discharge routing.
//!
//! Covers `plans/hydrology.md` verification gates V14 and V15.

mod support;

use causafera_core::StateFingerprint;
use causafera_domains::{
    HydrologyEvolutionModel, HydrologyEvolutionProposal, HydrologyReceiptTotals,
    HydrologyTransferReceipt, process, validate_boundary_transfers, validate_paired_transfers,
};
use causafera_geography::{
    HydraulicFraction, HydraulicSubstrateCell, HydraulicSubstrateParts, HydrologyCarrierKey,
    HydrologyCellKey, HydrologyCellState, HydrologyCellStorage, HydrologyConveyanceEdge,
    HydrologyEdgeKey, HydrologyField, HydrologyStateError, SURFACE_CELL_COUNT,
};
use causafera_types::WaterVolume;

use support::{
    BOOTSTRAP_TRACE, ChunkBuilder, Ground, Scenario, cell, chunk, conveyance, edge, field_set,
    inert_substrate, storage, terrain_from,
};

/// An aquifer that conducts laterally and does nothing else.
fn aquifer(conductance: u64) -> Ground {
    Ground {
        surface_capacity: 1_000_000_000,
        soil_capacity: 0,
        groundwater_capacity: 1_000_000_000,
        infiltration_limit: 0,
        percolation: (0, 1),
        specific_yield: (1, 5),
        aquifer_base_mm: 0,
        baseflow_threshold: 0,
        baseflow: (0, 1),
        surface_conductance: 0,
        groundwater_conductance: conductance,
    }
}

fn groundwater_of(proposal: &HydrologyEvolutionProposal, key: HydrologyCellKey) -> u64 {
    proposal
        .after_state()
        .cell(key)
        .expect("the cell is resident")
        .groundwater()
        .get()
}

fn surface_of(proposal: &HydrologyEvolutionProposal, key: HydrologyCellKey) -> u64 {
    proposal
        .after_state()
        .cell(key)
        .expect("the cell is resident")
        .surface_water()
        .get()
}

fn edge_storage(proposal: &HydrologyEvolutionProposal, key: HydrologyEdgeKey) -> u64 {
    proposal
        .after_conveyance()
        .edge(key)
        .expect("the edge survived the tick")
        .storage()
        .get()
}

fn receipt_of(
    proposal: &HydrologyEvolutionProposal,
    process_kind: u32,
    source: HydrologyCarrierKey,
) -> &HydrologyTransferReceipt {
    proposal
        .transfer_receipts()
        .iter()
        .find(|receipt| receipt.process_kind() == process_kind && receipt.source() == source)
        .expect("the transfer was proposed")
}

fn assert_conserved(proposal: &HydrologyEvolutionProposal) {
    let ledger = proposal.conservation();
    assert_eq!(ledger.residual(), 0);
    let totals = HydrologyReceiptTotals::from_receipts(proposal.transfer_receipts()).unwrap();
    assert!(totals.agrees_with(ledger));
    validate_paired_transfers(proposal.transfer_receipts()).unwrap();
    validate_boundary_transfers(proposal.transfer_receipts()).unwrap();
}

// ---------------------------------------------------------------------------
// V14 — groundwater head and flow
// ---------------------------------------------------------------------------

#[test]
fn groundwater_moves_toward_the_lower_water_table() {
    // saturated depth = floor(volume * yield_den / (cell_area * yield_num)).
    // With a fifth of specific yield over a square metre, 2 000 000 mm³ stands
    // ten millimetres above a base of zero, and the dry neighbour stands at zero.
    let ground = aquifer(1_000);
    let field = ChunkBuilder::new(0)
        .with(0, ground.build(), storage(0, 0, 2_000_000))
        .with(1, ground.build(), storage(0, 0, 0))
        .build();
    let state = field_set(vec![field]);
    let proposal =
        HydrologyEvolutionModel::propose(&state, Scenario::new(&[0]).request(1)).unwrap();

    let receipt = receipt_of(
        &proposal,
        process::GROUNDWATER_LATERAL,
        HydrologyCarrierKey::Cell(cell(0, 0)),
    );
    assert_eq!(receipt.accepted(), WaterVolume::new(10_000));
    assert_eq!(groundwater_of(&proposal, cell(0, 0)), 1_990_000);
    assert_eq!(groundwater_of(&proposal, cell(0, 1)), 10_000);
    assert_eq!(
        surface_of(&proposal, cell(0, 0)),
        0,
        "a groundwater transfer never touches surface storage"
    );
    assert_conserved(&proposal);
}

#[test]
fn the_aquifer_base_shifts_the_head_it_is_measured_from() {
    // Equal stored volume, unequal aquifer base: the cell whose aquifer sits
    // higher has the higher water table and drains into the lower one.
    let high = Ground {
        aquifer_base_mm: 500,
        ..aquifer(1_000)
    };
    let low = Ground {
        aquifer_base_mm: 0,
        ..aquifer(1_000)
    };
    let field = ChunkBuilder::new(0)
        .with(0, high.build(), storage(0, 0, 1_000_000))
        .with(1, low.build(), storage(0, 0, 1_000_000))
        .build();
    let state = field_set(vec![field]);
    let proposal =
        HydrologyEvolutionModel::propose(&state, Scenario::new(&[0]).request(1)).unwrap();

    // Both hold five millimetres of saturated depth; the bases differ by 500 mm.
    assert_eq!(
        receipt_of(
            &proposal,
            process::GROUNDWATER_LATERAL,
            HydrologyCarrierKey::Cell(cell(0, 0))
        )
        .accepted(),
        WaterVolume::new(500_000)
    );
    assert_conserved(&proposal);
}

#[test]
fn a_specific_yield_of_zero_over_a_real_aquifer_is_refused_at_construction() {
    // The saturated depth divides by `cell_area * yield_num`, so the pairing is
    // rejected where it is visible rather than mid-substage.
    assert_eq!(
        HydraulicSubstrateCell::new(HydraulicSubstrateParts {
            groundwater_capacity: WaterVolume::new(1_000_000),
            specific_yield: HydraulicFraction::ZERO,
            surface_capacity: WaterVolume::ZERO,
            soil_capacity: WaterVolume::ZERO,
            infiltration_limit_per_tick: WaterVolume::ZERO,
            percolation_fraction: HydraulicFraction::ZERO,
            aquifer_base_elevation_mm: 0,
            baseflow_threshold: WaterVolume::ZERO,
            baseflow_fraction: HydraulicFraction::ZERO,
            surface_conductance_mm2_per_tick: 0,
            groundwater_conductance_mm2_per_tick: 0,
        }),
        Err(HydrologyStateError::ZeroSpecificYield)
    );
}

#[test]
fn stored_groundwater_can_never_reach_the_solver_without_a_specific_yield() {
    // Two constructors close this between them, which is why the solver's own
    // `GroundwaterWithoutSpecificYield` guard has no reachable case: a yield of
    // zero forces a capacity of zero, and a capacity of zero forces storage of
    // zero. Asserting the pair here is what makes the guard's unreachability a
    // tested property rather than a claim.
    let field = HydrologyField::from_parts(
        chunk(0),
        {
            let mut cells = vec![
                HydrologyCellState::initial(
                    HydrologyCellStorage::ZERO,
                    BOOTSTRAP_TRACE,
                    StateFingerprint::new([0; 32]),
                );
                SURFACE_CELL_COUNT
            ];
            cells[0] = HydrologyCellState::initial(
                storage(0, 0, 1_000_000),
                BOOTSTRAP_TRACE,
                StateFingerprint::new([0; 32]),
            );
            cells
        },
        vec![inert_substrate(); SURFACE_CELL_COUNT],
    );
    assert_eq!(field, Err(HydrologyStateError::StorageExceedsCapacity));
}

#[test]
fn groundwater_inflow_stops_at_the_receivers_explicit_capacity() {
    // The receiver has room for 4 000 mm³ and the head difference asks for
    // 10 000. What does not fit stays in the donor rather than vanishing.
    let donor = aquifer(1_000);
    let receiver = Ground {
        groundwater_capacity: 4_000,
        ..aquifer(1_000)
    };
    let field = ChunkBuilder::new(0)
        .with(0, donor.build(), storage(0, 0, 2_000_000))
        .with(1, receiver.build(), storage(0, 0, 0))
        .build();
    let state = field_set(vec![field]);
    let proposal =
        HydrologyEvolutionModel::propose(&state, Scenario::new(&[0]).request(1)).unwrap();

    let receipt = receipt_of(
        &proposal,
        process::GROUNDWATER_LATERAL,
        HydrologyCarrierKey::Cell(cell(0, 0)),
    );
    assert_eq!(receipt.requested(), WaterVolume::new(10_000));
    assert_eq!(receipt.accepted(), WaterVolume::new(4_000));
    assert_eq!(receipt.unaccepted(), WaterVolume::new(6_000));
    assert_eq!(groundwater_of(&proposal, cell(0, 0)), 1_996_000);
    assert_eq!(groundwater_of(&proposal, cell(0, 1)), 4_000);
    assert_conserved(&proposal);
}

// ---------------------------------------------------------------------------
// V15 — baseflow
// ---------------------------------------------------------------------------

#[test]
fn baseflow_is_the_excess_above_threshold_times_its_fraction() {
    // 1 000 stored, 400 threshold, half of the 600 excess: exactly 300.
    let ground = Ground {
        baseflow_threshold: 400,
        baseflow: (1, 2),
        ..aquifer(0)
    };
    let field = ChunkBuilder::new(0)
        .with(0, ground.build(), storage(0, 0, 1_000))
        .with(1, ground.build(), storage(0, 0, 0))
        .build();
    let state = field_set(vec![field]);
    let key = HydrologyEdgeKey::new(cell(0, 0), cell(0, 1)).unwrap();
    let scenario = Scenario::new(&[0]).with_conveyance(conveyance(vec![edge(
        cell(0, 0),
        cell(0, 1),
        0,
        10_000,
        (0, 1),
        10_000,
    )]));
    let proposal = HydrologyEvolutionModel::propose(&state, scenario.request(1)).unwrap();

    let receipt = receipt_of(
        &proposal,
        process::BASEFLOW,
        HydrologyCarrierKey::Cell(cell(0, 0)),
    );
    assert_eq!(receipt.accepted(), WaterVolume::new(300));
    assert_eq!(receipt.target(), HydrologyCarrierKey::Edge(key));
    assert_eq!(groundwater_of(&proposal, cell(0, 0)), 700);
    assert_eq!(edge_storage(&proposal, key), 300);
    assert_conserved(&proposal);
}

#[test]
fn groundwater_below_the_threshold_produces_no_baseflow() {
    let ground = Ground {
        baseflow_threshold: 1_000,
        baseflow: (1, 1),
        ..aquifer(0)
    };
    let field = ChunkBuilder::new(0)
        .with(0, ground.build(), storage(0, 0, 900))
        .with(1, ground.build(), storage(0, 0, 0))
        .build();
    let state = field_set(vec![field]);
    let scenario = Scenario::new(&[0]).with_conveyance(conveyance(vec![edge(
        cell(0, 0),
        cell(0, 1),
        0,
        10_000,
        (0, 1),
        10_000,
    )]));
    let proposal = HydrologyEvolutionModel::propose(&state, scenario.request(1)).unwrap();

    assert!(
        !proposal
            .transfer_receipts()
            .iter()
            .any(|receipt| receipt.process_kind() == process::BASEFLOW)
    );
    assert_eq!(groundwater_of(&proposal, cell(0, 0)), 900);
    assert_conserved(&proposal);
}

#[test]
fn a_cell_with_no_outgoing_edge_retains_its_groundwater() {
    // Baseflow needs somewhere to go. A local minimum has no outlet, and the
    // water stays rather than leaving the accounting.
    let ground = Ground {
        baseflow_threshold: 0,
        baseflow: (1, 1),
        ..aquifer(0)
    };
    let field = ChunkBuilder::new(0)
        .with(0, ground.build(), storage(0, 0, 5_000))
        .build();
    let state = field_set(vec![field]);
    let proposal =
        HydrologyEvolutionModel::propose(&state, Scenario::new(&[0]).request(1)).unwrap();

    assert_eq!(groundwater_of(&proposal, cell(0, 0)), 5_000);
    assert_conserved(&proposal);
}

#[test]
fn baseflow_is_bounded_by_the_edges_remaining_storage_and_inlet() {
    for (capacity, inlet, expected) in [(10_000_u64, 120_u64, 120_u64), (150, 10_000, 150)] {
        let ground = Ground {
            baseflow_threshold: 0,
            baseflow: (1, 1),
            ..aquifer(0)
        };
        let field = ChunkBuilder::new(0)
            .with(0, ground.build(), storage(0, 0, 1_000))
            .with(1, ground.build(), storage(0, 0, 0))
            .build();
        let state = field_set(vec![field]);
        let key = HydrologyEdgeKey::new(cell(0, 0), cell(0, 1)).unwrap();
        let scenario = Scenario::new(&[0]).with_conveyance(conveyance(vec![edge(
            cell(0, 0),
            cell(0, 1),
            0,
            capacity,
            (0, 1),
            inlet,
        )]));
        let proposal = HydrologyEvolutionModel::propose(&state, scenario.request(1)).unwrap();

        let receipt = receipt_of(
            &proposal,
            process::BASEFLOW,
            HydrologyCarrierKey::Cell(cell(0, 0)),
        );
        assert_eq!(receipt.requested(), WaterVolume::new(1_000));
        assert_eq!(receipt.accepted(), WaterVolume::new(expected));
        assert_eq!(edge_storage(&proposal, key), expected);
        assert_eq!(groundwater_of(&proposal, cell(0, 0)), 1_000 - expected);
        assert_conserved(&proposal);
    }
}

#[test]
fn baseflow_competes_canonically_with_groundwater_lateral_outflow() {
    // Cell 1 holds four units, owes four to its downhill neighbour and four to its
    // outgoing edge, and can pay four in total. Both demands cross the same face,
    // so the reduction order is the canonical one and each accepted amount is
    // exact.
    let donor = Ground {
        baseflow_threshold: 0,
        baseflow: (1, 1),
        aquifer_base_mm: 4,
        ..aquifer(1)
    };
    let receiver = Ground {
        aquifer_base_mm: 0,
        ..aquifer(1)
    };
    let field = ChunkBuilder::new(0)
        .with(1, donor.build(), storage(0, 0, 4))
        .with(2, receiver.build(), storage(0, 0, 0))
        .build();
    let state = field_set(vec![field]);
    let key = HydrologyEdgeKey::new(cell(0, 1), cell(0, 2)).unwrap();
    let scenario = Scenario::new(&[0]).with_conveyance(conveyance(vec![edge(
        cell(0, 1),
        cell(0, 2),
        0,
        10_000,
        (0, 1),
        10_000,
    )]));
    let proposal = HydrologyEvolutionModel::propose(&state, scenario.request(1)).unwrap();

    // Two demands of four against four available: floor(4*4/8) = 2 each, no
    // remainder to distribute.
    assert_eq!(
        receipt_of(
            &proposal,
            process::GROUNDWATER_LATERAL,
            HydrologyCarrierKey::Cell(cell(0, 1))
        )
        .accepted(),
        WaterVolume::new(2)
    );
    assert_eq!(
        receipt_of(
            &proposal,
            process::BASEFLOW,
            HydrologyCarrierKey::Cell(cell(0, 1))
        )
        .accepted(),
        WaterVolume::new(2)
    );
    assert_eq!(groundwater_of(&proposal, cell(0, 1)), 0);
    assert_eq!(groundwater_of(&proposal, cell(0, 2)), 2);
    assert_eq!(edge_storage(&proposal, key), 2);
    assert_conserved(&proposal);
}

// ---------------------------------------------------------------------------
// V15 — conveyance storage-discharge routing
// ---------------------------------------------------------------------------

#[test]
fn an_edge_releases_its_exact_fraction_onto_the_outlets_surface() {
    let ground = aquifer(0);
    let field = ChunkBuilder::new(0)
        .with(0, ground.build(), storage(0, 0, 0))
        .with(1, ground.build(), storage(0, 0, 0))
        .build();
    let state = field_set(vec![field]);
    let key = HydrologyEdgeKey::new(cell(0, 0), cell(0, 1)).unwrap();
    let scenario = Scenario::new(&[0]).with_conveyance(conveyance(vec![edge(
        cell(0, 0),
        cell(0, 1),
        1_000,
        10_000,
        (1, 4),
        10_000,
    )]));
    let proposal = HydrologyEvolutionModel::propose(&state, scenario.request(1)).unwrap();

    let receipt = receipt_of(
        &proposal,
        process::CONVEYANCE_RELEASE,
        HydrologyCarrierKey::Edge(key),
    );
    assert_eq!(receipt.accepted(), WaterVolume::new(250));
    assert_eq!(receipt.target(), HydrologyCarrierKey::Cell(cell(0, 1)));
    assert_eq!(edge_storage(&proposal, key), 750);
    assert_eq!(surface_of(&proposal, cell(0, 1)), 250);
    assert_conserved(&proposal);
}

#[test]
fn a_release_the_outlet_cannot_hold_stays_in_its_edge() {
    let ground = Ground {
        surface_capacity: 100,
        ..aquifer(0)
    };
    let field = ChunkBuilder::new(0)
        .with(0, aquifer(0).build(), storage(0, 0, 0))
        .with(1, ground.build(), storage(0, 0, 0))
        .build();
    let state = field_set(vec![field]);
    let key = HydrologyEdgeKey::new(cell(0, 0), cell(0, 1)).unwrap();
    let scenario = Scenario::new(&[0]).with_conveyance(conveyance(vec![edge(
        cell(0, 0),
        cell(0, 1),
        1_000,
        10_000,
        (1, 1),
        10_000,
    )]));
    let proposal = HydrologyEvolutionModel::propose(&state, scenario.request(1)).unwrap();

    let receipt = receipt_of(
        &proposal,
        process::CONVEYANCE_RELEASE,
        HydrologyCarrierKey::Edge(key),
    );
    assert_eq!(receipt.requested(), WaterVolume::new(1_000));
    assert_eq!(receipt.accepted(), WaterVolume::new(100));
    assert_eq!(
        edge_storage(&proposal, key),
        900,
        "the remainder is retained"
    );
    assert_eq!(surface_of(&proposal, cell(0, 1)), 100);
    assert_conserved(&proposal);
}

#[test]
fn a_zero_release_fraction_keeps_everything() {
    let field = ChunkBuilder::new(0)
        .with(0, aquifer(0).build(), storage(0, 0, 0))
        .with(1, aquifer(0).build(), storage(0, 0, 0))
        .build();
    let state = field_set(vec![field]);
    let key = HydrologyEdgeKey::new(cell(0, 0), cell(0, 1)).unwrap();
    let scenario = Scenario::new(&[0]).with_conveyance(conveyance(vec![edge(
        cell(0, 0),
        cell(0, 1),
        1_000,
        10_000,
        (0, 1),
        10_000,
    )]));
    let proposal = HydrologyEvolutionModel::propose(&state, scenario.request(1)).unwrap();

    assert!(
        !proposal
            .transfer_receipts()
            .iter()
            .any(|receipt| receipt.process_kind() == process::CONVEYANCE_RELEASE)
    );
    assert_eq!(edge_storage(&proposal, key), 1_000);
    assert_conserved(&proposal);
}

/// Three upstream edges into cell 33, whose own outgoing edge continues to 65.
fn oversubscribed_chain(order: [usize; 3]) -> Vec<HydrologyConveyanceEdge> {
    let upstream = [
        edge(cell(0, 1), cell(0, 33), 400, 10_000, (1, 1), 10_000),
        edge(cell(0, 32), cell(0, 33), 400, 10_000, (1, 1), 10_000),
        edge(cell(0, 34), cell(0, 33), 400, 10_000, (1, 1), 10_000),
    ];
    let mut edges: Vec<HydrologyConveyanceEdge> =
        order.into_iter().map(|index| upstream[index]).collect();
    edges.push(edge(cell(0, 33), cell(0, 65), 0, 500, (1, 1), 10_000));
    edges
}

#[test]
fn three_edges_oversubscribing_one_downstream_edge_allocate_exactly() {
    let mut builder = ChunkBuilder::new(0);
    for ordinal in [1_u16, 32, 33, 34, 65] {
        builder = builder.with(ordinal, aquifer(0).build(), storage(0, 0, 0));
    }
    let state = field_set(vec![builder.build()]);
    let scenario = Scenario::new(&[0]).with_conveyance(conveyance(oversubscribed_chain([0, 1, 2])));
    let proposal = HydrologyEvolutionModel::propose(&state, scenario.request(1)).unwrap();

    // Three demands of 400 against 500 of downstream room: floor(400*500/1200)
    // is 166 apiece with equal remainders, so the two spare units go to the two
    // lowest source-edge keys.
    let downstream = HydrologyEdgeKey::new(cell(0, 33), cell(0, 65)).unwrap();
    let accepted: Vec<u64> = [1_u16, 32, 34]
        .into_iter()
        .map(|ordinal| {
            let key = HydrologyEdgeKey::new(cell(0, ordinal), cell(0, 33)).unwrap();
            receipt_of(
                &proposal,
                process::CONVEYANCE_RELEASE,
                HydrologyCarrierKey::Edge(key),
            )
            .accepted()
            .get()
        })
        .collect();
    assert_eq!(accepted, vec![167, 167, 166]);
    assert_eq!(accepted.iter().sum::<u64>(), 500);
    assert_eq!(edge_storage(&proposal, downstream), 500);
    for (ordinal, retained) in [(1_u16, 233_u64), (32, 233), (34, 234)] {
        let key = HydrologyEdgeKey::new(cell(0, ordinal), cell(0, 33)).unwrap();
        assert_eq!(edge_storage(&proposal, key), retained);
    }
    assert_conserved(&proposal);
}

#[test]
fn water_entering_an_edge_this_tick_cannot_leave_it_in_the_same_tick() {
    // The downstream edge starts empty, receives 500, and releases from its frozen
    // pre-release storage of zero. No cascade is possible however long the chain.
    let mut builder = ChunkBuilder::new(0);
    for ordinal in [1_u16, 32, 33, 34, 65] {
        builder = builder.with(ordinal, aquifer(0).build(), storage(0, 0, 0));
    }
    let state = field_set(vec![builder.build()]);
    let scenario = Scenario::new(&[0]).with_conveyance(conveyance(oversubscribed_chain([0, 1, 2])));
    let proposal = HydrologyEvolutionModel::propose(&state, scenario.request(1)).unwrap();

    assert_eq!(
        surface_of(&proposal, cell(0, 65)),
        0,
        "the downstream outlet sees nothing until the next tick"
    );
    assert_eq!(
        edge_storage(
            &proposal,
            HydrologyEdgeKey::new(cell(0, 33), cell(0, 65)).unwrap()
        ),
        500
    );
    assert_conserved(&proposal);
}

#[test]
fn edge_insertion_order_does_not_change_the_result() {
    let mut builder = ChunkBuilder::new(0);
    for ordinal in [1_u16, 32, 33, 34, 65] {
        builder = builder.with(ordinal, aquifer(0).build(), storage(0, 0, 0));
    }
    let state = field_set(vec![builder.build()]);

    let mut proposals = Vec::new();
    for order in [[0, 1, 2], [2, 1, 0], [1, 0, 2], [1, 2, 0]] {
        let scenario = Scenario::new(&[0]).with_conveyance(conveyance(oversubscribed_chain(order)));
        proposals.push(HydrologyEvolutionModel::propose(&state, scenario.request(1)).unwrap());
    }
    for proposal in &proposals[1..] {
        assert_eq!(proposal, &proposals[0]);
    }
}

#[test]
fn a_surface_face_carrying_a_directed_edge_fills_the_edge_not_the_neighbour() {
    // Cell 0 is above cell 1 and the face between them carries an edge directed
    // that way, so the water enters the channel instead of the neighbour's
    // surface. A reverse-head transfer would take the ordinary path.
    let ground = Ground {
        surface_conductance: 1_000,
        ..aquifer(0)
    };
    let field = ChunkBuilder::new(0)
        .with(0, ground.build(), storage(1_000_000, 0, 0))
        .with(1, ground.build(), storage(0, 0, 0))
        .build();
    let state = field_set(vec![field]);
    let key = HydrologyEdgeKey::new(cell(0, 0), cell(0, 1)).unwrap();
    let scenario = Scenario::new(&[0])
        .with_terrain(terrain_from(
            &[0],
            |_, ordinal| {
                if ordinal == 0 { 100 } else { 0 }
            },
        ))
        .with_conveyance(conveyance(vec![edge(
            cell(0, 0),
            cell(0, 1),
            0,
            10_000_000,
            (0, 1),
            10_000_000,
        )]));
    let proposal = HydrologyEvolutionModel::propose(&state, scenario.request(1)).unwrap();

    let receipt = receipt_of(
        &proposal,
        process::CONVEYANCE_INFLOW,
        HydrologyCarrierKey::Cell(cell(0, 0)),
    );
    assert_eq!(receipt.accepted(), WaterVolume::new(101_000));
    assert_eq!(receipt.target(), HydrologyCarrierKey::Edge(key));
    assert_eq!(edge_storage(&proposal, key), 101_000);
    assert_eq!(
        surface_of(&proposal, cell(0, 1)),
        0,
        "the neighbour's surface received nothing"
    );
    assert_conserved(&proposal);
}

#[test]
fn a_reverse_head_transfer_never_enters_the_directed_edge() {
    // The edge runs from cell 0 to cell 1; the head runs the other way. The water
    // takes the ordinary surface path and the channel is left alone.
    let ground = Ground {
        surface_conductance: 1_000,
        ..aquifer(0)
    };
    let field = ChunkBuilder::new(0)
        .with(0, ground.build(), storage(0, 0, 0))
        .with(1, ground.build(), storage(1_000_000, 0, 0))
        .build();
    let state = field_set(vec![field]);
    let key = HydrologyEdgeKey::new(cell(0, 0), cell(0, 1)).unwrap();
    let scenario = Scenario::new(&[0])
        .with_terrain(terrain_from(
            &[0],
            |_, ordinal| {
                if ordinal == 1 { 100 } else { 0 }
            },
        ))
        .with_conveyance(conveyance(vec![edge(
            cell(0, 0),
            cell(0, 1),
            0,
            10_000_000,
            (0, 1),
            10_000_000,
        )]));
    let proposal = HydrologyEvolutionModel::propose(&state, scenario.request(1)).unwrap();

    let receipt = receipt_of(
        &proposal,
        process::SURFACE_LATERAL,
        HydrologyCarrierKey::Cell(cell(0, 1)),
    );
    assert_eq!(receipt.accepted(), WaterVolume::new(101_000));
    assert_eq!(receipt.target(), HydrologyCarrierKey::Cell(cell(0, 0)));
    assert_eq!(edge_storage(&proposal, key), 0);
    assert_eq!(surface_of(&proposal, cell(0, 0)), 101_000);
    assert_conserved(&proposal);
}

#[test]
fn surface_inflow_spends_the_inlet_budget_before_baseflow_sees_it() {
    // One inlet budget, two claimants, and the substage order decides. Surface
    // routing runs first, so what it takes is not available to baseflow.
    let ground = Ground {
        surface_conductance: 1_000,
        baseflow_threshold: 0,
        baseflow: (1, 1),
        aquifer_base_mm: 0,
        ..aquifer(0)
    };
    let field = ChunkBuilder::new(0)
        .with(0, ground.build(), storage(1_000_000, 0, 5_000))
        .with(1, ground.build(), storage(0, 0, 0))
        .build();
    let state = field_set(vec![field]);
    let key = HydrologyEdgeKey::new(cell(0, 0), cell(0, 1)).unwrap();
    let scenario = Scenario::new(&[0])
        .with_terrain(terrain_from(
            &[0],
            |_, ordinal| {
                if ordinal == 0 { 100 } else { 0 }
            },
        ))
        .with_conveyance(conveyance(vec![edge(
            cell(0, 0),
            cell(0, 1),
            0,
            10_000_000,
            (0, 1),
            101_200,
        )]));
    let proposal = HydrologyEvolutionModel::propose(&state, scenario.request(1)).unwrap();

    assert_eq!(
        receipt_of(
            &proposal,
            process::CONVEYANCE_INFLOW,
            HydrologyCarrierKey::Cell(cell(0, 0))
        )
        .accepted(),
        WaterVolume::new(101_000)
    );
    let baseflow = receipt_of(
        &proposal,
        process::BASEFLOW,
        HydrologyCarrierKey::Cell(cell(0, 0)),
    );
    assert_eq!(baseflow.requested(), WaterVolume::new(5_000));
    assert_eq!(
        baseflow.accepted(),
        WaterVolume::new(200),
        "only the unspent inlet budget remained"
    );
    assert_eq!(edge_storage(&proposal, key), 101_200);
    assert_eq!(groundwater_of(&proposal, cell(0, 0)), 4_800);
    assert_conserved(&proposal);
}
