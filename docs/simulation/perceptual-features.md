# Perceptual Features

The engine must not use a huge predefined list of semantically meaningful observations. Instead, it uses generic perceptual primitives that extract structural patterns from raw Ground Truth state.

## Generic Feature Relations

Candidate universal feature relations include:

- CHANGE
- MAGNITUDE
- DIRECTION
- VARIANCE
- PERIODICITY
- SYNCHRONY
- RECURRENCE
- DURATION
- SPATIAL_RELATION
- TEMPORAL_RELATION
- CO_OCCURRENCE
- STRUCTURAL_SIMILARITY
- RELATIVE_DIFFERENCE
- SEQUENCE_SIMILARITY

## Example Pipeline

**Ground Truth:**

```text
joint angular velocity:
+0.12
-0.09
+0.11
-0.13
...
```

**Generic extractor output:**

```text
target: perceived_substructure_19
relation: PERIODIC_CHANGE
frequency_band: 71
magnitude_band: 12
persistence: high
```

The extractor does **not** produce:

```text
TREMOR
```

## Separation of Concerns

Subjective concept formation may later group similar features into agent-specific categories. Observer analytics may produce a human gloss such as "rhythmic tremor-like movement." This gloss is not fed back into the simulation. It belongs exclusively to the observer and explanation layers.

## Implementation Notes

- Feature extraction must be attention-driven and sparse.
- Do not continuously extract features for every agent from all world state.
- Extractors operate on physically accessible state only.
- No agent has omniscient access to Ground Truth labels.

## Subjective Scene Construction

Generic features are not a complete world model. Future agent cognition must pass features through an explicit **subjective scene construction** layer that builds a transient, agent-specific model of the currently experienced situation before concept formation or belief construction. See `docs/architecture/cognition-rebaseline.md` and `docs/rfc/RFC-COG-001.md`.

## Related Documents

- `docs/simulation/emergent-concepts.md` - How agents form concepts from perceived features
- `docs/cognition/attention.md` - Attention mechanisms that drive feature extraction
- `docs/ontology/primitive-vs-emergent.md` - Primitive vs emergent distinction
- `docs/architecture/cognition-rebaseline.md` - The missing layer between features and concepts
- `docs/rfc/RFC-COG-001.md` - Proposed design for the subjective scene model
