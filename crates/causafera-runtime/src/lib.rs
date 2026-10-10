// The inventory test includes an integration-test fixture by path; that file names
// this crate the way an integration test does.
#[cfg(test)]
extern crate self as causafera_runtime;

mod actors;
mod benchmark;
mod benchmark_validation;
mod bootstrap;
mod bounded_snapshot;
mod carrier;
mod config;
#[cfg(test)]
mod digest_inventory;
mod digests;
mod experiment;
mod field_raster;
#[cfg(test)]
mod history_digest_tests;
mod history_reader;
mod history_transaction;
mod hydrology;
mod hydrology_config;
mod hydrology_events;
mod hydrology_projection;
mod hydrology_undo;
mod hydrology_validation;
mod mana;
mod material_surface;
mod pattern_history;
mod population;
mod recipe_identity;
mod resolution;
pub mod runtime;
pub mod snapshot_sections;
mod snapshots;
mod terrain_regions;
mod thermal;
mod thermal_conservation_validation;
mod thermal_events;
mod tick_preimage;
mod tick_transaction;
mod tick_undo;

pub use actors::*;
pub use benchmark::*;
pub use benchmark_validation::MaterialSurfaceLoopBenchmarkError;
pub use bootstrap::*;
pub use bounded_snapshot::*;
pub use carrier::*;
pub use config::*;
pub use digests::*;
pub use experiment::*;
pub use history_transaction::{HistoryCommitPhase, RuntimeHistoryResume};
pub use hydrology::*;
pub use hydrology_config::*;
pub use hydrology_events::*;
pub(crate) use mana::*;
pub use material_surface::*;
pub use pattern_history::*;
pub use population::*;
pub use recipe_identity::runtime_recipe_fingerprint;
pub(crate) use resolution::*;
pub use runtime::*;
pub use snapshot_sections::*;
pub use snapshots::*;
pub(crate) use thermal::*;
pub(crate) use tick_transaction::*;
