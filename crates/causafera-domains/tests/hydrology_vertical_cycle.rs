//! The local vertical cycle: forcing acceptance, infiltration, percolation, and
//! evapotranspiration over one frozen state (`plans/hydrology.md` §5.1–§5.4,
//! verification gates V2, V4–V7).
//!
//! Every quantity asserted here is computed from the plan's stated equation
//! rather than read back from the solver, so an implementation that changed its
//! mind about an equation fails rather than re-baselines.

mod support;

use causafera_domains::{
    HydrologyBucket, HydrologyError, HydrologyEventKind, HydrologyEvolutionModel,
    HydrologyTransferReceipt, allocate_largest_remainder, process, substage,
};
use causafera_geography::{HydrologyCarrierKey, HydrologyCellKey};
use causafera_types::{TraceId, WaterVolume};
use support::*;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn receipts_for(
    receipts: &[HydrologyTransferReceipt],
    kind: u32,
) -> Vec<&HydrologyTransferReceipt> {
    receipts
        .iter()
        .filter(|receipt| receipt.process_kind() == kind)
        .collect()
}

fn receipt_at(
    receipts: &[HydrologyTransferReceipt],
    kind: u32,
    cell: HydrologyCellKey,
) -> &HydrologyTransferReceipt {
    let carrier = HydrologyCarrierKey::Cell(cell);
    receipts
        .iter()
        .find(|receipt| {
            receipt.process_kind() == kind
                && (receipt.source() == carrier || receipt.target() == carrier)
        })
        .expect("the expected receipt must exist")
}

// ---------------------------------------------------------------------------
// Allocation
// ---------------------------------------------------------------------------

#[test]
fn weighted_allocation_sums_to_the_record_total_exactly() {
    // Given: a total that does not divide evenly by any of its weights.
    // When/Then: the parts still sum to the whole. Rounding each share
    // independently would leave a shortfall that has to go somewhere.
    for total in [0_u128, 1, 7, 100, 1_000_000_007] {
        for weights in [
            vec![1_u128],
            vec![1, 1, 1],
            vec![1, 2, 3],
            vec![7, 11, 13, 17],
            vec![1, 1_000_000],
        ] {
            let shares = allocate_largest_remainder(total, &weights).unwrap();
            assert_eq!(
                shares.iter().sum::<u128>(),
                total,
                "total {total} over weights {weights:?}"
            );
            assert_eq!(shares.len(), weights.len());
        }
    }
}

#[test]
fn allocation_breaks_equal_remainders_by_ascending_key() {
    // Three equal weights over one unit: every remainder ties, so the tie-break
    // decides. Ascending key order means the first member takes it, every time.
    assert_eq!(
        allocate_largest_remainder(1, &[1, 1, 1]).unwrap(),
        vec![1, 0, 0]
    );
    assert_eq!(
        allocate_largest_remainder(2, &[1, 1, 1]).unwrap(),
        vec![1, 1, 0]
    );
    // Unequal remainders dominate the tie-break: 10 over (1, 2) gives bases
    // 3 and 6 with remainders 1 and 2, so the larger remainder takes the unit.
    assert_eq!(allocate_largest_remainder(10, &[1, 2]).unwrap(), vec![3, 7]);
}

#[test]
fn allocation_refuses_a_positive_total_with_nothing_to_allocate_it_to() {
    assert_eq!(
        allocate_largest_remainder(1, &[]),
        Err(HydrologyError::UnallocatableTotal)
    );
    assert_eq!(
        allocate_largest_remainder(1, &[0, 0]),
        Err(HydrologyError::UnallocatableTotal)
    );
    assert_eq!(
        allocate_largest_remainder(0, &[]).unwrap(),
        Vec::<u128>::new()
    );
    assert_eq!(allocate_largest_remainder(0, &[0]).unwrap(), vec![0]);
}

// ---------------------------------------------------------------------------
// V2 — precipitation source and ancestry
// ---------------------------------------------------------------------------

