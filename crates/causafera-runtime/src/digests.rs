use crate::*;
use causafera_core::*;
use causafera_types::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DigestSchemaVersion(u16);

impl DigestSchemaVersion {
    pub const fn new(raw: u16) -> Self {
        Self(raw)
    }

    pub const fn raw(self) -> u16 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PhysicalStateDigest {
    pub schema_version: DigestSchemaVersion,
    pub fingerprint: StateFingerprint,
}

impl PhysicalStateDigest {
    pub const fn bytes(self) -> [u8; 32] {
        self.fingerprint.bytes()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HistoryDigest {
    pub schema_version: DigestSchemaVersion,
    pub fingerprint: StateFingerprint,
}

impl HistoryDigest {
    pub const fn bytes(self) -> [u8; 32] {
        self.fingerprint.bytes()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ExperimentDigest {
    pub schema_version: DigestSchemaVersion,
    pub fingerprint: StateFingerprint,
}

impl ExperimentDigest {
    pub const fn bytes(self) -> [u8; 32] {
        self.fingerprint.bytes()
    }
}

pub(crate) fn write_chart_chunk(digest: &mut CanonicalDigest, chunk: ChartChunkCoord) {
    digest.write(chunk.chart.raw());
    digest.write(chunk.chunk.x as u64);
    digest.write(chunk.chunk.y as u64);
    digest.write(chunk.chunk.z as u64);
}

pub(crate) fn write_optional_trace(digest: &mut CanonicalDigest, trace: Option<TraceId>) {
    match trace {
        Some(trace) => {
            digest.write(1);
            digest.write(trace.raw());
        }
        None => digest.write(0),
    }
}

pub(crate) fn write_population_aggregate(
    digest: &mut CanonicalDigest,
    aggregate: &PopulationAggregate,
) {
    write_chart_chunk(digest, aggregate.chart);
    digest.write(aggregate.count);
    digest.write(aggregate.births);
    digest.write(aggregate.deaths);
    digest.write(aggregate.material_inflow as u64);
    digest.write(aggregate.material_outflow as u64);
    digest.write(aggregate.causal_ancestry.len() as u64);
    for trace in &aggregate.causal_ancestry {
        digest.write(trace.raw());
    }
}

pub(crate) fn chart_chunk_hash(chunk: ChartChunkCoord) -> u64 {
    mix64(
        chunk.chart.raw()
            ^ (chunk.chunk.x as u64).rotate_left(7)
            ^ (chunk.chunk.y as u64).rotate_left(19)
            ^ (chunk.chunk.z as u64).rotate_left(31),
    )
}

pub(crate) fn ordered_trace_causes(causes: [TraceId; 2]) -> Vec<TraceId> {
    let mut ordered = causes.to_vec();
    ordered.sort_unstable();
    ordered.dedup();
    ordered
}

pub(crate) fn fingerprint_u64(tag: u64, value: u64) -> StateFingerprint {
    fingerprint_words([tag, value, mix64(tag ^ value), tag.rotate_left(17) ^ value])
}

pub(crate) fn fingerprint_i64(tag: u64, value: i64) -> StateFingerprint {
    fingerprint_u64(tag, value as u64)
}

pub(crate) fn experiment_recipe_mana_source_receipt_fingerprint(
    record: &ExperimentRecipeManaSource,
    executed_tick: u64,
    source_trace: TraceId,
    before_intensity: i64,
    after_intensity: i64,
    recipe_hash: StateFingerprint,
) -> StateFingerprint {
    let mut digest = CanonicalDigest::new();
    digest.write(0x0303);
    digest.write(record.source_record_id);
    digest.write(record.scheduled_tick);
    digest.write(executed_tick);
    digest.write(source_trace.raw());
    digest.write(before_intensity as u64);
    digest.write(after_intensity as u64);
    digest.write_bytes(recipe_hash.bytes());
    digest.write(record.policy_schema_id);
    digest.finish()
}

pub(crate) fn fingerprint_pair(tag: u64, first: i64, second: i64) -> StateFingerprint {
    fingerprint_words([
        tag,
        first as u64,
        second as u64,
        mix64(tag ^ first as u64 ^ (second as u64).rotate_left(23)),
    ])
}

pub(crate) fn fingerprint_population_aggregate(
    aggregate: &PopulationAggregate,
) -> StateFingerprint {
    fingerprint_words([
        0x0803,
        chart_chunk_hash(aggregate.chart),
        aggregate.count ^ aggregate.births.rotate_left(11) ^ aggregate.deaths.rotate_left(23),
        (aggregate.material_inflow as u64) ^ (aggregate.material_outflow as u64).rotate_left(31),
    ])
}

pub(crate) fn fingerprint_material_flow(aggregate: &PopulationAggregate) -> StateFingerprint {
    fingerprint_pair(
        0x0C01,
        aggregate.material_inflow,
        aggregate.material_outflow,
    )
}

pub(crate) fn fingerprint_words(words: [u64; 4]) -> StateFingerprint {
    let mut bytes = [0_u8; 32];
    for (index, word) in words.into_iter().enumerate() {
        bytes[index * 8..(index + 1) * 8].copy_from_slice(&word.to_le_bytes());
    }
    StateFingerprint::new(bytes)
}

pub(crate) fn mix64(mut value: u64) -> u64 {
    value ^= value >> 30;
    value = value.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value ^= value >> 27;
    value = value.wrapping_mul(0x94D0_49BB_1331_11EB);
    value ^ (value >> 31)
}

pub(crate) struct CanonicalDigest([u64; 4]);

impl CanonicalDigest {
    pub(crate) fn new() -> Self {
        Self([
            0x243F_6A88_85A3_08D3,
            0x1319_8A2E_0370_7344,
            0xA409_3822_299F_31D0,
            0x082E_FA98_EC4E_6C89,
        ])
    }

    pub(crate) fn write(&mut self, value: u64) {
        for (index, lane) in self.0.iter_mut().enumerate() {
            *lane = mix64(
                lane.wrapping_add(value.rotate_left((index as u32 * 13) + 1))
                    .wrapping_add(index as u64),
            );
        }
    }

    pub(crate) fn write_bytes(&mut self, bytes: [u8; 32]) {
        for chunk in bytes.chunks_exact(8) {
            self.write(u64::from_le_bytes(chunk.try_into().expect("exact chunk")));
        }
    }

    pub(crate) fn finish(self) -> StateFingerprint {
        fingerprint_words(self.0)
    }
}
