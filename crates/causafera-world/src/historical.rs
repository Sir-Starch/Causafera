use causafera_core::StateFingerprint;
use causafera_types::{
    ChunkId, HistoricalBootstrapId, HistoricalProcessSchemaId, HistoricalStageId, SimulationTime,
    TraceId,
};
use thiserror::Error;

pub const MAX_HISTORICAL_STAGES: usize = 64;
pub const MAX_STAGE_TARGETS: usize = 256;
pub const MAX_STAGE_DEPENDENCIES: usize = 32;
pub const MAX_STAGE_EXTERNAL_CAUSES: usize = 64;

/// One bounded causal-synthesis step. Process identity is an opaque schema ID,
/// never a named high-level event such as a war, plague, or discovery.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HistoricalStage {
    id: HistoricalStageId,
    process: HistoricalProcessSchemaId,
    starts_at: SimulationTime,
    ends_at: SimulationTime,
    detail_ordinal: u8,
    targets: Vec<ChunkId>,
    dependencies: Vec<HistoricalStageId>,
    external_causes: Vec<TraceId>,
    parameters: StateFingerprint,
}

impl HistoricalStage {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: HistoricalStageId,
        process: HistoricalProcessSchemaId,
        starts_at: SimulationTime,
        ends_at: SimulationTime,
        detail_ordinal: u8,
        targets: Vec<ChunkId>,
        dependencies: Vec<HistoricalStageId>,
        external_causes: Vec<TraceId>,
        parameters: StateFingerprint,
    ) -> Result<Self, HistoricalBootstrapError> {
        if starts_at >= ends_at {
            return Err(HistoricalBootstrapError::EmptyTimeSpan { stage: id });
        }
        validate_sorted_nonempty(&targets, MAX_STAGE_TARGETS, id, "targets")?;
        validate_sorted(&dependencies, MAX_STAGE_DEPENDENCIES, id, "dependencies")?;
        validate_sorted(
            &external_causes,
            MAX_STAGE_EXTERNAL_CAUSES,
            id,
            "external causes",
        )?;
        Ok(Self {
            id,
            process,
            starts_at,
            ends_at,
            detail_ordinal,
            targets,
            dependencies,
            external_causes,
            parameters,
        })
    }

    pub const fn id(&self) -> HistoricalStageId {
        self.id
    }
    pub const fn process(&self) -> HistoricalProcessSchemaId {
        self.process
    }
    pub const fn starts_at(&self) -> SimulationTime {
        self.starts_at
    }
    pub const fn ends_at(&self) -> SimulationTime {
        self.ends_at
    }
    pub const fn detail_ordinal(&self) -> u8 {
        self.detail_ordinal
    }
    pub fn targets(&self) -> &[ChunkId] {
        &self.targets
    }
    pub fn dependencies(&self) -> &[HistoricalStageId] {
        &self.dependencies
    }
    pub fn external_causes(&self) -> &[TraceId] {
        &self.external_causes
    }
    pub const fn parameters(&self) -> StateFingerprint {
        self.parameters
    }
}

/// Canonical plan for low- or high-resolution historical synthesis.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HistoricalBootstrapPlan {
    id: HistoricalBootstrapId,
    world_seed: u64,
    stages: Vec<HistoricalStage>,
}

impl HistoricalBootstrapPlan {
    pub fn new(
        id: HistoricalBootstrapId,
        world_seed: u64,
        mut stages: Vec<HistoricalStage>,
    ) -> Result<Self, HistoricalBootstrapError> {
        if stages.is_empty() || stages.len() > MAX_HISTORICAL_STAGES {
            return Err(HistoricalBootstrapError::InvalidStageCount(stages.len()));
        }
        stages.sort_by_key(HistoricalStage::id);
        for window in stages.windows(2) {
            if window[0].id == window[1].id {
                return Err(HistoricalBootstrapError::DuplicateStage(window[0].id));
            }
        }
        for (index, stage) in stages.iter().enumerate() {
            for dependency in &stage.dependencies {
                let Some(parent) = stages[..index]
                    .iter()
                    .find(|candidate| candidate.id == *dependency)
                else {
                    return Err(HistoricalBootstrapError::UnknownOrForwardDependency {
                        stage: stage.id,
                        dependency: *dependency,
                    });
                };
                if parent.ends_at > stage.starts_at {
                    return Err(HistoricalBootstrapError::OverlappingDependency {
                        stage: stage.id,
                        dependency: *dependency,
                    });
                }
            }
        }
        Ok(Self {
            id,
            world_seed,
            stages,
        })
    }

