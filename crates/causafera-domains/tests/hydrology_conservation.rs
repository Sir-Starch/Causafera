//! Exact conservation and receipt-ledger cross-validation
//! (`plans/hydrology.md` §8, verification gates V3, V16, V17 in part).
//!
//! The conservation receipt is not trusted here. Every check either recomputes
//! the budget from the two field sets independently, or folds the per-transfer
//! receipts back up and compares the two derivations. A ledger validated
//! against the bookkeeping that produced it agrees with itself and proves
//! nothing.

mod support;

use causafera_domains::{
    HydrologyBucket, HydrologyError, HydrologyEvolutionLimits, HydrologyEvolutionModel,
    HydrologyEvolutionProposal, HydrologyEvolutionRequest, HydrologyReceiptTotals, process,
    validate_boundary_transfers, validate_paired_transfers,
};
use causafera_geography::HydrologyFieldSet;
use causafera_types::WaterVolume;
use support::*;

/// The total water the world holds, summed straight out of the field set.
fn total_water(state: &HydrologyFieldSet) -> i128 {
    state.total_storage().expect("totals stay in range").get()
}

/// Every assertion the plan makes about a committed tick, checked against
/// derivations that do not come from the solver's own running totals.
fn assert_ledger_closes(before: &HydrologyFieldSet, proposal: &HydrologyEvolutionProposal) {
    let receipt = proposal.conservation();

    // 1. The residual is exactly zero.
    assert_eq!(receipt.residual(), 0, "residual must be exactly zero");
    assert!(receipt.require_balanced().is_ok());

    // 2. The before and after totals are the two field sets' own sums, not the
    //    solver's accumulators.
    assert_eq!(
        receipt.storage_before().unwrap() - receipt.conveyance_before(),
        total_water(before),
        "declared pre-state must match the state that was handed in"
    );
    assert_eq!(
        receipt.storage_after().unwrap() - receipt.conveyance_after(),
        total_water(proposal.after_state()),
        "declared post-state must match the state that came out"
    );

    // 3. The source and sink terms recomputed from the receipts agree with the
    //    aggregate literals.
    let totals = HydrologyReceiptTotals::from_receipts(proposal.transfer_receipts()).unwrap();
    assert!(
        totals.agrees_with(receipt),
        "receipt totals {totals:?} disagree with the aggregate literals"
    );

    // 4. Every internal transfer moved the same amount out of one bucket as into
    //    the other, and every source or sink receipt matches its own delta.
    validate_paired_transfers(proposal.transfer_receipts()).expect("transfers must be paired");
    validate_boundary_transfers(proposal.transfer_receipts()).expect("sinks must be exact");

    // 5. And the whole budget balances when written out longhand.
    assert_eq!(
        total_water(before) + totals.accepted_precipitation + totals.accepted_external_inflow,
        total_water(proposal.after_state())
            + totals.accepted_evapotranspiration
            + totals.boundary_exports,
        "before + sources must equal after + sinks"
    );
}

// ---------------------------------------------------------------------------
// V3 — zero-forcing negative control
// ---------------------------------------------------------------------------

#[test]
fn with_no_forcing_and_no_processes_total_water_is_unchanged() {
    let ground = Ground {
        infiltration_limit: 0,
        percolation: (0, 1),
        ..Ground::default()
    };
    let state = field_set(vec![
        ChunkBuilder::new(0)
            .with(0, ground.build(), storage(100, 200, 300))
            .with(500, ground.build(), storage(7, 0, 0))
            .build(),
    ]);
    let before = total_water(&state);
    assert_eq!(before, 607, "the fixture must actually hold water");

    let proposal =
        HydrologyEvolutionModel::propose(&state, Scenario::new(&[0]).request(5)).unwrap();

    assert_eq!(total_water(proposal.after_state()), before);
    assert!(
        proposal.transfer_receipts().is_empty(),
        "nothing was asked of this world, so nothing is recorded"
    );
    assert!(proposal.events().is_empty());
    assert!(proposal.applied_forcing().is_empty());
    assert!(proposal.terminal_leaves().is_empty());
    assert_ledger_closes(&state, &proposal);
}

