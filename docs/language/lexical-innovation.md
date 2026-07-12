# Lexical Innovation

Lexical innovation is the process by which new lexemes enter a language. It is driven by communicative pressure: when agents repeatedly need to express a distinction and lack an established form, they create or adopt one.

## Constraints on Novel Forms

Novel lexeme forms must respect the language's phonological constraints. Each language defines:

- Phoneme inventory
- Syllable constraints
- Phonotactics
- Stress behaviour
- Productive morphology

A phonotactic generator produces candidate forms that satisfy these constraints.

## Deterministic Generation

New root generation is deterministic in strict mode. Possible key inputs:

```text
world_seed
language_id
speaker_id
concept_id
coinage_event_id
```

Given the same inputs, the generator produces the same form. This preserves determinism while allowing rich variation across different speakers, concepts, and languages.

## Example Generation

```text
allowed onset: t, k, m, n, r, s, v, tr, kr
vowels: a, e, i, o
codas: n, r, s, m
```

Candidate output: `/tren/`

## Initial Adoption

Only the originating speaker initially possesses the association between the new form and their concept. Community adoption occurs through subsequent communication. The new lexeme spreads through the social network or fails to catch on and dies out.

## Innovation Strategies

When communicative pressure arises, speakers may choose among multiple strategies:

1. **Novel root creation** - Generate a new phonological form
2. **Semantic extension** - Broaden an existing lexeme to cover the new concept
3. **Compounding** - Combine existing lexemes
4. **Derivation** - Apply productive morphology to an existing root
5. **Borrowing** - Adopt a form from another language or dialect
6. **Metonymy** - Use a related concept's word

The choice depends on cognitive cost, social context, and language-specific resources.

## Related Documents

- `docs/language/phonology.md` - The sound system constraining innovation
- `docs/language/communication.md` - Communicative pressure that drives innovation
- `docs/language/lexicon.md` - How new lexemes enter the community lexicon
