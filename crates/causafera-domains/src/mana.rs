use std::collections::{BTreeMap, BTreeSet};

use causafera_core::{CausalTarget, StateFingerprint};
use causafera_types::{
    CHUNK_SIZE, ChartChunkCoord, LocalCoord, ManaFieldId, PhysicalPatternId, SimulationTime,
    TraceId,
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
    pub effect_threshold: i64,
    pub effect_hysteresis: i64,
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
        if self.maximum_intensity <= 0 || self.effect_hysteresis < 0 {
            return Err(ManaError::InvalidParameters);
        }
        if self.effect_threshold > 0 && self.effect_hysteresis >= self.effect_threshold {
            return Err(ManaError::InvalidParameters);
        }
        Ok(self)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ManaPhysicalEffectSchemaId(u64);

impl ManaPhysicalEffectSchemaId {
    pub const fn new(raw: u64) -> Self {
        Self(raw)
    }

    pub const fn raw(self) -> u64 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CarrierAdapterSchemaId(u64);

impl CarrierAdapterSchemaId {
    pub const fn new(raw: u64) -> Self {
        Self(raw)
    }

    pub const fn raw(self) -> u64 {
        self.0
    }
}

pub trait PhysicalCarrierAdapter {
    fn schema(&self) -> CarrierAdapterSchemaId;

    fn emit_samples(&self, time: SimulationTime, cause: TraceId) -> Vec<PhysicalPatternSample>;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ManaPhysicalEffectProposal {
    pub schema: ManaPhysicalEffectSchemaId,
    pub target: CausalTarget,
    pub before: StateFingerprint,
    pub after: StateFingerprint,
    pub causes: Vec<TraceId>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct PhysicalPatternSample {
    pub chunk: ChartChunkCoord,
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
    chunk: ChartChunkCoord,
    extent: u8,
    observed_through: SimulationTime,
    intensity: Vec<i64>,
    last_change: Vec<Option<TraceId>>,
    last_change_before: Vec<i64>,
}

/// Which of a chunk's six axis faces have an active same-chart neighbour.
///
/// A cell sitting on an open face has a neighbour across the seam, so that
/// neighbour is counted when the cell's outgoing mana is divided into equal
/// shares; the share itself is delivered by [`ManaFieldSet`]'s boundary
/// exchange rather than by the field's own stencil. A closed face is the edge
/// of the simulated region and reflects, which is the only place a boundary is
/// allowed to be physically visible.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct OpenFaces {
    negative: [bool; 3],
    positive: [bool; 3],
}

impl OpenFaces {
    /// No neighbouring chunk is active, so every face reflects.
    pub const fn none() -> Self {
        Self {
            negative: [false; 3],
            positive: [false; 3],
        }
    }

    fn open(&mut self, axis: usize, positive: bool) {
        if positive {
            self.positive[axis] = true;
        } else {
            self.negative[axis] = true;
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ManaFieldSnapshot {
    pub id: ManaFieldId,
    pub chunk: ChartChunkCoord,
    pub extent: u8,
    pub observed_through: SimulationTime,
    pub intensity: Vec<i64>,
    pub last_change: Vec<Option<TraceId>>,
    pub last_change_before: Vec<i64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ManaFieldSetSnapshot {
    pub fields: Vec<ManaFieldSnapshot>,
}

impl ManaField {
    pub fn new(id: ManaFieldId, chunk: ChartChunkCoord, extent: u8) -> Result<Self, ManaError> {
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
            last_change_before: vec![0; volume],
        })
    }

    pub const fn id(&self) -> ManaFieldId {
        self.id
    }

    pub const fn chunk(&self) -> ChartChunkCoord {
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

    pub fn last_change(&self) -> &[Option<TraceId>] {
        &self.last_change
    }

    pub fn last_change_before(&self) -> &[i64] {
        &self.last_change_before
    }

    pub fn export_snapshot(&self) -> ManaFieldSnapshot {
        ManaFieldSnapshot {
            id: self.id,
            chunk: self.chunk,
            extent: self.extent,
            observed_through: self.observed_through,
            intensity: self.intensity.clone(),
            last_change: self.last_change.clone(),
            last_change_before: self.last_change_before.clone(),
        }
    }

    pub fn import_snapshot(snapshot: ManaFieldSnapshot) -> Result<Self, ManaError> {
        let volume = usize::from(snapshot.extent).pow(3);
        if snapshot.extent == 0
            || snapshot.extent > CHUNK_SIZE
            || snapshot.intensity.len() != volume
            || snapshot.last_change.len() != volume
            || snapshot.last_change_before.len() != volume
        {
            return Err(ManaError::InvalidExtent);
        }
        Ok(Self {
            id: snapshot.id,
            chunk: snapshot.chunk,
            extent: snapshot.extent,
            observed_through: snapshot.observed_through,
            intensity: snapshot.intensity,
            last_change: snapshot.last_change,
            last_change_before: snapshot.last_change_before,
        })
    }

    pub fn intensity_at(&self, position: LocalCoord) -> Option<i64> {
        self.index(position).map(|index| self.intensity[index])
    }

    pub fn propose_evolution(
        &self,
        through: SimulationTime,
        parameters: ManaParameters,
        samples: &[PhysicalPatternSample],
        history: &[PhysicalPatternSample],
        open_faces: OpenFaces,
    ) -> Result<ManaEvolutionProposal, ManaError> {
        let parameters = parameters.validate()?;
        if through <= self.observed_through {
            return Err(ManaError::NonMonotonicTime);
        }
        if samples.len().saturating_add(history.len()) > MAX_MANA_SAMPLES {
            return Err(ManaError::TooManySamples);
        }

        let mut canonical = samples.to_vec();
        canonical.sort_unstable();
        if canonical.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err(ManaError::DuplicateSample);
        }
        for sample in &canonical {
            if sample.chunk != self.chunk {
                return Err(ManaError::SampleOutsideChunk);
            }
            if sample.observed_at <= self.observed_through || sample.observed_at > through {
                return Err(ManaError::SampleOutsideWindow);
            }
            if self.index(sample.position).is_none() {
                return Err(ManaError::PositionOutsideField);
            }
        }
        let mut combined = canonical.clone();
        combined.extend_from_slice(history);
        combined.sort_unstable();
        for sample in history {
            if sample.chunk != self.chunk {
                return Err(ManaError::SampleOutsideChunk);
            }
            if self.index(sample.position).is_none() {
                return Err(ManaError::PositionOutsideField);
            }
        }

        let groups = group_samples(&combined);
        let mut injected = self.intensity.clone();
        let mut direct_causes = vec![Vec::new(); injected.len()];
        for (pattern, group) in &groups {
            if group.len() < 2 {
                continue;
            }
            let score = pattern_score(group, parameters);
            let group_causes: Vec<_> = group.iter().map(|sample| sample.cause).collect();
            for sample in canonical.iter().filter(|sample| sample.pattern == *pattern) {
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

        let counts = neighbor_counts(self.extent, open_faces);
        let mut proposed = vec![0; injected.len()];
        let mut changes = Vec::new();
        for (index, proposed_value) in proposed.iter_mut().enumerate() {
            let value = diffuse_cell(index, self.extent, &injected, parameters, &counts);
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
            base_intensity: self.intensity.clone(),
            diffusion_input: injected,
            open_faces,
            proposed,
            inherited_traces: self.last_change.clone(),
            inherited_change_before: self.last_change_before.clone(),
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
pub struct ManaFieldSet {
    fields: BTreeMap<ChartChunkCoord, ManaField>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExperimentRecipeManaSourceManaProposal {
    pub chunk: ChartChunkCoord,
    pub cell_index: u16,
    pub before: i64,
    pub after: i64,
}

impl ManaFieldSet {
    pub fn new(fields: Vec<ManaField>) -> Result<Self, ManaError> {
        if fields.is_empty() {
            return Err(ManaError::InvalidFieldSet);
        }
        let mut ordered = BTreeMap::new();
        for field in fields {
            if ordered.insert(field.chunk(), field).is_some() {
                return Err(ManaError::DuplicateFieldChunk);
            }
        }
        Ok(Self { fields: ordered })
    }

    pub fn fields(&self) -> &BTreeMap<ChartChunkCoord, ManaField> {
        &self.fields
    }

    pub fn export_snapshot(&self) -> ManaFieldSetSnapshot {
        ManaFieldSetSnapshot {
            fields: self
                .fields
                .values()
                .map(ManaField::export_snapshot)
                .collect(),
        }
    }

    pub fn import_snapshot(snapshot: ManaFieldSetSnapshot) -> Result<Self, ManaError> {
        let fields = snapshot
            .fields
            .into_iter()
            .map(ManaField::import_snapshot)
            .collect::<Result<Vec<_>, _>>()?;
        Self::new(fields)
    }

    pub fn field(&self, chunk: ChartChunkCoord) -> Option<&ManaField> {
        self.fields.get(&chunk)
    }

    pub fn propose_experiment_recipe_mana_source(
        &self,
        chunk: ChartChunkCoord,
        cell_index: u16,
        amount: i64,
    ) -> Result<ExperimentRecipeManaSourceManaProposal, ManaError> {
        if amount <= 0 {
            return Err(ManaError::InvalidSourceAmount);
        }
        let field = self
            .fields
            .get(&chunk)
            .ok_or(ManaError::UnknownFieldChunk)?;
        let index = usize::from(cell_index);
        let before = field
            .intensity
            .get(index)
            .copied()
            .ok_or(ManaError::PositionOutsideField)?;
        Ok(ExperimentRecipeManaSourceManaProposal {
            chunk,
            cell_index,
            before,
            after: before.saturating_add(amount),
        })
    }

    pub fn commit_experiment_recipe_mana_source(
        mut self,
        proposal: ExperimentRecipeManaSourceManaProposal,
        source_trace: TraceId,
    ) -> Result<Self, ManaError> {
        let field = self
            .fields
            .get_mut(&proposal.chunk)
            .ok_or(ManaError::UnknownFieldChunk)?;
        let index = usize::from(proposal.cell_index);
        if index >= field.intensity.len() {
            return Err(ManaError::PositionOutsideField);
        }
        field.intensity[index] = proposal.after;
        field.last_change[index] = Some(source_trace);
        field.last_change_before[index] = proposal.before;
        Ok(self)
    }

    pub fn total_intensity(&self) -> i64 {
        self.fields
            .values()
            .flat_map(ManaField::intensity)
            .copied()
            .sum()
    }

    pub fn maximum_intensity(&self) -> i64 {
        self.fields
            .values()
            .flat_map(ManaField::intensity)
            .copied()
            .max()
            .unwrap_or(0)
    }

    pub fn observed_through(&self) -> Option<SimulationTime> {
        self.fields.values().map(ManaField::observed_through).min()
    }

    pub fn propose_evolution(
        &self,
        through: SimulationTime,
        parameters: ManaParameters,
        samples: &[PhysicalPatternSample],
        history: &[PhysicalPatternSample],
    ) -> Result<ManaFieldSetEvolutionProposal, ManaError> {
        let mut samples_by_chunk = BTreeMap::<ChartChunkCoord, Vec<PhysicalPatternSample>>::new();
        for sample in samples {
            if !self.fields.contains_key(&sample.chunk) {
                return Err(ManaError::UnknownFieldChunk);
            }
            samples_by_chunk
                .entry(sample.chunk)
                .or_default()
                .push(*sample);
        }
        let mut history_by_chunk = BTreeMap::<ChartChunkCoord, Vec<PhysicalPatternSample>>::new();
        for sample in history {
            if !self.fields.contains_key(&sample.chunk) {
                return Err(ManaError::UnknownFieldChunk);
            }
            history_by_chunk
                .entry(sample.chunk)
                .or_default()
                .push(*sample);
        }

        let mut field_proposals = BTreeMap::new();
        for (chunk, field) in &self.fields {
            let field_samples = samples_by_chunk
                .get(chunk)
                .map(Vec::as_slice)
                .unwrap_or(&[]);
            let field_history = history_by_chunk
                .get(chunk)
                .map(Vec::as_slice)
                .unwrap_or(&[]);
            let proposal = field.propose_evolution(
                through,
                parameters,
                field_samples,
                field_history,
                open_faces_for(&self.fields, *chunk, field.extent)?,
            )?;
            field_proposals.insert(*chunk, proposal);
        }
        apply_boundary_exchange(&self.fields, &mut field_proposals, parameters)?;
        Ok(ManaFieldSetEvolutionProposal { field_proposals })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ManaFieldSetEvolutionProposal {
    field_proposals: BTreeMap<ChartChunkCoord, ManaEvolutionProposal>,
}

impl ManaFieldSetEvolutionProposal {
    pub fn field_proposals(&self) -> &BTreeMap<ChartChunkCoord, ManaEvolutionProposal> {
        &self.field_proposals
    }

    pub fn changes(&self) -> Vec<(ChartChunkCoord, ManaCellChange)> {
        self.field_proposals
            .iter()
            .flat_map(|(chunk, proposal)| {
                proposal
                    .changes()
                    .iter()
                    .cloned()
                    .map(|change| (*chunk, change))
            })
            .collect()
    }

    pub fn proposed_total_intensity(&self) -> i64 {
        self.field_proposals
            .values()
            .flat_map(ManaEvolutionProposal::proposed_intensity)
            .copied()
            .sum()
    }

    pub fn commit(
        self,
        committed_traces: &BTreeMap<ChartChunkCoord, Vec<TraceId>>,
    ) -> Result<ManaFieldSet, ManaError> {
        let mut fields = Vec::with_capacity(self.field_proposals.len());
        for (chunk, proposal) in self.field_proposals {
            let traces = committed_traces
                .get(&chunk)
                .map(Vec::as_slice)
                .unwrap_or(&[]);
            fields.push(proposal.commit(traces)?);
        }
        ManaFieldSet::new(fields)
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
    chunk: ChartChunkCoord,
    extent: u8,
    through: SimulationTime,
    base_intensity: Vec<i64>,
    /// The field after injection and before diffusion. The boundary exchange
    /// reads it so a cross-chunk share is computed from exactly the values the
    /// interior stencil diffused, not from an already-diffused result.
    diffusion_input: Vec<i64>,
    open_faces: OpenFaces,
    proposed: Vec<i64>,
    inherited_traces: Vec<Option<TraceId>>,
    inherited_change_before: Vec<i64>,
    changes: Vec<ManaCellChange>,
}

impl ManaEvolutionProposal {
    pub const fn chunk(&self) -> ChartChunkCoord {
        self.chunk
    }

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
        let mut last_change_before = self.inherited_change_before;
        for (change, trace) in self.changes.iter().zip(committed_traces) {
            let index = usize::from(change.cell_index);
            last_change[index] = Some(*trace);
            last_change_before[index] = change.before;
        }
        Ok(ManaField {
            id: self.field_id,
            chunk: self.chunk,
            extent: self.extent,
            observed_through: self.through,
            intensity: self.proposed,
            last_change,
            last_change_before,
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
    SampleOutsideChunk,
    PositionOutsideField,
    InvalidSourceAmount,
    CommitTraceMismatch,
    InvalidFieldSet,
    DuplicateFieldChunk,
    UnknownFieldChunk,
}

fn open_faces_for(
    fields: &BTreeMap<ChartChunkCoord, ManaField>,
    chunk: ChartChunkCoord,
    extent: u8,
) -> Result<OpenFaces, ManaError> {
    let mut faces = OpenFaces::none();
    for axis in 0..3 {
        for positive in [false, true] {
            let mut delta = [0_i32; 3];
            delta[axis] = if positive { 1 } else { -1 };
            let neighbor = chunk.same_chart_neighbor(delta[0], delta[1], delta[2]);
            let Some(neighbor_field) = fields.get(&neighbor) else {
                continue;
            };
            if neighbor_field.extent != extent {
                return Err(ManaError::InvalidFieldSet);
            }
            faces.open(axis, positive);
        }
    }
    Ok(faces)
}

fn apply_boundary_exchange(
    fields: &BTreeMap<ChartChunkCoord, ManaField>,
    proposals: &mut BTreeMap<ChartChunkCoord, ManaEvolutionProposal>,
    parameters: ManaParameters,
) -> Result<(), ManaError> {
    let rate = i64::from(parameters.diffusion);
    if rate == 0 {
        return Ok(());
    }
    let chunks = fields.keys().copied().collect::<Vec<_>>();
    for chunk in chunks {
        for direction in [
            BoundaryDirection::PositiveX,
            BoundaryDirection::PositiveY,
            BoundaryDirection::PositiveZ,
        ] {
            let neighbor = direction.neighbor(chunk);
            if fields.contains_key(&neighbor) {
                exchange_boundary_pair(chunk, neighbor, direction, fields, proposals, rate)?;
            }
        }
    }
    Ok(())
}

#[derive(Clone, Copy)]
enum BoundaryDirection {
    PositiveX,
    PositiveY,
    PositiveZ,
}

impl BoundaryDirection {
    const fn neighbor(self, chunk: ChartChunkCoord) -> ChartChunkCoord {
        match self {
            Self::PositiveX => chunk.same_chart_neighbor(1, 0, 0),
            Self::PositiveY => chunk.same_chart_neighbor(0, 1, 0),
            Self::PositiveZ => chunk.same_chart_neighbor(0, 0, 1),
        }
    }
}

fn exchange_boundary_pair(
    left_chunk: ChartChunkCoord,
    right_chunk: ChartChunkCoord,
    direction: BoundaryDirection,
    fields: &BTreeMap<ChartChunkCoord, ManaField>,
    proposals: &mut BTreeMap<ChartChunkCoord, ManaEvolutionProposal>,
    rate: i64,
) -> Result<(), ManaError> {
    let left_field = fields
        .get(&left_chunk)
        .ok_or(ManaError::UnknownFieldChunk)?;
    let right_field = fields
        .get(&right_chunk)
        .ok_or(ManaError::UnknownFieldChunk)?;
    if left_field.extent != right_field.extent {
        return Err(ManaError::InvalidFieldSet);
    }
    let extent = left_field.extent;
    let pairs = boundary_pairs(extent, direction);
    let mut transfers = Vec::with_capacity(pairs.len());
    for (left_index, right_index) in pairs {
        let left = proposals
            .get(&left_chunk)
            .ok_or(ManaError::UnknownFieldChunk)?;
        let right = proposals
            .get(&right_chunk)
            .ok_or(ManaError::UnknownFieldChunk)?;
        // Each side hands over the same share it would have handed an in-chunk
        // neighbour, computed from the pre-diffusion values the interior
        // stencil used. Reading the proposed values instead would both double
        // the seam's conductance and make the result depend on the order the
        // faces happen to be visited.
        let given = diffusion_share(
            left.diffusion_input[left_index],
            rate,
            neighbor_count(left_index, extent, left.open_faces),
        );
        let taken = diffusion_share(
            right.diffusion_input[right_index],
            rate,
            neighbor_count(right_index, extent, right.open_faces),
        );
        transfers.push((left_index, right_index, given, taken));
    }
    // Each side has already parted with its share inside its own stencil,
    // because the open face was counted there. All that is left is delivery.
    for (left_index, right_index, given, taken) in transfers {
        let left_trace = left_field.last_change[left_index];
        let right_trace = right_field.last_change[right_index];
        if taken != 0 {
            apply_exchange_delta(
                proposals,
                left_chunk,
                left_index,
                taken,
                [left_trace, right_trace],
            )?;
        }
        if given != 0 {
            apply_exchange_delta(
                proposals,
                right_chunk,
                right_index,
                given,
                [left_trace, right_trace],
            )?;
        }
    }
    Ok(())
}

fn apply_exchange_delta(
    proposals: &mut BTreeMap<ChartChunkCoord, ManaEvolutionProposal>,
    chunk: ChartChunkCoord,
    index: usize,
    delta: i64,
    causes: [Option<TraceId>; 2],
) -> Result<(), ManaError> {
    let proposal = proposals
        .get_mut(&chunk)
        .ok_or(ManaError::UnknownFieldChunk)?;
    let before = proposal.proposed[index];
    let base = proposal.base_intensity[index];
    let after = before.saturating_add(delta);
    proposal.proposed[index] = after;
    let cell_index = index as u16;
    if let Some(change_index) = proposal
        .changes
        .iter()
        .position(|change| change.cell_index == cell_index)
    {
        if after == base {
            proposal.changes.remove(change_index);
            return Ok(());
        }
        let change = &mut proposal.changes[change_index];
        change.after = after;
        for cause in causes.into_iter().flatten() {
            if !change.causes.contains(&cause) {
                change.causes.push(cause);
            }
        }
        change.causes.sort_unstable();
        return Ok(());
    }
    if after != base {
        proposal.changes.push(ManaCellChange {
            cell_index,
            before: base,
            after,
            causes: causes
                .into_iter()
                .flatten()
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect(),
        });
    }
    Ok(())
}

fn boundary_pairs(extent: u8, direction: BoundaryDirection) -> Vec<(usize, usize)> {
    let side = usize::from(extent);
    let mut pairs = Vec::with_capacity(side * side);
    for a in 0..side {
        for b in 0..side {
            let (left, right) = match direction {
                BoundaryDirection::PositiveX => ((side - 1, a, b), (0, a, b)),
                BoundaryDirection::PositiveY => ((a, side - 1, b), (a, 0, b)),
                BoundaryDirection::PositiveZ => ((a, b, side - 1), (a, b, 0)),
            };
            pairs.push((flat_index(left, side), flat_index(right, side)));
        }
    }
    pairs
}

fn flat_index((x, y, z): (usize, usize, usize), side: usize) -> usize {
    z * side * side + y * side + x
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

fn diffuse_cell(
    index: usize,
    extent: u8,
    values: &[i64],
    parameters: ManaParameters,
    counts: &[i64],
) -> i64 {
    let rate = i64::from(parameters.diffusion);
    let current = values[index];
    let decay = current.saturating_mul(i64::from(parameters.decay)) / MANA_SCALE;
    // The cell gives every neighbour the same share and loses exactly the sum
    // of those shares. Subtracting an undivided budget instead would destroy
    // the truncation remainder on every cell of every tick.
    let count = counts[index];
    let outgoing = diffusion_share(current, rate, count).saturating_mul(count);
    let incoming = neighbor_indices(index, extent)
        .iter()
        .map(|neighbor| diffusion_share(values[*neighbor], rate, counts[*neighbor]))
        .fold(0i64, i64::saturating_add);
    current
        .saturating_sub(decay)
        .saturating_sub(outgoing)
        .saturating_add(incoming)
        .clamp(0, parameters.maximum_intensity)
}

/// One neighbour's share of a cell's diffused mana. Truncating here and
/// multiplying back up is what keeps the stencil conservative.
fn diffusion_share(value: i64, rate: i64, count: i64) -> i64 {
    if count <= 0 {
        return 0;
    }
    value
        .saturating_mul(rate)
        .saturating_div(MANA_SCALE)
        .saturating_div(count)
}

/// The neighbour count of every cell in a field, built once per proposal so the
/// stencil never recomputes it per neighbour per cell.
fn neighbor_counts(extent: u8, open_faces: OpenFaces) -> Vec<i64> {
    (0..usize::from(extent).pow(3))
        .map(|index| neighbor_count(index, extent, open_faces))
        .collect()
}

/// How many neighbours a cell has, counting the one across an open chunk face.
/// Only a face with no active neighbour reduces the count, so a cell's share
/// does not depend on where the chunk grid happens to fall (INV-037).
fn neighbor_count(index: usize, extent: u8, open_faces: OpenFaces) -> i64 {
    let side = usize::from(extent);
    let plane = side * side;
    let z = index / plane;
    let within = index % plane;
    let y = within / side;
    let x = within % side;
    let mut count = 0_i64;
    for (axis, coordinate) in [x, y, z].into_iter().enumerate() {
        count += if coordinate > 0 {
            1
        } else {
            i64::from(open_faces.negative[axis])
        };
        count += if coordinate + 1 < side {
            1
        } else {
            i64::from(open_faces.positive[axis])
        };
    }
    count
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
    use causafera_types::{ChunkCoord, SpatialChartId};

    fn chart_chunk() -> ChartChunkCoord {
        ChartChunkCoord::new(SpatialChartId::new(1), ChunkCoord::new(0, 0, 0))
    }

    fn chart_chunk_at(chart: u64, x: i32) -> ChartChunkCoord {
        ChartChunkCoord::new(SpatialChartId::new(chart), ChunkCoord::new(x, 0, 0))
    }

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
            effect_threshold: 0,
            effect_hysteresis: 0,
        }
    }

    fn sample(tick: u64, x: u8, ordinal: u32) -> PhysicalPatternSample {
        sample_in(chart_chunk(), tick, x, ordinal)
    }

    fn sample_in(chunk: ChartChunkCoord, tick: u64, x: u8, ordinal: u32) -> PhysicalPatternSample {
        PhysicalPatternSample {
            chunk,
            pattern: PhysicalPatternId::new(7),
            position: LocalCoord::new(x, 1, 1),
            observed_at: SimulationTime::new(tick),
            magnitude: 1_024,
            source_ordinal: ordinal,
            cause: TraceId::new(u64::from(ordinal) + 1),
        }
    }

    fn field_set(left: ChartChunkCoord, right: ChartChunkCoord) -> ManaFieldSet {
        ManaFieldSet::new(vec![
            ManaField::new(ManaFieldId::new(1), left, 3).unwrap(),
            ManaField::new(ManaFieldId::new(2), right, 3).unwrap(),
        ])
        .unwrap()
    }

    #[test]
    fn evolution_is_canonical_and_proposal_only() {
        let field = ManaField::new(ManaFieldId::new(1), chart_chunk(), 3).unwrap();
        let ordered = vec![sample(1, 1, 0), sample(2, 1, 1), sample(3, 1, 2)];
        let reversed = ordered.iter().copied().rev().collect::<Vec<_>>();
        let a = field
            .propose_evolution(
                SimulationTime::new(3),
                parameters(),
                &ordered,
                &[],
                OpenFaces::none(),
            )
            .unwrap();
        let b = field
            .propose_evolution(
                SimulationTime::new(3),
                parameters(),
                &reversed,
                &[],
                OpenFaces::none(),
            )
            .unwrap();
        assert_eq!(a, b);
        assert!(field.intensity().iter().all(|value| *value == 0));
        assert!(a.proposed_intensity().iter().any(|value| *value > 0));
        assert!(a.changes().iter().all(|change| !change.causes.is_empty()));
    }

    #[test]
    fn regular_repetition_responds_more_than_isolated_samples() {
        let field = ManaField::new(ManaFieldId::new(1), chart_chunk(), 3).unwrap();
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
            .propose_evolution(
                SimulationTime::new(3),
                parameters(),
                &repeated,
                &[],
                OpenFaces::none(),
            )
            .unwrap()
            .proposed_intensity()
            .iter()
            .sum();
        let isolated_total: i64 = field
            .propose_evolution(
                SimulationTime::new(3),
                parameters(),
                &isolated,
                &[],
                OpenFaces::none(),
            )
            .unwrap()
            .proposed_intensity()
            .iter()
            .sum();
        assert!(repeated_total > isolated_total);
    }

    #[test]
    fn commit_requires_one_trace_per_changed_cell() {
        let field = ManaField::new(ManaFieldId::new(1), chart_chunk(), 3).unwrap();
        let proposal = field
            .propose_evolution(
                SimulationTime::new(2),
                parameters(),
                &[sample(1, 1, 0), sample(2, 1, 1)],
                &[],
                OpenFaces::none(),
            )
            .unwrap();
        assert_eq!(
            proposal.clone().commit(&[]),
            Err(ManaError::CommitTraceMismatch)
        );
        let traces = (0..proposal.changes().len())
            .map(|index| TraceId::new(100 + index as u64))
            .collect::<Vec<_>>();
        let committed = proposal.commit(&traces).unwrap();
        assert_eq!(committed.observed_through(), SimulationTime::new(2));
    }

    #[test]
    fn experiment_recipe_mana_source_proposes_without_mutating() {
        // Given: an existing field set with an empty target cell.
        let field = ManaField::new(ManaFieldId::new(1), chart_chunk(), 3).unwrap();
        let fields = ManaFieldSet::new(vec![field]).unwrap();

        // When: a positive experiment-recipe source amount is proposed.
        let proposal = fields
            .propose_experiment_recipe_mana_source(chart_chunk(), 4, 7)
            .unwrap();

        // Then: the proposal contains the fixed-point transition and the field remains unchanged.
        assert_eq!(proposal.chunk, chart_chunk());
        assert_eq!(proposal.cell_index, 4);
        assert_eq!(proposal.before, 0);
        assert_eq!(proposal.after, 7);
        assert_eq!(fields.field(chart_chunk()).unwrap().intensity()[4], 0);
        assert_eq!(fields.observed_through(), Some(SimulationTime::new(0)));
    }

    #[test]
    fn experiment_recipe_mana_source_commit_sets_trace_without_advancing_clock() {
        // Given: a pure source proposal for an existing mana cell.
        let fields = ManaFieldSet::new(vec![
            ManaField::new(ManaFieldId::new(1), chart_chunk(), 3).unwrap(),
        ])
        .unwrap();
        let proposal = fields
            .propose_experiment_recipe_mana_source(chart_chunk(), 4, 7)
            .unwrap();

        // When: the proposal is committed with its source trace.
        let committed = fields
            .commit_experiment_recipe_mana_source(proposal, TraceId::new(42))
            .unwrap();

        // Then: only the cell value and last-change trace change; the evolution clock does not.
        let field = committed.field(chart_chunk()).unwrap();
        assert_eq!(field.intensity()[4], 7);
        assert_eq!(field.last_change()[4], Some(TraceId::new(42)));
        assert_eq!(field.last_change_before()[4], 0);
        assert_eq!(field.observed_through(), SimulationTime::new(0));
    }

    #[test]
    fn experiment_recipe_mana_source_saturates_at_i64_maximum() {
        // Given: a target cell already at the fixed-point maximum.
        let mut field = ManaField::new(ManaFieldId::new(1), chart_chunk(), 1)
            .unwrap()
            .export_snapshot();
        field.intensity[0] = i64::MAX;
        let fields = ManaFieldSet::import_snapshot(ManaFieldSetSnapshot {
            fields: vec![field],
        })
        .unwrap();

        // When: a positive source is proposed at that cell.
        let proposal = fields
            .propose_experiment_recipe_mana_source(chart_chunk(), 0, 1)
            .unwrap();

        // Then: fixed-point addition saturates instead of overflowing.
        assert_eq!(proposal.before, i64::MAX);
        assert_eq!(proposal.after, i64::MAX);
    }

    #[test]
    fn experiment_recipe_mana_source_rejects_invalid_targets_and_amounts() {
        // Given: a field set containing exactly one bounded field.
        let fields = ManaFieldSet::new(vec![
            ManaField::new(ManaFieldId::new(1), chart_chunk(), 3).unwrap(),
        ])
        .unwrap();

        // When: source proposals use an unknown chunk, invalid cell, or non-positive amount.
        // Then: every invalid source is rejected before mutation.
        assert_eq!(
            fields.propose_experiment_recipe_mana_source(chart_chunk_at(2, 0), 0, 1),
            Err(ManaError::UnknownFieldChunk)
        );
        assert_eq!(
            fields.propose_experiment_recipe_mana_source(chart_chunk(), 27, 1),
            Err(ManaError::PositionOutsideField)
        );
        assert_eq!(
            fields.propose_experiment_recipe_mana_source(chart_chunk(), 0, 0),
            Err(ManaError::InvalidSourceAmount)
        );
        assert_eq!(
            fields.propose_experiment_recipe_mana_source(chart_chunk(), 0, -1),
            Err(ManaError::InvalidSourceAmount)
        );
    }

    #[test]
    fn boundary_exchange_that_returns_to_start_emits_no_mana_change() {
        let field = ManaField::new(ManaFieldId::new(1), chart_chunk(), 3).unwrap();
        let proposal = field
            .propose_evolution(
                SimulationTime::new(1),
                parameters(),
                &[],
                &[],
                OpenFaces::none(),
            )
            .unwrap();
        let mut proposals = BTreeMap::from([(chart_chunk(), proposal)]);

        apply_exchange_delta(&mut proposals, chart_chunk(), 0, 5, [None, None]).unwrap();
        apply_exchange_delta(&mut proposals, chart_chunk(), 0, -5, [None, None]).unwrap();

        assert!(
            proposals[&chart_chunk()].changes().is_empty(),
            "a net-zero boundary exchange must not require a causal commit trace"
        );
    }

    #[test]
    fn boundary_exchange_canonicalizes_parent_traces() {
        let field = ManaField::new(ManaFieldId::new(1), chart_chunk(), 3).unwrap();
        let proposal = field
            .propose_evolution(
                SimulationTime::new(1),
                parameters(),
                &[],
                &[],
                OpenFaces::none(),
            )
            .unwrap();
        let mut proposals = BTreeMap::from([(chart_chunk(), proposal)]);

        apply_exchange_delta(
            &mut proposals,
            chart_chunk(),
            0,
            5,
            [Some(TraceId::new(5)), Some(TraceId::new(3))],
        )
        .unwrap();

        assert_eq!(
            proposals[&chart_chunk()].changes()[0].causes,
            vec![TraceId::new(3), TraceId::new(5)],
            "boundary-exchange parents must be strictly ordered for provenance commits"
        );
    }

    #[test]
    fn validates_bounds_and_parameters() {
        assert_eq!(
            ManaField::new(ManaFieldId::new(1), chart_chunk(), 0),
            Err(ManaError::InvalidExtent)
        );
        let field = ManaField::new(ManaFieldId::new(1), chart_chunk(), 2).unwrap();
        assert_eq!(
            field.propose_evolution(
                SimulationTime::new(1),
                parameters(),
                &[sample(1, 2, 0)],
                &[],
                OpenFaces::none()
            ),
            Err(ManaError::PositionOutsideField)
        );
    }

    #[test]
    fn historical_periodicity_changes_current_response() {
        let field = ManaField::new(ManaFieldId::new(1), chart_chunk(), 3).unwrap();
        let current = [sample(5, 1, 5)];
        let periodic = [
            sample(1, 1, 1),
            sample(2, 1, 2),
            sample(3, 1, 3),
            sample(4, 1, 4),
        ];
        let irregular = [sample(1, 1, 1), sample(2, 1, 2), sample(4, 1, 4)];

        let periodic_total: i64 = field
            .propose_evolution(
                SimulationTime::new(5),
                parameters(),
                &current,
                &periodic,
                OpenFaces::none(),
            )
            .unwrap()
            .proposed_intensity()
            .iter()
            .sum();
        let irregular_total: i64 = field
            .propose_evolution(
                SimulationTime::new(5),
                parameters(),
                &current,
                &irregular,
                OpenFaces::none(),
            )
            .unwrap()
            .proposed_intensity()
            .iter()
            .sum();

        assert!(periodic_total > irregular_total);
    }

    #[test]
    fn burst_and_cross_tick_evidence_are_distinguishable() {
        let field = ManaField::new(ManaFieldId::new(1), chart_chunk(), 3).unwrap();
        let repeated_current = [sample(3, 1, 3)];
        let repeated_history = [sample(1, 1, 1), sample(2, 1, 2)];
        let burst = [sample(3, 0, 10), sample(3, 1, 11), sample(3, 2, 12)];

        let repeated_total: i64 = field
            .propose_evolution(
                SimulationTime::new(3),
                parameters(),
                &repeated_current,
                &repeated_history,
                OpenFaces::none(),
            )
            .unwrap()
            .proposed_intensity()
            .iter()
            .sum();
        let burst_total: i64 = field
            .propose_evolution(
                SimulationTime::new(3),
                parameters(),
                &burst,
                &[],
                OpenFaces::none(),
            )
            .unwrap()
            .proposed_intensity()
            .iter()
            .sum();

        assert_ne!(repeated_total, burst_total);
    }

    #[test]
    fn historical_samples_skip_time_window_validation() {
        let field = ManaField::new(ManaFieldId::new(1), chart_chunk(), 3).unwrap();
        let proposal = field
            .propose_evolution(
                SimulationTime::new(2),
                parameters(),
                &[sample(2, 1, 2)],
                &[sample(0, 1, 0)],
                OpenFaces::none(),
            )
            .unwrap();

        assert!(proposal.proposed_intensity().iter().any(|value| *value > 0));
    }

    #[test]
    fn cross_chunk_exchange_is_canonical_under_sample_partitioning() {
        let left = chart_chunk_at(1, 0);
        let right = chart_chunk_at(1, 1);
        let fields = field_set(left, right);
        let samples = [sample_in(left, 1, 2, 1), sample_in(right, 1, 0, 2)];
        let reversed = [samples[1], samples[0]];

        let first = fields
            .propose_evolution(SimulationTime::new(1), parameters(), &samples, &[])
            .unwrap();
        let second = fields
            .propose_evolution(SimulationTime::new(1), parameters(), &reversed, &[])
            .unwrap();

        assert_eq!(first, second);
        assert_eq!(
            first.proposed_total_intensity(),
            second.proposed_total_intensity()
        );
    }

    /// A chunk face carries no physical meaning (INV-037), so a cell must hand
    /// the same share across a seam as it hands an in-chunk neighbour, and a
    /// cell sitting on a seam must not over-feed the neighbours on its own side.
    #[test]
    fn a_seam_conducts_exactly_as_the_interior_does() {
        const EXTENT: u8 = 8;
        const V: i64 = 1_000_000;
        let side = usize::from(EXTENT);
        let at = |x: usize, y: usize, z: usize| z * side * side + y * side + x;

        let left = chart_chunk_at(1, 0);
        let right = chart_chunk_at(1, 1);
        let blank = |id: u64, chunk: ChartChunkCoord| ManaFieldSnapshot {
            id: ManaFieldId::new(id),
            chunk,
            extent: EXTENT,
            observed_through: SimulationTime::default(),
            intensity: vec![0; side.pow(3)],
            last_change: vec![None; side.pow(3)],
            last_change_before: vec![0; side.pow(3)],
        };
        let seeded = |source_x: usize| {
            let mut left_snapshot = blank(1, left);
            left_snapshot.intensity[at(source_x, 4, 4)] = V;
            ManaFieldSet::new(vec![
                ManaField::import_snapshot(left_snapshot).unwrap(),
                ManaField::import_snapshot(blank(2, right)).unwrap(),
            ])
            .unwrap()
        };

        let mut parameters = parameters();
        parameters.maximum_intensity = i64::MAX / 4;

        let interior = seeded(3)
            .propose_evolution(SimulationTime::new(1), parameters, &[], &[])
            .unwrap();
        let interior_received = interior
            .field_proposals()
            .get(&left)
            .unwrap()
            .proposed_intensity()[at(4, 4, 4)];

        let seam = seeded(7)
            .propose_evolution(SimulationTime::new(1), parameters, &[], &[])
            .unwrap();
        let seam_received = seam
            .field_proposals()
            .get(&right)
            .unwrap()
            .proposed_intensity()[at(0, 4, 4)];
        let seam_sibling = seam
            .field_proposals()
            .get(&left)
            .unwrap()
            .proposed_intensity()[at(6, 4, 4)];

        assert!(interior_received > 0);
        assert_eq!(seam_received, interior_received);
        assert_eq!(seam_sibling, interior_received);
    }

    /// Diffusion moves mana; it must not destroy it. The cell gives away the
    /// sum of the shares its neighbours receive, so the only sanctioned losses
    /// are decay and the clamp.
    #[test]
    fn diffusion_alone_conserves_mana_across_a_seam() {
        const EXTENT: u8 = 5;
        let side = usize::from(EXTENT);
        let left = chart_chunk_at(1, 0);
        let right = chart_chunk_at(1, 1);

        let mut parameters = parameters();
        parameters.decay = 0;
        parameters.maximum_intensity = i64::MAX / 4;

        let seeded = |chunk: ChartChunkCoord, id: u64, seed: i64| {
            let mut intensity = vec![0; side.pow(3)];
            for (index, value) in intensity.iter_mut().enumerate() {
                *value = seed + (index as i64 % 97) * 1_013;
            }
            ManaField::import_snapshot(ManaFieldSnapshot {
                id: ManaFieldId::new(id),
                chunk,
                extent: EXTENT,
                observed_through: SimulationTime::default(),
                intensity,
                last_change: vec![None; side.pow(3)],
                last_change_before: vec![0; side.pow(3)],
            })
            .unwrap()
        };
        let fields =
            ManaFieldSet::new(vec![seeded(left, 1, 7_000), seeded(right, 2, 40_000)]).unwrap();
        let before = fields.total_intensity();

        let after = fields
            .propose_evolution(SimulationTime::new(1), parameters, &[], &[])
            .unwrap()
            .proposed_total_intensity();

        assert_eq!(after, before);
    }

    #[test]
    fn chart_boundaries_do_not_cross_by_implicit_integer_adjacency() {
        let left = chart_chunk_at(1, 0);
        let same_chart_right = chart_chunk_at(1, 1);
        let other_chart_right = chart_chunk_at(2, 1);
        let samples = [sample_in(left, 1, 2, 1), sample_in(left, 2, 2, 2)];

        let same_chart = field_set(left, same_chart_right)
            .propose_evolution(SimulationTime::new(2), parameters(), &samples, &[])
            .unwrap();
        let cross_chart = field_set(left, other_chart_right)
            .propose_evolution(SimulationTime::new(2), parameters(), &samples, &[])
            .unwrap();

        let same_neighbor_total: i64 = same_chart
            .field_proposals()
            .get(&same_chart_right)
            .unwrap()
            .proposed_intensity()
            .iter()
            .sum();
        let cross_neighbor_total: i64 = cross_chart
            .field_proposals()
            .get(&other_chart_right)
            .unwrap()
            .proposed_intensity()
            .iter()
            .sum();

        assert!(same_neighbor_total > 0);
        assert_eq!(cross_neighbor_total, 0);
    }
}
