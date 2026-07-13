use ontopolis_core::phases::Phase;
use ontopolis_core::provenance::{CausalEffect, CausalEventSnapshot, CausalTraceSnapshot};
use ontopolis_domains::{ManaFieldSetSnapshot, ManaFieldSnapshot};
use ontopolis_persistence::{
    LittleEndianDecoder, LittleEndianEncoder, PersistenceError, SectionPayload, SnapshotEnvelope,
    SnapshotHeader, FORMAT_MAJOR_V1, FORMAT_MINOR_V1,
};
use ontopolis_resolution::{
    ChannelWeight, ResolutionEntry, ResolutionFieldSnapshot, ResolutionPolicySnapshot,
};
use ontopolis_types::{
    AngularVelocity, AttentionTargetId, ChartChunkCoord, ChunkCoord, Direction3D, EventId,
    EventKindId, FeatureRelation, FeatureValue, LocalCoord, ManaFieldId, PerceivedObjectId,
    PerceptId, PhysicalPatternId, ResolutionChannelId, ResolutionFieldId, SelfAssociationId,
    SimulationTime, SpatialChartId, StateObjectKindId, StatePropertyId, SubjectiveBodyPartId,
    TraceId, Velocity, WorldCoord,
};

use crate::{
    ActionKindId, ActionProposal, ActionRejection, ActionValidationResult, ActiveChunkSnapshot,
    ActorId, ActorObjectiveSnapshot, ActorObjectiveStateSnapshot, ActorPhysicalObject,
    ActorSubjectiveSnapshot, ActorSubjectiveStateSnapshot, BootstrapReceiptRecord,
    BootstrapReceiptSnapshot, CarrierAdapterConfig, ExperimentManifestSnapshot, GenericFeature,
    MinimalBodyState, PatternHistorySnapshot, PerceivedSelf, PhysicalCountersSnapshot,
    PopulationAggregate, PopulationAggregateSnapshot, RuntimeConfig, RuntimeRecipeSnapshot,
    RuntimeSnapshotData, RuntimeState, SensorAperture, SensorKindId, SpatialChunkSnapshot,
    SubjectiveSceneSnapshot, SubjectiveTarget, SystemRegistrationSnapshot, TerrainCarrierSnapshot,
};

pub const SECTION_RUNTIME_RECIPE: u16 = 0x0001;
pub const SECTION_SPATIAL_CHUNKS: u16 = 0x0002;
pub const SECTION_MANA_FIELDS: u16 = 0x0003;
pub const SECTION_RESOLUTION_FIELD: u16 = 0x0004;
pub const SECTION_PATTERN_HISTORY: u16 = 0x0005;
pub const SECTION_PHYSICAL_COUNTERS: u16 = 0x0006;
pub const SECTION_ACTOR_OBJECTIVE: u16 = 0x0007;
pub const SECTION_ACTOR_SUBJECTIVE: u16 = 0x0008;
pub const SECTION_POPULATION_BOOTSTRAP: u16 = 0x0009;
pub const SECTION_CAUSAL_TRACES: u16 = 0x000A;
pub const SECTION_EXPERIMENT_MANIFEST: u16 = 0x000B;

/// Encode a `CausalTraceSnapshot` into section bytes.
pub fn encode_trace_section(snapshot: &CausalTraceSnapshot) -> Vec<u8> {
    let mut buf = Vec::new();
    let mut enc = LittleEndianEncoder::new(&mut buf);

    enc.write_u64(snapshot.next_event_id);
    enc.write_u64(snapshot.next_trace_id);
    enc.write_u64(snapshot.events.len() as u64);

    // Encode forward SoA arrays for compactness and canonical ordering.
    for event in &snapshot.events {
        enc.write_u64(event.event_id.raw());
    }
    for event in &snapshot.events {
        enc.write_u64(event.trace_id.raw());
    }
    for event in &snapshot.events {
        enc.write_u64(event.time.raw());
    }
    for event in &snapshot.events {
        enc.write_u16(u16::from(event.phase.id().0));
    }
    for event in &snapshot.events {
        enc.write_u64(event.kind.raw());
    }

    // Cause offsets (event_count + 1 entries).
    let mut cause_offset: u32 = 0;
    enc.write_u32(cause_offset);
    for event in &snapshot.events {
        cause_offset += event.causes.len() as u32;
        enc.write_u32(cause_offset);
    }

    // Flat causes.
    for event in &snapshot.events {
        for cause in &event.causes {
            enc.write_u64(cause.raw());
        }
    }

    // Effect offsets (event_count + 1 entries).
    let mut effect_offset: u32 = 0;
    enc.write_u32(effect_offset);
    for event in &snapshot.events {
        effect_offset += event.effects.len() as u32;
        enc.write_u32(effect_offset);
    }

    // Flat effects.
    for event in &snapshot.events {
        for effect in &event.effects {
            enc.write_u64(effect.target().object_kind().raw());
            enc.write_u64(effect.target().object_id());
            enc.write_u64(effect.target().property().raw());
            enc.write_fixed(&effect.before().bytes());
            enc.write_fixed(&effect.after().bytes());
        }
    }

    buf
}

/// Decode section bytes into a `CausalTraceSnapshot`.
pub fn decode_trace_section(bytes: &[u8]) -> Result<CausalTraceSnapshot, PersistenceError> {
    let mut dec = LittleEndianDecoder::new(bytes);

    let next_event_id = dec.read_u64()?;
    let next_trace_id = dec.read_u64()?;
    let event_count = dec.read_u64()? as usize;

    if event_count > u32::MAX as usize {
        return Err(PersistenceError::codec("event count exceeds u32"));
    }

    let mut events = Vec::with_capacity(event_count);

    // Read event_ids.
    let mut event_ids = Vec::with_capacity(event_count);
    for _ in 0..event_count {
        event_ids.push(EventId::new(dec.read_u64()?));
    }

    // Read trace_ids.
    let mut trace_ids = Vec::with_capacity(event_count);
    for _ in 0..event_count {
        trace_ids.push(TraceId::new(dec.read_u64()?));
    }

    // Read times.
    let mut times = Vec::with_capacity(event_count);
    for _ in 0..event_count {
        times.push(SimulationTime::new(dec.read_u64()?));
    }

    // Read phases.
    let mut phases = Vec::with_capacity(event_count);
    for _ in 0..event_count {
        let phase_id = dec.read_u16()?;
        let phase = Phase::ALL
            .iter()
            .find(|p| p.id().0 == phase_id as u8)
            .copied()
            .ok_or_else(|| PersistenceError::codec(format!("unknown phase id {phase_id}")))?;
        phases.push(phase);
    }

    // Read kinds.
    let mut kinds = Vec::with_capacity(event_count);
    for _ in 0..event_count {
        kinds.push(EventKindId::new(dec.read_u64()?));
    }

    // Read cause offsets.
    let mut cause_offsets = Vec::with_capacity(event_count + 1);
    for _ in 0..=event_count {
        cause_offsets.push(dec.read_u32()?);
    }

    // Read flat causes.
    let total_causes = cause_offsets.last().copied().unwrap_or(0) as usize;
    let mut causes = Vec::with_capacity(total_causes);
    for _ in 0..total_causes {
        causes.push(TraceId::new(dec.read_u64()?));
    }

    // Read effect offsets.
    let mut effect_offsets = Vec::with_capacity(event_count + 1);
    for _ in 0..=event_count {
        effect_offsets.push(dec.read_u32()?);
    }

    // Read flat effects.
    let total_effects = effect_offsets.last().copied().unwrap_or(0) as usize;
    let mut effects = Vec::with_capacity(total_effects);
    for _ in 0..total_effects {
        let object_kind = StateObjectKindId::new(dec.read_u64()?);
        let object_id = dec.read_u64()?;
        let property = StatePropertyId::new(dec.read_u64()?);
        let target =
            ontopolis_core::provenance::CausalTarget::new(object_kind, object_id, property);
        let before = ontopolis_core::provenance::StateFingerprint::new(*dec.read_fixed::<32>()?);
        let after = ontopolis_core::provenance::StateFingerprint::new(*dec.read_fixed::<32>()?);
        effects.push(
            CausalEffect::new(target, before, after)
                .map_err(|e| PersistenceError::codec(format!("invalid causal effect: {e}")))?,
        );
    }

    // Reassemble events from SoA arrays.
    for index in 0..event_count {
        let cause_start = cause_offsets[index] as usize;
        let cause_end = cause_offsets[index + 1] as usize;
        let effect_start = effect_offsets[index] as usize;
        let effect_end = effect_offsets[index + 1] as usize;
        events.push(CausalEventSnapshot {
            event_id: event_ids[index],
            trace_id: trace_ids[index],
            time: times[index],
            phase: phases[index],
            kind: kinds[index],
            causes: causes[cause_start..cause_end].to_vec(),
            effects: effects[effect_start..effect_end].to_vec(),
        });
    }

    Ok(CausalTraceSnapshot {
        next_event_id,
        next_trace_id,
        events,
    })
}

/// Convenience: encode a live trace store directly into section bytes.
pub fn encode_trace_store(store: &ontopolis_core::provenance::CausalTraceStore) -> Vec<u8> {
    encode_trace_section(&store.export_snapshot())
}

/// Convenience: decode section bytes and import into a live trace store.
pub fn decode_trace_store(
    bytes: &[u8],
) -> Result<ontopolis_core::provenance::CausalTraceStore, PersistenceError> {
    let snapshot = decode_trace_section(bytes)?;
    ontopolis_core::provenance::CausalTraceStore::import_snapshot(snapshot)
        .map_err(|e| PersistenceError::codec(format!("import failed: {e}")))
}

pub fn encode_runtime_recipe_section(snapshot: &RuntimeRecipeSnapshot) -> Vec<u8> {
    let mut buf = Vec::new();
    let mut enc = LittleEndianEncoder::new(&mut buf);
    enc.write_u64(snapshot.seed);
    encode_runtime_config(&mut enc, &snapshot.config);
    enc.write_u64(snapshot.completed_time.raw());
    enc.write_u64(snapshot.system_registrations.len() as u64);
    for registration in &snapshot.system_registrations {
        enc.write_u16(u16::from(registration.phase.id().0));
        enc.write_u64(registration.system_schema_id);
        enc.write_u16(registration.revision);
        enc.write_u16(registration.registration_order);
    }
    buf
}

pub fn decode_runtime_recipe_section(
    bytes: &[u8],
) -> Result<RuntimeRecipeSnapshot, PersistenceError> {
    let mut dec = LittleEndianDecoder::new(bytes);
    let seed = dec.read_u64()?;
    let config = decode_runtime_config(&mut dec)?;
    let completed_time = SimulationTime::new(dec.read_u64()?);
    let count = read_count(&mut dec, 256, "system registration")?;
    let mut system_registrations = Vec::with_capacity(count);
    for _ in 0..count {
        system_registrations.push(SystemRegistrationSnapshot {
            phase: decode_phase(dec.read_u16()?)?,
            system_schema_id: dec.read_u64()?,
            revision: dec.read_u16()?,
            registration_order: dec.read_u16()?,
        });
    }
    require_empty(&dec)?;
    Ok(RuntimeRecipeSnapshot {
        seed,
        config,
        system_registrations,
        completed_time,
    })
}

pub fn encode_spatial_section(snapshot: &SpatialChunkSnapshot) -> Vec<u8> {
    let mut buf = Vec::new();
    let mut enc = LittleEndianEncoder::new(&mut buf);
    enc.write_u64(snapshot.active_chunks.len() as u64);
    for chunk in &snapshot.active_chunks {
        encode_chart_chunk(&mut enc, chunk.chunk);
        enc.write_i64(chunk.relevance);
        enc.write_u8(chunk.level);
        enc.write_i64(chunk.total_mana);
        enc.write_u64(chunk.event_count);
        encode_option_trace(&mut enc, chunk.last_transition);
    }
    enc.write_u64(snapshot.carrier_adapters.len() as u64);
    for carrier in &snapshot.carrier_adapters {
        encode_terrain_carrier(&mut enc, carrier);
    }
    buf
}

