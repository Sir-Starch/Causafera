use std::error::Error;
use std::fmt;

use ontopolis_types::{ChunkId, PlaceId};

/// Objective containment levels in the physical world hierarchy.
///
/// These levels describe authoritative spatial structure. Political regions,
/// ownership claims, observer labels, and causal resolution are separate
/// overlays and are deliberately absent here.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum SpatialLevel {
    World,
    Landmass,
    Basin,
    Region,
    Territory,
    Chunk,
    Parcel,
    Structure,
    InteriorSpace,
}

impl SpatialLevel {
    /// Return the only valid direct child level.
    pub const fn child(self) -> Option<Self> {
        match self {
            Self::World => Some(Self::Landmass),
            Self::Landmass => Some(Self::Basin),
            Self::Basin => Some(Self::Region),
            Self::Region => Some(Self::Territory),
            Self::Territory => Some(Self::Chunk),
            Self::Chunk => Some(Self::Parcel),
            Self::Parcel => Some(Self::Structure),
            Self::Structure => Some(Self::InteriorSpace),
            Self::InteriorSpace => None,
        }
    }
}

/// Failure while constructing a spatial hierarchy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HierarchyError {
    UnknownParent(PlaceId),
    InvalidTransition {
        parent: SpatialLevel,
        child: SpatialLevel,
    },
    CapacityExceeded,
}

impl fmt::Display for HierarchyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownParent(parent) => write!(formatter, "unknown parent place {parent}"),
            Self::InvalidTransition { parent, child } => {
                write!(
                    formatter,
                    "invalid spatial transition {parent:?} -> {child:?}"
                )
            }
            Self::CapacityExceeded => formatter.write_str("spatial hierarchy capacity exceeded"),
        }
    }
}

impl Error for HierarchyError {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ChildRange {
    start: u32,
    len: u32,
}

/// One immutable node in the spatial containment hierarchy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SpatialNode {
    id: PlaceId,
    level: SpatialLevel,
    parent: Option<PlaceId>,
    children: ChildRange,
}

impl SpatialNode {
    pub const fn id(&self) -> PlaceId {
        self.id
    }

    pub const fn level(&self) -> SpatialLevel {
        self.level
    }

    pub const fn parent(&self) -> Option<PlaceId> {
        self.parent
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PendingNode {
    id: PlaceId,
    level: SpatialLevel,
    parent: Option<PlaceId>,
}

/// Deterministic construction surface for a spatial hierarchy.
///
/// Place IDs are assigned in insertion order. Finalization preserves that
/// order while grouping child references into contiguous ranges.
#[derive(Clone, Debug)]
pub struct SpatialHierarchyBuilder {
    world_seed: u64,
    nodes: Vec<PendingNode>,
}

impl SpatialHierarchyBuilder {
    pub fn new(world_seed: u64) -> Self {
        Self {
            world_seed,
            nodes: vec![PendingNode {
                id: PlaceId::new(0),
                level: SpatialLevel::World,
                parent: None,
            }],
        }
    }

    pub const fn root(&self) -> PlaceId {
        PlaceId::new(0)
    }

    /// Add a child after validating the documented containment sequence.
    pub fn add_child(
        &mut self,
        parent: PlaceId,
        level: SpatialLevel,
    ) -> Result<PlaceId, HierarchyError> {
        let parent_index = usize::try_from(parent.raw())
            .ok()
            .filter(|index| *index < self.nodes.len())
            .ok_or(HierarchyError::UnknownParent(parent))?;
        let parent_level = self.nodes[parent_index].level;

        if parent_level.child() != Some(level) {
            return Err(HierarchyError::InvalidTransition {
                parent: parent_level,
                child: level,
            });
        }
        if self.nodes.len() >= u32::MAX as usize {
            return Err(HierarchyError::CapacityExceeded);
        }

        let id = PlaceId::new(self.nodes.len() as u64);
        self.nodes.push(PendingNode {
            id,
            level,
            parent: Some(parent),
        });
        Ok(id)
    }

    /// Finalize into immutable dense node and child arrays.
    pub fn finish(self) -> SpatialHierarchy {
        let mut child_counts = vec![0_u32; self.nodes.len()];
        for node in self.nodes.iter().skip(1) {
            let parent = node.parent.expect("non-root nodes always have a parent");
            child_counts[parent.raw() as usize] += 1;
        }

        let mut child_offsets = vec![0_u32; self.nodes.len()];
        let mut offset = 0_u32;
        for (index, count) in child_counts.iter().copied().enumerate() {
            child_offsets[index] = offset;
            offset += count;
        }

        let mut cursors = child_offsets.clone();
        let mut child_ids = vec![PlaceId::new(0); self.nodes.len().saturating_sub(1)];
        for node in self.nodes.iter().skip(1) {
            let parent_index = node.parent.expect("validated parent").raw() as usize;
            let child_index = cursors[parent_index] as usize;
            child_ids[child_index] = node.id;
            cursors[parent_index] += 1;
        }

        let nodes = self
            .nodes
            .into_iter()
            .enumerate()
            .map(|(index, node)| SpatialNode {
                id: node.id,
                level: node.level,
                parent: node.parent,
                children: ChildRange {
                    start: child_offsets[index],
                    len: child_counts[index],
                },
            })
            .collect();

        SpatialHierarchy {
            world_seed: self.world_seed,
            nodes,
            child_ids,
        }
    }
}

/// Immutable authoritative spatial containment hierarchy.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpatialHierarchy {
    world_seed: u64,
    nodes: Vec<SpatialNode>,
    child_ids: Vec<PlaceId>,
}

impl SpatialHierarchy {
    pub const fn world_seed(&self) -> u64 {
        self.world_seed
    }

