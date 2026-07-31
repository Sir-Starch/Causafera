//! Explicit boundary behaviour: no-flux retention, open export, the exact
//! head/conductance equation, and the refusal to assume either.
//!
//! Covers `plans/hydrology.md` verification gates V13 and the export half of V17.

mod support;

use causafera_domains::{
    HydrologyError, HydrologyEvolutionModel, HydrologyEvolutionProposal, HydrologyReceiptTotals,
    HydrologyTransferReceipt, process, validate_boundary_transfers, validate_paired_transfers,
};
use causafera_geography::{
    FaceDirection, FluxBoundary, HydrologyBoundaryCondition, HydrologyBoundaryMap,
    HydrologyCarrierKey, HydrologyExteriorFaceKey,
};
use causafera_types::WaterVolume;

use support::{
    ChunkBuilder, Forcing, Ground, Scenario, boundary_map, cell, closed_perimeter, field_set,
    storage, terrain_from,
};

/// Ground that conducts nothing vertically, so a test's assertions are about the
/// boundary rather than about infiltration it did not ask for.
fn exportable(surface_conductance: u64, groundwater_conductance: u64) -> Ground {
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
        surface_conductance,
        groundwater_conductance,
    }
}

/// One boundary map where a single face of cell zero is open, everything else
/// closed. `channel` picks which of the face's two channels opens.
fn one_open_face(
    direction: FaceDirection,
    surface: FluxBoundary,
    groundwater: FluxBoundary,
) -> HydrologyBoundaryMap {
    let opened = HydrologyExteriorFaceKey::new(cell(0, 0), direction);
    boundary_map(&[0], move |face| {
        if face == opened {
            HydrologyBoundaryCondition::new(surface, groundwater)
        } else {
            HydrologyBoundaryCondition::CLOSED
        }
    })
}

fn export_receipt(
    proposal: &HydrologyEvolutionProposal,
    process_kind: u32,
) -> &HydrologyTransferReceipt {
    proposal
        .transfer_receipts()
        .iter()
        .find(|receipt| receipt.process_kind() == process_kind)
        .expect("the export was proposed")
}

fn assert_budget_closes(proposal: &HydrologyEvolutionProposal) {
    let ledger = proposal.conservation();
    assert_eq!(ledger.residual(), 0);
    assert_eq!(
        ledger.storage_before().unwrap() + ledger.sources().unwrap(),
        ledger.storage_after().unwrap() + ledger.sinks().unwrap(),
        "before + sources == after + sinks"
    );
    // The ledger's aggregate terms recomputed from the per-transfer receipts,
    // which is a second derivation rather than the solver agreeing with itself.
    let totals = HydrologyReceiptTotals::from_receipts(proposal.transfer_receipts()).unwrap();
    assert!(totals.agrees_with(ledger));
    validate_paired_transfers(proposal.transfer_receipts()).unwrap();
    validate_boundary_transfers(proposal.transfer_receipts()).unwrap();
}

// ---------------------------------------------------------------------------
// V13 — a face with no record is a validation failure, never a default
// ---------------------------------------------------------------------------

#[test]
fn an_exterior_face_without_a_boundary_record_rejects_the_proposal() {
    // Exporting and blocking are both physical claims. A solver that picked one
    // when the record was missing would be inventing a wall or a drain.
    let field = ChunkBuilder::new(0)
        .with(0, exportable(1_000, 0).build(), storage(10_000_000, 0, 0))
        .build();
    let state = field_set(vec![field]);
    let scenario = Scenario::new(&[0]).with_boundaries(HydrologyBoundaryMap::default());

    assert_eq!(
        HydrologyEvolutionModel::propose(&state, scenario.request(1)),
        Err(HydrologyError::UnspecifiedBoundaryFace)
    );
}

