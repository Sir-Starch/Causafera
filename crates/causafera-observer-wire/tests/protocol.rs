use causafera_observer_api::{
    MaterialSurfaceDelta, MaterialSurfaceGateDelta, ObserverWorldSnapshot,
};
use causafera_observer_wire::{decode_world_snapshot, encode_world_snapshot};
use causafera_types::{SimulationTime, TraceId};

#[test]
fn world_snapshot_roundtrips_local_mana_and_gate_only_deltas() {
    let snapshot = ObserverWorldSnapshot {
        time: SimulationTime::new(9),
        chunks: Vec::new(),
        material_surface_delta_schema_version: 3,
        material_surface_deltas: vec![MaterialSurfaceDelta {
            chart_id: 1,
            chunk_x: 0,
            chunk_y: 0,
            chunk_z: 0,
            cell_ordinal: 2,
            before_condition: 4,
            after_condition: 5,
            mana_total: 7,
            contact_trace: Some(TraceId::new(10)),
            mana_effect_trace: Some(TraceId::new(12)),
            transition_tick: 9,
            mana_transition_trace: Some(TraceId::new(11)),
            mana_before: Some(0),
            mana_after: Some(3),
            local_mana_before: Some(0),
            local_mana_after: Some(3),
            local_mana_transition_trace_id: Some(TraceId::new(11)),
        }],
        material_surface_gate_deltas: vec![MaterialSurfaceGateDelta {
            chart_id: 1,
            chunk_x: 0,
            chunk_y: 0,
            chunk_z: 0,
            cell_ordinal: 2,
            before_active: true,
            after_active: false,
            local_mana_before: 3,
            local_mana_after: 0,
            local_mana_transition_trace_id: TraceId::new(13),
            gate_transition_trace_id: TraceId::new(14),
            contact_trace_id: None,
            transition_tick: 10,
        }],
    };

    assert_eq!(
        decode_world_snapshot(&encode_world_snapshot(&snapshot)),
        Ok(snapshot)
    );
}
