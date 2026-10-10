use std::collections::BTreeMap;

use causafera_core::{
    CausalEventSnapshot, CausalTraceSnapshot, HistoryAnchor, HistoryQuery, HistoryReader,
    HistoryRecord, MAX_QUERY_PAGE_BYTES, MAX_QUERY_PAGE_RECORDS,
};
use causafera_persistence::{
    FORMAT_MAJOR_V1, FORMAT_MINOR_V1, HISTORY_ANCHOR_ENCODED_LEN, PersistenceError, SectionPayload,
    SnapshotEnvelope, SnapshotHeader, encode_history_anchor,
};

use crate::RuntimeSnapshotData;
use crate::snapshot_sections::{
    CURRENT_SECTION_MINOR, EXPERIMENT_RECIPE_MANA_SOURCE_RECEIPTS_SECTION_MAJOR,
    HYDROLOGY_SECTION_ID, HYDROLOGY_SECTION_MAJOR, MANA_SECTION_MAJOR, MATERIAL_SURFACE_SECTION_ID,
    MATERIAL_SURFACE_SECTION_MAJOR, PHYSICAL_COUNTERS_SECTION_MAJOR,
    POPULATION_BOOTSTRAP_SECTION_MAJOR, RUNTIME_RECIPE_SECTION_MAJOR, SECTION_ACTOR_OBJECTIVE,
    SECTION_ACTOR_SUBJECTIVE, SECTION_EXPERIMENT_MANIFEST,
    SECTION_EXPERIMENT_RECIPE_MANA_SOURCE_RECEIPTS, SECTION_MANA_FIELDS, SECTION_PATTERN_HISTORY,
    SECTION_PHYSICAL_COUNTERS, SECTION_POPULATION_BOOTSTRAP, SECTION_RESOLUTION_FIELD,
    SECTION_RUNTIME_RECIPE, SECTION_SPATIAL_CHUNKS, THERMAL_SECTION_ID, THERMAL_SECTION_MAJOR,
    decode_runtime_recipe_section, disassemble_envelope_with_traces,
    encode_actor_objective_section, encode_actor_subjective_section,
    encode_experiment_manifest_section, encode_experiment_recipe_mana_source_receipts_section,
    encode_hydrology_section, encode_mana_section, encode_material_surface_section,
    encode_pattern_history_section, encode_physical_counters_section, encode_population_section,
    encode_resolution_section, encode_runtime_recipe_section, encode_spatial_section,
    encode_thermal_section,
};

// allow: SIZE_OK — revision-2 bounded snapshot encode/decode is one versioned codec boundary.

pub const SECTION_HISTORY_ANCHOR: u16 = 0x0010;
pub const HISTORY_ANCHOR_SECTION_MAJOR: u16 = 1;
pub const BOUNDED_ENVELOPE_CODEC_REVISION: u32 = 2;
pub const PHYSICAL_DIGEST_SCHEMA_V9: u16 = causafera_persistence::CURRENT_DIGEST_SCHEMA_VERSION;

