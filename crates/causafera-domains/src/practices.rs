use causafera_types::{
    ActionPatternId, PracticeConditionId, PracticeExecutionId, PracticeId, PracticeOperationId,
    SimulationTime,
};

pub const MAX_PRACTICE_INSTRUCTIONS: usize = 64;
pub const MAX_EXECUTION_STEPS: usize = 256;
pub const MAX_EXECUTION_EMISSIONS: usize = 128;
pub const MAX_CONDITION_EVIDENCE: usize = 32;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PracticeCondition {
    pub id: PracticeConditionId,
    pub threshold: i32,
    pub tolerance: u32,
}

impl PracticeCondition {
    fn accepts(self, evidence: i32) -> bool {
        evidence.saturating_add_unsigned(self.tolerance) >= self.threshold
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PracticeInstruction {
    Perform {
        operation: PracticeOperationId,
        action: ActionPatternId,
        duration_ticks: u32,
        tolerance: u32,
    },
    Branch {
        condition: PracticeCondition,
        when_met: u8,
        when_unmet: u8,
    },
    Wait {
        duration_ticks: u32,
    },
    Repeat {
        target: u8,
        count: u8,
    },
    Halt,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Practice {
    id: PracticeId,
    parent: Option<PracticeId>,
    instructions: Vec<PracticeInstruction>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PracticeError {
    Empty,
    TooManyInstructions,
    InvalidBranch,
    InvalidRepeat,
    ZeroDuration,
    DuplicateEvidence,
    TooMuchEvidence,
    StepBudgetExceeded,
    EmissionBudgetExceeded,
    MutationOutOfBounds,
    InvalidChild,
}

impl Practice {
    pub fn new(
        id: PracticeId,
        parent: Option<PracticeId>,
        instructions: Vec<PracticeInstruction>,
    ) -> Result<Self, PracticeError> {
        if instructions.is_empty() {
            return Err(PracticeError::Empty);
        }
        if instructions.len() > MAX_PRACTICE_INSTRUCTIONS {
            return Err(PracticeError::TooManyInstructions);
        }
        validate_instructions(&instructions)?;
        Ok(Self {
            id,
            parent,
            instructions,
        })
    }

    pub const fn id(&self) -> PracticeId {
        self.id
    }

    pub const fn parent(&self) -> Option<PracticeId> {
        self.parent
    }

    pub fn instructions(&self) -> &[PracticeInstruction] {
        &self.instructions
    }

    pub fn mutate(
        &self,
        child: PracticeId,
        instruction_index: usize,
        replacement: PracticeInstruction,
    ) -> Result<Self, PracticeError> {
        if child == self.id {
            return Err(PracticeError::InvalidChild);
        }
        let Some(slot) = self.instructions.get(instruction_index) else {
            return Err(PracticeError::MutationOutOfBounds);
        };
        if *slot == replacement {
            return Err(PracticeError::MutationOutOfBounds);
        }
        let mut instructions = self.instructions.clone();
        instructions[instruction_index] = replacement;
        Self::new(child, Some(self.id), instructions)
    }

    pub fn execute(
        &self,
        execution: PracticeExecutionId,
        started_at: SimulationTime,
        evidence: &[ConditionEvidence],
    ) -> Result<PracticeExecution, PracticeError> {
        validate_evidence(evidence)?;
        let mut pc = 0usize;
        let mut elapsed = 0u64;
        let mut visited = [0u8; MAX_PRACTICE_INSTRUCTIONS];
        let mut emissions = Vec::new();

        for _ in 0..MAX_EXECUTION_STEPS {
            let instruction = self
                .instructions
                .get(pc)
                .copied()
                .unwrap_or(PracticeInstruction::Halt);
            match instruction {
                PracticeInstruction::Perform {
                    operation,
                    action,
                    duration_ticks,
                    tolerance,
                } => {
                    if emissions.len() == MAX_EXECUTION_EMISSIONS {
                        return Err(PracticeError::EmissionBudgetExceeded);
                    }
                    emissions.push(PracticeEmission {
                        operation,
                        action,
                        offset_ticks: elapsed,
                        duration_ticks,
                        tolerance,
                    });
                    elapsed = elapsed.saturating_add(u64::from(duration_ticks));
                    pc += 1;
                }
                PracticeInstruction::Wait { duration_ticks } => {
                    elapsed = elapsed.saturating_add(u64::from(duration_ticks));
                    pc += 1;
                }
                PracticeInstruction::Branch {
                    condition,
                    when_met,
                    when_unmet,
                } => {
                    let value = evidence
                        .binary_search_by_key(&condition.id, |item| item.condition)
                        .ok()
                        .map_or(i32::MIN, |index| evidence[index].value);
                    pc = usize::from(if condition.accepts(value) {
                        when_met
                    } else {
                        when_unmet
                    });
                }
                PracticeInstruction::Repeat { target, count } => {
                    if visited[pc] < count {
                        visited[pc] += 1;
                        pc = usize::from(target);
                    } else {
                        pc += 1;
                    }
                }
                PracticeInstruction::Halt => {
                    return Ok(PracticeExecution {
                        id: execution,
                        practice: self.id,
                        started_at,
                        elapsed_ticks: elapsed,
                        emissions,
                    });
                }
            }
        }
        Err(PracticeError::StepBudgetExceeded)
    }
}

fn validate_instructions(instructions: &[PracticeInstruction]) -> Result<(), PracticeError> {
    for (index, instruction) in instructions.iter().enumerate() {
        match *instruction {
            PracticeInstruction::Perform {
                duration_ticks: 0, ..
            }
            | PracticeInstruction::Wait { duration_ticks: 0 } => {
                return Err(PracticeError::ZeroDuration);
            }
            PracticeInstruction::Branch {
                when_met,
                when_unmet,
                ..
            } if usize::from(when_met) >= instructions.len()
                || usize::from(when_unmet) >= instructions.len()
                || usize::from(when_met) <= index
                || usize::from(when_unmet) <= index =>
            {
                return Err(PracticeError::InvalidBranch);
            }
            PracticeInstruction::Repeat { target, count }
                if usize::from(target) >= index || count == 0 =>
            {
                return Err(PracticeError::InvalidRepeat);
            }
            _ => {}
        }
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ConditionEvidence {
    pub condition: PracticeConditionId,
    pub value: i32,
}

fn validate_evidence(evidence: &[ConditionEvidence]) -> Result<(), PracticeError> {
    if evidence.len() > MAX_CONDITION_EVIDENCE {
        return Err(PracticeError::TooMuchEvidence);
    }
    if evidence
        .windows(2)
        .any(|pair| pair[0].condition >= pair[1].condition)
    {
        return Err(PracticeError::DuplicateEvidence);
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PracticeEmission {
    pub operation: PracticeOperationId,
    pub action: ActionPatternId,
    pub offset_ticks: u64,
    pub duration_ticks: u32,
    pub tolerance: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PracticeExecution {
    pub id: PracticeExecutionId,
    pub practice: PracticeId,
    pub started_at: SimulationTime,
    pub elapsed_ticks: u64,
    pub emissions: Vec<PracticeEmission>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn practice() -> Practice {
        Practice::new(
            PracticeId::new(1),
            None,
            vec![
                PracticeInstruction::Branch {
                    condition: PracticeCondition {
                        id: PracticeConditionId::new(4),
                        threshold: 10,
                        tolerance: 1,
                    },
                    when_met: 1,
                    when_unmet: 2,
                },
                PracticeInstruction::Perform {
                    operation: PracticeOperationId::new(7),
                    action: ActionPatternId::new(8),
                    duration_ticks: 3,
                    tolerance: 2,
                },
                PracticeInstruction::Halt,
            ],
        )
        .unwrap()
    }

    #[test]
    fn execution_is_deterministic_and_proposal_only() {
        let evidence = [ConditionEvidence {
            condition: PracticeConditionId::new(4),
            value: 9,
        }];
        let first = practice()
            .execute(
                PracticeExecutionId::new(2),
                SimulationTime::new(5),
                &evidence,
            )
            .unwrap();
        let second = practice()
            .execute(
                PracticeExecutionId::new(2),
                SimulationTime::new(5),
                &evidence,
            )
            .unwrap();
        assert_eq!(first, second);
        assert_eq!(first.emissions.len(), 1);
    }

    #[test]
    fn mutation_creates_child_lineage() {
        let child = practice()
            .mutate(
                PracticeId::new(2),
                1,
                PracticeInstruction::Wait { duration_ticks: 4 },
            )
            .unwrap();
        assert_eq!(child.parent(), Some(PracticeId::new(1)));
        assert_eq!(child.id(), PracticeId::new(2));
    }

    #[test]
    fn invalid_control_flow_is_rejected() {
        assert_eq!(
            Practice::new(
                PracticeId::new(1),
                None,
                vec![PracticeInstruction::Repeat {
                    target: 0,
                    count: 1
                }]
            ),
            Err(PracticeError::InvalidRepeat)
        );
    }
}