#[test]
fn accepted_precipitation_reaches_surface_storage_and_cites_its_origin() {
    // Given: one cell of dry ground with nothing else configured.
    let ground = Ground {
        infiltration_limit: 0,
        percolation: (0, 1),
        ..Ground::default()
    };
    let state = field_set(vec![
        ChunkBuilder::new(0)
            .with(0, ground.build(), storage(0, 0, 0))
            .build(),
    ]);
    let scenario = Scenario::new(&[0]).with_forcing(vec![
        Forcing::new(1, 5)
            .target(cell(0, 0), 1)
            .precipitation(600)
            .build(),
    ]);

    // When: the tick runs.
    let proposal = HydrologyEvolutionModel::propose(&state, scenario.request(5)).unwrap();

    // Then: the water is in surface storage, counted once as a source, and the
    // receipt names the origin that authorised it.
    assert_eq!(
        proposal
            .after_state()
            .cell(cell(0, 0))
            .unwrap()
            .surface_water(),
        WaterVolume::new(600)
    );
    assert_eq!(proposal.conservation().accepted_precipitation(), 600);
    assert_eq!(proposal.conservation().residual(), 0);

    let receipt = receipt_at(
        proposal.transfer_receipts(),
        process::PRECIPITATION,
        cell(0, 0),
    );
    assert_eq!(receipt.requested(), WaterVolume::new(600));
    assert_eq!(receipt.accepted(), WaterVolume::new(600));
    assert_eq!(receipt.unaccepted(), WaterVolume::ZERO);
    assert_eq!(receipt.forcing_origin(), Some(ORIGIN_TRACE));
    assert_eq!(receipt.causal_parents(), &[ORIGIN_TRACE]);
    assert_eq!(receipt.source_bucket(), HydrologyBucket::External);
    assert_eq!(receipt.target_bucket(), HydrologyBucket::Surface);
    assert_eq!(
        receipt.source(),
        HydrologyCarrierKey::ForcingRecord {
            scheduled_tick: 5,
            forcing_id: 1
        },
        "the source is the record that delivered it, not an anonymous outside"
    );

    // And: the record is marked applied exactly once, with an application event
    // citing its one origin.
    assert_eq!(proposal.applied_forcing(), &[(5, 1)]);
    let application = proposal
        .events()
        .iter()
        .find(|event| event.kind == HydrologyEventKind::ForcingApplication)
        .expect("an applied record emits an application event");
    assert_eq!(application.causes.len(), 1);
    assert_eq!(application.effects.len(), 1);
}

#[test]
fn a_records_total_is_split_across_its_members_by_weight() {
    let ground = Ground {
        infiltration_limit: 0,
        percolation: (0, 1),
        ..Ground::default()
    };
    let state = field_set(vec![
        ChunkBuilder::new(0)
            .with(0, ground.build(), storage(0, 0, 0))
            .with(1, ground.build(), storage(0, 0, 0))
            .with(2, ground.build(), storage(0, 0, 0))
            .build(),
    ]);
    // 100 over weights 1, 2, 3: bases floor(100/6)=16, floor(200/6)=33 and
    // floor(300/6)=50 sum to 99. The remainders are 100 mod 6 = 4, 200 mod 6 = 2
    // and 0, so the single leftover unit goes to member 0.
    let scenario = Scenario::new(&[0]).with_forcing(vec![
        Forcing::new(1, 5)
            .target(cell(0, 0), 1)
            .target(cell(0, 1), 2)
            .target(cell(0, 2), 3)
            .precipitation(100)
            .build(),
    ]);

    let proposal = HydrologyEvolutionModel::propose(&state, scenario.request(5)).unwrap();
    let surface = |ordinal| {
        proposal
            .after_state()
            .cell(cell(0, ordinal))
            .unwrap()
            .surface_water()
            .get()
    };
    assert_eq!([surface(0), surface(1), surface(2)], [17, 33, 50]);
    assert_eq!(surface(0) + surface(1) + surface(2), 100);
    assert_eq!(proposal.conservation().accepted_precipitation(), 100);
}

#[test]
fn overlapping_records_reduce_in_canonical_schedule_order() {
    // Given: a cell whose surface can hold 150, and two records that together
    // ask for 200 — the second one must be the one that is refused.
    let ground = Ground {
        surface_capacity: 150,
        infiltration_limit: 0,
        percolation: (0, 1),
        ..Ground::default()
    };
    let state = field_set(vec![
        ChunkBuilder::new(0)
            .with(0, ground.build(), storage(0, 0, 0))
            .build(),
    ]);
    let scenario = Scenario::new(&[0]).with_forcing(vec![
        Forcing::new(1, 5)
            .target(cell(0, 0), 1)
            .precipitation(100)
            .build(),
        Forcing::new(2, 5)
            .target(cell(0, 0), 1)
            .precipitation(100)
            .build(),
    ]);

    let proposal = HydrologyEvolutionModel::propose(&state, scenario.request(5)).unwrap();

    // Then: the cell is exactly full, and the shortfall sits on the later
    // record. Capacity was not clamped away silently — it is on a receipt.
    assert_eq!(
        proposal
            .after_state()
            .cell(cell(0, 0))
            .unwrap()
            .surface_water(),
        WaterVolume::new(150)
    );
    assert_eq!(proposal.conservation().accepted_precipitation(), 150);
    let precipitation = receipts_for(proposal.transfer_receipts(), process::PRECIPITATION);
    assert_eq!(precipitation.len(), 2);
    assert_eq!(precipitation[0].accepted(), WaterVolume::new(100));
    assert_eq!(precipitation[0].unaccepted(), WaterVolume::ZERO);
    assert_eq!(precipitation[1].accepted(), WaterVolume::new(50));
    assert_eq!(precipitation[1].unaccepted(), WaterVolume::new(50));

    // And: the settlement records both records' shares, so the rejection stays
    // attributable to the record that caused it.
    let settlement = &proposal.forcing_settlements()[0];
    assert_eq!(settlement.accepted_source, WaterVolume::new(150));
    assert_eq!(settlement.rejected_source, WaterVolume::new(50));
    assert_eq!(settlement.allocations.len(), 2);
    assert_eq!(settlement.allocations[0].forcing_id, 1);
    assert_eq!(settlement.allocations[1].forcing_id, 2);
}

