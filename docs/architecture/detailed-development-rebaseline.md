# Detailed Development Rebaseline

**Status:** Accepted

**Effective from:** completion of Phase 26

**Supersedes:** the fixed post-Phase-26 sequence that reserved Phase 27 for optional narrative
surface realization

## Decision

Phases 0–26 form the completed **Foundation Era**. They established minimum valid contracts and the
first executable, replayable, persistent, observable causal loop. They do not assert that every
domain is deeply simulated or that semantic emergence has been demonstrated.

Ontopolis is now in the **Detailed Development Program**. The program has no predetermined number
of phases and no reserved final phase. Future numbered phases are created only by accepted
ExecPlans for bounded, evidence-bearing work. The absence of a fixed endpoint is deliberate: the
required depth must be discovered through integration, long-run experiments, explanation quality,
performance measurements, and architectural gaps rather than guessed in advance.

## Normative priority

```text
AUTHORITATIVE SIMULATION DEPTH
    ↓ enables evidence
EXPLANATION / ANALYTICS DEPTH
    ↓ defines bounded inspection needs
OBSERVER READ MODELS AND PROTOCOL
    ↓ stabilizes presentation contracts
BATCHED UI MILESTONES
    ↓ only after terminal readiness
OPTIONAL LLM SURFACE INTEGRATION
```

This is a priority and dependency order, not permission for Explanation to lag indefinitely.
Explanation must evolve with the simulation so that accepted behavior remains causally inspectable.
UI polish is intentionally batched; diagnostic read models may be added earlier when required to
validate simulation or Explanation.

## Meaning of Foundation completion

A completed Foundation phase means its stated minimal acceptance criteria were met. It may provide
only validated types, isolated deterministic operations, or one bounded integration path. It does
not imply any of the following unless separately demonstrated:

- realistic or comprehensive domain dynamics;
- production historical synthesis;
- meaningful cross-domain emergence;
- representative agent behavior;
- valid domain-level recovery or similarity metrics;
- mature observer coverage;
- accepted performance at target scale.

Completed foundation plans remain historically correct. Their deliberately excluded work becomes
input to Detailed Development rather than being retroactively treated as implemented.

## Capability maturity model

Maturity is assigned to a capability, not to a crate or broad domain name.

| Level | Name | Required evidence |
| --- | --- | --- |
| M0 | Documented | Intent, primitive/emergent boundary, incoming/outgoing carriers, and risks are documented. |
| M1 | Contracted | Validated deterministic types and isolated operations exist; failure cases are tested. |
| M2 | Executable | Production scheduler/bootstrap mutates authoritative domain state through proposal/commit with provenance. |
| M3 | Coupled | Real cross-domain inputs and outputs exist; resolution and persistence preserve the state without semantic shortcuts. |
| M4 | Observable | Bounded read models and domain-valid Explanation metrics reconstruct significant changes with uncertainty and trace support. |
| M5 | Validated | Replay-verified long runs, controls, counterfactuals, negative cases, persistence equivalence, and representative benchmarks support the claimed envelope. |

Foundation work is mostly M1, with selected vertical paths reaching parts of M2–M4. “Mature” is
reserved for capabilities that reach M5 within an explicitly stated envelope.

## Detailed Development acceptance template

Every domain-depth ExecPlan must answer:

1. Which exact capabilities advance, and from which maturity level to which target level?
2. What authoritative state exists, and which scheduler phase owns mutation?
3. Which physical or informational carriers enter and leave the capability?
4. How does it interact with at least one other domain without semantic shortcuts?
5. How is state represented under causal resolution and after save/resume?
6. Which causal traces make changes reconstructable?
7. Which typed metrics and counterfactuals let Explanation distinguish causes, correlations,
   persistence, recovery, insufficiency, and alternatives?
8. Which bounded observer projection is required for validation?
9. What representative workload measures time, memory, provenance growth, and observer overhead?
10. Which negative control proves that an observed effect is not produced by a timer, fixture,
    digest artifact, or observer behavior?

An implementation cannot claim M4 while Explanation cannot inspect it, or M5 while persistence and
representative benchmarks are absent.

## Immediate correction priorities

The live foundation loop exposed four program-level blockers:

1. **Physical causality:** replace counter/sample-only feedback with durable physical, material,
   geometric, terrain, body, or other explicitly owned state changes.
2. **Production bootstrap:** remove fixture-created actors/state from production paths; historical
   bootstrap and resolution promotion must create causally accounted state.
3. **Analytical validity:** use digests only for equality/divergence and introduce typed domain
   state distances, recovery criteria, baselines, and negative controls.
4. **Causal legibility:** add bounded causal and domain inspection read models before decorative UI
   expansion.

These priorities do not predetermine all later sequencing. A domain maturity audit will build the
first accepted set of Detailed Development ExecPlans.

## Explanation continuity policy

Explanation is developed concurrently at second priority:

- new authoritative capabilities register typed observer/explanation schemas;
- metrics operate on domain values or validated state vectors, never arbitrary digest-byte
  arithmetic;
- claims identify observation window, comparison, units/scales, uncertainty, evidence gaps, and
  supporting traces;
- causal queries distinguish state change, causal ancestry, correlation, counterfactual effect,
  persistence, and recovery;
- deterministic rendering remains sufficient to understand every accepted claim;
- unsupported or under-observed behavior remains explicitly unknown.

The objective is that simulation completion does not arrive years before its causal interpreter.

## Observer and UI cadence

Observer contracts are added when required by simulation validation or Explanation. They remain
bounded, versioned, and read-only. The priority is causal slices, domain series, objective versus
subjective comparison, resolution transitions, and provenance-backed state deltas.

The desktop UI is updated at coherent milestones. A new internal field does not automatically earn
a panel. UI work is warranted when a stable read model enables a complete inspection workflow or
when missing presentation blocks validation by humans.

## Terminal optional LLM gate

Optional LLM surface realization is removed from the numbered roadmap. It may be proposed only
after all of the following are true:

- the target simulation scope has M5 evidence for every claimed core capability;
- production execution contains no fixture/demo residents, history, or world state;
- representative long runs produce reproducible, nontrivial cross-domain phenomena;
- deterministic Explanation can answer the important “what happened?” and “why?” questions with
  typed evidence, uncertainty, alternatives, and causal provenance;
- the observer/UI can reveal every structured source packet behind generated prose;
- persistence, determinism, performance, and provenance envelopes are accepted;
- a dedicated RFC demonstrates compliance with INV-011 and retains a no-LLM operating mode.

Satisfying the gate authorizes consideration, not adoption. LLM output remains optional,
non-authoritative, removable, and downstream of validated fact packets.

## Consequences

- Phase 26 is the last preallocated roadmap phase.
- “Phase 27” has no assigned meaning until a future accepted ExecPlan allocates it, if numbering is
  still useful at that time.
- Optional narrative realization has no phase number and no current schedule.
- TODOs use `Detailed Development — <workstream>` instead of pretending unfinished depth belongs to
  an already completed Foundation phase.
- Domain coverage reports both foundation baseline and detailed maturity/gaps.
- UI changes become less frequent while simulation and Explanation changes become substantially
  deeper.

## Related documents

- `plans/detailed-development-rebaseline.md`
- `docs/roadmap/roadmap.md`
- `docs/ontology/domain-coverage-matrix.md`
- `docs/development/todo-backlog.md`
- `docs/explanation/architecture.md`
- `docs/explanation/optional-llm-surface.md`
- `docs/architecture/observer.md`
- `docs/architecture/performance.md`
- `docs/architecture/invariants.md`
