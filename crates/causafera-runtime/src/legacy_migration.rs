use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use causafera_core::{
    Criticality, DomainEvidenceRecord, EvidenceSchemaId, EvidenceSchemaVersion, HistoryError,
    HistoryRecord, OwnedCausalRecord, StateFingerprint, canonical_history_records,
};
use causafera_persistence::{HistoryStore, LittleEndianEncoder, SnapshotEnvelope};
use causafera_types::{SimulationTime, TraceId};

use crate::bundle::{bundle_io, discard_failed_bundle};
use crate::history_transaction::{
    RuntimeHistoryCoordinator, runtime_history_decoder_registry, runtime_history_namespace,
};
use crate::{BundleOptions, HydrologyRuntimeState, Runtime, RuntimeError, RuntimeSnapshotData};

mod proof;
pub use proof::{LegacyMigrationProof, SourceCompleteness};

const LEGACY_HYDROLOGY_TRANSFER_SCHEMA: EvidenceSchemaId = EvidenceSchemaId::new(0x0014_F001);
const LEGACY_HYDROLOGY_CONSERVATION_SCHEMA: EvidenceSchemaId = EvidenceSchemaId::new(0x0014_F002);
const LEGACY_HYDROLOGY_FORCING_SCHEMA: EvidenceSchemaId = EvidenceSchemaId::new(0x0014_F003);
const LEGACY_THERMAL_TRANSFER_SCHEMA: EvidenceSchemaId = EvidenceSchemaId::new(0x0014_F004);
const LEGACY_THERMAL_CONSERVATION_SCHEMA: EvidenceSchemaId = EvidenceSchemaId::new(0x0014_F005);
const LEGACY_THERMAL_BOUNDARY_SCHEMA: EvidenceSchemaId = EvidenceSchemaId::new(0x0014_F006);
const LEGACY_MATERIAL_TRANSITION_SCHEMA: EvidenceSchemaId = EvidenceSchemaId::new(0x0014_F007);
const LEGACY_MATERIAL_GATE_SCHEMA: EvidenceSchemaId = EvidenceSchemaId::new(0x0014_F008);
const LEGACY_MATERIAL_THERMAL_SCHEMA: EvidenceSchemaId = EvidenceSchemaId::new(0x0014_F009);
const LEGACY_BOOTSTRAP_RECEIPT_SCHEMA: EvidenceSchemaId = EvidenceSchemaId::new(0x0014_F00A);
const LEGACY_RECIPE_SOURCE_RECEIPT_SCHEMA: EvidenceSchemaId = EvidenceSchemaId::new(0x0014_F00B);
const LEGACY_EVIDENCE_VERSION: EvidenceSchemaVersion = EvidenceSchemaVersion::new(1, 0);

const REQUIRED_SECTIONS: &[(u64, u16)] = &[
    (0x0001, 7),
    (0x0002, 1),
    (0x0003, 2),
    (0x0004, 1),
    (0x0005, 1),
    (0x0006, 3),
    (0x0007, 1),
    (0x0008, 1),
    (0x0009, 2),
    (0x000A, 1),
    (0x000C, 3),
    (0x000D, 1),
    (0x000E, 2),
    (0x000F, 1),
];

fn validate_legacy_matrix(envelope: &SnapshotEnvelope) -> Result<(), RuntimeError> {
    if envelope.header.format_major != 1
        || envelope.header.codec_revision != 1
        || envelope.header.physical_digest_schema != 8
        || envelope.header.history_digest_schema != 8
    {
        return Err(RuntimeError::InvalidSnapshot(
            "unsupported legacy migration header",
        ));
    }
    let has_required = REQUIRED_SECTIONS.iter().all(|(id, major)| {
        envelope.sections.get(id).is_some_and(|section| {
            section.section_major == *major && section.section_minor == 0 && section.flags == 0
        })
    });
    let has_optional = envelope.sections.get(&0x000B).is_none_or(|section| {
        section.section_major == 1 && section.section_minor == 0 && section.flags == 0
    });
    if !has_required
        || !has_optional
        || envelope.sections.len()
            != REQUIRED_SECTIONS.len() + usize::from(envelope.sections.contains_key(&0x000B))
    {
        return Err(RuntimeError::InvalidSnapshot(
            "unsupported legacy migration section combination",
        ));
    }
    Ok(())
}