pub fn decode_spatial_section(bytes: &[u8]) -> Result<SpatialChunkSnapshot, PersistenceError> {
    let mut dec = LittleEndianDecoder::new(bytes);
    let active_count = read_count(&mut dec, 65_536, "active chunk")?;
    let mut active_chunks = Vec::with_capacity(active_count);
    for _ in 0..active_count {
        active_chunks.push(ActiveChunkSnapshot {
            chunk: decode_chart_chunk(&mut dec)?,
            relevance: dec.read_i64()?,
            level: dec.read_u8()?,
            total_mana: dec.read_i64()?,
            event_count: dec.read_u64()?,
            last_transition: decode_option_trace(&mut dec)?,
        });
    }
    reject_unsorted_chunks(active_chunks.iter().map(|entry| entry.chunk))?;
    let carrier_count = read_count(&mut dec, 65_536, "terrain carrier")?;
    let mut carrier_adapters = Vec::with_capacity(carrier_count);
    for _ in 0..carrier_count {
        carrier_adapters.push(decode_terrain_carrier(&mut dec)?);
    }
    reject_unsorted_chunks(carrier_adapters.iter().map(|entry| entry.chunk))?;
    require_empty(&dec)?;
    Ok(SpatialChunkSnapshot {
        active_chunks,
        carrier_adapters,
    })
}

pub fn encode_mana_section(snapshot: &ManaFieldSetSnapshot) -> Vec<u8> {
    let mut buf = Vec::new();
    let mut enc = LittleEndianEncoder::new(&mut buf);
    enc.write_u64(snapshot.fields.len() as u64);
    for field in &snapshot.fields {
        enc.write_u64(field.id.raw());
        encode_chart_chunk(&mut enc, field.chunk);
        enc.write_u8(field.extent);
        enc.write_u64(field.observed_through.raw());
        enc.write_u64(field.intensity.len() as u64);
        for value in &field.intensity {
            enc.write_i64(*value);
        }
        enc.write_u64(field.last_change.len() as u64);
        for trace in &field.last_change {
            encode_option_trace(&mut enc, *trace);
        }
    }
    buf
}

pub fn decode_mana_section(bytes: &[u8]) -> Result<ManaFieldSetSnapshot, PersistenceError> {
    let mut dec = LittleEndianDecoder::new(bytes);
    let field_count = read_count(&mut dec, 65_536, "mana field")?;
    let mut fields = Vec::with_capacity(field_count);
    for _ in 0..field_count {
        let id = ManaFieldId::new(dec.read_u64()?);
        let chunk = decode_chart_chunk(&mut dec)?;
        let extent = dec.read_u8()?;
        let observed_through = SimulationTime::new(dec.read_u64()?);
        let intensity_count = read_count(&mut dec, 32 * 32 * 32, "mana intensity")?;
        let mut intensity = Vec::with_capacity(intensity_count);
        for _ in 0..intensity_count {
            intensity.push(dec.read_i64()?);
        }
        let trace_count = read_count(&mut dec, 32 * 32 * 32, "mana trace")?;
        let mut last_change = Vec::with_capacity(trace_count);
        for _ in 0..trace_count {
            last_change.push(decode_option_trace(&mut dec)?);
        }
        fields.push(ManaFieldSnapshot {
            id,
            chunk,
            extent,
            observed_through,
            intensity,
            last_change,
        });
    }
    reject_unsorted_chunks(fields.iter().map(|entry| entry.chunk))?;
    require_empty(&dec)?;
    Ok(ManaFieldSetSnapshot { fields })
}

pub fn encode_resolution_section(
    field: &ResolutionFieldSnapshot,
    policy: &ResolutionPolicySnapshot,
) -> Vec<u8> {
    let mut buf = Vec::new();
    let mut enc = LittleEndianEncoder::new(&mut buf);
    enc.write_u64(field.id.raw());
    enc.write_u64(field.evaluated_through.raw());
    enc.write_u64(field.entries.len() as u64);
    for entry in &field.entries {
        encode_chart_chunk(&mut enc, entry.chunk);
        enc.write_i64(entry.relevance);
        enc.write_u8(entry.level);
        enc.write_u64(entry.last_trace.raw());
    }
    enc.write_i64(policy.maximum_relevance);
    enc.write_i64(policy.retained_relevance);
    enc.write_i64(policy.hysteresis);
    enc.write_u64(policy.thresholds.len() as u64);
    for threshold in &policy.thresholds {
        enc.write_i64(*threshold);
    }
    enc.write_u64(policy.channels.len() as u64);
    for channel in &policy.channels {
        enc.write_u64(channel.channel().raw());
        enc.write_i64(channel.weight());
    }
    buf
}

pub fn decode_resolution_section(
    bytes: &[u8],
) -> Result<(ResolutionFieldSnapshot, ResolutionPolicySnapshot), PersistenceError> {
    let mut dec = LittleEndianDecoder::new(bytes);
    let id = ResolutionFieldId::new(dec.read_u64()?);
    let evaluated_through = SimulationTime::new(dec.read_u64()?);
    let entry_count = read_count(&mut dec, 65_536, "resolution entry")?;
    let mut entries = Vec::with_capacity(entry_count);
    for _ in 0..entry_count {
        entries.push(ResolutionEntry {
            chunk: decode_chart_chunk(&mut dec)?,
            relevance: dec.read_i64()?,
            level: dec.read_u8()?,
            last_trace: TraceId::new(dec.read_u64()?),
        });
    }
    reject_unsorted_chunks(entries.iter().map(|entry| entry.chunk))?;
    let maximum_relevance = dec.read_i64()?;
    let retained_relevance = dec.read_i64()?;
    let hysteresis = dec.read_i64()?;
    let threshold_count = read_count(&mut dec, 16, "resolution threshold")?;
    let mut thresholds = Vec::with_capacity(threshold_count);
    for _ in 0..threshold_count {
        thresholds.push(dec.read_i64()?);
    }
    let channel_count = read_count(&mut dec, 64, "resolution channel")?;
    let mut channels = Vec::with_capacity(channel_count);
    for _ in 0..channel_count {
        let channel = ResolutionChannelId::new(dec.read_u64()?);
        let weight = dec.read_i64()?;
        channels.push(
            ChannelWeight::new(channel, weight)
                .map_err(|e| PersistenceError::codec(format!("invalid channel: {e}")))?,
        );
    }
    require_empty(&dec)?;
    Ok((
        ResolutionFieldSnapshot {
            id,
            evaluated_through,
            entries,
        },
        ResolutionPolicySnapshot {
            maximum_relevance,
            retained_relevance,
            hysteresis,
            thresholds,
            channels,
        },
    ))
}

pub fn encode_pattern_history_section(snapshot: &PatternHistorySnapshot) -> Vec<u8> {
    let mut buf = Vec::new();
    let mut enc = LittleEndianEncoder::new(&mut buf);
    enc.write_u64(snapshot.global_cap as u64);
    enc.write_u64(snapshot.per_pattern_cap as u64);
    enc.write_u64(snapshot.samples.len() as u64);
    for sample in &snapshot.samples {
        encode_pattern_sample(&mut enc, sample);
    }
    buf
}

pub fn decode_pattern_history_section(
    bytes: &[u8],
) -> Result<PatternHistorySnapshot, PersistenceError> {
    let mut dec = LittleEndianDecoder::new(bytes);
    let global_cap = usize_from_u64(dec.read_u64()?, "global pattern cap")?;
    let per_pattern_cap = usize_from_u64(dec.read_u64()?, "per-pattern cap")?;
    let count = read_count(&mut dec, global_cap.max(1), "pattern sample")?;
    let mut samples = Vec::with_capacity(count);
    for _ in 0..count {
        samples.push(decode_pattern_sample(&mut dec)?);
    }
    require_empty(&dec)?;
    Ok(PatternHistorySnapshot {
        samples,
        global_cap,
        per_pattern_cap,
    })
}

pub fn encode_physical_counters_section(snapshot: &PhysicalCountersSnapshot) -> Vec<u8> {
    let mut buf = Vec::new();
    let mut enc = LittleEndianEncoder::new(&mut buf);
    enc.write_u64(snapshot.pending_samples.len() as u64);
    for sample in &snapshot.pending_samples {
        encode_pattern_sample(&mut enc, sample);
    }
    enc.write_u64(snapshot.latest_physical_trace.raw());
    encode_option_trace(&mut enc, snapshot.latest_mana_trace);
    enc.write_u64(snapshot.advanced_through.raw());
    for value in [
        snapshot.physical_counter,
        snapshot.physical_events,
        snapshot.mana_cell_changes,
        snapshot.mana_physical_effects,
        snapshot.resolution_changes,
        snapshot.resolution_transitions,
        snapshot.perceived_actor_features,
        snapshot.subjective_actor_objects,
        snapshot.actor_actions_committed,
        snapshot.actor_actions_rejected,
        snapshot.population_births,
        snapshot.population_deaths,
        snapshot.population_movements,
        snapshot.actor_promotions,
        snapshot.actor_demotions,
        snapshot.material_activity_events,
        snapshot.next_actor_id,
    ] {
        enc.write_u64(value);
    }
    enc.write_u32(snapshot.last_mana_changes);
    encode_bool(&mut enc, snapshot.mana_effect_active);
    enc.write_u32(snapshot.physical_mana_effect_boost);
    buf
}

pub fn decode_physical_counters_section(
    bytes: &[u8],
) -> Result<PhysicalCountersSnapshot, PersistenceError> {
    let mut dec = LittleEndianDecoder::new(bytes);
    let pending_count = read_count(&mut dec, 4_096, "pending sample")?;
    let mut pending_samples = Vec::with_capacity(pending_count);
    for _ in 0..pending_count {
        pending_samples.push(decode_pattern_sample(&mut dec)?);
    }
    let latest_physical_trace = TraceId::new(dec.read_u64()?);
    let latest_mana_trace = decode_option_trace(&mut dec)?;
    let advanced_through = SimulationTime::new(dec.read_u64()?);
    let physical_counter = dec.read_u64()?;
    let physical_events = dec.read_u64()?;
    let mana_cell_changes = dec.read_u64()?;
    let mana_physical_effects = dec.read_u64()?;
    let resolution_changes = dec.read_u64()?;
    let resolution_transitions = dec.read_u64()?;
    let perceived_actor_features = dec.read_u64()?;
    let subjective_actor_objects = dec.read_u64()?;
    let actor_actions_committed = dec.read_u64()?;
    let actor_actions_rejected = dec.read_u64()?;
    let population_births = dec.read_u64()?;
    let population_deaths = dec.read_u64()?;
    let population_movements = dec.read_u64()?;
    let actor_promotions = dec.read_u64()?;
    let actor_demotions = dec.read_u64()?;
    let material_activity_events = dec.read_u64()?;
    let next_actor_id = dec.read_u64()?;
    let last_mana_changes = dec.read_u32()?;
    let mana_effect_active = decode_bool(&mut dec)?;
    let physical_mana_effect_boost = dec.read_u32()?;
    require_empty(&dec)?;
    Ok(PhysicalCountersSnapshot {
        pending_samples,
        latest_physical_trace,
        latest_mana_trace,
        advanced_through,
        physical_counter,
        physical_events,
        mana_cell_changes,
        mana_physical_effects,
        resolution_changes,
        resolution_transitions,
        perceived_actor_features,
        subjective_actor_objects,
        actor_actions_committed,
        actor_actions_rejected,
        population_births,
        population_deaths,
        population_movements,
        actor_promotions,
        actor_demotions,
        material_activity_events,
        next_actor_id,
        last_mana_changes,
        mana_effect_active,
        physical_mana_effect_boost,
    })
}

