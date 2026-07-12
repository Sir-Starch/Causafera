use std::sync::{Arc, Mutex, MutexGuard};

use ontopolis_core::{
    CausalCommitError, CausalEffect, CausalEffectError, CausalEventProposal,
    CausalEventProposalError, CausalTarget, CausalTraceStore, DeterministicConfig,
    EventProposalKey, Phase, RandomStream, Scheduler, StateFingerprint, System,
};
use ontopolis_domains::{ManaError, ManaField, ManaParameters, PhysicalPatternSample};
use ontopolis_resolution::{
    CausalRelevanceSignal, ChannelWeight, ResolutionError, ResolutionField, ResolutionPolicy,
};
use ontopolis_types::{
    ChunkCoord, ChunkId, EventKindId, LocalCoord, ManaFieldId, PhysicalPatternId,
    ResolutionChannelId, ResolutionFieldId, SimulationTime, StateObjectKindId, StatePropertyId,
    TraceId,
};
use thiserror::Error;

pub const MAX_RUNTIME_TICKS: u64 = 1_000_000;

const PHYSICAL_SYSTEM_ID: u64 = 10;
const MANA_SYSTEM_ID: u64 = 20;
const RESOLUTION_SYSTEM_ID: u64 = 30;
const ROOT_EVENT_KIND: u64 = 1;
const PHYSICAL_EVENT_KIND: u64 = 2;
const MANA_EVENT_KIND: u64 = 3;
const RESOLUTION_EVENT_KIND: u64 = 4;
const RUNTIME_OBJECT_KIND: u64 = 1;
const PHYSICAL_OBJECT_KIND: u64 = 2;
const MANA_OBJECT_KIND: u64 = 3;
const RESOLUTION_OBJECT_KIND: u64 = 4;
const ROOT_PROPERTY: u64 = 1;
const PHYSICAL_PROPERTY: u64 = 2;
const MANA_PROPERTY: u64 = 3;
const RESOLUTION_PROPERTY: u64 = 4;
const RESOLUTION_CHANNEL: u64 = 1;
const ROOT_CHUNK: u64 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PhysicalPatternSchedule {
    pub interval_ticks: u64,
    pub magnitude: u32,
    pub suppressed_from: Option<SimulationTime>,
    pub suppressed_through: Option<SimulationTime>,
}

impl PhysicalPatternSchedule {
    pub const fn continuous(magnitude: u32) -> Self {
        Self {
            interval_ticks: 1,
            magnitude,
            suppressed_from: None,
            suppressed_through: None,
        }
    }

    pub fn with_suppression(
        mut self,
        from: SimulationTime,
        through: SimulationTime,
    ) -> Result<Self, RuntimeError> {
        if from.raw() == 0 || through < from {
            return Err(RuntimeError::InvalidPatternSchedule);
        }
        self.suppressed_from = Some(from);
        self.suppressed_through = Some(through);
        Ok(self)
    }

    fn validate(self) -> Result<Self, RuntimeError> {
        if self.interval_ticks == 0 || self.magnitude == 0 {
            return Err(RuntimeError::InvalidPatternSchedule);
        }
        match (self.suppressed_from, self.suppressed_through) {
            (None, None) => {}
            (Some(from), Some(through)) if from.raw() > 0 && through >= from => {}
            _ => return Err(RuntimeError::InvalidPatternSchedule),
        }
        Ok(self)
    }

