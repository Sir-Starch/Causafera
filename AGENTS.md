# Causafera Agent Guidelines

All AI agents working on Causafera must follow these rules.

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

## Detailed Development Priorities

Phases 0–26 are the completed Foundation Era. They prove minimum contracts and an executable causal
loop, not mature simulation depth. Current work belongs to the open-ended Detailed Development
Program described in `docs/architecture/detailed-development-rebaseline.md`.

- Prioritize authoritative simulation depth first, Explanation/analytics second, observer protocol
  as required for inspection, and batched UI milestones last.
- Keep Explanation current enough that every accepted simulation capability remains causally
  inspectable; do not postpone all interpretation until the end.
- Do not update UI for every internal field. Add UI work when a stable read model enables a complete
  inspection workflow.
- Treat digests as equality/divergence anchors only. Never use digest-byte distance as physical,
  recovery, stability, or semantic distance. See INV-038.
- Do not use fixture/demo constructors in production bootstrap or runtime sessions. See INV-039.
- Assign new phase numbers only through accepted ExecPlans. The final number of phases is unknown.
- Optional LLM surface realization is not a numbered phase and is forbidden until the terminal gate
  in the Detailed Development rebaseline is satisfied.

## Cognitive Architecture Constraints

When working on cognition-related code (attention, memory, concepts, beliefs, language, or social inference), observe the following boundaries established by the subjective scene rebaseline:

- Agents do not directly know authoritative entity identity. `EntityId`, `BodySegmentId`, `PlaceId`, and similar Ground Truth identifiers are not subjective knowledge. See `docs/architecture/invariants.md` INV-027.
- Subjective scene construction is a required intermediate layer between generic perceptual features and concept/belief formation. Do not pass `Feature` lists directly into concept systems without a subjective scene mapping step. See `docs/architecture/cognition-rebaseline.md`.
- Objective body state (`BodyStructure`, physiological state) and subjective body schema are distinct. Do not give cognition crates direct omniscient access to complete biological state. See `docs/architecture/invariants.md` INV-034.
- Persistent autobiographical memory and active working context are distinct. Do not equate stored memory with currently active cognition. See `docs/architecture/invariants.md` INV-032.
- Semantic situation enums (e.g., `AnxietySituation`, `CombatSituation`) must not replace subjective scene construction. Situations emerge from lower-level cognitive processes. See `docs/rfc/RFC-COG-001.md`.

## Spatial Architecture Constraints

- Local physical space is full bounded 3D; global geography is a finite charted 2.5D planetary surface with selective volumetric 3D. See `docs/rfc/RFC-GEO-002.md` and INV-036.
- `WorldCoord` and bare `ChunkCoord` are local-chart lattice addresses, not unique global planetary coordinates. Global geography must carry chart identity or use registered transforms.
- Spatial containment does not define geometry, adjacency, metric distance, ownership, or jurisdiction. Causal resolution may change detail but not topology or geometry. See INV-037.
- Future subjective spatial relations must be derived from physically accessible local geometry and mapped to relative agent-local cues; do not place chart/frame/place IDs or exact authoritative poses in cognition.

## What Not To Do

- Do not implement hunger systems, random jobs, fake cities, fake languages, or placeholder magical schools.
- Do not create demo residents or fake history.
- Do not add English vocabulary as simulation state.
- Do not implement gameplay features during foundation phases.
- Do not make unverified emergence or scale claims.
