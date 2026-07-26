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

## Plan locations and control-plane adapters

- `plans/` contains the authoritative project ExecPlans. These are the sole source of truth for
  architecture, implementation stages, acceptance criteria, and plan status.
- `.omo/plans/` contains tool-specific adapters or execution-control plans used by the OmO
  orchestration tooling (e.g., Momus review paths, `start-work` ledger references). These adapters
  are non-authoritative and intentionally minimal; they do not supersede, copy, revise, or
  independently version the canonical ExecPlans.
- Substantive plan changes must never be made only in `.omo/plans/`. Any accepted review finding
  must be applied to the canonical file under `plans/`.

## Execution safety and checkpoints

- An implementation wave is complete only when its focused diagnostics, tests, and applicable build
  checks are green. RED or partially integrated work is not a checkpoint.
- Create an atomic local checkpoint commit after every completed green wave. The commit must contain
  the wave's implementation and direct tests, stage only the documented file allowlist, and be
  recorded in the ExecPlan Progress section with the commands that passed.
- Do not begin a second implementation wave while a completed prior wave exists only as uncommitted
  working-tree state. If commits are explicitly prohibited, pause after the first green wave and
  obtain authorization before continuing.
- Parallel writing agents never own Git state. They must not run `git checkout`, `git restore`,
  `git reset`, `git clean`, `git stash`, staging, or commit commands. The lead agent reads every
  touched file, runs verification, integrates the wave, and creates the checkpoint.
- Never use a worktree-discarding command to recover from a failed edit. Inspect snapshots and Git
  objects read-only, materialize only verified blobs through an explicit path/hash allowlist, and
  preserve all unrelated dirty paths.
- Before each checkpoint, record `git status`, inspect both staged and unstaged diffs, confirm no
  secrets or temporary artifacts are included, and stage files by explicit path. A broad `git add .`
  is not an acceptable checkpoint procedure.

## Active Planning

- [`plans/ui-localization-architecture.md`](plans/ui-localization-architecture.md) — Accepted complete observer UI localization architecture across five languages (`en`, `ru`, `zh-Hans`, `de`, `es`).
- [`plans/conserved-thermal-energy-carrier.md`](plans/conserved-thermal-energy-carrier.md) — Accepted
  bounded conserved thermal storage and same-chart transfer tranche; implementation branch created from
  this acceptance commit.
- [`plans/biological-mana-coupling.md`](plans/biological-mana-coupling.md) — accepted architecture with pending implementation stages for physical biological mana coupling and its downstream validation.
- [`plans/local-mana-material-surface-coupling.md`](plans/local-mana-material-surface-coupling.md) — accepted bounded local mana-cell to material-surface coupling slice; replaces the global mana-total gate with per-surface local hysteresis.
- [`plans/terrain-carrier-participation.md`](plans/terrain-carrier-participation.md) — accepted and implemented; the terrain carrier reaches the tick loop as a standing spatial structure, so the world seed varies the simulation (`TODO-RUNTIME-002`).

## Draft Plans

- [`plans/experiment-recipe-mana-source.md`](plans/experiment-recipe-mana-source.md) — Draft
  bounded production-path experiment-recipe mana source slice; not active and not an operator API.
- [`plans/observer-field-raster-map.md`](plans/observer-field-raster-map.md) — Draft bounded
  per-chunk raster projection of the terrain and mana fields, plus a config-gated two-dimensional
  active chunk shape, so the chart instrument renders measured relief and a measured mana field
  instead of one aggregate per chunk.

## Paused Audit and Evidence Material

- [`plans/detailed-development-maturity-audit.md`](plans/detailed-development-maturity-audit.md) — intentionally paused evidence audit for the frozen `26026fb3862e` baseline. Completed Todos 1–4 remain diagnostic groundwork; unfinished deep audit work is not implementation guidance or a prerequisite.

## Historical Plans and Records

Completed Foundation Era ExecPlans, the completed Detailed Development rebaseline, and the public-source-readiness record live in [historical plans and records](plans/history/README.md). They preserve provenance, not current guidance. For current project state, use the [roadmap](docs/roadmap/roadmap.md), [maturity matrix](docs/ontology/domain-coverage-matrix.md), and active plan above.

- [`plans/actor-material-mana-loop.md`](plans/actor-material-mana-loop.md) — completed and
  verified first Detailed Development production vertical slice; preserved as its implementation
  record rather than current planning guidance.