#[test]
fn with_no_forcing_but_active_processes_water_moves_without_changing_its_total() {
    let ground = Ground {
        infiltration_limit: 40,
        percolation: (1, 3),
        ..Ground::default()
    };
    let state = field_set(vec![
        ChunkBuilder::new(0)
            .with(0, ground.build(), storage(500, 90, 0))
            .build(),
    ]);
    let before = total_water(&state);

    let proposal =
        HydrologyEvolutionModel::propose(&state, Scenario::new(&[0]).request(5)).unwrap();

    // Infiltration moves 40 surface into soil (90 + 40 = 130); percolation then
    // moves floor(130/3) = 43 of that into groundwater.
    let after = *proposal.after_state().cell(cell(0, 0)).unwrap();
    assert_eq!(after.surface_water(), WaterVolume::new(460));
    assert_eq!(after.soil_water(), WaterVolume::new(87));
    assert_eq!(after.groundwater(), WaterVolume::new(43));
    assert_eq!(total_water(proposal.after_state()), before);

    let totals = HydrologyReceiptTotals::from_receipts(proposal.transfer_receipts()).unwrap();
    assert_eq!(
        totals.internal_transfers, 83,
        "40 infiltrated plus 43 percolated"
    );
    assert_eq!(totals.accepted_precipitation, 0);
    assert_eq!(totals.accepted_evapotranspiration, 0);
    assert_ledger_closes(&state, &proposal);
}

// ---------------------------------------------------------------------------
// Sources and sinks
// ---------------------------------------------------------------------------

#[test]
fn external_inflow_is_a_source_distinct_from_precipitation() {
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
            .precipitation(120)
            .external_inflow(80)
            .build(),
    ]);

    let proposal = HydrologyEvolutionModel::propose(&state, scenario.request(5)).unwrap();

    // Both are sources, and both are counted — but under their own process
    // identities, so a downstream reader can tell rain from an inflow.
    assert_eq!(proposal.conservation().accepted_precipitation(), 120);
    assert_eq!(proposal.conservation().accepted_external_inflow(), 80);
    assert_eq!(
        proposal
            .after_state()
            .cell(cell(0, 0))
            .unwrap()
            .surface_water(),
        WaterVolume::new(200)
    );
    assert_eq!(
        proposal
            .transfer_receipts()
            .iter()
            .filter(|receipt| receipt.process_kind() == process::EXTERNAL_INFLOW)
            .count(),
        1
    );
    assert_ledger_closes(&state, &proposal);
}

#[test]
fn a_source_and_a_sink_in_the_same_tick_both_reach_the_ledger() {
    let ground = Ground {
        infiltration_limit: 60,
        percolation: (1, 4),
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
            .potential_et(70)
            .build(),
    ]);

    let proposal = HydrologyEvolutionModel::propose(&state, scenario.request(5)).unwrap();

    // 400 in; 60 infiltrates; floor(60/4) = 15 percolates; ET then takes 70 —
    // all of it from the 340 left on the surface.
    let after = *proposal.after_state().cell(cell(0, 0)).unwrap();
    assert_eq!(after.surface_water(), WaterVolume::new(270));
    assert_eq!(after.soil_water(), WaterVolume::new(45));
    assert_eq!(after.groundwater(), WaterVolume::new(15));
    assert_eq!(proposal.conservation().accepted_precipitation(), 400);
    assert_eq!(proposal.conservation().accepted_evapotranspiration(), 70);
    assert_eq!(total_water(proposal.after_state()), 330);
    assert_ledger_closes(&state, &proposal);
}

// ---------------------------------------------------------------------------
// V16 — closed-basin conservation across many ticks
// ---------------------------------------------------------------------------

