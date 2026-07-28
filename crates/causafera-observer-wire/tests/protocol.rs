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

/// The bounded bootstrap summary rides the existing runtime-summary payload as
/// additive fields, so these tests are about two things at once: that the new
/// fields survive a roundtrip in canonical order, and that a reader written
/// against fields 1..=27 is unaffected by them.
mod bootstrap_summary {
    use causafera_observer_api::{
        BOOTSTRAP_SUMMARY_SCHEMA_ABSENT, BOOTSTRAP_SUMMARY_SCHEMA_V1,
        MAX_BOOTSTRAP_RECEIPT_SUMMARIES, ObserverBootstrapReceipt, ObserverBootstrapSummary,
        ObserverSnapshot,
    };
    use causafera_observer_wire::{decode_observer_snapshot, encode_observer_snapshot};
    use causafera_types::{SimulationTime, TraceId};

    fn receipt(stage: u64, dependencies: Vec<TraceId>) -> ObserverBootstrapReceipt {
        ObserverBootstrapReceipt {
            stage,
            completed_at: SimulationTime::new(stage),
            result: [stage as u8; 32],
            completion_trace: TraceId::new(100 + stage),
            dependency_traces: dependencies,
        }
    }

    fn summary(receipts: Vec<ObserverBootstrapReceipt>) -> ObserverSnapshot {
        ObserverSnapshot {
            time: SimulationTime::new(3),
            digest_schema_version: 7,
            physical_digest: [1; 32],
            history_digest: [2; 32],
            mana_total: 0,
            mana_maximum: 0,
            active_chunk_count: 9,
            resolution_relevance: 0,
            resolution_level: 0,
            causal_trace_count: 40,
            actor_count: 8,
            population_total: 504,
            physical_events: 0,
            mana_cell_changes: 0,
            mana_physical_effects: 0,
            resolution_transitions: 0,
            actor_actions_committed: 0,
            actor_actions_rejected: 0,
            population_births: 0,
            population_deaths: 0,
            population_movements: 0,
            bytes_per_chunk: 0,
            latest_trace: TraceId::new(39),
            thermal_total_cell_energy: 0,
            thermal_total_reservoir_budget: 0,
            thermal_active_chunk_count: 9,
            thermal_active_cell_count: 243,
            bootstrap: ObserverBootstrapSummary {
                schema_version: BOOTSTRAP_SUMMARY_SCHEMA_V1,
                plan_id: 0x0123_4567_89AB_CDEF,
                world_seed: 4_242,
                stage_count: receipts.len() as u32,
                complete: true,
                configured_population: 512,
                configured_promotion_limit: 8,
                receipts,
            },
        }
    }

    #[test]
    fn a_six_receipt_summary_roundtrips_with_byte_stable_encoding() {
        // Given: a full six-stage record.
        let snapshot = summary(vec![
            receipt(1, Vec::new()),
            receipt(2, vec![TraceId::new(101)]),
            receipt(3, vec![TraceId::new(102)]),
            receipt(4, vec![TraceId::new(103)]),
            receipt(5, vec![TraceId::new(104)]),
            receipt(6, vec![TraceId::new(105)]),
        ]);
        let encoded = encode_observer_snapshot(&snapshot);

        // Then: it decodes back unchanged and re-encodes to the same bytes.
        let decoded = decode_observer_snapshot(&encoded).expect("canonical payload must decode");
        assert_eq!(decoded, snapshot);
        assert_eq!(encode_observer_snapshot(&decoded), encoded);
    }

    #[test]
    fn a_payload_written_before_the_summary_existed_still_decodes() {
        // Given: a payload carrying only fields 1..=27, as a reader written
        // before the bootstrap summary would have produced.
        let snapshot = summary(vec![receipt(1, Vec::new())]);
        let encoded = encode_observer_snapshot(&snapshot);
        let legacy = strip_fields_from(&encoded, 28);

        // Then: the existing fields keep their meaning and the summary reports
        // itself absent rather than claiming an empty record.
        let decoded = decode_observer_snapshot(&legacy).expect("legacy payload must decode");
        assert_eq!(decoded.actor_count, snapshot.actor_count);
        assert_eq!(decoded.latest_trace, snapshot.latest_trace);
        assert_eq!(
            decoded.bootstrap.schema_version,
            BOOTSTRAP_SUMMARY_SCHEMA_ABSENT
        );
        assert!(!decoded.bootstrap.complete);
        assert!(decoded.bootstrap.receipts.is_empty());
    }

