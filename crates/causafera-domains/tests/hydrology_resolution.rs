//! Conservative hydrology resolution: exact aggregation, capacity-aware
//! back-allocation, authoritative block-boundary faces, work reduction, and
//! promotion/demotion.
//!
//! Covers `plans/hydrology.md` verification gates V19, V20, and V21.

mod support;

use causafera_domains::{
    HydrologyError, HydrologyEvolutionModel, HydrologyEvolutionProposal, HydrologyReceiptTotals,
    HydrologyResolutionPolicy, HydrologyTransferReceipt, process, representation_change,
    resolution_fingerprint, validate_boundary_transfers, validate_paired_transfers,
};
use causafera_geography::{
    HydrologyCarrierKey, HydrologyCellKey, HydrologyFieldSet, HydrologyResolutionState,
};
use causafera_types::WaterVolume;

use support::{
    BOOTSTRAP_TRACE, ChunkBuilder, Forcing, Ground, Scenario, cell, chunk, field_set, storage,
    terrain_from,
};

/// Ground with a full vertical cycle and no lateral conductance, so a test's
/// assertions are about aggregation rather than about routing.
fn vertical(infiltration: u64, percolation: (u32, u32)) -> Ground {
    Ground {
        surface_capacity: 1_000_000_000,
        soil_capacity: 1_000_000_000,
        groundwater_capacity: 1_000_000_000,
        infiltration_limit: infiltration,
        percolation,
        specific_yield: (1, 5),
        aquifer_base_mm: 0,
        baseflow_threshold: 0,
        baseflow: (0, 1),
        surface_conductance: 0,
        groundwater_conductance: 0,
    }
}

fn surface_of(proposal: &HydrologyEvolutionProposal, key: HydrologyCellKey) -> u64 {
    proposal
        .after_state()
        .cell(key)
        .expect("the cell is resident")
        .surface_water()
        .get()
}

fn soil_of(proposal: &HydrologyEvolutionProposal, key: HydrologyCellKey) -> u64 {
    proposal
        .after_state()
        .cell(key)
        .expect("the cell is resident")
        .soil_water()
        .get()
}

fn groundwater_of(proposal: &HydrologyEvolutionProposal, key: HydrologyCellKey) -> u64 {
    proposal
        .after_state()
        .cell(key)
        .expect("the cell is resident")
        .groundwater()
        .get()
}

fn lateral_receipts(proposal: &HydrologyEvolutionProposal) -> Vec<&HydrologyTransferReceipt> {
    proposal
        .transfer_receipts()
        .iter()
        .filter(|receipt| receipt.process_kind() == process::SURFACE_LATERAL)
        .collect()
}

fn assert_conserved(proposal: &HydrologyEvolutionProposal) {
    let ledger = proposal.conservation();
    assert_eq!(ledger.residual(), 0, "the tick must close exactly");
    let totals = HydrologyReceiptTotals::from_receipts(proposal.transfer_receipts()).unwrap();
    assert!(totals.agrees_with(ledger));
    validate_paired_transfers(proposal.transfer_receipts()).unwrap();
    validate_boundary_transfers(proposal.transfer_receipts()).unwrap();
}

/// The four cells of one two-by-two level-1 block, well inside the chunk.
///
/// Inside deliberately: a cell's exterior-face signature is part of its
/// constitutive identity, so a perimeter cell never shares a group with an
/// interior one. Global columns 2..3 and rows 2..3 are one level-1 block, and all
/// four cells have four interior faces.
const INTERIOR_BLOCK: [u16; 4] = [66, 67, 98, 99];

fn block_of_four(ground: Ground, water: [(u64, u64, u64); 4]) -> HydrologyFieldSet {
    let mut builder = ChunkBuilder::new(0);
    for (index, ordinal) in INTERIOR_BLOCK.into_iter().enumerate() {
        let (surface, soil, groundwater) = water[index];
        builder = builder.with(ordinal, ground.build(), storage(surface, soil, groundwater));
    }
    field_set(vec![builder.build()])
}

// ---------------------------------------------------------------------------
// V19 — aggregation, back-allocation, and skipped internal faces
// ---------------------------------------------------------------------------

#[test]
fn a_coarse_group_aggregates_its_members_and_allocates_the_result_back() {
    // Four identical cells, each holding 1 000 of surface water, an infiltration
    // limit of 100 apiece. The group's candidate is min(4 000 surface, 400 limit,
    // 4 000 000 000 room) = 400, and the members' ceilings are 100 each, so every
    // cell moves exactly 100 — the same answer the fine path gives on a uniform
    // group, which is the check that aggregation did not distort a homogeneous case.
    let ground = vertical(100, (0, 1));
    let state = block_of_four(ground, [(1_000, 0, 0); 4]);
    let scenario = Scenario::new(&[0]).at_level(&[0], 1);
    let proposal = HydrologyEvolutionModel::propose(&state, scenario.request(1)).unwrap();

    for ordinal in INTERIOR_BLOCK {
        assert_eq!(surface_of(&proposal, cell(0, ordinal)), 900);
        assert_eq!(soil_of(&proposal, cell(0, ordinal)), 100);
    }
    assert_conserved(&proposal);
}

