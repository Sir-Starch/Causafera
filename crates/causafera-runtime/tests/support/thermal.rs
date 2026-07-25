use causafera_domains::{
    THERMAL_SCALE, ThermalEnergy, ThermalField, ThermalFieldSet, ThermalParameters,
};
use causafera_runtime::ThermalSnapshot;
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
    ThermalParameters::new(transfer_fraction, THERMAL_SCALE, THERMAL_SCALE)
        .expect("parameters must be valid")
}

pub fn total_energy(snapshot: &ThermalSnapshot) -> i128 {
    snapshot
        .field_set
        .fields
        .iter()
        .flat_map(|field| &field.energy)
        .map(|energy| i128::from(*energy))
        .chain(
            snapshot
                .reservoirs
                .iter()
                .map(|reservoir| i128::from(reservoir.budget)),
        )
        .sum()
}
