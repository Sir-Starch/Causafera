# Language Bootstrap

The main simulation must not begin with thousands of adults lacking language. Initial languages must result from pre-simulation historical bootstrap that generates complete linguistic systems with history, variation, and internal coherence.

## What Not to Do

Do not manually define a dictionary such as:

```text
finger = kera
water = mura
house = dom
```

This introduces English semantic labels as authoritative simulation state. It creates a fake language that is merely English in disguise.

## What the Bootstrap Must Generate

Instead, initial world synthesis must generate:

- Existing concept distributions (what concepts agents in the pre-simulation world have formed)
- Language communities (who speaks what to whom)
- Phonologies (sound systems for each language)
- Language lineages (historical relationships between languages)
- Established lexeme lineages (words with histories of use and change)
- Grammatical structures (how each language encodes communicative content)
- Writing systems where historically appropriate

## Bootstrap Method

The historical bootstrap may use lower-resolution cultural simulation and constrained causal synthesis. It does not need to simulate every conversation that ever occurred. It needs to produce plausible endpoint states that have the right structural properties:

- Lexicons with etymological depth
- Sound systems with plausible phoneme inventories
- Grammatical systems with internal coherence
- Semantic distributions with polysemy and variation
- Dialectal or register variation within communities

## Resulting Lexicon

The resulting lexicon contains internal IDs and generated forms:

```text
ConceptId 112
↔ community association
LexemeId 817
phonological form /kera/
```

`finger` is an observer gloss. It is not stored as simulation meaning. The simulation knows only that ConceptId 112 is associated with LexemeId 817 with some probability distribution across the community.

## Historical Depth

The bootstrap must produce languages with apparent historical depth:

- Old words and new words
- Regular sound changes visible in cognate sets
- Borrowings from contact languages
- Semantic drift visible in polysemy patterns
- Fossilized morphology in irregular forms

This depth is essential for making the simulation world feel historically real rather than procedurally generated.

## Related Documents

- `docs/world/historical-bootstrap.md` - General historical bootstrap strategy
- `docs/language/phonology.md` - Sound system generation
- `docs/language/lexical-innovation.md` - How new forms enter the system
- `docs/language/language-change.md` - How languages evolve over time

## Implemented foundation

Phase 13 implements a bounded structural bootstrap. An explicit seed generates opaque phonological inventories, a language ancestry tree, and lexeme form lineages with formation times. Descendant languages inherit form ancestry, but no historical people, English dictionary, or observer gloss is synthesized as authoritative state. This is a causal contract for later historical bootstrap, not a claim of complete linguistic realism.