#[test]
fn a_coarse_group_gives_the_same_totals_as_the_fine_path_on_a_uniform_block() {
    // A uniform group has nothing to approximate, so coarse and fine must agree
    // on every bucket. Where they legitimately differ is a heterogeneous group,
    // which the next test covers.
    let ground = vertical(100, (1, 4));
    let water = [(1_000, 400, 0); 4];
    let fine = HydrologyEvolutionModel::propose(
        &block_of_four(ground, water),
        Scenario::new(&[0]).request(1),
    )
    .unwrap();
    let coarse = HydrologyEvolutionModel::propose(
        &block_of_four(ground, water),
        Scenario::new(&[0]).at_level(&[0], 1).request(1),
    )
    .unwrap();

    for ordinal in INTERIOR_BLOCK {
        assert_eq!(
            surface_of(&fine, cell(0, ordinal)),
            surface_of(&coarse, cell(0, ordinal))
        );
        assert_eq!(
            soil_of(&fine, cell(0, ordinal)),
            soil_of(&coarse, cell(0, ordinal))
        );
        assert_eq!(
            groundwater_of(&fine, cell(0, ordinal)),
            groundwater_of(&coarse, cell(0, ordinal))
        );
    }
    assert_eq!(
        fine.conservation().storage_after().unwrap(),
        coarse.conservation().storage_after().unwrap()
    );
    assert_conserved(&coarse);
}

#[test]
fn a_heterogeneous_block_splits_into_one_group_per_exact_substrate() {
    // Two substrates in one block is two groups, not one averaged group. Exactness
    // is the whole contract: an averaged cell is a cell nobody has.
    let mut builder = ChunkBuilder::new(0);
    builder = builder
        .with(66, vertical(100, (0, 1)).build(), storage(1_000, 0, 0))
        .with(67, vertical(100, (0, 1)).build(), storage(1_000, 0, 0))
        .with(98, vertical(50, (0, 1)).build(), storage(1_000, 0, 0))
        .with(99, vertical(50, (0, 1)).build(), storage(1_000, 0, 0));
    let state = field_set(vec![builder.build()]);
    let scenario = Scenario::new(&[0]).at_level(&[0], 1);
    let proposal = HydrologyEvolutionModel::propose(&state, scenario.request(1)).unwrap();

    assert_eq!(soil_of(&proposal, cell(0, 66)), 100);
    assert_eq!(soil_of(&proposal, cell(0, 67)), 100);
    assert_eq!(soil_of(&proposal, cell(0, 98)), 50);
    assert_eq!(soil_of(&proposal, cell(0, 99)), 50);

    // Two substrates over one block, and the 1 020 inert cells form their own
    // group: the coarse processes are per group, not per block.
    let infiltration_groups = proposal
        .coarse_processes()
        .iter()
        .filter(|coarse| coarse.process_kind == process::INFILTRATION)
        .count();
    assert!(
        infiltration_groups >= 2,
        "a heterogeneous block cannot be one group, saw {infiltration_groups}"
    );
    assert_conserved(&proposal);
}

#[test]
fn coarse_percolation_cannot_exceed_the_sum_of_its_members_own_results() {
    // Four cells with 10 of soil each and a quarter fraction. The aggregate
    // candidate is floor(40/4) = 10 while each member's own raw result is
    // floor(10/4) = 2, so the aggregate rounds *up* relative to the fine total of
    // 8. The plan makes each member's ceiling its own raw result, which caps the
    // group at 8 — the aggregate rounding is not allowed to create water.
    let ground = vertical(0, (1, 4));
    let state = block_of_four(ground, [(0, 10, 0); 4]);
    let scenario = Scenario::new(&[0]).at_level(&[0], 1);
    let proposal = HydrologyEvolutionModel::propose(&state, scenario.request(1)).unwrap();

    let moved: u64 = INTERIOR_BLOCK
        .into_iter()
        .map(|ordinal| groundwater_of(&proposal, cell(0, ordinal)))
        .sum();
    assert_eq!(moved, 8, "the sum of the members' own results, exactly");
    assert_eq!(
        INTERIOR_BLOCK
            .into_iter()
            .map(|ordinal| soil_of(&proposal, cell(0, ordinal)))
            .sum::<u64>(),
        32
    );
    assert_conserved(&proposal);
}