pub fn encode_actor_objective_section(snapshot: &ActorObjectiveStateSnapshot) -> Vec<u8> {
    let mut buf = Vec::new();
    let mut enc = LittleEndianEncoder::new(&mut buf);
    enc.write_i64(snapshot.actor_action_bounds);
    enc.write_u64(snapshot.actors.len() as u64);
    for (actor, state) in &snapshot.actors {
        enc.write_u64(actor.raw());
        encode_actor_objective(&mut enc, state);
    }
    enc.write_u64(snapshot.actor_ancestry.len() as u64);
    for (actor, ancestry) in &snapshot.actor_ancestry {
        enc.write_u64(actor.raw());
        encode_trace_vec(&mut enc, ancestry);
    }
    enc.write_u64(snapshot.actor_objects.len() as u64);
    for (key, object) in &snapshot.actor_objects {
        enc.write_u64(*key);
        encode_actor_object(&mut enc, object);
    }
    buf
}

pub fn decode_actor_objective_section(
    bytes: &[u8],
) -> Result<ActorObjectiveStateSnapshot, PersistenceError> {
    let mut dec = LittleEndianDecoder::new(bytes);
    let actor_action_bounds = dec.read_i64()?;
    let actor_count = read_count(&mut dec, 128, "actor objective")?;
    let mut actors = Vec::with_capacity(actor_count);
    for _ in 0..actor_count {
        actors.push((
            ActorId::new(dec.read_u64()?),
            decode_actor_objective(&mut dec)?,
        ));
    }
    reject_unsorted_ids(actors.iter().map(|(id, _)| id.raw()))?;
    let ancestry_count = read_count(&mut dec, 128, "actor ancestry")?;
    let mut actor_ancestry = Vec::with_capacity(ancestry_count);
    for _ in 0..ancestry_count {
        actor_ancestry.push((
            ActorId::new(dec.read_u64()?),
            decode_trace_vec(&mut dec, 4_096)?,
        ));
    }
    reject_unsorted_ids(actor_ancestry.iter().map(|(id, _)| id.raw()))?;
    let object_count = read_count(&mut dec, 1_024, "actor object")?;
    let mut actor_objects = Vec::with_capacity(object_count);
    for _ in 0..object_count {
        actor_objects.push((dec.read_u64()?, decode_actor_object(&mut dec)?));
    }
    reject_unsorted_ids(actor_objects.iter().map(|(id, _)| *id))?;
    require_empty(&dec)?;
    Ok(ActorObjectiveStateSnapshot {
        actors,
        actor_ancestry,
        actor_objects,
        actor_action_bounds,
    })
}

pub fn encode_actor_subjective_section(snapshot: &ActorSubjectiveStateSnapshot) -> Vec<u8> {
    let mut buf = Vec::new();
    let mut enc = LittleEndianEncoder::new(&mut buf);
    enc.write_u64(snapshot.actors.len() as u64);
    for (actor, state) in &snapshot.actors {
        enc.write_u64(actor.raw());
        encode_actor_subjective(&mut enc, state);
    }
    buf
}

pub fn decode_actor_subjective_section(
    bytes: &[u8],
) -> Result<ActorSubjectiveStateSnapshot, PersistenceError> {
    let mut dec = LittleEndianDecoder::new(bytes);
    let actor_count = read_count(&mut dec, 128, "actor subjective")?;
    let mut actors = Vec::with_capacity(actor_count);
    for _ in 0..actor_count {
        actors.push((
            ActorId::new(dec.read_u64()?),
            decode_actor_subjective(&mut dec)?,
        ));
    }
    reject_unsorted_ids(actors.iter().map(|(id, _)| id.raw()))?;
    require_empty(&dec)?;
    Ok(ActorSubjectiveStateSnapshot { actors })
}

pub fn encode_population_section(
    population: &PopulationAggregateSnapshot,
    bootstrap: &BootstrapReceiptSnapshot,
) -> Vec<u8> {
    let mut buf = Vec::new();
    let mut enc = LittleEndianEncoder::new(&mut buf);
    enc.write_u64(population.aggregates.len() as u64);
    for aggregate in &population.aggregates {
        encode_population_aggregate(&mut enc, aggregate);
    }
    enc.write_u64(population.aggregate_actor_pool.len() as u64);
    for (chunk, actors) in &population.aggregate_actor_pool {
        encode_chart_chunk(&mut enc, *chunk);
        enc.write_u64(actors.len() as u64);
        for actor in actors {
            enc.write_u64(actor.raw());
        }
    }
    enc.write_u64(bootstrap.receipts.len() as u64);
    for receipt in &bootstrap.receipts {
        enc.write_u64(receipt.stage.raw());
        enc.write_u64(receipt.trace.raw());
    }
    buf
}

pub fn decode_population_section(
    bytes: &[u8],
) -> Result<(PopulationAggregateSnapshot, BootstrapReceiptSnapshot), PersistenceError> {
    let mut dec = LittleEndianDecoder::new(bytes);
    let aggregate_count = read_count(&mut dec, 65_536, "population aggregate")?;
    let mut aggregates = Vec::with_capacity(aggregate_count);
    for _ in 0..aggregate_count {
        aggregates.push(decode_population_aggregate(&mut dec)?);
    }
    reject_unsorted_chunks(aggregates.iter().map(|entry| entry.chart))?;
    let pool_count = read_count(&mut dec, 65_536, "aggregate actor pool")?;
    let mut aggregate_actor_pool = Vec::with_capacity(pool_count);
    for _ in 0..pool_count {
        let chunk = decode_chart_chunk(&mut dec)?;
        let actor_count = read_count(&mut dec, 128, "pooled actor")?;
        let mut actors = Vec::with_capacity(actor_count);
        for _ in 0..actor_count {
            actors.push(ActorId::new(dec.read_u64()?));
        }
        aggregate_actor_pool.push((chunk, actors));
    }
    reject_unsorted_chunks(aggregate_actor_pool.iter().map(|(chunk, _)| *chunk))?;
    let receipt_count = read_count(&mut dec, 4_096, "bootstrap receipt")?;
    let mut receipts = Vec::with_capacity(receipt_count);
    for _ in 0..receipt_count {
        receipts.push(BootstrapReceiptRecord {
            stage: ontopolis_types::HistoricalStageId::new(dec.read_u64()?),
            trace: TraceId::new(dec.read_u64()?),
        });
    }
    require_empty(&dec)?;
    Ok((
        PopulationAggregateSnapshot {
            aggregates,
            aggregate_actor_pool,
        },
        BootstrapReceiptSnapshot { receipts },
    ))
}

pub fn encode_experiment_manifest_section(snapshot: &ExperimentManifestSnapshot) -> Vec<u8> {
    let mut buf = Vec::new();
    let mut enc = LittleEndianEncoder::new(&mut buf);
    enc.write_u16(snapshot.format_version);
    enc.write_u64(snapshot.seed_set.len() as u64);
    for seed in &snapshot.seed_set {
        enc.write_u64(*seed);
    }
    enc.write_u64(snapshot.checkpoint_interval);
    enc.write_u64(snapshot.bootstrap_population);
    enc.write_u64(snapshot.suppression_from.raw());
    enc.write_u64(snapshot.suppression_through.raw());
    enc.write_u64(snapshot.warm_up_ticks);
    enc.write_u64(snapshot.duration_ticks);
    encode_physical_digest(&mut enc, snapshot.physical_digest);
    encode_history_digest(&mut enc, snapshot.history_digest);
    encode_trace_vec(&mut enc, &snapshot.supporting_traces);
    encode_bool(&mut enc, snapshot.evidence_sufficient);
    buf
}

pub fn decode_experiment_manifest_section(
    bytes: &[u8],
) -> Result<ExperimentManifestSnapshot, PersistenceError> {
    let mut dec = LittleEndianDecoder::new(bytes);
    let format_version = dec.read_u16()?;
    let seed_count = read_count(&mut dec, 1_024, "experiment seed")?;
    let mut seed_set = Vec::with_capacity(seed_count);
    for _ in 0..seed_count {
        seed_set.push(dec.read_u64()?);
    }
    let checkpoint_interval = dec.read_u64()?;
    let bootstrap_population = dec.read_u64()?;
    let suppression_from = SimulationTime::new(dec.read_u64()?);
    let suppression_through = SimulationTime::new(dec.read_u64()?);
    let warm_up_ticks = dec.read_u64()?;
    let duration_ticks = dec.read_u64()?;
    let physical_digest = decode_physical_digest(&mut dec)?;
    let history_digest = decode_history_digest(&mut dec)?;
    let supporting_traces = decode_trace_vec(&mut dec, 4_096)?;
    let evidence_sufficient = decode_bool(&mut dec)?;
    require_empty(&dec)?;
    Ok(ExperimentManifestSnapshot {
        format_version,
        seed_set,
        checkpoint_interval,
        bootstrap_population,
        suppression_from,
        suppression_through,
        warm_up_ticks,
        duration_ticks,
        physical_digest,
        history_digest,
        supporting_traces,
        evidence_sufficient,
    })
}

fn encode_runtime_config(enc: &mut LittleEndianEncoder<'_>, config: &RuntimeConfig) {
    enc.write_u64(config.deterministic.world_seed);
    enc.write_u8(config.chunk_extent);
    enc.write_u8(config.active_chunk_radius);
    enc.write_u64(config.chart_id.raw());
    enc.write_u64(config.pattern_schedule.interval_ticks);
    enc.write_u32(config.pattern_schedule.magnitude);
    encode_option_time(enc, config.pattern_schedule.suppressed_from);
    encode_option_time(enc, config.pattern_schedule.suppressed_through);
    enc.write_u16(config.mana_parameters.base_response);
    enc.write_u16(config.mana_parameters.recurrence_response);
    enc.write_u16(config.mana_parameters.periodicity_response);
    enc.write_u16(config.mana_parameters.synchrony_response);
    enc.write_u16(config.mana_parameters.spatial_response);
    enc.write_u16(config.mana_parameters.diffusion);
    enc.write_u16(config.mana_parameters.decay);
    enc.write_i64(config.mana_parameters.maximum_intensity);
    enc.write_i64(config.mana_parameters.effect_threshold);
    enc.write_i64(config.mana_parameters.effect_hysteresis);
    match config.carrier_adapter {
        CarrierAdapterConfig::TerrainSeed { terrain_seed } => {
            enc.write_u8(1);
            enc.write_u64(terrain_seed);
        }
    }
    enc.write_u8(config.actor_count);
    enc.write_u8(config.sensor_count);
    enc.write_i64(config.action_bounds);
    enc.write_u64(config.bootstrap_population);
}

