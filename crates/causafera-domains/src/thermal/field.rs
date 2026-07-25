use std::collections::{BTreeMap, BTreeSet};

use causafera_types::{
    CHUNK_SIZE, ChartChunkCoord, LocalCoord, ThermalEnergy, TraceId, flat_index,
};

use super::{ThermalCellChange, ThermalCellKey, ThermalCellTransferReceipt, ThermalError};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ThermalField {
    chunk: ChartChunkCoord,
    extent: u8,
    energy: Vec<ThermalEnergy>,
    last_change: Vec<TraceId>,
    last_change_before: Vec<ThermalEnergy>,
}

impl ThermalField {
    pub fn new(
        chunk: ChartChunkCoord,
        extent: u8,
        bootstrap_trace: TraceId,
    ) -> Result<Self, ThermalError> {
        let volume = volume(extent)?;
        Self::from_energy(
            chunk,
            extent,
            vec![ThermalEnergy::ZERO; volume],
            bootstrap_trace,
        )
    }

    pub fn from_energy(
        chunk: ChartChunkCoord,
        extent: u8,
        energy: Vec<ThermalEnergy>,
        bootstrap_trace: TraceId,
    ) -> Result<Self, ThermalError> {
        let volume = volume(extent)?;
        if energy.len() != volume {
            return Err(ThermalError::InvalidEnergyLength);
        }
        Ok(Self {
            chunk,
            extent,
            last_change: vec![bootstrap_trace; volume],
            last_change_before: energy.clone(),
            energy,
        })
    }

    pub fn from_snapshot_parts(
        chunk: ChartChunkCoord,
        extent: u8,
        energy: Vec<ThermalEnergy>,
        last_change: Vec<TraceId>,
        last_change_before: Vec<ThermalEnergy>,
    ) -> Result<Self, ThermalError> {
        let volume = volume(extent)?;
        if energy.len() != volume
            || last_change.len() != volume
            || last_change_before.len() != volume
        {
            return Err(ThermalError::InvalidEnergyLength);
        }
        Ok(Self {
            chunk,
            extent,
            energy,
            last_change,
            last_change_before,
        })
    }

    pub const fn chunk(&self) -> ChartChunkCoord {
        self.chunk
    }

    pub const fn extent(&self) -> u8 {
        self.extent
    }

    pub fn energy(&self) -> &[ThermalEnergy] {
        &self.energy
    }

    pub fn last_change(&self) -> &[TraceId] {
        &self.last_change
    }

    pub fn last_change_before(&self) -> &[ThermalEnergy] {
        &self.last_change_before
    }

    pub fn energy_at(&self, position: LocalCoord) -> Option<ThermalEnergy> {
        flat_index(position, self.extent).map(|index| self.energy[index])
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ThermalFieldSet {
    fields: BTreeMap<ChartChunkCoord, ThermalField>,
    batch_sequence: u64,
    conservation_last_change: TraceId,
}

pub struct ThermalCommittedTraces<'a> {
    pub changes: &'a [ThermalCellChange],
    pub receipts: &'a [ThermalCellTransferReceipt],
    pub cell_traces: &'a BTreeMap<ThermalCellKey, TraceId>,
    pub reservoir_traces: &'a BTreeMap<super::ThermalReservoirId, TraceId>,
    pub conservation_trace: TraceId,
}

impl ThermalFieldSet {
    pub fn new(
        fields: Vec<ThermalField>,
        conservation_last_change: TraceId,
    ) -> Result<Self, ThermalError> {
        if fields.is_empty() {
            return Err(ThermalError::InvalidFieldSet);
        }
        let mut ordered = BTreeMap::new();
        for field in fields {
            if ordered.insert(field.chunk(), field).is_some() {
                return Err(ThermalError::DuplicateFieldChunk);
            }
        }
        Ok(Self {
            fields: ordered,
            batch_sequence: 0,
            conservation_last_change,
        })
    }

    pub fn fields(&self) -> &BTreeMap<ChartChunkCoord, ThermalField> {
        &self.fields
    }

