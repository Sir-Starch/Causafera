# Causafera Plans

ExecPlans are structured architectural proposals required before beginning implementation of major simulation subsystems. They align work with the project thesis and invariants in [README.md](README.md). This document is the authoritative plan index and status map, not a complete project-status summary.

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

## Plan authority

- `plans/` contains the authoritative project ExecPlans. Invariants, accepted ADRs, RFCs, and
  plans each have their own authority; a canonical ExecPlan is the source of truth for the scope,
  implementation stages, acceptance criteria, decisions, progress, and status of the specific work
  it describes.
- Tool-specific execution artifacts may exist outside `plans/` to coordinate work in a particular
  environment. They are non-authoritative adapters and must not duplicate, supersede, independently
  revise, or independently version a canonical ExecPlan.
- Substantive plan changes must be made in the canonical file under `plans/`. Findings produced by
  any review or orchestration tool become project decisions only after they are incorporated into
  that canonical plan.
- When a pull request completes the implementation of a plan, it must also remove the plan from
  Active Plans and place it under Completed Detailed Development Plans.

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

## Active Plans

This section lists only accepted or in-progress work with genuinely unfinished implementation stages, acceptance criteria, or close-out work.


## Blocked and Paused Work

This section lists accepted proposals whose implementation is blocked by unbuilt prerequisites, as well as paused audits that are neither active priorities nor completed records. Work listed here is not current implementation guidance and does not block present development.

- [`plans/biological-mana-coupling.md`](plans/biological-mana-coupling.md) — accepted architecture that is not current implementation work; implementation is currently blocked until detailed biology contracts and prerequisites mature (`TODO-BIO-003`).
- [`plans/detailed-development-maturity-audit.md`](plans/detailed-development-maturity-audit.md) — intentionally paused evidence audit for the frozen `26026fb3862e` baseline. Completed Todos 1–4 remain diagnostic groundwork; unfinished deep audit work does not block current work and is not implementation guidance or a prerequisite (`TODO-DEPTH-001`).

## Draft Plans

This section lists proposals that have not yet been accepted.

- [`plans/experiment-recipe-mana-source.md`](plans/experiment-recipe-mana-source.md) — Draft bounded production-path experiment-recipe mana source slice; not active and not an operator API.

## Completed Detailed Development Plans

These plans have been accepted and implemented. They are preserved as implementation and decision records rather than current guidance.