fn decode_runtime_config(
    dec: &mut LittleEndianDecoder<'_>,
) -> Result<RuntimeConfig, PersistenceError> {
    let mut config = RuntimeConfig::new(dec.read_u64()?);
    config.chunk_extent = dec.read_u8()?;
    config.active_chunk_radius = dec.read_u8()?;
    config.chart_id = SpatialChartId::new(dec.read_u64()?);
    config.pattern_schedule.interval_ticks = dec.read_u64()?;
    config.pattern_schedule.magnitude = dec.read_u32()?;
    config.pattern_schedule.suppressed_from = decode_option_time(dec)?;
    config.pattern_schedule.suppressed_through = decode_option_time(dec)?;
    config.mana_parameters.base_response = dec.read_u16()?;
    config.mana_parameters.recurrence_response = dec.read_u16()?;
    config.mana_parameters.periodicity_response = dec.read_u16()?;
    config.mana_parameters.synchrony_response = dec.read_u16()?;
    config.mana_parameters.spatial_response = dec.read_u16()?;
    config.mana_parameters.diffusion = dec.read_u16()?;
    config.mana_parameters.decay = dec.read_u16()?;
    config.mana_parameters.maximum_intensity = dec.read_i64()?;
    config.mana_parameters.effect_threshold = dec.read_i64()?;
    config.mana_parameters.effect_hysteresis = dec.read_i64()?;
    config.carrier_adapter = match dec.read_u8()? {
        1 => CarrierAdapterConfig::terrain_seed(dec.read_u64()?),
        value => {
            return Err(PersistenceError::codec(format!(
                "unknown carrier adapter {value}"
            )));
        }
    };
    config.actor_count = dec.read_u8()?;
    config.sensor_count = dec.read_u8()?;
    config.action_bounds = dec.read_i64()?;
    config.bootstrap_population = dec.read_u64()?;
    Ok(config)
}

fn encode_actor_objective(enc: &mut LittleEndianEncoder<'_>, snapshot: &ActorObjectiveSnapshot) {
    encode_body(enc, snapshot.body);
    enc.write_u64(snapshot.sensors.len() as u64);
    for sensor in &snapshot.sensors {
        encode_local_coord(enc, sensor.position);
        enc.write_u8(sensor.range);
        enc.write_u64(sensor.kind.raw());
    }
    enc.write_u64(snapshot.features.len() as u64);
    for feature in &snapshot.features {
        encode_feature(enc, feature);
    }
    enc.write_u64(snapshot.proposals.len() as u64);
    for proposal in &snapshot.proposals {
        enc.write_u64(proposal.action_kind.raw());
        encode_subjective_target(enc, proposal.target);
        enc.write_i64(proposal.intensity);
    }
    enc.write_u64(snapshot.validation_results.len() as u64);
    for result in &snapshot.validation_results {
        match result {
            ActionValidationResult::Valid { trace } => {
                enc.write_u8(1);
                enc.write_u64(trace.raw());
            }
            ActionValidationResult::Invalid { cause, trace } => {
                enc.write_u8(2);
                encode_rejection(enc, *cause);
                enc.write_u64(trace.raw());
            }
        }
    }
}

fn decode_actor_objective(
    dec: &mut LittleEndianDecoder<'_>,
) -> Result<ActorObjectiveSnapshot, PersistenceError> {
    let body = decode_body(dec)?;
    let sensor_count = read_count(dec, 16, "sensor")?;
    let mut sensors = Vec::with_capacity(sensor_count);
    for _ in 0..sensor_count {
        sensors.push(SensorAperture::new(
            decode_local_coord(dec)?,
            dec.read_u8()?,
            SensorKindId::new(dec.read_u64()?),
        ));
    }
    let feature_count = read_count(dec, 64, "feature")?;
    let mut features = Vec::with_capacity(feature_count);
    for _ in 0..feature_count {
        features.push(decode_feature(dec)?);
    }
    let proposal_count = read_count(dec, 64, "action proposal")?;
    let mut proposals = Vec::with_capacity(proposal_count);
    for _ in 0..proposal_count {
        proposals.push(ActionProposal {
            action_kind: ActionKindId::new(dec.read_u64()?),
            target: decode_subjective_target(dec)?,
            intensity: dec.read_i64()?,
        });
    }
    let result_count = read_count(dec, 256, "validation result")?;
    let mut validation_results = Vec::with_capacity(result_count);
    for _ in 0..result_count {
        validation_results.push(match dec.read_u8()? {
            1 => ActionValidationResult::Valid {
                trace: TraceId::new(dec.read_u64()?),
            },
            2 => ActionValidationResult::Invalid {
                cause: decode_rejection(dec)?,
                trace: TraceId::new(dec.read_u64()?),
            },
            value => {
                return Err(PersistenceError::codec(format!(
                    "unknown action result {value}"
                )));
            }
        });
    }
    Ok(ActorObjectiveSnapshot {
        body,
        sensors,
        features,
        proposals,
        validation_results,
    })
}

fn encode_actor_subjective(enc: &mut LittleEndianEncoder<'_>, snapshot: &ActorSubjectiveSnapshot) {
    match &snapshot.subjective_scene {
        Some(scene) => {
            enc.write_u8(1);
            encode_runtime_subjective_scene(enc, scene);
        }
        None => enc.write_u8(0),
    }
    encode_continuity(enc, &snapshot.continuity);
    encode_attention(enc, &snapshot.attention);
    encode_body_schema(enc, &snapshot.body_schema);
    encode_self_model(enc, &snapshot.self_model);
}

fn decode_actor_subjective(
    dec: &mut LittleEndianDecoder<'_>,
) -> Result<ActorSubjectiveSnapshot, PersistenceError> {
    let subjective_scene = match dec.read_u8()? {
        0 => None,
        1 => Some(decode_runtime_subjective_scene(dec)?),
        value => {
            return Err(PersistenceError::codec(format!(
                "unknown scene option {value}"
            )));
        }
    };
    Ok(ActorSubjectiveSnapshot {
        subjective_scene,
        continuity: decode_continuity(dec)?,
        attention: decode_attention(dec)?,
        body_schema: decode_body_schema(dec)?,
        self_model: decode_self_model(dec)?,
    })
}

fn encode_runtime_subjective_scene(
    enc: &mut LittleEndianEncoder<'_>,
    scene: &SubjectiveSceneSnapshot,
) {
    enc.write_u8(scene.perceived_self.energy_band);
    enc.write_u8(scene.perceived_self.motion_band);
    enc.write_u64(scene.objects.len() as u64);
    for object in &scene.objects {
        encode_scene_object(enc, object);
    }
    encode_body_schema(enc, &scene.body_schema);
    enc.write_u64(scene.active_goals.len() as u64);
    for goal in &scene.active_goals {
        enc.write_u64(goal.action_kind.raw());
        encode_subjective_target(enc, goal.target);
        enc.write_i64(goal.urgency);
    }
    encode_cognition_scene(enc, &scene.inner);
}

fn decode_runtime_subjective_scene(
    dec: &mut LittleEndianDecoder<'_>,
) -> Result<SubjectiveSceneSnapshot, PersistenceError> {
    let perceived_self = PerceivedSelf {
        energy_band: dec.read_u8()?,
        motion_band: dec.read_u8()?,
    };
    let object_count = read_count(dec, 16, "scene object")?;
    let mut objects = Vec::with_capacity(object_count);
    for _ in 0..object_count {
        objects.push(decode_scene_object(dec)?);
    }
    let body_schema = decode_body_schema(dec)?;
    let goal_count = read_count(dec, 16, "active goal")?;
    let mut active_goals = Vec::with_capacity(goal_count);
    for _ in 0..goal_count {
        active_goals.push(crate::ActiveGoal {
            action_kind: ActionKindId::new(dec.read_u64()?),
            target: decode_subjective_target(dec)?,
            urgency: dec.read_i64()?,
        });
    }
    let inner = decode_cognition_scene(dec)?;
    Ok(SubjectiveSceneSnapshot {
        perceived_self,
        objects,
        body_schema,
        active_goals,
        inner,
    })
}

fn encode_cognition_scene(
    enc: &mut LittleEndianEncoder<'_>,
    scene: &ontopolis_cognition::SubjectiveSceneSnapshot,
) {
    enc.write_u64(scene.time.raw());
    enc.write_u64(scene.objects.len() as u64);
    for object in &scene.objects {
        encode_scene_object(enc, object);
    }
    encode_body_schema(enc, &scene.body_schema);
    enc.write_u64(scene.active_self.len() as u64);
    for association in &scene.active_self {
        encode_self_association(enc, *association);
    }
}

fn decode_cognition_scene(
    dec: &mut LittleEndianDecoder<'_>,
) -> Result<ontopolis_cognition::SubjectiveSceneSnapshot, PersistenceError> {
    let time = SimulationTime::new(dec.read_u64()?);
    let object_count = read_count(dec, 16, "inner scene object")?;
    let mut objects = Vec::with_capacity(object_count);
    for _ in 0..object_count {
        objects.push(decode_scene_object(dec)?);
    }
    let body_schema = decode_body_schema(dec)?;
    let self_count = read_count(dec, 4, "active self")?;
    let mut active_self = Vec::with_capacity(self_count);
    for _ in 0..self_count {
        active_self.push(decode_self_association(dec)?);
    }
    Ok(ontopolis_cognition::SubjectiveSceneSnapshot {
        time,
        objects,
        body_schema,
        active_self,
    })
}

fn encode_body(enc: &mut LittleEndianEncoder<'_>, body: MinimalBodyState) {
    encode_world_coord(enc, body.position);
    enc.write_u64(body.orientation.yaw.to_bits());
    enc.write_u64(body.orientation.pitch.to_bits());
    enc.write_u64(body.orientation.roll.to_bits());
    enc.write_u64(body.velocity.x.to_bits());
    enc.write_u64(body.velocity.y.to_bits());
    enc.write_u64(body.velocity.z.to_bits());
    enc.write_u64(body.angular_velocity.x.to_bits());
    enc.write_u64(body.angular_velocity.y.to_bits());
    enc.write_u64(body.angular_velocity.z.to_bits());
    enc.write_i64(body.energy);
}

fn decode_body(dec: &mut LittleEndianDecoder<'_>) -> Result<MinimalBodyState, PersistenceError> {
    Ok(MinimalBodyState {
        position: decode_world_coord(dec)?,
        orientation: ontopolis_types::Orientation::new(
            f64::from_bits(dec.read_u64()?),
            f64::from_bits(dec.read_u64()?),
            f64::from_bits(dec.read_u64()?),
        ),
        velocity: Velocity::new(
            f64::from_bits(dec.read_u64()?),
            f64::from_bits(dec.read_u64()?),
            f64::from_bits(dec.read_u64()?),
        ),
        angular_velocity: AngularVelocity::new(
            f64::from_bits(dec.read_u64()?),
            f64::from_bits(dec.read_u64()?),
            f64::from_bits(dec.read_u64()?),
        ),
        energy: dec.read_i64()?,
    })
}

fn encode_feature(enc: &mut LittleEndianEncoder<'_>, feature: &GenericFeature) {
    enc.write_u64(feature.percept.raw());
    enc.write_u64(feature.attention_target.raw());
    enc.write_u8(feature_relation_tag(feature.relation));
    encode_feature_value(enc, feature.value);
    for value in feature.appearance.0 {
        enc.write_u16(value);
    }
    for value in feature.relative_position {
        enc.write_u32(value as u32);
    }
    enc.write_u32(feature.strength.raw());
    enc.write_u64(feature.time.raw());
}

fn decode_feature(dec: &mut LittleEndianDecoder<'_>) -> Result<GenericFeature, PersistenceError> {
    let percept = PerceptId::new(dec.read_u64()?);
    let attention_target = AttentionTargetId::new(dec.read_u64()?);
    let relation = decode_feature_relation(dec.read_u8()?)?;
    let value = decode_feature_value(dec)?;
    let mut appearance = [0_u16; 4];
    for item in &mut appearance {
        *item = dec.read_u16()?;
    }
    let mut relative_position = [0_i32; 3];
    for item in &mut relative_position {
        *item = dec.read_u32()? as i32;
    }
    let strength = ontopolis_cognition::CognitiveWeight::new(dec.read_u32()?)
        .map_err(|e| PersistenceError::codec(format!("invalid cognitive weight: {e}")))?;
    Ok(GenericFeature {
        percept,
        attention_target,
        relation,
        value,
        appearance: ontopolis_cognition::AppearanceSignature(appearance),
        relative_position,
        strength,
        time: SimulationTime::new(dec.read_u64()?),
    })
}

