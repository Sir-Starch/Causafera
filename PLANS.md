# Causafera Plans

ExecPlans are structured architectural proposals required before beginning implementation of major simulation subsystems. They align work with the project thesis and invariants in [README.md](README.md). This page lists only work that remains current; it is not a project-status summary.

## ExecPlan Requirements

The following changes require an ExecPlan:

- Foundational subsystems
- Language architecture
- Concept formation
- Causal resolution
- Geographic models
- Biological representation
- Mana implementation
- Explanation architecture changes
- Observer protocol changes
- Persistence format changes
- CUDA work

## ExecPlan Structure

```text
Goal
Context
Relevant invariants
Ontology domains affected
Causal carriers affected
Relevant documents
Current state
Proposed architecture
Primitive vs emergent review
Non-goals
Implementation stages
Verification
Benchmark plan
Determinism impact
Memory impact
Observer impact
Explanation impact
Persistence impact
Cross-domain effects
Risks
Documentation changes
TODO changes
Decision log
Progress
```

## Active Planning

- [`plans/biological-mana-coupling.md`](plans/biological-mana-coupling.md) — accepted architecture with pending implementation stages for physical biological mana coupling and its downstream validation.

## Draft Plans

- [`plans/experiment-recipe-mana-source.md`](plans/experiment-recipe-mana-source.md) — Draft
  bounded production-path experiment-recipe mana source slice; not active and not an operator API.

## Paused Audit and Evidence Material

- [`plans/detailed-development-maturity-audit.md`](plans/detailed-development-maturity-audit.md) — intentionally paused evidence audit for the frozen `26026fb3862e` baseline. Completed Todos 1–4 remain diagnostic groundwork; unfinished deep audit work is not implementation guidance or a prerequisite.

## Historical Plans and Records

Completed Foundation Era ExecPlans, the completed Detailed Development rebaseline, and the public-source-readiness record live in [historical plans and records](plans/history/README.md). They preserve provenance, not current guidance. For current project state, use the [roadmap](docs/roadmap/roadmap.md), [maturity matrix](docs/ontology/domain-coverage-matrix.md), and active plan above.

- [`plans/actor-material-mana-loop.md`](plans/actor-material-mana-loop.md) — completed and
  verified first Detailed Development production vertical slice; preserved as its implementation
  record rather than current planning guidance.
