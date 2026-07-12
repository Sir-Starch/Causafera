# Semantic Layer

The semantic layer defines how agents connect language to meaning. It does not provide objective definitions. Instead, it models the subjective, distributed, and often inconsistent process by which agents associate lexical forms with concepts.

## No Objective Meaning Field

Do not implement:

```rust
struct Lexeme {
    meaning: ConceptId,
}
```

Meaning differs between agents. The same phonological form may activate different concepts in different speakers, or even in the same speaker on different occasions.

## Agent Lexicon Entry

Each agent maintains weighted associations between lexemes and concepts:

```text
AgentLexiconEntry

lexeme_id
semantic_associations:
    concept_a → weight
    concept_b → weight

familiarity
production_probability
register_associations
source_provenance
```

## Community Aggregation

Language analytics may aggregate usage across a community to produce distributions:

```text
Lexeme /tren/

Concept 8172: 48%
Concept 9921: 29%
Concept 553: 11%
other: 12%
```

There may be no single correct meaning. The distribution itself is the closest thing to a community-level semantic description.

## Supported Semantic Phenomena

The architecture must support future implementation of:

- **Polysemy**: One lexeme associated with multiple related concepts
- **Homonymy**: Phonologically identical forms with unrelated meanings
- **Synonym competition**: Multiple lexemes competing for the same conceptual space
- **Semantic broadening**: A lexeme expanding to cover more concepts over time
- **Semantic narrowing**: A lexeme restricting to a subset of earlier associations
- **Semantic drift**: Gradual shift in dominant associations
- **Pejoration**: Acquisition of negative connotations
- **Amelioration**: Acquisition of positive connotations

Phase 0 does not implement these mechanisms. It documents the requirements so future implementation respects the architectural foundation.

Phase 14 now implements the minimal bounded update mechanism: percept-supported exposures create or revise fixed-point concept associations in an individual lexicon. This permits divergent meanings and gradual association changes. It does not yet classify those changes as broadening, narrowing, pejoration, or other observer-level phenomena.

## Semantic Inference for Unknown Forms

When an agent encounters an unfamiliar lexeme, it generates candidate meaning hypotheses based on context, phonological similarity to known forms, and current conversational relevance. These hypotheses are probabilistic and update with subsequent exposure.

## Related Documents

- `docs/language/lexicon.md` - Lexeme structure
- `docs/language/semantic-drift.md` - How meanings change over time
- `docs/cognition/memory.md` - How agents store and update associations