fn encode_feature_value(enc: &mut LittleEndianEncoder<'_>, value: FeatureValue) {
    match value {
        FeatureValue::Scalar(value) => {
            enc.write_u8(1);
            enc.write_u64(value.to_bits());
        }
        FeatureValue::Direction(direction) => {
            enc.write_u8(2);
            enc.write_u64(direction.x.to_bits());
            enc.write_u64(direction.y.to_bits());
            enc.write_u64(direction.z.to_bits());
        }
        FeatureValue::FrequencyBand(value) => {
            enc.write_u8(3);
            enc.write_u8(value);
        }
        FeatureValue::MagnitudeBand(value) => {
            enc.write_u8(4);
            enc.write_u8(value);
        }
    }
}

fn decode_feature_value(
    dec: &mut LittleEndianDecoder<'_>,
) -> Result<FeatureValue, PersistenceError> {
    Ok(match dec.read_u8()? {
        1 => FeatureValue::Scalar(f64::from_bits(dec.read_u64()?)),
        2 => FeatureValue::Direction(Direction3D::new(
            f64::from_bits(dec.read_u64()?),
            f64::from_bits(dec.read_u64()?),
            f64::from_bits(dec.read_u64()?),
        )),
        3 => FeatureValue::FrequencyBand(dec.read_u8()?),
        4 => FeatureValue::MagnitudeBand(dec.read_u8()?),
        value => {
            return Err(PersistenceError::codec(format!(
                "unknown feature value {value}"
            )));
        }
    })
}

fn feature_relation_tag(relation: FeatureRelation) -> u8 {
    match relation {
        FeatureRelation::Change => 1,
        FeatureRelation::Magnitude => 2,
        FeatureRelation::Direction => 3,
        FeatureRelation::Variance => 4,
        FeatureRelation::Periodicity => 5,
        FeatureRelation::Synchrony => 6,
        FeatureRelation::Recurrence => 7,
        FeatureRelation::Duration => 8,
        FeatureRelation::SpatialRelation => 9,
        FeatureRelation::TemporalRelation => 10,
        FeatureRelation::CoOccurrence => 11,
        FeatureRelation::StructuralSimilarity => 12,
        FeatureRelation::RelativeDifference => 13,
        FeatureRelation::SequenceSimilarity => 14,
    }
}

fn decode_feature_relation(tag: u8) -> Result<FeatureRelation, PersistenceError> {
    Ok(match tag {
        1 => FeatureRelation::Change,
        2 => FeatureRelation::Magnitude,
        3 => FeatureRelation::Direction,
        4 => FeatureRelation::Variance,
        5 => FeatureRelation::Periodicity,
        6 => FeatureRelation::Synchrony,
        7 => FeatureRelation::Recurrence,
        8 => FeatureRelation::Duration,
        9 => FeatureRelation::SpatialRelation,
        10 => FeatureRelation::TemporalRelation,
        11 => FeatureRelation::CoOccurrence,
        12 => FeatureRelation::StructuralSimilarity,
        13 => FeatureRelation::RelativeDifference,
        14 => FeatureRelation::SequenceSimilarity,
        value => {
            return Err(PersistenceError::codec(format!(
                "unknown feature relation {value}"
            )));
        }
    })
}

fn encode_subjective_target(enc: &mut LittleEndianEncoder<'_>, target: SubjectiveTarget) {
    match target {
        SubjectiveTarget::SelfBody => enc.write_u8(1),
        SubjectiveTarget::Object(id) => {
            enc.write_u8(2);
            enc.write_u64(id.raw());
        }
        SubjectiveTarget::Relative(relative) => {
            enc.write_u8(3);
            for value in relative {
                enc.write_u32(value as u32);
            }
        }
    }
}

fn decode_subjective_target(
    dec: &mut LittleEndianDecoder<'_>,
) -> Result<SubjectiveTarget, PersistenceError> {
    Ok(match dec.read_u8()? {
        1 => SubjectiveTarget::SelfBody,
        2 => SubjectiveTarget::Object(PerceivedObjectId::new(dec.read_u64()?)),
        3 => SubjectiveTarget::Relative([
            dec.read_u32()? as i32,
            dec.read_u32()? as i32,
            dec.read_u32()? as i32,
        ]),
        value => {
            return Err(PersistenceError::codec(format!(
                "unknown subjective target {value}"
            )));
        }
    })
}

fn encode_rejection(enc: &mut LittleEndianEncoder<'_>, rejection: ActionRejection) {
    enc.write_u8(match rejection {
        ActionRejection::MissingSubjectiveTarget => 1,
        ActionRejection::OutOfBounds => 2,
        ActionRejection::InsufficientEnergy => 3,
    });
}

fn decode_rejection(
    dec: &mut LittleEndianDecoder<'_>,
) -> Result<ActionRejection, PersistenceError> {
    Ok(match dec.read_u8()? {
        1 => ActionRejection::MissingSubjectiveTarget,
        2 => ActionRejection::OutOfBounds,
        3 => ActionRejection::InsufficientEnergy,
        value => {
            return Err(PersistenceError::codec(format!(
                "unknown rejection {value}"
            )));
        }
    })
}

fn encode_attention(
    enc: &mut LittleEndianEncoder<'_>,
    snapshot: &ontopolis_cognition::AttentionStateSnapshot,
) {
    enc.write_u8(snapshot.config.capacity);
    enc.write_u32(snapshot.config.salience_threshold);
    enc.write_u32(snapshot.config.continuity_bonus);
    encode_option_time(enc, snapshot.last_update);
    enc.write_u64(snapshot.foci.len() as u64);
    for focus in &snapshot.foci {
        enc.write_u64(focus.target.raw());
        enc.write_u64(focus.active_since.raw());
        enc.write_u64(focus.supporting_percept.raw());
    }
}

fn decode_attention(
    dec: &mut LittleEndianDecoder<'_>,
) -> Result<ontopolis_cognition::AttentionStateSnapshot, PersistenceError> {
    let config = ontopolis_cognition::AttentionConfigSnapshot {
        capacity: dec.read_u8()?,
        salience_threshold: dec.read_u32()?,
        continuity_bonus: dec.read_u32()?,
    };
    let last_update = decode_option_time(dec)?;
    let count = read_count(dec, 8, "attention focus")?;
    let mut foci = Vec::with_capacity(count);
    for _ in 0..count {
        foci.push(ontopolis_cognition::AttentionFocus {
            target: AttentionTargetId::new(dec.read_u64()?),
            active_since: SimulationTime::new(dec.read_u64()?),
            supporting_percept: PerceptId::new(dec.read_u64()?),
        });
    }
    Ok(ontopolis_cognition::AttentionStateSnapshot {
        config,
        foci,
        last_update,
    })
}

fn encode_body_schema(
    enc: &mut LittleEndianEncoder<'_>,
    snapshot: &ontopolis_cognition::BodySchemaSnapshot,
) {
    enc.write_u64(snapshot.parts.len() as u64);
    for part in &snapshot.parts {
        enc.write_u64(part.id.raw());
        for value in part.relative_position {
            enc.write_u32(value as u32);
        }
        enc.write_u32(part.extent);
        enc.write_u32(part.mobility.raw());
        enc.write_u32(part.confidence.raw());
    }
}

fn decode_body_schema(
    dec: &mut LittleEndianDecoder<'_>,
) -> Result<ontopolis_cognition::BodySchemaSnapshot, PersistenceError> {
    let count = read_count(dec, 16, "body schema part")?;
    let mut parts = Vec::with_capacity(count);
    for _ in 0..count {
        parts.push(ontopolis_cognition::BodySchemaPart {
            id: SubjectiveBodyPartId::new(dec.read_u64()?),
            relative_position: [
                dec.read_u32()? as i32,
                dec.read_u32()? as i32,
                dec.read_u32()? as i32,
            ],
            extent: dec.read_u32()?,
            mobility: ontopolis_cognition::CognitiveWeight::new(dec.read_u32()?)
                .map_err(|e| PersistenceError::codec(format!("invalid mobility: {e}")))?,
            confidence: ontopolis_cognition::CognitiveWeight::new(dec.read_u32()?)
                .map_err(|e| PersistenceError::codec(format!("invalid confidence: {e}")))?,
        });
    }
    Ok(ontopolis_cognition::BodySchemaSnapshot { parts })
}

fn encode_self_model(
    enc: &mut LittleEndianEncoder<'_>,
    snapshot: &ontopolis_cognition::SelfModelSnapshot,
) {
    enc.write_u64(snapshot.associations.len() as u64);
    for association in &snapshot.associations {
        encode_self_association(enc, *association);
    }
}

fn decode_self_model(
    dec: &mut LittleEndianDecoder<'_>,
) -> Result<ontopolis_cognition::SelfModelSnapshot, PersistenceError> {
    let count = read_count(dec, 16, "self association")?;
    let mut associations = Vec::with_capacity(count);
    for _ in 0..count {
        associations.push(decode_self_association(dec)?);
    }
    Ok(ontopolis_cognition::SelfModelSnapshot { associations })
}

fn encode_self_association(
    enc: &mut LittleEndianEncoder<'_>,
    association: ontopolis_cognition::SelfAssociation,
) {
    enc.write_u64(association.id.raw());
    enc.write_u32(association.strength.raw());
    enc.write_u64(association.supporting_percept.raw());
}

fn decode_self_association(
    dec: &mut LittleEndianDecoder<'_>,
) -> Result<ontopolis_cognition::SelfAssociation, PersistenceError> {
    Ok(ontopolis_cognition::SelfAssociation {
        id: SelfAssociationId::new(dec.read_u64()?),
        strength: ontopolis_cognition::CognitiveWeight::new(dec.read_u32()?)
            .map_err(|e| PersistenceError::codec(format!("invalid self strength: {e}")))?,
        supporting_percept: PerceptId::new(dec.read_u64()?),
    })
}

fn encode_scene_object(
    enc: &mut LittleEndianEncoder<'_>,
    object: &ontopolis_cognition::SceneObject,
) {
    enc.write_u64(object.id.raw());
    for value in object.appearance.0 {
        enc.write_u16(value);
    }
    for value in object.relative_position {
        enc.write_u32(value as u32);
    }
    enc.write_u32(object.confidence.raw());
    enc.write_u64(object.supporting_percept.raw());
}

fn decode_scene_object(
    dec: &mut LittleEndianDecoder<'_>,
) -> Result<ontopolis_cognition::SceneObject, PersistenceError> {
    let id = PerceivedObjectId::new(dec.read_u64()?);
    let mut appearance = [0_u16; 4];
    for value in &mut appearance {
        *value = dec.read_u16()?;
    }
    Ok(ontopolis_cognition::SceneObject {
        id,
        appearance: ontopolis_cognition::AppearanceSignature(appearance),
        relative_position: [
            dec.read_u32()? as i32,
            dec.read_u32()? as i32,
            dec.read_u32()? as i32,
        ],
        confidence: ontopolis_cognition::CognitiveWeight::new(dec.read_u32()?)
            .map_err(|e| PersistenceError::codec(format!("invalid object confidence: {e}")))?,
        supporting_percept: PerceptId::new(dec.read_u64()?),
    })
}

fn encode_continuity(
    enc: &mut LittleEndianEncoder<'_>,
    snapshot: &ontopolis_cognition::SceneContinuitySnapshot,
) {
    enc.write_u64(snapshot.next_object_id);
    encode_option_time(enc, snapshot.last_update);
    enc.write_u32(snapshot.appearance_tolerance);
    enc.write_u32(snapshot.position_tolerance);
    enc.write_u64(snapshot.tracked.len() as u64);
    for object in &snapshot.tracked {
        enc.write_u64(object.id.raw());
        enc.write_u64(object.last_target.raw());
        for value in object.appearance.0 {
            enc.write_u16(value);
        }
        for value in object.relative_position {
            enc.write_u32(value as u32);
        }
        enc.write_u32(object.confidence.raw());
        enc.write_u64(object.last_seen.raw());
        enc.write_u64(object.supporting_percept.raw());
    }
}

