# Phase 21 Historical Bootstrap

> **Historical record.** This completed ExecPlan describes a Foundation Era project stage. Its implementation status and terminology may be outdated; use [the documentation index](../../docs/index.md), [roadmap](../../docs/roadmap/roadmap.md), and [active plans](../../PLANS.md) for current guidance.

## Goal

Implement a bounded deterministic orchestration and provenance contract for causally constrained deep/recent historical synthesis without generating fake history or semantic high-level events.

## Context

Phases 3–20 provide spatial, terrain, biological, language, practice, epistemic, mana, resolution, social, economy, and city carrier foundations. They lack a common contract that orders low/high-resolution synthesis, derives stable stage seeds, and proves that endpoint state continues declared causal ancestry.

## Relevant invariants

INV-006, INV-009, INV-014 through INV-020, and INV-023.

## Ontology domains affected

World generation, time, geography, causal resolution, provenance, and all future domain bootstrap adapters.

## Causal carriers affected

Bounded synthesis stages, opaque process schemas, target chunks, parameter/result fingerprints, stage dependencies, committed traces, and simulation-time intervals.

## Relevant documents

Historical bootstrap, world-generation provenance, determinism, provenance, language bootstrap, causal resolution, ADR-002/003, RFC-TRACE-001, RFC-RES-001, and RFC-LANG-001.

## Current state

Terrain and language have independent deterministic generation contracts. No cross-domain historical plan validates temporal ordering, target resolution, or exact committed trace continuity.

## Proposed architecture

Add `HistoricalBootstrapPlan` to `causafera-world`. It canonically orders bounded `HistoricalStage` records containing opaque process identity, a non-empty time span, numeric detail ordinal, target chunks, prior-stage dependencies, external trace causes, and parameter fingerprint. A domain-separated seed is derived per stage. `HistoricalStageReceipt` records committed result fingerprint and trace; plan validation requires one receipt per stage and exact dependency-trace continuation.

## Primitive vs emergent review

Time, numeric resolution, spatial targets, schema identity, fingerprints, dependency edges, seed inputs, and traces are primitive bookkeeping. Geological eras, migrations, settlements, wars, plagues, discoveries, districts, families, institutions, and historical narratives are domain outcomes or analytical classifications.

## Non-goals

Generating residents, families, cities, lore, event tables, geology/ecology/population algorithms, domain aggregation, state mutation, scheduler integration, cache formats, persistence, observer protocol, GPU work, plausibility or scale claims, and Phase 22 transfer/metaphysics.

## Implementation stages

1. Add opaque historical bootstrap IDs and a bounded canonical stage DAG.
2. Add deterministic stage seed derivation and committed receipt validation.
3. Test order independence, ancestry continuation, bounds, and semantic exclusion.
4. Accept RFC-HIST-001 and update TODO, roadmap, ontology, world docs, changelog, and phase registry.

## Verification

Workspace tests, strict clippy, formatting, diff checks, architectural string/float/semantic-enum searches, and knowledge-graph refresh.

## Benchmark plan

No performance claim. Benchmarking belongs with concrete domain adapters and cache policies.

## Determinism impact

No system entropy, locale, floating point, hash iteration, or producer ordering. Stage order is typed-ID canonical; seeds use explicit world/plan/stage/process/time inputs.

## Memory impact

Stage, target, dependency, external-cause, and receipt-cause vectors have hard caps.

## Observer impact

None. Future read-only views may expose numeric plans and traces with non-authoritative glosses.

## Explanation impact

Receipts allow future explanations to traverse actual committed ancestry; narrative cannot fill missing receipts.

## Persistence impact

None. Future snapshots must preserve canonical plans, receipts, fingerprints, and trace identities.

## Cross-domain effects

Concrete domain adapters may consume stage seeds/detail and emit committed receipts. This phase does not implement or couple those adapters.

## Risks

Opaque process IDs could become hidden event enums; receipts could be mistaken for mutation authorization; numeric detail could be mistaken for a domain aggregation algorithm.

## Documentation changes

Accept RFC-HIST-001 and update world, ontology, roadmap, TODO, changelog, index, and rebaseline status.

## TODO changes

Add and complete TODO-HIST-001. Leave concrete domain synthesis and existing TODO-GEO-002 pending.

## Decision log

- 2026-07-12: Implement orchestration/provenance contracts, not procedural historical content.
- 2026-07-12: Require exact trace continuation from declared stage dependencies.
- 2026-07-12: Keep process meaning opaque and historical classification downstream.

## Progress

- [x] Stage plan and deterministic seed contracts implemented.
- [x] Receipt ancestry validation implemented.
- [x] Documentation and phase tracking updated.
- [x] Full verification passes.