#[test]
fn a_partial_boundary_map_rejects_as_firmly_as_an_empty_one() {
    // Recording the perimeter of one chunk and forgetting the other is the same
    // omission, and the whole proposal is refused rather than the missing faces
    // being treated as walls.
    let state = field_set(vec![
        ChunkBuilder::new(0)
            .with(0, exportable(1_000, 0).build(), storage(1_000, 0, 0))
            .build(),
        ChunkBuilder::new(1).build(),
    ]);
    let scenario = Scenario::new(&[0, 1]).with_boundaries(closed_perimeter(&[0]));

    assert_eq!(
        HydrologyEvolutionModel::propose(&state, scenario.request(1)),
        Err(HydrologyError::UnspecifiedBoundaryFace)
    );
}

// ---------------------------------------------------------------------------
// V13 — no-flux retains
// ---------------------------------------------------------------------------

#[test]
fn a_no_flux_perimeter_retains_every_drop() {
    // High ground, high water, plenty of conductance, and nowhere to go.
    let field = ChunkBuilder::new(0)
        .with(
            0,
            exportable(1_000_000, 0).build(),
            storage(50_000_000, 0, 0),
        )
        .build();
    let state = field_set(vec![field]);
    let scenario =
        Scenario::new(&[0]).with_terrain(terrain_from(
            &[0],
            |_, ordinal| {
                if ordinal == 0 { 10_000 } else { 0 }
            },
        ));
    let proposal = HydrologyEvolutionModel::propose(&state, scenario.request(1)).unwrap();

    assert_eq!(proposal.conservation().boundary_exports(), 0);
    assert_eq!(
        proposal
            .after_state()
            .cell(cell(0, 0))
            .unwrap()
            .surface_water(),
        WaterVolume::new(50_000_000)
    );
    assert_budget_closes(&proposal);
}

// ---------------------------------------------------------------------------
// V13 — open export removes exactly the recorded amount
// ---------------------------------------------------------------------------

#[test]
fn an_open_surface_face_exports_head_difference_times_conductance() {
    // head 100 mm (terrain) + 0 mm (0.5 mm³ of depth floors to zero) against an
    // external head of 30 mm, through 7 mm² of conductance: exactly 490 mm³.
    let field = ChunkBuilder::new(0)
        .with(0, exportable(1_000, 0).build(), storage(500, 0, 0))
        .build();
    let state = field_set(vec![field]);
    let scenario = Scenario::new(&[0])
        .with_terrain(terrain_from(
            &[0],
            |_, ordinal| {
                if ordinal == 0 { 100 } else { 0 }
            },
        ))
        .with_boundaries(one_open_face(
            FaceDirection::NegX,
            FluxBoundary::Open {
                external_head_mm: 30,
                conductance_mm2_per_tick: 7,
            },
            FluxBoundary::NoFlux,
        ));
    let proposal = HydrologyEvolutionModel::propose(&state, scenario.request(1)).unwrap();

    let receipt = export_receipt(&proposal, process::SURFACE_BOUNDARY_EXPORT);
    assert_eq!(receipt.requested(), WaterVolume::new(490));
    assert_eq!(receipt.accepted(), WaterVolume::new(490));
    assert_eq!(
        receipt.target(),
        HydrologyCarrierKey::ExteriorFace(HydrologyExteriorFaceKey::new(
            cell(0, 0),
            FaceDirection::NegX
        )),
        "the sink names the face it left through, not a generic outside"
    );
    assert_eq!(proposal.conservation().boundary_exports(), 490);
    assert_eq!(
        proposal
            .after_state()
            .cell(cell(0, 0))
            .unwrap()
            .surface_water(),
        WaterVolume::new(10)
    );
    assert_budget_closes(&proposal);
}

#[test]
fn an_equal_or_lower_external_head_exports_nothing() {
    for external_head_mm in [100, 250] {
        let field = ChunkBuilder::new(0)
            .with(0, exportable(1_000, 0).build(), storage(500, 0, 0))
            .build();
        let state = field_set(vec![field]);
        let scenario = Scenario::new(&[0])
            .with_terrain(terrain_from(
                &[0],
                |_, ordinal| {
                    if ordinal == 0 { 100 } else { 0 }
                },
            ))
            .with_boundaries(one_open_face(
                FaceDirection::NegX,
                FluxBoundary::Open {
                    external_head_mm,
                    conductance_mm2_per_tick: 7,
                },
                FluxBoundary::NoFlux,
            ));
        let proposal = HydrologyEvolutionModel::propose(&state, scenario.request(1)).unwrap();

        assert_eq!(proposal.conservation().boundary_exports(), 0);
        assert_eq!(
            proposal
                .after_state()
                .cell(cell(0, 0))
                .unwrap()
                .surface_water(),
            WaterVolume::new(500),
            "an open face is not a drain; it is a head comparison"
        );
    }
}