fn decode_continuity(
    dec: &mut LittleEndianDecoder<'_>,
) -> Result<ontopolis_cognition::SceneContinuitySnapshot, PersistenceError> {
    let next_object_id = dec.read_u64()?;
    let last_update = decode_option_time(dec)?;
    let appearance_tolerance = dec.read_u32()?;
    let position_tolerance = dec.read_u32()?;
    let count = read_count(dec, 32, "tracked object")?;
    let mut tracked = Vec::with_capacity(count);
    for _ in 0..count {
        let id = PerceivedObjectId::new(dec.read_u64()?);
        let last_target = AttentionTargetId::new(dec.read_u64()?);
        let mut appearance = [0_u16; 4];
        for value in &mut appearance {
            *value = dec.read_u16()?;
        }
        tracked.push(ontopolis_cognition::TrackedObjectSnapshot {
            id,
            last_target,
            appearance: ontopolis_cognition::AppearanceSignature(appearance),
            relative_position: [
                dec.read_u32()? as i32,
                dec.read_u32()? as i32,
                dec.read_u32()? as i32,
            ],
            confidence: ontopolis_cognition::CognitiveWeight::new(dec.read_u32()?)
                .map_err(|e| PersistenceError::codec(format!("invalid tracked confidence: {e}")))?,
            last_seen: SimulationTime::new(dec.read_u64()?),
            supporting_percept: PerceptId::new(dec.read_u64()?),
        });
    }
    Ok(ontopolis_cognition::SceneContinuitySnapshot {
        tracked,
        next_object_id,
        last_update,
        appearance_tolerance,
        position_tolerance,
    })
}

fn encode_terrain_carrier(enc: &mut LittleEndianEncoder<'_>, snapshot: &TerrainCarrierSnapshot) {
    encode_chart_chunk(enc, snapshot.chunk);
    enc.write_u8(snapshot.field_extent);
    enc.write_u64(snapshot.world_seed);
    enc.write_u64(snapshot.generation_trace.raw());
    enc.write_u64(snapshot.generator);
    enc.write_u64(snapshot.parameters);
    encode_trace_vec(enc, &snapshot.causal_inputs);
    enc.write_u64(snapshot.elevations_mm.len() as u64);
    for value in &snapshot.elevations_mm {
        enc.write_u32(*value as u32);
    }
    enc.write_u64(snapshot.surface_materials.len() as u64);
    for material in &snapshot.surface_materials {
        enc.write_u64(material.raw());
    }
    enc.write_u64(snapshot.roughness_mm.len() as u64);
    for value in &snapshot.roughness_mm {
        enc.write_u32(*value);
    }
}

fn decode_terrain_carrier(
    dec: &mut LittleEndianDecoder<'_>,
) -> Result<TerrainCarrierSnapshot, PersistenceError> {
    let chunk = decode_chart_chunk(dec)?;
    let field_extent = dec.read_u8()?;
    let world_seed = dec.read_u64()?;
    let generation_trace = TraceId::new(dec.read_u64()?);
    let generator = dec.read_u64()?;
    let parameters = dec.read_u64()?;
    let causal_inputs = decode_trace_vec(dec, 4_096)?;
    let elevation_count = read_count(dec, 32 * 32, "terrain elevation")?;
    let mut elevations_mm = Vec::with_capacity(elevation_count);
    for _ in 0..elevation_count {
        elevations_mm.push(dec.read_u32()? as i32);
    }
    let material_count = read_count(dec, 32 * 32, "terrain material")?;
    let mut surface_materials = Vec::with_capacity(material_count);
    for _ in 0..material_count {
        surface_materials.push(ontopolis_types::MaterialId::new(dec.read_u64()?));
    }
    let roughness_count = read_count(dec, 32 * 32, "terrain roughness")?;
    let mut roughness_mm = Vec::with_capacity(roughness_count);
    for _ in 0..roughness_count {
        roughness_mm.push(dec.read_u32()?);
    }
    Ok(TerrainCarrierSnapshot {
        chunk,
        field_extent,
        world_seed,
        generation_trace,
        generator,
        parameters,
        causal_inputs,
        elevations_mm,
        surface_materials,
        roughness_mm,
    })
}

fn encode_actor_object(enc: &mut LittleEndianEncoder<'_>, object: &ActorPhysicalObject) {
    enc.write_u64(object.object_key);
    encode_world_coord(enc, object.position);
    enc.write_i64(object.magnitude);
    encode_bool(enc, object.accessible);
    encode_bool(enc, object.occluded);
    enc.write_u64(object.trace.raw());
}

fn decode_actor_object(
    dec: &mut LittleEndianDecoder<'_>,
) -> Result<ActorPhysicalObject, PersistenceError> {
    Ok(ActorPhysicalObject {
        object_key: dec.read_u64()?,
        position: decode_world_coord(dec)?,
        magnitude: dec.read_i64()?,
        accessible: decode_bool(dec)?,
        occluded: decode_bool(dec)?,
        trace: TraceId::new(dec.read_u64()?),
    })
}

fn encode_population_aggregate(enc: &mut LittleEndianEncoder<'_>, aggregate: &PopulationAggregate) {
    encode_chart_chunk(enc, aggregate.chart);
    enc.write_u64(aggregate.count);
    enc.write_u64(aggregate.births);
    enc.write_u64(aggregate.deaths);
    enc.write_i64(aggregate.material_inflow);
    enc.write_i64(aggregate.material_outflow);
    encode_trace_vec(enc, &aggregate.causal_ancestry);
}

fn decode_population_aggregate(
    dec: &mut LittleEndianDecoder<'_>,
) -> Result<PopulationAggregate, PersistenceError> {
    Ok(PopulationAggregate {
        chart: decode_chart_chunk(dec)?,
        count: dec.read_u64()?,
        births: dec.read_u64()?,
        deaths: dec.read_u64()?,
        material_inflow: dec.read_i64()?,
        material_outflow: dec.read_i64()?,
        causal_ancestry: decode_trace_vec(dec, 4_096)?,
    })
}

fn encode_pattern_sample(
    enc: &mut LittleEndianEncoder<'_>,
    sample: &ontopolis_domains::PhysicalPatternSample,
) {
    encode_chart_chunk(enc, sample.chunk);
    enc.write_u64(sample.pattern.raw());
    encode_local_coord(enc, sample.position);
    enc.write_u64(sample.observed_at.raw());
    enc.write_u32(sample.magnitude);
    enc.write_u32(sample.source_ordinal);
    enc.write_u64(sample.cause.raw());
}

fn decode_pattern_sample(
    dec: &mut LittleEndianDecoder<'_>,
) -> Result<ontopolis_domains::PhysicalPatternSample, PersistenceError> {
    Ok(ontopolis_domains::PhysicalPatternSample {
        chunk: decode_chart_chunk(dec)?,
        pattern: PhysicalPatternId::new(dec.read_u64()?),
        position: decode_local_coord(dec)?,
        observed_at: SimulationTime::new(dec.read_u64()?),
        magnitude: dec.read_u32()?,
        source_ordinal: dec.read_u32()?,
        cause: TraceId::new(dec.read_u64()?),
    })
}

fn encode_physical_digest(enc: &mut LittleEndianEncoder<'_>, digest: crate::PhysicalStateDigest) {
    enc.write_u16(digest.schema_version.raw());
    enc.write_fixed(&digest.bytes());
}

fn decode_physical_digest(
    dec: &mut LittleEndianDecoder<'_>,
) -> Result<crate::PhysicalStateDigest, PersistenceError> {
    Ok(crate::PhysicalStateDigest {
        schema_version: crate::DigestSchemaVersion::new(dec.read_u16()?),
        fingerprint: ontopolis_core::StateFingerprint::new(*dec.read_fixed::<32>()?),
    })
}

fn encode_history_digest(enc: &mut LittleEndianEncoder<'_>, digest: crate::HistoryDigest) {
    enc.write_u16(digest.schema_version.raw());
    enc.write_fixed(&digest.bytes());
}

fn decode_history_digest(
    dec: &mut LittleEndianDecoder<'_>,
) -> Result<crate::HistoryDigest, PersistenceError> {
    Ok(crate::HistoryDigest {
        schema_version: crate::DigestSchemaVersion::new(dec.read_u16()?),
        fingerprint: ontopolis_core::StateFingerprint::new(*dec.read_fixed::<32>()?),
    })
}

fn encode_chart_chunk(enc: &mut LittleEndianEncoder<'_>, chunk: ChartChunkCoord) {
    enc.write_u64(chunk.chart.raw());
    enc.write_u32(chunk.chunk.x as u32);
    enc.write_u32(chunk.chunk.y as u32);
    enc.write_u32(chunk.chunk.z as u32);
}

fn decode_chart_chunk(
    dec: &mut LittleEndianDecoder<'_>,
) -> Result<ChartChunkCoord, PersistenceError> {
    Ok(ChartChunkCoord::new(
        SpatialChartId::new(dec.read_u64()?),
        ChunkCoord::new(
            dec.read_u32()? as i32,
            dec.read_u32()? as i32,
            dec.read_u32()? as i32,
        ),
    ))
}

fn encode_world_coord(enc: &mut LittleEndianEncoder<'_>, coord: WorldCoord) {
    enc.write_i64(coord.x);
    enc.write_i64(coord.y);
    enc.write_i64(coord.z);
}

fn decode_world_coord(dec: &mut LittleEndianDecoder<'_>) -> Result<WorldCoord, PersistenceError> {
    Ok(WorldCoord::new(
        dec.read_i64()?,
        dec.read_i64()?,
        dec.read_i64()?,
    ))
}

fn encode_local_coord(enc: &mut LittleEndianEncoder<'_>, coord: LocalCoord) {
    enc.write_u8(coord.x);
    enc.write_u8(coord.y);
    enc.write_u8(coord.z);
}

fn decode_local_coord(dec: &mut LittleEndianDecoder<'_>) -> Result<LocalCoord, PersistenceError> {
    Ok(LocalCoord::new(
        dec.read_u8()?,
        dec.read_u8()?,
        dec.read_u8()?,
    ))
}

fn encode_trace_vec(enc: &mut LittleEndianEncoder<'_>, traces: &[TraceId]) {
    enc.write_u64(traces.len() as u64);
    for trace in traces {
        enc.write_u64(trace.raw());
    }
}

fn decode_trace_vec(
    dec: &mut LittleEndianDecoder<'_>,
    max: usize,
) -> Result<Vec<TraceId>, PersistenceError> {
    let count = read_count(dec, max, "trace vector")?;
    let mut traces = Vec::with_capacity(count);
    for _ in 0..count {
        traces.push(TraceId::new(dec.read_u64()?));
    }
    Ok(traces)
}

fn encode_option_trace(enc: &mut LittleEndianEncoder<'_>, trace: Option<TraceId>) {
    match trace {
        Some(trace) => {
            enc.write_u8(1);
            enc.write_u64(trace.raw());
        }
        None => enc.write_u8(0),
    }
}

fn decode_option_trace(
    dec: &mut LittleEndianDecoder<'_>,
) -> Result<Option<TraceId>, PersistenceError> {
    Ok(match dec.read_u8()? {
        0 => None,
        1 => Some(TraceId::new(dec.read_u64()?)),
        value => {
            return Err(PersistenceError::codec(format!(
                "invalid trace option {value}"
            )));
        }
    })
}