fn archived_record(
    schema: EvidenceSchemaId,
    time: SimulationTime,
    trace: TraceId,
    mut anchors: Vec<TraceId>,
    key: Vec<u8>,
    payload: Vec<u8>,
) -> Result<HistoryRecord, RuntimeError> {
    anchors.push(trace);
    anchors.sort_unstable();
    anchors.dedup();
    let digest = StateFingerprint::new(*blake3::hash(&payload).as_bytes());
    Ok(HistoryRecord::Evidence(DomainEvidenceRecord::new(
        time.raw(),
        schema,
        LEGACY_EVIDENCE_VERSION,
        Criticality::ExplanationOnly,
        time,
        anchors,
        key,
        payload,
        digest,
    )?))
}

fn migration_record_bytes(record: &HistoryRecord) -> Result<usize, RuntimeError> {
    let estimate = match record {
        HistoryRecord::Causal(record) => record.canonical_field_size()?.checked_add(32),
        HistoryRecord::Evidence(record) => record
            .causal_anchors()
            .len()
            .checked_mul(8)
            .and_then(|size| size.checked_add(record.canonical_key().len()))
            .and_then(|size| size.checked_add(record.canonical_payload().len()))
            .and_then(|size| size.checked_add(128)),
    };
    estimate.ok_or(HistoryError::ArithmeticOverflow.into())
}

fn indexed_key(trace: TraceId, index: usize) -> Result<Vec<u8>, RuntimeError> {
    let mut key = trace.raw().to_le_bytes().to_vec();
    key.extend_from_slice(
        &u64::try_from(index)
            .map_err(|_| HistoryError::ArithmeticOverflow)?
            .to_le_bytes(),
    );
    Ok(key)
}

fn archive_hydrology_receipts(
    hydrology: &HydrologyRuntimeState,
    records: &mut Vec<HistoryRecord>,
) -> Result<(), RuntimeError> {
    for (trace, receipts) in &hydrology.receipts {
        for (index, receipt) in receipts.iter().enumerate() {
            let mut key = trace.raw().to_le_bytes().to_vec();
            key.extend_from_slice(
                &u64::try_from(index)
                    .map_err(|_| HistoryError::ArithmeticOverflow)?
                    .to_le_bytes(),
            );
            let mut anchors = receipt.causal_parents().to_vec();
            anchors.extend(receipt.forcing_origin());
            records.push(archived_record(
                LEGACY_HYDROLOGY_TRANSFER_SCHEMA,
                SimulationTime::new(receipt.tick()),
                *trace,
                anchors,
                key,
                crate::snapshot_sections::encode_legacy_hydrology_receipt(receipt),
            )?);
        }
    }
    for (trace, receipt) in &hydrology.conservation_receipts {
        records.push(archived_record(
            LEGACY_HYDROLOGY_CONSERVATION_SCHEMA,
            SimulationTime::new(receipt.tick()),
            *trace,
            Vec::new(),
            trace.raw().to_le_bytes().to_vec(),
            crate::snapshot_sections::encode_legacy_hydrology_conservation(receipt),
        )?);
    }
    for forcing in &hydrology.forcing {
        let mut key = forcing.scheduled_tick().to_le_bytes().to_vec();
        key.extend_from_slice(&forcing.forcing_id().to_le_bytes());
        records.push(archived_record(
            LEGACY_HYDROLOGY_FORCING_SCHEMA,
            SimulationTime::new(forcing.applied_at().unwrap_or(0)),
            forcing.origin_trace(),
            Vec::new(),
            key,
            crate::snapshot_sections::encode_legacy_hydrology_forcing(forcing),
        )?);
    }
    Ok(())
}

