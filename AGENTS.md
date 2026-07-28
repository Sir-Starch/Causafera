# Causafera Agent Guidelines

Shared guidance for every coding agent working in this repository, regardless of tool or harness.

## Project purpose

Causafera is a deterministic causal world-simulation engine. Physical processes, bounded subjective
agents, language, institutions, history, and an information-sensitive mana field interact to produce
traceable outcomes. Every accepted behaviour must be replayable, causally inspectable, and grounded
in simulated state rather than in authored labels. See the
[project thesis](docs/vision/project-thesis.md) and
[what distinguishes Causafera](docs/vision/uniqueness.md).

## Scope and authority

This file applies repository-wide. A more specific `AGENTS.md` nested under a subdirectory overrides
this one for the paths it covers. Direct instructions in the current task take precedence over both.

Source authority, in order of what each source actually settles:

- **Intended contracts** — [hard invariants](docs/architecture/invariants.md), accepted ADRs in
  `docs/adr/`, accepted RFCs in `docs/rfc/`, and canonical ExecPlans in `plans/`.
- **Implemented behaviour** — current source and tests. Documentation describes intent; code decides
  what exists today.
- **Project status** — [roadmap](docs/roadmap/roadmap.md),
  [maturity matrix](docs/ontology/domain-coverage-matrix.md),
  [TODO backlog](docs/development/todo-backlog.md), and [CHANGELOG.md](CHANGELOG.md).
- **Not authoritative** — generated summaries, agent memory, tool indexes, chat history, and session
  transcripts are non-authoritative navigation hints and provenance, not contracts. Completed and
  historical plans are not current implementation guidance; they may be consulted for accepted
  decisions and provenance that have not been explicitly superseded by later canonical sources.

When sources conflict, resolve the conflict explicitly: state which source you followed and why, and
either fix the stale source or record the discrepancy. Do not silently pick one.

## Current development model

- Phases 0–26 are the completed historical **Foundation Era**. They proved minimum valid contracts
  and the first executable causal loop.
- They do **not** assert current maturity, domain depth, or present implementation priorities.
- Current work belongs to the open-ended **Detailed Development Program**
  ([rebaseline](docs/architecture/detailed-development-rebaseline.md)). It has no predetermined
  number of phases and no reserved final phase.
- Priority order: authoritative simulation depth and cross-domain coupling first, then
  Explanation/analytics depth, then the observer read models and protocol required for inspection,
  then coherent batched UI milestones. Explanation must keep pace well enough that accepted
  behaviour stays causally inspectable.
- Derive what to work on from active plans, the roadmap, the backlog, and evidence-backed maturity —
  never from a Foundation Era phase number.
- Optional LLM surface realization is not a numbered phase and remains blocked until the terminal
  gate in the rebaseline is satisfied.

## Task-scoped reading

Read what the task requires. Do not read a fixed document list before every change, and do not
recursively read the documentation tree.

- Always: current code, tests, and `git status` for the surfaces you are touching.
- Use [`docs/index.md`](docs/index.md) as navigation to find the right document, not as a reading list.
- Architectural or authoritative behaviour change: the [invariants](docs/architecture/invariants.md),
  the relevant subsystem docs, relevant ADRs and RFCs, and the active ExecPlan.
- Foundational concept change: the [project thesis](docs/vision/project-thesis.md) and
  [uniqueness](docs/vision/uniqueness.md) documents.
- Historical plans under `plans/history/`: only when their decisions or provenance bear on the
  current change.
- [`CONTRIBUTING.md`](CONTRIBUTING.md): when contribution workflow, validation, governance,
  licensing, or pull-request requirements matter.

## Planning and scope

- Substantial architectural, multi-stage, cross-domain, persistence, protocol, or performance work
  requires an ExecPlan following [`PLANS.md`](PLANS.md).
- A bounded local fix does not require an ExecPlan.
- ExecPlans under `plans/` are authoritative. Tool-specific execution artifacts elsewhere are
  non-authoritative adapters.
- Material decisions discovered during implementation — scope changes, rejected alternatives,
  contract adjustments — must be recorded in the canonical plan's Decision Log and Progress sections.
- Keep scope bounded. No unrelated opportunistic implementation, refactoring, or drive-by edits.

## Non-negotiable architecture

The full contract is [`docs/architecture/invariants.md`](docs/architecture/invariants.md) (INV-001
through INV-043). The broadly applicable rules:

- **Deterministic mutation.** Authoritative state changes follow the scheduler's
  proposal/reduce/commit boundary. Nothing mutates authoritative state outside it.
- **Explicit RNG.** Randomness comes from explicit deterministic streams. Never from thread
  scheduling, system time, locale, pointer identity, or hash-map iteration order.
- **Provenance and conservation.** Significant state changes carry causal traces. Conserved
  quantities are accounted for across transfer, aggregation, and resolution changes.
- **No Ground Truth in cognition.** Agents never receive authoritative identity (`EntityId`,
  `PlaceId`, chart/frame IDs) or omniscient state. Perception flows through a constructed subjective
  scene; objective body state and subjective body schema stay distinct, as do persistent
  autobiographical memory and active working context.
- **No semantic labels as meaning.** No convenience domain enums, no human-language strings, and no
  developer analytical labels as authoritative simulation meaning.
- **Mana reads structure.** Mana responds to measurable physical structure and information, never to
  semantic concepts or named schools.
