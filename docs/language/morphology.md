# Morphology

Morphology defines how languages build complex words from simpler units. It operates between the phonological form and the grammatical encoding, providing the compositional machinery that lets languages express complex meanings without requiring a separate lexeme for every distinction.

## Morphological Units

The morphology layer recognizes several unit types:

- **Roots** - Core lexical morphemes carrying primary semantic content
- **Affixes** - Bound morphemes modifying root meaning (prefixes, suffixes, infixes, circumfixes)
- **Stems** - Roots plus required derivational morphology
- **Word forms** - Fully inflected units ready for syntactic use

## Productive Processes

Languages may define productive morphological processes:

- **Derivation** - Creating new lexemes from existing ones (nominalization, verbalization)
- **Inflection** - Marking grammatical categories on existing lexemes (tense, number, case)
- **Compounding** - Combining independent lexemes into complex words
- **Reduplication** - Repeating all or part of a morpheme for semantic or grammatical effect
- **Incorporation** - Combining multiple lexical elements into a single word form

## Morphological Change

Morphological systems evolve. The architecture must support future implementation of:

```text
frequent lexical sequence
↓
phonological reduction
↓
boundary weakening
↓
reanalysis
↓
new productive morpheme
```

This grammaticalization pathway is a major mechanism of language change. Today's independent word may become tomorrow's affix.

## Morphophonology

Morphological combination may trigger phonological processes:

- Assimilation across morpheme boundaries
- Vowel reduction in unstressed syllables
- Consonant cluster simplification
- Epenthesis to break illegal clusters

These processes create allomorphy: the same morpheme appearing in different phonological forms depending on context.

## Constraints on Complexity

Morphological systems vary in complexity. Some languages are predominantly isolating (little morphology), others are highly synthetic (extensive affixation). The architecture must accommodate this variation without privileging any particular type.

## Related Documents

- `docs/language/phonology.md` - The sound system that morphology operates within
- `docs/language/grammar.md` - How morphologically complex words enter syntactic structures
- `docs/language/language-change.md` - Grammaticalization and morphological evolution