const BOUNDED_STATE_SECTIONS: [(u16, u16); 13] = [
    (SECTION_RUNTIME_RECIPE, RUNTIME_RECIPE_SECTION_MAJOR),
    (SECTION_SPATIAL_CHUNKS, 1),
    (SECTION_MANA_FIELDS, MANA_SECTION_MAJOR),
    (SECTION_RESOLUTION_FIELD, 1),
    (SECTION_PATTERN_HISTORY, 1),
    (SECTION_PHYSICAL_COUNTERS, PHYSICAL_COUNTERS_SECTION_MAJOR),
    (SECTION_ACTOR_OBJECTIVE, 1),
    (SECTION_ACTOR_SUBJECTIVE, 1),
    (
        SECTION_POPULATION_BOOTSTRAP,
        POPULATION_BOOTSTRAP_SECTION_MAJOR,
    ),
    (MATERIAL_SURFACE_SECTION_ID, MATERIAL_SURFACE_SECTION_MAJOR),
    (
        SECTION_EXPERIMENT_RECIPE_MANA_SOURCE_RECEIPTS,
        EXPERIMENT_RECIPE_MANA_SOURCE_RECEIPTS_SECTION_MAJOR,
    ),
    (THERMAL_SECTION_ID, THERMAL_SECTION_MAJOR),
    (HYDROLOGY_SECTION_ID, HYDROLOGY_SECTION_MAJOR),
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BoundedSnapshotMetadata {
    pub runtime_recipe_fingerprint: [u8; 32],
    pub physical_digest: [u8; 32],
    pub anchor: HistoryAnchor,
}

pub fn assemble_bounded_envelope(
    data: &RuntimeSnapshotData,
    metadata: &BoundedSnapshotMetadata,
) -> Result<SnapshotEnvelope, PersistenceError> {
    validate_metadata(data, metadata)?;
    let mut sections = BTreeMap::new();
    insert(
        &mut sections,
        SECTION_RUNTIME_RECIPE,
        RUNTIME_RECIPE_SECTION_MAJOR,
        encode_runtime_recipe_section(&data.recipe),
    );
    insert(
        &mut sections,
        SECTION_SPATIAL_CHUNKS,
        1,
        encode_spatial_section(&data.spatial),
    );
    insert(
        &mut sections,
        SECTION_MANA_FIELDS,
        MANA_SECTION_MAJOR,
        encode_mana_section(&data.mana),
    );
    insert(
        &mut sections,
        THERMAL_SECTION_ID,
        THERMAL_SECTION_MAJOR,
        encode_thermal_section(&data.thermal),
    );
    insert(
        &mut sections,
        HYDROLOGY_SECTION_ID,
        HYDROLOGY_SECTION_MAJOR,
        encode_hydrology_section(&data.hydrology),
    );
    insert(
        &mut sections,
        SECTION_RESOLUTION_FIELD,
        1,
        encode_resolution_section(&data.resolution, &data.resolution_policy),
    );
    insert(
        &mut sections,
        SECTION_PATTERN_HISTORY,
        1,
        encode_pattern_history_section(&data.pattern_history),
    );
    insert(
        &mut sections,
        SECTION_PHYSICAL_COUNTERS,
        PHYSICAL_COUNTERS_SECTION_MAJOR,
        encode_physical_counters_section(&data.physical_counters),
    );
    insert(
        &mut sections,
        MATERIAL_SURFACE_SECTION_ID,
        MATERIAL_SURFACE_SECTION_MAJOR,
        encode_material_surface_section(&data.material_surfaces),
    );
    insert(
        &mut sections,
        SECTION_ACTOR_OBJECTIVE,
        1,
        encode_actor_objective_section(&data.actors_objective),
    );
    insert(
        &mut sections,
        SECTION_ACTOR_SUBJECTIVE,
        1,
        encode_actor_subjective_section(&data.actors_subjective),
    );
    insert(
        &mut sections,
        SECTION_POPULATION_BOOTSTRAP,
        POPULATION_BOOTSTRAP_SECTION_MAJOR,
        encode_population_section(&data.population, &data.bootstrap),
    );
    insert(
        &mut sections,
        SECTION_EXPERIMENT_RECIPE_MANA_SOURCE_RECEIPTS,
        EXPERIMENT_RECIPE_MANA_SOURCE_RECEIPTS_SECTION_MAJOR,
        encode_experiment_recipe_mana_source_receipts_section(
            &data.experiment_recipe_mana_source_receipts,
        ),
    );
    if let Some(manifest) = &data.experiment_manifest {
        insert(
            &mut sections,
            SECTION_EXPERIMENT_MANIFEST,
            1,
            encode_experiment_manifest_section(manifest),
        );
    }
    sections.insert(
        u64::from(SECTION_HISTORY_ANCHOR),
        SectionPayload {
            section_major: HISTORY_ANCHOR_SECTION_MAJOR,
            section_minor: CURRENT_SECTION_MINOR,
            flags: 0,
            decoded_size_limit: u64::try_from(HISTORY_ANCHOR_ENCODED_LEN)
                .map_err(|_| PersistenceError::codec("history anchor length does not fit u64"))?,
            bytes: encode_history_anchor(&metadata.anchor)?,
        },
    );

    Ok(SnapshotEnvelope::new(
        SnapshotHeader {
            format_major: FORMAT_MAJOR_V1,
            format_minor: FORMAT_MINOR_V1,
            codec_revision: BOUNDED_ENVELOPE_CODEC_REVISION,
            world_seed: data.recipe.seed,
            completed_time: data.recipe.completed_time.raw(),
            runtime_recipe_fingerprint: metadata.runtime_recipe_fingerprint,
            physical_digest_schema: PHYSICAL_DIGEST_SCHEMA_V9,
            physical_digest: metadata.physical_digest,
            history_digest_schema: metadata.anchor.history_digest_schema(),
            history_digest: *metadata.anchor.logical_digest(),
            section_count: 0,
            section_directory_offset: 0,
            payload_integrity: [0; 32],
        },
        sections,
    ))
}

pub fn history_anchor_from_bounded_envelope(
    envelope: &SnapshotEnvelope,
) -> Result<HistoryAnchor, PersistenceError> {
    if envelope.header.codec_revision != BOUNDED_ENVELOPE_CODEC_REVISION {
        return Err(PersistenceError::codec(
            "unsupported bounded envelope codec revision",
        ));
    }
    if envelope
        .sections
        .contains_key(&u64::from(crate::SECTION_CAUSAL_TRACES))
    {
        return Err(PersistenceError::codec(
            "bounded envelope contains legacy causal traces",
        ));
    }
    let payload = envelope
        .sections
        .get(&u64::from(SECTION_HISTORY_ANCHOR))
        .ok_or(PersistenceError::MissingRequiredSection {
            schema_id: u64::from(SECTION_HISTORY_ANCHOR),
        })?;
    if payload.section_major != HISTORY_ANCHOR_SECTION_MAJOR
        || payload.section_minor != CURRENT_SECTION_MINOR
        || payload.flags != 0
        || payload.decoded_size_limit
            != u64::try_from(HISTORY_ANCHOR_ENCODED_LEN)
                .map_err(|_| PersistenceError::codec("history anchor length does not fit u64"))?
    {
        return Err(PersistenceError::codec(
            "unsupported history anchor section metadata",
        ));
    }
    let anchor = causafera_persistence::decode_history_anchor(&payload.bytes)?;
    validate_header(envelope, &anchor)?;
    Ok(anchor)
}

pub fn disassemble_bounded_envelope(
    envelope: &SnapshotEnvelope,
    history: &dyn HistoryReader,
) -> Result<RuntimeSnapshotData, PersistenceError> {
    disassemble_bounded_envelope_with_trace_snapshot(envelope, history, trace_snapshot_from_history)
}

pub(crate) fn disassemble_bounded_envelope_for_resume(
    envelope: &SnapshotEnvelope,
    history: &dyn HistoryReader,
) -> Result<RuntimeSnapshotData, PersistenceError> {
    disassemble_bounded_envelope_with_trace_snapshot(
        envelope,
        history,
        frontier_trace_snapshot_from_history,
    )
}

fn disassemble_bounded_envelope_with_trace_snapshot(
    envelope: &SnapshotEnvelope,
    history: &dyn HistoryReader,
    trace_snapshot: fn(
        &dyn HistoryReader,
        &HistoryAnchor,
    ) -> Result<CausalTraceSnapshot, PersistenceError>,
) -> Result<RuntimeSnapshotData, PersistenceError> {
    validate_bounded_sections(envelope)?;
    let anchor = history_anchor_from_bounded_envelope(envelope)?;
    if history.anchor()? != anchor {
        return Err(PersistenceError::InvalidHistoryAnchor {
            invariant: "history reader anchor does not match bounded envelope",
        });
    }
    let recipe_payload = envelope
        .sections
        .get(&u64::from(SECTION_RUNTIME_RECIPE))
        .ok_or(PersistenceError::MissingRequiredSection {
            schema_id: u64::from(SECTION_RUNTIME_RECIPE),
        })?;
    let recipe = decode_runtime_recipe_section(&recipe_payload.bytes)?;
    if recipe.seed != envelope.header.world_seed
        || recipe.completed_time.raw() != envelope.header.completed_time
    {
        return Err(PersistenceError::codec(
            "bounded envelope header does not match runtime recipe",
        ));
    }
    let traces = trace_snapshot(history, &anchor)?;
    disassemble_envelope_with_traces(envelope, Some(traces))
}

pub fn disassemble_snapshot_bytes(
    bytes: &[u8],
    history: Option<&dyn HistoryReader>,
) -> Result<RuntimeSnapshotData, PersistenceError> {
    let envelope = SnapshotEnvelope::decode(bytes)?;
    match envelope.header.codec_revision {
        1 => crate::disassemble_envelope(&envelope),
        BOUNDED_ENVELOPE_CODEC_REVISION => {
            let reader = history.ok_or_else(|| {
                PersistenceError::codec("bounded snapshot requires a durable history reader")
            })?;
            disassemble_bounded_envelope(&envelope, reader)
        }
        revision => Err(PersistenceError::codec(format!(
            "unsupported snapshot codec revision {revision}"
        ))),
    }
}

fn validate_bounded_sections(envelope: &SnapshotEnvelope) -> Result<(), PersistenceError> {
    for (schema_id, major) in BOUNDED_STATE_SECTIONS {
        let payload = envelope.sections.get(&u64::from(schema_id)).ok_or(
            PersistenceError::MissingRequiredSection {
                schema_id: u64::from(schema_id),
            },
        )?;
        validate_section_metadata(schema_id, major, payload, 0)?;
    }
    if let Some(payload) = envelope
        .sections
        .get(&u64::from(SECTION_EXPERIMENT_MANIFEST))
    {
        validate_section_metadata(SECTION_EXPERIMENT_MANIFEST, 1, payload, 0)?;
    }
    for schema_id in envelope.sections.keys().copied() {
        let known = schema_id == u64::from(SECTION_HISTORY_ANCHOR)
            || schema_id == u64::from(SECTION_EXPERIMENT_MANIFEST)
            || BOUNDED_STATE_SECTIONS
                .iter()
                .any(|(id, _)| schema_id == u64::from(*id));
        if !known {
            return Err(PersistenceError::codec(format!(
                "unknown bounded snapshot section {schema_id:#06x}"
            )));
        }
    }
    Ok(())
}

fn validate_section_metadata(
    schema_id: u16,
    major: u16,
    payload: &SectionPayload,
    decoded_size_limit: u64,
) -> Result<(), PersistenceError> {
    if payload.section_major != major
        || payload.section_minor != CURRENT_SECTION_MINOR
        || payload.flags != 0
        || payload.decoded_size_limit != decoded_size_limit
    {
        return Err(PersistenceError::codec(format!(
            "unsupported bounded section {schema_id:#06x} metadata"
        )));
    }
    Ok(())
}

fn trace_snapshot_from_history(
    history: &dyn HistoryReader,
    anchor: &HistoryAnchor,
) -> Result<CausalTraceSnapshot, PersistenceError> {
    let mut offset = 0_u64;
    let mut events = Vec::new();
    loop {
        let query = HistoryQuery::new(offset, MAX_QUERY_PAGE_RECORDS, MAX_QUERY_PAGE_BYTES)?;
        let page = history.records(query)?;
        events.extend(page.records().iter().filter_map(|record| match record {
            HistoryRecord::Causal(record) => Some(CausalEventSnapshot {
                event_id: record.event_id(),
                trace_id: record.trace_id(),
                time: record.time(),
                phase: record.phase(),
                kind: record.kind(),
                causes: record.causes().to_vec(),
                effects: record.effects().to_vec(),
            }),
            HistoryRecord::Evidence(_) => None,
        }));
        match page.next_offset() {
            Some(next) if next > offset => offset = next,
            Some(_) => {
                return Err(PersistenceError::codec(
                    "history reader did not advance while reconstructing bounded snapshot",
                ));
            }
            None => break,
        }
    }
    let frontier = anchor.frontier();
    let Some(last) = events.last() else {
        return Err(PersistenceError::InvalidHistoryAnchor {
            invariant: "bounded snapshot history has no anchored frontier",
        });
    };
    if last.event_id != frontier.event_id
        || last.trace_id != frontier.trace_id
        || last.time != frontier.time
    {
        return Err(PersistenceError::InvalidHistoryAnchor {
            invariant: "history reader records do not reach bounded envelope frontier",
        });
    }
    let next_event_id = frontier
        .event_id
        .raw()
        .checked_add(1)
        .ok_or_else(|| PersistenceError::codec("bounded event frontier overflows"))?;
    let next_trace_id = frontier
        .trace_id
        .raw()
        .checked_add(1)
        .ok_or_else(|| PersistenceError::codec("bounded trace frontier overflows"))?;
    Ok(CausalTraceSnapshot {
        next_event_id,
        next_trace_id,
        events,
    })
}

fn frontier_trace_snapshot_from_history(
    history: &dyn HistoryReader,
    anchor: &HistoryAnchor,
) -> Result<CausalTraceSnapshot, PersistenceError> {
    let frontier = anchor.frontier();
    let record =
        history
            .event(frontier.trace_id)?
            .ok_or(PersistenceError::InvalidHistoryAnchor {
                invariant: "history reader does not contain bounded envelope frontier",
            })?;
    if record.event_id() != frontier.event_id
        || record.trace_id() != frontier.trace_id
        || record.time() != frontier.time
    {
        return Err(PersistenceError::InvalidHistoryAnchor {
            invariant: "history reader frontier record does not match bounded envelope",
        });
    }
    let next_event_id = frontier
        .event_id
        .raw()
        .checked_add(1)
        .ok_or_else(|| PersistenceError::codec("bounded event frontier overflows"))?;
    let next_trace_id = frontier
        .trace_id
        .raw()
        .checked_add(1)
        .ok_or_else(|| PersistenceError::codec("bounded trace frontier overflows"))?;
    Ok(CausalTraceSnapshot {
        next_event_id,
        next_trace_id,
        events: Vec::new(),
    })
}

fn insert(sections: &mut BTreeMap<u64, SectionPayload>, id: u16, major: u16, bytes: Vec<u8>) {
    sections.insert(
        u64::from(id),
        SectionPayload {
            section_major: major,
            section_minor: CURRENT_SECTION_MINOR,
            flags: 0,
            decoded_size_limit: 0,
            bytes,
        },
    );
}

fn validate_metadata(
    data: &RuntimeSnapshotData,
    metadata: &BoundedSnapshotMetadata,
) -> Result<(), PersistenceError> {
    let anchor = &metadata.anchor;
    if anchor.logical_generation() != anchor.durable_generation() {
        return Err(PersistenceError::InvalidHistoryAnchor {
            invariant: "portable snapshot generation must be durable",
        });
    }
    if anchor.physical_digest_schema() != causafera_persistence::CURRENT_DIGEST_SCHEMA_VERSION
        || anchor.history_digest_schema() != causafera_persistence::CURRENT_DIGEST_SCHEMA_VERSION
    {
        return Err(PersistenceError::InvalidHistoryAnchor {
            invariant: "bounded envelope requires digest schema 9",
        });
    }
    if anchor.frontier().time != data.recipe.completed_time {
        return Err(PersistenceError::InvalidHistoryAnchor {
            invariant: "frontier time must equal completed runtime time",
        });
    }
    Ok(())
}

fn validate_header(
    envelope: &SnapshotEnvelope,
    anchor: &HistoryAnchor,
) -> Result<(), PersistenceError> {
    if envelope.header.completed_time != anchor.frontier().time.raw()
        || envelope.header.physical_digest_schema != anchor.physical_digest_schema()
        || envelope.header.history_digest_schema != anchor.history_digest_schema()
        || envelope.header.history_digest != *anchor.logical_digest()
    {
        return Err(PersistenceError::InvalidHistoryAnchor {
            invariant: "anchor does not match bounded envelope header",
        });
    }
    if anchor.logical_generation() != anchor.durable_generation() {
        return Err(PersistenceError::InvalidHistoryAnchor {
            invariant: "portable snapshot generation must be durable",
        });
    }
    Ok(())
}
