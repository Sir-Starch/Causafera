# Communication

Communication is the process by which agents transfer information through physical utterances. It is not direct concept transmission. The listener reconstructs meaning from physical signals, and misunderstanding is a normal part of the process.

## The Communication Pipeline

Authoritative simulation process:

```text
speaker concept state
↓
semantic message construction
↓
language encoding
↓
physical utterance
↓
listener sensory acquisition
↓
phonological decoding
↓
lexical candidate recognition
↓
contextual semantic hypotheses
↓
listener interpretation
```

The listener does not receive the speaker's communicative intent directly. They receive acoustic signals and must infer what was meant.

## Speaker Intent vs Listener Interpretation

Three distinct layers must be preserved:

1. **Speaker intent** - What the speaker wanted to communicate
2. **Physical utterance** - The actual acoustic event produced
3. **Listener interpretation** - What the listener understood

Speaker intent may be retained for provenance and debugging, but the listener cannot access it directly. This separation enables:

- Honest misunderstanding
- Strategic deception
- Partial comprehension
- Creative reinterpretation

## Unknown Lexeme Handling

When a listener encounters an unfamiliar form, they generate probabilistic meaning hypotheses:

```text
/tren/

candidate meaning hypotheses:
    concept 81 → 0.31
    concept 91 → 0.46
    concept 118 → 0.38
```

Later uses update these associations. A single exposure rarely fixes meaning. Multiple exposures in varying contexts gradually shape the listener's lexical entry.

## Communicative Pressure

Not every concept requires a word. Lexical creation occurs when agents repeatedly need to communicate a distinction and no sufficiently established lexeme exists.

```text
communicative intent
↓
required concept reference
↓
no sufficiently established lexeme
↓
lexical pressure
↓
communication strategy
```

Possible strategies when lexical pressure arises:

- Description (using multiple known words)
- Composition (combining existing morphemes)
- Derivation (applying productive morphology)
- Metonymy (using a related concept's word)
- Geographic label (naming after a place)
- Occupational label (naming after a role)
- Borrowing (adopting a foreign word)
- Novel root creation (inventing a new phonological form)

The choice depends on language structure, speaker knowledge, social context, and cognitive cost.

## Strategic Communication

Communication is not assumed honest. Agents may:

- Report a belief honestly
- Exaggerate
- Conceal
- Fabricate
- Signal group membership
- Repeat information without believing it

A false claim may become socially successful. Repeated behaviour created by that claim may later affect mana. Therefore a strategic lie may eventually become physically reinforced.

## Related Documents

- `docs/language/semantic-layer.md` - How meaning is associated with forms
- `docs/language/lexical-innovation.md` - How new words are created under pressure
- `docs/cognition/strategic-communication.md` - Lying and manipulation
- `docs/language/translation.md` - Cross-linguistic communication

## Implemented boundary

Phase 13 represents the three critical layers as separate Rust types. Encoding checks a speaker-local association and emits only an ordered form and time. Decoding accepts that form, a physically acquired `PerceptId`, listener-local lexical associations, and contextual hypotheses. The listener API has no speaker-intent argument, so misunderstanding is structural rather than optional commentary.
