# Detailed Development Rebaseline

**Status:** Completed and verified on 2026-07-13

## Goal

End the fixed foundation-era roadmap after Phase 26 and move Ontopolis into an open-ended detailed
development program. The program deepens every authoritative domain until it participates in
meaningful, reproducible, causally reconstructable long runs. Simulation work has first priority,
Explanation evolves alongside it, observer contracts follow inspection needs, and UI work is
batched at stable milestones. Optional LLM wording is not a numbered phase and remains deferred
until the simulation, deterministic Explanation Engine, and inspection UI satisfy terminal gates.

## Context

Phases 0–26 successfully produced a deterministic scheduler, domain contracts, provenance,
persistence, a bounded executable causal loop, typed Explanation IR, observer v1 transport, and a
desktop observer. Those phases deliberately optimized for minimum valid foundations and an
end-to-end executable path. Their completion does not mean that geography, biology, cognition,
language, society, economy, history, metaphysics, or Explanation have reached production depth.

The first live observer made that distinction visible. A 192-tick diagnostic produced 15,501
traces, of which 15,296 were mana cell changes. Physical feedback mainly changes a counter and
sample magnitude; population lifecycle uses tick-modulo schedules; production session actors
enter through a fixture helper; observer v1 exposes only runtime summary, Explanation IR, and
chunk summary queries; and the current recovery analysis treats digest bytes as a distance metric.
The project is executable, but the executable path is a causal research harness rather than a
mature world simulation.

## Relevant invariants

- INV-001 through INV-010: subjective access, semantic boundaries, geography, and resolution.
- INV-011 through INV-013: LLM, Explanation, and observer non-authority.
- INV-014 through INV-020: provenance, phase control, performance, and inspectable emergence.
- INV-021 through INV-026: UI separation and evidence-bearing explanations.
- INV-027 through INV-035: subjective scene and cognitive continuity boundaries.
- INV-036 and INV-037: multiscale spatial geometry and separation from containment/resolution.
- New invariant: digests establish identity/equality, never domain distance or recovery.
- New invariant: production state must be causally initialized; fixtures are test-only.

## Ontology domains affected

All domains in `docs/ontology/domain-coverage-matrix.md`. This rebaseline changes their maturity
interpretation and sequencing policy; it does not add authoritative semantic taxonomies.

## Causal carriers affected

Physical fields, material and energy transfers, geometry, terrain and environmental processes,
biological state, sensory signals, subjective scenes, actions, population flows, practices,
utterances and documents, social records, causal-resolution signals, mana patterns, and historical
bootstrap receipts.

## Relevant documents

- `docs/vision/project-thesis.md`
- `docs/vision/uniqueness.md`
- `docs/architecture/invariants.md`
- `docs/architecture/cognition-rebaseline.md`
- `docs/architecture/observer.md`
- `docs/architecture/performance.md`
- `docs/explanation/architecture.md`
- `docs/explanation/optional-llm-surface.md`
- `docs/rfc/RFC-EXPLAIN-001.md`
- `docs/ontology/domain-coverage-matrix.md`
- `docs/roadmap/roadmap.md`
- `docs/development/todo-backlog.md`
- ADR-004, ADR-005, and ADR-006

## Current state

- Foundation Phases 0–26 are completed within their explicitly minimal scopes.
- Runtime, persistence, observer transport, and desktop UI execute without fabricated UI data.
- Domain crates contain many validated contracts that are not yet coupled into the production
  scheduler or historical bootstrap.
- Runtime summaries compress internal state into a few counters and chunk aggregates.
- Explanation IR preserves evidence structure, but its analytical ontology and several metrics are
  still minimal; digest-distance recovery is invalid for physical interpretation.
- Phase 27 incorrectly reserves optional narrative/LLM realization before the unknown amount of
  remaining simulation-depth work.

## Proposed architecture

### Program model

