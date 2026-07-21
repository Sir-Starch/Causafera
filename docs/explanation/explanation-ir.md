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

## Bounded material-surface loop claim

The active actor/material/mana slice supplies `MaterialSurfaceLoopClaim` as a live, read-only
Explanation input. It records typed condition bounds, field-total context, an observation window,
the actor-contact trace, an optional mana-effect trace, and optional in-world mana transition
values. Its deterministic claims distinguish supported evidence from explicit insufficiency and
include control/window schemas plus `MATERIAL_SURFACE_LOOP_MANA_TRANSITION_SCHEMA` (opaque schema
ID 14). That claim is `Supported` only when a mana-transition trace and its before/after values are
available; otherwise it is `Unknown`. Explanation may cite source-derived in-world ancestry, but
it never renders why an operator selected a source, its recipe identity, or its policy.

The live input reads only the bounded retained material-transition window. Retention is
deterministic and protects the newest mana-mediated transition when ordinary contacts would
otherwise fill the window; when contact ancestry is absent, the claim remains explicitly
insufficient rather than inventing a causal anchor.

## Related Documents

- `docs/explanation/architecture.md` - Explanation pipeline
- `docs/explanation/deterministic-rendering.md` - Rendering IR to text
- `docs/explanation/causal-summaries.md` - Causal trace summaries
- `docs/explanation/confidence.md` - Confidence representation
