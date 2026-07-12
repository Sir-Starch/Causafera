# Document Lineage

Document lineage tracks the ancestry, descent, and modification history of documents. It enables provenance inspection, error tracing, and the reconstruction of how information has changed as it propagated through time and space.

## Why Lineage Matters

A document is not an isolated object. It exists in a network of copies, translations, and adaptations:

```text
Original treatise on metallurgy (Year 234)
    ↓
Copied by Scribe A (Year 267, with minor errors)
    ↓
Copied by Scribe B (Year 289, with additional errors)
    ↓
Translated into coastal dialect (Year 312, with conceptual approximations)
    ↓
Edited by Guild Master (Year 345, with updated practices)
    ↓
Copied by Monastery C (Year 401, with ritual annotations added)
```

Without lineage, it is impossible to know which version is most reliable, what changes were introduced when, or how errors accumulated.

## Lineage Structure

Document lineage is a directed graph:

- **Nodes** are individual document instances
- **Edges** are copying, translation, or editing relationships
- **Edge labels** indicate the type of transformation and its context

Each edge records:

- Source document
- Target document
- Transformation type (copy, translation, edit, compilation)
- Agent responsible
- Time and place
- Known changes (if recorded)

## Copying Error Tracking

Copying errors are particularly important to track because they may:

- Introduce false information
- Alter instructions with practical consequences
- Create variants that diverge into separate traditions
- Become standardized and then magically relevant

When possible, the system should record:

- What was in the source
- What appeared in the copy
- Whether the change was intentional or accidental

## Lineage and Reliability

Lineage enables reliability assessment:

- A document close to the original with few intermediaries is likely more accurate
- A document with many copying steps has accumulated more opportunities for error
- A document produced by a reliable scribe in a careful institution is likely more accurate
- A document with known errors in its ancestry inherits uncertainty

## Lineage and Magic

In a mana-sensitive world, document lineage may have magical relevance:

- The original of a ritual text may have different properties from copies
- A copy made with specific procedures may acquire magical significance
- The accumulation of copying errors may alter the mana-coupling properties of a text

## Related Documents

- `docs/epistemics/documents.md` - Document structure and lifecycle
- `docs/epistemics/writing.md` - Writing technology
- `docs/language/writing-systems.md` - How language is encoded
