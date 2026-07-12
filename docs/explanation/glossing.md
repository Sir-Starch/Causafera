# Glossing

Glossing produces human-readable labels for simulation entities. A gloss is not a name stored in simulation state. It is an observer-side translation from internal IDs to familiar language.

## Gloss Production

Glosses may be produced from:

- observer analytical classifications;
- local lexeme forms;
- descriptive feature summaries;
- historical usage patterns.

## Example

**Simulation state:**

```text
ConceptId 8172
prototype: periodic_change on similar body substructure, frequency cluster 71
```

**Observer gloss:**

> rhythmic tremor-like movement

**UI display:**

> Tremor-like motion (confidence: 0.84)

## Gloss vs Simulation Meaning

The gloss "tremor" is not simulation state. The simulation contains:

- generic perceptual features;
- concept structures built from those features;
- lexeme associations with concepts.

The English word "tremor" is produced by the observer layer for human convenience.

## Localization

Glosses must be localizable. Changing the observer UI locale must not alter authoritative simulation state. See `docs/explanation/localization.md`.

## Related Documents

- `docs/explanation/analytical-ontology.md` - Categories that produce glosses
- `docs/explanation/classification.md` - Classification confidence
- `docs/explanation/localization.md` - Gloss localization
- `docs/language/translation.md` - Cross-language gloss mapping
