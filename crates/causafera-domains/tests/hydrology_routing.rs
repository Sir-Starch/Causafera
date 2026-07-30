//! Surface routing: head, harmonic face conductance, donor and receiver
//! reduction, frozen substages, and chunk seams.
//!
//! Covers `plans/hydrology.md` verification gates V8, V9, V10, V11, and V12, and
//! the routing half of V16.

mod support;

use std::collections::BTreeSet;

use causafera_domains::{
    HydrologyError, HydrologyEvolutionLimits, HydrologyEvolutionModel, HydrologyEvolutionProposal,
    HydrologyTransferReceipt, process,
};
use causafera_geography::{
    HydrologyActiveRegion, HydrologyCarrierKey, HydrologyCellKey, HydrologyFieldSet,
    MAX_HYDROLOGY_CAUSES_PER_EVENT, MAX_HYDROLOGY_EFFECTS_PER_EVENT,
};
use causafera_types::{TraceId, WaterVolume};

use support::{ChunkBuilder, Ground, Scenario, cell, chunk, field_set, storage, terrain_from};

/// Ground that conducts laterally and does nothing vertically, so a test's
/// assertions are about routing rather than about infiltration it did not ask for.
fn conductive(surface: u64, groundwater: u64) -> Ground {
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
        surface_conductance: surface,
        groundwater_conductance: groundwater,
    }
}