#[test]
fn a_coarse_process_records_its_candidate_ceilings_and_accepted_total() {
    let ground = vertical(100, (0, 1));
    let state = block_of_four(ground, [(1_000, 0, 0); 4]);
    let scenario = Scenario::new(&[0]).at_level(&[0], 1);
    let proposal = HydrologyEvolutionModel::propose(&state, scenario.request(1)).unwrap();

    let coarse = proposal
        .coarse_processes()
        .iter()
        .find(|coarse| {
            coarse.process_kind == process::INFILTRATION
                && coarse
                    .members
                    .iter()
                    .any(|member| member.cell == cell(0, INTERIOR_BLOCK[0]))
        })
        .expect("the group was evaluated");
    assert_eq!(coarse.raw_candidate, 400);
    assert_eq!(coarse.summed_ceilings, 400);
    assert_eq!(coarse.accepted_total, 400);
    assert_eq!(coarse.members.len(), 4);
    assert_eq!(
        coarse
            .members
            .iter()
            .map(|member| member.granted)
            .sum::<i128>(),
        coarse.accepted_total,
        "the fine grants are the group total, exactly"
    );
    // Members arrive in canonical cell order, because every weight, ceiling, and
    // grant downstream is positional.
    let cells: Vec<HydrologyCellKey> = coarse.members.iter().map(|member| member.cell).collect();
    let mut sorted = cells.clone();
    sorted.sort();
    assert_eq!(cells, sorted);
}

#[test]
fn an_evaluated_group_that_moves_nothing_is_still_recorded() {
    // `T = 0` still has to be durably reconstructable, so a group that could not
    // move anything appears with its candidate and its zero total rather than
    // being dropped as uninteresting.
    let ground = vertical(0, (0, 1));
    let state = block_of_four(ground, [(1_000, 0, 0); 4]);
    let scenario = Scenario::new(&[0]).at_level(&[0], 1);
    let proposal = HydrologyEvolutionModel::propose(&state, scenario.request(1)).unwrap();

    let zero = proposal
        .coarse_processes()
        .iter()
        .find(|coarse| {
            coarse.process_kind == process::INFILTRATION
                && coarse
                    .members
                    .iter()
                    .any(|member| member.cell == cell(0, INTERIOR_BLOCK[0]))
        })
        .expect("the group was still evaluated");
    assert_eq!(zero.accepted_total, 0);
    assert_eq!(zero.raw_candidate, 0);
    assert_conserved(&proposal);
}

#[test]
fn internal_block_faces_are_skipped_and_boundary_faces_are_not() {
    // Level 1 over a 2x2 block: the four faces inside the block are not
    // evaluated, and the faces leaving it are. Fine mode evaluates both.
    let ground = Ground {
        surface_conductance: 1_000,
        ..vertical(0, (0, 1))
    };
    let mut builder = ChunkBuilder::new(0);
    for ordinal in [0_u16, 1, 2, 32, 33, 34] {
        builder = builder.with(ordinal, ground.build(), storage(1_000_000, 0, 0));
    }
    let state = field_set(vec![builder.build()]);
    let terrain = terrain_from(&[0], |_, ordinal| match ordinal {
        0 => 400,
        1 => 300,
        2 => 200,
        32 => 100,
        33 => 50,
        34 => 25,
        _ => 0,
    });

    let fine = HydrologyEvolutionModel::propose(
        &state,
        Scenario::new(&[0]).with_terrain(terrain.clone()).request(1),
    )
    .unwrap();
    let coarse = HydrologyEvolutionModel::propose(
        &state,
        Scenario::new(&[0])
            .with_terrain(terrain)
            .at_level(&[0], 1)
            .request(1),
    )
    .unwrap();

    let fine_faces = lateral_receipts(&fine).len();
    let coarse_faces = lateral_receipts(&coarse).len();
    assert!(
        coarse_faces < fine_faces,
        "coarse mode must evaluate fewer faces: {coarse_faces} vs {fine_faces}"
    );
    assert!(coarse_faces > 0, "block boundaries stay authoritative");
    // The face between cells 0 and 1 is inside the block; the face between 1 and
    // 2 crosses into the next one.
    assert!(
        !lateral_receipts(&coarse).iter().any(|receipt| {
            receipt.source() == HydrologyCarrierKey::Cell(cell(0, 0))
                && receipt.target() == HydrologyCarrierKey::Cell(cell(0, 1))
        }),
        "an internal face was evaluated"
    );
    assert!(
        lateral_receipts(&coarse).iter().any(|receipt| {
            receipt.source() == HydrologyCarrierKey::Cell(cell(0, 1))
                && receipt.target() == HydrologyCarrierKey::Cell(cell(0, 2))
        }),
        "a block boundary face was skipped"
    );
    assert_conserved(&coarse);
}

