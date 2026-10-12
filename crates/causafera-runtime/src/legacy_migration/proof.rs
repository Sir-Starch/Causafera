use std::collections::BTreeMap;
use std::path::Path;

use causafera_core::{CausalHistoryReader, HistoryRecord, StateFingerprint};
use causafera_persistence::{
    HistoryStore, MigrationProofReference, SnapshotEnvelope, atomic_write,
};
use causafera_types::TraceId;
use serde::{Deserialize, Serialize};

use crate::bundle::bundle_io;
use crate::{
    BundleSave, DigestSchemaVersion, HistoryDigest, LEGACY_DIGEST_SCHEMA_VERSION,
    PhysicalStateDigest, Runtime, RuntimeError, RuntimeSnapshotData, RuntimeState,
};

use super::{
    LEGACY_BOOTSTRAP_RECEIPT_SCHEMA, LEGACY_HYDROLOGY_CONSERVATION_SCHEMA,
    LEGACY_HYDROLOGY_FORCING_SCHEMA, LEGACY_HYDROLOGY_TRANSFER_SCHEMA, LEGACY_MATERIAL_GATE_SCHEMA,
    LEGACY_MATERIAL_THERMAL_SCHEMA, LEGACY_MATERIAL_TRANSITION_SCHEMA,
    LEGACY_RECIPE_SOURCE_RECEIPT_SCHEMA, LEGACY_THERMAL_BOUNDARY_SCHEMA,
    LEGACY_THERMAL_CONSERVATION_SCHEMA, LEGACY_THERMAL_TRANSFER_SCHEMA,
};

