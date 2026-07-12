# Confidence

Every analytical classification and explanation claim includes confidence. Confidence is not a binary flag. It is a continuous measure that must be preserved through the explanation pipeline.

## Confidence Representation

Confidence may be represented as:

- scalar values (0.0 to 1.0);
- qualitative bands (high, moderate, low, uncertain);
- multi-dimensional vectors (separate confidence in classification, origin, associations, etc.).

## Confidence in Rendering

Rendering must adapt to confidence levels:

**High confidence (0.9+):**

> The effect is strongly associated with porous volcanic material.

**Moderate confidence (0.6-0.9):**

> The effect appears associated with a specific volcanic material.

**Low confidence (0.3-0.6):**

> There may be an association with local geological materials.

**Uncertain (<0.3):**

> The causes of this effect are not well understood.

## Confidence Sources

Confidence may derive from:

- pattern match strength;
- evidence quantity;
- evidence quality;
- alternative hypothesis competitiveness;
- historical stability of the classification;
- expert agreement (if applicable).

## Related Documents

- `docs/explanation/classification.md` - Classification confidence
- `docs/explanation/explanation-ir.md` - IR confidence fields
- `docs/explanation/deterministic-rendering.md` - Confidence-aware rendering
