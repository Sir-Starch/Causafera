# RFC-LANG-002: Lexical Innovation and Semantic Mapping

**Status:** Accepted

## Summary

Lexical innovation arises from repeated unmet communicative need. Forms obey the speaker's phonological constraints, meanings remain subjective weighted associations, and adoption requires percept-supported exposure.

## Communication boundary

`CommunicativeIntent`, `PhysicalUtterance`, and `ListenerInterpretation` are distinct records. Encoding may use the speaker's private concept association. Decoding receives only the observed form, a listener percept, the listener's lexicon, and contextual candidates. It cannot inspect speaker intent or copy a speaker concept directly.

## Pressure and coinage

`PressureStore` accumulates bounded fixed-point pressure per subjective `ConceptId`. Only pressure above a numeric threshold permits coinage. Form generation is deterministic from seed, `LanguageId`, `LexemeId`, and the speaker's subjective concept ID. Coinage creates a form lineage; it does not establish community meaning.

## Adoption and semantic revision

`AdoptionHistory` retains bounded transmission records supported by `PerceptId`. Exposure revises an individual `AgentLexiconEntry` using a deterministic fixed-point moving update. Different listeners may associate the same lexeme with different concepts. Repeated contextual use can therefore produce polysemy, misunderstanding, and minimal semantic drift without a global dictionary.

## Bounds and determinism

Pressure, semantic association, interpretation candidate, use, and transmission collections have hard maxima and canonical ordering. No floating point, semantic enum, string meaning, authoritative entity ID, or nondeterministic map is used.

## Deferred work

Full social-network diffusion, prestige, borrowing, productive morphology, grammar change, realistic acoustic propagation, strategic deception policies, and cohort-scale sound change remain future work. Phase 15 practices are deliberately not part of this RFC.
