# World Generation Provenance

All world generation must preserve provenance. Every geographic feature must be traceable to its generation cause.

## Provenance Requirements

For every generated feature, record:

- **Generation cause**: what process created it
- **Input parameters**: what values determined its properties
- **Seed contribution**: which seed values influenced it
- **Causal dependencies**: what other features it depends on
- **Generation time**: when it was created (simulation time or generation phase)

## Provenance Chain

Example provenance for a river:

```text
WorldSeed: 0x7a3f...
    ↓
TectonicSimulation (phase: tectonic)
    ↓
MountainRange: Eastern Spine
    ↓
PrecipitationPattern: orographic rainfall
    ↓
DrainageNetwork: Eastern Basin
    ↓
River: Thorn River
    ↓
ErosionSimulation (phase: erosion)
    ↓
RiverValley: Thorn Valley
    ↓
SoilDeposition: alluvial plain
    ↓
AgriculturalSuitability: high
    ↓
Settlement: Thornford (founding cause: river crossing + fertile plain)
```

## Provenance Storage

Provenance may be stored:

- **Inline**: with the feature itself (for simple cases)
- **Separate**: in a provenance graph (for complex chains)
- **Compressed**: as generation parameters and seed (for reproducible features)

Phase 21 adds a cross-domain orchestration layer: a canonical historical stage retains target chunks, time span, numeric detail, process/parameter identity, dependencies, and external traces. Its receipt retains a result fingerprint and committed trace. Receipt validation cannot replace domain event provenance; it proves that endpoint synthesis continued the declared stage ancestry.

## Provenance and Explanation

The Explanation Engine uses provenance to answer causal questions:

> Why does Thornford exist?

The explanation traces the provenance chain from settlement founding back through agricultural suitability, soil deposition, river valley formation, to the original tectonic simulation.

## Provenance and Determinism

Provenance supports deterministic replay:

- Given the same seed and parameters, the same features must be generated
- Provenance records enable verification of determinism
- Differences in generated features must be traceable to differences in inputs

## Provenance and Causal Resolution

The Causal Resolution Field uses provenance to determine relevance:

- Features with shared provenance are likely causally related
- Distant features with independent provenance may be aggregated
- Features with complex provenance chains may require detailed simulation

## Related Documents

- `geography-philosophy.md` — geographic causality
- `spatial-hierarchy.md` — spatial organization
- `docs/explanation/causal-summaries.md` — causal explanation
- `docs/architecture/invariants.md` — INV-023: World generation has provenance

## TODO Categories

- `WORLD` — general world systems
- `TRACE` — provenance and causal tracing
