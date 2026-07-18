# Data-Oriented Storage

Causafera prefers data-oriented storage patterns that optimize for cache locality, dense iteration, and deterministic batch execution.

## Preferred Patterns

- **Dense arrays** - contiguous memory for hot data
- **Structure-of-arrays** - separate arrays per field rather than array-of-structures
- **Typed IDs** - compact, type-safe identifiers instead of pointers or UUIDs
- **Packed numeric state** - fixed-size numeric representations
- **Sparse side stores** - rarely accessed data in separate sparse structures

## What to Avoid

- Giant object graphs with pointer chasing
- Per-agent strings in hot simulation paths
- UUIDs in hot simulation paths
- Heap-allocated collections per entity
- Dynamic dispatch in tight loops

## Typed IDs

The engine uses typed IDs for all entity references:

```text
AgentId
BodyId
BodySegmentId
EventId
TraceId
FeatureId
PerceptId
ConceptId
LexemeId
LanguageId
PracticeId
DocumentId
OrganizationId
PlaceId
ChunkId
FormationId
MaterialId
PathogenId
PopulationLineageId
AggregateId
```

Typed IDs provide compile-time separation between different entity kinds. They are compact and cache-friendly. They do not carry vtable or allocation overhead.

Do not use UUIDs in hot simulation paths. UUIDs may be used at persistence boundaries or observer protocol boundaries where external interoperability is required.

## Memory Layout

Hot simulation data should be organized for sequential access:

- agents with similar update patterns near each other
- fields accessed together in the same cache line
- cold data moved to sparse stores
- boolean flags packed into bitsets

## Determinism and Data Layout

Data layout choices must not compromise determinism. Structure-of-arrays iteration order must be stable. Sparse store traversal must use deterministic ordering.

## Performance Implications

Data-oriented storage is not premature optimization. It is an architectural requirement because:

- agent count scales with memory efficiency
- perceptual feature processing requires dense iteration
- causal edge emission requires sequential writes
- deterministic batch execution requires predictable memory access

## Example

Instead of:

```rust
struct Agent {
    name: String,
    position: Vec3,
    health: f32,
    inventory: Vec<Item>,
    concepts: HashMap<ConceptId, f32>,
}
```

Prefer:

```rust
struct AgentPositions { data: Vec<Vec3> }
struct AgentHealths { data: Vec<f32> }
struct AgentInventories { sparse: SparseVec<Vec<ItemId>> }
```

The exact implementation will vary by domain and phase. The principle remains: separate hot dense data from cold sparse data.