    pub const fn root(&self) -> PlaceId {
        PlaceId::new(0)
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    pub fn node(&self, id: PlaceId) -> Option<&SpatialNode> {
        usize::try_from(id.raw())
            .ok()
            .and_then(|index| self.nodes.get(index))
    }

    /// Return a direct slice over the node's children in insertion order.
    pub fn children(&self, id: PlaceId) -> Option<&[PlaceId]> {
        let range = self.node(id)?.children;
        let start = range.start as usize;
        let end = start + range.len as usize;
        Some(&self.child_ids[start..end])
    }

    /// Convert a hierarchy place to a chunk identity only when it is a chunk.
    pub fn chunk_id(&self, id: PlaceId) -> Option<ChunkId> {
        (self.node(id)?.level == SpatialLevel::Chunk).then(|| ChunkId::new(id.raw()))
    }

    /// Resolve a chunk identity back to its hierarchy place.
    pub fn place_for_chunk(&self, id: ChunkId) -> Option<PlaceId> {
        let place = PlaceId::new(id.raw());
        (self.node(place)?.level == SpatialLevel::Chunk).then_some(place)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn complete_hierarchy(seed: u64) -> (SpatialHierarchy, Vec<PlaceId>) {
        let mut builder = SpatialHierarchyBuilder::new(seed);
        let mut places = vec![builder.root()];
        for level in [
            SpatialLevel::Landmass,
            SpatialLevel::Basin,
            SpatialLevel::Region,
            SpatialLevel::Territory,
            SpatialLevel::Chunk,
            SpatialLevel::Parcel,
            SpatialLevel::Structure,
            SpatialLevel::InteriorSpace,
        ] {
            places.push(builder.add_child(*places.last().unwrap(), level).unwrap());
        }
        (builder.finish(), places)
    }

    #[test]
    fn complete_chain_has_valid_parents_and_children() {
        let (hierarchy, places) = complete_hierarchy(42);

        assert_eq!(hierarchy.len(), 9);
        assert!(!hierarchy.is_empty());
        for pair in places.windows(2) {
            assert_eq!(hierarchy.node(pair[1]).unwrap().parent(), Some(pair[0]));
            assert_eq!(hierarchy.children(pair[0]), Some(&pair[1..2]));
        }
        assert_eq!(hierarchy.children(*places.last().unwrap()), Some(&[][..]));
    }

    #[test]
    fn children_preserve_insertion_order_when_construction_is_interleaved() {
        let mut builder = SpatialHierarchyBuilder::new(7);
        let root = builder.root();
        let first = builder.add_child(root, SpatialLevel::Landmass).unwrap();
        builder.add_child(first, SpatialLevel::Basin).unwrap();
        let second = builder.add_child(root, SpatialLevel::Landmass).unwrap();
        let hierarchy = builder.finish();

        assert_eq!(hierarchy.children(root), Some(&[first, second][..]));
    }

    #[test]
    fn invalid_level_transition_is_rejected() {
        let mut builder = SpatialHierarchyBuilder::new(0);

        assert_eq!(
            builder.add_child(builder.root(), SpatialLevel::Chunk),
            Err(HierarchyError::InvalidTransition {
                parent: SpatialLevel::World,
                child: SpatialLevel::Chunk,
            })
        );
    }

    #[test]
    fn unknown_parent_is_rejected() {
        let mut builder = SpatialHierarchyBuilder::new(0);

        assert_eq!(
            builder.add_child(PlaceId::new(99), SpatialLevel::Landmass),
            Err(HierarchyError::UnknownParent(PlaceId::new(99)))
        );
    }

    #[test]
    fn identical_inputs_produce_identical_hierarchies() {
        assert_eq!(complete_hierarchy(91).0, complete_hierarchy(91).0);
    }

    #[test]
    fn chunk_identity_conversion_is_level_checked() {
        let (hierarchy, places) = complete_hierarchy(3);
        let chunk = places[5];

        assert_eq!(hierarchy.chunk_id(chunk), Some(ChunkId::new(chunk.raw())));
        assert_eq!(
            hierarchy.place_for_chunk(ChunkId::new(chunk.raw())),
            Some(chunk)
        );
        assert_eq!(hierarchy.chunk_id(places[4]), None);
        assert_eq!(
            hierarchy.place_for_chunk(ChunkId::new(places[4].raw())),
            None
        );
    }
}
