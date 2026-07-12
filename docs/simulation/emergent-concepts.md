# Emergent Concepts

Agents construct concepts from perceived similarities and useful distinctions. A concept is not a developer-defined enum. It is a subjective cognitive structure built from generic perceptual features.

## Concept Structure

A concept may contain:

- prototype features
- feature weights
- exemplars
- boundary confidence
- recent activation
- social associations

## Subjectivity

Concepts need not correspond to Ground Truth categories. Different agents may construct incompatible concepts from the same population of perceived features.

**Example:**

```text
Concept 8172

prototype:
    periodic_change on similar body substructure
    frequency cluster 71
    geographic association
    repeated association with practice lineage 19
```

A physician may heavily weight physiological motion patterns. A tax official may heavily weight district and occupation. Both construct valid but different concepts from the same underlying population.

## Attention-Driven Formation

Concept formation must be attention-driven and sparse. Do not continuously cluster all world features for every agent. Formation occurs when:

- an agent repeatedly encounters similar perceptual features;
- communicative pressure requires distinguishing a category;
- a feature pattern proves predictively useful;
- social transmission introduces a concept from another agent.

## Concept Evolution

Concepts are not static. They may:

- shift prototypes as new exemplars accumulate;
- split when subpopulations diverge;
- merge when distinct feature sets overlap;
- lose activation and fade;
- acquire social associations through communication.

## Related Documents

- `docs/simulation/perceptual-features.md` - Generic perceptual primitives
- `docs/language/semantic-layer.md` - How concepts connect to language
- `docs/cognition/memory.md` - Memory structures that hold concepts
- `docs/cognition/prediction.md` - Predictive utility driving concept formation

## RFC

- `RFC-CONCEPT-001: Sparse Subjective Concept Formation`

## Phase 11 Implementation Status

`ConceptStore` retains at most 32 agent-local prototypes and accepts at most 32 explicitly attended `ConceptObservation` values per update. Inputs contain only a quantized appearance signature, numeric salience and predictive utility, and subjective `PerceptId` support. Matching, integer running-mean revision, activation decay, allocation, and active-concept ranking are deterministic.

The minimal store does not continuously cluster world state, name categories, inspect authoritative identity, or claim objective correctness. Prototype split/merge, social transmission, consolidation, richer exemplars, and durable storage remain future work.
