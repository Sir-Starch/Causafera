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

## Cognitive Architecture Constraints

When working on cognition-related code (attention, memory, concepts, beliefs, language, or social inference), observe the following boundaries established by the subjective scene rebaseline:

- Agents do not directly know authoritative entity identity. `EntityId`, `BodySegmentId`, `PlaceId`, and similar Ground Truth identifiers are not subjective knowledge. See `docs/architecture/invariants.md` INV-027.
- Subjective scene construction is a required intermediate layer between generic perceptual features and concept/belief formation. Do not pass `Feature` lists directly into concept systems without a subjective scene mapping step. See `docs/architecture/cognition-rebaseline.md`.
- Objective body state (`BodyStructure`, physiological state) and subjective body schema are distinct. Do not give cognition crates direct omniscient access to complete biological state. See `docs/architecture/invariants.md` INV-034.
- Persistent autobiographical memory and active working context are distinct. Do not equate stored memory with currently active cognition. See `docs/architecture/invariants.md` INV-032.
- Semantic situation enums (e.g., `AnxietySituation`, `CombatSituation`) must not replace subjective scene construction. Situations emerge from lower-level cognitive processes. See `docs/rfc/RFC-COG-001.md`.

## What Not To Do

- Do not implement hunger systems, random jobs, fake cities, fake languages, or placeholder magical schools.
- Do not create demo residents or fake history.
- Do not add English vocabulary as simulation state.
- Do not implement gameplay features during foundation phases.
- Do not make unverified emergence or scale claims.
