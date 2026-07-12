# Analytical Ontology

The observer layer may contain human-designed analytical categories. These are observer classifications, not Ground Truth domain labels exposed to agents.

## Purpose

Analytical categories bridge the gap between raw simulation state and human understanding. They provide familiar labels for complex structural patterns without imposing those labels on the simulation itself.

## Example Categories

- body-part-like structure
- periodic motion
- tremor-like motion
- disease-like pattern
- social category
- occupational category
- geographic association
- practice lineage

## Classification Example

**Authoritative data:**

```text
substructure 41
attached to structure 18
distal depth 3
articulated
frequently participates in grasp actions
```

**Observer classifier:**

```text
gloss candidate:
    "finger"
confidence:
    0.96
```

The UI may display "Finger." The simulation contains no English `Finger` concept unless an actual simulated language independently produces a concept translated that way.

## Separation from Ground Truth

Observer classifications must never become authoritative simulation state. Agents do not know they are classified as having "fingers." They have their own subjective concepts built from perceptual features.

## Related Documents

- `docs/explanation/classification.md` - Classification confidence and methods
- `docs/explanation/glossing.md` - Human-readable glosses
- `docs/ontology/primitive-vs-emergent.md` - Primitive vs emergent distinction
- `docs/simulation/perceptual-features.md` - Features that feed into classification
