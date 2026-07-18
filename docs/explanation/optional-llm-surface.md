# Optional LLM Surface

LLMs may eventually be used in the explanation pipeline, but Explanation IR existence alone is not
sufficient authorization. They are optional, strictly limited, and currently not scheduled.

## Implementation status

Optional LLM surface realization is not a numbered roadmap phase. It is the final possible
integration stage after the unknown amount of Detailed Development required to mature the
simulation, deterministic Explanation Engine, observer contracts, persistence, provenance, and
performance. No final phase number is reserved.

A dedicated future RFC may propose implementation only after every terminal condition in
`docs/architecture/detailed-development-rebaseline.md` passes. Until then this document defines a
boundary, not planned work.

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

LLM use is optional. Causafera must remain fully understandable without it. All core functionality must work with deterministic rendering alone.

## Non-Authoritative

LLM outputs are never authoritative. They are presentation-layer polish. The underlying Explanation IR and fact packets remain the ground truth for any explanation.

## Related Documents

- `docs/explanation/architecture.md` - Explanation pipeline
- `docs/explanation/explanation-ir.md` - Source IR
- `docs/explanation/deterministic-rendering.md` - Deterministic alternative
- `docs/architecture/invariants.md` - INV-011: LLMs are non-authoritative
- `docs/architecture/detailed-development-rebaseline.md` - terminal readiness gate