#[test]
fn a_closed_basin_conserves_exactly_across_a_hundred_ticks() {
    let ground = Ground {
        infiltration_limit: 25,
        percolation: (1, 7),
        ..Ground::default()
    };
    let mut state = field_set(vec![
        ChunkBuilder::new(0)
            .with(0, ground.build(), storage(4_000, 900, 0))
            .with(1, ground.build(), storage(0, 5_000, 0))
            .with(1023, ground.build(), storage(777, 0, 13))
            .build(),
        ChunkBuilder::new(-1)
            .with(0, ground.build(), storage(1_234, 56, 7))
            .build(),
    ]);
    let opening = total_water(&state);
    assert_eq!(opening, 11_987);

    let mut moving_ticks = 0_usize;
    for tick in 1..=100_u64 {
        let scenario = Scenario::new(&[-1, 0]);
        let proposal = HydrologyEvolutionModel::propose(&state, scenario.request(tick)).unwrap();

        assert_ledger_closes(&state, &proposal);
        assert_eq!(
            total_water(proposal.after_state()),
            opening,
            "tick {tick} changed the world's total water with no source or sink"
        );
        let totals = HydrologyReceiptTotals::from_receipts(proposal.transfer_receipts()).unwrap();
        assert_eq!(totals.accepted_precipitation, 0);
        assert_eq!(totals.accepted_external_inflow, 0);
        assert_eq!(totals.accepted_evapotranspiration, 0);
        assert_eq!(totals.boundary_exports, 0);
        if totals.internal_transfers > 0 {
            moving_ticks += 1;
        }
        assert_eq!(proposal.batch_sequence(), tick);
        state = proposal.after_state().clone();
    }

    // Non-vacuous: a world where nothing ever moved would conserve trivially.
    assert!(
        moving_ticks >= 90,
        "only {moving_ticks} of 100 ticks moved water; the run must be doing work"
    );
    assert_eq!(total_water(&state), opening);
}

// ---------------------------------------------------------------------------
// Refusals
// ---------------------------------------------------------------------------

#[test]
fn a_forcing_total_that_would_overflow_the_carrier_is_refused() {
    // Given: a cell already holding almost the whole carrier range, whose
    // capacity is also the whole range, and rain that would push it past.
    let ground = Ground {
        surface_capacity: u64::MAX,
        soil_capacity: u64::MAX,
        groundwater_capacity: u64::MAX,
        infiltration_limit: 0,
        percolation: (0, 1),
        ..Ground::default()
    };
    let state = field_set(vec![
        ChunkBuilder::new(0)
            .with(0, ground.build(), storage(u64::MAX - 10, 0, 0))
            .build(),
    ]);
    let scenario = Scenario::new(&[0]).with_forcing(vec![
        Forcing::new(1, 5)
            .target(cell(0, 0), 1)
            .precipitation(u64::MAX)
            .build(),
    ]);

    let proposal = HydrologyEvolutionModel::propose(&state, scenario.request(5)).unwrap();

    // Then: the cell fills to its capacity and the rest is refused, rather than
    // wrapping. Capacity is the bound that engages, and it is on the receipt.
    assert_eq!(
        proposal
            .after_state()
            .cell(cell(0, 0))
            .unwrap()
            .surface_water(),
        WaterVolume::new(u64::MAX)
    );
    assert_eq!(proposal.conservation().accepted_precipitation(), 10);
    assert_eq!(proposal.conservation().residual(), 0);
    let receipt = &proposal.transfer_receipts()[0];
    assert_eq!(receipt.requested(), WaterVolume::new(u64::MAX));
    assert_eq!(receipt.accepted(), WaterVolume::new(10));
    assert_eq!(
        receipt.unaccepted(),
        WaterVolume::new(u64::MAX - 10),
        "the refused remainder is recorded, not wrapped away"
    );
    assert_ledger_closes(&state, &proposal);
}