#[test]
fn a_non_resident_target_rolls_back_the_whole_proposal() {
    // Given: a schedule whose second record targets a chunk that is not resident.
    let state = field_set(vec![
        ChunkBuilder::new(0)
            .with(0, Ground::default().build(), storage(0, 0, 0))
            .build(),
    ]);
    let scenario = Scenario::new(&[0]).with_forcing(vec![
        Forcing::new(1, 5)
            .target(cell(0, 0), 1)
            .precipitation(100)
            .build(),
        Forcing::new(2, 5)
            .target(cell(7, 0), 1)
            .precipitation(100)
            .build(),
    ]);

    // Then: nothing is applied at all. Partial application would make the same
    // inputs land differently depending on validation order.
    assert_eq!(
        HydrologyEvolutionModel::propose(&state, scenario.request(5)),
        Err(HydrologyError::ForcingTargetNotResident)
    );
}

#[test]
fn a_record_reaching_the_wrong_tick_is_refused() {
    let state = field_set(vec![
        ChunkBuilder::new(0)
            .with(0, Ground::default().build(), storage(0, 0, 0))
            .build(),
    ]);
    let scenario =
        Scenario::new(&[0]).with_forcing(vec![Forcing::new(1, 5).target(cell(0, 0), 1).build()]);
    assert_eq!(
        HydrologyEvolutionModel::propose(&state, scenario.request(6)),
        Err(HydrologyError::ForcingTickMismatch {
            scheduled: 5,
            tick: 6
        })
    );
}

#[test]
fn a_targeted_cell_records_its_forcing_even_when_nothing_fits() {
    // Given: a cell with no surface capacity at all and rain scheduled onto it.
    let ground = Ground {
        surface_capacity: 0,
        soil_capacity: 0,
        groundwater_capacity: 0,
        infiltration_limit: 0,
        percolation: (0, 1),
        specific_yield: (0, 1),
        ..Ground::default()
    };
    let state = field_set(vec![
        ChunkBuilder::new(0)
            .with(0, ground.build(), storage(0, 0, 0))
            .build(),
    ]);
    let scenario = Scenario::new(&[0]).with_forcing(vec![
        Forcing::new(1, 5)
            .target(cell(0, 0), 1)
            .precipitation(500)
            .build(),
    ]);

    let proposal = HydrologyEvolutionModel::propose(&state, scenario.request(5)).unwrap();

    // Then: no water moved, and the cell still carries a durable record that it
    // was rained on. "Nothing fit" and "nothing was scheduled" are different
    // facts and must stay distinguishable after receipts are evicted.
    assert_eq!(proposal.conservation().accepted_precipitation(), 0);
    assert_eq!(proposal.conservation().residual(), 0);
    let settlement = &proposal.forcing_settlements()[0];
    assert_eq!(settlement.accepted_source, WaterVolume::ZERO);
    assert_eq!(settlement.rejected_source, WaterVolume::new(500));
    assert_ne!(
        settlement.fingerprint_before, settlement.fingerprint_after,
        "the forcing-input fingerprint moves even with nothing accepted"
    );
    assert_eq!(
        proposal
            .after_state()
            .cell(cell(0, 0))
            .unwrap()
            .forcing_input_fingerprint(),
        settlement.fingerprint_after
    );
    assert!(
        proposal
            .events()
            .iter()
            .any(|event| event.kind == HydrologyEventKind::ForcingSettlement)
    );
}

// ---------------------------------------------------------------------------
// V4/V5 — infiltration bounds and the saturated-soil counterfactual
// ---------------------------------------------------------------------------

