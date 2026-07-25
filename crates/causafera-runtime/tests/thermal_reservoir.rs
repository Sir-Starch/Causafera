use std::collections::BTreeSet;

use causafera_domains::{
    THERMAL_SCALE, ThermalActiveRegion, ThermalBoundaryBehavior, ThermalCellKey, ThermalEnergy,
    ThermalEvolutionRequest, ThermalField, ThermalFieldSet, ThermalInjectionProposal,
    ThermalParameters, ThermalReservoir, ThermalReservoirId, ThermalReservoirSchedule,
};
use causafera_runtime::{
    Runtime, RuntimeConfig, RuntimeError, THERMAL_RESERVOIR_TRANSFER_EVENT_KIND,
};
use causafera_types::{ChartChunkCoord, ChunkCoord, SpatialChartId, TraceId};

#[path = "support/thermal.rs"]
mod support;
use support::{chunk, field_set, parameters, total_energy};

#[test]
fn exhaustion() {
    // Given: the finite production-bootstrap per-tick reservoir.
    let mut runtime = Runtime::new(RuntimeConfig::new(1_701)).expect("runtime must bootstrap");
    let initial = runtime
        .export_snapshot()
        .expect("initial state must export");
    let initial_total = total_energy(&initial.thermal);

    // When: execution continues beyond its finite budget.
    runtime.run_ticks(10).expect("thermal ticks must execute");
    let exhausted = runtime.export_snapshot().expect("final state must export");

    // Then: the budget reaches zero exactly and no extra transfer event is emitted.
    assert_eq!(exhausted.thermal.reservoirs[0].budget, 0);
    assert_eq!(total_energy(&exhausted.thermal), initial_total);
    assert_eq!(
        exhausted
            .traces
            .events
            .iter()
            .filter(|event| event.kind.raw() == THERMAL_RESERVOIR_TRANSFER_EVENT_KIND)
            .count(),
        8
    );
}

#[test]
fn exact_global_conservation() {
    // Given: the production thermal carrier's initial conserved total.
    let mut runtime = Runtime::new(RuntimeConfig::new(1_702)).expect("runtime must bootstrap");
    let initial_total = total_energy(
        &runtime
            .export_snapshot()
            .expect("state must export")
            .thermal,
    );

    // When: reservoir injection and diffusion run over many ticks.
    runtime.run_ticks(64).expect("thermal ticks must execute");
    let after = runtime.export_snapshot().expect("state must export");

    // Then: cell energy plus finite reservoir budget remains exact.
    assert_eq!(total_energy(&after.thermal), initial_total);
}

#[test]
fn per_tick_residual_zero() {
    // Given: a production runtime with a finite thermal reservoir.
    let mut runtime = Runtime::new(RuntimeConfig::new(1_703)).expect("runtime must bootstrap");

    // When: each tick is observed independently.
    for _ in 0..32 {
        runtime.tick().expect("thermal tick must execute");
        let snapshot = runtime.export_snapshot().expect("state must export");

        // Then: the authoritative receipt for that tick has an exact zero residual.
        assert_eq!(
            snapshot
                .thermal
                .conservation_receipts
                .last()
                .expect("tick must retain conservation receipt")
                .residual,
            0
        );
    }
}

#[test]
fn same_tick_propagation() {
    // Given: a production reservoir targeting a zero-energy cell with cooler neighbors.
    let mut runtime = Runtime::new(RuntimeConfig::new(1_704)).expect("runtime must bootstrap");

    // When: its first injection and diffusion execute in one Physics phase.
    runtime.tick().expect("first thermal tick must execute");
    let snapshot = runtime.export_snapshot().expect("state must export");
    let target = snapshot.thermal.reservoirs[0].target;
    let receipt = snapshot
        .thermal
        .transfer_receipts
        .iter()
        .find(|receipt| receipt.cell == target)
        .expect("target receipt must exist");

    // Then: accepted energy participates in positive outgoing flux in that same tick.
    assert!(receipt.faces.iter().any(|face| face.signed_flux > 0));
    assert_eq!(receipt.reservoirs[0].scheduled_injection, THERMAL_SCALE / 8);
    assert_eq!(receipt.reservoirs[0].accepted_injection, THERMAL_SCALE / 8);
    assert_eq!(receipt.reservoirs[0].rejected_injection, 0);
    let transfer_trace = receipt.reservoirs[0]
        .transfer_trace_id
        .expect("accepted injection must retain its transfer trace");
    let cell_change_trace = receipt
        .cell_change_trace_id
        .expect("changing target must retain its cell trace");
    let target_event = snapshot
        .traces
        .events
        .iter()
        .find(|event| event.trace_id == cell_change_trace)
        .expect("target cell event must exist");
    assert!(
        target_event
            .causes
            .contains(&snapshot.thermal.reservoirs[0].bootstrap_trace)
    );
    assert!(!target_event.causes.contains(&transfer_trace));
    for face in &receipt.faces {
        let Some(neighbor_receipt) = snapshot
            .thermal
            .transfer_receipts
            .iter()
            .find(|candidate| candidate.cell == face.neighbor)
        else {
            continue;
        };
        let Some(neighbor_trace) = neighbor_receipt.cell_change_trace_id else {
            continue;
        };
        let neighbor_event = snapshot
            .traces
            .events
            .iter()
            .find(|event| event.trace_id == neighbor_trace)
            .expect("neighbor cell event must exist");
        assert!(!neighbor_event.causes.contains(&transfer_trace));
    }
    let target_field = snapshot
        .thermal
        .field_set
        .fields
        .iter()
        .find(|field| field.chunk == target.chunk)
        .expect("target field must exist");
    assert_eq!(
        target_field.last_change[usize::from(target.cell_index)],
        cell_change_trace
    );
    assert_eq!(snapshot.thermal.reservoirs[0].last_change, transfer_trace);
}

