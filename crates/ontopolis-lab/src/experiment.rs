use std::time::{Duration, Instant};

use ontopolis_metaphysics::{
    AttractorEvidence, AttractorProbe, AttractorResearchError, FieldTrajectoryObservation,
};
use ontopolis_runtime::{
    MAX_RUNTIME_TICKS, PhysicalPatternSchedule, Runtime, RuntimeConfig, RuntimeError,
    RuntimeSnapshot,
};
use ontopolis_types::{
    AttractorExperimentId, AttractorProbeSchemaId, ExperimentId, SimulationTime,
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
        if let Some(through) = self.pattern_schedule.suppressed_through {
            if through.raw() > self.ticks {
                return Err(ExperimentError::SuppressionOutsideRun);
            }
        }
        Ok(())
    }
}

/// Deterministic portion of an experiment result.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExperimentResult {
    pub id: ExperimentId,
    pub world_seed: u64,
    pub ticks: u64,
    pub checkpoints: Vec<RuntimeSnapshot>,
    pub final_snapshot: RuntimeSnapshot,
    pub attractor_evidence: AttractorEvidence,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReplayVerifiedExperiment {
    pub result: ExperimentResult,
    /// Report-only measurement; excluded from deterministic result equality.
    pub elapsed: Duration,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LongRunExperimentReport {
    pub control: ReplayVerifiedExperiment,
    pub intervention: ReplayVerifiedExperiment,
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
        if first != replay {
            return Err(ExperimentError::ReplayMismatch);
        }
        Ok(ReplayVerifiedExperiment {
            result: first,
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
        let control =
            ExperimentConfig::new(ExperimentId::new(1), world_seed, ticks, checkpoint_interval)?;
        let intervention =
            ExperimentConfig::new(ExperimentId::new(2), world_seed, ticks, checkpoint_interval)?
                .with_suppression(suppression_from, suppression_through)?;
        let control = Self::run_replay_verified(control)?;
        let intervention = Self::run_replay_verified(intervention)?;
        let trajectories_diverged = control.result.final_snapshot.canonical_state
            != intervention.result.final_snapshot.canonical_state;
        if !trajectories_diverged {
            return Err(ExperimentError::InterventionHadNoEffect);
        }
        Ok(LongRunExperimentReport {
            control,
            intervention,
            trajectories_diverged,
        })
    }

    fn run_deterministic(config: &ExperimentConfig) -> Result<ExperimentResult, ExperimentError> {
        let mut runtime_config = RuntimeConfig::new(config.world_seed);
        runtime_config.pattern_schedule = config.pattern_schedule;
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
        let final_snapshot = *checkpoints
            .last()
            .expect("validated experiments always retain an initial and final checkpoint");
        let observations = checkpoints
            .iter()
            .map(|snapshot| {
                FieldTrajectoryObservation::new(
                    snapshot.time,
                    snapshot.canonical_state,
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
            final_snapshot,
            attractor_evidence,
        })
    }
}

#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum ExperimentError {
    #[error("invalid experiment tick count: {ticks}")]
    InvalidTickCount { ticks: u64 },
    #[error("invalid checkpoint interval: {interval}")]
    InvalidCheckpointInterval { interval: u64 },
    #[error("too many experiment checkpoints: {count}")]
    TooManyCheckpoints { count: usize },
    #[error("physical suppression window lies outside the experiment")]
    SuppressionOutsideRun,
    #[error("strict deterministic replay produced a different result")]
    ReplayMismatch,
    #[error("configured intervention did not change the canonical trajectory")]
    InterventionHadNoEffect,
    #[error(transparent)]
    Runtime(#[from] RuntimeError),
    #[error(transparent)]
    Attractor(#[from] AttractorResearchError),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
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
        assert_eq!(report.control.result.final_snapshot.time.raw(), 256);
        assert!(
            report.control.result.final_snapshot.physical_events
                > report.intervention.result.final_snapshot.physical_events
        );
    }

    #[test]
    fn invalid_experiment_bounds_are_rejected() {
        assert!(matches!(
            ExperimentConfig::new(ExperimentId::new(1), 0, 0, 1),
            Err(ExperimentError::InvalidTickCount { .. })
        ));
    }
}