fn encode_option_time(enc: &mut LittleEndianEncoder<'_>, time: Option<SimulationTime>) {
    match time {
        Some(time) => {
            enc.write_u8(1);
            enc.write_u64(time.raw());
        }
        None => enc.write_u8(0),
    }
}

fn decode_option_time(
    dec: &mut LittleEndianDecoder<'_>,
) -> Result<Option<SimulationTime>, PersistenceError> {
    Ok(match dec.read_u8()? {
        0 => None,
        1 => Some(SimulationTime::new(dec.read_u64()?)),
        value => {
            return Err(PersistenceError::codec(format!(
                "invalid time option {value}"
            )));
        }
    })
}

fn encode_bool(enc: &mut LittleEndianEncoder<'_>, value: bool) {
    enc.write_u8(u8::from(value));
}

fn decode_bool(dec: &mut LittleEndianDecoder<'_>) -> Result<bool, PersistenceError> {
    Ok(match dec.read_u8()? {
        0 => false,
        1 => true,
        value => return Err(PersistenceError::codec(format!("invalid bool {value}"))),
    })
}

fn decode_phase(phase_id: u16) -> Result<Phase, PersistenceError> {
    Phase::ALL
        .iter()
        .find(|phase| phase.id().0 == phase_id as u8)
        .copied()
        .ok_or_else(|| PersistenceError::codec(format!("unknown phase id {phase_id}")))
}

fn read_count(
    dec: &mut LittleEndianDecoder<'_>,
    max: usize,
    label: &str,
) -> Result<usize, PersistenceError> {
    let count = usize_from_u64(dec.read_u64()?, label)?;
    if count > max {
        return Err(PersistenceError::codec(format!(
            "{label} count {count} exceeds {max}"
        )));
    }
    Ok(count)
}

fn usize_from_u64(value: u64, label: &str) -> Result<usize, PersistenceError> {
    usize::try_from(value)
        .map_err(|_| PersistenceError::codec(format!("{label} count exceeds usize")))
}

fn reject_unsorted_ids(ids: impl Iterator<Item = u64>) -> Result<(), PersistenceError> {
    let mut previous = None;
    for id in ids {
        if previous.is_some_and(|previous| previous >= id) {
            return Err(PersistenceError::codec("IDs must be strictly ordered"));
        }
        previous = Some(id);
    }
    Ok(())
}

fn reject_unsorted_chunks(
    chunks: impl Iterator<Item = ChartChunkCoord>,
) -> Result<(), PersistenceError> {
    let mut previous = None;
    for chunk in chunks {
        if previous.is_some_and(|previous| previous >= chunk) {
            return Err(PersistenceError::codec("chunks must be strictly ordered"));
        }
        previous = Some(chunk);
    }
    Ok(())
}

fn require_empty(dec: &LittleEndianDecoder<'_>) -> Result<(), PersistenceError> {
    if dec.is_empty() {
        Ok(())
    } else {
        Err(PersistenceError::codec(
            "trailing authoritative section bytes",
        ))
    }
}

/// Assemble a `RuntimeSnapshotData` into a canonical `SnapshotEnvelope`.
pub fn assemble_envelope(
    data: &RuntimeSnapshotData,
) -> Result<SnapshotEnvelope, PersistenceError> {
    let mut sections = std::collections::BTreeMap::new();

    sections.insert(
        u64::from(SECTION_RUNTIME_RECIPE),
        SectionPayload {
            section_major: 1,
            section_minor: 0,
            flags: 0,
            decoded_size_limit: 0,
            bytes: encode_runtime_recipe_section(&data.recipe),
        },
    );
    sections.insert(
        u64::from(SECTION_SPATIAL_CHUNKS),
        SectionPayload {
            section_major: 1,
            section_minor: 0,
            flags: 0,
            decoded_size_limit: 0,
            bytes: encode_spatial_section(&data.spatial),
        },
    );
    sections.insert(
        u64::from(SECTION_MANA_FIELDS),
        SectionPayload {
            section_major: 1,
            section_minor: 0,
            flags: 0,
            decoded_size_limit: 0,
            bytes: encode_mana_section(&data.mana),
        },
    );
    sections.insert(
        u64::from(SECTION_RESOLUTION_FIELD),
        SectionPayload {
            section_major: 1,
            section_minor: 0,
            flags: 0,
            decoded_size_limit: 0,
            bytes: encode_resolution_section(&data.resolution, &data.resolution_policy),
        },
    );
    sections.insert(
        u64::from(SECTION_PATTERN_HISTORY),
        SectionPayload {
            section_major: 1,
            section_minor: 0,
            flags: 0,
            decoded_size_limit: 0,
            bytes: encode_pattern_history_section(&data.pattern_history),
        },
    );
    sections.insert(
        u64::from(SECTION_PHYSICAL_COUNTERS),
        SectionPayload {
            section_major: 1,
            section_minor: 0,
            flags: 0,
            decoded_size_limit: 0,
            bytes: encode_physical_counters_section(&data.physical_counters),
        },
    );
    sections.insert(
        u64::from(SECTION_ACTOR_OBJECTIVE),
        SectionPayload {
            section_major: 1,
            section_minor: 0,
            flags: 0,
            decoded_size_limit: 0,
            bytes: encode_actor_objective_section(&data.actors_objective),
        },
    );
    sections.insert(
        u64::from(SECTION_ACTOR_SUBJECTIVE),
        SectionPayload {
            section_major: 1,
            section_minor: 0,
            flags: 0,
            decoded_size_limit: 0,
            bytes: encode_actor_subjective_section(&data.actors_subjective),
        },
    );
    sections.insert(
        u64::from(SECTION_POPULATION_BOOTSTRAP),
        SectionPayload {
            section_major: 1,
            section_minor: 0,
            flags: 0,
            decoded_size_limit: 0,
            bytes: encode_population_section(&data.population, &data.bootstrap),
        },
    );
    sections.insert(
        u64::from(SECTION_CAUSAL_TRACES),
        SectionPayload {
            section_major: 1,
            section_minor: 0,
            flags: 0,
            decoded_size_limit: 0,
            bytes: encode_trace_section(&data.traces),
        },
    );
    if let Some(ref manifest) = data.experiment_manifest {
        sections.insert(
            u64::from(SECTION_EXPERIMENT_MANIFEST),
            SectionPayload {
                section_major: 1,
                section_minor: 0,
                flags: 0,
                decoded_size_limit: 0,
                bytes: encode_experiment_manifest_section(manifest),
            },
        );
    }

    let (physical_digest, history_digest) = {
        let temp_state = RuntimeState::import_snapshot(data.clone())
            .map_err(|e| PersistenceError::codec(format!("digest computation failed: {e}")))?;
        let phys = temp_state.physical_state_digest(data.recipe.completed_time);
        let hist = temp_state.history_digest();
        (phys.bytes(), hist.bytes())
    };

    let header = SnapshotHeader {
        format_major: FORMAT_MAJOR_V1,
        format_minor: FORMAT_MINOR_V1,
        codec_revision: 1,
        world_seed: data.recipe.seed,
        completed_time: data.recipe.completed_time.raw(),
        runtime_recipe_fingerprint: [0u8; 32],
        physical_digest_schema: 1,
        physical_digest,
        history_digest_schema: 1,
        history_digest,
        section_count: 0,
        section_directory_offset: 0,
        payload_integrity: [0u8; 32],
    };

    Ok(SnapshotEnvelope::new(header, sections))
}