#[test]
fn an_export_the_donor_cannot_fund_is_reduced_and_recorded() {
    // The face asks for 490 and the cell holds 100. The donor reduction pays what
    // it has, and the receipt keeps the demand so the limiter stays visible.
    let field = ChunkBuilder::new(0)
        .with(0, exportable(1_000, 0).build(), storage(100, 0, 0))
        .build();
    let state = field_set(vec![field]);
    let scenario = Scenario::new(&[0])
        .with_terrain(terrain_from(
            &[0],
            |_, ordinal| {
                if ordinal == 0 { 100 } else { 0 }
            },
        ))
        .with_boundaries(one_open_face(
            FaceDirection::NegX,
            FluxBoundary::Open {
                external_head_mm: 30,
                conductance_mm2_per_tick: 7,
            },
            FluxBoundary::NoFlux,
        ));
    let proposal = HydrologyEvolutionModel::propose(&state, scenario.request(1)).unwrap();

    let receipt = export_receipt(&proposal, process::SURFACE_BOUNDARY_EXPORT);
    assert_eq!(receipt.requested(), WaterVolume::new(490));
    assert_eq!(receipt.accepted(), WaterVolume::new(100));
    assert_eq!(receipt.unaccepted(), WaterVolume::new(390));
    assert_eq!(proposal.conservation().boundary_exports(), 100);
    assert_budget_closes(&proposal);
}

#[test]
fn a_surface_export_competes_with_internal_outflow_for_the_same_donor() {
    // One open face and one downhill neighbour, both asking the same donor for
    // three units against four available. Reduced together in canonical order:
    // interior faces sort before exterior ones.
    let ground = exportable(1, 0);
    let field = ChunkBuilder::new(0)
        .with(0, ground.build(), storage(4, 0, 0))
        .with(1, ground.build(), storage(0, 0, 0))
        .build();
    let state = field_set(vec![field]);
    let scenario = Scenario::new(&[0])
        .with_terrain(terrain_from(
            &[0],
            |_, ordinal| {
                if ordinal == 0 { 3 } else { 0 }
            },
        ))
        .with_boundaries(one_open_face(
            FaceDirection::NegX,
            FluxBoundary::Open {
                external_head_mm: 0,
                conductance_mm2_per_tick: 1,
            },
            FluxBoundary::NoFlux,
        ));
    let proposal = HydrologyEvolutionModel::propose(&state, scenario.request(1)).unwrap();

    // Two demands of three against four available: floor(3*4/6) = 2 each, and the
    // two spare remainders are equal, so the first spare unit goes to the lower
    // canonical key — the interior face.
    let lateral = proposal
        .transfer_receipts()
        .iter()
        .find(|receipt| receipt.process_kind() == process::SURFACE_LATERAL)
        .unwrap();
    assert_eq!(lateral.accepted(), WaterVolume::new(2));
    assert_eq!(
        export_receipt(&proposal, process::SURFACE_BOUNDARY_EXPORT).accepted(),
        WaterVolume::new(2)
    );
    assert_eq!(
        proposal
            .after_state()
            .cell(cell(0, 0))
            .unwrap()
            .surface_water(),
        WaterVolume::ZERO
    );
    assert_budget_closes(&proposal);
}

// ---------------------------------------------------------------------------
// V13 — the two channels of one face are independent
// ---------------------------------------------------------------------------