fn archive_remaining_receipts(
    data: &RuntimeSnapshotData,
    records: &mut Vec<HistoryRecord>,
) -> Result<(), RuntimeError> {
    let thermal_time = data
        .thermal
        .conservation_receipts
        .iter()
        .map(|receipt| (receipt.trace, receipt.tick))
        .collect::<std::collections::BTreeMap<_, _>>();
    for (index, receipt) in data.thermal.transfer_receipts.iter().enumerate() {
        let tick = thermal_time
            .get(&receipt.conservation_trace)
            .copied()
            .ok_or(RuntimeError::InvalidSnapshot(
                "thermal transfer has no conservation receipt",
            ))?;
        let mut key = receipt.conservation_trace.raw().to_le_bytes().to_vec();
        key.extend_from_slice(&receipt.cell.chunk.chart.raw().to_le_bytes());
        key.extend_from_slice(&receipt.cell.cell_index.to_le_bytes());
        key.extend_from_slice(
            &u64::try_from(index)
                .map_err(|_| HistoryError::ArithmeticOverflow)?
                .to_le_bytes(),
        );
        records.push(archived_record(
            LEGACY_THERMAL_TRANSFER_SCHEMA,
            SimulationTime::new(tick),
            receipt.conservation_trace,
            receipt.cell_change_trace_id.into_iter().collect(),
            key,
            crate::snapshot_sections::encode_legacy_thermal_transfer(receipt),
        )?);
    }
    for receipt in &data.thermal.conservation_receipts {
        records.push(archived_record(
            LEGACY_THERMAL_CONSERVATION_SCHEMA,
            SimulationTime::new(receipt.tick),
            receipt.trace,
            Vec::new(),
            receipt.trace.raw().to_le_bytes().to_vec(),
            crate::snapshot_sections::encode_legacy_thermal_conservation(receipt),
        )?);
    }
    let boundary_trace = data.thermal.field_set.conservation_last_change;
    let boundary_tick = thermal_time.get(&boundary_trace).copied().unwrap_or(0);
    for (index, receipt) in data.thermal.boundary_records.iter().enumerate() {
        let mut key = boundary_trace.raw().to_le_bytes().to_vec();
        key.extend_from_slice(
            &u64::try_from(index)
                .map_err(|_| HistoryError::ArithmeticOverflow)?
                .to_le_bytes(),
        );
        records.push(archived_record(
            LEGACY_THERMAL_BOUNDARY_SCHEMA,
            SimulationTime::new(boundary_tick),
            boundary_trace,
            Vec::new(),
            key,
            crate::snapshot_sections::encode_legacy_thermal_boundary(receipt),
        )?);
    }
    for (index, transition) in data.material_surfaces.transitions.iter().enumerate() {
        records.push(archived_record(
            LEGACY_MATERIAL_TRANSITION_SCHEMA,
            transition.occurred_at,
            transition.transition_trace,
            transition
                .contact_trace
                .into_iter()
                .chain(transition.mana_effect_trace)
                .collect(),
            indexed_key(transition.transition_trace, index)?,
            crate::snapshot_sections::encode_legacy_material_transition(transition),
        )?);
    }
    for (index, transition) in data.material_surfaces.gate_transitions.iter().enumerate() {
        records.push(archived_record(
            LEGACY_MATERIAL_GATE_SCHEMA,
            transition.occurred_at,
            transition.transition_trace,
            transition
                .contact_trace
                .into_iter()
                .chain(Some(transition.local_mana_trace))
                .collect(),
            indexed_key(transition.transition_trace, index)?,
            crate::snapshot_sections::encode_legacy_material_gate(transition),
        )?);
    }
    for (index, transition) in data
        .material_surfaces
        .thermal_transitions
        .iter()
        .enumerate()
    {
        records.push(archived_record(
            LEGACY_MATERIAL_THERMAL_SCHEMA,
            transition.occurred_at,
            transition.transition_trace,
            vec![transition.cell_trace],
            indexed_key(transition.transition_trace, index)?,
            crate::snapshot_sections::encode_legacy_material_thermal(transition),
        )?);
    }
    for receipt in &data.bootstrap.receipts {
        let mut payload = Vec::new();
        let mut enc = LittleEndianEncoder::new(&mut payload);
        enc.write_u64(receipt.stage.raw());
        enc.write_u64(receipt.completed_at.raw());
        enc.write_fixed(&receipt.result.bytes());
        enc.write_u64(receipt.trace.raw());
        enc.write_u64(receipt.causes.len() as u64);
        for cause in &receipt.causes {
            enc.write_u64(cause.raw());
        }
        records.push(archived_record(
            LEGACY_BOOTSTRAP_RECEIPT_SCHEMA,
            receipt.completed_at,
            receipt.trace,
            receipt.causes.clone(),
            receipt.stage.raw().to_le_bytes().to_vec(),
            payload,
        )?);
    }
    for receipt in &data.experiment_recipe_mana_source_receipts {
        let mut payload = Vec::new();
        let mut enc = LittleEndianEncoder::new(&mut payload);
        for value in [
            receipt.source_record_id,
            receipt.scheduled_tick,
            receipt.executed_tick,
            receipt.source_trace.raw(),
        ] {
            enc.write_u64(value);
        }
        enc.write_i64(receipt.before_intensity);
        enc.write_i64(receipt.after_intensity);
        enc.write_fixed(&receipt.recipe_hash.bytes());
        enc.write_u64(receipt.policy_schema_id);
        records.push(archived_record(
            LEGACY_RECIPE_SOURCE_RECEIPT_SCHEMA,
            SimulationTime::new(receipt.executed_tick),
            receipt.source_trace,
            Vec::new(),
            receipt.source_record_id.to_le_bytes().to_vec(),
            payload,
        )?);
    }
    Ok(())
}

