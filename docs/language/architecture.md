# Language Architecture

The language subsystem lives in the `ontopolis-language` crate. It simulates how agents construct, transmit, and decode meaning through physical utterances and written forms.

## Core Principle

A word is not a string with an objective meaning. Language is a distributed system where individual agents maintain subjective associations between lexical forms and their own conceptual structures. The subsystem must never store English glosses as authoritative simulation state.

## Separation of Concerns

The language subsystem separates the following layers:

- **Subjective concepts** - Agent-internal conceptual structures formed from perceptual experience
- **Lexical items** - Socially transmitted linguistic form lineages (lexemes)
- **Phonological forms** - Physical sound patterns that realize lexical items
- **Morphological constructions** - Compositional patterns for building complex word forms
- **Grammatical encoding** - Structural frames that organize communicative content
- **Physical utterances** - Actual acoustic events produced by speakers
- **Written forms** - Persistent glyph sequences on physical media

Each layer is independently addressable and modifiable. A change in phonology (such as a sound shift) does not require rewriting conceptual structures. A new concept does not automatically create a lexical entry.

## Agent Lexicon Model

Individual agents maintain their own lexicon entries. An entry tracks:

- Which lexeme the agent knows
- What concepts the agent associates with that lexeme (with weights)
- Familiarity and production probability
- Register and social associations
- Source provenance (where the agent learned it)

Community-level language analytics may aggregate these individual distributions, but there is no single authoritative dictionary stored in simulation state.

## Language Communities

Languages are not global objects. They exist as patterns of shared practice within communities. Agents may belong to multiple overlapping speech communities, each with distinct phonologies, lexicons, and grammatical conventions.

## Interaction with Other Domains

- **Cognition**: Concept formation drives lexical need; language exposure shapes concept boundaries
- **Mana**: Physical utterances create acoustic patterns that may couple with local mana fields
- **Epistemics**: Documents preserve and transmit linguistic forms across time and space
- **Isekai**: Transferred agents bring foreign conceptual and lexical priors

## Invariants

- INV-LANG-001: Simulation has no privileged human interface language
- INV-008: Language decoding does not directly transfer speaker concepts

## Related Documents

- `docs/language/semantic-layer.md` - How meaning is represented
- `docs/language/lexicon.md` - Lexeme structure and social transmission
- `docs/language/phonology.md` - Sound systems and constraints
- `docs/language/grammar.md` - Communicative frames and encoding
- `docs/language/communication.md` - The full communication pipeline
- `docs/cognition/strategic-communication.md` - Lying and communicative intent