    pub const fn id(&self) -> HistoricalBootstrapId {
        self.id
    }
    pub const fn world_seed(&self) -> u64 {
        self.world_seed
    }
    pub fn stages(&self) -> &[HistoricalStage] {
        &self.stages
    }

    /// Domain-separated deterministic seed contribution for a stage adapter.
    pub fn stage_seed(&self, stage: HistoricalStageId) -> Option<u64> {
        let stage = self.stages.iter().find(|candidate| candidate.id == stage)?;
        Some(mix64(
            self.world_seed
                ^ self.id.raw().rotate_left(7)
                ^ stage.id.raw().rotate_left(19)
                ^ stage.process.raw().rotate_left(31)
                ^ stage.starts_at.raw().rotate_left(43),
        ))
    }

    pub fn validate_receipts(
        &self,
        mut receipts: Vec<HistoricalStageReceipt>,
    ) -> Result<HistoricalBootstrapRecord, HistoricalBootstrapError> {
        receipts.sort_by_key(HistoricalStageReceipt::stage);
        if receipts.len() != self.stages.len() {
            return Err(HistoricalBootstrapError::IncompleteReceipts);
        }
        for (index, (stage, receipt)) in self.stages.iter().zip(&receipts).enumerate() {
            if receipt.stage != stage.id || receipt.completed_at != stage.ends_at {
                return Err(HistoricalBootstrapError::ReceiptMismatch { stage: stage.id });
            }
            let mut required = stage.external_causes.clone();
            for dependency in &stage.dependencies {
                let trace = receipts[..index]
                    .iter()
                    .find(|candidate| candidate.stage == *dependency)
                    .map(|candidate| candidate.trace)
                    .expect("validated plan dependencies precede their consumers");
                required.push(trace);
            }
            required.sort_unstable();
            required.dedup();
            if receipt.causes != required {
                return Err(HistoricalBootstrapError::ReceiptCauseMismatch { stage: stage.id });
            }
        }
        let mut traces: Vec<_> = receipts.iter().map(|receipt| receipt.trace).collect();
        traces.sort_unstable();
        if traces.windows(2).any(|window| window[0] == window[1]) {
            return Err(HistoricalBootstrapError::DuplicateReceiptTrace);
        }
        Ok(HistoricalBootstrapRecord {
            plan: self.clone(),
            receipts,
        })
    }
}

/// Evidence that a stage adapter committed its state changes through the
/// authoritative provenance boundary.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HistoricalStageReceipt {
    stage: HistoricalStageId,
    completed_at: SimulationTime,
    result: StateFingerprint,
    trace: TraceId,
    causes: Vec<TraceId>,
}

impl HistoricalStageReceipt {
    pub fn new(
        stage: HistoricalStageId,
        completed_at: SimulationTime,
        result: StateFingerprint,
        trace: TraceId,
        causes: Vec<TraceId>,
    ) -> Result<Self, HistoricalBootstrapError> {
        validate_sorted(
            &causes,
            MAX_STAGE_EXTERNAL_CAUSES + MAX_STAGE_DEPENDENCIES,
            stage,
            "receipt causes",
        )?;
        Ok(Self {
            stage,
            completed_at,
            result,
            trace,
            causes,
        })
    }