- **Physical domains are causal state.** Geography, biology, matter, and energy are simulated causal
  state, not decoration or backdrop.
- **Chunks are addressing.** Chunks are addressing and computation units. They are not physical
  walls, not metric units, and not geometry. Containment defines neither adjacency nor distance nor
  ownership. Causal resolution may change detail, never topology or geometry.
- **Downstream layers are non-authoritative.** Explanation, observer, UI, analytics, and any optional
  LLM wording read state and never mutate it, and never feed classifications back into simulation.
- **Digests are identities.** State digests anchor equality and divergence only. Digest-byte distance
  is never a physical, semantic, recovery, or stability metric.
- **No fixtures in production.** Fixture and demo constructors must not appear in production
  bootstrap or runtime sessions. Production state requires causal initialization.
- **Modular architecture.** Keep modules cohesive and strictly scoped. Do not accumulate unrelated
  state and methods in a single module such as `runtime.rs`; prefer named sibling modules.
- **Deliberate versioning.** Persistence formats, observer read models, wire/protocol schemas,
  Explanation output, and digest definitions are versioned intentionally. Changing one is a contract
  change, not an implementation detail.

## Implementation practice

- Capture a failing test or a faithful failing scenario before changing behaviour. Documentation
  changes are verified for factual accuracy, executable commands, and link validity instead.
- Implement the smallest change that satisfies the accepted scope; pair implementation with its
  direct tests.
- Cover, where applicable: replay equivalence, input-order independence, observer-locale
  independence, negative controls and counterfactuals, save/resume equivalence, causal ancestry, and
  resolution promotion/demotion conservation.
- Do not implement hunger systems, random jobs, fake cities, invented vocabularies, placeholder
  magical schools, demo residents, or fabricated history.
- Do not make unverified emergence, maturity, or scale claims.

## Code discovery and tools

Use whatever tooling is available; no specific tool is a repository requirement.

- **Exact text search** for identifiers, TODO IDs, filenames, error strings, configuration keys,
  documentation, and literals.
- **LSP, call hierarchy, or an available semantic code graph** for architecture, relationships, call
  paths, and change-impact analysis.
- MCP servers and semantic indexes are optional conveniences, not requirements.
- Keep queries narrow and bounded. Do not run the same discovery twice through two mechanisms unless
  you are verifying a concrete uncertainty.
- Read current source before relying on a generated index, cached graph, or prior summary.

## Git safety

Protecting unrelated and uncommitted work outranks convenience.

- Inspect `git status` before and after making changes.
- Never run worktree-discarding commands — `git checkout` on paths, `git restore`, `git reset`,
  `git clean`, `git stash` — unless the task explicitly authorizes them for exact paths and a stated
  purpose.
- Never overwrite a dirty file from another snapshot, branch, or commit. Recovery uses read-only
  object inspection plus an explicit path/hash allowlist, preserving every unrelated dirty path.
- Stage explicit paths. A broad `git add .` is not an acceptable procedure.
- Do not amend, rebase, force-push, rewrite history, or delete branches outside explicit task scope.
- **Checkpoints for accepted plans.** Create a local checkpoint commit after each independently
  verified green wave. Never checkpoint a RED, uncompilable, or partially integrated state, and never
  carry multiple completed waves only in the working tree. Before each checkpoint, inspect status and
  both staged and unstaged diffs, stage only the wave's allowlist, rerun its focused verification, and
  record the commit hash and evidence in the ExecPlan Progress section. If checkpoint commits are
  prohibited, stop after one green wave and request authorization.
- **Delegation.** A delegated writing agent does not own Git state and must not run staging, commit,
  or worktree-discarding commands unless its task is explicitly Git-only. The coordinating agent owns
  integration, recovery, staging, and commits.

## Validation

Run focused checks while iterating; run broad checks for the surfaces you actually changed.
Documentation-only work does not automatically require the full runtime suite.

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo test --workspace --no-default-features
cargo run -p xtask -- ci
pnpm lint
pnpm typecheck
pnpm build
git diff --check
node tools/audit/check-entry-points.mjs
node tools/audit/run-source-tests.mjs
```

- Never report a skipped, unavailable, interrupted, or failing check as passing. Say what did not run
  and why.
- Performance and scale claims require representative, reproducible measurements — never estimates.

The complete pinned toolchain and full validation suite are in [`CONTRIBUTING.md`](CONTRIBUTING.md).

## Documentation

Update a document when the fact it records changes — not as a reflex for every commit.

- **Subsystem docs** when behaviour or architecture changes.
- **ADR, RFC, invariant, or ExecPlan** when its decision or contract changes.
- **[CHANGELOG.md](CHANGELOG.md)** for notable changes.
- **[TODO backlog](docs/development/todo-backlog.md)** when a TODO opens, changes, or closes.
- **[Roadmap](docs/roadmap/roadmap.md)** and
  **[maturity matrix](docs/ontology/domain-coverage-matrix.md)** only when project status, priority,
  scope, or evidence-backed maturity actually changes.

Link canonical sources instead of copying them. Preserve terminology: Ground Truth, subjective scene,
Explanation Engine, observer, and mana each have a distinct architectural meaning.

## Completion reporting

Report what happened, not what was intended:

- every file changed, and why;
- material decisions and assumptions made during the work;
- the validation commands run and their actual results, including failures, skips, and anything the
  environment could not execute;
- work that was in scope but left undone, and why;
- any claim that is not yet backed by evidence, labelled as such.