    pub fn from_snapshot_parts(
        fields: Vec<ThermalField>,
        batch_sequence: u64,
        conservation_last_change: TraceId,
    ) -> Result<Self, ThermalError> {
        let mut field_set = Self::new(fields, conservation_last_change)?;
        field_set.batch_sequence = batch_sequence;
        Ok(field_set)
    }

    pub fn field(&self, chunk: ChartChunkCoord) -> Option<&ThermalField> {
        self.fields.get(&chunk)
    }

    pub const fn batch_sequence(&self) -> u64 {
        self.batch_sequence
    }

    pub const fn conservation_last_change(&self) -> TraceId {
        self.conservation_last_change
    }

    pub fn install_committed_traces(&mut self, committed: ThermalCommittedTraces<'_>) {
        let changed = committed
            .changes
            .iter()
            .map(|change| {
                (
                    change.cell,
                    (
                        change.before,
                        committed.cell_traces.get(&change.cell).copied(),
                    ),
                )
            })
            .collect::<BTreeMap<_, _>>();
        let targeted = committed
            .receipts
            .iter()
            .map(|receipt| (receipt.cell, receipt))
            .collect::<BTreeMap<_, _>>();
        for (chunk, field) in &mut self.fields {
            for index in 0..field.energy.len() {
                let Ok(cell_index) = u16::try_from(index) else {
                    continue;
                };
                let key = ThermalCellKey::new(*chunk, cell_index);
                if let Some((before, Some(trace))) = changed.get(&key) {
                    field.last_change[index] = *trace;
                    field.last_change_before[index] = *before;
                    continue;
                }
                let Some(receipt) = targeted.get(&key) else {
                    continue;
                };
                let transfer_trace = receipt
                    .reservoirs
                    .iter()
                    .filter(|record| record.accepted_injection != ThermalEnergy::ZERO)
                    .filter_map(|record| committed.reservoir_traces.get(&record.id).copied())
                    .max();
                if let Some(trace) = transfer_trace {
                    field.last_change[index] = trace;
                    field.last_change_before[index] = receipt.pre_state;
                }
            }
        }
        self.conservation_last_change = committed.conservation_trace;
    }

    pub(crate) fn with_energy(
        &self,
        energy: &BTreeMap<ThermalCellKey, ThermalEnergy>,
    ) -> Result<Self, ThermalError> {
        let mut after = self.clone();
        for (chunk, field) in &mut after.fields {
            for (index, value) in field.energy.iter_mut().enumerate() {
                let cell_index =
                    u16::try_from(index).map_err(|_| ThermalError::PositionOutsideField)?;
                let key = ThermalCellKey::new(*chunk, cell_index);
                if let Some(updated) = energy.get(&key) {
                    *value = *updated;
                }
            }
        }
        after.batch_sequence = after
            .batch_sequence
            .checked_add(1)
            .ok_or(ThermalError::ArithmeticOverflow)?;
        Ok(after)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ThermalActiveRegion {
    active_chunks: BTreeSet<ChartChunkCoord>,
    resident_chunks: BTreeSet<ChartChunkCoord>,
}

impl ThermalActiveRegion {
    pub fn new(
        active_chunks: BTreeSet<ChartChunkCoord>,
        resident_chunks: BTreeSet<ChartChunkCoord>,
    ) -> Result<Self, ThermalError> {
        if !resident_chunks.is_subset(&active_chunks) {
            return Err(ThermalError::InvalidActiveRegion);
        }
        Ok(Self {
            active_chunks,
            resident_chunks,
        })
    }

    pub fn active_chunks(&self) -> &BTreeSet<ChartChunkCoord> {
        &self.active_chunks
    }

    pub fn resident_chunks(&self) -> &BTreeSet<ChartChunkCoord> {
        &self.resident_chunks
    }
}

pub(crate) fn volume(extent: u8) -> Result<usize, ThermalError> {
    if extent == 0 || extent > CHUNK_SIZE {
        return Err(ThermalError::InvalidExtent);
    }
    Ok(usize::from(extent).pow(3))
}