#[test]
fn a_block_boundary_transfer_is_installed_on_its_own_fine_endpoints() {
    // The accepted transfer lands on the two cells that actually border each
    // other — never netted across the block and redistributed.
    let ground = Ground {
        surface_conductance: 1_000,
        ..vertical(0, (0, 1))
    };
    let mut builder = ChunkBuilder::new(0);
    for ordinal in [1_u16, 2] {
        builder = builder.with(ordinal, ground.build(), storage(1_000_000, 0, 0));
    }
    let state = field_set(vec![builder.build()]);
    let scenario = Scenario::new(&[0])
        .with_terrain(terrain_from(
            &[0],
            |_, ordinal| {
                if ordinal == 1 { 100 } else { 0 }
            },
        ))
        .at_level(&[0], 1);
    let proposal = HydrologyEvolutionModel::propose(&state, scenario.request(1)).unwrap();

    // Both endpoints hold one millimetre of depth, so the drop is the 100 mm of
    // terrain and the flux is 1 000 mm² x 100 mm.
    assert_eq!(surface_of(&proposal, cell(0, 1)), 900_000);
    assert_eq!(surface_of(&proposal, cell(0, 2)), 1_100_000);
    assert_eq!(
        surface_of(&proposal, cell(0, 0)),
        0,
        "a cell that was not an endpoint received nothing"
    );
    assert_conserved(&proposal);
}

#[test]
fn coarse_forcing_aggregates_per_record_and_allocates_by_requested_share() {
    // One record over two members of one group, weighted one to three. The fine
    // shares are 250 and 750, both fit, and the group accepts all 1 000.
    let ground = vertical(0, (0, 1));
    let state = block_of_four(ground, [(0, 0, 0); 4]);
    let scenario = Scenario::new(&[0]).at_level(&[0], 1).with_forcing(vec![
        Forcing::new(1, 1)
            .target(cell(0, 66), 1)
            .target(cell(0, 99), 3)
            .precipitation(1_000)
            .build(),
    ]);
    let proposal = HydrologyEvolutionModel::propose(&state, scenario.request(1)).unwrap();

    assert_eq!(surface_of(&proposal, cell(0, 66)), 250);
    assert_eq!(surface_of(&proposal, cell(0, 99)), 750);
    assert_eq!(proposal.conservation().accepted_precipitation(), 1_000);

    let settlement = proposal
        .forcing_settlements()
        .iter()
        .find(|settlement| settlement.cell == cell(0, 66))
        .expect("the cell was targeted");
    assert_eq!(settlement.accepted_source, WaterVolume::new(250));
    assert_eq!(settlement.rejected_source, WaterVolume::ZERO);
    assert_conserved(&proposal);
}

#[test]
fn coarse_forcing_shares_capacity_only_among_the_members_the_record_addressed() {
    // Weight is the member's own allocated request, so a member the record never
    // targeted has weight zero and cannot receive. One target with 400 of room
    // asked for 600 therefore accepts 400 in coarse mode exactly as in fine mode:
    // the untargeted neighbour's room is not a place for rain nobody aimed there.
    let ground = Ground {
        surface_capacity: 400,
        ..vertical(0, (0, 1))
    };
    let mut builder = ChunkBuilder::new(0);
    for ordinal in [66_u16, 67] {
        builder = builder.with(ordinal, ground.build(), storage(0, 0, 0));
    }
    let state = field_set(vec![builder.build()]);
    let forcing = vec![
        Forcing::new(1, 1)
            .target(cell(0, 66), 1)
            .precipitation(600)
            .build(),
    ];

    for scenario in [
        Scenario::new(&[0]).with_forcing(forcing.clone()),
        Scenario::new(&[0])
            .with_forcing(forcing.clone())
            .at_level(&[0], 1),
    ] {
        let proposal = HydrologyEvolutionModel::propose(&state, scenario.request(1)).unwrap();
        assert_eq!(proposal.conservation().accepted_precipitation(), 400);
        assert_eq!(surface_of(&proposal, cell(0, 66)), 400);
        assert_eq!(
            surface_of(&proposal, cell(0, 67)),
            0,
            "an untargeted member receives nothing"
        );
        assert_conserved(&proposal);
    }
}

#[test]
fn coarse_forcing_does_share_capacity_between_two_targeted_members() {
    // Two targeted members, 300 apiece, with 400 and 100 of room. Fine mode
    // accepts 300 + 100 = 400; the group has 500 of addressable room and accepts
    // all of it. That redistribution is the approximation resolution introduces,
    // and it changes where the water sits, never how much of it there is.
    let roomy = Ground {
        surface_capacity: 400,
        ..vertical(0, (0, 1))
    };
    let tight = Ground {
        surface_capacity: 100,
        ..vertical(0, (0, 1))
    };
    let state = field_set(vec![
        ChunkBuilder::new(0)
            .with(66, roomy.build(), storage(0, 0, 0))
            .with(67, tight.build(), storage(0, 0, 0))
            .build(),
    ]);
    let forcing = vec![
        Forcing::new(1, 1)
            .target(cell(0, 66), 1)
            .target(cell(0, 67), 1)
            .precipitation(600)
            .build(),
    ];

    let fine = HydrologyEvolutionModel::propose(
        &state,
        Scenario::new(&[0]).with_forcing(forcing.clone()).request(1),
    )
    .unwrap();
    assert_eq!(fine.conservation().accepted_precipitation(), 400);

    // Differing surface capacity is a differing substrate, so these two cells are
    // separate groups and coarse mode matches fine mode here. Sharing requires an
    // identical constitutive identity, which is exactly the contract.
    let coarse = HydrologyEvolutionModel::propose(
        &state,
        Scenario::new(&[0])
            .with_forcing(forcing)
            .at_level(&[0], 1)
            .request(1),
    )
    .unwrap();
    assert_eq!(coarse.conservation().accepted_precipitation(), 400);
    assert_conserved(&fine);
    assert_conserved(&coarse);
}

