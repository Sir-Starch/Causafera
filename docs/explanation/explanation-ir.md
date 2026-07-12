# Explanation IR

Explanation IR is a structured intermediate representation for simulation explanations. It contains typed claims, evidence references, and causal traces. No human prose belongs in Explanation IR.

## Structure

Conceptual example:

```text
PhenomenonExplanation

subject:
    ConceptId 8172

classification:
    EMERGENT_SOCIAL_CATEGORY

display_label:
    local_lexeme 4412

origin:
    repeated perception of similar individuals

key_associations:
    rhythmic distal-hand movement
    South Canal residence
    bakery work

historical_transitions:
    physiological perception
    → occupational association
    → geographic identity
    → inherited social category

confidence:
    ...
```

## Required Capabilities

Explanation IR must support:

- typed claims;
- evidence references;
- causal trace references;
- confidence levels;
- alternative interpretations;
- temporal ranges;
- objective / local / agent perspectives.

## Perspectives

The same phenomenon may have different Explanation IR depending on perspective:

- **Objective:** Ground Truth and observer analytics
- **Local:** Community belief distribution
- **Agent-specific:** One subjective model
- **Historical:** Development over time

## Related Documents

- `docs/explanation/architecture.md` - Explanation pipeline
- `docs/explanation/deterministic-rendering.md` - Rendering IR to text
- `docs/explanation/causal-summaries.md` - Causal trace summaries
- `docs/explanation/confidence.md` - Confidence representation