const ALGORITHM_VERSION: u16 = 1;
const COVERAGE_DOMAIN: &[u8] = b"causafera/legacy-coverage/v1\0";
const PROOF_DOMAIN: &[u8] = b"causafera/legacy-migration-proof/v1\0";

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub enum SourceCompleteness {
    Complete,
    Partial,
    Unknown,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct CoverageRange {
    pub first_time: u64,
    pub last_time: u64,
    pub first_trace: u64,
    pub last_trace: u64,
    pub copied_count: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct MissingRange {
    pub first_time: u64,
    pub last_time: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct EvidenceCoverageEntry {
    pub schema_id: u64,
    pub schema_major: u16,
    pub schema_minor: u16,
    pub criticality: String,
    pub source_field: String,
    pub source_available: bool,
    pub source_completeness: SourceCompleteness,
    pub first_guaranteed_typed_tick: Option<u64>,
    pub copied_ranges: Vec<CoverageRange>,
    pub missing_ranges: Vec<MissingRange>,
    pub copied_count: u64,
    pub copied_payload_digest: [u8; 32],
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct LegacyMigrationProof {
    pub algorithm_version: u16,
    pub source_envelope_digest: [u8; 32],
    pub source_envelope_bytes: u64,
    pub source_format_major: u16,
    pub source_format_minor: u16,
    pub source_codec_revision: u32,
    pub source_physical_digest_schema: u16,
    pub source_history_digest_schema: u16,
    pub source_world_seed: u64,
    pub source_completed_time: u64,
    pub source_sections: Vec<(u64, u16)>,
    pub source_physical_digest_v8: [u8; 32],
    pub source_history_digest_v8: [u8; 32],
    pub source_experiment_digest_v8: [u8; 32],
    pub source_causal_events: u64,
    pub source_to_destination_event_ids: String,
    pub source_to_destination_trace_ids: String,
    pub source_to_destination_evidence_keys: String,
    pub destination_generation: u64,
    pub destination_causal_events: u64,
    pub destination_evidence_records: u64,
    pub destination_physical_digest_v9: [u8; 32],
    pub destination_history_digest_v9: [u8; 32],
    pub destination_experiment_digest_v9: [u8; 32],
    pub destination_storage_root: [u8; 32],
    pub destination_anchor_namespace: [u8; 16],
    pub destination_anchor_frontier: (u64, u64, u64, u64),
    pub destination_evidence_manifest_digest: [u8; 32],
    pub destination_checkpoint_digest: [u8; 32],
    pub coverage: Vec<EvidenceCoverageEntry>,
    pub coverage_digest: [u8; 32],
    pub proof_digest: [u8; 32],
}

fn json_bytes<T: Serialize>(value: &T) -> Result<Vec<u8>, RuntimeError> {
    serde_json::to_vec(value).map_err(|error| {
        RuntimeError::SnapshotAssembly(causafera_persistence::PersistenceError::codec(format!(
            "encode migration proof: {error}"
        )))
    })
}

fn digest(domain: &[u8], bytes: &[u8]) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new();
    hasher.update(domain);
    hasher.update(bytes);
    *hasher.finalize().as_bytes()
}

fn trace_frontier(record: &causafera_core::DomainEvidenceRecord) -> u64 {
    record
        .causal_anchors()
        .iter()
        .map(|trace| trace.raw())
        .max()
        .unwrap_or(0)
}

fn first_retained_tick(traces: &[TraceId], data: &RuntimeSnapshotData) -> Option<u64> {
    let first = traces.first()?;
    data.traces
        .events
        .iter()
        .find(|event| event.trace_id == *first)
        .map(|event| event.time.raw())
}

fn coverage(
    data: &RuntimeSnapshotData,
    records: &[HistoryRecord],
) -> Result<Vec<EvidenceCoverageEntry>, RuntimeError> {
    let hydrology_first = first_retained_tick(&data.hydrology.retained_batches, data);
    let thermal_first = first_retained_tick(&data.thermal.receipt_batches, data);
    let schemas = [
        (
            LEGACY_HYDROLOGY_TRANSFER_SCHEMA,
            "hydrology.receipts",
            hydrology_first,
        ),
        (
            LEGACY_HYDROLOGY_CONSERVATION_SCHEMA,
            "hydrology.conservation_receipts",
            hydrology_first,
        ),
        (
            LEGACY_HYDROLOGY_FORCING_SCHEMA,
            "hydrology.forcing",
            Some(0),
        ),
        (
            LEGACY_THERMAL_TRANSFER_SCHEMA,
            "thermal.transfer_receipts",
            thermal_first,
        ),
        (
            LEGACY_THERMAL_CONSERVATION_SCHEMA,
            "thermal.conservation_receipts",
            thermal_first,
        ),
        (
            LEGACY_THERMAL_BOUNDARY_SCHEMA,
            "thermal.boundary_records",
            Some(data.recipe.completed_time.raw()),
        ),
        (
            LEGACY_MATERIAL_TRANSITION_SCHEMA,
            "material_surfaces.transitions",
            None,
        ),
        (
            LEGACY_MATERIAL_GATE_SCHEMA,
            "material_surfaces.gate_transitions",
            None,
        ),
        (
            LEGACY_MATERIAL_THERMAL_SCHEMA,
            "material_surfaces.thermal_transitions",
            None,
        ),
        (
            LEGACY_BOOTSTRAP_RECEIPT_SCHEMA,
            "bootstrap.receipts",
            Some(0),
        ),
        (
            LEGACY_RECIPE_SOURCE_RECEIPT_SCHEMA,
            "experiment_recipe_mana_source_receipts",
            Some(0),
        ),
    ];
    let mut entries = Vec::with_capacity(schemas.len());
    for (schema, source_field, guaranteed) in schemas {
        let mut per_tick = BTreeMap::<u64, CoverageRange>::new();
        let mut payload_hasher = blake3::Hasher::new();
        let mut copied_count = 0_u64;
        for record in records {
            let HistoryRecord::Evidence(record) = record else {
                continue;
            };
            if record.schema_id() != schema {
                continue;
            }
            let time = record.time().raw();
            let trace = trace_frontier(record);
            let range = per_tick.entry(time).or_insert(CoverageRange {
                first_time: time,
                last_time: time,
                first_trace: trace,
                last_trace: trace,
                copied_count: 0,
            });
            range.first_trace = range.first_trace.min(trace);
            range.last_trace = range.last_trace.max(trace);
            range.copied_count =
                range
                    .copied_count
                    .checked_add(1)
                    .ok_or(RuntimeError::InvalidSnapshot(
                        "migration receipt count overflow",
                    ))?;
            copied_count = copied_count
                .checked_add(1)
                .ok_or(RuntimeError::InvalidSnapshot(
                    "migration receipt count overflow",
                ))?;
            payload_hasher.update(record.canonical_key());
            payload_hasher.update(&record.payload_digest().bytes());
        }
        let completed = data.recipe.completed_time.raw();
        let missing_ranges = match guaranteed {
            Some(first) if first > 0 => vec![MissingRange {
                first_time: 0,
                last_time: first - 1,
            }],
            None => vec![MissingRange {
                first_time: 0,
                last_time: completed,
            }],
            Some(_) => Vec::new(),
        };
        let source_completeness = match guaranteed {
            None => SourceCompleteness::Unknown,
            Some(0) => SourceCompleteness::Complete,
            Some(_) => SourceCompleteness::Partial,
        };
        entries.push(EvidenceCoverageEntry {
            schema_id: schema.raw(),
            schema_major: 1,
            schema_minor: 0,
            criticality: "ExplanationOnly".to_owned(),
            source_field: source_field.to_owned(),
            source_available: true,
            source_completeness,
            first_guaranteed_typed_tick: guaranteed,
            copied_ranges: per_tick.into_values().collect(),
            missing_ranges,
            copied_count,
            copied_payload_digest: *payload_hasher.finalize().as_bytes(),
        });
    }
    Ok(entries)
}

impl LegacyMigrationProof {
    pub(crate) fn from_source(
        source_bytes: &[u8],
        envelope: &SnapshotEnvelope,
        data: &RuntimeSnapshotData,
        records: &[HistoryRecord],
    ) -> Result<Self, RuntimeError> {
        let v8 = LEGACY_DIGEST_SCHEMA_VERSION;
        let experiment = RuntimeState::canonical_state_for_schema(
            data.recipe.seed,
            [0; 32],
            PhysicalStateDigest {
                schema_version: DigestSchemaVersion::new(v8.raw()),
                fingerprint: StateFingerprint::new(envelope.header.physical_digest),
            },
            HistoryDigest {
                schema_version: DigestSchemaVersion::new(v8.raw()),
                fingerprint: StateFingerprint::new(envelope.header.history_digest),
            },
            v8,
        );
        let coverage = coverage(data, records)?;
        let coverage_digest = digest(COVERAGE_DOMAIN, &json_bytes(&coverage)?);
        let source_causal_events = u64::try_from(data.traces.events.len())
            .map_err(|_| RuntimeError::InvalidSnapshot("migration causal event count overflow"))?;
        let destination_evidence_records = coverage.iter().try_fold(0_u64, |count, entry| {
            count
                .checked_add(entry.copied_count)
                .ok_or(RuntimeError::InvalidSnapshot(
                    "migration evidence count overflow",
                ))
        })?;
        Ok(Self {
            algorithm_version: ALGORITHM_VERSION,
            source_envelope_digest: *blake3::hash(source_bytes).as_bytes(),
            source_envelope_bytes: u64::try_from(source_bytes.len())
                .map_err(|_| RuntimeError::InvalidSnapshot("migration source size overflow"))?,
            source_format_major: envelope.header.format_major,
            source_format_minor: envelope.header.format_minor,
            source_codec_revision: envelope.header.codec_revision,
            source_physical_digest_schema: envelope.header.physical_digest_schema,
            source_history_digest_schema: envelope.header.history_digest_schema,
            source_world_seed: envelope.header.world_seed,
            source_completed_time: envelope.header.completed_time,
            source_sections: envelope
                .sections
                .iter()
                .map(|(id, section)| (*id, section.section_major))
                .collect(),
            source_physical_digest_v8: envelope.header.physical_digest,
            source_history_digest_v8: envelope.header.history_digest,
            source_experiment_digest_v8: experiment.bytes(),
            source_causal_events,
            source_to_destination_event_ids: "identity".to_owned(),
            source_to_destination_trace_ids: "identity".to_owned(),
            source_to_destination_evidence_keys:
                "schema-specific source key retained in archived record".to_owned(),
            destination_generation: 0,
            destination_causal_events: source_causal_events,
            destination_evidence_records,
            destination_physical_digest_v9: [0; 32],
            destination_history_digest_v9: [0; 32],
            destination_experiment_digest_v9: [0; 32],
            destination_storage_root: [0; 32],
            destination_anchor_namespace: [0; 16],
            destination_anchor_frontier: (0, 0, 0, 0),
            destination_evidence_manifest_digest: [0; 32],
            destination_checkpoint_digest: [0; 32],
            coverage,
            coverage_digest,
            proof_digest: [0; 32],
        })
    }

    pub(crate) fn finish(
        &mut self,
        runtime: &Runtime,
        saved: &BundleSave,
    ) -> Result<(), RuntimeError> {
        let snapshot = runtime.snapshot()?;
        let anchor = runtime.history_reader()?.anchor()?;
        self.destination_generation = saved.generation;
        self.destination_physical_digest_v9 = snapshot.physical_state_digest.bytes();
        self.destination_history_digest_v9 = snapshot.history_digest.bytes();
        self.destination_experiment_digest_v9 = snapshot.canonical_state.bytes();
        self.destination_storage_root = *anchor.storage_root();
        self.destination_anchor_namespace = *anchor.namespace();
        let frontier = anchor.frontier();
        self.destination_anchor_frontier = (
            frontier.generation,
            frontier.event_id.raw(),
            frontier.trace_id.raw(),
            frontier.time.raw(),
        );
        self.destination_evidence_manifest_digest = *anchor.evidence_manifest_digest();
        self.destination_checkpoint_digest = saved.checkpoint_digest;
        self.proof_digest = self.calculate_digest()?;
        Ok(())
    }

    fn calculate_digest(&self) -> Result<[u8; 32], RuntimeError> {
        let mut unsigned = self.clone();
        unsigned.proof_digest = [0; 32];
        Ok(digest(PROOF_DOMAIN, &json_bytes(&unsigned)?))
    }

    pub(crate) fn write(&self, root: &Path) -> Result<MigrationProofReference, RuntimeError> {
        let path = root
            .join("migration")
            .join(format!("{}.proof", hex(&self.source_envelope_digest)));
        let bytes = json_bytes(self)?;
        atomic_write(&path, &bytes)?;
        Ok(MigrationProofReference {
            source_digest: self.source_envelope_digest,
            file_digest: *blake3::hash(&bytes).as_bytes(),
        })
    }

    pub fn read(root: &Path) -> Result<Option<Self>, RuntimeError> {
        let migration = root.join("migration");
        let mut entries = std::fs::read_dir(&migration)
            .map_err(|error| bundle_io("read migration directory", &error))?;
        let Some(entry) = entries.next() else {
            return Ok(None);
        };
        let entry = entry.map_err(|error| bundle_io("read migration proof entry", &error))?;
        if entries.next().is_some() {
            return Err(RuntimeError::InvalidSnapshot("multiple migration proofs"));
        }
        let bytes = std::fs::read(entry.path())
            .map_err(|error| bundle_io("read migration proof", &error))?;
        let proof: Self = serde_json::from_slice(&bytes)
            .map_err(|_| RuntimeError::InvalidSnapshot("invalid migration proof encoding"))?;
        if entry.file_name().to_str()
            != Some(format!("{}.proof", hex(&proof.source_envelope_digest)).as_str())
            || proof.algorithm_version != ALGORITHM_VERSION
            || proof.coverage_digest != digest(COVERAGE_DOMAIN, &json_bytes(&proof.coverage)?)
            || proof.proof_digest != proof.calculate_digest()?
        {
            return Err(RuntimeError::InvalidSnapshot(
                "migration proof digest mismatch",
            ));
        }
        Ok(Some(proof))
    }

    pub(crate) fn verify_against_store(&self, store: &HistoryStore) -> Result<(), RuntimeError> {
        let (events, evidence) = store.record_counts_at(self.destination_generation)?;
        let anchor = store
            .reader_at_durable_generation(self.destination_generation)?
            .anchor()?;
        let checkpoint = store
            .checkpoint_at_durable_generation(self.destination_generation)?
            .ok_or(RuntimeError::InvalidSnapshot(
                "migration checkpoint is missing",
            ))?;
        let checkpoint_row = store
            .catalog()
            .checkpoint_for_generation(self.destination_generation)?
            .ok_or(RuntimeError::InvalidSnapshot(
                "migration checkpoint is missing",
            ))?;
        if events != self.destination_causal_events
            || evidence != self.destination_evidence_records
            || *anchor.logical_digest() != self.destination_history_digest_v9
            || *anchor.storage_root() != self.destination_storage_root
            || *anchor.namespace() != self.destination_anchor_namespace
            || (
                anchor.frontier().generation,
                anchor.frontier().event_id.raw(),
                anchor.frontier().trace_id.raw(),
                anchor.frontier().time.raw(),
            ) != self.destination_anchor_frontier
            || *anchor.evidence_manifest_digest() != self.destination_evidence_manifest_digest
            || checkpoint_row.digest != self.destination_checkpoint_digest
            || checkpoint.header.physical_digest != self.destination_physical_digest_v9
        {
            return Err(RuntimeError::InvalidSnapshot(
                "migration proof disagrees with bundle",
            ));
        }
        Ok(())
    }
}

fn hex(bytes: &[u8; 32]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
