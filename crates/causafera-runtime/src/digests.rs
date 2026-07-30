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

/// Write one committed causal event into a history digest.
///
/// Shared by the incremental accumulator and the full-rescan oracle so the two
/// cannot drift apart: a field added here reaches both, and a differential test
/// that passes is then evidence about the accumulator rather than about two
/// copies of the same loop.
///
/// Deliberately does not touch `CausalTraceStore::children`, which
/// `commit_batch` back-patches onto already-committed parents — see
/// [`HistoryDigestPrefix`].
pub(crate) fn write_trace_event(digest: &mut CanonicalDigest, event: CausalEventRef<'_>) {
    digest.write(event.event_id.raw());
    digest.write(event.trace_id.raw());
    digest.write(event.time.raw());
    digest.write(u64::from(event.phase.id().0));
    digest.write(event.kind.raw());
    digest.write(event.causes.len() as u64);
    for cause in event.causes {
        digest.write(cause.raw());
    }
    digest.write(event.effects.len() as u64);
    for effect in event.effects {
        digest.write(effect.target().object_kind().raw());
        digest.write(effect.target().object_id());
        digest.write(effect.target().property().raw());
        digest.write_bytes(effect.before().bytes());
        digest.write_bytes(effect.after().bytes());
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

/// A chunk's identity within its chart.
///
/// The three axis terms must not be able to cancel each other, because the hash
/// keys object identity: two chunks that hash alike make their mana cells and
/// population aggregates the same object, which the snapshot validators reject.
/// A sign-extended `-1` is all ones on every axis, so the original form
/// collapsed `(-1, -1, 0)` onto `(0, 0, 0)` and `(-1, 0, 0)` onto `(0, -1, 0)`
/// the moment a chart had two dimensions.
///
/// The x term is unchanged and the off-line terms are zero at zero, so every
/// chunk of every line-shaped chart — which is every recorded fixture and every
/// replay-verified experiment — keeps exactly the identity it had.
pub(crate) fn chart_chunk_hash(chunk: ChartChunkCoord) -> u64 {
    mix64(
        chunk.chart.raw()
            ^ (chunk.chunk.x as u64).rotate_left(7)
            ^ off_line_axis(chunk.chunk.y, 19)
            ^ off_line_axis(chunk.chunk.z, 31),
    )
}

/// An axis term that is zero at the origin and cannot reproduce a sign-extended
/// x term: the lane is taken as thirty-two bits and folded against itself, so no
/// coordinate maps to all ones.
fn off_line_axis(value: i32, rotation: u32) -> u64 {
    let lane = u64::from(value as u32);
    lane.rotate_left(rotation) ^ lane.rotate_left(rotation.wrapping_add(22) % 64)
}

/// Write the canonical production bootstrap record into a state digest.
///
/// The record is authoritative equality input: two runs of the same seed and
/// recipe agree on it, and any change to a stage's parameters, targets, result,
/// or receipt ancestry moves the digest. The written bytes are an identity, never
/// a distance — `CURRENT_DIGEST_SCHEMA_VERSION` moved to 7 when this was added.
pub(crate) fn write_bootstrap_record(
    digest: &mut CanonicalDigest,
    bootstrap: &BootstrapRuntimeState,
) {
    let Some(record) = bootstrap.record() else {
        digest.write(0);
        return;
    };
    digest.write(1);
    let plan = record.plan();
    digest.write(plan.id().raw());
    digest.write(plan.world_seed());
    digest.write(plan.stages().len() as u64);
    for stage in plan.stages() {
        digest.write(stage.id().raw());
        digest.write(stage.process().raw());
        digest.write(stage.starts_at().raw());
        digest.write(stage.ends_at().raw());
        digest.write(u64::from(stage.detail_ordinal()));
        digest.write(stage.targets().len() as u64);
        for target in stage.targets() {
            digest.write(target.raw());
        }
        digest.write(stage.dependencies().len() as u64);
        for dependency in stage.dependencies() {
            digest.write(dependency.raw());
        }
        digest.write(stage.external_causes().len() as u64);
        for cause in stage.external_causes() {
            digest.write(cause.raw());
        }
        digest.write_bytes(stage.parameters().bytes());
    }
    digest.write(record.receipts().len() as u64);
    for receipt in record.receipts() {
        digest.write(receipt.stage().raw());
        digest.write(receipt.completed_at().raw());
        digest.write_bytes(receipt.result().bytes());
        digest.write(receipt.trace().raw());
        digest.write(receipt.causes().len() as u64);
        for cause in receipt.causes() {
            digest.write(cause.raw());
        }
    }
    digest.write(bootstrap.stage_results().len() as u64);
    for (stage, result) in bootstrap.stage_results() {
        digest.write(stage.raw());
        digest.write_bytes(result.bytes());
    }
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

fn fingerprint_words(words: [u64; 4]) -> StateFingerprint {
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

#[derive(Clone, Copy)]
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

    /// Write a length-prefixed byte slice.
    ///
    /// Length-prefixed so two adjacent slices cannot be confused for one longer
    /// one, and chunked the same way `write_bytes` is so a 32-byte fingerprint
    /// written either way agrees.
    pub(crate) fn write_slice(&mut self, bytes: &[u8]) {
        self.write(bytes.len() as u64);
        let mut chunks = bytes.chunks_exact(8);
        for chunk in &mut chunks {
            self.write(u64::from_le_bytes(chunk.try_into().expect("exact chunk")));
        }
        let tail = chunks.remainder();
        if !tail.is_empty() {
            let mut padded = [0_u8; 8];
            padded[..tail.len()].copy_from_slice(tail);
            self.write(u64::from_le_bytes(padded));
        }
    }

    pub(crate) fn finish(self) -> StateFingerprint {
        fingerprint_words(self.0)
    }
}

/// A `history_digest` accumulator that has already folded in a prefix of the
/// causal trace store.
///
/// `history_digest` used to re-walk every event ever committed on every call,
/// which meant every tick and every observer poll paid for the whole run so far
/// — measured at 46% of tick time in the first 64-tick batch of the smallest
/// workload and 87% by tick 512. `CanonicalDigest` is a pure streaming
/// accumulator, so the same sequence of `write` calls reaches the same state
/// whether it is performed in one pass or resumed across many, and
/// `history_digest` writes the trace store *first* with only bounded state after
/// it. Keeping the absorbed prefix here therefore reproduces the full rescan's
/// output exactly, with no digest schema change, while charging each call only
/// for what was committed since the previous one.
///
/// Three preconditions make that sound, all of them properties of
/// `CausalTraceStore` rather than assumptions about it:
///
/// - It is append-only. `commit_batch` is its only `&mut self` method, and it
///   only pushes; no path truncates, clears, removes, drains or retains, so an
///   event this prefix has folded in can never change afterwards.
/// - `commit_batch` never rewrites an earlier event's `cause_offsets` or
///   `effect_offsets` entries — it pushes new ones — so the causes and effects
///   an absorbed event contributed stay the bytes that were absorbed.
/// - **`children` is the one exception and must stay unread.** `commit_batch`
///   back-patches it, appending to an already-committed parent's entry, so
///   folding it into the prefix would be wrong. `history_digest` reads
///   `event_id`, `trace_id`, `time`, `phase`, `kind`, `causes` and `effects`,
///   and never `children`; adding it would silently break this.
#[derive(Clone)]
pub(crate) struct HistoryDigestPrefix {
    accumulator: CanonicalDigest,
    absorbed_events: usize,
}

impl HistoryDigestPrefix {
    /// An empty prefix carrying only the digest's fixed header.
    pub(crate) fn new() -> Self {
        let mut accumulator = CanonicalDigest::new();
        accumulator.write(u64::from(CURRENT_DIGEST_SCHEMA_VERSION.raw()));
        accumulator.write(HISTORY_DIGEST_DOMAIN);
        Self {
            accumulator,
            absorbed_events: 0,
        }
    }

    /// How many trace events are already folded in.
    ///
    /// Callers resume from exactly this index into the store.
    pub(crate) fn absorbed_events(&self) -> usize {
        self.absorbed_events
    }

    /// A copy of the accumulated prefix, to write further state into.
    ///
    /// Copying rather than draining is what keeps the prefix reusable: the
    /// bounded tail `history_digest` writes after the trace store differs every
    /// tick, so it goes into a throwaway copy and never into the prefix itself.
    pub(crate) fn resume(&self) -> CanonicalDigest {
        self.accumulator
    }

    /// Adopt `accumulator` as the prefix through `absorbed_events`.
    ///
    /// The caller must have produced it by folding exactly the store's events in
    /// `self.absorbed_events()..absorbed_events` into [`Self::resume`], in
    /// commit order and with nothing else written.
    pub(crate) fn advance(&mut self, accumulator: CanonicalDigest, absorbed_events: usize) {
        self.accumulator = accumulator;
        self.absorbed_events = absorbed_events;
    }
}

/// Write the complete hydrology state into a state digest.
///
/// Authoritative equality input under schema 8: two runs of the same seed and
/// recipe agree on every volume, every anchor, every edge, every boundary, every
/// forcing record, and the synthetic-node counter, and any change to one of them
/// moves the digest. The written bytes are an identity, never a distance.
///
/// A disabled domain writes a single zero, which is what makes "this world has no
/// water" a digestible claim rather than an absence.
pub(crate) fn write_hydrology_state(
    digest: &mut CanonicalDigest,
    hydrology: &crate::HydrologyRuntimeState,
) {
    if !hydrology.enabled {
        digest.write(0);
        return;
    }
    digest.write(1);
    digest.write(hydrology.next_node_id);
    digest.write(hydrology.fields.batch_sequence());
    digest.write(hydrology.fields.conservation_last_change().raw());
    digest.write(u64::from(hydrology.resolution_policy.enabled));
    digest.write(u64::from(hydrology.resolution_policy.max_level));

    digest.write(hydrology.metrics.len() as u64);
    for (chart, metric) in hydrology.metrics.entries() {
        digest.write(chart.raw());
        digest.write(metric.cell_area_mm2().get());
        digest.write(metric.orthogonal_edge_length_mm().get());
        digest.write(metric.timestep_millis().get());
    }

    digest.write(hydrology.fields.fields().len() as u64);
    for (chunk, field) in hydrology.fields.fields() {
        write_chart_chunk(digest, *chunk);
        for cell in field.cells() {
            let storage = cell.storage();
            digest.write(storage.surface.get());
            digest.write(storage.soil.get());
            digest.write(storage.groundwater.get());
            digest.write(cell.surface_last_change().raw());
            digest.write(cell.soil_last_change().raw());
            digest.write(cell.groundwater_last_change().raw());
            digest.write_bytes(cell.forcing_input_fingerprint().bytes());
            digest.write(cell.forcing_last_change().raw());
        }
        for ground in field.substrate() {
            digest.write_slice(ground.constitutive_key().bytes());
        }
    }

    digest.write(hydrology.conveyance.len() as u64);
    for (key, edge) in hydrology.conveyance.edges() {
        digest.write_slice(&causafera_geography::HydrologyCarrierKey::Edge(*key).encode());
        digest.write(edge.storage().get());
        digest.write(edge.capacity().get());
        digest.write(edge.last_change().raw());
    }

    digest.write(hydrology.boundaries.len() as u64);
    for (face, condition) in hydrology.boundaries.records() {
        digest.write_slice(&causafera_geography::HydrologyCarrierKey::ExteriorFace(*face).encode());
        let (
            surface_kind,
            surface_head,
            surface_conductance,
            ground_kind,
            ground_head,
            ground_conductance,
        ) = condition.constitutive_kind();
        digest.write(u64::from(surface_kind));
        digest.write(surface_head as u64);
        digest.write(surface_conductance);
        digest.write(u64::from(ground_kind));
        digest.write(ground_head as u64);
        digest.write(ground_conductance);
    }

    digest.write(hydrology.forcing.len() as u64);
    for record in &hydrology.forcing {
        digest.write(record.scheduled_tick());
        digest.write(record.forcing_id());
        digest.write(record.precipitation_volume().get());
        digest.write(record.potential_et_volume().get());
        digest.write(record.external_inflow_volume().get());
        digest.write(record.origin_trace().raw());
        digest.write(record.producer_policy_schema());
        match record.applied_at() {
            None => digest.write(0),
            Some(tick) => {
                digest.write(1);
                digest.write(tick);
            }
        }
    }

    digest.write(hydrology.resolution.len() as u64);
    for (chunk, entry) in &hydrology.resolution {
        write_chart_chunk(digest, *chunk);
        digest.write(u64::from(entry.level()));
        digest.write(entry.last_change().raw());
    }

    // The registry's ordinals are authoritative: they are what every causal target
    // in the store was written against, so two states that agree on their water but
    // disagree on their addressing are not the same state.
    digest.write(hydrology.registry.cells().len() as u64);
    digest.write(hydrology.registry.edges().len() as u64);
    for (edge, ordinal) in hydrology.registry.edges() {
        digest.write_slice(&causafera_geography::HydrologyCarrierKey::Edge(*edge).encode());
        digest.write(*ordinal);
    }
    digest.write(hydrology.registry.forcing().len() as u64);
    for ((scheduled_tick, forcing_id), ordinal) in hydrology.registry.forcing() {
        digest.write(*scheduled_tick);
        digest.write(*forcing_id);
        digest.write(*ordinal);
    }

    // Retained typed batches are digested by their conservation ledgers. The
    // per-transfer receipts are evictable detail; the ledger each batch closed is
    // the claim about the world.
    digest.write(hydrology.retained_batches.len() as u64);
    for trace in &hydrology.retained_batches {
        digest.write(trace.raw());
        match hydrology.conservation_receipts.get(trace) {
            None => digest.write(0),
            Some(receipt) => {
                digest.write(1);
                digest.write(receipt.tick());
                digest.write(receipt.batch_sequence());
                for term in [
                    receipt.surface_before(),
                    receipt.soil_before(),
                    receipt.groundwater_before(),
                    receipt.conveyance_before(),
                    receipt.surface_after(),
                    receipt.soil_after(),
                    receipt.groundwater_after(),
                    receipt.conveyance_after(),
                    receipt.accepted_precipitation(),
                    receipt.accepted_external_inflow(),
                    receipt.accepted_evapotranspiration(),
                    receipt.boundary_exports(),
                    receipt.residual(),
                ] {
                    digest.write_slice(&term.to_be_bytes());
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use causafera_types::{ChunkCoord, SpatialChartId};

    use super::*;

    fn chunk(x: i32, y: i32, z: i32) -> ChartChunkCoord {
        ChartChunkCoord::new(SpatialChartId::new(1), ChunkCoord::new(x, y, z))
    }

    /// Object identity keyed by this hash is only sound while it separates
    /// chunks, and the mana validator compares hashes exclusive-ored with a cell
    /// index, so near-collisions in the low bits are collisions too.
    #[test]
    fn chunk_identity_separates_every_chunk_of_the_largest_block() {
        let radius = 4;
        let mut seen = std::collections::BTreeMap::new();
        for z in -radius..=radius {
            for y in -radius..=radius {
                for x in -radius..=radius {
                    let hash = chart_chunk_hash(chunk(x, y, z));
                    if let Some(other) = seen.insert(hash, (x, y, z)) {
                        panic!("({x}, {y}, {z}) collides with {other:?}");
                    }
                }
            }
        }
        // The mana lattice runs to 32³ cells, so any two chunk identities must
        // differ above the fifteen bits a cell index can occupy.
        let hashes = seen.keys().copied().collect::<Vec<_>>();
        for (index, left) in hashes.iter().enumerate() {
            for right in &hashes[index + 1..] {
                assert!(
                    left ^ right >= 1 << 15,
                    "chunk identities {left} and {right} are within one cell index"
                );
            }
        }
    }

    /// Every recorded fixture was produced against a line-shaped chart. The
    /// identity of those chunks may not move.
    #[test]
    fn line_shaped_charts_keep_the_identity_they_were_recorded_with() {
        for x in -4..=4 {
            assert_eq!(
                chart_chunk_hash(chunk(x, 0, 0)),
                mix64(SpatialChartId::new(1).raw() ^ (x as u64).rotate_left(7)),
            );
        }
    }
}