#[test]
fn infiltration_never_exceeds_availability_its_limit_or_the_receiving_room() {
    // Three cells, each bounded by a different one of the three terms in
    // `min(surface, infiltration_limit, soil_capacity - soil)`.
    let by_surface = Ground {
        infiltration_limit: 1_000,
        soil_capacity: 1_000,
        percolation: (0, 1),
        ..Ground::default()
    };
    let by_limit = Ground {
        infiltration_limit: 40,
        soil_capacity: 1_000,
        percolation: (0, 1),
        ..Ground::default()
    };
    let by_room = Ground {
        infiltration_limit: 1_000,
        soil_capacity: 300,
        percolation: (0, 1),
        ..Ground::default()
    };
    let state = field_set(vec![
        ChunkBuilder::new(0)
            .with(0, by_surface.build(), storage(30, 0, 0))
            .with(1, by_limit.build(), storage(500, 0, 0))
            .with(2, by_room.build(), storage(500, 280, 0))
            .build(),
    ]);
    let scenario = Scenario::new(&[0]);

    let proposal = HydrologyEvolutionModel::propose(&state, scenario.request(5)).unwrap();
    let soil = |ordinal| {
        proposal
            .after_state()
            .cell(cell(0, ordinal))
            .unwrap()
            .soil_water()
            .get()
    };
    assert_eq!(soil(0), 30, "bounded by available surface water");
    assert_eq!(soil(1), 40, "bounded by the per-tick limit");
    assert_eq!(soil(2), 300, "bounded by remaining soil capacity");

    // And the receiving-room limiter is visible on its receipt rather than
    // vanishing as a clamp.
    let limited = receipt_at(
        proposal.transfer_receipts(),
        process::INFILTRATION,
        cell(0, 2),
    );
    assert_eq!(limited.requested(), WaterVolume::new(500));
    assert_eq!(limited.accepted(), WaterVolume::new(20));
    assert_eq!(limited.unaccepted(), WaterVolume::new(480));
}

#[test]
fn saturated_soil_infiltrates_less_and_retains_more_surface_water() {
    // Given: identical forcing and identical ground, differing only in how much
    // water the soil already holds.
    let ground = Ground {
        soil_capacity: 1_000,
        infiltration_limit: 500,
        percolation: (0, 1),
        ..Ground::default()
    };
    let run = |initial_soil: u64| {
        let state = field_set(vec![
            ChunkBuilder::new(0)
                .with(0, ground.build(), storage(0, initial_soil, 0))
                .build(),
        ]);
        let scenario = Scenario::new(&[0]).with_forcing(vec![
            Forcing::new(1, 5)
                .target(cell(0, 0), 1)
                .precipitation(400)
                .build(),
        ]);
        let proposal = HydrologyEvolutionModel::propose(&state, scenario.request(5)).unwrap();
        let after = *proposal.after_state().cell(cell(0, 0)).unwrap();
        (after.surface_water().get(), after.soil_water().get())
    };

    let (dry_surface, dry_soil) = run(0);
    let (wet_surface, wet_soil) = run(900);

    assert_eq!((dry_surface, dry_soil), (0, 400), "dry soil takes it all");
    assert_eq!(
        (wet_surface, wet_soil),
        (300, 1_000),
        "saturated soil takes only its remaining 100"
    );
    assert!(
        wet_surface > dry_surface,
        "saturated soil must leave more water on the surface"
    );
    assert!(
        wet_soil - 900 < dry_soil,
        "saturated soil must infiltrate strictly less"
    );
}

#[test]
fn zero_infiltration_capability_moves_nothing_and_emits_no_event() {
    let ground = Ground {
        infiltration_limit: 0,
        percolation: (0, 1),
        ..Ground::default()
    };
    let state = field_set(vec![
        ChunkBuilder::new(0)
            .with(0, ground.build(), storage(500, 0, 0))
            .build(),
    ]);
    let proposal =
        HydrologyEvolutionModel::propose(&state, Scenario::new(&[0]).request(5)).unwrap();

    assert_eq!(
        proposal.after_state().cell(cell(0, 0)).unwrap().storage(),
        storage(500, 0, 0)
    );
    assert!(receipts_for(proposal.transfer_receipts(), process::INFILTRATION).is_empty());
    assert!(proposal.events().is_empty());
    assert_eq!(proposal.conservation().residual(), 0);
}

// ---------------------------------------------------------------------------
// V6 — percolation and groundwater capacity
// ---------------------------------------------------------------------------