Phases 0–26 become the closed **Foundation Era**. Work after Phase 26 belongs to the **Detailed
Development Program**. The program has no predetermined final phase number. A numbered phase or
bounded batch is allocated only when its ExecPlan is accepted; the roadmap does not reserve a
terminal phase in advance.

### Priority order

1. **Simulation:** authoritative state, causal mutation, cross-domain coupling, resolution,
   persistence, determinism, performance, and long-run validation.
2. **Explanation:** domain metrics, causal queries, uncertainty, counterfactuals, and deterministic
   rendering kept current with accepted simulation capabilities.
3. **Observer protocol:** added when simulation or Explanation requires a bounded inspection path.
4. **UI:** updated in coherent milestone batches, not for every new field or internal refactor.
5. **Optional LLM surface:** considered only after terminal readiness gates are satisfied.

### Detailed maturity levels

- **M0 — Documented:** domain intent and boundaries exist.
- **M1 — Contracted:** validated types and isolated deterministic operations exist.
- **M2 — Executable:** production scheduler/bootstrap mutates real domain state with provenance.
- **M3 — Coupled:** the domain both affects and is affected by other domains through physical or
  informational carriers; causal resolution and persistence preserve it.
- **M4 — Observable:** bounded observer projections and domain-valid Explanation metrics can
  reconstruct significant changes without digest-distance or narrative inference.
- **M5 — Validated:** replay-verified long runs, counterfactuals, failure/negative controls, and
  representative benchmarks support the claimed behavior and envelope.

Foundation completion usually establishes M1 and occasionally parts of M2–M4. A domain is not
called mature until its claimed scope reaches M5. Maturity is recorded per capability, not awarded
to an entire crate because one contract exists.

### Terminal LLM gate

Optional LLM surface realization remains unnumbered and unplanned until all conditions hold:

- every domain claimed by the target simulation scope has accepted M5 evidence;
- no fixture/demo initialization remains in production execution paths;
- long-run experiments produce nontrivial cross-domain behavior with reproducible controls;
- Explanation can answer important world/result questions with typed evidence, uncertainty,
  alternatives, and causal traces using deterministic rendering alone;
- observer protocol and UI can expose the complete structured source packet behind prose;
- performance, persistence, determinism, and provenance envelopes are measured and accepted;
- an explicit future RFC authorizes the optional surface without weakening INV-011.

Passing the gate permits an integration proposal; it does not make an LLM required.

## Primitive vs emergent review

Detailed implementation must deepen physical and informational mechanisms without promoting
human interpretations into authoritative enums. Species, occupations, institutions, technologies,
rituals, religions, illnesses, wars, settlements, and historical periods remain emergent or
observer/agent classifications unless a lower-level structural primitive independently requires a
typed identity.

## Non-goals

- Implementing every detailed domain in this rebaseline change.
- Reserving a final phase number or predicting how many phases remain.
- Treating maximum depth as unbounded molecular or voxel fidelity.
- Updating the UI for every simulation field.
- Adding narrative generation, an LLM dependency, fake residents, or fake history.
- Rewriting completed foundation plans to pretend their original scopes were larger.

## Implementation stages

1. Add this ExecPlan and register it as active.
2. Add the Detailed Development architecture rebaseline and maturity gates.
3. Replace the post-26 fixed roadmap with the open-ended program and remove Phase 27.
4. Rebaseline domain coverage and TODO semantics; add immediate critical work packages.
5. Correct digest/recovery and production-fixture architectural contracts.
6. Update Explanation, observer, UI cadence, LLM, README, index, agent guidance, and historical
   cross-references.
7. Verify stale phase references, documentation links, formatting, and workspace integrity.
8. Record completion in `PLANS.md` and the changelog.

## Verification

- `rg` finds no active roadmap assignment of optional LLM work to Phase 27.
- Every remaining `Phase 27` reference is either removed or explicitly marked as superseded
  historical context.