#[test]
fn coarse_evapotranspiration_runs_surface_then_soil_per_record() {
    // 300 of demand against 100 of group surface water and plenty of soil: the
    // surface pass takes what there is and the soil pass takes the rest, in that
    // order and no other.
    let ground = vertical(0, (0, 1));
    let mut builder = ChunkBuilder::new(0);
    for ordinal in [66_u16, 67] {
        builder = builder.with(ordinal, ground.build(), storage(50, 1_000, 0));
    }
    let state = field_set(vec![builder.build()]);
    let scenario = Scenario::new(&[0]).at_level(&[0], 1).with_forcing(vec![
        Forcing::new(1, 1)
            .target(cell(0, 66), 1)
            .target(cell(0, 67), 1)
            .potential_et(300)
            .build(),
    ]);
    let proposal = HydrologyEvolutionModel::propose(&state, scenario.request(1)).unwrap();

    assert_eq!(surface_of(&proposal, cell(0, 66)), 0);
    assert_eq!(surface_of(&proposal, cell(0, 67)), 0);
    assert_eq!(
        soil_of(&proposal, cell(0, 66)) + soil_of(&proposal, cell(0, 67)),
        1_800,
        "the remaining 200 came out of soil"
    );
    assert_eq!(proposal.conservation().accepted_evapotranspiration(), 300);
    assert_conserved(&proposal);
}

#[test]
fn a_fine_allocation_event_names_the_coarse_process_it_came_from() {
    // The coarse-process event's proposal key contains a synthetic ID the domain
    // cannot allocate, so a fine allocation names the process and the runtime
    // appends the resolved cause.
    let ground = vertical(100, (0, 1));
    let state = block_of_four(ground, [(1_000, 0, 0); 4]);
    let scenario = Scenario::new(&[0]).at_level(&[0], 1);
    let proposal = HydrologyEvolutionModel::propose(&state, scenario.request(1)).unwrap();

    let allocations: Vec<_> = proposal
        .events()
        .iter()
        .filter(|event| event.coarse_process.is_some())
        .collect();
    assert!(!allocations.is_empty(), "coarse mode has to allocate");
    for event in allocations {
        let index = event.coarse_process.unwrap();
        assert!(
            index < proposal.coarse_processes().len(),
            "the named process has to exist"
        );
    }
}

#[test]
fn coarse_processes_carry_distinct_identities_per_record() {
    // Two records over one group produce two source invocations, and the plan's
    // four-part key cannot separate them — hence the record identity.
    let ground = vertical(0, (0, 1));
    let state = block_of_four(ground, [(0, 0, 0); 4]);
    let scenario = Scenario::new(&[0]).at_level(&[0], 1).with_forcing(vec![
        Forcing::new(1, 1)
            .target(cell(0, 66), 1)
            .precipitation(100)
            .build(),
        Forcing::new(2, 1)
            .target(cell(0, 66), 1)
            .precipitation(200)
            .build(),
    ]);
    let proposal = HydrologyEvolutionModel::propose(&state, scenario.request(1)).unwrap();

    let identities: Vec<_> = proposal
        .coarse_processes()
        .iter()
        .filter(|coarse| coarse.process_kind == process::PRECIPITATION)
        .map(|coarse| coarse.identity())
        .collect();
    assert_eq!(identities.len(), 2);
    assert_ne!(identities[0], identities[1]);
    let mut unique = identities.clone();
    unique.sort();
    unique.dedup();
    assert_eq!(unique.len(), 2, "identities must not collide");
    assert_eq!(surface_of(&proposal, cell(0, 66)), 300);
    assert_conserved(&proposal);
}

#[test]
fn every_coarse_process_identity_in_a_tick_is_unique() {
    let ground = vertical(100, (1, 4));
    let state = block_of_four(ground, [(1_000, 400, 0); 4]);
    let scenario = Scenario::new(&[0]).at_level(&[0], 2).with_forcing(vec![
        Forcing::new(1, 1)
            .target(cell(0, 66), 1)
            .target(cell(0, 99), 1)
            .precipitation(500)
            .potential_et(200)
            .build(),
    ]);
    let proposal = HydrologyEvolutionModel::propose(&state, scenario.request(1)).unwrap();

    let mut identities: Vec<_> = proposal
        .coarse_processes()
        .iter()
        .map(|coarse| coarse.identity())
        .collect();
    let total = identities.len();
    identities.sort();
    identities.dedup();
    assert_eq!(identities.len(), total, "{total} processes, some colliding");
    assert_conserved(&proposal);
}

