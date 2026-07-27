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
    pub active_chunk_shape: ActiveChunkShape,
    pub chart_id: SpatialChartId,
    pub pattern_schedule: PhysicalPatternSchedule,
    pub mana_parameters: ManaParameters,
    pub carrier_adapter: CarrierAdapterConfig,
    pub terrain_participation: TerrainParticipation,
    pub actor_count: u8,
    pub sensor_count: u8,
    pub action_bounds: i64,
    pub bootstrap_population: u64,
    pub material_surface_signals_enabled: bool,
    pub experiment_recipe_mana_sources: ExperimentRecipeManaSourceRecipe,
}

/// The shape the active chunk set takes around the chart origin.
///
/// Widening the set changes which chunks exist, and therefore changes state
/// hashes by construction. `Line` is what every recorded fixture and replay was
/// produced against and stays the default; a session that wants a map with two
/// dimensions asks for `Area` explicitly.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ActiveChunkShape {
    /// `2 * radius + 1` chunks along x, with y and z pinned to zero.
    #[default]
    Line,
    /// The square block of `(2 * radius + 1)²` chunks in the z = 0 plane.
    ///
    /// A Euclidean disc was considered and rejected: at radius 1 it is a
    /// five-chunk cross, which is a worse chart than the nine-chunk block the
    /// observer's own bounds were already written for.
    Area,
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

/// Whether the terrain carrier reaches the tick loop.
///
/// Terrain is authoritative world state. A carrier that is generated, persisted
/// and projected but never causally consumed is world content that exists
/// without participating, so the participation is a stated contract rather than
/// an accident of which system happens to read `carrier_adapters`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TerrainParticipation {
    /// The carrier presents its standing structure to the physical pattern
    /// stream on every tick the pattern schedule emits.
    ///
    /// Terrain does not change in this milestone, so what varies between ticks
    /// is not the structure but the field's response to it. The carrier is
    /// still a standing physical presence, and the mana model's spatial and
    /// recurrence channels exist to answer exactly that.
    #[default]
    Standing,
    /// The carrier never reaches the tick loop. Terrain is then generated,
    /// persisted and projected but causally inert, which is the behaviour
    /// recorded in `TODO-RUNTIME-002` and is retained only so a configuration
    /// can isolate the rest of the loop from terrain.
    Inert,
}

