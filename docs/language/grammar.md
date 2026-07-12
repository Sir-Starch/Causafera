# Grammar

Grammar defines how languages encode communicative content into structured sequences. It operates on semantic frames and produces ordered lexical and morphological sequences that realize those frames as physical utterances.

## Semantic Frames

Internal communicative content uses structured semantic frames, not natural language prose:

```text
SpeechAct:
    type: SHARE_HYPOTHESIS

Claim:
    relation: POSSIBLE_CAUSAL_ASSOCIATION
    subject: ConceptId 8172
    object: MaterialPatternId 91

speaker_confidence: 0.43
```

This frame exists in the speaker's cognitive state before any language-specific encoding occurs.

## Grammatical Encoding

Languages encode frames according to their grammatical structures. Potential structures include:

- **Word order** - Fixed or flexible sequencing of constituents
- **Grammatical roles** - Subject, object, oblique marking
- **Tense and aspect** - Temporal location and internal temporal structure
- **Evidentiality** - Source of information (direct, reported, inferred)
- **Number** - Singular, plural, collective, or other quantity marking
- **Classification systems** - Noun classifiers, verbal classifiers, or measure words

## No Prose in Simulation

The engine does not generate natural-language prose inside authoritative simulation. It produces:

- Lexical sequences (ordered LexemeIds)
- Morphological markings (affix specifications)
- Prosodic patterns (stress, tone, boundary cues)

The observer layer renders these into human-readable text using localization resources and analytical glosses.

## Grammar as Community Convention

Grammatical structures are community-level conventions, not individual inventions. Agents acquire grammar through exposure during language acquisition. Individual agents may have incomplete or partially idiosyncratic grammars, especially for low-frequency constructions.

## Grammatical Change

Grammatical systems evolve through:

- Reanalysis of ambiguous structures
- Grammaticalization of lexical items into functional morphemes
- Constructional change (shifts in the meaning or distribution of constructions)
- Contact-induced change (borrowing of grammatical patterns)

## Related Documents

- `docs/language/morphology.md` - The morphological resources grammar uses
- `docs/language/communication.md` - How frames become utterances
- `docs/language/language-change.md` - How grammatical systems evolve