#[test]
fn net_zero_target() {
    // Given: a center cell at 20 units receiving 60 while six cold faces each carry 10 away.
    let mut energy = vec![ThermalEnergy::ZERO; 27];
    energy[13] = ThermalEnergy::new(20).expect("fixture energy must be valid");
    let fields = field_set(energy);
    let target = ThermalCellKey::new(chunk(), 13);
    let reservoir = ThermalReservoir {
        id: ThermalReservoirId::new(1),
        target,
        budget: ThermalEnergy::new(60).expect("budget must be valid"),
        schedule: ThermalReservoirSchedule::OneShot,
        bootstrap_trace: TraceId::new(2),
        last_change: TraceId::new(2),
    };
    let injection = ThermalInjectionProposal {
        reservoir_id: reservoir.id,
        target,
        scheduled_amount: reservoir.budget,
    };
    let active = BTreeSet::from([chunk()]);
    let region = ThermalActiveRegion::new(active.clone(), active).expect("region must be valid");

    // When: post-injection frozen-state diffusion is proposed.
    let proposal = fields
        .propose_evolution(ThermalEvolutionRequest {
            tick: 1,
            parameters: parameters(128),
            active_region: &region,
            boundary_behavior: ThermalBoundaryBehavior::NoFluxOutsideActiveRegion,
            reservoirs: &[reservoir],
            injections: &[injection],
        })
        .expect("evolution must succeed");

    // Then: no unchanged cell event exists, but the target receipt retains the accepted transfer.
    assert!(
        proposal
            .cell_changes()
            .iter()
            .all(|change| change.cell != target)
    );
    let receipt = proposal
        .transfer_receipts()
        .iter()
        .find(|receipt| receipt.cell == target)
        .expect("target receipt must exist");
    assert_eq!(receipt.pre_state.get(), 80);
    assert_eq!(receipt.post_state.get(), 20);
    assert_eq!(receipt.reservoirs[0].accepted_injection.get(), 60);
}

#[test]
fn domain_preflight_failure_preserves_field_set() {
    // Given: an unchanged field set whose active region claims a missing resident neighbor.
    let field = ThermalField::new(chunk(), 3, TraceId::new(1)).expect("field must be valid");
    let fields =
        ThermalFieldSet::new(vec![field], TraceId::new(1)).expect("field set must be valid");
    let before = fields.clone();
    let missing = ChartChunkCoord::new(SpatialChartId::new(1), ChunkCoord::new(1, 0, 0));
    let active = BTreeSet::from([chunk(), missing]);
    let region = ThermalActiveRegion::new(active.clone(), active).expect("region must be valid");

    // When: preflight rejects before a causal batch can commit.
    let error = fields
        .propose_evolution(ThermalEvolutionRequest {
            tick: 1,
            parameters: ThermalParameters::new(128, THERMAL_SCALE, THERMAL_SCALE)
                .expect("parameters must be valid"),
            active_region: &region,
            boundary_behavior: ThermalBoundaryBehavior::NoFluxOutsideActiveRegion,
            reservoirs: &[],
            injections: &[],
        })
        .expect_err("missing active field must reject");

    // Then: the runtime error is specific and the authoritative source remains untouched.
    assert_eq!(
        RuntimeError::from(error),
        RuntimeError::ThermalRegionIncomplete
    );
    assert_eq!(fields, before);
}

#[test]
fn same_target_headroom() {
    // Given: two ordered reservoirs competing for five units of target-cell headroom.
    let mut energy = vec![ThermalEnergy::ZERO; 27];
    energy[13] = ThermalEnergy::new(ThermalEnergy::MAX.get() - 5)
        .expect("near-maximum energy must be valid");
    let fields = field_set(energy);
    let target = ThermalCellKey::new(chunk(), 13);
    let reservoirs = [1_u64, 2].map(|id| ThermalReservoir {
        id: ThermalReservoirId::new(id),
        target,
        budget: ThermalEnergy::new(4).expect("budget must be valid"),
        schedule: ThermalReservoirSchedule::OneShot,
        bootstrap_trace: TraceId::new(id + 1),
        last_change: TraceId::new(id + 1),
    });
    let injections = reservoirs.map(|reservoir| ThermalInjectionProposal {
        reservoir_id: reservoir.id,
        target,
        scheduled_amount: reservoir.budget,
    });
    let active = BTreeSet::from([chunk()]);
    let region = ThermalActiveRegion::new(active.clone(), active).expect("region must be valid");

    // When: canonical reservoir-ID ordering allocates combined headroom.
    let proposal = fields
        .propose_evolution(ThermalEvolutionRequest {
            tick: 1,
            parameters: parameters(1),
            active_region: &region,
            boundary_behavior: ThermalBoundaryBehavior::NoFluxOutsideActiveRegion,
            reservoirs: &reservoirs,
            injections: &injections,
        })
        .expect("headroom-capped evolution must succeed");
    let receipt = proposal
        .transfer_receipts()
        .iter()
        .find(|receipt| receipt.cell == target)
        .expect("target receipt must exist");

    // Then: the lower ID receives four units and the higher ID receives only the final unit.
    assert_eq!(receipt.reservoirs[0].accepted_injection.get(), 4);
    assert_eq!(receipt.reservoirs[0].rejected_injection.get(), 0);
    assert_eq!(receipt.reservoirs[1].accepted_injection.get(), 1);
    assert_eq!(receipt.reservoirs[1].rejected_injection.get(), 3);
    assert_eq!(proposal.conservation_receipt().residual, 0);
}