#[test]
fn percolation_follows_its_exact_fraction_and_floors() {
    // 999 at one quarter floors to 249; the remaining 750 stays in soil, and the
    // dropped quarter-unit stays there too rather than becoming a sink.
    let ground = Ground {
        infiltration_limit: 0,
        percolation: (1, 4),
        ..Ground::default()
    };
    let state = field_set(vec![
        ChunkBuilder::new(0)
            .with(0, ground.build(), storage(0, 999, 0))
            .build(),
    ]);
    let proposal =
        HydrologyEvolutionModel::propose(&state, Scenario::new(&[0]).request(5)).unwrap();

    let after = *proposal.after_state().cell(cell(0, 0)).unwrap();
    assert_eq!(after.groundwater(), WaterVolume::new(249));
    assert_eq!(after.soil_water(), WaterVolume::new(750));
    assert_eq!(
        after.soil_water().get() + after.groundwater().get(),
        999,
        "the floored quarter-unit stays in soil"
    );
    assert_eq!(proposal.conservation().residual(), 0);
}

#[test]
fn percolation_into_full_groundwater_leaves_the_water_in_soil() {
    let ground = Ground {
        infiltration_limit: 0,
        percolation: (1, 2),
        groundwater_capacity: 100,
        ..Ground::default()
    };
    let state = field_set(vec![
        ChunkBuilder::new(0)
            .with(0, ground.build(), storage(0, 800, 100))
            .build(),
    ]);
    let proposal =
        HydrologyEvolutionModel::propose(&state, Scenario::new(&[0]).request(5)).unwrap();

    let after = *proposal.after_state().cell(cell(0, 0)).unwrap();
    assert_eq!(after.soil_water(), WaterVolume::new(800), "nothing moved");
    assert_eq!(after.groundwater(), WaterVolume::new(100));
    assert_eq!(proposal.conservation().residual(), 0);

    // The limiter is on the receipt: 400 wanted, 0 accepted.
    let receipt = receipt_at(
        proposal.transfer_receipts(),
        process::PERCOLATION,
        cell(0, 0),
    );
    assert_eq!(receipt.requested(), WaterVolume::new(400));
    assert_eq!(receipt.accepted(), WaterVolume::ZERO);
    assert_eq!(receipt.unaccepted(), WaterVolume::new(400));
    // No accepted transfer, so no bucket-change event.
    assert!(
        !proposal
            .events()
            .iter()
            .any(|event| event.key.process_kind() == process::PERCOLATION)
    );
}

#[test]
fn a_zero_percolation_fraction_moves_nothing() {
    let ground = Ground {
        infiltration_limit: 0,
        percolation: (0, 1),
        ..Ground::default()
    };
    let state = field_set(vec![
        ChunkBuilder::new(0)
            .with(0, ground.build(), storage(0, 800, 0))
            .build(),
    ]);
    let proposal =
        HydrologyEvolutionModel::propose(&state, Scenario::new(&[0]).request(5)).unwrap();
    assert_eq!(
        proposal.after_state().cell(cell(0, 0)).unwrap().storage(),
        storage(0, 800, 0)
    );
    assert!(receipts_for(proposal.transfer_receipts(), process::PERCOLATION).is_empty());
}

// ---------------------------------------------------------------------------
// V7 — evapotranspiration
// ---------------------------------------------------------------------------

#[test]
fn evapotranspiration_draws_surface_first_then_soil_and_never_groundwater() {
    // Given: 40 on the surface, 100 in soil, 500 in groundwater, and a demand of
    // 90 — enough to exhaust the surface and reach into soil, never deeper.
    let ground = Ground {
        infiltration_limit: 0,
        percolation: (0, 1),
        ..Ground::default()
    };
    let state = field_set(vec![
        ChunkBuilder::new(0)
            .with(0, ground.build(), storage(40, 100, 500))
            .build(),
    ]);
    let scenario = Scenario::new(&[0]).with_forcing(vec![
        Forcing::new(1, 5)
            .target(cell(0, 0), 1)
            .potential_et(90)
            .build(),
    ]);

    let proposal = HydrologyEvolutionModel::propose(&state, scenario.request(5)).unwrap();

    let after = *proposal.after_state().cell(cell(0, 0)).unwrap();
    assert_eq!(after.surface_water(), WaterVolume::ZERO);
    assert_eq!(after.soil_water(), WaterVolume::new(50));
    assert_eq!(
        after.groundwater(),
        WaterVolume::new(500),
        "groundwater is never withdrawn directly in this tranche"
    );
    assert_eq!(proposal.conservation().accepted_evapotranspiration(), 90);
    assert_eq!(proposal.conservation().residual(), 0);

    let surface = receipt_at(
        proposal.transfer_receipts(),
        process::EVAPOTRANSPIRATION_SURFACE,
        cell(0, 0),
    );
    assert_eq!(surface.requested(), WaterVolume::new(90));
    assert_eq!(surface.accepted(), WaterVolume::new(40));
    let soil = receipt_at(
        proposal.transfer_receipts(),
        process::EVAPOTRANSPIRATION_SOIL,
        cell(0, 0),
    );
    assert_eq!(
        soil.requested(),
        WaterVolume::new(50),
        "soil is asked only for what the surface could not meet"
    );
    assert_eq!(soil.accepted(), WaterVolume::new(50));
    assert_eq!(soil.target_bucket(), HydrologyBucket::External);
}

