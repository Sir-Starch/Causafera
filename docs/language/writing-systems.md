# Writing Systems

Writing systems are technologies for persistently encoding language on physical media. They are not transparent representations of speech but have their own conventions, limitations, and evolutionary dynamics.

## Components of a Writing System

A writing system defines:

- **Physical medium** - Clay, stone, parchment, paper, metal, wood
- **Glyph inventory** - The set of marks used
- **Glyph-to-sound mapping** - How glyphs relate to phonological units
- **Orthographic conventions** - Spelling rules, word separation, directionality
- **Punctuation and formatting** - Structural markers beyond lexical content

## Types of Writing Systems

The architecture must accommodate:

- **Logographic** - Glyphs represent words or morphemes
- **Syllabic** - Glyphs represent syllables
- **Alphabetic** - Glyphs represent phonemes
- **Abugida** - Glyphs represent consonants with inherent vowels
- **Abugida-like hybrids** - Mixed systems with multiple glyph types

## Writing and Language Change

Writing systems influence language evolution:

- Orthographic conservatism may preserve older forms in writing while speech changes
- Written standards may become prestige targets that influence spoken norms
- Copying errors may introduce variants that spread or become standardized

## Document Provenance

Written documents carry provenance:

- Physical medium and its properties
- Writing system used
- Scribe or author identity
- Time and place of production
- Copying history (if a copy of an earlier document)

Documents may be:

- Copied (with possible errors)
- Damaged (affecting readability)
- Misread (producing variant interpretations)
- Normalized (edited to conform to current standards)
- Translated (mapped to another language)
- Edited (intentionally modified)
- Partially lost (some content no longer recoverable)

## Writing and Mana

Mass-distributed symbols may interact with mana. Repeated glyph patterns, especially in ritual or bureaucratic contexts, create persistent informational structures that may couple with local mana fields.

## Related Documents

- `docs/epistemics/writing.md` - Writing as epistemic practice
- `docs/epistemics/documents.md` - Document structure and transmission
- `docs/epistemics/document-lineage.md` - Document ancestry and copying history

## Phase 16 Foundation Status

Opaque `WritingSystemId` and `GlyphId` values now identify physical conventions and marks without assigning objective sound or meaning. Documents and deterministic copy edits live in `causafera-epistemics`. Glyph-to-language interpretation, inventories, orthographic learning, and reading remain future work.
