# Identity

Identity is the property by which an entity remains the same entity over time despite change. In Ontopolis, identity is a research question, not a primitive engine feature. Phase 23 implements a criterion-comparison research boundary without selecting a final answer.

## No Primitive Soul Object

Do not use a primitive `Soul` object without an accepted RFC.

```text
// Forbidden without RFC-META-001 acceptance:
struct Soul {
    identity: SoulId,
    memories: Vec<Memory>,
}
```

The human concept of a soul may exist independently of the actual engine model. Agents may believe in souls. The simulation does not need to implement that belief as a primitive physical object.

## What Identity Might Be

Candidate approaches for investigation:

- **Biological continuity** - Identity persists as long as biological processes continue in an unbroken chain
- **Psychological continuity** - Identity persists as long as memories, personality, and cognitive patterns continue
- **Pattern persistence** - Identity is a stable information pattern that can be instantiated in different substrates
- **Social recognition** - Identity persists as long as a community treats the entity as the same person
- **Causal chain** - Identity is a particular causal thread through the history of the world

These are research hypotheses, not decided architecture.

## Implemented Research Boundary

`ontopolis-metaphysics::IdentityContinuityExperiment` stores bounded trace-backed observations over opaque evidence channels. Explicit `ContinuityCriterion` values may weight the same observations differently. Evaluation returns separate fixed-point scores and supporting traces, never an authoritative same-person boolean.

The records contain no `AgentId`, `SoulId`, English continuity category, or agent belief. Concrete evidence adapters remain future research.

## Identity and Change

An entity changes over time:

- Biological cells are replaced
- Memories are gained and lost
- Personality shifts
- Social roles change
- Physical location changes

At what point does change become discontinuity? This is a metaphysical question that different agents and societies may answer differently.

## Identity and Cross-World Transfer

Cross-world transfer raises acute identity questions:

- If a body stays in Earth and a copy appears in Ontopolis, is it the same person?
- If memories are partial, is the resulting entity a continuation or a new person?
- If multiple identity patterns bind to one body, how many persons are present?
- If an identity pattern binds to a fetus, does the resulting adult have two identities?

## Related Documents

- `docs/metaphysics/death-and-persistence.md` - What happens when biological continuity ends
- `docs/metaphysics/cross-world-continuity.md` - Identity across world boundaries
- `docs/isekai/transfer-types.md` - Kinds of cross-world transfer
- `docs/isekai/foreign-memory.md` - What survives transfer