#[test]
fn unmet_evapotranspiration_demand_is_recorded_not_treated_as_loss() {
    // Given: a demand of 500 against 30 of available water in total.
    let ground = Ground {
        infiltration_limit: 0,
        percolation: (0, 1),
        ..Ground::default()
    };
    let state = field_set(vec![
        ChunkBuilder::new(0)
            .with(0, ground.build(), storage(10, 20, 0))
            .build(),
    ]);
    let scenario = Scenario::new(&[0]).with_forcing(vec![
        Forcing::new(1, 5)
            .target(cell(0, 0), 1)
            .potential_et(500)
            .build(),
    ]);

    let proposal = HydrologyEvolutionModel::propose(&state, scenario.request(5)).unwrap();

    // Then: exactly 30 left, 470 recorded as unmet, and the ledger closes on the
    // 30 rather than on the demand.
    assert_eq!(proposal.conservation().accepted_evapotranspiration(), 30);
    assert_eq!(proposal.conservation().residual(), 0);
    let settlement = &proposal.forcing_settlements()[0];
    assert_eq!(settlement.accepted_et, WaterVolume::new(30));
    assert_eq!(settlement.unmet_et, WaterVolume::new(470));
    assert_eq!(
        proposal.after_state().cell(cell(0, 0)).unwrap().storage(),
        storage(0, 0, 0)
    );
}

#[test]
fn a_zero_demand_cell_emits_no_evapotranspiration_receipt_or_event() {
    let ground = Ground {
        infiltration_limit: 0,
        percolation: (0, 1),
        ..Ground::default()
    };
    let state = field_set(vec![
        ChunkBuilder::new(0)
            .with(0, ground.build(), storage(100, 100, 0))
            .build(),
    ]);
    let scenario = Scenario::new(&[0]).with_forcing(vec![
        Forcing::new(1, 5)
            .target(cell(0, 0), 1)
            .precipitation(0)
            .build(),
    ]);

    let proposal = HydrologyEvolutionModel::propose(&state, scenario.request(5)).unwrap();
    assert!(
        receipts_for(
            proposal.transfer_receipts(),
            process::EVAPOTRANSPIRATION_SURFACE
        )
        .is_empty()
    );
    assert!(
        receipts_for(
            proposal.transfer_receipts(),
            process::EVAPOTRANSPIRATION_SOIL
        )
        .is_empty()
    );
    assert_eq!(proposal.conservation().accepted_evapotranspiration(), 0);
    // The settlement event still exists: the record targeted this cell.
    assert_eq!(proposal.forcing_settlements().len(), 1);
}

#[test]
fn met_evapotranspiration_is_attributed_back_to_the_records_that_demanded_it() {
    // Two records demanding 100 and 300 against 200 of available water: each
    // gets its proportional share of what was actually met, so a receipt stays
    // traceable to an origin without every origin becoming a cause.
    let ground = Ground {
        infiltration_limit: 0,
        percolation: (0, 1),
        ..Ground::default()
    };
    let state = field_set(vec![
        ChunkBuilder::new(0)
            .with(0, ground.build(), storage(200, 0, 0))
            .build(),
    ]);
    let scenario = Scenario::new(&[0]).with_forcing(vec![
        Forcing::new(1, 5)
            .target(cell(0, 0), 1)
            .potential_et(100)
            .origin(TraceId::new(10))
            .build(),
        Forcing::new(2, 5)
            .target(cell(0, 0), 1)
            .potential_et(300)
            .origin(TraceId::new(11))
            .build(),
    ]);

    let proposal = HydrologyEvolutionModel::propose(&state, scenario.request(5)).unwrap();
    let settlement = &proposal.forcing_settlements()[0];
    assert_eq!(settlement.accepted_et, WaterVolume::new(200));
    assert_eq!(settlement.unmet_et, WaterVolume::new(200));
    assert_eq!(settlement.allocations[0].accepted_et, WaterVolume::new(50));
    assert_eq!(settlement.allocations[1].accepted_et, WaterVolume::new(150));
    assert_eq!(
        settlement.allocations[0].accepted_et.get() + settlement.allocations[1].accepted_et.get(),
        200
    );
    assert_eq!(settlement.allocations[0].origin, TraceId::new(10));
    assert_eq!(settlement.allocations[1].origin, TraceId::new(11));
}

// ---------------------------------------------------------------------------
// Same-tick ancestry
// ---------------------------------------------------------------------------

