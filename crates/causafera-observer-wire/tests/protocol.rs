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
        hydrology_deltas: Vec::new(),
        hydrology_delta_schema_version: 0,
        hydrology_transfer_summaries: Vec::new(),
        hydrology_transfer_schema_version: 0,
        hydrology_conveyance_summaries: Vec::new(),
        hydrology_conveyance_schema_version: 0,
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
        hydrology_deltas: Vec::new(),
        hydrology_delta_schema_version: 0,
        hydrology_transfer_summaries: Vec::new(),
        hydrology_transfer_schema_version: 0,
        hydrology_conveyance_summaries: Vec::new(),
        hydrology_conveyance_schema_version: 0,
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
        hydrology_deltas: Vec::new(),
        hydrology_delta_schema_version: 0,
        hydrology_transfer_summaries: Vec::new(),
        hydrology_transfer_schema_version: 0,
        hydrology_conveyance_summaries: Vec::new(),
        hydrology_conveyance_schema_version: 0,
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
        hydrology_deltas: Vec::new(),
        hydrology_delta_schema_version: 0,
        hydrology_transfer_summaries: Vec::new(),
        hydrology_transfer_schema_version: 0,
        hydrology_conveyance_summaries: Vec::new(),
        hydrology_conveyance_schema_version: 0,
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
        ObserverHydrologySummary, ObserverSnapshot,
    };
    use causafera_observer_wire::{WireError, decode_observer_snapshot, encode_observer_snapshot};
    use causafera_types::{SimulationTime, TraceId};

    fn receipt(stage: u64, dependencies: Vec<TraceId>) -> ObserverBootstrapReceipt {
        ObserverBootstrapReceipt {
            stage,
            completed_at: SimulationTime::new(stage),
            // Distinct from both top-level digests, so a test that locates the
            // result by byte pattern cannot find a digest instead.
            result: [0xA0 | stage as u8; 32],
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
                stage_seven: None,
                schema_version: BOOTSTRAP_SUMMARY_SCHEMA_V1,
                plan_id: 0x0123_4567_89AB_CDEF,
                world_seed: 4_242,
                stage_count: receipts.len() as u32,
                complete: true,
                configured_population: 512,
                configured_promotion_limit: 8,
                receipts,
            },
            hydrology: ObserverHydrologySummary::default(),
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
    fn a_receipt_result_of_the_wrong_length_is_rejected() {
        // Given: a valid single-receipt payload.
        let snapshot = summary(vec![receipt(1, Vec::new())]);
        let encoded = encode_observer_snapshot(&snapshot);

        // When: the receipt's result bytes are truncated to the wrong length.
        // The pattern is the receipt's own, which no digest shares.
        let mut broken = encoded.clone();
        let position =
            find_subsequence(&broken, &[0xA1_u8; 32]).expect("receipt result bytes are present");
        assert!(
            position > find_subsequence(&broken, &[2_u8; 32]).expect("history digest is present"),
            "the located bytes must be the receipt's, not a top-level digest's"
        );
        broken[position - 1] = 31;

        // Then: it is rejected rather than accepted with a short fingerprint.
        assert!(decode_observer_snapshot(&broken).is_err());

        // And, since the name previously promised it: a receipt with the field
        // removed outright is rejected too, for each of the four it requires.
        // The specific error, not merely an error: `is_err()` would still pass
        // if the helper mangled the payload instead of removing one field.
        for field in [1_u32, 2, 3, 4] {
            assert!(
                matches!(
                    decode_observer_snapshot(&receipt_without_field(&encoded, field)),
                    Err(WireError::MissingField(missing)) if missing == field
                ),
                "a receipt with no field {field} must be rejected as missing that field"
            );
        }

        // And the helper is a no-op on a field the receipt does not carry, which
        // is what makes the assertions above about removal rather than damage.
        assert_eq!(
            decode_observer_snapshot(&receipt_without_field(&encoded, 99))
                .expect("removing an absent field must leave a valid payload"),
            snapshot
        );
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

    /// A partially-present summary is a contradiction, not an older peer.
    ///
    /// Fields 28..=35 are one atomic optional group: either the payload carries
    /// none of them, which is what a reader written before the summary existed
    /// produced, or it carries all of the scalars consistently. Everything in
    /// between fails closed instead of being filled in with zeroes.
    /// The rejection tests below rewrite payload bytes with `strip_field` and
    /// `append_varint`. If those helpers mangled the payload, every one of them
    /// would pass for the wrong reason, so this control proves a round trip
    /// through both is byte-faithful first.
    #[test]
    fn the_byte_helpers_used_by_the_rejection_tests_are_faithful() {
        let snapshot = summary(vec![receipt(1, Vec::new()), receipt(2, Vec::new())]);
        let encoded = encode_observer_snapshot(&snapshot);
        for field in 28..=34_u32 {
            let stripped = strip_field(&encoded, field);
            assert!(
                stripped.len() < encoded.len(),
                "stripping field {field} must remove bytes"
            );
            let mut rebuilt = stripped;
            append_varint(
                &mut rebuilt,
                field,
                match field {
                    28 => 1,
                    29 => 0x0123_4567_89AB_CDEF,
                    30 => 4_242,
                    31 => 2,
                    32 => 1,
                    33 => 512,
                    34 => 8,
                    _ => unreachable!(),
                },
            );
            let decoded = decode_observer_snapshot(&rebuilt)
                .unwrap_or_else(|error| panic!("rebuilt field {field} must decode: {error}"));
            assert_eq!(decoded, snapshot, "rebuilt field {field} must round trip");
        }
    }

    #[test]
    fn a_partially_present_summary_group_is_rejected() {
        let snapshot = summary(vec![receipt(1, Vec::new())]);
        let encoded = encode_observer_snapshot(&snapshot);

        // Only the schema field, with nothing behind it.
        let mut schema_only = strip_fields_from(&encoded, 28);
        append_varint(&mut schema_only, 28, 1);
        assert!(decode_observer_snapshot(&schema_only).is_err());

        // Each individual scalar missing from an otherwise complete group.
        for field in 29..=34_u32 {
            let missing = strip_field(&encoded, field);
            assert!(
                decode_observer_snapshot(&missing).is_err(),
                "a summary missing field {field} must be rejected"
            );
        }
    }

    #[test]
    fn receipts_without_a_summary_to_interpret_them_are_rejected() {
        // Given: a payload whose scalars are stripped but whose receipts remain.
        let snapshot = summary(vec![receipt(1, Vec::new())]);
        let encoded = encode_observer_snapshot(&snapshot);
        let mut orphaned = encoded.clone();
        for field in 28..=34_u32 {
            orphaned = strip_field(&orphaned, field);
        }

        // Then: they are rejected rather than parsed and silently dropped.
        assert!(decode_observer_snapshot(&orphaned).is_err());
    }

    #[test]
    fn an_unknown_summary_schema_version_is_rejected() {
        // Given: a payload declaring a schema this build does not implement.
        let snapshot = summary(vec![receipt(1, Vec::new())]);
        let encoded = encode_observer_snapshot(&snapshot);
        for version in [0_u64, 2, 99] {
            let mut tampered = strip_field(&encoded, 28);
            append_varint(&mut tampered, 28, version);

            // Then: it fails closed rather than being read as version 1.
            assert!(
                decode_observer_snapshot(&tampered).is_err(),
                "schema version {version} must be rejected"
            );
        }
    }

    #[test]
    fn a_summary_that_contradicts_its_own_receipts_is_rejected() {
        let snapshot = summary(vec![receipt(1, Vec::new()), receipt(2, Vec::new())]);
        let encoded = encode_observer_snapshot(&snapshot);

        // A stage count the receipts do not match.
        let mut miscounted = strip_field(&encoded, 31);
        append_varint(&mut miscounted, 31, 6);
        assert!(decode_observer_snapshot(&miscounted).is_err());

        // A stage count past the six-stage envelope.
        let mut oversized = strip_field(&encoded, 31);
        append_varint(&mut oversized, 31, 7);
        assert!(decode_observer_snapshot(&oversized).is_err());

        // Completeness disagreeing with the stage count, in both directions.
        let mut incomplete = strip_field(&encoded, 32);
        append_varint(&mut incomplete, 32, 0);
        assert!(decode_observer_snapshot(&incomplete).is_err());

        let empty = summary(Vec::new());
        let mut claiming_complete = encode_observer_snapshot(&empty);
        claiming_complete = strip_field(&claiming_complete, 32);
        append_varint(&mut claiming_complete, 32, 1);
        assert!(decode_observer_snapshot(&claiming_complete).is_err());

        // Completeness that is neither zero nor one.
        let mut invalid_bool = strip_field(&encoded, 32);
        append_varint(&mut invalid_bool, 32, 2);
        assert!(decode_observer_snapshot(&invalid_bool).is_err());
    }

    #[test]
    fn a_duplicated_summary_scalar_is_rejected() {
        // Given: a valid payload with one summary scalar written twice.
        let snapshot = summary(vec![receipt(1, Vec::new())]);
        for field in 28..=34_u32 {
            let mut duplicated = encode_observer_snapshot(&snapshot);
            append_varint(&mut duplicated, field, 1);

            // Then: the second value is a contradiction, not an update.
            assert!(
                decode_observer_snapshot(&duplicated).is_err(),
                "a duplicated field {field} must be rejected"
            );
        }
    }

    #[test]
    fn a_summary_absent_in_full_still_decodes_as_absent() {
        // Given: a payload with the whole group removed, receipts included.
        let snapshot = summary(vec![receipt(1, Vec::new())]);
        let legacy = strip_fields_from(&encode_observer_snapshot(&snapshot), 28);

        // Then: this is the one tolerated incompleteness.
        let decoded = decode_observer_snapshot(&legacy).expect("legacy payload must decode");
        assert_eq!(
            decoded.bootstrap.schema_version,
            BOOTSTRAP_SUMMARY_SCHEMA_ABSENT
        );
        assert!(!decoded.bootstrap.complete);
        assert_eq!(decoded.bootstrap.stage_count, 0);
        assert!(decoded.bootstrap.receipts.is_empty());
    }

    /// The receipt's own fields get the same treatment as the summary's scalars.
    /// A duplicate inside a nested message is no less a contradiction for being
    /// nested, and a known field on the wrong wire type is malformed rather than
    /// unknown.
    #[test]
    fn a_contradictory_nested_receipt_is_rejected() {
        let snapshot = summary(vec![receipt(1, Vec::new())]);
        let encoded = encode_observer_snapshot(&snapshot);
        let nested = receipt_field_bytes(&encoded);
        // Strip the field-35 key and length prefix to recover the receipt body.
        let (_, after_key) = read_varint(&nested, 0);
        let (length, body_start) = read_varint(&nested, after_key);
        let body = &nested[body_start..body_start + length as usize];

        let rebuild = |body: Vec<u8>| {
            let mut out = strip_field(&encoded, 35);
            let mut framed = Vec::new();
            write_varint(&mut framed, (35_u64 << 3) | 2);
            write_varint(&mut framed, body.len() as u64);
            framed.extend_from_slice(&body);
            out.extend_from_slice(&framed);
            out
        };

        // Control: the body round-trips unchanged through the same helpers, so
        // the rejections below are about the rules and not about mangled bytes.
        assert_eq!(
            decode_observer_snapshot(&rebuild(body.to_vec())).expect("control must decode"),
            snapshot
        );

        // A duplicated scalar.
        for field in 1..=2_u32 {
            let mut duplicated = body.to_vec();
            append_varint(&mut duplicated, field, 9);
            assert!(
                decode_observer_snapshot(&rebuild(duplicated)).is_err(),
                "a duplicated receipt field {field} must be rejected"
            );
        }

        // A duplicated result fingerprint.
        let mut duplicated_result = body.to_vec();
        write_varint(&mut duplicated_result, (3_u64 << 3) | 2);
        write_varint(&mut duplicated_result, 32);
        duplicated_result.extend_from_slice(&[9_u8; 32]);
        assert!(decode_observer_snapshot(&rebuild(duplicated_result)).is_err());

        // A known field arriving on the wrong wire type.
        let mut wrong_wire = body.to_vec();
        write_varint(&mut wrong_wire, (1_u64 << 3) | 2);
        write_varint(&mut wrong_wire, 1);
        wrong_wire.push(0);
        assert!(decode_observer_snapshot(&rebuild(wrong_wire)).is_err());
    }

    /// A summary whose scalars all arrive on the wrong wire type is malformed,
    /// not a payload from a reader that predates the summary. Skipping mistyped
    /// known fields would let it decode as "absent" and report the wrong thing
    /// about the peer.
    #[test]
    fn a_mistyped_summary_field_is_rejected_rather_than_read_as_absent() {
        let snapshot = summary(vec![receipt(1, Vec::new())]);
        let encoded = encode_observer_snapshot(&snapshot);

        for field in 28..=34_u32 {
            let mut mistyped = strip_field(&encoded, field);
            // The same value, carried as a length-delimited field instead.
            write_varint(&mut mistyped, (u64::from(field) << 3) | 2);
            write_varint(&mut mistyped, 1);
            mistyped.push(1);
            assert!(
                decode_observer_snapshot(&mistyped).is_err(),
                "a mistyped field {field} must be rejected"
            );
        }

        // And the whole group mistyped at once, which is the case that would
        // otherwise fall through to the absent schema.
        let mut all_mistyped = strip_fields_from(&encoded, 28);
        for field in 28..=34_u32 {
            write_varint(&mut all_mistyped, (u64::from(field) << 3) | 2);
            write_varint(&mut all_mistyped, 1);
            all_mistyped.push(1);
        }
        assert!(decode_observer_snapshot(&all_mistyped).is_err());
    }

    /// The Rust and TypeScript decoders are one wire contract. Rust bounds the
    /// promotion limit to 32 bits; this pins that bound so the two cannot drift
    /// into disagreeing about which payloads are valid.
    #[test]
    fn a_promotion_limit_past_thirty_two_bits_is_rejected() {
        let snapshot = summary(vec![receipt(1, Vec::new())]);
        let encoded = encode_observer_snapshot(&snapshot);

        let mut at_bound = strip_field(&encoded, 34);
        append_varint(&mut at_bound, 34, u64::from(u32::MAX));
        assert!(decode_observer_snapshot(&at_bound).is_ok());

        let mut past_bound = strip_field(&encoded, 34);
        append_varint(&mut past_bound, 34, u64::from(u32::MAX) + 1);
        assert!(decode_observer_snapshot(&past_bound).is_err());
    }

    fn append_varint(out: &mut Vec<u8>, field: u32, value: u64) {
        write_varint(out, u64::from(field) << 3);
        write_varint(out, value);
    }

    fn write_varint(out: &mut Vec<u8>, mut value: u64) {
        loop {
            let byte = (value & 0x7F) as u8;
            value >>= 7;
            if value == 0 {
                out.push(byte);
                return;
            }
            out.push(byte | 0x80);
        }
    }

    /// Drop exactly one field, leaving the rest of the payload intact.
    fn strip_field(bytes: &[u8], target: u32) -> Vec<u8> {
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
            if field != target {
                out.extend_from_slice(&bytes[start..end]);
            }
            cursor = end;
        }
        out
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

    /// The payload with one field dropped from inside its first receipt record,
    /// re-length-prefixed so the rest of the message stays well formed.
    fn receipt_without_field(bytes: &[u8], target: u32) -> Vec<u8> {
        let record = receipt_field_bytes(bytes);
        let (_, after_key) = read_varint(&record, 0);
        let (length, after_length) = read_varint(&record, after_key);
        let body = &record[after_length..after_length + length as usize];

        let stripped = strip_field(body, target);
        let mut replacement = record[..after_key].to_vec();
        write_varint(&mut replacement, stripped.len() as u64);
        replacement.extend_from_slice(&stripped);

        let at = find_subsequence(bytes, &record).expect("the receipt record is present");
        let mut out = bytes[..at].to_vec();
        out.extend_from_slice(&replacement);
        out.extend_from_slice(&bytes[at + record.len()..]);
        out
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

/// The hydrology projection's wire contract (`plans/hydrology.md` §12, V28).
///
/// Every check here is byte-level on purpose. The additive-field promise is only
/// worth what a decoder written against the old contract actually does with a new
/// payload, and "the totals survive a round trip" says nothing about whether the
/// bytes carrying them are the ones the specification names.
mod hydrology_projection {
    use causafera_observer_api::{
        BOOTSTRAP_SUMMARY_SCHEMA_V1, FieldRasterKind, HYDROLOGY_CONVEYANCE_SUMMARY_SCHEMA_V1,
        HYDROLOGY_DELTA_SCHEMA_V1, HYDROLOGY_RASTER_VALUES_SCHEMA_V1,
        HYDROLOGY_SUMMARY_SCHEMA_ABSENT, HYDROLOGY_SUMMARY_SCHEMA_V1,
        HYDROLOGY_TRANSFER_SUMMARY_SCHEMA_V1, HydrologyCellDelta, HydrologyConveyanceSummary,
        HydrologyTransferSummary, MAX_HYDROLOGY_CONVEYANCE_SUMMARIES, MAX_HYDROLOGY_DELTAS,
        MAX_HYDROLOGY_TRANSFER_SUMMARIES, MAX_QUERY_RESPONSE_PAYLOAD_BYTES, OBSERVER_PROTOCOL_V1,
        ObserverBootstrapReceipt, ObserverBootstrapSummary, ObserverFieldRaster,
        ObserverHydrologyForcing, ObserverHydrologySummary, ObserverResponse, ObserverSnapshot,
        ObserverWorldSnapshot, QueryStatus,
    };
    use causafera_observer_wire::{
        decode_field_raster, decode_observer_snapshot, decode_response, decode_world_snapshot,
        encode_field_raster, encode_observer_snapshot, encode_response, encode_world_snapshot,
    };
    use causafera_types::{SimulationTime, TraceId};

    /* ------------------------------------------------------------ fixtures -- */

    /// The 22-byte cell body every cell-shaped carrier key embeds.
    fn cell_body(chart: u64, x: i32, y: i32, z: i32, ordinal: u16) -> Vec<u8> {
        let mut out = Vec::with_capacity(22);
        out.extend_from_slice(&chart.to_be_bytes());
        out.extend_from_slice(&x.to_be_bytes());
        out.extend_from_slice(&y.to_be_bytes());
        out.extend_from_slice(&z.to_be_bytes());
        out.extend_from_slice(&ordinal.to_be_bytes());
        out
    }

    fn cell_key(ordinal: u16) -> Vec<u8> {
        let mut out = vec![0x01];
        out.extend_from_slice(&cell_body(1, 0, 0, 0, ordinal));
        out
    }

    fn edge_key(low: u16, high: u16) -> Vec<u8> {
        let mut out = vec![0x02];
        out.extend_from_slice(&cell_body(1, 0, 0, 0, low));
        out.extend_from_slice(&cell_body(1, 0, 0, 0, high));
        out
    }

    fn exterior_face_key(ordinal: u16, direction: u8) -> Vec<u8> {
        let mut out = vec![0x03];
        out.extend_from_slice(&cell_body(1, 0, 0, 0, ordinal));
        out.push(direction);
        out
    }

    fn forcing_key(scheduled_tick: u64, forcing_id: u64) -> Vec<u8> {
        let mut out = vec![0x04];
        out.extend_from_slice(&scheduled_tick.to_be_bytes());
        out.extend_from_slice(&forcing_id.to_be_bytes());
        out
    }

    /// A summary whose totals sit above `u64::MAX` and whose residual is signed,
    /// because those are the values a narrowing bug destroys.
    fn hydrology_summary() -> ObserverHydrologySummary {
        ObserverHydrologySummary {
            schema_version: HYDROLOGY_SUMMARY_SCHEMA_V1,
            total_surface: u128::from(u64::MAX) + 1,
            total_soil: 2_000,
            total_groundwater: 3_000,
            total_conveyance: 4_000,
            latest_residual: 0,
            active_chunk_count: 3,
            latest_forcing: Some(ObserverHydrologyForcing {
                tick: 7,
                forcing_id: 11,
                origin_trace: TraceId::new(4_242),
                accepted_source: u128::from(u64::MAX) + 9,
                accepted_et: u64::MAX,
            }),
        }
    }

    fn snapshot(hydrology: ObserverHydrologySummary) -> ObserverSnapshot {
        ObserverSnapshot {
            time: SimulationTime::new(12),
            digest_schema_version: 8,
            physical_digest: [3; 32],
            history_digest: [4; 32],
            mana_total: 5,
            mana_maximum: 6,
            active_chunk_count: 7,
            resolution_relevance: 8,
            resolution_level: 1,
            causal_trace_count: 9,
            actor_count: 2,
            population_total: 10,
            physical_events: 11,
            mana_cell_changes: 12,
            mana_physical_effects: 13,
            resolution_transitions: 14,
            actor_actions_committed: 15,
            actor_actions_rejected: 16,
            population_births: 17,
            population_deaths: 18,
            population_movements: 19,
            bytes_per_chunk: 20,
            latest_trace: TraceId::new(21),
            thermal_total_cell_energy: 22,
            thermal_total_reservoir_budget: 23,
            thermal_active_chunk_count: 24,
            thermal_active_cell_count: 25,
            bootstrap: ObserverBootstrapSummary {
                schema_version: BOOTSTRAP_SUMMARY_SCHEMA_V1,
                plan_id: 0xFEED,
                world_seed: 99,
                stage_count: 1,
                complete: true,
                configured_population: 64,
                configured_promotion_limit: 4,
                receipts: vec![ObserverBootstrapReceipt {
                    stage: 1,
                    completed_at: SimulationTime::new(1),
                    result: [9; 32],
                    completion_trace: TraceId::new(31),
                    dependency_traces: Vec::new(),
                }],
                stage_seven: None,
            },
            hydrology,
        }
    }

    fn cell_delta(ordinal: u16, tick: u64) -> HydrologyCellDelta {
        HydrologyCellDelta {
            chart_id: 1,
            chunk_x: -2,
            chunk_y: 3,
            chunk_z: -4,
            cell_ordinal: ordinal,
            surface_before: u64::MAX,
            surface_after: u64::MAX - 5,
            soil_before: 100,
            soil_after: 105,
            groundwater_before: 7,
            groundwater_after: 7,
            net_forcing: -9,
            net_lateral_flow: 9,
            transition_trace: TraceId::new(50 + u64::from(ordinal)),
            conservation_trace: TraceId::new(900),
            transition_tick: tick,
        }
    }

    fn transfer(
        source: Vec<u8>,
        target: Vec<u8>,
        trace: u64,
        tick: u64,
    ) -> HydrologyTransferSummary {
        HydrologyTransferSummary {
            process_kind: 7,
            source_key: source,
            target_key: target,
            requested_volume: 1_000,
            accepted_volume: 600,
            unaccepted_volume: 400,
            transfer_trace: TraceId::new(trace),
            conservation_trace: TraceId::new(900),
            tick,
            forcing_origin_trace: Some(TraceId::new(4_242)),
        }
    }

    fn conveyance(low: u16, high: u16, tick: u64) -> HydrologyConveyanceSummary {
        HydrologyConveyanceSummary {
            edge_key: edge_key(low, high),
            storage: 500,
            capacity: 1_000,
            accepted_inflow: 40,
            accepted_release: 30,
            last_change_trace: TraceId::new(901),
            tick,
        }
    }

    fn world() -> ObserverWorldSnapshot {
        ObserverWorldSnapshot {
            time: SimulationTime::new(12),
            chunks: Vec::new(),
            material_surface_delta_schema_version: 0,
            material_surface_deltas: Vec::new(),
            material_surface_gate_deltas: Vec::new(),
            material_surface_thermal_deltas: Vec::new(),
            thermal_delta_schema_version: 0,
            thermal_deltas: Vec::new(),
            hydrology_deltas: vec![cell_delta(0, 12), cell_delta(1, 12)],
            hydrology_delta_schema_version: HYDROLOGY_DELTA_SCHEMA_V1,
            hydrology_transfer_summaries: vec![
                transfer(forcing_key(7, 11), cell_key(0), 601, 12),
                // A vertical process: one cell is legitimately both endpoints.
                transfer(cell_key(0), cell_key(0), 602, 12),
                transfer(cell_key(0), exterior_face_key(0, 3), 603, 12),
            ],
            hydrology_transfer_schema_version: HYDROLOGY_TRANSFER_SUMMARY_SCHEMA_V1,
            hydrology_conveyance_summaries: vec![conveyance(0, 1, 12)],
            hydrology_conveyance_schema_version: HYDROLOGY_CONVEYANCE_SUMMARY_SCHEMA_V1,
        }
    }

    fn hydrology_raster(values: Vec<u64>) -> ObserverFieldRaster {
        ObserverFieldRaster {
            chart_id: 1,
            chunk_x: -2,
            chunk_y: 3,
            chunk_z: -4,
            field: FieldRasterKind::HydrologySurfaceWater,
            detail_level: 0,
            edge: 2,
            depth: 1,
            values: Vec::new(),
            auxiliary: Vec::new(),
            cell_traces: Vec::new(),
            generation_trace: 77,
            unsigned_values: values,
            unsigned_values_schema_version: HYDROLOGY_RASTER_VALUES_SCHEMA_V1,
        }
    }

    /* ------------------------------------------------------- round trips -- */

    #[test]
    fn a_full_hydrology_summary_roundtrips_byte_stably() {
        let expected = snapshot(hydrology_summary());
        let encoded = encode_observer_snapshot(&expected);

        let decoded = decode_observer_snapshot(&encoded).expect("canonical payload must decode");
        assert_eq!(decoded, expected);
        assert_eq!(encode_observer_snapshot(&decoded), encoded);
        // The totals that a narrowing bug would destroy survive exactly.
        assert_eq!(decoded.hydrology.total_surface, u128::from(u64::MAX) + 1);
        assert_eq!(
            decoded
                .hydrology
                .latest_forcing
                .expect("the forcing group is present")
                .accepted_et,
            u64::MAX
        );
    }

    #[test]
    fn a_summary_without_an_applied_forcing_record_omits_the_whole_group() {
        let mut summary = hydrology_summary();
        summary.latest_forcing = None;
        let expected = snapshot(summary);
        let encoded = encode_observer_snapshot(&expected);

        // Fields 43..=47 are absent as a group rather than written as zeroes,
        // which is what lets "no record has been applied" stay distinguishable
        // from "a record applied and moved nothing".
        for field in 43..=47_u32 {
            assert!(
                !contains_field(&encoded, field),
                "field {field} must be absent when no forcing has applied"
            );
        }
        assert_eq!(
            decode_observer_snapshot(&encoded).expect("must decode"),
            expected
        );
    }

    #[test]
    fn a_payload_written_before_hydrology_existed_decodes_as_absent() {
        let expected = snapshot(hydrology_summary());
        let encoded = encode_observer_snapshot(&expected);
        let legacy = strip_fields_from(&encoded, 36);

        let decoded = decode_observer_snapshot(&legacy).expect("legacy payload must decode");
        assert_eq!(
            decoded.hydrology.schema_version,
            HYDROLOGY_SUMMARY_SCHEMA_ABSENT
        );
        assert_eq!(decoded.hydrology, ObserverHydrologySummary::default());
        // Everything the older contract covers is untouched.
        assert_eq!(decoded.latest_trace, expected.latest_trace);
        assert_eq!(decoded.thermal_active_cell_count, 25);
    }

    #[test]
    fn the_bootstrap_fields_are_byte_identical_with_and_without_hydrology() {
        // Given: the same bootstrap record projected twice, once beside a
        // hydrology summary and once without one.
        let with = encode_observer_snapshot(&snapshot(hydrology_summary()));
        let without = encode_observer_snapshot(&snapshot(ObserverHydrologySummary::default()));

        // Then: fields 1..=35 are the same bytes. Hydrology is appended, so a
        // reader that stops at 35 cannot tell the two payloads apart.
        let shared = strip_fields_from(&with, 36);
        assert_eq!(shared, without);
    }

    #[test]
    fn the_world_projection_roundtrips_every_hydrology_section() {
        let expected = world();
        let encoded = encode_world_snapshot(&expected);

        let decoded = decode_world_snapshot(&encoded).expect("canonical payload must decode");
        assert_eq!(decoded, expected);
        assert_eq!(encode_world_snapshot(&decoded), encoded);
        assert_eq!(decoded.hydrology_deltas[0].surface_before, u64::MAX);
    }

    #[test]
    fn an_unsigned_raster_band_roundtrips_values_above_the_signed_ceiling() {
        // Given: a lattice holding a volume no `i64` band could carry.
        let expected = hydrology_raster(vec![0, u64::MAX, i64::MAX as u64 + 1, 42]);
        let encoded = encode_field_raster(&expected);

        let decoded = decode_field_raster(&encoded).expect("canonical raster must decode");
        assert_eq!(decoded, expected);
        assert_eq!(decoded.unsigned_values[1], u64::MAX);
        assert!(decoded.values.is_empty());
    }

    /* --------------------------------------------------------- rejections -- */

    #[test]
    fn a_partially_present_hydrology_group_is_rejected() {
        let encoded = encode_observer_snapshot(&snapshot(hydrology_summary()));
        for field in 36..=42_u32 {
            let missing = strip_field(&encoded, field);
            assert!(
                decode_observer_snapshot(&missing).is_err(),
                "a summary missing field {field} must be rejected"
            );
        }
        // The forcing subgroup is all-or-nothing in the same way.
        for field in 43..=47_u32 {
            let missing = strip_field(&encoded, field);
            assert!(
                decode_observer_snapshot(&missing).is_err(),
                "a forcing group missing field {field} must be rejected"
            );
        }
    }

    #[test]
    fn a_forcing_record_with_no_summary_to_attribute_it_to_is_rejected() {
        let encoded = encode_observer_snapshot(&snapshot(hydrology_summary()));
        let mut orphaned = encoded.clone();
        for field in 36..=42_u32 {
            orphaned = strip_field(&orphaned, field);
        }
        assert!(decode_observer_snapshot(&orphaned).is_err());
    }

    #[test]
    fn an_unknown_hydrology_summary_schema_is_rejected() {
        let encoded = encode_observer_snapshot(&snapshot(hydrology_summary()));
        for version in [0_u64, 2, 99] {
            let mut tampered = strip_field(&encoded, 36);
            append_varint(&mut tampered, 36, version);
            assert!(
                decode_observer_snapshot(&tampered).is_err(),
                "hydrology schema {version} must be rejected"
            );
        }
    }

    #[test]
    fn a_duplicated_hydrology_field_is_rejected() {
        let expected = snapshot(hydrology_summary());
        for field in [36_u32, 42, 43, 44, 45] {
            let mut duplicated = encode_observer_snapshot(&expected);
            append_varint(&mut duplicated, field, 1);
            assert!(
                decode_observer_snapshot(&duplicated).is_err(),
                "a second value for field {field} must be rejected"
            );
        }
        for field in [37_u32, 38, 39, 40, 41, 46, 47] {
            let mut duplicated = encode_observer_snapshot(&expected);
            append_bytes(&mut duplicated, field, &[1]);
            assert!(
                decode_observer_snapshot(&duplicated).is_err(),
                "a second value for field {field} must be rejected"
            );
        }
    }

    #[test]
    fn a_hydrology_field_on_the_wrong_wire_type_is_rejected() {
        let expected = snapshot(hydrology_summary());
        // A scalar arriving length-delimited, and a byte integer arriving as a
        // varint. Either one would otherwise be skipped as an unknown field and
        // the group read as partially absent.
        for field in [36_u32, 42, 43] {
            let mut tampered = strip_field(&encode_observer_snapshot(&expected), field);
            append_bytes(&mut tampered, field, &[1]);
            assert!(
                decode_observer_snapshot(&tampered).is_err(),
                "field {field} on the wrong wire type must be rejected"
            );
        }
        for field in [37_u32, 41, 46] {
            let mut tampered = strip_field(&encode_observer_snapshot(&expected), field);
            append_varint(&mut tampered, field, 1);
            assert!(
                decode_observer_snapshot(&tampered).is_err(),
                "field {field} on the wrong wire type must be rejected"
            );
        }
    }

    #[test]
    fn a_noncanonical_byte_integer_is_rejected() {
        let expected = snapshot(hydrology_summary());
        // `[0x80, 0x00]` is zero written in two bytes: a second encoding of one
        // value, which would give one projection two byte strings.
        for field in [37_u32, 38, 39, 40, 41, 46, 47] {
            let mut tampered = strip_field(&encode_observer_snapshot(&expected), field);
            append_bytes(&mut tampered, field, &[0x80, 0x00]);
            assert!(
                decode_observer_snapshot(&tampered).is_err(),
                "a noncanonical zero in field {field} must be rejected"
            );
        }
    }

    #[test]
    fn a_byte_integer_outside_its_declared_domain_is_rejected() {
        let expected = snapshot(hydrology_summary());
        // Field 47 is a `u64`; ten payload bytes describe a wider value.
        let mut too_wide = strip_field(&encode_observer_snapshot(&expected), 47);
        append_bytes(
            &mut too_wide,
            47,
            &[0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x7F],
        );
        assert!(decode_observer_snapshot(&too_wide).is_err());
    }

    #[test]
    fn every_hydrology_projection_bound_rejects_one_past_its_limit() {
        for (field, bound, build) in [
            (
                9_u32,
                MAX_HYDROLOGY_DELTAS,
                (|count: usize| {
                    let mut snapshot = world();
                    snapshot.hydrology_deltas = (0..count)
                        .map(|index| cell_delta(index as u16, 12))
                        .collect();
                    snapshot
                }) as fn(usize) -> ObserverWorldSnapshot,
            ),
            (11, MAX_HYDROLOGY_TRANSFER_SUMMARIES, |count: usize| {
                let mut snapshot = world();
                snapshot.hydrology_transfer_summaries = (0..count)
                    .map(|index| transfer(cell_key(0), cell_key(1), 600 + index as u64, 12))
                    .collect();
                snapshot
            }),
            (13, MAX_HYDROLOGY_CONVEYANCE_SUMMARIES, |count: usize| {
                let mut snapshot = world();
                snapshot.hydrology_conveyance_summaries = (0..count)
                    .map(|index| conveyance(index as u16, index as u16 + 100, 12))
                    .collect();
                snapshot
            }),
        ] {
            // At the limit the projection decodes; the producer's own `take`
            // would hide anything past it, so the excess entry is appended to
            // already-encoded bytes.
            let at_limit = encode_world_snapshot(&build(bound));
            assert!(
                decode_world_snapshot(&at_limit).is_ok(),
                "a projection at the bound must decode"
            );
            let over_limit = encode_world_snapshot(&build(bound + 1));
            assert_eq!(
                repeated_field_count(&over_limit, field),
                bound,
                "the producer caps what it writes for field {field}"
            );
        }
    }

    #[test]
    fn a_decoder_rejects_one_entry_past_every_bound() {
        // Given: payloads with one entry too many, built by appending to bytes
        // the producer already capped.
        let deltas = {
            let mut snapshot = world();
            snapshot.hydrology_deltas = (0..MAX_HYDROLOGY_DELTAS)
                .map(|index| cell_delta(index as u16, 12))
                .collect();
            let mut bytes = encode_world_snapshot(&snapshot);
            let extra = nested_field_bytes(&bytes, 9);
            append_bytes(&mut bytes, 9, &extra);
            bytes
        };
        assert!(decode_world_snapshot(&deltas).is_err());

        let transfers = {
            let mut snapshot = world();
            snapshot.hydrology_transfer_summaries = (0..MAX_HYDROLOGY_TRANSFER_SUMMARIES)
                .map(|index| transfer(cell_key(0), cell_key(1), 600 + index as u64, 12))
                .collect();
            let mut bytes = encode_world_snapshot(&snapshot);
            let extra = nested_field_bytes(&bytes, 11);
            append_bytes(&mut bytes, 11, &extra);
            bytes
        };
        assert!(decode_world_snapshot(&transfers).is_err());

        let conveyance_bytes = {
            let mut snapshot = world();
            snapshot.hydrology_conveyance_summaries = (0..MAX_HYDROLOGY_CONVEYANCE_SUMMARIES)
                .map(|index| conveyance(index as u16, index as u16 + 100, 12))
                .collect();
            let mut bytes = encode_world_snapshot(&snapshot);
            let extra = nested_field_bytes(&bytes, 13);
            append_bytes(&mut bytes, 13, &extra);
            bytes
        };
        assert!(decode_world_snapshot(&conveyance_bytes).is_err());
    }

    #[test]
    fn entries_without_a_schema_to_interpret_them_are_rejected() {
        for (entry_field, schema_field) in [(9_u32, 10_u32), (11, 12), (13, 14)] {
            let stripped = strip_field(&encode_world_snapshot(&world()), schema_field);
            assert!(
                decode_world_snapshot(&stripped).is_err(),
                "field {entry_field} without field {schema_field} must be rejected"
            );
        }
    }

    #[test]
    fn a_duplicated_hydrology_key_is_rejected() {
        // Two deltas for one cell in one tick disagree about what it did.
        let mut duplicated = world();
        duplicated.hydrology_deltas = vec![cell_delta(0, 12), cell_delta(0, 12)];
        assert!(decode_world_snapshot(&encode_world_snapshot(&duplicated)).is_err());

        // Two summaries under one canonical transfer key count one movement
        // twice.
        let mut twice = world();
        twice.hydrology_transfer_summaries = vec![
            transfer(cell_key(0), cell_key(1), 601, 12),
            transfer(cell_key(0), cell_key(1), 601, 12),
        ];
        assert!(decode_world_snapshot(&encode_world_snapshot(&twice)).is_err());

        let mut edges = world();
        edges.hydrology_conveyance_summaries = vec![conveyance(0, 1, 12), conveyance(0, 1, 12)];
        assert!(decode_world_snapshot(&encode_world_snapshot(&edges)).is_err());

        // The same key at a different tick is a different row, not a duplicate.
        let mut across_ticks = world();
        across_ticks.hydrology_deltas = vec![cell_delta(0, 12), cell_delta(0, 11)];
        assert!(decode_world_snapshot(&encode_world_snapshot(&across_ticks)).is_ok());
    }

    #[test]
    fn a_malformed_carrier_key_is_rejected() {
        let cases: Vec<(&str, Vec<u8>)> = vec![
            ("an unknown variant", vec![0x09; 23]),
            ("an empty key", Vec::new()),
            ("a cell key one byte short", cell_key(0)[..22].to_vec()),
            ("a cell key one byte long", {
                let mut bytes = cell_key(0);
                bytes.push(0);
                bytes
            }),
            ("an unknown face direction", exterior_face_key(0, 4)),
            ("a reversed edge", edge_key(3, 1)),
            ("an edge from a cell to itself", edge_key(2, 2)),
        ];
        for (what, key) in cases {
            let mut snapshot = world();
            snapshot.hydrology_transfer_summaries = vec![transfer(key, forcing_key(1, 1), 601, 12)];
            assert!(
                decode_world_snapshot(&encode_world_snapshot(&snapshot)).is_err(),
                "{what} must be rejected"
            );
        }
    }

    #[test]
    fn a_transfer_between_one_non_cell_carrier_and_itself_is_rejected() {
        // A cell may legitimately be both endpoints — infiltration moves water
        // between buckets inside one cell — and every other carrier may not.
        let mut vertical = world();
        vertical.hydrology_transfer_summaries = vec![transfer(cell_key(0), cell_key(0), 601, 12)];
        assert!(decode_world_snapshot(&encode_world_snapshot(&vertical)).is_ok());

        for key in [edge_key(0, 1), exterior_face_key(0, 2), forcing_key(3, 4)] {
            let mut snapshot = world();
            snapshot.hydrology_transfer_summaries = vec![transfer(key.clone(), key, 601, 12)];
            assert!(decode_world_snapshot(&encode_world_snapshot(&snapshot)).is_err());
        }
    }

    #[test]
    fn a_transfer_whose_volumes_do_not_close_is_rejected() {
        let mut invented = world();
        let mut summary = transfer(cell_key(0), cell_key(1), 601, 12);
        summary.accepted_volume = summary.requested_volume + 1;
        invented.hydrology_transfer_summaries = vec![summary];
        assert!(decode_world_snapshot(&encode_world_snapshot(&invented)).is_err());

        let mut lost = world();
        let mut summary = transfer(cell_key(0), cell_key(1), 601, 12);
        summary.unaccepted_volume += 1;
        lost.hydrology_transfer_summaries = vec![summary];
        assert!(decode_world_snapshot(&encode_world_snapshot(&lost)).is_err());
    }

    #[test]
    fn a_conveyance_summary_that_does_not_name_an_edge_is_rejected() {
        let mut snapshot = world();
        snapshot.hydrology_conveyance_summaries = vec![HydrologyConveyanceSummary {
            edge_key: cell_key(0),
            ..conveyance(0, 1, 12)
        }];
        assert!(decode_world_snapshot(&encode_world_snapshot(&snapshot)).is_err());
    }

    #[test]
    fn the_two_raster_bands_are_mutually_exclusive() {
        // A hydrology raster carrying the signed band would have wrapped every
        // volume above `i64::MAX` on the way in.
        let mut signed = hydrology_raster(vec![1, 2, 3, 4]);
        signed.values = vec![1, 2, 3, 4];
        assert!(decode_field_raster(&encode_field_raster(&signed)).is_err());

        // A terrain raster carrying the unsigned band would have lost every
        // elevation below sea level.
        let mut terrain = hydrology_raster(vec![1, 2, 3, 4]);
        terrain.field = FieldRasterKind::TerrainElevation;
        terrain.values = vec![-1, 2, -3, 4];
        assert!(decode_field_raster(&encode_field_raster(&terrain)).is_err());
    }

    #[test]
    fn a_hydrology_raster_without_its_schema_marker_is_rejected() {
        let raster = hydrology_raster(vec![1, 2, 3, 4]);
        let stripped = strip_field(&encode_field_raster(&raster), 14);
        assert!(decode_field_raster(&stripped).is_err());

        let mut wrong_version = strip_field(&encode_field_raster(&raster), 14);
        append_varint(&mut wrong_version, 14, 2);
        assert!(decode_field_raster(&wrong_version).is_err());
    }

    #[test]
    fn a_hydrology_raster_whose_band_does_not_fill_its_lattice_is_rejected() {
        let short = hydrology_raster(vec![1, 2, 3]);
        assert!(decode_field_raster(&encode_field_raster(&short)).is_err());

        let long = hydrology_raster(vec![1, 2, 3, 4, 5]);
        assert!(decode_field_raster(&encode_field_raster(&long)).is_err());
    }

    #[test]
    fn a_noncanonical_value_in_the_unsigned_band_is_rejected() {
        let raster = hydrology_raster(vec![1, 2, 3, 4]);
        let mut tampered = strip_field(&encode_field_raster(&raster), 13);
        // Four values, the last of them zero written in two bytes.
        append_bytes(&mut tampered, 13, &[1, 2, 3, 0x80, 0x00]);
        assert!(decode_field_raster(&tampered).is_err());
    }

    #[test]
    fn the_response_cap_is_enforced_on_both_sides() {
        let oversized = ObserverResponse {
            request_id: 1,
            protocol_version: OBSERVER_PROTOCOL_V1,
            status: QueryStatus::Ok,
            payload: vec![0; MAX_QUERY_RESPONSE_PAYLOAD_BYTES + 1],
        };
        // The producer refuses to emit it at all.
        assert!(encode_response(&oversized).is_err());

        // And a peer that receives one anyway refuses before allocating it.
        let mut forged = Vec::new();
        append_varint(&mut forged, 1, 1);
        append_varint(&mut forged, 2, u64::from(OBSERVER_PROTOCOL_V1));
        append_varint(&mut forged, 3, 1);
        append_bytes(&mut forged, 4, &oversized.payload);
        assert!(decode_response(&forged).is_err());

        let at_cap = ObserverResponse {
            payload: vec![0; MAX_QUERY_RESPONSE_PAYLOAD_BYTES],
            ..oversized
        };
        let encoded = encode_response(&at_cap).expect("a response at the cap must encode");
        assert_eq!(decode_response(&encoded).expect("must decode"), at_cap);
    }

    /* ------------------------------------------------------- byte helpers -- */

    fn append_varint(out: &mut Vec<u8>, field: u32, value: u64) {
        write_varint(out, u64::from(field) << 3);
        write_varint(out, value);
    }

    fn append_bytes(out: &mut Vec<u8>, field: u32, value: &[u8]) {
        write_varint(out, (u64::from(field) << 3) | 2);
        write_varint(out, value.len() as u64);
        out.extend_from_slice(value);
    }

    fn write_varint(out: &mut Vec<u8>, mut value: u64) {
        loop {
            let byte = (value & 0x7F) as u8;
            value >>= 7;
            if value == 0 {
                out.push(byte);
                return;
            }
            out.push(byte | 0x80);
        }
    }

    fn read_varint(bytes: &[u8], mut at: usize) -> (u64, usize) {
        let mut value = 0_u64;
        let mut shift = 0;
        loop {
            let byte = bytes[at];
            at += 1;
            value |= u64::from(byte & 0x7F) << shift;
            if byte & 0x80 == 0 {
                return (value, at);
            }
            shift += 7;
        }
    }

    /// Walk the payload's fields, yielding `(field, wire, start, end)`.
    fn fields(bytes: &[u8]) -> Vec<(u32, u8, usize, usize)> {
        let mut out = Vec::new();
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
            out.push((field, wire, start, end));
            cursor = end;
        }
        out
    }

    fn strip_field(bytes: &[u8], target: u32) -> Vec<u8> {
        let mut out = Vec::with_capacity(bytes.len());
        for (field, _, start, end) in fields(bytes) {
            if field != target {
                out.extend_from_slice(&bytes[start..end]);
            }
        }
        out
    }

    fn strip_fields_from(bytes: &[u8], from: u32) -> Vec<u8> {
        let mut out = Vec::with_capacity(bytes.len());
        for (field, _, start, end) in fields(bytes) {
            if field < from {
                out.extend_from_slice(&bytes[start..end]);
            }
        }
        out
    }

    fn contains_field(bytes: &[u8], target: u32) -> bool {
        fields(bytes).iter().any(|(field, ..)| *field == target)
    }

    fn repeated_field_count(bytes: &[u8], target: u32) -> usize {
        fields(bytes)
            .iter()
            .filter(|(field, ..)| *field == target)
            .count()
    }

    /// The body of the first occurrence of a length-delimited field.
    fn nested_field_bytes(bytes: &[u8], target: u32) -> Vec<u8> {
        for (field, wire, start, end) in fields(bytes) {
            if field == target && wire == 2 {
                let (_, after_key) = read_varint(bytes, start);
                let (_, after_len) = read_varint(bytes, after_key);
                return bytes[after_len..end].to_vec();
            }
        }
        panic!("field {target} is not present");
    }
}