fn surface_ground(conductance: u64, capacity: u64) -> Ground {
    Ground {
        surface_capacity: capacity,
        ..conductive(conductance, 0)
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

fn lateral(
    proposal: &HydrologyEvolutionProposal,
    from: HydrologyCellKey,
    to: HydrologyCellKey,
) -> &HydrologyTransferReceipt {
    proposal
        .transfer_receipts()
        .iter()
        .find(|receipt| {
            receipt.process_kind() == process::SURFACE_LATERAL
                && receipt.source() == HydrologyCarrierKey::Cell(from)
                && receipt.target() == HydrologyCarrierKey::Cell(to)
        })
        .expect("the lateral transfer was proposed")
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
    assert_eq!(
        ledger.storage_before().unwrap() + ledger.sources().unwrap(),
        ledger.storage_after().unwrap() + ledger.sinks().unwrap()
    );
}

// ---------------------------------------------------------------------------
// V8 — surface downhill response
// ---------------------------------------------------------------------------

/// Two cells holding the same water at different elevations, one metre apart.
fn slope(high_mm: i32, low_mm: i32, water: u64) -> (HydrologyFieldSet, Scenario) {
    let ground = surface_ground(1_000, 1_000_000_000);
    let field = ChunkBuilder::new(0)
        .with(0, ground.build(), storage(water, 0, 0))
        .with(1, ground.build(), storage(water, 0, 0))
        .build();
    let scenario =
        Scenario::new(&[0]).with_terrain(terrain_from(&[0], move |_, ordinal| match ordinal {
            0 => high_mm,
            1 => low_mm,
            _ => 0,
        }));
    (field_set(vec![field]), scenario)
}

#[test]
fn water_moves_toward_the_lower_surface_head() {
    // Ten millimetres of water on both cells, a hundred millimetres of terrain
    // between them. Head is terrain plus ponded depth, so the drop is exactly the
    // terrain difference and the flux is the harmonic face conductance times it.
    let (state, scenario) = slope(100, 0, 10_000_000);
    let proposal = HydrologyEvolutionModel::propose(&state, scenario.request(1)).unwrap();

    let receipt = lateral(&proposal, cell(0, 0), cell(0, 1));
    assert_eq!(receipt.requested(), WaterVolume::new(100_000));
    assert_eq!(receipt.accepted(), WaterVolume::new(100_000));
    assert_eq!(surface_of(&proposal, cell(0, 0)), 9_900_000);
    assert_eq!(surface_of(&proposal, cell(0, 1)), 10_100_000);
    assert_conserved(&proposal);
}

#[test]
fn equal_heads_move_nothing() {
    // Equal water on equal ground. There is no gradient, so there is no transfer
    // and no receipt claiming one happened.
    let (state, scenario) = slope(0, 0, 10_000_000);
    let proposal = HydrologyEvolutionModel::propose(&state, scenario.request(1)).unwrap();

    assert!(lateral_receipts(&proposal).is_empty());
    assert_eq!(surface_of(&proposal, cell(0, 0)), 10_000_000);
    assert_eq!(surface_of(&proposal, cell(0, 1)), 10_000_000);
    assert_conserved(&proposal);
}

#[test]
fn ponded_depth_participates_in_head_not_just_terrain() {
    // Level ground, unequal water: the deeper cell has the higher surface and
    // drains into the shallower one. Head is the water's own top, not the
    // ground's.
    let ground = surface_ground(1_000, 1_000_000_000);
    let field = ChunkBuilder::new(0)
        .with(0, ground.build(), storage(20_000_000, 0, 0))
        .with(1, ground.build(), storage(5_000_000, 0, 0))
        .build();
    let state = field_set(vec![field]);
    let proposal =
        HydrologyEvolutionModel::propose(&state, Scenario::new(&[0]).request(1)).unwrap();

    // Depths are 20 mm and 5 mm, so the drop is 15 mm over a 1 000 mm² face.
    let receipt = lateral(&proposal, cell(0, 0), cell(0, 1));
    assert_eq!(receipt.accepted(), WaterVolume::new(15_000));
    assert_conserved(&proposal);
}

#[test]
fn ground_that_cannot_conduct_stops_the_face_whichever_side_it_is_on() {
    // The harmonic rule is symmetric and a zero endpoint gives zero. Ground that
    // cannot pass water does not become passable by being next to ground that can.
    for (donor_conductance, receiver_conductance) in [(1_000, 0), (0, 1_000)] {
        let field = ChunkBuilder::new(0)
            .with(
                0,
                surface_ground(donor_conductance, 1_000_000_000).build(),
                storage(10_000_000, 0, 0),
            )
            .with(
                1,
                surface_ground(receiver_conductance, 1_000_000_000).build(),
                storage(0, 0, 0),
            )
            .build();
        let state = field_set(vec![field]);
        let proposal =
            HydrologyEvolutionModel::propose(&state, Scenario::new(&[0]).request(1)).unwrap();
        assert!(lateral_receipts(&proposal).is_empty());
        assert_eq!(surface_of(&proposal, cell(0, 0)), 10_000_000);
    }
}

#[test]
fn face_conductance_is_the_harmonic_mean_of_its_two_endpoints() {
    // floor(2 * 300 * 700 / 1000) = 420, not the arithmetic mean of 500.
    let field = ChunkBuilder::new(0)
        .with(
            0,
            surface_ground(300, 1_000_000_000).build(),
            storage(1_000_000, 0, 0),
        )
        .with(
            1,
            surface_ground(700, 1_000_000_000).build(),
            storage(0, 0, 0),
        )
        .build();
    let state = field_set(vec![field]);
    let proposal =
        HydrologyEvolutionModel::propose(&state, Scenario::new(&[0]).request(1)).unwrap();

    // One millimetre of depth against a dry neighbour: 420 mm² × 1 mm.
    assert_eq!(
        lateral(&proposal, cell(0, 0), cell(0, 1)).accepted(),
        WaterVolume::new(420)
    );
}

// ---------------------------------------------------------------------------
// V9 — donor and receiver oversubscription
// ---------------------------------------------------------------------------

#[test]
fn a_donor_asked_for_more_than_it_owns_pays_out_exactly_what_it_owns() {
    // Cell 33 sits above all four of its neighbours by 1, 2, 3, and 4 mm in
    // ascending canonical edge-key order, so the raw demands are 1, 2, 3, and 4
    // against seven available units.
    let donor = conductive(1, 0);
    let neighbour = conductive(1, 0);
    let field = ChunkBuilder::new(0)
        .with(33, donor.build(), storage(7, 0, 0))
        .with(1, neighbour.build(), storage(0, 0, 0))
        .with(32, neighbour.build(), storage(0, 0, 0))
        .with(34, neighbour.build(), storage(0, 0, 0))
        .with(65, neighbour.build(), storage(0, 0, 0))
        .build();
    let state = field_set(vec![field]);
    let scenario =
        Scenario::new(&[0]).with_terrain(terrain_from(&[0], |_, ordinal| match ordinal {
            33 => 1_000,
            1 => 999,
            32 => 998,
            34 => 997,
            65 => 996,
            _ => 0,
        }));
    let proposal = HydrologyEvolutionModel::propose(&state, scenario.request(1)).unwrap();

    // floor(raw * 7 / 10) is [0, 1, 2, 2] with remainders [7, 4, 1, 8]; the two
    // spare units go to the largest remainders, descending.
    let accepted: Vec<u64> = [1_u16, 32, 34, 65]
        .into_iter()
        .map(|ordinal| {
            lateral(&proposal, cell(0, 33), cell(0, ordinal))
                .accepted()
                .get()
        })
        .collect();
    assert_eq!(accepted, vec![1, 1, 2, 3]);
    assert_eq!(
        accepted.iter().sum::<u64>(),
        7,
        "exactly what the donor owned"
    );
    assert_eq!(surface_of(&proposal, cell(0, 33)), 0);
    assert_conserved(&proposal);
}

#[test]
fn every_donor_demand_is_recorded_even_when_it_is_reduced_to_nothing() {
    // A limiter that engaged is evidence. The requested amount stays on the
    // receipt so a reduced demand cannot be mistaken for a process that had
    // nothing to do.
    let donor = conductive(1, 0);
    let field = ChunkBuilder::new(0)
        .with(33, donor.build(), storage(0, 0, 0))
        .with(1, donor.build(), storage(0, 0, 0))
        .build();
    let state = field_set(vec![field]);
    let scenario =
        Scenario::new(&[0]).with_terrain(terrain_from(&[0], |_, ordinal| match ordinal {
            33 => 500,
            _ => 0,
        }));
    let proposal = HydrologyEvolutionModel::propose(&state, scenario.request(1)).unwrap();

    let receipt = lateral(&proposal, cell(0, 33), cell(0, 1));
    assert_eq!(receipt.requested(), WaterVolume::new(500));
    assert_eq!(receipt.accepted(), WaterVolume::ZERO);
    assert_eq!(receipt.unaccepted(), WaterVolume::new(500));
}

#[test]
fn simultaneous_inflows_above_receiver_capacity_are_reduced_canonically() {
    // Two donors, four units each, into a receiver with room for five. The
    // largest-remainder rule splits the five by ascending donor key and each
    // donor keeps what was refused — it is not lost in transit.
    let donor = surface_ground(1, 1_000_000_000);
    let receiver = surface_ground(1, 5);
    let field = ChunkBuilder::new(0)
        .with(32, donor.build(), storage(4, 0, 0))
        .with(34, donor.build(), storage(4, 0, 0))
        .with(33, receiver.build(), storage(0, 0, 0))
        .build();
    let state = field_set(vec![field]);
    let scenario =
        Scenario::new(&[0]).with_terrain(terrain_from(&[0], |_, ordinal| match ordinal {
            32 | 34 => 4,
            _ => 0,
        }));
    let proposal = HydrologyEvolutionModel::propose(&state, scenario.request(1)).unwrap();

    assert_eq!(
        lateral(&proposal, cell(0, 32), cell(0, 33)).accepted(),
        WaterVolume::new(3)
    );
    assert_eq!(
        lateral(&proposal, cell(0, 34), cell(0, 33)).accepted(),
        WaterVolume::new(2)
    );
    assert_eq!(
        surface_of(&proposal, cell(0, 33)),
        5,
        "filled, never overfilled"
    );
    assert_eq!(
        surface_of(&proposal, cell(0, 32)),
        1,
        "rejected water stays home"
    );
    assert_eq!(surface_of(&proposal, cell(0, 34)), 2);
    assert_conserved(&proposal);
}

// ---------------------------------------------------------------------------
// V10 — frozen substage
// ---------------------------------------------------------------------------

#[test]
fn one_unit_cannot_cross_two_faces_in_one_routing_substage() {
    // A three-cell staircase with one unit at the top. Every demand is computed
    // from the same frozen state, so the middle cell cannot pass on water it only
    // receives during this substage.
    let ground = conductive(1, 0);
    let field = ChunkBuilder::new(0)
        .with(0, ground.build(), storage(1, 0, 0))
        .with(1, ground.build(), storage(0, 0, 0))
        .with(2, ground.build(), storage(0, 0, 0))
        .build();
    let state = field_set(vec![field]);
    let scenario =
        Scenario::new(&[0]).with_terrain(terrain_from(&[0], |_, ordinal| match ordinal {
            0 => 2,
            1 => 1,
            _ => 0,
        }));
    let proposal = HydrologyEvolutionModel::propose(&state, scenario.request(1)).unwrap();

    assert_eq!(surface_of(&proposal, cell(0, 0)), 0);
    assert_eq!(surface_of(&proposal, cell(0, 1)), 1, "the unit stops here");
    assert_eq!(surface_of(&proposal, cell(0, 2)), 0);
    // The middle cell's demand existed and was refused for want of water, which
    // is a different fact from the demand never being computed.
    let onward = lateral(&proposal, cell(0, 1), cell(0, 2));
    assert_eq!(onward.requested(), WaterVolume::new(1));
    assert_eq!(onward.accepted(), WaterVolume::ZERO);
    assert_conserved(&proposal);
}

#[test]
fn a_cell_that_passes_water_through_anchors_nothing_and_orphans_nothing() {
    // Ten units in, ten out: the middle cell's bucket is unchanged, so it has no
    // state change to anchor and emits no bucket-change event — an effect claiming
    // `before == after` is not a state change and the causal contract refuses one.
    // Its two transfers stay attributable through the other endpoint of each.
    let ground = conductive(1, 0);
    let field = ChunkBuilder::new(0)
        .with(0, ground.build(), storage(1_000_000, 0, 0))
        .with(1, ground.build(), storage(1_000_000, 0, 0))
        .with(2, ground.build(), storage(1_000_000, 0, 0))
        .build();
    let state = field_set(vec![field]);
    let scenario =
        Scenario::new(&[0]).with_terrain(terrain_from(&[0], |_, ordinal| match ordinal {
            0 => 20,
            1 => 10,
            _ => 0,
        }));
    let proposal = HydrologyEvolutionModel::propose(&state, scenario.request(1)).unwrap();

    assert!(
        !proposal
            .cell_changes()
            .iter()
            .any(|change| change.cell == cell(0, 1)),
        "an unchanged bucket has no change to record"
    );
    assert_eq!(
        surface_of(&proposal, cell(0, 1)),
        1_000_000,
        "ten in, ten out"
    );

    // Neither transfer across the middle cell is an orphan: the inbound one names
    // the donor's event and the outbound one names the receiver's.
    let inbound = lateral(&proposal, cell(0, 0), cell(0, 1));
    assert!(inbound.transfer_event().is_some(), "the donor settled");
    let outbound = lateral(&proposal, cell(0, 1), cell(0, 2));
    assert!(outbound.storage_event().is_some(), "the receiver settled");
    assert_conserved(&proposal);
}

// ---------------------------------------------------------------------------
// V11 — same-chart seam equivalence
// ---------------------------------------------------------------------------

#[test]
fn a_chunk_seam_behaves_exactly_like_an_interior_face() {
    // The same physical pair, once inside a chunk and once across a chunk
    // boundary. Chunks are addressing, so the seam is not a wall and not a
    // discount (INV-043).
    let donor = surface_ground(1_000, 1_000_000_000);
    let receiver = surface_ground(1_000, 1_000_000_000);

    let interior_field = ChunkBuilder::new(0)
        .with(30, donor.build(), storage(1_000_000, 0, 0))
        .with(31, receiver.build(), storage(0, 0, 0))
        .build();
    let interior_state = field_set(vec![interior_field]);
    let interior_scenario =
        Scenario::new(&[0]).with_terrain(terrain_from(&[0], |_, ordinal| match ordinal {
            30 => 100,
            _ => 0,
        }));
    let interior =
        HydrologyEvolutionModel::propose(&interior_state, interior_scenario.request(1)).unwrap();

    let seam_state = field_set(vec![
        ChunkBuilder::new(0)
            .with(31, donor.build(), storage(1_000_000, 0, 0))
            .build(),
        ChunkBuilder::new(1)
            .with(0, receiver.build(), storage(0, 0, 0))
            .build(),
    ]);
    let seam_scenario =
        Scenario::new(&[0, 1]).with_terrain(terrain_from(&[0, 1], |chunk_x, ordinal| {
            if chunk_x == 0 && ordinal == 31 {
                100
            } else {
                0
            }
        }));
    let seam = HydrologyEvolutionModel::propose(&seam_state, seam_scenario.request(1)).unwrap();

    let across = lateral(&interior, cell(0, 30), cell(0, 31));
    let seam_receipt = lateral(&seam, cell(0, 31), cell(1, 0));
    assert_eq!(across.requested(), seam_receipt.requested());
    assert_eq!(across.accepted(), seam_receipt.accepted());
    assert_eq!(across.accepted(), WaterVolume::new(101_000));
    assert_eq!(surface_of(&seam, cell(0, 31)), 899_000);
    assert_eq!(surface_of(&seam, cell(1, 0)), 101_000);
    assert_conserved(&seam);
}

// ---------------------------------------------------------------------------
// V12 — every face processed exactly once
// ---------------------------------------------------------------------------

#[test]
fn each_face_produces_one_receipt_whichever_side_is_visited_first() {
    let ground = conductive(1, 0);
    let build = |order: [u16; 3]| {
        let mut builder = ChunkBuilder::new(0);
        for ordinal in order {
            let water = if ordinal == 33 { 1_000 } else { 0 };
            builder = builder.with(ordinal, ground.build(), storage(water, 0, 0));
        }
        field_set(vec![builder.build()])
    };
    let scenario =
        Scenario::new(&[0]).with_terrain(terrain_from(&[0], |_, ordinal| match ordinal {
            33 => 10,
            _ => 0,
        }));

    let forward =
        HydrologyEvolutionModel::propose(&build([1, 33, 65]), scenario.request(1)).unwrap();
    let reversed =
        HydrologyEvolutionModel::propose(&build([65, 33, 1]), scenario.request(1)).unwrap();

    assert_eq!(
        lateral_receipts(&forward).len(),
        2,
        "one per canonical face"
    );
    assert_eq!(forward.transfer_receipts(), reversed.transfer_receipts());
    assert_eq!(forward.after_state(), reversed.after_state());
    assert_eq!(forward.events(), reversed.events());
    assert_eq!(forward.terminal_leaves(), reversed.terminal_leaves());
    assert_eq!(forward.conservation(), reversed.conservation());
}

#[test]
fn chunk_construction_order_does_not_change_the_proposal() {
    let donor = surface_ground(1_000, 1_000_000_000);
    let build = |reversed: bool| {
        let mut fields = vec![
            ChunkBuilder::new(0)
                .with(31, donor.build(), storage(1_000_000, 0, 0))
                .build(),
            ChunkBuilder::new(1)
                .with(0, donor.build(), storage(0, 0, 0))
                .build(),
        ];
        if reversed {
            fields.reverse();
        }
        field_set(fields)
    };
    let scenario =
        Scenario::new(&[0, 1]).with_terrain(terrain_from(&[0, 1], |chunk_x, ordinal| {
            if chunk_x == 0 && ordinal == 31 {
                100
            } else {
                0
            }
        }));

    let forward = HydrologyEvolutionModel::propose(&build(false), scenario.request(1)).unwrap();
    let backward = HydrologyEvolutionModel::propose(&build(true), scenario.request(1)).unwrap();
    assert_eq!(forward, backward);
}

// ---------------------------------------------------------------------------
// V16 — a closed basin conserves across many ticks of real routing
// ---------------------------------------------------------------------------

#[test]
fn a_sloped_closed_basin_conserves_exactly_across_a_hundred_ticks() {
    // Real internal transfers, no sources, no sinks, no open boundary. Over a
    // hundred ticks the total must not drift by a single cubic millimetre.
    let ground = surface_ground(1_000, 1_000_000_000);
    let mut builder = ChunkBuilder::new(0);
    for ordinal in 0..8_u16 {
        builder = builder.with(ordinal, ground.build(), storage(10_000_000, 0, 0));
    }
    let mut state = field_set(vec![builder.build()]);
    let scenario = Scenario::new(&[0]).with_terrain(terrain_from(&[0], |_, ordinal| {
        if ordinal < 8 {
            8_000 - i32::from(ordinal) * 1_000
        } else {
            0
        }
    }));

    let initial = state.total_storage().unwrap().get();
    let mut moved = 0_i128;
    for tick in 1..=100 {
        let proposal = HydrologyEvolutionModel::propose(&state, scenario.request(tick)).unwrap();
        let ledger = proposal.conservation();
        assert_eq!(ledger.residual(), 0, "tick {tick} must close exactly");
        assert_eq!(
            ledger.storage_before().unwrap(),
            ledger.storage_after().unwrap(),
            "a closed basin ends every tick with what it started with"
        );
        moved += proposal
            .transfer_receipts()
            .iter()
            .map(|receipt| receipt.accepted().as_i128())
            .sum::<i128>();
        state = proposal.after_state().clone();
    }
    assert_eq!(state.total_storage().unwrap().get(), initial);
    assert!(moved > 0, "the run has to have actually moved water");
}

// ---------------------------------------------------------------------------
// Preconditions and fan-in bounds
// ---------------------------------------------------------------------------

#[test]
fn a_request_whose_resident_set_disagrees_with_the_field_set_is_refused() {
    // The active region is not decoration. A solver that routed over a chunk the
    // request does not consider resident would be exchanging water across an edge
    // of the world it was told did not exist.
    let ground = surface_ground(1_000, 1_000_000_000);
    let state = field_set(vec![
        ChunkBuilder::new(0)
            .with(0, ground.build(), storage(1_000_000, 0, 0))
            .build(),
    ]);
    let mut scenario = Scenario::new(&[0]);
    let two: BTreeSet<_> = [chunk(0), chunk(1)].into_iter().collect();
    scenario.active = HydrologyActiveRegion::new(two.clone(), two).unwrap();

    assert_eq!(
        HydrologyEvolutionModel::propose(&state, scenario.request(1)),
        Err(HydrologyError::ResidencyMismatch)
    );
}

/// A cell surrounded on all four faces by conductive, unequal-elevation
/// neighbours, each anchored to a *distinct* pre-tick trace.
///
/// The distinct traces matter: causes are deduplicated, so a fixture where every
/// cell shares one bootstrap anchor collapses a five-way fan-in into one cause
/// and proves nothing about the bound.
fn crowded_cell() -> (HydrologyFieldSet, Scenario) {
    let ground = conductive(1_000, 1_000);
    let mut builder = ChunkBuilder::new(0);
    for (index, ordinal) in [1_u16, 32, 33, 34, 65].into_iter().enumerate() {
        let anchor = 100 + index as u64 * 3;
        builder = builder
            .with(ordinal, ground.build(), storage(5_000_000, 0, 2_000_000))
            .with_traces(
                ordinal,
                TraceId::new(anchor),
                TraceId::new(anchor + 1),
                TraceId::new(anchor + 2),
            );
    }
    let scenario =
        Scenario::new(&[0]).with_terrain(terrain_from(&[0], |_, ordinal| match ordinal {
            33 => 500,
            1 => 400,
            32 => 300,
            34 => 200,
            65 => 100,
            _ => 0,
        }));
    (field_set(vec![builder.build()]), scenario)
}

#[test]
fn every_event_of_a_busy_tick_stays_within_the_fan_in_bounds() {
    // The widest fan-in surface and groundwater routing can produce. The caps hold
    // with room to spare, and the assertion is against the shared constants rather
    // than against numbers copied out of the plan.
    let (state, scenario) = crowded_cell();
    let proposal = HydrologyEvolutionModel::propose(&state, scenario.request(1)).unwrap();

    let widest = proposal
        .events()
        .iter()
        .map(|event| event.causes.len())
        .max()
        .unwrap_or(0);
    assert!(
        widest >= 5,
        "the fixture has to actually crowd an event, saw {widest}"
    );

    assert!(
        !proposal.events().is_empty(),
        "the tick has to do something"
    );
    for event in proposal.events() {
        assert!(
            event.causes.len() <= MAX_HYDROLOGY_CAUSES_PER_EVENT,
            "{:?} cites {} causes",
            event.kind,
            event.causes.len()
        );
        assert!(event.effects.len() <= MAX_HYDROLOGY_EFFECTS_PER_EVENT);
        let mut unique = event.causes.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(unique.len(), event.causes.len(), "causes must be distinct");
    }
    assert_conserved(&proposal);
}

#[test]
fn an_event_past_the_cause_cap_is_refused_before_the_proposal_is_returned() {
    // The trace store would reject the batch atomically. Refusing here means a
    // proposal that cannot possibly commit is never handed back as valid.
    let (state, scenario) = crowded_cell();
    let mut request = scenario.request(1);
    request.limits = HydrologyEvolutionLimits {
        max_causes_per_event: 2,
        ..HydrologyEvolutionLimits::default()
    };

    assert!(matches!(
        HydrologyEvolutionModel::propose(&state, request),
        Err(HydrologyError::EventCauseLimitExceeded { max: 2, .. })
    ));
}

#[test]
fn an_event_past_the_effect_cap_is_refused_too() {
    let (state, scenario) = crowded_cell();
    let mut request = scenario.request(1);
    request.limits = HydrologyEvolutionLimits {
        max_effects_per_event: 0,
        ..HydrologyEvolutionLimits::default()
    };

    assert!(matches!(
        HydrologyEvolutionModel::propose(&state, request),
        Err(HydrologyError::EventEffectLimitExceeded { max: 0, .. })
    ));
}

#[test]
fn surface_material_identity_alone_changes_nothing() {
    // V31's negative: terrain carries a surface material identity, and hydrology
    // gives it no hydraulic meaning in this tranche — no permeability, no soil
    // class, no conductance. Two worlds identical but for that identity produce
    // the same proposal, byte for byte, which is what keeps a material name from
    // becoming authoritative simulation meaning through the back door.
    let ground = surface_ground(1_000, 1_000_000_000);
    let field = ChunkBuilder::new(0)
        .with(0, ground.build(), storage(10_000_000, 0, 0))
        .with(1, ground.build(), storage(0, 0, 0))
        .build();
    let state = field_set(vec![field]);

    let proposal_of = |material: u64| {
        let scenario = Scenario::new(&[0]).with_terrain(support::terrain_of_material(
            &[0],
            |_, ordinal| match ordinal {
                0 => 100,
                1 => 0,
                _ => 0,
            },
            causafera_types::MaterialId::new(material),
        ));
        HydrologyEvolutionModel::propose(&state, scenario.request(1)).unwrap()
    };

    let one = proposal_of(1);
    let other = proposal_of(7_919);
    assert_ne!(
        one.transfer_receipts(),
        [] as [causafera_domains::HydrologyTransferReceipt; 0],
        "the fixture must actually move water, or this proves nothing"
    );
    assert_eq!(one, other);
}
