# Classification

Observer analytical classification assigns familiar labels to simulation structures with measurable confidence.

## Confidence Levels

Every classification includes confidence:

```text
finger-like body structure: 0.96
tremor-like motion: 0.84
disease-like cluster: 0.61
```

## Confidence-Aware Rendering

Rendering must respect uncertainty:

- **High confidence:** "finger"
- **Moderate confidence:** "finger-like body structure"
- **Low confidence:** "distal articulated body structure"

Do not turn uncertain analytics into confident human statements.

## Classification Methods

Classification may use:

- structural pattern matching;
- statistical clustering;
- feature similarity scoring;
- historical precedent comparison;
- multi-factor confidence combination.

## Alternative Interpretations

Where multiple classifications are plausible, the system should:

- present alternatives with confidence scores;
- explain why each alternative was considered;
- indicate where classification is genuinely ambiguous.

## Related Documents

- `docs/explanation/analytical-ontology.md` - Available analytical categories
- `docs/explanation/confidence.md` - Confidence representation in detail
- `docs/explanation/glossing.md` - Gloss production from classifications
