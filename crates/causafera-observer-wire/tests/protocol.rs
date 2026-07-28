use causafera_observer_api::{
    MaterialSurfaceDelta, MaterialSurfaceGateDelta, ObserverWorldSnapshot, ThermalFieldDelta,
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
        material_surface_thermal_deltas: Vec::new(),
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
        thermal_delta_schema_version: 0,
        thermal_deltas: Vec::new(),
    };

    assert_eq!(
        decode_world_snapshot(&encode_world_snapshot(&snapshot)),
        Ok(snapshot)
    );
}

#[test]
fn v2_world_snapshot_omits_gate_only_deltas() {
    let snapshot = ObserverWorldSnapshot {
        time: SimulationTime::new(9),
        chunks: Vec::new(),
        material_surface_delta_schema_version: 2,
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
            local_mana_before: None,
            local_mana_after: None,
            local_mana_transition_trace_id: None,
        }],
        material_surface_thermal_deltas: Vec::new(),
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
        thermal_delta_schema_version: 0,
        thermal_deltas: Vec::new(),
    };

    let decoded = decode_world_snapshot(&encode_world_snapshot(&snapshot)).unwrap();
    assert_eq!(decoded.material_surface_delta_schema_version, 2);
    assert!(decoded.material_surface_gate_deltas.is_empty());
    assert_eq!(
        decoded.material_surface_deltas,
        snapshot.material_surface_deltas
    );
}

#[test]
fn thermal_roundtrip_preserves_signed_receipt_summaries() {
    // Given: a bounded thermal receipt projection with signed cell and face values.
    let snapshot = ObserverWorldSnapshot {
        time: SimulationTime::new(12),
        chunks: Vec::new(),
        material_surface_delta_schema_version: 0,
        material_surface_deltas: Vec::new(),
        material_surface_thermal_deltas: Vec::new(),
        material_surface_gate_deltas: Vec::new(),
        thermal_delta_schema_version: 1,
        thermal_deltas: vec![ThermalFieldDelta {
            chart_id: 7,
            chunk_x: -1,
            chunk_y: 2,
            chunk_z: -3,
            cell_ordinal: 4,
            pre_state_energy: 1_000,
            post_state_energy: 970,
            reservoir_scheduled_injection: 80,
            reservoir_accepted_injection: 60,
            reservoir_rejected_injection: 20,
            net_face_flux: -90,
            face_count: 3,
        }],
    };

    // When: the observer world projection crosses the Rust protobuf codec.
    let encoded = encode_world_snapshot(&snapshot);

    // Then: signed summaries and the thermal schema survive exactly.
    assert_eq!(decode_world_snapshot(&encoded), Ok(snapshot));
}

#[test]
fn thermal_roundtrip_caps_deltas_at_sixty_four() {
    let delta = ThermalFieldDelta {
        chart_id: 7,
        chunk_x: 0,
        chunk_y: 0,
        chunk_z: 0,
        cell_ordinal: 4,
        pre_state_energy: 1,
        post_state_energy: 1,
        reservoir_scheduled_injection: 0,
        reservoir_accepted_injection: 0,
        reservoir_rejected_injection: 0,
        net_face_flux: 0,
        face_count: 0,
    };
    let snapshot = ObserverWorldSnapshot {
        time: SimulationTime::new(12),
        chunks: Vec::new(),
        material_surface_delta_schema_version: 0,
        material_surface_deltas: Vec::new(),
        material_surface_thermal_deltas: Vec::new(),
        material_surface_gate_deltas: Vec::new(),
        thermal_delta_schema_version: 1,
        thermal_deltas: vec![delta; 65],
    };

    let decoded = decode_world_snapshot(&encode_world_snapshot(&snapshot)).unwrap();

    assert_eq!(decoded.thermal_deltas.len(), 64);
}
