use std::collections::{BTreeMap, BTreeSet};

use ontopolis_types::{
    CHUNK_SIZE, ChunkCoord, LocalCoord, ManaFieldId, PhysicalPatternId, SimulationTime, TraceId,
};

pub const MANA_SCALE: i64 = 1_024;
pub const MAX_MANA_SAMPLES: usize = 4_096;
pub const MAX_MANA_CELLS: usize = CHUNK_SIZE as usize * CHUNK_SIZE as usize * CHUNK_SIZE as usize;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ManaParameters {
    pub base_response: u16,
    pub recurrence_response: u16,
    pub periodicity_response: u16,
    pub synchrony_response: u16,
    pub spatial_response: u16,
    pub diffusion: u16,
    pub decay: u16,
    pub maximum_intensity: i64,
}

impl ManaParameters {
    pub fn validate(self) -> Result<Self, ManaError> {
        let fractions = [
            self.base_response,
            self.recurrence_response,
            self.periodicity_response,
            self.synchrony_response,
            self.spatial_response,
            self.diffusion,
            self.decay,
        ];
        if fractions.iter().any(|value| i64::from(*value) > MANA_SCALE)
            || u32::from(self.diffusion) + u32::from(self.decay) > MANA_SCALE as u32
        {
            return Err(ManaError::InvalidParameters);
        }
        if self.maximum_intensity <= 0 {
            return Err(ManaError::InvalidParameters);
        }
        Ok(self)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct PhysicalPatternSample {
    pub pattern: PhysicalPatternId,
    pub position: LocalCoord,
    pub observed_at: SimulationTime,
    pub magnitude: u32,
    pub source_ordinal: u32,
    pub cause: TraceId,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ManaField {
    id: ManaFieldId,
    chunk: ChunkCoord,
    extent: u8,
    observed_through: SimulationTime,
    intensity: Vec<i64>,
    last_change: Vec<Option<TraceId>>,
}

impl ManaField {
    pub fn new(id: ManaFieldId, chunk: ChunkCoord, extent: u8) -> Result<Self, ManaError> {
        if extent == 0 || extent > CHUNK_SIZE {
            return Err(ManaError::InvalidExtent);
        }
        let volume = usize::from(extent).pow(3);
        Ok(Self {
            id,
            chunk,
            extent,
            observed_through: SimulationTime::default(),
            intensity: vec![0; volume],
            last_change: vec![None; volume],
        })
    }

    pub const fn id(&self) -> ManaFieldId {
        self.id
    }

    pub const fn chunk(&self) -> ChunkCoord {
        self.chunk
    }

    pub const fn extent(&self) -> u8 {
        self.extent
    }

    pub const fn observed_through(&self) -> SimulationTime {
        self.observed_through
    }

    pub fn intensity(&self) -> &[i64] {
        &self.intensity
    }

    pub fn intensity_at(&self, position: LocalCoord) -> Option<i64> {
        self.index(position).map(|index| self.intensity[index])
    }

    pub fn propose_evolution(
        &self,
        through: SimulationTime,
        parameters: ManaParameters,
        samples: &[PhysicalPatternSample],
    ) -> Result<ManaEvolutionProposal, ManaError> {
        let parameters = parameters.validate()?;
        if through <= self.observed_through {
            return Err(ManaError::NonMonotonicTime);
        }
        if samples.len() > MAX_MANA_SAMPLES {
            return Err(ManaError::TooManySamples);
        }

        let mut canonical = samples.to_vec();
        canonical.sort_unstable();
        if canonical.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err(ManaError::DuplicateSample);
        }
        for sample in &canonical {
            if sample.observed_at <= self.observed_through || sample.observed_at > through {
                return Err(ManaError::SampleOutsideWindow);
            }
            if self.index(sample.position).is_none() {
                return Err(ManaError::PositionOutsideField);
            }
        }

        let groups = group_samples(&canonical);
        let mut injected = self.intensity.clone();
        let mut direct_causes = vec![Vec::new(); injected.len()];
        for group in groups.values() {
            let score = pattern_score(group, parameters);
            let group_causes: Vec<_> = group.iter().map(|sample| sample.cause).collect();
            for sample in group {
                let index = self.index(sample.position).expect("samples were validated");
                let response = i64::from(sample.magnitude)
                    .saturating_mul(score)
                    .saturating_div(MANA_SCALE);
                injected[index] = injected[index]
                    .saturating_add(response)
                    .min(parameters.maximum_intensity);
                direct_causes[index].extend_from_slice(&group_causes);
            }
        }

        let mut proposed = vec![0; injected.len()];
        let mut changes = Vec::new();
        for (index, proposed_value) in proposed.iter_mut().enumerate() {
            let value = diffuse_cell(index, self.extent, &injected, parameters);
            *proposed_value = value;
            if value != self.intensity[index] {
                let mut causes = BTreeSet::new();
                for contributor in contributing_indices(index, self.extent) {
                    causes.extend(direct_causes[contributor].iter().copied());
                    if let Some(trace) = self.last_change[contributor] {
                        causes.insert(trace);
                    }
                }
                changes.push(ManaCellChange {
                    cell_index: index as u16,
                    before: self.intensity[index],
                    after: value,
                    causes: causes.into_iter().collect(),
                });
            }
        }

        Ok(ManaEvolutionProposal {
            field_id: self.id,
            chunk: self.chunk,
            extent: self.extent,
            through,
            proposed,
            inherited_traces: self.last_change.clone(),
            changes,
        })
    }

    fn index(&self, position: LocalCoord) -> Option<usize> {
        if position.x >= self.extent || position.y >= self.extent || position.z >= self.extent {
            return None;
        }
        let extent = usize::from(self.extent);
        Some(
            usize::from(position.z) * extent * extent
                + usize::from(position.y) * extent
                + usize::from(position.x),
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ManaCellChange {
    pub cell_index: u16,
    pub before: i64,
    pub after: i64,
    pub causes: Vec<TraceId>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ManaEvolutionProposal {
    field_id: ManaFieldId,
    chunk: ChunkCoord,
    extent: u8,
    through: SimulationTime,
    proposed: Vec<i64>,
    inherited_traces: Vec<Option<TraceId>>,
    changes: Vec<ManaCellChange>,
}

impl ManaEvolutionProposal {
    pub fn proposed_intensity(&self) -> &[i64] {
        &self.proposed
    }

    pub fn changes(&self) -> &[ManaCellChange] {
        &self.changes
    }

    pub fn commit(self, committed_traces: &[TraceId]) -> Result<ManaField, ManaError> {
        if committed_traces.len() != self.changes.len() {
            return Err(ManaError::CommitTraceMismatch);
        }
        let mut last_change = self.inherited_traces;
        for (change, trace) in self.changes.iter().zip(committed_traces) {
            last_change[usize::from(change.cell_index)] = Some(*trace);
        }
        Ok(ManaField {
            id: self.field_id,
            chunk: self.chunk,
            extent: self.extent,
            observed_through: self.through,
            intensity: self.proposed,
            last_change,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ManaError {
    InvalidExtent,
    InvalidParameters,
    NonMonotonicTime,
    TooManySamples,
    DuplicateSample,
    SampleOutsideWindow,
    PositionOutsideField,
    CommitTraceMismatch,
}

fn group_samples(
    samples: &[PhysicalPatternSample],
) -> BTreeMap<PhysicalPatternId, Vec<PhysicalPatternSample>> {
    let mut groups = BTreeMap::new();
    for sample in samples {
        groups
            .entry(sample.pattern)
            .or_insert_with(Vec::new)
            .push(*sample);
    }
    groups
}

fn pattern_score(samples: &[PhysicalPatternSample], parameters: ManaParameters) -> i64 {
    let recurrence = samples.len().saturating_sub(1) as i64;
    let positions: BTreeSet<_> = samples.iter().map(|sample| sample.position).collect();
    let spatial = positions.len().saturating_sub(1) as i64;

    let mut per_tick = BTreeMap::<SimulationTime, usize>::new();
    for sample in samples {
        *per_tick.entry(sample.observed_at).or_default() += 1;
    }
    let synchrony = per_tick
        .values()
        .map(|count| count.saturating_sub(1))
        .sum::<usize>() as i64;
    let times: Vec<_> = per_tick.keys().map(|time| time.raw()).collect();
    let intervals: Vec<_> = times.windows(2).map(|pair| pair[1] - pair[0]).collect();
    let periodicity = if intervals.len() >= 2
        && intervals[0] > 0
        && intervals.iter().all(|interval| *interval == intervals[0])
    {
        intervals.len().saturating_sub(1) as i64
    } else {
        0
    };

    i64::from(parameters.base_response)
        .saturating_add(recurrence.saturating_mul(i64::from(parameters.recurrence_response)))
        .saturating_add(periodicity.saturating_mul(i64::from(parameters.periodicity_response)))
        .saturating_add(synchrony.saturating_mul(i64::from(parameters.synchrony_response)))
        .saturating_add(spatial.saturating_mul(i64::from(parameters.spatial_response)))
}

fn diffuse_cell(index: usize, extent: u8, values: &[i64], parameters: ManaParameters) -> i64 {
    let neighbors = neighbor_indices(index, extent);
    let current = values[index];
    let decay = current.saturating_mul(i64::from(parameters.decay)) / MANA_SCALE;
    let outgoing = current.saturating_mul(i64::from(parameters.diffusion)) / MANA_SCALE;
    let incoming = neighbors
        .iter()
        .map(|neighbor| {
            let count = neighbor_indices(*neighbor, extent).len() as i64;
            values[*neighbor]
                .saturating_mul(i64::from(parameters.diffusion))
                .saturating_div(MANA_SCALE)
                .saturating_div(count.max(1))
        })
        .fold(0i64, i64::saturating_add);
    current
        .saturating_sub(decay)
        .saturating_sub(outgoing)
        .saturating_add(incoming)
        .clamp(0, parameters.maximum_intensity)
}

fn contributing_indices(index: usize, extent: u8) -> Vec<usize> {
    let mut indices = neighbor_indices(index, extent);
    indices.push(index);
    indices.sort_unstable();
    indices
}

fn neighbor_indices(index: usize, extent: u8) -> Vec<usize> {
    let side = usize::from(extent);
    let plane = side * side;
    let z = index / plane;
    let within = index % plane;
    let y = within / side;
    let x = within % side;
    let mut neighbors = Vec::with_capacity(6);
    if x > 0 {
        neighbors.push(index - 1);
    }
    if x + 1 < side {
        neighbors.push(index + 1);
    }
    if y > 0 {
        neighbors.push(index - side);
    }
    if y + 1 < side {
        neighbors.push(index + side);
    }
    if z > 0 {
        neighbors.push(index - plane);
    }
    if z + 1 < side {
        neighbors.push(index + plane);
    }
    neighbors
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parameters() -> ManaParameters {
        ManaParameters {
            base_response: 128,
            recurrence_response: 128,
            periodicity_response: 256,
            synchrony_response: 128,
            spatial_response: 128,
            diffusion: 128,
            decay: 16,
            maximum_intensity: 10_000,
        }
    }

    fn sample(tick: u64, x: u8, ordinal: u32) -> PhysicalPatternSample {
        PhysicalPatternSample {
            pattern: PhysicalPatternId::new(7),
            position: LocalCoord::new(x, 1, 1),
            observed_at: SimulationTime::new(tick),
            magnitude: 1_024,
            source_ordinal: ordinal,
            cause: TraceId::new(u64::from(ordinal) + 1),
        }
    }

    #[test]
    fn evolution_is_canonical_and_proposal_only() {
        let field = ManaField::new(ManaFieldId::new(1), ChunkCoord::new(0, 0, 0), 3).unwrap();
        let ordered = vec![sample(1, 1, 0), sample(2, 1, 1), sample(3, 1, 2)];
        let reversed = ordered.iter().copied().rev().collect::<Vec<_>>();
        let a = field
            .propose_evolution(SimulationTime::new(3), parameters(), &ordered)
            .unwrap();
        let b = field
            .propose_evolution(SimulationTime::new(3), parameters(), &reversed)
            .unwrap();
        assert_eq!(a, b);
        assert!(field.intensity().iter().all(|value| *value == 0));
        assert!(a.proposed_intensity().iter().any(|value| *value > 0));
        assert!(a.changes().iter().all(|change| !change.causes.is_empty()));
    }

    #[test]
    fn regular_repetition_responds_more_than_isolated_samples() {
        let field = ManaField::new(ManaFieldId::new(1), ChunkCoord::new(0, 0, 0), 3).unwrap();
        let repeated = vec![sample(1, 1, 0), sample(2, 1, 1), sample(3, 1, 2)];
        let isolated = vec![
            PhysicalPatternSample {
                pattern: PhysicalPatternId::new(1),
                ..sample(1, 1, 0)
            },
            PhysicalPatternSample {
                pattern: PhysicalPatternId::new(2),
                ..sample(2, 1, 1)
            },
            PhysicalPatternSample {
                pattern: PhysicalPatternId::new(3),
                ..sample(3, 1, 2)
            },
        ];
        let repeated_total: i64 = field
            .propose_evolution(SimulationTime::new(3), parameters(), &repeated)
            .unwrap()
            .proposed_intensity()
            .iter()
            .sum();
        let isolated_total: i64 = field
            .propose_evolution(SimulationTime::new(3), parameters(), &isolated)
            .unwrap()
            .proposed_intensity()
            .iter()
            .sum();
        assert!(repeated_total > isolated_total);
    }

    #[test]
    fn commit_requires_one_trace_per_changed_cell() {
        let field = ManaField::new(ManaFieldId::new(1), ChunkCoord::new(0, 0, 0), 3).unwrap();
        let proposal = field
            .propose_evolution(SimulationTime::new(1), parameters(), &[sample(1, 1, 0)])
            .unwrap();
        assert_eq!(
            proposal.clone().commit(&[]),
            Err(ManaError::CommitTraceMismatch)
        );
        let traces = (0..proposal.changes().len())
            .map(|index| TraceId::new(100 + index as u64))
            .collect::<Vec<_>>();
        let committed = proposal.commit(&traces).unwrap();
        assert_eq!(committed.observed_through(), SimulationTime::new(1));
    }

    #[test]
    fn validates_bounds_and_parameters() {
        assert_eq!(
            ManaField::new(ManaFieldId::new(1), ChunkCoord::new(0, 0, 0), 0),
            Err(ManaError::InvalidExtent)
        );
        let field = ManaField::new(ManaFieldId::new(1), ChunkCoord::new(0, 0, 0), 2).unwrap();
        assert_eq!(
            field.propose_evolution(SimulationTime::new(1), parameters(), &[sample(1, 2, 0)]),
            Err(ManaError::PositionOutsideField)
        );
    }
}
