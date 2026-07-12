# Language Inspection

The UI supports inspecting words, their histories, and their semantic associations.

## Word Inspector

Example display:

```text
Tren
Local form: /tren/

Current usage:
Primarily a label for a South Canal occupational community.

Earlier associations:
Year 144: rhythmic hand movement
Year 181: affected bakery workers
Year 219: South Canal bakers
Year 301: hereditary occupational identity
```

## Display Principles

- The English word "Tren" is a romanized display of the local phonological form.
- Descriptions are observer explanations, not simulation state.
- The simulation does not store the English explanatory paragraph.
- All displayed text is produced by the Explanation Engine from structured IR.

## Semantic Drift Visualization

The UI may show:

- concept association weights over time;
- competing meanings in different communities;
- geographic variation in usage;
- social register associations;
- etymological chains.

## Cross-Language Comparison

Where multiple languages exist, the UI may show:

- cognate relationships;
- borrowing patterns;
- translation approximations;
- concept gaps.

## Related Documents

- `docs/language/lexicon.md` - Lexeme structure
- `docs/language/semantic-drift.md` - Semantic change mechanisms
- `docs/language/translation.md` - Cross-language mapping
- `docs/explanation/glossing.md` - Gloss production
