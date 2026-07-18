# Phases 23–24 Metaphysical Research and Long-Run Experiments

> **Historical record.** This completed ExecPlan describes a Foundation Era project stage. Its implementation status and terminology may be outdated; use [the documentation index](../../docs/index.md), [roadmap](../../docs/roadmap/roadmap.md), and [active plans](../../PLANS.md) for current guidance.

## Goal
Provide metaphysically neutral research contracts and a real deterministic headless simulation path that can execute bounded long-run control/intervention experiments without runtime errors.

## Context
Phase 22 completed cross-world transfer bookkeeping, but `causafera-metaphysics`, `causafera-runtime`, `causafera-lab`, and the CLI are placeholders. The accepted mana, causal provenance, resolution, and scheduler foundations are not yet integrated into an executable loop. During implementation, the user identified that 3D lattice coordinates, 2D terrain, containment, and resolution existed without an accepted global spatial geometry model; this foundational gap is part of the plan.

## Relevant invariants
INV-003 through INV-007, INV-010 through INV-019, INV-023, INV-025 through INV-027, and INV-030.

## Ontology domains affected
Spatial geometry, geography, mana, causal provenance, causal resolution, metaphysics research, experimental analytics, simulation runtime, time, and physical pattern carriers.

## Causal carriers affected
Trace-backed physical recurrence samples, fixed-point mana transitions, directed relevance signals, identity-continuity evidence, perturbation observations, deterministic checkpoints, and counterfactual run comparisons.

## Relevant documents
Coordinates, spatial hierarchy, terrain, geology, metaphysics subsystem documents, epistemic experiments and replication, analytics phenomenon evaluation, performance methodology, architecture determinism/provenance, ADR-003/004, RFC-GEO-001/002, RFC-TRACE-001, RFC-MANA-001, RFC-RES-001, RFC-ISEKAI-001, and RFC-META-001/002.

## Current state
The metaphysics crate stores an unbounded byte pattern next to authoritative `AgentId`; runtime only owns an empty scheduler; lab only stores a string and seed; CLI `run` and `lab` only print placeholder text. No executable domain system advances authoritative state.

## Proposed architecture
Identity research stores bounded, typed continuity observations and evaluates opaque criterion schemas without producing an authoritative same-person verdict. Attractor research stores bounded field observations and classifies only numeric persistence/recovery evidence; it never creates a god, spirit, soul, or semantic mana type.

The runtime registers three sequential scheduler systems over one centralized synchronized state: a physical recurrence producer commits a real causal carrier event, mana proposes and commits traced cell changes, and resolution consumes a trace-backed numeric signal and commits its replacement field. Runtime errors are captured and returned at the public tick boundary. A canonical integer state digest supports strict replay checks.

The lab executes bounded control/intervention runs, records numeric checkpoints, verifies same-seed replay, and compares final trajectories. Human experiment names remain CLI/observer metadata and never enter authoritative state.

RFC-GEO-002 establishes a finite closed charted default world surface, fixed-point 2.5D geography, layered subsurface, and bounded full local 3D frames. Existing bare lattice coordinates are explicitly local-chart addresses. The runtime's mana cube is one local 3D frame, not the whole planet.

## Primitive vs emergent review
Typed IDs, fixed-point values, time, fingerprints, causal edges, opaque criterion/channel identities, observation windows, state digests, and experiment run bookkeeping are primitive. Personal sameness, souls, ghosts, reincarnation, gods, spirits, artifacts, intentional response, sacredness, and emergence claims remain hypotheses or downstream interpretations.

## Non-goals
Final identity metaphysics, semantic attractor entities, belief-reactive mana, full society/cognition integration, invented populations/history, complete phenomenon mining, persistence snapshots, observer protocol, UI, narrative, GPU work, or scale claims.

## Implementation stages
1. Replace metaphysics placeholders with bounded identity-continuity and attractor-observation research contracts.
2. Accept RFC-GEO-002 and add minimal chart-qualified surface/bounded local-3D coordinate contracts.
3. Integrate physical recurrence, mana, resolution, and provenance into a fallible deterministic runtime.
4. Add bounded long-run control/intervention execution, checkpoint metrics, canonical replay comparison, and CLI commands.
5. Add integration tests that execute many ticks, compare replay, and demonstrate a causal intervention difference.
6. Accept RFC-META-001/002 and update subsystem, ontology, TODO, roadmap, changelog, rebaseline, index, and PLANS.

## Verification
Workspace tests, strict clippy, formatting, diff checks, architectural searches, two identical long-run executions with equal digest, a control/intervention comparison, actual CLI run/lab invocations, and knowledge-graph refresh.

## Benchmark plan
Record elapsed wall time and numeric activity counts as measurements, not claims. Use bounded test workloads. Broader population/scale benchmarks remain TODO-PERF-001.

## Determinism impact
Strict scheduler ordering, explicit seeds, fixed-point/integer arithmetic, canonical event keys, stable vectors, fixed schema IDs, and canonical digest mixing. Wall time is report-only and excluded from authoritative results and replay equality.

## Memory impact
Metaphysical observations, checkpoints, ticks, field extent, and causal batches are bounded. The initial runtime deliberately uses a small dense mana field; provenance grows linearly with accepted transitions and is measured in summaries.

## Observer impact
None. CLI output is a non-authoritative headless diagnostic surface.

## Explanation impact
Research results expose numeric evidence and causal trace counts but make no semantic identity or divinity claim. Full Explanation IR remains Phase 25.

## Persistence impact
None. Runtime state is in-memory; TODO-PERSIST-001 remains separate from successful bounded execution.

## Cross-domain effects
Physical recurrence drives mana; committed mana changes drive resolution relevance; lab probes read snapshots only. The experiment harness does not mutate internal state outside configured physical intervention schedules.

## Risks
An attractor probe could be mistaken for an attractor entity; runtime fixture schemas could become semantic labels; centralized locking could be overextended; provenance growth could make very large runs costly; a replay digest could omit state.

## Documentation changes
Accept RFC-META-001/002 and update metaphysics, experiments, analytics/performance scope, runtime guidance, ontology matrices, roadmap, TODO, changelog, rebaseline, index, and PLANS.

## TODO changes
Complete TODO-GEO-003, TODO-META-001, TODO-META-002, and TODO-LAB-001. Record Phase 24 completion without prematurely completing TODO-ANALYTICS-001, TODO-PERF-001, TODO-PERSIST-001, or Phase 25 work.

## Decision log
- 2026-07-12: Identity experiments return evidence by opaque criterion, never a Ground Truth identity verdict.
- 2026-07-12: Attractor observations describe persistence and perturbation response, never semantic beings.
- 2026-07-12: The first runnable world is a bounded causal field experiment, not a fake city or population.
- 2026-07-12: Wall-clock metrics are excluded from canonical experiment equality.
- 2026-07-12: Global geography is a finite charted 2.5D surface; full 3D is local and bounded; containment is not geometry.

## Progress
- [x] Metaphysics research contracts implemented.
- [x] Multiscale spatial geometry contract implemented.
- [x] Deterministic causal runtime implemented.
- [x] Long-run laboratory and CLI implemented.
- [x] Documentation and phase tracking updated.
- [x] Full verification passes: 147 workspace tests, strict clippy, formatting, diff checks, exact replay, a successful 1,000-tick control/intervention lab, and refreshed code graph.
