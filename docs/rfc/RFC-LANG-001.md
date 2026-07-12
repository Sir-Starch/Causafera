# RFC-LANG-001: Historical Language Bootstrap

**Status:** Accepted

## Summary

Initial language state is synthesized deterministically as bounded language and lexeme lineages. It contains opaque phonological units, ordered forms, ancestry, and formation time, never manually authored dictionaries or human glosses.

## Decision

`LanguageBootstrap::generate` accepts an explicit seed and bounded counts. It creates a canonical lineage tree, per-lineage `PhonemeInventory`, and inherited `LexemeLineage` records. A lexeme contains a form and history, not a `ConceptId` meaning. Individual agents' associations live separately in `AgentLexiconEntry`.

Phonological units are opaque numeric categories. Observer rendering may map them to IPA or convenient glyphs, but those renderings are not simulation state. The minimal phonotactic contract partitions a sorted inventory into onset, nucleus, and optional coda ranges and generates compact valid forms using integer seed mixing.

## Resolution

Bootstrap is structural and lower-resolution: it creates lineage endpoints and inherited forms without simulating fake historical speakers or every utterance. Rich sound change, contact, grammar, and historical population synthesis remain future extensions over the same ancestry records.

## Determinism and bounds

Language and lexeme counts are capped; unit inventories and forms have hard maxima. Generation uses only seed, opaque IDs, ordinals, integer mixing, and canonical ordering. Locale, strings, hash iteration, system entropy, and floating point cannot affect results.

## Consequences

- Initial languages have inspectable ancestry without English-in-disguise dictionaries.
- Meanings can disagree across speakers because lexemes have no objective meaning field.
- Historical plausibility is not claimed until richer synthesis and benchmarks exist.
- Writing, morphology, grammar evolution, and physical acoustics remain outside Phase 13.
