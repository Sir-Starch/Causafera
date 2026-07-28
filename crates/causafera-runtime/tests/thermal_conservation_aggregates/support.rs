use std::collections::BTreeSet;

use causafera_core::provenance::CausalEventSnapshot;
use causafera_domains::ThermalCellKey;
use causafera_runtime::{Runtime, RuntimeConfig, RuntimeSnapshotData};
use causafera_types::TraceId;

pub(super) const FIXTURE_SEED: u64 = 2026;
pub(super) const FIXTURE_TICKS: u64 = 7;

pub(super) fn runtime_config(seed: u64) -> RuntimeConfig {
    RuntimeConfig::new(seed)
}

pub(super) fn fixture() -> RuntimeSnapshotData {
    let mut runtime =
        Runtime::new(runtime_config(FIXTURE_SEED)).expect("runtime bootstrap must succeed");
    runtime
        .run_ticks(FIXTURE_TICKS)
        .expect("fixture ticks must execute");
    runtime.export_snapshot().expect("snapshot must export")
}

pub(super) fn event_by_trace(
    snapshot: &RuntimeSnapshotData,
    trace: TraceId,
) -> Option<&CausalEventSnapshot> {
    snapshot
        .traces
        .events
        .iter()
        .find(|event| event.trace_id == trace)
}

pub(super) fn find_unbound_cell(snapshot: &RuntimeSnapshotData) -> Option<(usize, usize)> {
    let receipt_cells: BTreeSet<ThermalCellKey> = snapshot
        .thermal
        .transfer_receipts
        .iter()
        .map(|receipt| receipt.cell)
        .collect();
    let boundary_cells: BTreeSet<ThermalCellKey> = snapshot
        .thermal
        .boundary_records
        .iter()
        .map(|record| record.cell)
        .collect();
    for (field_index, field) in snapshot.thermal.field_set.fields.iter().enumerate() {
        for (cell_index, _) in field.energy.iter().enumerate() {
            let key = ThermalCellKey::new(
                field.chunk,
                u16::try_from(cell_index).expect("cell index fits u16"),
            );
            if !receipt_cells.contains(&key) && !boundary_cells.contains(&key) {
                return Some((field_index, cell_index));
            }
        }
    }
    None
}
