# Ontopolis Agent Guidelines

All AI agents working on Ontopolis must follow these rules.

## Required Reading Order

Before making any changes, read:

1. `docs/index.md`
2. `docs/vision/project-thesis.md`
3. `docs/vision/uniqueness.md`
4. `docs/architecture/invariants.md`
5. `docs/ontology/domain-coverage-matrix.md`
6. Relevant subsystem documentation
7. Relevant ADRs
8. Relevant RFCs

## Core Rules

- Use an ExecPlan for multi-stage work (see `PLANS.md`).
- Never introduce semantic domain enums merely for convenience.
- Never use English labels as authoritative simulation meaning.
- Never directly expose Ground Truth to agents.
- Never let LLMs mutate authoritative state.
- Never let the Explanation Engine mutate simulation state.
- Preserve deterministic RNG rules.
- Treat geography and biology as causal state.
- Preserve language intent/utterance/interpretation separation.
- Benchmark performance claims.
- Update TODO and documentation.
- Avoid unrelated opportunistic implementation.
- Use `codebase-memory-mcp` to maintain and query a semantic knowledge graph of the codebase (via `search_graph`, `trace_path`, `get_code_snippet`, and Cypher queries) as the primary method for code discovery and dependency tracing, prioritizing it over raw text searches.

## What Not To Do

- Do not implement hunger systems, random jobs, fake cities, fake languages, or placeholder magical schools.
- Do not create demo residents or fake history.
- Do not add English vocabulary as simulation state.
- Do not implement gameplay features during foundation phases.
- Do not make unverified emergence or scale claims.
