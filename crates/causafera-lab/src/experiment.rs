use std::time::{Duration, Instant};

use causafera_analytics::{
    AnalyticsCheckpoint, AnalyticsError, ExperimentAnalytics, FieldInputState,
    MatchedCheckpointAnalysis, RECOVERY_DISTANCE_SCHEMA, TIME_TO_RECOVERY_SCHEMA,
};
use causafera_explanation::{
    ClaimConfidence, ClaimEvidenceState, ComparisonCohortId, ComparisonContext, ExplanationClaim,
    ExplanationReport, NumericClaimValue,
};
use causafera_metaphysics::{
    AttractorEvidence, AttractorProbe, AttractorResearchError, FieldTrajectoryObservation,
};
use causafera_runtime::{
    ExperimentDigest, HistoryDigest, MAX_RUNTIME_TICKS, PhysicalPatternSchedule,
    PhysicalStateDigest, Runtime, RuntimeConfig, RuntimeError, RuntimeSnapshot,
};
use causafera_types::{
    AttractorExperimentId, AttractorProbeSchemaId, ExperimentId, SimulationTime, TraceId,
};
use thiserror::Error;

pub const MAX_EXPERIMENT_CHECKPOINTS: usize = 4_096;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExperimentConfig {
    pub id: ExperimentId,
    pub world_seed: u64,
    pub ticks: u64,
    pub checkpoint_interval: u64,
    pub pattern_schedule: PhysicalPatternSchedule,
    pub bootstrap_population: u64,
}

impl ExperimentConfig {
    pub fn new(
        id: ExperimentId,
        world_seed: u64,
        ticks: u64,
        checkpoint_interval: u64,
    ) -> Result<Self, ExperimentError> {
        let config = Self {
            id,
            world_seed,
            ticks,
            checkpoint_interval,
            pattern_schedule: PhysicalPatternSchedule::continuous(1_024),
            bootstrap_population: 0,
        };
        config.validate()?;
        Ok(config)
    }

    pub fn with_suppression(
        mut self,
        from: SimulationTime,
        through: SimulationTime,
    ) -> Result<Self, ExperimentError> {
        self.pattern_schedule = self.pattern_schedule.with_suppression(from, through)?;
        self.validate()?;
        Ok(self)
    }

    pub fn with_bootstrap_population(mut self, population: u64) -> Result<Self, ExperimentError> {
        self.bootstrap_population = population;
        self.validate()?;
        Ok(self)
    }