#[test]
fn each_substage_cites_the_event_that_produced_the_water_it_consumed() {
    // Given: a cell with distinct prior traces per bucket, so which trace each
    // event cited is observable rather than inferred.
    let ground = Ground {
        infiltration_limit: 1_000,
        percolation: (1, 2),
        ..Ground::default()
    };
    let state = field_set(vec![
        ChunkBuilder::new(0)
            .with(0, ground.build(), storage(0, 0, 0))
            .with_traces(0, TraceId::new(11), TraceId::new(12), TraceId::new(13))
            .build(),
    ]);
    let scenario = Scenario::new(&[0]).with_forcing(vec![
        Forcing::new(1, 5)
            .target(cell(0, 0), 1)
            .precipitation(400)
            .origin(TraceId::new(7))
            .build(),
    ]);

    let proposal = HydrologyEvolutionModel::propose(&state, scenario.request(5)).unwrap();

    let by_process = |substage_ordinal: u8, kind: u32| {
        proposal
            .events()
            .iter()
            .find(|event| {
                event.key.substage_ordinal() == substage_ordinal && event.key.process_kind() == kind
            })
            .expect("the expected event must exist")
    };

    // The settlement cites the cell's own prior surface and soil traces plus the
    // record's origin — and nothing else.
    let settlement = by_process(substage::FORCING, process::FORCING_SETTLEMENT);
    assert_ne!(process::FORCING_SETTLEMENT, process::FORCING_APPLICATION);
    assert_eq!(settlement.kind, HydrologyEventKind::ForcingSettlement);
    assert_eq!(settlement.causes.len(), 3);

    // Infiltration cites the settlement (a sibling in this same batch) rather
    // than the surface bucket's pre-tick trace, because the settlement is what
    // put the water there.
    let infiltration = by_process(substage::INFILTRATION, process::INFILTRATION);
    assert!(
        infiltration.causes.iter().any(|cause| matches!(
            cause,
            causafera_core::CausalEventDagCause::Local(key) if *key == settlement.key
        )),
        "infiltration must cite the forcing settlement it consumed"
    );
    assert!(
        infiltration.causes.iter().any(|cause| matches!(
            cause,
            causafera_core::CausalEventDagCause::Existing(trace) if *trace == TraceId::new(12)
        )),
        "and the soil bucket's own prior trace"
    );

    // Percolation cites the infiltration event, which is what filled the soil.
    let percolation = by_process(substage::PERCOLATION, process::PERCOLATION);
    assert!(
        percolation.causes.iter().any(|cause| matches!(
            cause,
            causafera_core::CausalEventDagCause::Local(key) if *key == infiltration.key
        )),
        "percolation must cite the infiltration that filled the soil"
    );
    assert!(
        percolation.causes.iter().any(|cause| matches!(
            cause,
            causafera_core::CausalEventDagCause::Existing(trace) if *trace == TraceId::new(13)
        )),
        "and the groundwater bucket's own prior trace"
    );

    // Every event's causes are strictly ordered and deduplicated.
    for event in proposal.events() {
        let mut sorted = event.causes.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(event.causes, sorted, "causes must be canonical and unique");
        assert!(!event.effects.is_empty());
    }
}

#[test]
fn terminal_leaves_name_the_last_event_to_touch_each_bucket_in_canonical_order() {
    // Given: a cell where every bucket moves and then the surface moves again,
    // so "terminal" is a different event from "first".
    let ground = Ground {
        infiltration_limit: 1_000,
        percolation: (1, 2),
        ..Ground::default()
    };
    let state = field_set(vec![
        ChunkBuilder::new(0)
            .with(0, ground.build(), storage(0, 0, 0))
            .build(),
    ]);
    let scenario = Scenario::new(&[0]).with_forcing(vec![
        Forcing::new(1, 5)
            .target(cell(0, 0), 1)
            .precipitation(400)
            .potential_et(50)
            .build(),
    ]);

    let proposal = HydrologyEvolutionModel::propose(&state, scenario.request(5)).unwrap();

    let mut sorted = proposal.terminal_leaves().to_vec();
    sorted.sort();
    assert_eq!(
        proposal.terminal_leaves(),
        sorted.as_slice(),
        "leaves must arrive in canonical (carrier, bucket, key) order"
    );

    // Surface, soil, groundwater, and the forcing input all have a terminal
    // anchor; nothing else does, because nothing else changed.
    let tags: Vec<u8> = proposal
        .terminal_leaves()
        .iter()
        .map(|leaf| leaf.bucket_tag)
        .collect();
    assert_eq!(
        tags,
        vec![
            HydrologyBucket::Surface.tag(),
            HydrologyBucket::Soil.tag(),
            HydrologyBucket::Groundwater.tag(),
            HydrologyBucket::ForcingInput.tag(),
        ]
    );

    // Each bucket's anchor is the last event that actually moved it, which is a
    // different event per bucket: infiltration emptied the surface, percolation
    // was the last to touch groundwater, and ET was the last to touch soil.
    let terminal = |tag: u8| {
        proposal
            .terminal_leaves()
            .iter()
            .find(|leaf| leaf.bucket_tag == tag)
            .expect("bucket must have a terminal anchor")
            .event
            .substage_ordinal()
    };
    assert_eq!(
        terminal(HydrologyBucket::Surface.tag()),
        substage::INFILTRATION
    );
    assert_eq!(
        terminal(HydrologyBucket::Soil.tag()),
        substage::EVAPOTRANSPIRATION
    );
    assert_eq!(
        terminal(HydrologyBucket::Groundwater.tag()),
        substage::PERCOLATION
    );
    assert_eq!(
        terminal(HydrologyBucket::ForcingInput.tag()),
        substage::FORCING
    );
}