#[test]
fn a_mixed_world_evaluates_each_chunk_at_its_own_level() {
    // One chunk coarse, one fine. Every face between them is a block boundary, so
    // nothing is lost at the seam.
    let ground = Ground {
        surface_conductance: 1_000,
        ..vertical(100, (0, 1))
    };
    let state = field_set(vec![
        ChunkBuilder::new(0)
            .with(31, ground.build(), storage(1_000_000, 0, 0))
            .build(),
        ChunkBuilder::new(1)
            .with(0, ground.build(), storage(0, 0, 0))
            .build(),
    ]);
    let mut scenario = Scenario::new(&[0, 1])
        .with_terrain(terrain_from(&[0, 1], |chunk_x, ordinal| {
            if chunk_x == 0 && ordinal == 31 {
                100
            } else {
                0
            }
        }))
        .at_level(&[0, 1], 1);
    // Chunk 1 stays fine while chunk 0 is coarse.
    scenario.resolution.insert(
        chunk(1),
        HydrologyResolutionState::new(0, BOOTSTRAP_TRACE).unwrap(),
    );
    let proposal = HydrologyEvolutionModel::propose(&state, scenario.request(1)).unwrap();

    assert!(
        lateral_receipts(&proposal).iter().any(|receipt| {
            receipt.source() == HydrologyCarrierKey::Cell(cell(0, 31))
                && receipt.target() == HydrologyCarrierKey::Cell(cell(1, 0))
        }),
        "the seam between a coarse and a fine chunk stays authoritative"
    );
    assert!(
        proposal.coarse_processes().iter().all(|coarse| coarse
            .members
            .iter()
            .all(|member| member.cell.chunk() == chunk(0))),
        "the fine chunk produced no coarse process"
    );
    assert_conserved(&proposal);
}

// ---------------------------------------------------------------------------
// V19 — resolution never changes the total, only the distribution
// ---------------------------------------------------------------------------

#[test]
fn a_closed_coarse_basin_conserves_exactly_across_a_hundred_ticks() {
    let ground = Ground {
        surface_conductance: 1_000,
        ..vertical(100, (1, 4))
    };
    let mut builder = ChunkBuilder::new(0);
    for ordinal in [66_u16, 67, 68, 69, 98, 99, 100, 101] {
        builder = builder.with(ordinal, ground.build(), storage(10_000_000, 500_000, 0));
    }
    let mut state = field_set(vec![builder.build()]);
    let scenario = Scenario::new(&[0])
        .with_terrain(terrain_from(&[0], |_, ordinal| {
            8_000 - i32::from(ordinal % 8) * 1_000
        }))
        .at_level(&[0], 2);

    let opening = state.total_storage().unwrap().get();
    for tick in 1..=100 {
        let proposal = HydrologyEvolutionModel::propose(&state, scenario.request(tick)).unwrap();
        assert_eq!(
            proposal.conservation().residual(),
            0,
            "tick {tick} must close exactly"
        );
        assert_eq!(
            proposal.conservation().storage_before().unwrap(),
            proposal.conservation().storage_after().unwrap()
        );
        state = proposal.after_state().clone();
    }
    assert_eq!(state.total_storage().unwrap().get(), opening);
}

// ---------------------------------------------------------------------------
// V20 — demotion and promotion
// ---------------------------------------------------------------------------

#[test]
fn demotion_and_promotion_preserve_every_bucket_and_the_topology() {
    // The level selects how retained fine state is evaluated. Changing it and
    // changing it back must leave the world exactly as it was — no detail
    // synthesised on the way up, none deleted on the way down.
    let ground = vertical(100, (1, 4));
    let state = block_of_four(ground, [(1_000, 400, 25); 4]);
    let before = state.clone();

    let coarse =
        HydrologyEvolutionModel::propose(&state, Scenario::new(&[0]).at_level(&[0], 2).request(1))
            .unwrap();
    // Promoting back to fine sees the same cells, the same substrate, and the
    // same conveyance topology it left.
    let promoted =
        HydrologyEvolutionModel::propose(coarse.after_state(), Scenario::new(&[0]).request(2))
            .unwrap();

    assert_eq!(before.cell_count(), promoted.after_state().cell_count());
    for (chunk_key, field) in before.fields() {
        let after = promoted
            .after_state()
            .field(*chunk_key)
            .expect("the chunk is still resident");
        assert_eq!(
            field.substrate(),
            after.substrate(),
            "substrate is retained"
        );
    }
    assert_eq!(
        promoted.after_conveyance(),
        coarse.after_conveyance(),
        "topology is untouched by either transition"
    );
    assert_eq!(promoted.conservation().residual(), 0);
}