#[test]
fn a_faces_surface_and_groundwater_channels_are_independent() {
    // Open to groundwater, closed to surface. Only the water table leaves.
    let ground = Ground {
        aquifer_base_mm: 0,
        ..exportable(1_000, 1_000)
    };
    let field = ChunkBuilder::new(0)
        .with(0, ground.build(), storage(1_000, 0, 2_000_000))
        .build();
    let state = field_set(vec![field]);
    let scenario = Scenario::new(&[0])
        .with_terrain(terrain_from(
            &[0],
            |_, ordinal| {
                if ordinal == 0 { 100 } else { 0 }
            },
        ))
        .with_boundaries(one_open_face(
            FaceDirection::NegX,
            FluxBoundary::NoFlux,
            FluxBoundary::Open {
                external_head_mm: 0,
                conductance_mm2_per_tick: 3,
            },
        ));
    let proposal = HydrologyEvolutionModel::propose(&state, scenario.request(1)).unwrap();

    // saturated depth = floor(2 000 000 * 5 / (1 000 000 * 1)) = 10 mm above a
    // base of zero, so the export is 10 mm × 3 mm² = 30 mm³.
    let receipt = export_receipt(&proposal, process::GROUNDWATER_BOUNDARY_EXPORT);
    assert_eq!(receipt.accepted(), WaterVolume::new(30));
    assert_eq!(
        proposal
            .after_state()
            .cell(cell(0, 0))
            .unwrap()
            .surface_water(),
        WaterVolume::new(1_000),
        "the closed surface channel exported nothing"
    );
    assert_eq!(
        proposal
            .after_state()
            .cell(cell(0, 0))
            .unwrap()
            .groundwater(),
        WaterVolume::new(1_999_970)
    );
    assert_budget_closes(&proposal);
}

// ---------------------------------------------------------------------------
// V17 — the whole budget across forcing, ET, routing, and export
// ---------------------------------------------------------------------------

#[test]
fn the_whole_budget_closes_across_forcing_evapotranspiration_and_export() {
    let ground = Ground {
        surface_capacity: 1_000_000_000,
        soil_capacity: 1_000_000,
        groundwater_capacity: 1_000_000,
        infiltration_limit: 5_000,
        percolation: (1, 4),
        specific_yield: (1, 5),
        aquifer_base_mm: 0,
        baseflow_threshold: 0,
        baseflow: (0, 1),
        surface_conductance: 1_000,
        groundwater_conductance: 1_000,
    };
    let mut builder = ChunkBuilder::new(0);
    for ordinal in 0..4_u16 {
        builder = builder.with(
            ordinal,
            ground.build(),
            storage(2_000_000, 100_000, 400_000),
        );
    }
    let mut state = field_set(vec![builder.build()]);
    let mut scenario = Scenario::new(&[0])
        .with_terrain(terrain_from(&[0], |_, ordinal| {
            if ordinal < 4 {
                4_000 - i32::from(ordinal) * 1_000
            } else {
                0
            }
        }))
        .with_boundaries(one_open_face(
            FaceDirection::NegX,
            FluxBoundary::Open {
                external_head_mm: 0,
                conductance_mm2_per_tick: 50,
            },
            FluxBoundary::Open {
                external_head_mm: 0,
                conductance_mm2_per_tick: 20,
            },
        ));

    let mut ledger_sources = 0_i128;
    let mut ledger_sinks = 0_i128;
    let opening = state.total_storage().unwrap().get();
    for tick in 1..=25_u64 {
        scenario.forcing = vec![
            Forcing::new(tick, tick)
                .target(cell(0, 0), 1)
                .target(cell(0, 3), 3)
                .precipitation(90_000)
                .potential_et(7_000)
                .build(),
        ];
        let proposal = HydrologyEvolutionModel::propose(&state, scenario.request(tick)).unwrap();
        let per_tick = proposal.conservation();
        assert_eq!(per_tick.residual(), 0, "tick {tick} must close exactly");
        assert_budget_closes(&proposal);
        ledger_sources += per_tick.sources().unwrap();
        ledger_sinks += per_tick.sinks().unwrap();
        state = proposal.after_state().clone();
    }

    assert_eq!(
        opening + ledger_sources - ledger_sinks,
        state.total_storage().unwrap().get(),
        "the aggregate run closes as exactly as each tick did"
    );
    assert!(
        ledger_sinks > 0,
        "the run has to have actually exported water"
    );
}
