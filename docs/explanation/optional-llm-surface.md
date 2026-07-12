# Optional LLM Surface

LLMs may be used in the explanation pipeline, but only after Explanation IR exists. They are optional and strictly limited.

## Allowed Pipeline

```text
Explanation IR
↓
validated fact packet
↓
LLM wording
↓
UI prose
```

## What LLMs May Do

The LLM may improve:

- sentence flow;
- tone;
- narrative readability.

## What LLMs May Not Do

The LLM may not:

- inspect raw authoritative state independently;
- discover causal relationships;
- resolve uncertainty;
- invent missing events;
- modify history.

## Fact Packet Association

Every LLM-generated explanation must remain associated with its source fact packet. The UI must be able to expose structured source data on demand.

## Optional Nature

LLM use is optional. Ontopolis must remain fully understandable without it. All core functionality must work with deterministic rendering alone.

## Non-Authoritative

LLM outputs are never authoritative. They are presentation-layer polish. The underlying Explanation IR and fact packets remain the ground truth for any explanation.

## Related Documents

- `docs/explanation/architecture.md` - Explanation pipeline
- `docs/explanation/explanation-ir.md` - Source IR
- `docs/explanation/deterministic-rendering.md` - Deterministic alternative
- `docs/architecture/invariants.md` - INV-011: LLMs are non-authoritative