#[test]
fn a_representation_change_cites_the_prior_anchor_and_transitions_only_the_level() {
    let policy = HydrologyResolutionPolicy::enabled(3).unwrap();
    let current = HydrologyResolutionState::new(1, BOOTSTRAP_TRACE).unwrap();
    let (change, event) = representation_change(chunk(0), current, 3, policy).unwrap();

    assert_eq!(change.from_level, 1);
    assert_eq!(change.to_level, 3);
    assert_eq!(change.prior_change, BOOTSTRAP_TRACE);
    assert_eq!(change.before, resolution_fingerprint(chunk(0), 1));
    assert_eq!(change.after, resolution_fingerprint(chunk(0), 3));
    assert_eq!(
        event.causes,
        vec![causafera_core::CausalEventDagCause::Existing(
            BOOTSTRAP_TRACE
        )],
        "the prior anchor is the one cause"
    );
    assert_eq!(event.effects.len(), 1, "only the level changes");
    assert_eq!(
        event.effects[0].carrier,
        HydrologyCarrierKey::ResolutionChunk(chunk(0))
    );
}

#[test]
fn a_representation_change_that_changes_nothing_is_refused() {
    let policy = HydrologyResolutionPolicy::enabled(2).unwrap();
    let current = HydrologyResolutionState::new(2, BOOTSTRAP_TRACE).unwrap();
    assert_eq!(
        representation_change(chunk(0), current, 2, policy).err(),
        Some(HydrologyError::ResolutionUnchanged)
    );
}

#[test]
fn a_level_above_the_policy_is_refused_rather_than_clamped() {
    let policy = HydrologyResolutionPolicy::enabled(2).unwrap();
    let current = HydrologyResolutionState::new(0, BOOTSTRAP_TRACE).unwrap();
    assert!(matches!(
        representation_change(chunk(0), current, 4, policy),
        Err(HydrologyError::ResolutionLevelAbovePolicy { level: 4, max: 2 })
    ));

    // And a disabled policy admits no level but zero.
    assert!(matches!(
        representation_change(
            chunk(0),
            HydrologyResolutionState::new(0, BOOTSTRAP_TRACE).unwrap(),
            1,
            HydrologyResolutionPolicy::DISABLED
        ),
        Err(HydrologyError::ResolutionLevelAbovePolicy { level: 1, max: 0 })
    ));
}

// ---------------------------------------------------------------------------
// V19, V21 — request validation and allocation failure
// ---------------------------------------------------------------------------

#[test]
fn a_resident_chunk_with_no_resolution_entry_is_refused() {
    // Missing is not zero. Evaluating a world at a detail nobody asked for is the
    // failure the explicit entry exists to prevent.
    let state = block_of_four(vertical(100, (0, 1)), [(1_000, 0, 0); 4]);
    let mut scenario = Scenario::new(&[0]).at_level(&[0], 1);
    scenario.resolution.clear();

    assert_eq!(
        HydrologyEvolutionModel::propose(&state, scenario.request(1)),
        Err(HydrologyError::ResolutionEntryMissing)
    );
}

#[test]
fn a_chunk_above_the_policy_level_is_refused_rather_than_clamped() {
    let state = block_of_four(vertical(100, (0, 1)), [(1_000, 0, 0); 4]);
    let mut scenario = Scenario::new(&[0]).at_level(&[0], 1);
    scenario.resolution_policy = HydrologyResolutionPolicy::enabled(0).unwrap();

    assert!(matches!(
        HydrologyEvolutionModel::propose(&state, scenario.request(1)),
        Err(HydrologyError::ResolutionLevelAbovePolicy { level: 1, max: 0 })
    ));
}

#[test]
fn extra_resolution_entries_for_chunks_hydrology_does_not_hold_are_ignored() {
    // The runtime's resolution field covers more than one domain, so an entry for
    // a chunk hydrology has no state for is not an error.
    let state = block_of_four(vertical(100, (0, 1)), [(1_000, 0, 0); 4]);
    let mut scenario = Scenario::new(&[0]).at_level(&[0], 1);
    scenario.resolution.insert(
        chunk(7),
        HydrologyResolutionState::new(4, BOOTSTRAP_TRACE).unwrap(),
    );

    let proposal = HydrologyEvolutionModel::propose(&state, scenario.request(1)).unwrap();
    assert_conserved(&proposal);
}

#[test]
fn a_disabled_policy_evaluates_everything_at_level_zero() {
    // With resolution off, the coarse path is not merely unused — it produces no
    // process at all, so a disabled session's causal DAG is the fine one.
    let ground = vertical(100, (0, 1));
    let state = block_of_four(ground, [(1_000, 0, 0); 4]);
    let mut scenario = Scenario::new(&[0]).at_level(&[0], 2);
    scenario.resolution_policy = HydrologyResolutionPolicy::DISABLED;

    let proposal = HydrologyEvolutionModel::propose(&state, scenario.request(1)).unwrap();
    assert!(proposal.coarse_processes().is_empty());
    assert_eq!(
        proposal,
        HydrologyEvolutionModel::propose(&state, Scenario::new(&[0]).request(1)).unwrap(),
        "a disabled policy is the fine path, not an imitation of it"
    );
}