    pub const fn stage(&self) -> HistoricalStageId {
        self.stage
    }
    pub const fn completed_at(&self) -> SimulationTime {
        self.completed_at
    }
    pub const fn result(&self) -> StateFingerprint {
        self.result
    }
    pub const fn trace(&self) -> TraceId {
        self.trace
    }
    pub fn causes(&self) -> &[TraceId] {
        &self.causes
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HistoricalBootstrapRecord {
    plan: HistoricalBootstrapPlan,
    receipts: Vec<HistoricalStageReceipt>,
}

impl HistoricalBootstrapRecord {
    pub const fn plan(&self) -> &HistoricalBootstrapPlan {
        &self.plan
    }
    pub fn receipts(&self) -> &[HistoricalStageReceipt] {
        &self.receipts
    }
}

#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum HistoricalBootstrapError {
    #[error("historical bootstrap requires 1..={MAX_HISTORICAL_STAGES} stages, got {0}")]
    InvalidStageCount(usize),
    #[error("historical stage {stage} has an empty or reversed time span")]
    EmptyTimeSpan { stage: HistoricalStageId },
    #[error("historical stage {stage} has invalid {field} count {count}")]
    InvalidFieldCount {
        stage: HistoricalStageId,
        field: &'static str,
        count: usize,
    },
    #[error("historical stage {stage} has non-canonical {field}")]
    NonCanonicalField {
        stage: HistoricalStageId,
        field: &'static str,
    },
    #[error("duplicate historical stage {0}")]
    DuplicateStage(HistoricalStageId),
    #[error("stage {stage} references unknown or forward dependency {dependency}")]
    UnknownOrForwardDependency {
        stage: HistoricalStageId,
        dependency: HistoricalStageId,
    },
    #[error("stage {stage} overlaps dependency {dependency}")]
    OverlappingDependency {
        stage: HistoricalStageId,
        dependency: HistoricalStageId,
    },
    #[error("historical bootstrap receipts are incomplete")]
    IncompleteReceipts,
    #[error("receipt does not match stage {stage}")]
    ReceiptMismatch { stage: HistoricalStageId },
    #[error("receipt causes do not match declared ancestry for stage {stage}")]
    ReceiptCauseMismatch { stage: HistoricalStageId },
    #[error("historical bootstrap receipts reuse a trace")]
    DuplicateReceiptTrace,
}

fn validate_sorted<T: Ord>(
    values: &[T],
    maximum: usize,
    stage: HistoricalStageId,
    field: &'static str,
) -> Result<(), HistoricalBootstrapError> {
    if values.len() > maximum {
        return Err(HistoricalBootstrapError::InvalidFieldCount {
            stage,
            field,
            count: values.len(),
        });
    }
    if values.windows(2).any(|window| window[0] >= window[1]) {
        return Err(HistoricalBootstrapError::NonCanonicalField { stage, field });
    }
    Ok(())
}

fn validate_sorted_nonempty<T: Ord>(
    values: &[T],
    maximum: usize,
    stage: HistoricalStageId,
    field: &'static str,
) -> Result<(), HistoricalBootstrapError> {
    if values.is_empty() {
        return Err(HistoricalBootstrapError::InvalidFieldCount {
            stage,
            field,
            count: 0,
        });
    }
    validate_sorted(values, maximum, stage, field)
}

const fn mix64(mut value: u64) -> u64 {
    value ^= value >> 30;
    value = value.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value ^= value >> 27;
    value = value.wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fingerprint(value: u8) -> StateFingerprint {
        StateFingerprint::new([value; 32])
    }

    fn stage(
        id: u64,
        starts: u64,
        ends: u64,
        dependencies: Vec<HistoricalStageId>,
    ) -> HistoricalStage {
        HistoricalStage::new(
            HistoricalStageId::new(id),
            HistoricalProcessSchemaId::new(100 + id),
            SimulationTime::new(starts),
            SimulationTime::new(ends),
            id as u8,
            vec![ChunkId::new(1)],
            dependencies,
            vec![],
            fingerprint(id as u8),
        )
        .unwrap()
    }

    #[test]
    fn plan_is_canonical_and_stage_seeds_are_order_independent() {
        let early = stage(1, 0, 10, vec![]);
        let recent = stage(2, 10, 20, vec![HistoricalStageId::new(1)]);
        let plan =
            HistoricalBootstrapPlan::new(HistoricalBootstrapId::new(7), 55, vec![recent, early])
                .unwrap();
        assert_eq!(plan.stages()[0].id(), HistoricalStageId::new(1));
        assert_eq!(
            plan.stage_seed(HistoricalStageId::new(2)),
            plan.stage_seed(HistoricalStageId::new(2))
        );
    }

    #[test]
    fn receipts_must_continue_declared_causal_ancestry() {
        let plan = HistoricalBootstrapPlan::new(
            HistoricalBootstrapId::new(1),
            9,
            vec![
                stage(1, 0, 10, vec![]),
                stage(2, 10, 20, vec![HistoricalStageId::new(1)]),
            ],
        )
        .unwrap();
        let first = HistoricalStageReceipt::new(
            HistoricalStageId::new(1),
            SimulationTime::new(10),
            fingerprint(3),
            TraceId::new(40),
            vec![],
        )
        .unwrap();
        let second = HistoricalStageReceipt::new(
            HistoricalStageId::new(2),
            SimulationTime::new(20),
            fingerprint(4),
            TraceId::new(41),
            vec![TraceId::new(40)],
        )
        .unwrap();
        let record = plan.validate_receipts(vec![second, first]).unwrap();
        assert_eq!(record.receipts().len(), 2);
    }

    /// RFC-HIST-001 derives the seed contribution from the world seed, the plan
    /// identity, the stage identity, the process schema and the start time.
    /// Each of those must move it on its own, or the "domain-separated" in the
    /// contract is only a description of the code rather than a property of it:
    /// a derivation over plan ID and stage ID alone satisfies every
    /// coarser-grained test, because the plan ID already hashes the world seed.
    #[test]
    fn every_declared_input_moves_the_stage_seed_on_its_own() {
        let build = |world_seed: u64, plan: u64, process: u64, starts: u64| {
            HistoricalBootstrapPlan::new(
                HistoricalBootstrapId::new(plan),
                world_seed,
                vec![
                    HistoricalStage::new(
                        HistoricalStageId::new(1),
                        HistoricalProcessSchemaId::new(process),
                        SimulationTime::new(starts),
                        SimulationTime::new(starts + 10),
                        0,
                        vec![ChunkId::new(1)],
                        vec![],
                        vec![],
                        fingerprint(1),
                    )
                    .unwrap(),
                ],
            )
            .unwrap()
            .stage_seed(HistoricalStageId::new(1))
            .unwrap()
        };

        let baseline = build(5, 7, 100, 0);
        assert_ne!(
            baseline,
            build(6, 7, 100, 0),
            "world seed must move the seed"
        );
        assert_ne!(baseline, build(5, 8, 100, 0), "plan identity must move it");
        assert_ne!(baseline, build(5, 7, 101, 0), "process schema must move it");
        assert_ne!(baseline, build(5, 7, 100, 1), "start time must move it");
        assert_eq!(baseline, build(5, 7, 100, 0), "and it must be stable");
    }

    /// Two stages of one plan differ only in stage identity, so this pins that
    /// input separately from the four above.
    #[test]
    fn stages_of_one_plan_do_not_share_a_seed() {
        let plan = HistoricalBootstrapPlan::new(
            HistoricalBootstrapId::new(3),
            11,
            vec![
                stage(1, 0, 10, vec![]),
                stage(2, 10, 20, vec![HistoricalStageId::new(1)]),
            ],
        )
        .unwrap();
        assert_ne!(
            plan.stage_seed(HistoricalStageId::new(1)),
            plan.stage_seed(HistoricalStageId::new(2))
        );
    }

    #[test]
    fn semantic_high_level_outcomes_cannot_enter_the_contract() {
        let stage = stage(1, 0, 10, vec![]);
        assert_eq!(stage.process(), HistoricalProcessSchemaId::new(101));
        assert_eq!(stage.targets(), &[ChunkId::new(1)]);
    }
}
