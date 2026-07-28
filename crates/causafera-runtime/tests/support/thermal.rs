use causafera_domains::{
    THERMAL_SCALE, ThermalEnergy, ThermalField, ThermalFieldSet, ThermalParameters,
};
use causafera_runtime::RuntimeSnapshotData;
use causafera_types::{ChartChunkCoord, ChunkCoord, SpatialChartId, TraceId};

pub fn chunk() -> ChartChunkCoord {
    ChartChunkCoord::new(SpatialChartId::new(1), ChunkCoord::new(0, 0, 0))
}

pub fn field_set(energy: Vec<ThermalEnergy>) -> ThermalFieldSet {
    let field = ThermalField::from_energy(chunk(), 3, energy, TraceId::new(1))
        .expect("field must be valid");
    ThermalFieldSet::new(vec![field], TraceId::new(1)).expect("field set must be valid")
}

pub fn parameters(transfer_fraction: i64) -> ThermalParameters {
    ThermalParameters::new(transfer_fraction, THERMAL_SCALE, THERMAL_SCALE, 0, 1)
        .expect("parameters must be valid")
}

/// The full three-bucket conserved total: cell energy, reservoir budgets, and material
/// surfaces' retained heat (`TODO-THERMAL-002`).
pub fn total_energy(snapshot: &RuntimeSnapshotData) -> i128 {
    snapshot
        .thermal
        .field_set
        .fields
        .iter()
        .flat_map(|field| &field.energy)
        .map(|energy| i128::from(*energy))
        .chain(
            snapshot
                .thermal
                .reservoirs
                .iter()
                .map(|reservoir| i128::from(reservoir.budget)),
        )
        .chain(
            snapshot
                .material_surfaces
                .records
                .iter()
                .map(|record| i128::from(record.surface.thermal.retained_energy.get())),
        )
        .sum()
}