impl Runtime {
    pub fn migrate_legacy_snapshot(
        source: &Path,
        destination: &Path,
        options: BundleOptions,
    ) -> Result<Self, RuntimeError> {
        let source_bytes =
            std::fs::read(source).map_err(|error| bundle_io("read legacy snapshot", &error))?;
        let envelope = SnapshotEnvelope::decode(&source_bytes)?;
        validate_legacy_matrix(&envelope)?;
        let source_digest = *blake3::hash(&source_bytes).as_bytes();
        if destination.exists() {
            if let Ok((runtime, _)) = Self::open_bundle(destination) {
                let proof = LegacyMigrationProof::read(destination)?;
                if proof
                    .as_ref()
                    .is_some_and(|proof| proof.source_envelope_digest == source_digest)
                {
                    return Ok(runtime);
                }
            }
            return Err(RuntimeError::BundleExists {
                path: destination.display().to_string(),
            });
        }
        let imported = Self::from_snapshot_bytes(&source_bytes, None)?;
        let mut data = imported.export_snapshot_with_full_history()?;
        drop(imported);
        if data.recipe.seed != envelope.header.world_seed
            || data.recipe.seed != data.recipe.config.deterministic.world_seed
        {
            return Err(RuntimeError::InvalidSnapshot(
                "legacy snapshot seed lineage mismatch",
            ));
        }
        static NEXT_STAGE: AtomicU64 = AtomicU64::new(0);
        let parent = destination
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        let name = destination
            .file_name()
            .ok_or(RuntimeError::InvalidSnapshot(
                "migration destination has no name",
            ))?;
        let mut stage_name = name.to_os_string();
        stage_name.push(format!(
            ".migration-p{}-{}",
            std::process::id(),
            NEXT_STAGE.fetch_add(1, Ordering::Relaxed)
        ));
        let stage: PathBuf = parent.join(stage_name);
        if stage.exists() {
            return Err(RuntimeError::BundleExists {
                path: stage.display().to_string(),
            });
        }
        let created = (|| {
            let registry = runtime_history_decoder_registry()?;
            let mut store = HistoryStore::create_with_registry(
                &stage,
                runtime_history_namespace(data.recipe.seed),
                options.durability,
                registry,
            )?;
            let mut records = Vec::with_capacity(data.traces.events.len());
            for event in &data.traces.events {
                records.push(HistoryRecord::Causal(OwnedCausalRecord::new(
                    event.time.raw(),
                    event.event_id,
                    event.trace_id,
                    event.time,
                    event.phase,
                    event.kind,
                    event.causes.clone(),
                    event.effects.clone(),
                )?));
            }
            archive_hydrology_receipts(&data.hydrology, &mut records)?;
            archive_remaining_receipts(&data, &mut records)?;
            let mut proof =
                LegacyMigrationProof::from_source(&source_bytes, &envelope, &data, &records)?;
            let mut frame = Vec::new();
            let mut frame_bytes = 8_usize;
            for record in canonical_history_records(records)? {
                let bytes = migration_record_bytes(&record)?;
                let next_bytes = frame_bytes
                    .checked_add(bytes)
                    .ok_or(HistoryError::ArithmeticOverflow)?;
                if !frame.is_empty()
                    && (frame.len() == 16_384
                        || next_bytes > causafera_core::MAX_FRAME_DECODED_BYTES - 1024)
                {
                    store.append_tick(std::mem::take(&mut frame))?;
                    frame_bytes = 8;
                }
                frame_bytes = frame_bytes
                    .checked_add(bytes)
                    .ok_or(HistoryError::ArithmeticOverflow)?;
                frame.push(record);
            }
            if !frame.is_empty() {
                store.append_tick(frame)?;
            }
            store.flush_durable()?;
            let generation = store.durable_generation();
            data.traces.events.clear();
            let resume = RuntimeHistoryCoordinator::resume_open_bundle(
                store,
                &stage,
                options.cadence,
                None,
            )?;
            let mut runtime =
                Self::from_snapshot_with_history(data, Some(generation), Some(resume))?;
            let saved = runtime.save_bundle()?;
            proof.finish(&runtime, &saved)?;
            let reference = proof.write(&stage)?;
            runtime
                .lock_state()?
                .history
                .register_migration_proof(reference)?;
            runtime.shutdown()?;
            if destination.exists() {
                return Err(RuntimeError::BundleExists {
                    path: destination.display().to_string(),
                });
            }
            std::fs::rename(&stage, destination)
                .map_err(|error| bundle_io("publish migrated bundle", &error))?;
            std::fs::File::open(parent)
                .and_then(|directory| directory.sync_all())
                .map_err(|error| bundle_io("sync migrated bundle parent", &error))?;
            Self::open_bundle(destination).map(|(runtime, _)| runtime)
        })();
        if created.is_err() && stage.exists() {
            discard_failed_bundle(&stage, false);
        }
        created
    }
}
