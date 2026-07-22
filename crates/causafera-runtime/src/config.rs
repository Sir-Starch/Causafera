use crate::*;
use causafera_core::*;
use causafera_domains::*;
use causafera_types::*;
use std::collections::BTreeSet;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeConfig {
    pub deterministic: DeterministicConfig,
    pub chunk_extent: u8,
    pub active_chunk_radius: u8,
    pub chart_id: SpatialChartId,
    pub pattern_schedule: PhysicalPatternSchedule,
    pub mana_parameters: ManaParameters,
    pub carrier_adapter: CarrierAdapterConfig,
    pub actor_count: u8,
    pub sensor_count: u8,
    pub action_bounds: i64,
    pub bootstrap_population: u64,
    pub material_surface_signals_enabled: bool,
    pub experiment_recipe_mana_sources: ExperimentRecipeManaSourceRecipe,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CarrierAdapterConfig {
    TerrainSeed { terrain_seed: u64 },
}

impl CarrierAdapterConfig {
    pub const fn terrain_seed(terrain_seed: u64) -> Self {
        Self::TerrainSeed { terrain_seed }
    }
}

impl RuntimeConfig {
    pub fn new(world_seed: u64) -> Self {
        Self {
            deterministic: DeterministicConfig::new(world_seed),
            chunk_extent: 3,
            active_chunk_radius: 1,
            chart_id: SpatialChartId::new(1),
            pattern_schedule: PhysicalPatternSchedule::continuous(1_024),
            mana_parameters: ManaParameters {
                base_response: 128,
                recurrence_response: 128,
                periodicity_response: 128,
                synchrony_response: 128,
                spatial_response: 128,
                diffusion: 128,
                decay: 24,
                maximum_intensity: 1_000_000,
                effect_threshold: 4_096,
                effect_hysteresis: 2_000,
            },
            carrier_adapter: CarrierAdapterConfig::terrain_seed(world_seed),
            actor_count: 0,
            sensor_count: 0,
            action_bounds: 8,
            bootstrap_population: 0,
            material_surface_signals_enabled: true,
            experiment_recipe_mana_sources: ExperimentRecipeManaSourceRecipe {
                records: Vec::new(),
                recipe_budget: 0,
            },
        }
    }

    pub(crate) fn validate(mut self) -> Result<Self, RuntimeError> {
        if self.chunk_extent < 3 {
            return Err(RuntimeError::InvalidFieldExtent);
        }
        if self.active_chunk_radius > 4 {
            return Err(RuntimeError::InvalidActiveChunkRadius);
        }
        self.pattern_schedule.validate()?;
        self.mana_parameters.validate()?;
        if self.actor_count > 128
            || self.sensor_count > 16
            || self.action_bounds < 0
            || self.bootstrap_population > 10_000
        {
            return Err(RuntimeError::InvalidActorConfig);
        }
        if self.experiment_recipe_mana_sources.records.len() > MAX_EXPERIMENT_RECIPE_MANA_SOURCES {
            return Err(RuntimeError::ExperimentRecipeSourceCountExceeded {
                count: self.experiment_recipe_mana_sources.records.len(),
            });
        }
        if self.experiment_recipe_mana_sources.recipe_budget < 0 {
            return Err(RuntimeError::InvalidExperimentRecipeBudget {
                budget: self.experiment_recipe_mana_sources.recipe_budget,
            });
        }
        let active_chunks = active_chunk_keys(self.chart_id, self.active_chunk_radius);
        let active_chunks = active_chunks.into_iter().collect::<BTreeSet<_>>();
        let mut source_ids = BTreeSet::new();
        let mut canonical_keys = BTreeSet::new();
        let mut enabled_amount = 0_i128;
        let cells_per_extent = u32::from(self.chunk_extent).pow(3);
        for record in &self.experiment_recipe_mana_sources.records {
            if record.source_record_id == 0 {
                return Err(RuntimeError::InvalidExperimentRecipeSourceId {
                    source_record_id: record.source_record_id,
                });
            }
            if !source_ids.insert(record.source_record_id) {
                return Err(RuntimeError::DuplicateExperimentRecipeSourceId {
                    source_record_id: record.source_record_id,
                });
            }
            if !(1..=MAX_RUNTIME_TICKS).contains(&record.scheduled_tick) {
                return Err(RuntimeError::InvalidExperimentRecipeScheduledTick {
                    scheduled_tick: record.scheduled_tick,
                });
            }
            if !canonical_keys.insert((
                record.scheduled_tick,
                record.target_chunk,
                record.cell_index,
            )) {
                return Err(RuntimeError::DuplicateExperimentRecipeCanonicalKey {
                    scheduled_tick: record.scheduled_tick,
                    target_chunk: record.target_chunk,
                    cell_index: record.cell_index,
                });
            }
            if record.amount < 0 {
                return Err(RuntimeError::InvalidExperimentRecipeAmount {
                    amount: record.amount,
                });
            }
            if record.per_record_maximum < 0 {
                return Err(RuntimeError::InvalidExperimentRecipeMaximum {
                    maximum: record.per_record_maximum,
                });
            }
            if record.amount > record.per_record_maximum {
                return Err(RuntimeError::ExperimentRecipeAmountExceedsMaximum {
                    amount: record.amount,
                    maximum: record.per_record_maximum,
                });
            }
            if record.policy_schema_id != EXPERIMENT_RECIPE_MANA_SOURCE_POLICY_SCHEMA_V1 {
                return Err(RuntimeError::InvalidExperimentRecipePolicySchema {
                    policy_schema_id: record.policy_schema_id,
                });
            }
            if record.target_chunk.chart != self.chart_id {
                return Err(RuntimeError::InvalidExperimentRecipeTargetChart {
                    source_record_id: record.source_record_id,
                    chart: record.target_chunk.chart.raw(),
                });
            }
            if !active_chunks.contains(&record.target_chunk) {
                return Err(RuntimeError::InactiveExperimentRecipeTargetChunk {
                    source_record_id: record.source_record_id,
                    target_chunk: record.target_chunk,
                });
            }
            if u32::from(record.cell_index) >= cells_per_extent {
                return Err(RuntimeError::InvalidExperimentRecipeCellIndex {
                    source_record_id: record.source_record_id,
                    cell_index: record.cell_index,
                    cell_count: cells_per_extent,
                });
            }
            if record.enabled && record.amount != 0 {
                enabled_amount += i128::from(record.amount);
            }
        }
        if enabled_amount > i128::from(self.experiment_recipe_mana_sources.recipe_budget) {
            return Err(RuntimeError::ExperimentRecipeBudgetExceeded {
                enabled_amount,
                recipe_budget: self.experiment_recipe_mana_sources.recipe_budget,
            });
        }
        self.experiment_recipe_mana_sources
            .records
            .sort_unstable_by_key(|record| (record.scheduled_tick, record.source_record_id));
        Ok(self)
    }
}