impl RuntimeConfig {
    pub fn new(world_seed: u64) -> Self {
        Self {
            deterministic: DeterministicConfig::new(world_seed),
            chunk_extent: 3,
            active_chunk_radius: 1,
            active_chunk_shape: ActiveChunkShape::Line,
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
                effect_threshold: 6_144,
                effect_hysteresis: 1_536,
            },
            carrier_adapter: CarrierAdapterConfig::terrain_seed(world_seed),
            terrain_participation: TerrainParticipation::Standing,
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
        // The per-field bounds above compose into configurations the runtime
        // cannot execute: cognition caps how many cues one actor may be handed
        // per tick, and perception assembles that batch from every promoted
        // actor's object plus every contacted material surface, once per sensor
        // aperture. Rejecting here fails at construction instead of part-way
        // through a run that already paid for bootstrap and warm-up ticks.
        let worst_case = worst_case_scene_cue_count(&self);
        if worst_case > MAX_RUNNABLE_SCENE_CUES {
            return Err(RuntimeError::SceneCueBudgetExceeded {
                worst_case,
                maximum: MAX_RUNNABLE_SCENE_CUES,
                sensor_count: self.sensor_count,
                actor_count: self.actor_count,
                surface_count: worst_case_contacted_surface_count(&self),
            });
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
        let active_chunks = active_chunk_keys(
            self.chart_id,
            self.active_chunk_radius,
            self.active_chunk_shape,
        );
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

#[cfg(test)]
mod tests {
    use super::*;

    fn candidate(
        actor_count: u8,
        sensor_count: u8,
        radius: u8,
        shape: ActiveChunkShape,
    ) -> RuntimeConfig {
        let mut config = RuntimeConfig::new(7);
        config.chunk_extent = 3;
        config.active_chunk_radius = radius;
        config.active_chunk_shape = shape;
        config.actor_count = actor_count;
        config.sensor_count = sensor_count;
        config.bootstrap_population = 512;
        config
    }

    #[test]
    fn validation_rejects_every_configuration_the_boundary_sweep_found_unrunnable() {
        // Each pair is a `sensor_count` and the lowest `actor_count` that failed
        // for it, as located by the Wave 1 harness's exhaustive sweep at
        // `active_chunk_radius = 0`. The bound has to reject at least these, or
        // it readmits the defect it exists to close. It rejects strictly more:
        // beyond roughly eight sensors an aperture stops seeing every signal, so
        // configurations such as 4 actors with 16 sensors run today even though
        // their worst case does not fit. Rejecting those is deliberate, because
        // nothing about the configuration keeps a longer run inside the cap.
        for (sensor_count, actor_count) in [(1u8, 64u8), (2, 32), (4, 16), (8, 8), (16, 5)] {
            let error = candidate(actor_count, sensor_count, 0, ActiveChunkShape::Line)
                .validate()
                .expect_err("the sweep observed this configuration failing at tick 5");
            assert!(
                matches!(error, RuntimeError::SceneCueBudgetExceeded { .. }),
                "{actor_count} actors with {sensor_count} sensors should be rejected for its cue \
                 budget, got {error}"
            );
        }
    }

    #[test]
    fn validation_accepts_the_largest_configuration_the_cue_cap_admits() {
        for (sensor_count, actor_count) in [(1u8, 63u8), (2, 31), (4, 15), (8, 7), (16, 3)] {
            let accepted = candidate(actor_count, sensor_count, 0, ActiveChunkShape::Line);
            assert_eq!(
                worst_case_scene_cue_count(&accepted),
                MAX_RUNNABLE_SCENE_CUES,
                "{actor_count} actors with {sensor_count} sensors should sit exactly on the bound"
            );
            accepted
                .validate()
                .expect("a configuration exactly on the bound is runnable");
        }
    }

    #[test]
    fn runtime_construction_rejects_an_unrunnable_configuration() {
        let Err(error) = Runtime::new(candidate(64, 1, 0, ActiveChunkShape::Line)) else {
            panic!("64 actors with one sensor cannot execute a tick, so construction must fail");
        };
        assert!(
            matches!(
                error,
                RuntimeError::SceneCueBudgetExceeded {
                    worst_case: 65,
                    maximum: 64,
                    ..
                }
            ),
            "construction should fail with the cue budget error, got {error}"
        );
    }

    #[test]
    fn runtime_on_the_cue_budget_boundary_constructs_and_ticks() {
        let mut runtime = Runtime::new(candidate(63, 1, 0, ActiveChunkShape::Line))
            .expect("a configuration exactly on the bound constructs");
        runtime
            .run_ticks(8)
            .expect("a configuration exactly on the bound ticks");
    }

    #[test]
    fn widening_the_active_chunk_set_spends_the_same_cue_budget() {
        // Every active chunk is bootstrapped with a material surface, so a wider
        // chart raises the worst-case cue count even though `actor_count` and
        // `sensor_count` are unchanged.
        candidate(8, 2, 1, ActiveChunkShape::Area)
            .validate()
            .expect("nine chunks leave 8 actors with 2 sensors inside the cap");
        let error = candidate(8, 2, 2, ActiveChunkShape::Area)
            .validate()
            .expect_err("twenty-five chunks push the same actors past the cap");
        assert!(
            matches!(
                error,
                RuntimeError::SceneCueBudgetExceeded {
                    worst_case: 66,
                    surface_count: 25,
                    ..
                }
            ),
            "expected the surface term to carry the rejection, got {error}"
        );
    }

    #[test]
    fn disabling_material_surface_signals_removes_the_surface_term() {
        let mut config = candidate(8, 2, 4, ActiveChunkShape::Area);
        assert!(
            config.clone().validate().is_err(),
            "eighty-one surfaces should not fit alongside 8 actors on 2 sensors"
        );
        config.material_surface_signals_enabled = false;
        assert_eq!(worst_case_scene_cue_count(&config), 16);
        config
            .validate()
            .expect("with surface signals off only the actor term is spent");
    }
}