#[test]
fn a_batch_past_the_transfer_limit_is_refused_before_it_is_returned() {
    // Given: a world where every one of 1 024 cells produces two receipts, and a
    // limit of three.
    let ground = Ground {
        infiltration_limit: 10,
        percolation: (1, 2),
        ..Ground::default()
    };
    let mut builder = ChunkBuilder::new(0);
    for ordinal in 0..8_u16 {
        builder = builder.with(ordinal, ground.build(), storage(100, 100, 0));
    }
    let state = field_set(vec![builder.build()]);

    let scenario = Scenario::new(&[0]);
    let mut request = scenario.request(5);
    request.limits = HydrologyEvolutionLimits {
        max_transfers_per_tick: 3,
        ..HydrologyEvolutionLimits::default()
    };

    match HydrologyEvolutionModel::propose(&state, request) {
        Err(HydrologyError::TransferLimitExceeded { count, max }) => {
            assert_eq!(max, 3);
            assert_eq!(count, 16, "eight cells infiltrating and percolating");
        }
        other => panic!("expected the transfer limit to reject the batch, got {other:?}"),
    }

    // And: the same world inside its limit is accepted, so the bound is what
    // rejected it rather than the fixture being unrunnable.
    let proposal =
        HydrologyEvolutionModel::propose(&state, Scenario::new(&[0]).request(5)).unwrap();
    assert_eq!(proposal.transfer_receipts().len(), 16);
    assert_ledger_closes(&state, &proposal);
}

// ---------------------------------------------------------------------------
// Receipt shape
// ---------------------------------------------------------------------------

#[test]
fn every_receipt_carries_the_batch_and_tick_it_belongs_to() {
    let ground = Ground {
        infiltration_limit: 30,
        percolation: (1, 2),
        ..Ground::default()
    };
    let state = field_set(vec![
        ChunkBuilder::new(0)
            .with(0, ground.build(), storage(500, 0, 0))
            .build(),
    ]);
    let first = HydrologyEvolutionModel::propose(&state, Scenario::new(&[0]).request(11)).unwrap();
    assert_eq!(first.batch_sequence(), 1);
    for receipt in first.transfer_receipts() {
        assert_eq!(receipt.tick(), 11);
        assert_eq!(receipt.batch_sequence(), 1);
    }

    let second =
        HydrologyEvolutionModel::propose(first.after_state(), Scenario::new(&[0]).request(12))
            .unwrap();
    assert_eq!(second.batch_sequence(), 2);
    for receipt in second.transfer_receipts() {
        assert_eq!(receipt.tick(), 12);
        assert_eq!(receipt.batch_sequence(), 2);
    }
}

#[test]
fn receipt_keys_are_unique_within_a_tick() {
    // The observer projects transfers keyed on
    // `(tick, process_kind, source, target)`. Two receipts sharing that key
    // would be indistinguishable on the wire and one would silently win.
    let ground = Ground {
        infiltration_limit: 40,
        percolation: (1, 3),
        ..Ground::default()
    };
    let mut builder = ChunkBuilder::new(0);
    for ordinal in 0..6_u16 {
        builder = builder.with(ordinal, ground.build(), storage(300, 120, 0));
    }
    let state = field_set(vec![builder.build()]);
    let scenario = Scenario::new(&[0]).with_forcing(vec![
        Forcing::new(1, 5)
            .target(cell(0, 0), 1)
            .target(cell(0, 1), 2)
            .precipitation(90)
            .potential_et(30)
            .build(),
        Forcing::new(2, 5)
            .target(cell(0, 0), 1)
            .precipitation(10)
            .build(),
    ]);

    let proposal = HydrologyEvolutionModel::propose(&state, scenario.request(5)).unwrap();
    assert!(proposal.transfer_receipts().len() > 6);

    let mut keys: Vec<_> = proposal
        .transfer_receipts()
        .iter()
        .map(|receipt| receipt.canonical_key())
        .collect();
    let total = keys.len();
    keys.sort();
    keys.dedup();
    assert_eq!(keys.len(), total, "receipt canonical keys must be unique");

    // Two records rained on cell 0, and each keeps its own receipt because the
    // source carrier names the record.
    assert_eq!(
        proposal
            .transfer_receipts()
            .iter()
            .filter(|receipt| receipt.process_kind() == process::PRECIPITATION
                && receipt.target() == causafera_geography::HydrologyCarrierKey::Cell(cell(0, 0)))
            .count(),
        2
    );
    assert_ledger_closes(&state, &proposal);
}

