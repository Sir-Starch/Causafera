# Explanation Architecture

The Explanation Engine converts complex structured simulation state and provenance into understandable human explanations without modifying, inventing, or becoming authoritative over simulation state.

## Pipeline

```text
AUTHORITATIVE SIMULATION
↓
ANALYTICS / CAUSAL QUERY
↓
OBSERVER ANALYTICAL CLASSIFICATION
↓
EXPLANATION IR
↓
DETERMINISTIC HUMAN-READABLE RENDERING
↓
OPTIONAL LLM SURFACE REALIZATION (TERMINAL GATE; NOT SCHEDULED)
↓
UI
```

## Crate

`ontopolis-explanation`

## Core Responsibilities

- Query authoritative simulation state and provenance;
- Apply observer analytical classifications;
- Produce structured Explanation IR;
- Render Explanation IR into human-readable text deterministically;
- After the terminal readiness gate only, optionally pass structured fact packets to LLMs for prose polish;
- Never modify simulation state;
- Never invent events or causal relationships;
- Never resolve uncertainty into false confidence.

## Non-Authoritative Nature

The Explanation Engine is strictly non-authoritative. Its outputs are for human understanding only. They cannot feed back into simulation state without an explicit physical or experimental intervention API.

## Detailed Development role

Explanation is the second-priority continuous workstream after authoritative simulation. Every
simulation capability targeting observable or validated maturity must add domain-valid metrics,
causal queries, uncertainty behavior, negative controls, typed evidence, and deterministic
rendering. State digests may establish equality or divergence, but digest-byte arithmetic may not
stand in for physical similarity or recovery.

Observer protocol work follows the bounded evidence needs of Explanation. UI rendering is batched
after read models stabilize. Optional LLM realization is not a numbered phase and remains forbidden
until the terminal gate in `docs/architecture/detailed-development-rebaseline.md` passes.

## Related Documents

- `docs/explanation/analytical-ontology.md` - Observer classifications
- `docs/explanation/explanation-ir.md` - Intermediate representation
- `docs/explanation/deterministic-rendering.md` - Text rendering
- `docs/explanation/optional-llm-surface.md` - LLM integration limits
- `docs/architecture/detailed-development-rebaseline.md` - priority, maturity, and terminal gate
- `docs/architecture/invariants.md` - INV-012: Explanation systems are non-authoritative
