//! The identity of the world a runtime was configured to simulate.
//!
//! Part of the experiment digest (digest schema 9) and of every bounded
//! snapshot header. It names the seed, the configuration and the compiled
//! system registry, and nothing that changes while the world runs.

use causafera_types::SimulationTime;

use crate::runtime::runtime_system_registrations;
use crate::snapshot_sections::encode_runtime_recipe_section;
use crate::{RuntimeConfig, RuntimeRecipeSnapshot};

const RUNTIME_RECIPE_FINGERPRINT_DOMAIN: &[u8] = b"causafera/runtime-recipe-fingerprint/v1";

/// BLAKE3 of the canonical recipe section, with the completed time pinned to
/// zero so the fingerprint names the world rather than how far it has run.
pub fn runtime_recipe_fingerprint(config: &RuntimeConfig) -> [u8; 32] {
    let recipe = RuntimeRecipeSnapshot {
        seed: config.deterministic.world_seed,
        config: config.clone(),
        system_registrations: runtime_system_registrations(),
        completed_time: SimulationTime::new(0),
    };
    let mut hasher = blake3::Hasher::new();
    hasher.update(RUNTIME_RECIPE_FINGERPRINT_DOMAIN);
    hasher.update(&encode_runtime_recipe_section(&recipe));
    *hasher.finalize().as_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fingerprint_names_the_seed_and_ignores_nothing_else_it_encodes() {
        let first = runtime_recipe_fingerprint(&RuntimeConfig::new(7));
        assert_eq!(first, runtime_recipe_fingerprint(&RuntimeConfig::new(7)));
        assert_ne!(first, runtime_recipe_fingerprint(&RuntimeConfig::new(8)));
        let mut widened = RuntimeConfig::new(7);
        widened.chunk_extent += 1;
        assert_ne!(first, runtime_recipe_fingerprint(&widened));
    }
}