- [`plans/hydrology.md`](plans/hydrology.md) — completed and implemented; conserved multi-resolution hydrology. Geography-owned fixed-point surface/soil/groundwater/conveyance storage with derived hydraulic coefficients, explicit persisted forcing, a fixed nine-substage tick with head-driven routing across same-chart seams, an exactly-zero conservation residual per tick, conservative block-level resolution driven by the engine's resolution field, a seventh canonical bootstrap stage, snapshot section `0x000F` v1, a whole-tick staging transaction, additive observer protocol-V1 fields with a lossless unsigned raster band, and ten typed Explanation claim schemas. Moves `CURRENT_DIGEST_SCHEMA_VERSION` 7→8 and `RUNTIME_RECIPE_SECTION_MAJOR` 6→7; closes `TODO-HYDRO-001`. `docs/rfc/RFC-HYDRO-001.md` is Accepted. Measured ceilings are in `docs/performance/benchmarks.md`.
- [`plans/production-bootstrap-receipt-closure.md`](plans/production-bootstrap-receipt-closure.md) — completed and implemented; binds the six runtime bootstrap stages to the canonical `causafera-world` historical DAG/receipt contract, emits one terminal receipt per stage anchored to a real bounded stage-result transition, persists and fail-closed validates the record, exposes bounded observer/Explanation evidence, and removes the last fixture actor constructors. Moves `CURRENT_DIGEST_SCHEMA_VERSION` 6→7, population/bootstrap section major 1→2, and `RUNTIME_RECIPE_SECTION_MAJOR` 5→6; advances `TODO-RUNTIME-001`, `TODO-OBSERVER-003`, and `TODO-EXPLAIN-003` without closing any of them.
- [`plans/thermal-conservation-aggregate-validation.md`](plans/thermal-conservation-aggregate-validation.md) — completed and implemented; cross-validates every `ThermalConservationReceipt`'s aggregate literals against materialized state and per-receipt data on snapshot import, closes `TODO-THERMAL-006`, and leaves `THERMAL_SECTION_MAJOR`, `MATERIAL_SURFACE_SECTION_MAJOR`, and `CURRENT_DIGEST_SCHEMA_VERSION` unchanged.
- [`plans/conserved-thermal-energy-carrier.md`](plans/conserved-thermal-energy-carrier.md) — completed and implemented; bounded conserved thermal storage and same-chart transfer tranche.
- [`plans/local-mana-material-surface-coupling.md`](plans/local-mana-material-surface-coupling.md) — completed and implemented; bounded local mana-cell to material-surface coupling slice replacing the global mana-total gate with per-surface local hysteresis.
- [`plans/terrain-carrier-participation.md`](plans/terrain-carrier-participation.md) — completed and implemented; the terrain carrier reaches the tick loop as a standing spatial structure (`TODO-RUNTIME-002`).
- [`plans/observer-locale-coverage.md`](plans/observer-locale-coverage.md) — completed and implemented; the observer presents itself in five locales with INV-007 coverage (`TODO-UI-006`).
- [`plans/terrain-chunk-boundary-continuity.md`](plans/terrain-chunk-boundary-continuity.md) — completed and implemented; terrain elevation, roughness, and material are generated from chart position so adjacent chunks meet continuously at boundaries (`TODO-GEO-005`).
- [`plans/terrain-structure-cross-chunk-neighbours.md`](plans/terrain-structure-cross-chunk-neighbours.md) — completed and implemented; the standing terrain carrier reads real neighbouring chunks' terrain across chunk boundaries (`TODO-GEO-006`).
- [`plans/mana-gate-calibration.md`](plans/mana-gate-calibration.md) — completed and implemented; local mana effect gate recalibration against populated fields (`TODO-MANA-007`).
- [`plans/coherent-surface-material-regions.md`](plans/coherent-surface-material-regions.md) — completed and implemented; surface material is generated as spatially coherent regions instead of per-cell noise (`TODO-GEO-004`).
- [`plans/performance-baseline-and-digest-cost.md`](plans/performance-baseline-and-digest-cost.md) — completed and implemented; checked-in statistical benchmark harness, scene-cue config validation, and incremental `history_digest` (`TODO-PERF-001`). Follow-ups continue as `TODO-PERF-002` and `TODO-PERF-003`.
- [`plans/mana-seam-saturation-ceiling.md`](plans/mana-seam-saturation-ceiling.md) — completed and implemented; cross-seam mana deliveries bounded by `maximum_intensity` (`TODO-MANA-006`).
- [`plans/thermal-material-surface-coupling.md`](plans/thermal-material-surface-coupling.md) — completed and implemented; bounded, conserved retained-heat exchange between material surfaces and thermal cells (`TODO-THERMAL-002`).
- [`plans/observer-field-raster-map.md`](plans/observer-field-raster-map.md) — completed and implemented; per-chunk raster projection of terrain and mana fields, plus optional area-shaped active chunk geometry (`TODO-OBS-001`).
- [`plans/actor-material-mana-loop.md`](plans/actor-material-mana-loop.md) — completed and implemented; first Detailed Development production vertical slice linking material surfaces, contact actions, perception, and mana effects (`TODO-SIM-001`).

## Historical Foundation Records

Completed Foundation Era ExecPlans, the completed Detailed Development rebaseline, and the public-source-readiness record live in [historical plans and records](plans/history/README.md). They preserve provenance, not current guidance. For current project state, use the [roadmap](docs/roadmap/roadmap.md), [maturity matrix](docs/ontology/domain-coverage-matrix.md), and active plans above.