- Roadmap, rebaseline, domain matrix, TODO backlog, agent guidance, Explanation RFC, and README
  agree on priority and terminal-gate policy.
- New invariants have no contradictory documentation.
- Markdown links resolve to repository files.
- `git diff --check` passes.
- Documentation-only work must not alter runtime behavior; existing targeted tests remain valid.

## Benchmark plan

No runtime benchmark is claimed by this documentation rebaseline. It elevates benchmark work to a
first-class Detailed Development requirement. Each future domain ExecPlan must define a
representative workload, CPU reference behavior, memory envelope, and observer-off/observer-on
measurements before scale or maturity claims.

## Determinism impact

No authoritative state changes in this plan. Future M2–M5 work must preserve same-seed replay,
canonical reduction, explicit RNG streams, locale independence, and save/resume equivalence.

## Memory impact

None in this rebaseline. Future depth work must budget hot/cold state, provenance growth,
resolution representation, observer projection size, and persistence size before implementation.

## Observer impact

Observer work becomes demand-driven by simulation inspection and Explanation evidence. Bounded
causal slices and domain projections take precedence over decorative views. UI changes are batched
after protocol/read-model contracts stabilize.

## Explanation impact

Explanation becomes a continuously maintained second-priority workstream. Digests may anchor
identity but cannot measure physical distance. Every new mature domain must provide typed metrics,
causal query support, uncertainty behavior, negative controls, and deterministic rendering metadata
before M4/M5 can be claimed.

## Persistence impact

No format change. Future domain maturity cannot exceed M2 unless authoritative state is included in
the canonical persistence inventory and uninterrupted-versus-resumed equivalence is proven.

## Cross-domain effects

The maturity model prevents isolated contract completion from being mistaken for simulation depth.
It forces each domain to document incoming/outgoing carriers, scheduler ownership, resolution,
persistence, observer exposure, Explanation evidence, and performance before maturity claims.

## Risks

- “Maximum detail” may expand scope without measurable value; maturity gates require executable
  causal effects and evidence rather than document volume.
- Explanation may lag simulation; M4/M5 block maturity claims until it catches up.
- UI batching may temporarily hide new internals; bounded diagnostic projections are still required.
- Premature phase numbering may recreate a fake terminal roadmap; only accepted ExecPlans allocate
  new phases.
- Foundation `Completed` labels may still be misread; all status surfaces require the same legend.

## Documentation changes

Architecture rebaseline, roadmap, invariants, domain coverage, Explanation architecture/RFC, LLM
policy, observer/UI cadence, README, documentation index, agent guidance, historical rebaseline
references, changelog, and plans index.

## TODO changes

- Add domain maturity audit/sequencing.
- Promote valid phenomenon analytics and recovery metrics to critical priority.
- Add durable physical-state coupling and production bootstrap/fixture removal.
- Add bounded causal/domain observer inspection.
- Reclassify performance and pending domain work into Detailed Development rather than old phases.

## Decision log

- 2026-07-13: Phases 0–26 are the completed Foundation Era, not proof of mature domain simulation.
- 2026-07-13: Detailed Development has no predetermined phase count or reserved final phase.
- 2026-07-13: Simulation > Explanation > observer protocol > batched UI is the implementation order.
- 2026-07-13: Optional LLM wording is an unnumbered terminal integration gate, not Phase 27.
- 2026-07-13: Digest equality is valid evidence; digest byte distance is not a physical metric.
- 2026-07-13: Production fixtures are architectural debt and must be removed from runtime bootstrap.

## Progress

- [x] Architecture and live-runtime evidence audited.
- [x] Detailed Development policy accepted across architecture documents.
- [x] Phase 27 removed from the active roadmap and historical references marked superseded.
- [x] Domain matrix and TODO backlog rebaselined.
- [x] Explanation and fixture corrections recorded.
- [x] Documentation consistency verified.
- [x] Plan completed and indexed.