    #[test]
    fn more_receipts_than_the_current_bootstrap_can_produce_are_rejected() {
        // Given: a payload with one receipt more than the six-stage envelope.
        let receipts = (1..=MAX_BOOTSTRAP_RECEIPT_SUMMARIES as u64 + 1)
            .map(|stage| receipt(stage, Vec::new()))
            .collect::<Vec<_>>();
        let mut snapshot = summary(receipts.clone());
        snapshot.bootstrap.stage_count = receipts.len() as u32;

        // The encoder caps what it writes, so the over-long payload is built by
        // appending one more receipt to a full one.
        let mut encoded = encode_observer_snapshot(&snapshot);
        let mut extra = summary(vec![receipt(7, Vec::new())]);
        extra.bootstrap.stage_count = 1;
        let tail = encode_observer_snapshot(&extra);
        encoded.extend_from_slice(&receipt_field_bytes(&tail));

        // Then: it is rejected before the list grows.
        assert!(decode_observer_snapshot(&encoded).is_err());
    }

    #[test]
    fn an_out_of_order_receipt_list_is_rejected() {
        // Given: two receipts written with descending stage identity.
        let snapshot = summary(vec![receipt(2, Vec::new()), receipt(1, Vec::new())]);
        let encoded = encode_observer_snapshot(&snapshot);

        // Then: the decoder rejects the non-canonical order.
        assert!(decode_observer_snapshot(&encoded).is_err());
    }

    #[test]
    fn a_receipt_missing_its_result_or_trace_is_rejected() {
        // Given: a valid single-receipt payload.
        let snapshot = summary(vec![receipt(1, Vec::new())]);
        let encoded = encode_observer_snapshot(&snapshot);

        // When: the receipt's result bytes are truncated to the wrong length.
        let mut broken = encoded.clone();
        let position = find_subsequence(&broken, &[1_u8; 32]).expect("result bytes are present");
        broken[position - 1] = 31;

        // Then: it is rejected rather than accepted with a short fingerprint.
        assert!(decode_observer_snapshot(&broken).is_err());
    }

    #[test]
    fn an_unknown_trailing_field_is_skipped_rather_than_failing() {
        // Given: a valid payload plus a field this build does not know.
        let snapshot = summary(vec![receipt(1, Vec::new())]);
        let mut encoded = encode_observer_snapshot(&snapshot);
        // Field 200, varint wire type, value 1.
        encoded.extend_from_slice(&[0xC0, 0x0C, 0x01]);

        // Then: the known fields still decode unchanged.
        let decoded = decode_observer_snapshot(&encoded).expect("unknown fields must be skipped");
        assert_eq!(decoded, snapshot);
    }

    /// Drop every field whose number is `>= from`, leaving a payload as an older
    /// reader's peer would have written it.
    fn strip_fields_from(bytes: &[u8], from: u32) -> Vec<u8> {
        let mut out = Vec::with_capacity(bytes.len());
        let mut cursor = 0_usize;
        while cursor < bytes.len() {
            let start = cursor;
            let (key, next) = read_varint(bytes, cursor);
            cursor = next;
            let field = (key >> 3) as u32;
            let wire = (key & 7) as u8;
            let end = match wire {
                0 => read_varint(bytes, cursor).1,
                2 => {
                    let (length, after) = read_varint(bytes, cursor);
                    after + length as usize
                }
                other => panic!("unexpected wire type {other}"),
            };
            if field < from {
                out.extend_from_slice(&bytes[start..end]);
            }
            cursor = end;
        }
        out
    }

    /// The bytes of the first field-35 record in `bytes`.
    fn receipt_field_bytes(bytes: &[u8]) -> Vec<u8> {
        let mut cursor = 0_usize;
        while cursor < bytes.len() {
            let start = cursor;
            let (key, next) = read_varint(bytes, cursor);
            cursor = next;
            let field = (key >> 3) as u32;
            let wire = (key & 7) as u8;
            let end = match wire {
                0 => read_varint(bytes, cursor).1,
                2 => {
                    let (length, after) = read_varint(bytes, cursor);
                    after + length as usize
                }
                other => panic!("unexpected wire type {other}"),
            };
            if field == 35 {
                return bytes[start..end].to_vec();
            }
            cursor = end;
        }
        panic!("payload carries no receipt field");
    }

    fn read_varint(bytes: &[u8], mut cursor: usize) -> (u64, usize) {
        let mut value = 0_u64;
        let mut shift = 0_u32;
        loop {
            let byte = bytes[cursor];
            cursor += 1;
            value |= u64::from(byte & 0x7F) << shift;
            if byte & 0x80 == 0 {
                return (value, cursor);
            }
            shift += 7;
        }
    }

    fn find_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
        haystack
            .windows(needle.len())
            .position(|window| window == needle)
    }
}
