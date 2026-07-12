use crate::{SpatialHierarchy, SpatialHierarchyBuilder};

/// Authoritative world state at the Phase 3 spatial-skeleton boundary.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct World {
    hierarchy: SpatialHierarchy,
}

impl World {
    /// Construct an empty world hierarchy with explicit seed provenance.
    pub fn new(world_seed: u64) -> Self {
        Self {
            hierarchy: SpatialHierarchyBuilder::new(world_seed).finish(),
        }
    }

    /// Wrap a fully constructed hierarchy as world state.
    pub const fn from_hierarchy(hierarchy: SpatialHierarchy) -> Self {
        Self { hierarchy }
    }

    pub const fn hierarchy(&self) -> &SpatialHierarchy {
        &self.hierarchy
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SpatialLevel;

    #[test]
    fn new_world_retains_seed_and_contains_only_root() {
        let world = World::new(123);

        assert_eq!(world.hierarchy().world_seed(), 123);
        assert_eq!(world.hierarchy().len(), 1);
    }

    #[test]
    fn world_accepts_finalized_hierarchy() {
        let mut builder = SpatialHierarchyBuilder::new(55);
        builder
            .add_child(builder.root(), SpatialLevel::Landmass)
            .unwrap();
        let world = World::from_hierarchy(builder.finish());

        assert_eq!(world.hierarchy().len(), 2);
    }
}