    fn emits_at(self, time: SimulationTime) -> bool {
        if time.raw() % self.interval_ticks != 0 {
            return false;
        }
        !matches!(
            (self.suppressed_from, self.suppressed_through),
            (Some(from), Some(through)) if time >= from && time <= through
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeConfig {
    pub deterministic: DeterministicConfig,
    pub field_extent: u8,
    pub pattern_schedule: PhysicalPatternSchedule,
}

impl RuntimeConfig {
    pub fn new(world_seed: u64) -> Self {
        Self {
            deterministic: DeterministicConfig::new(world_seed),
            field_extent: 3,
            pattern_schedule: PhysicalPatternSchedule::continuous(1_024),
        }
    }

    fn validate(self) -> Result<Self, RuntimeError> {
        if self.field_extent < 3 {
            return Err(RuntimeError::InvalidFieldExtent);
        }
        self.pattern_schedule.validate()?;
        Ok(self)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RuntimeSnapshot {
    pub time: SimulationTime,
    pub canonical_state: StateFingerprint,
    pub mana_total: i64,
    pub mana_maximum: i64,
    pub mana_changed_components: u32,
    pub resolution_relevance: i64,
    pub resolution_level: u8,
    pub causal_trace_count: u64,
    pub physical_events: u64,
    pub mana_cell_changes: u64,
    pub resolution_changes: u64,
    pub latest_trace: TraceId,
}

/// Headless deterministic runtime for the first executable causal experiment.
pub struct Runtime {
    scheduler: Scheduler,
    state: Arc<Mutex<RuntimeState>>,
}

impl Runtime {
    pub fn new(config: RuntimeConfig) -> Result<Self, RuntimeError> {
        let config = config.validate()?;
        let state = Arc::new(Mutex::new(RuntimeState::new(&config)?));
        let mut scheduler = Scheduler::new(config.deterministic.clone());
        scheduler.register_system(
            Phase::Physics,
            Box::new(PhysicalPatternSystem::new(
                Arc::clone(&state),
                config.pattern_schedule,
                config.field_extent,
            )),
        );
        scheduler.register_system(
            Phase::Mana,
            Box::new(ManaRuntimeSystem::new(Arc::clone(&state))),
        );
        scheduler.register_system(
            Phase::Resolution,
            Box::new(ResolutionRuntimeSystem::new(Arc::clone(&state))),
        );
        Ok(Self { scheduler, state })
    }

    pub fn from_seed(world_seed: u64) -> Result<Self, RuntimeError> {
        Self::new(RuntimeConfig::new(world_seed))
    }

    pub fn current_time(&self) -> SimulationTime {
        self.scheduler.current_time()
    }

    pub fn tick(&mut self) -> Result<RuntimeSnapshot, RuntimeError> {
        if self.scheduler.current_time().raw() >= MAX_RUNTIME_TICKS {
            return Err(RuntimeError::TickLimitExceeded);
        }
        self.scheduler.tick();
        let state = self.lock_state()?;
        if let Some(error) = state.failure.clone() {
            return Err(error);
        }
        if state.advanced_through != self.scheduler.current_time() {
            return Err(RuntimeError::PhaseDesynchronized);
        }
        Ok(state.snapshot(self.scheduler.current_time()))
    }

    pub fn run_ticks(&mut self, ticks: u64) -> Result<RuntimeSnapshot, RuntimeError> {
        if ticks == 0 || ticks > MAX_RUNTIME_TICKS.saturating_sub(self.current_time().raw()) {
            return Err(RuntimeError::InvalidTickCount { ticks });
        }
        let mut snapshot = self.snapshot()?;
        for _ in 0..ticks {
            snapshot = self.tick()?;
        }
        Ok(snapshot)
    }

    pub fn snapshot(&self) -> Result<RuntimeSnapshot, RuntimeError> {
        let state = self.lock_state()?;
        if let Some(error) = state.failure.clone() {
            return Err(error);
        }
        Ok(state.snapshot(self.scheduler.current_time()))
    }

    fn lock_state(&self) -> Result<MutexGuard<'_, RuntimeState>, RuntimeError> {
        self.state.lock().map_err(|_| RuntimeError::StatePoisoned)
    }
}

impl Default for Runtime {
    fn default() -> Self {
        Self::from_seed(0).expect("default runtime configuration is valid")
    }
}

#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum RuntimeError {
    #[error("runtime field extent must be at least three")]
    InvalidFieldExtent,
    #[error("physical pattern schedule is invalid")]
    InvalidPatternSchedule,
    #[error("invalid runtime tick count: {ticks}")]
    InvalidTickCount { ticks: u64 },
    #[error("runtime tick limit exceeded")]
    TickLimitExceeded,
    #[error("runtime state lock was poisoned")]
    StatePoisoned,
    #[error("runtime systems did not advance through the scheduler time")]
    PhaseDesynchronized,
    #[error("causal effect construction failed: {0}")]
    CausalEffect(#[from] CausalEffectError),
    #[error("causal proposal construction failed: {0}")]
    CausalProposal(#[from] CausalEventProposalError),
    #[error("causal commit failed: {0}")]
    CausalCommit(#[from] CausalCommitError),
    #[error("mana evolution failed: {0:?}")]
    Mana(ManaError),
    #[error("resolution evolution failed: {0}")]
    Resolution(#[from] ResolutionError),
}

impl From<ManaError> for RuntimeError {
    fn from(error: ManaError) -> Self {
        Self::Mana(error)
    }
}

struct RuntimeState {
    traces: CausalTraceStore,
    mana: ManaField,
    resolution: ResolutionField,
    resolution_policy: ResolutionPolicy,
    physical_pattern: PhysicalPatternId,
    pending_samples: Vec<PhysicalPatternSample>,
    latest_physical_trace: TraceId,
    latest_mana_trace: Option<TraceId>,
    advanced_through: SimulationTime,
    physical_counter: u64,
    physical_events: u64,
    mana_cell_changes: u64,
    resolution_changes: u64,
    last_mana_changes: u32,
    failure: Option<RuntimeError>,
}

impl RuntimeState {
    fn new(config: &RuntimeConfig) -> Result<Self, RuntimeError> {
        let mut traces = CausalTraceStore::new();
        let root = CausalEventProposal::new(
            EventProposalKey::new(0, 0, 0),
            EventKindId::new(ROOT_EVENT_KIND),
            Vec::new(),
            vec![CausalEffect::new(
                CausalTarget::new(
                    StateObjectKindId::new(RUNTIME_OBJECT_KIND),
                    0,
                    StatePropertyId::new(ROOT_PROPERTY),
                ),
                fingerprint_u64(0x0101, 0),
                fingerprint_u64(0x0102, config.deterministic.world_seed),
            )?],
        )?;
        let root_trace =
            traces.commit_batch(SimulationTime::new(0), Phase::Physics, vec![root])?[0];
        let mana = ManaField::new(
            ManaFieldId::new(1),
            ChunkCoord::new(0, 0, 0),
            config.field_extent,
        )?;
        let resolution = ResolutionField::new(
            ResolutionFieldId::new(1),
            SimulationTime::new(0),
            vec![ChunkId::new(ROOT_CHUNK)],
            vec![root_trace],
        )?;
        let resolution_policy = ResolutionPolicy::new(
            10_000,
            900,
            100,
            vec![500, 2_000, 5_000],
            vec![ChannelWeight::new(
                ResolutionChannelId::new(RESOLUTION_CHANNEL),
                1_000,
            )?],
        )?;
        Ok(Self {
            traces,
            mana,
            resolution,
            resolution_policy,
            physical_pattern: PhysicalPatternId::new(mix64(
                config.deterministic.world_seed ^ 0xA076_1D64_78BD_642F,
            )),
            pending_samples: Vec::with_capacity(3),
            latest_physical_trace: root_trace,
            latest_mana_trace: None,
            advanced_through: SimulationTime::new(0),
            physical_counter: 0,
            physical_events: 0,
            mana_cell_changes: 0,
            resolution_changes: 0,
            last_mana_changes: 0,
            failure: None,
        })
    }

    fn snapshot(&self, time: SimulationTime) -> RuntimeSnapshot {
        let mana_total = self.mana.intensity().iter().copied().sum();
        let mana_maximum = self.mana.intensity().iter().copied().max().unwrap_or(0);
        let resolution = self
            .resolution
            .entry(ChunkId::new(ROOT_CHUNK))
            .expect("runtime resolution field always contains its root chunk");
        let latest_trace = self
            .traces
            .iter()
            .last()
            .expect("runtime always retains a root trace")
            .trace_id;
        RuntimeSnapshot {
            time,
            canonical_state: self.canonical_state(time),
            mana_total,
            mana_maximum,
            mana_changed_components: self.last_mana_changes,
            resolution_relevance: resolution.relevance,
            resolution_level: resolution.level,
            causal_trace_count: self.traces.len() as u64,
            physical_events: self.physical_events,
            mana_cell_changes: self.mana_cell_changes,
            resolution_changes: self.resolution_changes,
            latest_trace,
        }
    }

    fn canonical_state(&self, time: SimulationTime) -> StateFingerprint {
        let mut digest = CanonicalDigest::new();
        digest.write(time.raw());
        digest.write(self.physical_counter);
        digest.write(self.mana.observed_through().raw());
        for value in self.mana.intensity() {
            digest.write(*value as u64);
        }
        let entry = self
            .resolution
            .entry(ChunkId::new(ROOT_CHUNK))
            .expect("runtime resolution field always contains its root chunk");
        digest.write(self.resolution.evaluated_through().raw());
        digest.write(entry.relevance as u64);
        digest.write(u64::from(entry.level));
        for event in self.traces.iter() {
            digest.write(event.event_id.raw());
            digest.write(event.trace_id.raw());
            digest.write(event.time.raw());
            digest.write(u64::from(event.phase.id().0));
            digest.write(event.kind.raw());
            for cause in event.causes {
                digest.write(cause.raw());
            }
            for effect in event.effects {
                digest.write(effect.target().object_kind().raw());
                digest.write(effect.target().object_id());
                digest.write(effect.target().property().raw());
                digest.write_bytes(effect.before().bytes());
                digest.write_bytes(effect.after().bytes());
            }
        }
        digest.finish()
    }
}

struct PhysicalPatternSystem {
    state: Arc<Mutex<RuntimeState>>,
    schedule: PhysicalPatternSchedule,
    extent: u8,
    next_time: SimulationTime,
}

impl PhysicalPatternSystem {
    fn new(state: Arc<Mutex<RuntimeState>>, schedule: PhysicalPatternSchedule, extent: u8) -> Self {
        Self {
            state,
            schedule,
            extent,
            next_time: SimulationTime::new(1),
        }
    }

    fn execute(&mut self) -> Result<(), RuntimeError> {
        let mut state = self.state.lock().map_err(|_| RuntimeError::StatePoisoned)?;
        state.pending_samples.clear();
        if self.schedule.emits_at(self.next_time) {
            let next_counter = state.physical_counter.saturating_add(1);
            let event = CausalEventProposal::new(
                EventProposalKey::new(PHYSICAL_SYSTEM_ID, 0, 0),
                EventKindId::new(PHYSICAL_EVENT_KIND),
                vec![state.latest_physical_trace],
                vec![CausalEffect::new(
                    CausalTarget::new(
                        StateObjectKindId::new(PHYSICAL_OBJECT_KIND),
                        0,
                        StatePropertyId::new(PHYSICAL_PROPERTY),
                    ),
                    fingerprint_u64(0x0201, state.physical_counter),
                    fingerprint_u64(0x0201, next_counter),
                )?],
            )?;
            let trace = state
                .traces
                .commit_batch(self.next_time, Phase::Physics, vec![event])?[0];
            state.physical_counter = next_counter;
            state.physical_events += 1;
            state.latest_physical_trace = trace;
            let center = self.extent / 2;
            let pattern = state.physical_pattern;
            for (ordinal, x) in [center - 1, center, center + 1].into_iter().enumerate() {
                state.pending_samples.push(PhysicalPatternSample {
                    pattern,
                    position: LocalCoord::new(x, center, center),
                    observed_at: self.next_time,
                    magnitude: self.schedule.magnitude,
                    source_ordinal: ordinal as u32,
                    cause: trace,
                });
            }
        }
        self.next_time = self.next_time.tick();
        Ok(())
    }
}

impl System for PhysicalPatternSystem {
    fn run(&mut self, _stream: &mut RandomStream) {
        if let Err(error) = self.execute() {
            if let Ok(mut state) = self.state.lock() {
                state.failure.get_or_insert(error);
            }
        }
    }
}

struct ManaRuntimeSystem {
    state: Arc<Mutex<RuntimeState>>,
    next_time: SimulationTime,
    parameters: ManaParameters,
}

impl ManaRuntimeSystem {
    fn new(state: Arc<Mutex<RuntimeState>>) -> Self {
        Self {
            state,
            next_time: SimulationTime::new(1),
            parameters: ManaParameters {
                base_response: 128,
                recurrence_response: 128,
                periodicity_response: 0,
                synchrony_response: 128,
                spatial_response: 128,
                diffusion: 128,
                decay: 24,
                maximum_intensity: 1_000_000,
            },
        }
    }

    fn execute(&mut self) -> Result<(), RuntimeError> {
        let mut state = self.state.lock().map_err(|_| RuntimeError::StatePoisoned)?;
        let proposal = state.mana.propose_evolution(
            self.next_time,
            self.parameters,
            &state.pending_samples,
        )?;
        let changes = proposal.changes().to_vec();
        let events = changes
            .iter()
            .map(|change| {
                CausalEventProposal::new(
                    EventProposalKey::new(MANA_SYSTEM_ID, u64::from(change.cell_index), 0),
                    EventKindId::new(MANA_EVENT_KIND),
                    change.causes.clone(),
                    vec![CausalEffect::new(
                        CausalTarget::new(
                            StateObjectKindId::new(MANA_OBJECT_KIND),
                            u64::from(change.cell_index),
                            StatePropertyId::new(MANA_PROPERTY),
                        ),
                        fingerprint_i64(0x0301, change.before),
                        fingerprint_i64(0x0301, change.after),
                    )?],
                )
                .map_err(RuntimeError::from)
            })
            .collect::<Result<Vec<_>, _>>()?;
        let traces = state
            .traces
            .commit_batch(self.next_time, Phase::Mana, events)?;
        let changed_count = changes.len() as u32;
        state.mana = proposal.commit(&traces)?;
        state.pending_samples.clear();
        state.last_mana_changes = changed_count;
        state.mana_cell_changes = state
            .mana_cell_changes
            .saturating_add(u64::from(changed_count));
        if let Some(trace) = traces.last().copied() {
            state.latest_mana_trace = Some(trace);
        }
        self.next_time = self.next_time.tick();
        Ok(())
    }
}

impl System for ManaRuntimeSystem {
    fn run(&mut self, _stream: &mut RandomStream) {
        if let Err(error) = self.execute() {
            if let Ok(mut state) = self.state.lock() {
                state.failure.get_or_insert(error);
            }
        }
    }
}

struct ResolutionRuntimeSystem {
    state: Arc<Mutex<RuntimeState>>,
    next_time: SimulationTime,
}

impl ResolutionRuntimeSystem {
    fn new(state: Arc<Mutex<RuntimeState>>) -> Self {
        Self {
            state,
            next_time: SimulationTime::new(1),
        }
    }

    fn execute(&mut self) -> Result<(), RuntimeError> {
        let mut state = self.state.lock().map_err(|_| RuntimeError::StatePoisoned)?;
        let total: i64 = state.mana.intensity().iter().copied().sum();
        let signals = match (total > 0, state.latest_mana_trace) {
            (true, Some(trace)) => vec![CausalRelevanceSignal::new(
                ChunkId::new(ROOT_CHUNK),
                ChunkId::new(ROOT_CHUNK),
                ResolutionChannelId::new(RESOLUTION_CHANNEL),
                total.clamp(1, 1_000),
                trace,
                self.next_time.raw() as u32,
            )?],
            _ => Vec::new(),
        };
        let proposal = state.resolution.propose_evaluation(
            self.next_time,
            &state.resolution_policy,
            &signals,
        )?;
        let changes = proposal.changes().to_vec();
        let events = changes
            .iter()
            .map(|change| {
                CausalEventProposal::new(
                    EventProposalKey::new(RESOLUTION_SYSTEM_ID, change.chunk.raw(), 0),
                    EventKindId::new(RESOLUTION_EVENT_KIND),
                    change.causes().to_vec(),
                    vec![CausalEffect::new(
                        CausalTarget::new(
                            StateObjectKindId::new(RESOLUTION_OBJECT_KIND),
                            change.chunk.raw(),
                            StatePropertyId::new(RESOLUTION_PROPERTY),
                        ),
                        fingerprint_pair(
                            0x0401,
                            change.before_relevance,
                            i64::from(change.before_level),
                        ),
                        fingerprint_pair(
                            0x0401,
                            change.after_relevance,
                            i64::from(change.after_level),
                        ),
                    )?],
                )
                .map_err(RuntimeError::from)
            })
            .collect::<Result<Vec<_>, _>>()?;
        let traces = state
            .traces
            .commit_batch(self.next_time, Phase::Resolution, events)?;
        state.resolution = proposal.commit(&traces)?;
        state.resolution_changes = state
            .resolution_changes
            .saturating_add(changes.len() as u64);
        state.advanced_through = self.next_time;
        self.next_time = self.next_time.tick();
        Ok(())
    }
}

impl System for ResolutionRuntimeSystem {
    fn run(&mut self, _stream: &mut RandomStream) {
        if let Err(error) = self.execute() {
            if let Ok(mut state) = self.state.lock() {
                state.failure.get_or_insert(error);
            }
        }
    }
}

fn fingerprint_u64(tag: u64, value: u64) -> StateFingerprint {
    fingerprint_words([tag, value, mix64(tag ^ value), tag.rotate_left(17) ^ value])
}

fn fingerprint_i64(tag: u64, value: i64) -> StateFingerprint {
    fingerprint_u64(tag, value as u64)
}

fn fingerprint_pair(tag: u64, first: i64, second: i64) -> StateFingerprint {
    fingerprint_words([
        tag,
        first as u64,
        second as u64,
        mix64(tag ^ first as u64 ^ (second as u64).rotate_left(23)),
    ])
}

fn fingerprint_words(words: [u64; 4]) -> StateFingerprint {
    let mut bytes = [0_u8; 32];
    for (index, word) in words.into_iter().enumerate() {
        bytes[index * 8..(index + 1) * 8].copy_from_slice(&word.to_le_bytes());
    }
    StateFingerprint::new(bytes)
}

fn mix64(mut value: u64) -> u64 {
    value ^= value >> 30;
    value = value.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value ^= value >> 27;
    value = value.wrapping_mul(0x94D0_49BB_1331_11EB);
    value ^ (value >> 31)
}

struct CanonicalDigest([u64; 4]);

impl CanonicalDigest {
    fn new() -> Self {
        Self([
            0x243F_6A88_85A3_08D3,
            0x1319_8A2E_0370_7344,
            0xA409_3822_299F_31D0,
            0x082E_FA98_EC4E_6C89,
        ])
    }

    fn write(&mut self, value: u64) {
        for (index, lane) in self.0.iter_mut().enumerate() {
            *lane = mix64(
                lane.wrapping_add(value.rotate_left((index as u32 * 13) + 1))
                    .wrapping_add(index as u64),
            );
        }
    }

    fn write_bytes(&mut self, bytes: [u8; 32]) {
        for chunk in bytes.chunks_exact(8) {
            self.write(u64::from_le_bytes(chunk.try_into().expect("exact chunk")));
        }
    }

    fn finish(self) -> StateFingerprint {
        fingerprint_words(self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_executes_a_long_causal_run_without_errors() {
        let mut runtime = Runtime::from_seed(42).unwrap();
        let snapshot = runtime.run_ticks(512).unwrap();
        assert_eq!(snapshot.time, SimulationTime::new(512));
        assert!(snapshot.mana_total > 0);
        assert!(snapshot.causal_trace_count > 512);
        assert!(snapshot.resolution_level > 0);
    }

    #[test]
    fn strict_replay_has_identical_canonical_state() {
        let mut first = Runtime::from_seed(7).unwrap();
        let mut second = Runtime::from_seed(7).unwrap();
        let first = first.run_ticks(256).unwrap();
        let second = second.run_ticks(256).unwrap();
        assert_eq!(first, second);
    }

    #[test]
    fn physical_suppression_changes_the_causal_trajectory() {
        let control_config = RuntimeConfig::new(9);
        let mut intervention_config = control_config.clone();
        intervention_config.pattern_schedule = intervention_config
            .pattern_schedule
            .with_suppression(SimulationTime::new(80), SimulationTime::new(160))
            .unwrap();
        let mut control = Runtime::new(control_config).unwrap();
        let mut intervention = Runtime::new(intervention_config).unwrap();
        let control = control.run_ticks(256).unwrap();
        let intervention = intervention.run_ticks(256).unwrap();
        assert_ne!(control.canonical_state, intervention.canonical_state);
        assert!(control.physical_events > intervention.physical_events);
    }
}
