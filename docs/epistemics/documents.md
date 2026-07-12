# Documents

A document is a physical object that carries information. It is not a string containing knowledge. It has material properties, a history, and a relationship to the practices that produce and use it.

## Document Components

A document may contain:

- **Physical medium** - What it is made of and its condition
- **Glyph sequence** - The marks that encode language
- **Writing system** - The conventions for interpreting those marks
- **Lexical sequence** - The language content encoded
- **Diagrams** - Non-linguistic visual information
- **Numeric structures** - Quantitative data and tables
- **Document lineage** - Its ancestry and copying history

## Document Lifecycle

```text
production (authored or copied)
↓
use (read, referenced, cited)
↓
storage (preserved in library, archive, personal collection)
↓
degradation (physical decay, damage, loss)
↓
copying (reproduction, with possible errors)
↓
editing (intentional modification)
↓
translation (mapping to another language)
↓
loss (partial or complete destruction)
↓
archaeological residue (fragments, references, quotations)
```

## Document Transformations

Documents may be:

- **Copied** - Reproduced by hand, press, or other means
- **Damaged** - Affecting readability and completeness
- **Misread** - Producing variant interpretations
- **Normalized** - Edited to conform to current standards
- **Translated** - Mapped to another language
- **Edited** - Intentionally modified
- **Partially lost** - Some content no longer recoverable

## Copying Errors

Copying errors are a major mechanism of information change:

```text
three repetitions
↓
copying error
↓
eight repetitions
↓
institutional standardization
↓
new stable pattern
↓
mana response
```

A scribe's mistake may become institutionalized, then magically relevant.

## Document Provenance

Document provenance must support ancestry inspection:

- What earlier document was this copied from?
- What changes were made during copying?
- Who made those changes?
- When and where?

This provenance is essential for assessing reliability and for tracing the history of ideas.

## Related Documents

- `docs/epistemics/writing.md` - Writing as technology
- `docs/epistemics/document-lineage.md` - Document ancestry in detail
- `docs/language/writing-systems.md` - How language is encoded in documents

## Implemented Foundation

A document currently stores bounded opaque physical glyph IDs, medium and writing-system identities, creation time, optional parent, and transformation identity. Copying applies an explicit bounded insert/remove/replace script and returns both the child and its transformation record. Interpretation, authorship, physical degradation, and semantic document types remain future work.