/// Disassemble a `SnapshotEnvelope` into `RuntimeSnapshotData`.
pub fn disassemble_envelope(
    envelope: &SnapshotEnvelope,
) -> Result<RuntimeSnapshotData, PersistenceError> {
    let recipe = decode_runtime_recipe_section(
        envelope
            .sections
            .get(&u64::from(SECTION_RUNTIME_RECIPE))
            .ok_or(PersistenceError::MissingRequiredSection {
                schema_id: u64::from(SECTION_RUNTIME_RECIPE),
            })?
            .bytes
            .as_slice(),
    )?;
    let spatial = decode_spatial_section(
        envelope
            .sections
            .get(&u64::from(SECTION_SPATIAL_CHUNKS))
            .ok_or(PersistenceError::MissingRequiredSection {
                schema_id: u64::from(SECTION_SPATIAL_CHUNKS),
            })?
            .bytes
            .as_slice(),
    )?;
    let mana = decode_mana_section(
        envelope
            .sections
            .get(&u64::from(SECTION_MANA_FIELDS))
            .ok_or(PersistenceError::MissingRequiredSection {
                schema_id: u64::from(SECTION_MANA_FIELDS),
            })?
            .bytes
            .as_slice(),
    )?;
    let (resolution, resolution_policy) = decode_resolution_section(
        envelope
            .sections
            .get(&u64::from(SECTION_RESOLUTION_FIELD))
            .ok_or(PersistenceError::MissingRequiredSection {
                schema_id: u64::from(SECTION_RESOLUTION_FIELD),
            })?
            .bytes
            .as_slice(),
    )?;
    let pattern_history = decode_pattern_history_section(
        envelope
            .sections
            .get(&u64::from(SECTION_PATTERN_HISTORY))
            .ok_or(PersistenceError::MissingRequiredSection {
                schema_id: u64::from(SECTION_PATTERN_HISTORY),
            })?
            .bytes
            .as_slice(),
    )?;
    let physical_counters = decode_physical_counters_section(
        envelope
            .sections
            .get(&u64::from(SECTION_PHYSICAL_COUNTERS))
            .ok_or(PersistenceError::MissingRequiredSection {
                schema_id: u64::from(SECTION_PHYSICAL_COUNTERS),
            })?
            .bytes
            .as_slice(),
    )?;
    let actors_objective = decode_actor_objective_section(
        envelope
            .sections
            .get(&u64::from(SECTION_ACTOR_OBJECTIVE))
            .ok_or(PersistenceError::MissingRequiredSection {
                schema_id: u64::from(SECTION_ACTOR_OBJECTIVE),
            })?
            .bytes
            .as_slice(),
    )?;
    let actors_subjective = decode_actor_subjective_section(
        envelope
            .sections
            .get(&u64::from(SECTION_ACTOR_SUBJECTIVE))
            .ok_or(PersistenceError::MissingRequiredSection {
                schema_id: u64::from(SECTION_ACTOR_SUBJECTIVE),
            })?
            .bytes
            .as_slice(),
    )?;
    let (population, bootstrap) = decode_population_section(
        envelope
            .sections
            .get(&u64::from(SECTION_POPULATION_BOOTSTRAP))
            .ok_or(PersistenceError::MissingRequiredSection {
                schema_id: u64::from(SECTION_POPULATION_BOOTSTRAP),
            })?
            .bytes
            .as_slice(),
    )?;
    let traces = decode_trace_section(
        envelope
            .sections
            .get(&u64::from(SECTION_CAUSAL_TRACES))
            .ok_or(PersistenceError::MissingRequiredSection {
                schema_id: u64::from(SECTION_CAUSAL_TRACES),
            })?
            .bytes
            .as_slice(),
    )?;
    let experiment_manifest = envelope
        .sections
        .get(&u64::from(SECTION_EXPERIMENT_MANIFEST))
        .map(|payload| decode_experiment_manifest_section(payload.bytes.as_slice()))
        .transpose()?;

    Ok(RuntimeSnapshotData {
        recipe,
        spatial,
        mana,
        resolution,
        resolution_policy,
        pattern_history,
        physical_counters,
        actors_objective,
        actors_subjective,
        population,
        bootstrap,
        traces,
        experiment_manifest,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Runtime, RuntimeState};
    use ontopolis_core::Phase;
    use ontopolis_core::provenance::{
        CausalEffect, CausalEventProposal, CausalTarget, EventProposalKey, StateFingerprint,
    };
    use ontopolis_types::{
        EventKindId, SimulationTime, StateObjectKindId, StatePropertyId, TraceId,
    };

    fn fingerprint(byte: u8) -> StateFingerprint {
        StateFingerprint::new([byte; 32])
    }

    fn effect(object_id: u64, before: u8, after: u8) -> CausalEffect {
        CausalEffect::new(
            CausalTarget::new(
                StateObjectKindId::new(4),
                object_id,
                StatePropertyId::new(9),
            ),
            fingerprint(before),
            fingerprint(after),
        )
        .unwrap()
    }

    fn populated_snapshot_data() -> crate::RuntimeSnapshotData {
        let mut config = crate::RuntimeConfig::new(55);
        config.actor_count = 1;
        config.sensor_count = 1;
        config.bootstrap_population = 4;
        let mut runtime = Runtime::new(config).unwrap();
        runtime.run_ticks(16).unwrap();
        runtime.export_snapshot().unwrap()
    }

    fn proposal(key: u64, causes: Vec<TraceId>, object_id: u64) -> CausalEventProposal {
        CausalEventProposal::new(
            EventProposalKey::new(2, key, 0),
            EventKindId::new(7),
            causes,
            vec![effect(object_id, key as u8, (key + 1) as u8)],
        )
        .unwrap()
    }

    #[test]
    fn trace_store_roundtrip() {
        let mut store = ontopolis_core::provenance::CausalTraceStore::new();
        let root = CausalEventProposal::new(
            EventProposalKey::new(0, 0, 0),
            EventKindId::new(1),
            Vec::new(),
            vec![effect(0, 0, 1)],
        )
        .unwrap();
        let root_trace = store
            .commit_batch(SimulationTime::new(0), Phase::Physics, vec![root])
            .unwrap()[0];

        let child = proposal(1, vec![root_trace], 1);
        let child_trace = store
            .commit_batch(SimulationTime::new(1), Phase::Mana, vec![child])
            .unwrap()[0];

        let grandchild = proposal(2, vec![root_trace, child_trace], 2);
        store
            .commit_batch(SimulationTime::new(2), Phase::Resolution, vec![grandchild])
            .unwrap();

        let snapshot = store.export_snapshot();
        let encoded = encode_trace_section(&snapshot);
        let decoded_snapshot = decode_trace_section(&encoded).unwrap();
        let restored =
            ontopolis_core::provenance::CausalTraceStore::import_snapshot(decoded_snapshot)
                .unwrap();

        assert_eq!(restored.len(), store.len());
        assert_eq!(restored.export_snapshot(), store.export_snapshot());

        // Verify child traversal is identical.
        for trace_id in store.iter().map(|e| e.trace_id) {
            let original_children: Vec<TraceId> = store.children(trace_id).to_vec();
            let restored_children: Vec<TraceId> = restored.children(trace_id).to_vec();
            assert_eq!(
                original_children, restored_children,
                "children mismatch for {trace_id:?}"
            );
        }
    }

    #[test]
    fn encode_trace_store_convenience() {
        let mut store = ontopolis_core::provenance::CausalTraceStore::new();
        let root = CausalEventProposal::new(
            EventProposalKey::new(0, 0, 0),
            EventKindId::new(1),
            Vec::new(),
            vec![effect(0, 0, 1)],
        )
        .unwrap();
        store
            .commit_batch(SimulationTime::new(0), Phase::Physics, vec![root])
            .unwrap();

        let encoded = encode_trace_store(&store);
        let restored = decode_trace_store(&encoded).unwrap();
        assert_eq!(restored.len(), store.len());
    }

    #[test]
    fn empty_trace_store_roundtrip() {
        let store = ontopolis_core::provenance::CausalTraceStore::new();
        let encoded = encode_trace_store(&store);
        let restored = decode_trace_store(&encoded).unwrap();
        assert!(restored.is_empty());
    }

    #[test]
    fn stage_four_sections_roundtrip_individually() {
        let data = populated_snapshot_data();

        let recipe =
            decode_runtime_recipe_section(&encode_runtime_recipe_section(&data.recipe)).unwrap();
        assert_eq!(recipe, data.recipe);

        let spatial = decode_spatial_section(&encode_spatial_section(&data.spatial)).unwrap();
        assert_eq!(spatial, data.spatial);

        let mana = decode_mana_section(&encode_mana_section(&data.mana)).unwrap();
        assert_eq!(mana, data.mana);

        let (resolution, policy) = decode_resolution_section(&encode_resolution_section(
            &data.resolution,
            &data.resolution_policy,
        ))
        .unwrap();
        assert_eq!(resolution, data.resolution);
        assert_eq!(policy, data.resolution_policy);

        let history =
            decode_pattern_history_section(&encode_pattern_history_section(&data.pattern_history))
                .unwrap();
        assert_eq!(history, data.pattern_history);

        let counters = decode_physical_counters_section(&encode_physical_counters_section(
            &data.physical_counters,
        ))
        .unwrap();
        assert_eq!(counters, data.physical_counters);

        let objective =
            decode_actor_objective_section(&encode_actor_objective_section(&data.actors_objective))
                .unwrap();
        assert_eq!(objective, data.actors_objective);

        let subjective = decode_actor_subjective_section(&encode_actor_subjective_section(
            &data.actors_subjective,
        ))
        .unwrap();
        assert_eq!(subjective, data.actors_subjective);

        let (population, bootstrap) = decode_population_section(&encode_population_section(
            &data.population,
            &data.bootstrap,
        ))
        .unwrap();
        assert_eq!(population, data.population);
        assert_eq!(bootstrap, data.bootstrap);
    }

    #[test]
    fn experiment_manifest_section_roundtrips() {
        let data = populated_snapshot_data();
        let manifest = crate::ExperimentManifestSnapshot {
            format_version: 1,
            seed_set: vec![55],
            checkpoint_interval: 8,
            bootstrap_population: 4,
            suppression_from: SimulationTime::new(0),
            suppression_through: SimulationTime::new(0),
            warm_up_ticks: 0,
            duration_ticks: data.recipe.completed_time.raw(),
            physical_digest: RuntimeState::import_snapshot(data.clone())
                .unwrap()
                .snapshot(data.recipe.completed_time)
                .physical_state_digest,
            history_digest: RuntimeState::import_snapshot(data.clone())
                .unwrap()
                .snapshot(data.recipe.completed_time)
                .history_digest,
            supporting_traces: vec![data.physical_counters.latest_physical_trace],
            evidence_sufficient: true,
        };

        let decoded =
            decode_experiment_manifest_section(&encode_experiment_manifest_section(&manifest))
                .unwrap();

        assert_eq!(decoded, manifest);
    }

    #[test]
    fn section_decoders_reject_trailing_or_truncated_corruption() {
        let data = populated_snapshot_data();
        let cases = [
            encode_runtime_recipe_section(&data.recipe),
            encode_spatial_section(&data.spatial),
            encode_mana_section(&data.mana),
            encode_resolution_section(&data.resolution, &data.resolution_policy),
            encode_pattern_history_section(&data.pattern_history),
            encode_physical_counters_section(&data.physical_counters),
            encode_actor_objective_section(&data.actors_objective),
            encode_actor_subjective_section(&data.actors_subjective),
            encode_population_section(&data.population, &data.bootstrap),
        ];

        for mut encoded in cases {
            encoded.pop();
            assert!(
                decode_runtime_recipe_section(&encoded).is_err()
                    || decode_spatial_section(&encoded).is_err()
                    || decode_mana_section(&encoded).is_err()
                    || decode_resolution_section(&encoded).is_err()
                    || decode_pattern_history_section(&encoded).is_err()
                    || decode_physical_counters_section(&encoded).is_err()
                    || decode_actor_objective_section(&encoded).is_err()
                    || decode_actor_subjective_section(&encoded).is_err()
                    || decode_population_section(&encoded).is_err()
            );
        }

        let mut encoded = encode_spatial_section(&data.spatial);
        encoded.push(0);
        assert!(decode_spatial_section(&encoded).is_err());
    }

    #[test]
    fn section_decoders_reject_duplicate_or_unsorted_ids() {
        let mut data = populated_snapshot_data();
        data.spatial.active_chunks[1].chunk = data.spatial.active_chunks[0].chunk;
        assert!(decode_spatial_section(&encode_spatial_section(&data.spatial)).is_err());

        let mut data = populated_snapshot_data();
        data.mana.fields[1].chunk = data.mana.fields[0].chunk;
        assert!(decode_mana_section(&encode_mana_section(&data.mana)).is_err());

        let mut data = populated_snapshot_data();
        if !data.actors_objective.actors.is_empty() {
            let clone = data.actors_objective.actors.last().unwrap().clone();
            data.actors_objective.actors.push(clone);
        }
        assert!(
            decode_actor_objective_section(
                &encode_actor_objective_section(&data.actors_objective,)
            )
            .is_err()
        );
    }

    #[test]
    fn runtime_state_import_restores_physical_and_history_digest() {
        let data = populated_snapshot_data();
        let restored = RuntimeState::import_snapshot(data.clone()).unwrap();
        let snapshot = restored.snapshot(data.recipe.completed_time);

        let original = RuntimeState::import_snapshot(data).unwrap();
        let original_snapshot = original.snapshot(snapshot.time);

        assert_eq!(
            snapshot.physical_state_digest,
            original_snapshot.physical_state_digest
        );
        assert_eq!(snapshot.history_digest, original_snapshot.history_digest);
    }

    #[test]
    fn runtime_state_import_rejects_unknown_trace_reference() {
        let mut data = populated_snapshot_data();
        data.physical_counters.latest_physical_trace = TraceId::new(u64::MAX);

        assert!(matches!(
            RuntimeState::import_snapshot(data),
            Err(crate::RuntimeError::InvalidSnapshot(
                "unknown trace reference"
            ))
        ));
    }

    #[test]
    fn envelope_assemble_disassemble_roundtrip() {
        let data = populated_snapshot_data();
        let envelope = assemble_envelope(&data).unwrap();
        let encoded = envelope.encode().unwrap();
        let decoded = SnapshotEnvelope::decode(&encoded).unwrap();
        let restored = disassemble_envelope(&decoded).unwrap();

        assert_eq!(restored.recipe, data.recipe);
        assert_eq!(restored.spatial, data.spatial);
        assert_eq!(restored.mana, data.mana);
        assert_eq!(restored.resolution, data.resolution);
        assert_eq!(restored.resolution_policy, data.resolution_policy);
        assert_eq!(restored.pattern_history, data.pattern_history);
        assert_eq!(restored.physical_counters, data.physical_counters);
        assert_eq!(restored.actors_objective, data.actors_objective);
        assert_eq!(restored.actors_subjective, data.actors_subjective);
        assert_eq!(restored.population, data.population);
        assert_eq!(restored.bootstrap, data.bootstrap);
        assert_eq!(restored.traces, data.traces);
    }

    #[test]
    fn runtime_from_snapshot_resumes_at_completed_tick() {
        let mut runtime = Runtime::from_seed(42).unwrap();
        runtime.run_ticks(10).unwrap();

        let data = runtime.export_snapshot().unwrap();
        let completed_time = data.recipe.completed_time;

        let mut resumed = Runtime::from_snapshot(data).unwrap();
        assert_eq!(resumed.current_time(), completed_time);

        let mut original = Runtime::from_seed(42).unwrap();
        original.run_ticks(15).unwrap();

        resumed.run_ticks(5).unwrap();

        let original_snapshot = original.snapshot().unwrap();
        let resumed_snapshot = resumed.snapshot().unwrap();

        assert_eq!(
            original_snapshot.physical_state_digest,
            resumed_snapshot.physical_state_digest
        );
        assert_eq!(original_snapshot.history_digest, resumed_snapshot.history_digest);
    }
}
