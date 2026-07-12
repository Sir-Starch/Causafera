# Semantic Drift

Semantic drift is the gradual change in the concepts agents associate with a given lexeme. It occurs because meaning is not fixed in the lexeme itself but distributed across individual agent lexicons that update with each exposure.

## Mechanisms of Semantic Drift

### Contextual Reinterpretation

Listeners infer meaning from context. When a lexeme is used in novel contexts, listeners may update their associations. Over time, the community distribution shifts.

### Metaphorical Extension

A lexeme used metaphorically may eventually lose its literal association for some speakers. The metaphor becomes the primary meaning.

### Euphemism and Taboo

Words associated with unpleasant concepts may be replaced by euphemisms, which then acquire the unpleasant associations themselves (the "euphemism treadmill").

### Social Stratification

Different social groups may use the same form with different dominant associations. Prestige-driven adoption can shift community-wide distributions.

### Contact-Induced Shift

Bilingual or multilingual agents may calque semantic structures from one language to another, altering the target language's lexical semantics.

## Documenting Semantic Drift

The observer layer may track semantic distributions over time:

```text
Lexeme /tren/

Year 144:
    Concept 8172 (rhythmic hand movement): 78%
    other: 22%

Year 181:
    Concept 8172: 45%
    Concept 9921 (bakery work): 38%
    other: 17%

Year 301:
    Concept 9921: 67%
    Concept 11204 (South Canal identity): 21%
    other: 12%
```

## Drift and Magic

Semantic drift does not directly affect mana, since mana cannot inspect semantic concepts. However, if a lexeme is used in ritual contexts, drift may alter the physical patterns produced (utterance frequency, timing, co-occurrence with other forms). These physical changes may in turn alter mana coupling.

## Related Documents

- `docs/language/semantic-layer.md` - How agent-level associations work
- `docs/language/language-change.md` - Semantic drift as one type of language change
- `docs/language/translation.md` - How translation accelerates or redirects drift