#[test]
fn a_cell_that_is_only_rained_on_still_anchors_its_surface_change() {
    // Given: ground that cannot infiltrate, percolate, or evaporate anything, so
    // the tick's only state change is the rain landing.
    let ground = Ground {
        infiltration_limit: 0,
        percolation: (0, 1),
        ..Ground::default()
    };
    let state = field_set(vec![
        ChunkBuilder::new(0)
            .with(0, ground.build(), storage(0, 0, 0))
            .build(),
    ]);
    let scenario = Scenario::new(&[0]).with_forcing(vec![
        Forcing::new(1, 5)
            .target(cell(0, 0), 1)
            .precipitation(600)
            .build(),
    ]);

    let proposal = HydrologyEvolutionModel::propose(&state, scenario.request(5)).unwrap();

    // Then: the surface bucket has a terminal anchor. The transfer receipt also
    // records this, but receipts are evicted after eight batches and the anchor
    // is what has to outlive them — a bucket that gained water while its anchor
    // still pointed at a previous tick would be a provenance hole (INV-014).
    let settlement = &proposal.forcing_settlements()[0];
    let surface_leaf = proposal
        .terminal_leaves()
        .iter()
        .find(|leaf| leaf.bucket_tag == HydrologyBucket::Surface.tag())
        .expect("a surface change must be anchored");
    assert_eq!(surface_leaf.event, settlement.settlement_event);

    // And the settlement event genuinely accounts for the transition, rather
    // than being an anchor pointing at an event about something else.
    let event = proposal
        .events()
        .iter()
        .find(|event| event.kind == HydrologyEventKind::ForcingSettlement)
        .expect("the settlement event exists");
    assert_eq!(event.effects.len(), 2);
    assert!(
        event
            .effects
            .iter()
            .any(|effect| effect.property == causafera_domains::HydrologyProperty::Surface)
    );
    assert!(
        event
            .effects
            .iter()
            .any(|effect| effect.property == causafera_domains::HydrologyProperty::ForcingInput)
    );
}

// ---------------------------------------------------------------------------
// Order independence
// ---------------------------------------------------------------------------

#[test]
fn the_proposal_is_identical_across_chunk_and_record_construction_orders() {
    let ground = Ground {
        infiltration_limit: 200,
        percolation: (1, 3),
        ..Ground::default()
    };
    let build_state = |reversed: bool| {
        let a = ChunkBuilder::new(0)
            .with(0, ground.build(), storage(300, 100, 0))
            .with(31, ground.build(), storage(50, 0, 0))
            .build();
        let b = ChunkBuilder::new(-1)
            .with(1023, ground.build(), storage(90, 40, 10))
            .build();
        field_set(if reversed { vec![b, a] } else { vec![a, b] })
    };
    let records = vec![
        Forcing::new(1, 5)
            .target(cell(0, 0), 3)
            .target(cell(-1, 1023), 1)
            .precipitation(700)
            .potential_et(60)
            .build(),
        Forcing::new(2, 5)
            .target(cell(0, 31), 1)
            .precipitation(25)
            .build(),
    ];

    let forward = HydrologyEvolutionModel::propose(
        &build_state(false),
        Scenario::new(&[-1, 0])
            .with_forcing(records.clone())
            .request(5),
    )
    .unwrap();
    let reversed = HydrologyEvolutionModel::propose(
        &build_state(true),
        Scenario::new(&[-1, 0]).with_forcing(records).request(5),
    )
    .unwrap();

    assert_eq!(forward, reversed);
    assert_eq!(forward.conservation().residual(), 0);
    assert!(
        !forward.transfer_receipts().is_empty(),
        "the fixture must actually move water, or this proves nothing"
    );
}