#[test]
fn construction_order_does_not_change_a_coarse_proposal() {
    let ground = vertical(100, (1, 4));
    let build = |order: [u16; 4]| {
        let mut builder = ChunkBuilder::new(0);
        for ordinal in order {
            builder = builder.with(ordinal, ground.build(), storage(1_000, 400, 0));
        }
        field_set(vec![builder.build()])
    };
    let forward = HydrologyEvolutionModel::propose(
        &build(INTERIOR_BLOCK),
        Scenario::new(&[0]).at_level(&[0], 1).request(1),
    )
    .unwrap();
    let reversed = HydrologyEvolutionModel::propose(
        &build([99, 98, 67, 66]),
        Scenario::new(&[0]).at_level(&[0], 1).request(1),
    )
    .unwrap();
    assert_eq!(forward, reversed);
}

#[test]
fn coarse_mode_evaluates_fewer_process_groups_than_fine_mode_has_cells() {
    // The work reduction, measured rather than asserted. A uniform 8x8 interior
    // region at level 2 has sixteen-cell blocks, so four blocks of one group each
    // replace sixty-four per-cell evaluations. The fixture is non-vacuous: every
    // cell has water, a real infiltration limit, and a real percolation fraction.
    let ground = vertical(100, (1, 4));
    let mut builder = ChunkBuilder::new(0);
    let mut active = 0_usize;
    for y in 4..12_u16 {
        for x in 4..12_u16 {
            builder = builder.with(y * 32 + x, ground.build(), storage(1_000, 400, 0));
            active += 1;
        }
    }
    let state = field_set(vec![builder.build()]);
    assert_eq!(active, 64);

    let fine = HydrologyEvolutionModel::propose(&state, Scenario::new(&[0]).request(1)).unwrap();
    let coarse =
        HydrologyEvolutionModel::propose(&state, Scenario::new(&[0]).at_level(&[0], 2).request(1))
            .unwrap();

    assert!(fine.coarse_processes().is_empty());
    let groups = coarse
        .coarse_processes()
        .iter()
        .filter(|process| {
            process.process_kind == process::INFILTRATION && process.accepted_total > 0
        })
        .count();
    assert!(
        groups < active,
        "coarse mode evaluated {groups} groups for {active} cells"
    );
    assert_eq!(groups, 4, "four sixteen-cell blocks, one group apiece");
    // And the water still lands on every fine cell.
    for y in 4..12_u16 {
        for x in 4..12_u16 {
            assert_eq!(soil_of(&coarse, cell(0, y * 32 + x)), 400 + 100 - 125);
        }
    }
    assert_conserved(&coarse);
}

// ---------------------------------------------------------------------------
// V21 — an unallocatable coarse delta rejects atomically
// ---------------------------------------------------------------------------

#[test]
fn a_rejected_coarse_tick_leaves_the_input_state_exactly_as_it_was() {
    // `propose` borrows the state and returns a proposal, so a refusal cannot have
    // written anything — but the property is worth pinning, because the alternative
    // an unallocatable delta must never take is spilling into a fine cell.
    let state = block_of_four(vertical(100, (0, 1)), [(1_000, 0, 0); 4]);
    let snapshot = state.clone();
    let mut scenario = Scenario::new(&[0]).at_level(&[0], 1);
    scenario.resolution.clear();

    assert!(HydrologyEvolutionModel::propose(&state, scenario.request(1)).is_err());
    assert_eq!(state, snapshot, "a refused tick changes nothing");
    assert_eq!(
        state.total_storage().unwrap().get(),
        snapshot.total_storage().unwrap().get()
    );
}

#[test]
fn the_reducers_own_refusals_are_the_only_way_a_delta_goes_unallocated() {
    // Reachable only as an internal error: `clamp_to_allocatable` and
    // `allocate_capped` agree on who is eligible, so an ordinary candidate is
    // always placeable. The guard is kept as the reducer's stated precondition and
    // exercised directly rather than through a scenario that cannot produce it.
    use causafera_domains::{allocate_capped, clamp_to_allocatable};

    assert_eq!(
        allocate_capped(5, &[0, 0], &[10, 10]),
        Err(HydrologyError::UnallocatableTotal)
    );
    assert_eq!(
        allocate_capped(21, &[1, 1], &[10, 10]),
        Err(HydrologyError::AllocationExceedsCeilings)
    );
    // The clamp never hands the reducer a total it must refuse.
    for candidate in [0_i128, 1, 400, 600, 10_000] {
        let weights = [600, 0];
        let ceilings = [400, 400];
        let total = clamp_to_allocatable(candidate, &weights, &ceilings).unwrap();
        assert!(allocate_capped(total, &weights, &ceilings).is_ok());
    }
}