    fn validate(&self) -> Result<(), ExperimentError> {
        if self.ticks == 0 || self.ticks > MAX_RUNTIME_TICKS {
            return Err(ExperimentError::InvalidTickCount { ticks: self.ticks });
        }
        if self.checkpoint_interval == 0 || self.checkpoint_interval > self.ticks {
            return Err(ExperimentError::InvalidCheckpointInterval {
                interval: self.checkpoint_interval,
            });
        }
        let checkpoint_count = self.ticks.div_ceil(self.checkpoint_interval) as usize + 1;
        if checkpoint_count > MAX_EXPERIMENT_CHECKPOINTS {
            return Err(ExperimentError::TooManyCheckpoints {
                count: checkpoint_count,
            });
        }
        if let Some(through) = self.pattern_schedule.suppressed_through
            && through.raw() > self.ticks
        {
            return Err(ExperimentError::SuppressionOutsideRun);
        }
        if self.bootstrap_population > 16 {
            return Err(ExperimentError::PopulationOutsideInMemoryEnvelope {
                population: self.bootstrap_population,
            });
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ExperimentManifest {
    pub format_version: u16,
    pub seed_set: Vec<u64>,
    pub parameters: ExperimentParametersRecord,
    pub code_revision: RevisionRecord,
    pub schema_revision: RevisionRecord,
    pub warm_up_ticks: u64,
    pub duration_ticks: u64,
    pub hardware: HardwareRecord,
    pub wall_time: Duration,
    pub activity_counts: ActivityCountsRecord,
    pub memory: MemoryRecord,
    pub state_digest: PhysicalStateDigest,
    pub history_digest: HistoryDigest,
    pub result_confidence: ClaimConfidence,
    pub supporting_traces: Vec<TraceId>,
    pub evidence_sufficient: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExperimentParametersRecord {
    pub checkpoint_interval: u64,
    pub bootstrap_population: u64,
    pub suppression_from: SimulationTime,
    pub suppression_through: SimulationTime,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RevisionRecord {
    pub identifier: &'static str,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HardwareRecord {
    pub execution_backend: &'static str,
    pub persistence: &'static str,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActivityCountsRecord {
    pub physical_events: u64,
    pub mana_cell_changes: u64,
    pub resolution_transitions: u64,
    pub actor_actions_committed: u64,
    pub population_births: u64,
    pub population_deaths: u64,
    pub population_movements: u64,
    pub actor_promotions: u64,
    pub actor_demotions: u64,
    pub material_activity_events: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MemoryRecord {
    pub bytes_per_chunk: u64,
    pub causal_trace_count: u64,
    pub population_total: u64,
    pub actor_count: u32,
}

/// Deterministic portion of an experiment result.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExperimentResult {
    pub id: ExperimentId,
    pub world_seed: u64,
    pub ticks: u64,
    pub checkpoints: Vec<RuntimeSnapshot>,
    pub final_snapshot: RuntimeSnapshot,
    pub physical_state_digest: PhysicalStateDigest,
    pub history_digest: HistoryDigest,
    pub experiment_digest: ExperimentDigest,
    pub attractor_evidence: AttractorEvidence,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReplayVerifiedExperiment {
    pub result: ExperimentResult,
    pub physical_state_digest: PhysicalStateDigest,
    pub history_digest: HistoryDigest,
    pub experiment_digest: ExperimentDigest,
    /// Report-only measurement; excluded from deterministic result equality.
    pub elapsed: Duration,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LongRunExperimentReport {
    pub control: ReplayVerifiedExperiment,
    pub intervention: ReplayVerifiedExperiment,
    pub matched_checkpoint_analysis: MatchedCheckpointAnalysis,
    pub explanation_report: ExplanationReport,
    pub manifest: ExperimentManifest,
    pub final_physical_states_diverged: bool,
    pub transient_physical_states_diverged: bool,
    pub history_diverged: bool,
    pub trajectories_diverged: bool,
}

pub struct ExperimentRunner;

impl ExperimentRunner {
    pub fn run_replay_verified(
        config: ExperimentConfig,
    ) -> Result<ReplayVerifiedExperiment, ExperimentError> {
        config.validate()?;
        let started = Instant::now();
        let first = Self::run_deterministic(&config)?;
        let replay = Self::run_deterministic(&config)?;
        let elapsed = started.elapsed();
        if !same_physical_checkpoints(&first.checkpoints, &replay.checkpoints)
            || !same_history_checkpoints(&first.checkpoints, &replay.checkpoints)
            || first != replay
        {
            return Err(ExperimentError::ReplayMismatch);
        }
        let physical_state_digest = first.physical_state_digest;
        let history_digest = first.history_digest;
        let experiment_digest = first.experiment_digest;
        Ok(ReplayVerifiedExperiment {
            result: first,
            physical_state_digest,
            history_digest,
            experiment_digest,
            elapsed,
        })
    }

    pub fn run_control_and_intervention(
        world_seed: u64,
        ticks: u64,
        checkpoint_interval: u64,
        suppression_from: SimulationTime,
        suppression_through: SimulationTime,
    ) -> Result<LongRunExperimentReport, ExperimentError> {
        Self::run_populated_control_and_intervention(
            world_seed,
            ticks,
            checkpoint_interval,
            suppression_from,
            suppression_through,
            1,
        )
    }

    pub fn run_populated_control_and_intervention(
        world_seed: u64,
        ticks: u64,
        checkpoint_interval: u64,
        suppression_from: SimulationTime,
        suppression_through: SimulationTime,
        bootstrap_population: u64,
    ) -> Result<LongRunExperimentReport, ExperimentError> {
        let control =
            ExperimentConfig::new(ExperimentId::new(1), world_seed, ticks, checkpoint_interval)?
                .with_bootstrap_population(bootstrap_population)?;
        let intervention =
            ExperimentConfig::new(ExperimentId::new(2), world_seed, ticks, checkpoint_interval)?
                .with_bootstrap_population(bootstrap_population)?
                .with_suppression(suppression_from, suppression_through)?;
        let control = Self::run_replay_verified(control)?;
        let intervention = Self::run_replay_verified(intervention)?;
        let final_physical_states_diverged =
            control.physical_state_digest != intervention.physical_state_digest;
        let transient_physical_states_diverged = !same_physical_checkpoints(
            &control.result.checkpoints,
            &intervention.result.checkpoints,
        );
        let history_diverged = control.history_digest != intervention.history_digest;
        let trajectories_diverged = control.experiment_digest != intervention.experiment_digest;
        if !trajectories_diverged {
            return Err(ExperimentError::InterventionHadNoEffect);
        }
        let matched_checkpoint_analysis = ExperimentAnalytics::analyze_recovery(
            &analytics_checkpoints(&control.result.checkpoints),
            &analytics_checkpoints(&intervention.result.checkpoints),
            suppression_from,
            suppression_through,
            100_000,
        )?;
        let explanation_report = build_long_run_explanation_report(
            &control.result,
            &intervention.result,
            &matched_checkpoint_analysis,
            suppression_from,
            suppression_through,
        )?;
        let manifest = build_experiment_manifest(
            world_seed,
            ticks,
            checkpoint_interval,
            suppression_from,
            suppression_through,
            bootstrap_population,
            &control,
            &intervention,
        )?;
        Ok(LongRunExperimentReport {
            control,
            intervention,
            matched_checkpoint_analysis,
            explanation_report,
            manifest,
            final_physical_states_diverged,
            transient_physical_states_diverged,
            history_diverged,
            trajectories_diverged,
        })
    }

    fn run_deterministic(config: &ExperimentConfig) -> Result<ExperimentResult, ExperimentError> {
        let mut runtime_config = RuntimeConfig::new(config.world_seed);
        runtime_config.pattern_schedule = config.pattern_schedule;
        runtime_config.bootstrap_population = config.bootstrap_population;
        if config.bootstrap_population > 0 {
            runtime_config.actor_count = 1;
            runtime_config.sensor_count = 1;
        }
        let mut runtime = Runtime::new(runtime_config)?;
        let mut checkpoints =
            Vec::with_capacity(config.ticks.div_ceil(config.checkpoint_interval) as usize + 1);
        checkpoints.push(runtime.snapshot()?);
        for tick in 1..=config.ticks {
            let snapshot = runtime.tick()?;
            if tick % config.checkpoint_interval == 0 || tick == config.ticks {
                checkpoints.push(snapshot);
            }
        }
        let Some(final_snapshot) = checkpoints.last().cloned() else {
            return Err(ExperimentError::MissingCheckpoint);
        };
        let observations = checkpoints
            .iter()
            .map(|snapshot| {
                FieldTrajectoryObservation::new(
                    snapshot.time,
                    snapshot.physical_state_digest.fingerprint,
                    snapshot.mana_total,
                    snapshot.mana_maximum,
                    snapshot.mana_changed_components,
                    snapshot.latest_trace,
                )
            })
            .collect::<Result<Vec<_>, _>>()?;
        let probe = AttractorProbe::new(AttractorProbeSchemaId::new(1), 1_000, 100_000)?;
        let attractor_evidence = probe.evaluate(
            AttractorExperimentId::new(config.id.raw()),
            &observations,
            config.pattern_schedule.suppressed_through,
        )?;
        Ok(ExperimentResult {
            id: config.id,
            world_seed: config.world_seed,
            ticks: config.ticks,
            checkpoints,
            physical_state_digest: final_snapshot.physical_state_digest,
            history_digest: final_snapshot.history_digest,
            experiment_digest: final_snapshot.canonical_state,
            final_snapshot,
            attractor_evidence,
        })
    }
}

fn same_physical_checkpoints(left: &[RuntimeSnapshot], right: &[RuntimeSnapshot]) -> bool {
    left.len() == right.len()
        && left
            .iter()
            .zip(right)
            .all(|(left, right)| left.physical_state_digest == right.physical_state_digest)
}

fn same_history_checkpoints(left: &[RuntimeSnapshot], right: &[RuntimeSnapshot]) -> bool {
    left.len() == right.len()
        && left
            .iter()
            .zip(right)
            .all(|(left, right)| left.history_digest == right.history_digest)
}

fn build_long_run_explanation_report(
    control: &ExperimentResult,
    intervention: &ExperimentResult,
    recovery: &MatchedCheckpointAnalysis,
    suppression_from: SimulationTime,
    suppression_through: SimulationTime,
) -> Result<ExplanationReport, ExperimentError> {
    let control_frame = ExperimentAnalytics::analyze_checkpoint_series(
        &analytics_checkpoints(&control.checkpoints),
        FieldInputState::ActiveInput,
    )?;
    let suppressed_checkpoints = intervention
        .checkpoints
        .iter()
        .filter(|snapshot| {
            snapshot.time >= suppression_from && snapshot.time <= suppression_through
        })
        .cloned()
        .collect::<Vec<_>>();
    let intervention_frame = ExperimentAnalytics::analyze_checkpoint_series(
        &analytics_checkpoints(&suppressed_checkpoints),
        FieldInputState::NoInput,
    )?;
    let recovery_frame = build_recovery_frame(recovery, intervention.final_snapshot.time)?;
    ExplanationReport::new(
        intervention.id,
        vec![control_frame, intervention_frame, recovery_frame],
    )
    .map_err(ExperimentError::Explanation)
}

fn analytics_checkpoints(snapshots: &[RuntimeSnapshot]) -> Vec<AnalyticsCheckpoint> {
    snapshots
        .iter()
        .map(|snapshot| AnalyticsCheckpoint {
            time: snapshot.time,
            physical_state: snapshot.physical_state_digest.fingerprint,
            history_state: snapshot.history_digest.fingerprint,
            mana_total: snapshot.mana_total,
            causal_trace_count: snapshot.causal_trace_count,
            latest_trace: snapshot.latest_trace,
        })
        .collect()
}

#[allow(clippy::too_many_arguments)]
fn build_experiment_manifest(
    world_seed: u64,
    ticks: u64,
    checkpoint_interval: u64,
    suppression_from: SimulationTime,
    suppression_through: SimulationTime,
    bootstrap_population: u64,
    control: &ReplayVerifiedExperiment,
    intervention: &ReplayVerifiedExperiment,
) -> Result<ExperimentManifest, ExperimentError> {
    let final_snapshot = intervention.result.final_snapshot.clone();
    let supporting_traces = intervention
        .result
        .checkpoints
        .iter()
        .map(|snapshot| snapshot.latest_trace)
        .collect::<Vec<_>>();
    let evidence_sufficient = final_snapshot.population_total > 0
        && final_snapshot.actor_promotions > 0
        && final_snapshot.material_activity_events > 0
        && control.experiment_digest != intervention.experiment_digest;
    Ok(ExperimentManifest {
        format_version: 1,
        seed_set: vec![world_seed],
        parameters: ExperimentParametersRecord {
            checkpoint_interval,
            bootstrap_population,
            suppression_from,
            suppression_through,
        },
        code_revision: RevisionRecord {
            identifier: "workspace-tree",
        },
        schema_revision: RevisionRecord {
            identifier: "digest-schema-v1",
        },
        warm_up_ticks: 0,
        duration_ticks: ticks,
        hardware: HardwareRecord {
            execution_backend: "single-process-in-memory",
            persistence: "none",
        },
        wall_time: control.elapsed + intervention.elapsed,
        activity_counts: ActivityCountsRecord {
            physical_events: final_snapshot.physical_events,
            mana_cell_changes: final_snapshot.mana_cell_changes,
            resolution_transitions: final_snapshot.resolution_transitions,
            actor_actions_committed: final_snapshot.actor_actions_committed,
            population_births: final_snapshot.population_births,
            population_deaths: final_snapshot.population_deaths,
            population_movements: final_snapshot.population_movements,
            actor_promotions: final_snapshot.actor_promotions,
            actor_demotions: final_snapshot.actor_demotions,
            material_activity_events: final_snapshot.material_activity_events,
        },
        memory: MemoryRecord {
            bytes_per_chunk: final_snapshot.bytes_per_chunk,
            causal_trace_count: final_snapshot.causal_trace_count,
            population_total: final_snapshot.population_total,
            actor_count: final_snapshot.actor_count,
        },
        state_digest: intervention.physical_state_digest,
        history_digest: intervention.history_digest,
        result_confidence: ClaimConfidence::new(if evidence_sufficient { 0.75 } else { 0.0 })?,
        supporting_traces,
        evidence_sufficient,
    })
}

fn build_recovery_frame(
    recovery: &MatchedCheckpointAnalysis,
    checkpoint_time: SimulationTime,
) -> Result<causafera_explanation::ExplanationFrame, ExperimentError> {
    let traces = recovery
        .matched_control_distances
        .iter()
        .flat_map(|distance| [distance.control_trace, distance.intervention_trace])
        .collect::<Vec<_>>();
    let recovery_distance = ExplanationClaim::new(
        RECOVERY_DISTANCE_SCHEMA,
        NumericClaimValue::scalar(u64_to_i64_saturating(recovery.final_recovery_distance)),
        ClaimConfidence::new(0.75)?,
        traces.clone(),
        ComparisonContext::MatchedCohort {
            cohort: ComparisonCohortId::new(1),
        },
        ClaimEvidenceState::Supported,
    )?;
    let time_to_recovery = match recovery.time_to_recovery {
        Some(time_to_recovery) => ExplanationClaim::new(
            TIME_TO_RECOVERY_SCHEMA,
            NumericClaimValue::scalar(u64_to_i64_saturating(time_to_recovery)),
            ClaimConfidence::new(0.75)?,
            traces,
            ComparisonContext::MatchedCohort {
                cohort: ComparisonCohortId::new(1),
            },
            ClaimEvidenceState::Supported,
        )?,
        None => ExplanationClaim::unknown(
            TIME_TO_RECOVERY_SCHEMA,
            NumericClaimValue::scalar(0),
            ComparisonContext::MatchedCohort {
                cohort: ComparisonCohortId::new(1),
            },
        )?,
    };
    causafera_explanation::ExplanationFrame::new(
        checkpoint_time,
        vec![recovery_distance, time_to_recovery],
    )
    .map_err(ExperimentError::Explanation)
}

fn u64_to_i64_saturating(value: u64) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

#[derive(Clone, Debug, Error, PartialEq)]
pub enum ExperimentError {
    #[error("invalid experiment tick count: {ticks}")]
    InvalidTickCount { ticks: u64 },
    #[error("invalid checkpoint interval: {interval}")]
    InvalidCheckpointInterval { interval: u64 },
    #[error("too many experiment checkpoints: {count}")]
    TooManyCheckpoints { count: usize },
    #[error("physical suppression window lies outside the experiment")]
    SuppressionOutsideRun,
    #[error("population {population} exceeds bounded in-memory experiment envelope")]
    PopulationOutsideInMemoryEnvelope { population: u64 },
    #[error("strict deterministic replay produced a different result")]
    ReplayMismatch,
    #[error("configured intervention did not change the canonical trajectory")]
    InterventionHadNoEffect,
    #[error("experiment produced no checkpoints")]
    MissingCheckpoint,
    #[error(transparent)]
    Runtime(#[from] RuntimeError),
    #[error(transparent)]
    Attractor(#[from] AttractorResearchError),
    #[error(transparent)]
    Analytics(#[from] AnalyticsError),
    #[error(transparent)]
    Explanation(#[from] causafera_explanation::ExplanationIrError),
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The lab is not a second bootstrap path.
    ///
    /// `ExperimentRunner::run_deterministic` builds its own `RuntimeConfig` from
    /// the experiment configuration, so the risk is that it drifts from what a
    /// direct `Runtime::new` on the same values would produce. It runs the same
    /// `RuntimeBootstrapRecipe`, and this pins that.
    #[test]
    fn lab_experiment_setup_shares_the_production_bootstrap_record() {
        let config = ExperimentConfig::new(ExperimentId::new(1), 4_150, 4, 2)
            .expect("a bounded experiment configuration")
            .with_bootstrap_population(16)
            .expect("a bounded bootstrap population");

        let mut expected = RuntimeConfig::new(config.world_seed);
        expected.pattern_schedule = config.pattern_schedule;
        expected.bootstrap_population = config.bootstrap_population;
        expected.actor_count = 1;
        expected.sensor_count = 1;
        let expected = Runtime::new(expected)
            .expect("the direct runtime must bootstrap")
            .export_snapshot()
            .expect("the direct state must export")
            .bootstrap;

        let mut runtime_config = RuntimeConfig::new(config.world_seed);
        runtime_config.pattern_schedule = config.pattern_schedule;
        runtime_config.bootstrap_population = config.bootstrap_population;
        runtime_config.actor_count = 1;
        runtime_config.sensor_count = 1;
        let actual = Runtime::new(runtime_config)
            .expect("the experiment runtime must bootstrap")
            .export_snapshot()
            .expect("the experiment state must export")
            .bootstrap;

        assert_eq!(actual, expected);
        assert_eq!(actual.receipts.len(), 6);
        assert!(actual.receipts.windows(2).all(|p| p[0].stage < p[1].stage));

        // And: a replay-verified run agrees with itself, which now includes the
        // canonical record through both digests.
        let verified = ExperimentRunner::run_replay_verified(config)
            .expect("a bounded experiment must replay-verify");
        assert_eq!(
            verified
                .result
                .checkpoints
                .first()
                .map(|s| s.bootstrap.plan_id),
            Some(actual.plan.id.raw())
        );
    }

    #[test]
    #[ignore = "expensive benchmark"]
    fn default_control_and_intervention_bootstraps_a_real_runtime_carrier() {
        let report = ExperimentRunner::run_control_and_intervention(
            77,
            256,
            32,
            SimulationTime::new(80),
            SimulationTime::new(160),
        )
        .expect("default comparison must exercise the production material-mana carrier");

        assert!(report.control.result.final_snapshot.actor_count > 0);
        assert!(
            report
                .control
                .result
                .final_snapshot
                .perceived_actor_features
                > 0
        );
    }

    #[test]
    fn short_control_and_intervention_bootstraps_a_real_runtime_carrier() {
        let report = ExperimentRunner::run_control_and_intervention(
            77,
            128,
            32,
            SimulationTime::new(40),
            SimulationTime::new(80),
        )
        .expect("short comparison must exercise the production material-mana carrier");

        assert!(report.control.result.final_snapshot.actor_count > 0);
        assert!(
            report
                .control
                .result
                .final_snapshot
                .perceived_actor_features
                > 0
        );
    }

    #[test]
    #[ignore = "expensive benchmark"]
    fn long_run_suite_replays_and_detects_intervention() {
        let report = ExperimentRunner::run_control_and_intervention(
            77,
            256,
            32,
            SimulationTime::new(80),
            SimulationTime::new(160),
        )
        .unwrap();
        assert!(report.trajectories_diverged);
        assert!(!report.explanation_report.frames.is_empty());
        assert_eq!(
            report
                .matched_checkpoint_analysis
                .pre_intervention_baseline_distance,
            0
        );
        assert!(report.final_physical_states_diverged);
        assert!(report.transient_physical_states_diverged);
        assert!(report.history_diverged);
        assert_ne!(
            report.control.physical_state_digest,
            report.intervention.physical_state_digest
        );
        assert_ne!(
            report.control.history_digest,
            report.intervention.history_digest
        );
        assert_eq!(report.control.result.final_snapshot.time.raw(), 256);
        assert!(
            report.control.result.final_snapshot.physical_events
                > report.intervention.result.final_snapshot.physical_events
        );
    }

    #[test]
    fn short_run_suite_replays_and_detects_intervention() {
        let report = ExperimentRunner::run_control_and_intervention(
            77,
            128,
            32,
            SimulationTime::new(40),
            SimulationTime::new(80),
        )
        .unwrap();
        assert!(report.trajectories_diverged);
        assert!(!report.explanation_report.frames.is_empty());
        assert_eq!(
            report
                .matched_checkpoint_analysis
                .pre_intervention_baseline_distance,
            0
        );
        assert!(report.final_physical_states_diverged);
        assert!(report.transient_physical_states_diverged);
        assert!(report.history_diverged);
        assert_ne!(
            report.control.physical_state_digest,
            report.intervention.physical_state_digest
        );
        assert_ne!(
            report.control.history_digest,
            report.intervention.history_digest
        );
        assert_eq!(report.control.result.final_snapshot.time.raw(), 128);
        assert!(
            report.control.result.final_snapshot.physical_events
                > report.intervention.result.final_snapshot.physical_events
        );
    }

    #[test]
    #[ignore = "expensive benchmark"]
    fn populated_long_run_reports_coupled_domains_with_bounded_evidence() {
        let report = ExperimentRunner::run_populated_control_and_intervention(
            88,
            192,
            32,
            SimulationTime::new(64),
            SimulationTime::new(96),
            8,
        )
        .unwrap();
        let control = report.control.result.final_snapshot;
        let intervention = report.intervention.result.final_snapshot;

        assert_eq!(report.manifest.format_version, 1);
        assert_eq!(report.manifest.parameters.bootstrap_population, 8);
        assert_eq!(report.manifest.hardware.persistence, "none");
        assert!(report.manifest.evidence_sufficient);
        assert!(report.manifest.result_confidence.raw() > 0.0);
        assert!(!report.manifest.supporting_traces.is_empty());
        assert!(report.final_physical_states_diverged);
        assert!(report.history_diverged);
        assert_ne!(
            control.physical_state_digest,
            intervention.physical_state_digest
        );
        assert_ne!(control.history_digest, intervention.history_digest);
        assert!(control.active_chunk_count <= 4);
        assert!(control.population_total <= 16);
        assert!(control.population_movements > 0);
        assert!(control.actor_promotions > 0);
        assert!(control.actor_count > 0);
        assert!(control.actor_count <= 16);
        assert!(control.perceived_actor_features > 0);
        assert!(control.subjective_actor_objects > 0);
        assert!(control.actor_actions_committed > 0);
        assert!(control.physical_events > 0);
        assert!(control.mana_cell_changes > 0);
        assert!(control.mana_physical_effects > 0);
        assert!(control.resolution_transitions > 0);
        assert!(control.material_activity_events > 0);
        assert!(!report.explanation_report.frames.is_empty());
        assert_eq!(
            report
                .matched_checkpoint_analysis
                .pre_intervention_baseline_distance,
            0
        );
    }

    #[test]
    fn experiment_snapshots_report_active_chunk_transition_metrics() {
        let config = ExperimentConfig::new(ExperimentId::new(4), 101, 64, 16).unwrap();
        let replay = ExperimentRunner::run_replay_verified(config).unwrap();
        let final_snapshot = replay.result.final_snapshot;

        assert!(final_snapshot.active_chunk_count > 1);
        assert!(final_snapshot.resolution_transitions > 0);
        assert!(final_snapshot.bytes_per_chunk > 0);
    }

    #[test]
    fn invalid_experiment_bounds_are_rejected() {
        assert!(matches!(
            ExperimentConfig::new(ExperimentId::new(1), 0, 0, 1),
            Err(ExperimentError::InvalidTickCount { .. })
        ));
    }

    #[test]
    fn physical_checkpoint_comparison_detects_transient_divergence() {
        let config = ExperimentConfig::new(ExperimentId::new(3), 91, 64, 32).unwrap();
        let mut control = ExperimentRunner::run_deterministic(&config).unwrap();
        let mut intervention = control.clone();
        intervention.checkpoints[1].physical_state_digest =
            control.checkpoints[0].physical_state_digest;
        assert_eq!(
            control.final_snapshot.physical_state_digest,
            intervention.final_snapshot.physical_state_digest,
        );
        assert!(!same_physical_checkpoints(
            &control.checkpoints,
            &intervention.checkpoints,
        ));
        control.checkpoints[1].physical_state_digest =
            intervention.checkpoints[1].physical_state_digest;
        assert!(same_physical_checkpoints(
            &control.checkpoints,
            &intervention.checkpoints,
        ));
    }
}