#[test]
fn every_receipt_that_moved_water_names_the_event_that_carried_it() {
    // Receipts are evicted after eight batches; the events they name are not.
    // A receipt with no event to resolve against is an orphan the moment its
    // transfer trace is asked for, so the link is checked here rather than
    // discovered missing in Explanation.
    let ground = Ground {
        infiltration_limit: 50,
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
            .precipitation(300)
            .potential_et(40)
            .build(),
    ]);

    let proposal = HydrologyEvolutionModel::propose(&state, scenario.request(5)).unwrap();
    let emitted: Vec<_> = proposal.events().iter().map(|event| &event.key).collect();
    assert!(emitted.len() >= 4);

    let mut linked = 0_usize;
    for receipt in proposal.transfer_receipts() {
        if receipt.accepted().get() == 0 {
            // Nothing moved, so there is no bucket-change event to name.
            assert!(receipt.storage_event().is_none());
            continue;
        }
        linked += 1;
        let transfer = receipt
            .transfer_event()
            .expect("an accepted transfer names its event");
        assert!(
            emitted.contains(&transfer),
            "transfer event {transfer:?} is not in the batch"
        );
        let storage = receipt
            .storage_event()
            .expect("an accepted transfer names the event that settled its storage");
        assert!(
            emitted.contains(&storage),
            "storage event {storage:?} is not in the batch"
        );
    }
    assert!(linked >= 4, "only {linked} receipts moved water");
    assert_ledger_closes(&state, &proposal);
}

#[test]
fn a_refused_transfer_names_no_storage_event() {
    // Given: rain onto a cell with no room for it at all.
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
            .precipitation(400)
            .build(),
    ]);

    let proposal = HydrologyEvolutionModel::propose(&state, scenario.request(5)).unwrap();
    let receipt = &proposal.transfer_receipts()[0];

    // Then: the receipt still names the record's application event — that did
    // happen — but claims no storage change, because none did.
    assert_eq!(receipt.accepted(), WaterVolume::ZERO);
    assert_eq!(receipt.unaccepted(), WaterVolume::new(400));
    assert!(receipt.transfer_event().is_some());
    assert!(receipt.storage_event().is_none());
    assert_ledger_closes(&state, &proposal);
}

#[test]
fn a_sink_receipt_never_claims_to_be_an_internal_transfer() {
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
    let scenario = Scenario::new(&[0]).with_forcing(vec![
        Forcing::new(1, 5)
            .target(cell(0, 0), 1)
            .potential_et(200)
            .build(),
    ]);
    let proposal = HydrologyEvolutionModel::propose(&state, scenario.request(5)).unwrap();

    for receipt in proposal.transfer_receipts() {
        let touches_outside = receipt.source_bucket() == HydrologyBucket::External
            || receipt.target_bucket() == HydrologyBucket::External;
        assert_eq!(receipt.is_internal_transfer(), !touches_outside);
    }
    assert_ledger_closes(&state, &proposal);
}

// ---------------------------------------------------------------------------
// Requests that must be rejected outright
// ---------------------------------------------------------------------------

#[test]
fn a_chunk_whose_chart_has_no_registered_metric_cannot_be_evolved() {
    // The field set already refuses this at construction. The solver re-checks
    // it, because a caller can hand it any value object it likes and nothing
    // about a chunk with no cell area or timestep is computable.
    let state = field_set(vec![
        ChunkBuilder::new(0)
            .with(0, Ground::default().build(), storage(10, 0, 0))
            .build(),
    ]);
    let elsewhere = causafera_geography::HydrologyGridMetrics::new(vec![(
        causafera_types::SpatialChartId::new(99),
        causafera_geography::HydrologyGridMetric::new(nz64(1), nz64(1), nz64(1)),
    )])
    .unwrap();
    let scenario = Scenario::new(&[0]);
    let request = HydrologyEvolutionRequest {
        metrics: &elsewhere,
        ..scenario.request(5)
    };
    assert!(matches!(
        HydrologyEvolutionModel::propose(&state, request),
        Err(HydrologyError::State(
            causafera_geography::HydrologyStateError::UnknownMetricChart
        ))
    ));
}
