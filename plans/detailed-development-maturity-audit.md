# Detailed Development Maturity Audit

**Status:** Active

**Run ID:** `audit-26026fb3862e-20260715T004000Z`

**Source baseline:** `26026fb3862e8d178a2e59df7a68a2901e80b123`

## Goal

Produce an evidence-backed capability maturity audit for every domain in Ontopolis, deepen the
analysis only for the four blockers identified by the Detailed Development rebaseline, derive an
acyclic next-work sequence, and write one decision-complete downstream ExecPlan. This plan governs
audit, evidence, and documentation work only; it does not implement product behavior.

## Context

The Foundation Era proved minimum contracts and a bounded executable causal loop, but foundation
completion is not domain maturity. The provisional ranges in the domain coverage matrix mix
documentation, isolated contracts, narrow production paths, and observer infrastructure. Before a
new Detailed Development tranche is accepted, those claims must be split into capabilities and
bound to reproducible source, test, production, persistence, observer, Explanation, and benchmark
evidence.

Todo 1 froze the audit at source commit
`26026fb3862e8d178a2e59df7a68a2901e80b123` and tree
`8507defcd090b107eaf695b1289bd42d1ebd2f32`. The authoritative bootstrap record is
`.omo/evidence/audit-26026fb3862e-20260715T004000Z/task-1-bootstrap.json`; the preflight receipt is
`.omo/evidence/audit-26026fb3862e-20260715T004000Z/captures/task-01-preflight.command-receipt.json`.

## Relevant invariants

- INV-001 through INV-010 preserve Ground Truth, subjective access, semantic boundaries,
  geography, and causal-resolution separation.
- INV-011 through INV-013 keep LLM, Explanation, and observer surfaces non-authoritative.
- INV-014 through INV-020 require provenance, phase-controlled mutation, measured performance,
  and inspectable emergence rather than high-level random history.
- INV-021 through INV-026 preserve UI separation and evidence-bearing explanations.
- INV-027 through INV-035 preserve subjective identity, scene, memory, body-schema, and prediction
  boundaries.
- INV-036 and INV-037 preserve explicit spatial scope and separate geometry from containment and
  resolution.
- INV-038 limits digests to identity, equality, and divergence; digest-byte arithmetic is not a
  domain metric.
- INV-039 forbids fixture/demo construction in production initialization.
- INV-040 keeps biological mana interaction external, physical, and non-semantic.

## Ontology domains affected

The shallow audit covers exactly the 30 rows in `docs/ontology/domain-coverage-matrix.md`: Space,
Time, Matter, Energy, Pattern / Feature, Spatial geometry, Geography, Geology, Hydrology, Climate,
Ecology, Biology, Physical access / perception, Cognition, Language, Mana, Causal resolution,
Society, Economy, City infrastructure, Historical bootstrap, Epistemics, Practice, Isekai,
Metaphysics, Simulation runtime, Explanation / analytics, Observer, UI, and Optional LLM surface.

The audit changes no authoritative ontology. It records capability boundaries, evidence, gaps, and
dependencies.

## Causal carriers affected

The audit inventories incoming and outgoing physical, informational, biological, social, and
resolution carriers. These include position and motion, material contact, energy transfer, field
interaction, structural connection, physical utterances, documents, practices, institutional
records, pathogen transmission, inheritance, ecological pressure, role and resource records,
ritual synchronization, spatial organization, provenance traces, and historical-bootstrap
receipts. A capability cannot claim coupling merely because a type or crate exists.

## Relevant documents

- `AGENTS.md`
- `PLANS.md`
- `plans/detailed-development-rebaseline.md`
- `docs/vision/project-thesis.md`
- `docs/vision/uniqueness.md`
- `docs/architecture/invariants.md`
- `docs/architecture/detailed-development-rebaseline.md`
- `docs/architecture/determinism.md`
- `docs/architecture/performance.md`
- `docs/architecture/observer.md`
- `docs/architecture/provenance.md`
- `docs/ontology/domain-coverage-matrix.md`
- `docs/ontology/causal-carriers.md`
- `docs/ontology/lifecycle-audit.md`
- `docs/ontology/primitive-vs-emergent.md`
- `docs/development/todo-backlog.md`
- `docs/roadmap/roadmap.md`
- Relevant accepted ADRs and RFCs cited by each audited capability.

## Current state

- Foundation Phases 0–26 are complete within their bounded acceptance scopes.
- The matrix reports provisional broad-domain ranges rather than capability-level determinations.
- Current production execution includes known blockers: counter/sample-heavy physical feedback,
  fixture-created actors, dormant concrete historical bootstrap, timer-modulo lifecycle behavior,
  digest-byte recovery distance, limited causal/domain observer queries, and diagnostic-only
  benchmark infrastructure.
- Todo 1 found a clean audit worktree and a ready codebase-memory index at the source baseline.
- No audit result, sequencing document, or selected first-slice ExecPlan exists yet.

## Proposed architecture

### Immutable baseline and blob policy

All maturity evidence is evaluated against source baseline
`26026fb3862e8d178a2e59df7a68a2901e80b123`. Source definitions, tests, production roots, and
documentation citations bind to repository-relative path, baseline Git blob OID, and exact symbol
or inclusive line range. Candidate discovery starts from `git ls-tree` at that baseline and is
reconciled with baseline-indexed graph and complete LSP results. A worktree file, generated audit
tool, post-baseline governance edit, test name, green aggregate CI run, or reviewer assertion cannot
substitute for baseline source evidence.

Product-source changes after baseline capture invalidate the run and require a new run ID. Audit
tooling is pinned after its checker/schema stage; later tooling-blob changes likewise restart
collection. Governance documents may be updated by this audit without changing the source baseline,
but each live document revision receives its own blob binding and may not retroactively change
source claims.

### Cumulative maturity semantics

Maturity belongs to a capability, never automatically to a crate or broad domain. Levels are
cumulative and contiguous:

- M0 Documented requires intent, primitive/emergent boundary, carriers, and risks.
- M1 Contracted requires M0 plus baseline-bound deterministic definitions, isolated operations,
  exact tests, and applicable failure cases.
- M2 Executable requires M1 plus a fixture-free production scheduler/bootstrap path that mutates
  authoritative state through proposal/commit with provenance.
- M3 Coupled requires M2 plus real two-domain carrier flow and resolution/persistence preservation.
- M4 Observable requires M3 plus bounded read-only observer projection and domain-valid
  Explanation metrics with units, windows, uncertainty, insufficiency behavior, and traces.
- M5 Validated requires M4 plus replay, save/resume equivalence, controls, counterfactuals,
  negative cases, and representative time/memory/provenance/observer-overhead benchmarks.

The derived level is the highest fully evidenced contiguous prefix. Missing or invalid evidence at
one level caps all higher levels. Policy, validation, observer, persistence, and Explanation helper
capabilities do not claim authoritative mutation maturity; their evidence supports linked
authoritative capabilities.

### Two-depth audit

Depth one is a bounded shallow inventory of all 30 domains. It records stable capability IDs,
authoritative state and mutation ownership, carrier bindings, production reachability, resolution,
persistence, provenance, observer, Explanation, determinism, performance, negative controls,
current contiguous maturity, target maturity, and gaps.

Depth two traces only four blocker families:

1. durable physical causality;
2. production causal bootstrap;
3. analytical validity; and
4. causal legibility and benchmark readiness.

The deep audits trace fixed production roots and evidence paths. They do not expand into an
unbounded review of every implementation.

### Exact artifact list

The planned repository artifacts are:

1. `plans/detailed-development-maturity-audit.md`.
2. `tools/audit/validate-capability-audit.mjs`, `tools/audit/schema-contracts.json`,
   `tools/audit/adapter-contracts.json`, and `tools/audit/fixture-manifest.json`.
3. The 24 schemas under `tools/audit/schemas/`: `capability-audit-input`, `capability-audit`,
   `candidate-manifest`, `sequencing`, `closure-manifest`, `review-receipt`,
   `reviewer-dispatch`, `fixture-manifest`, `run-manifest`, `source-blobs`, `graph-receipt`,
   `lsp-receipt`, `command-intent`, `command-receipt`, `deep-audit-fragment`, `scope-receipt`,
   `authorization-receipt`, `finalize-receipt`, `recovery-receipt`, `bundle-manifest`,
   `bundle-wrapper`, `evidence-execution-manifest`, `attestation-manifest`, and
   `test-reconciliation`, each with its declared valid example and negative fixtures.
4. Portable maturity evidence under
   `docs/evidence/maturity-audit/26026fb3862e/`, including `bundle-manifest.json`.
5. `docs/architecture/capability-maturity-audit.md` with its embedded machine-readable audit.
6. `docs/architecture/detailed-development-sequencing.md` with its embedded dependency graph and
   deterministic selection.
7. `plans/detailed-development-first-vertical-slice.md`, the single downstream Draft ExecPlan.
8. Synchronized governance edits to `docs/ontology/domain-coverage-matrix.md`,
   `docs/development/todo-backlog.md`, `docs/roadmap/roadmap.md`, `docs/index.md`, `PLANS.md`, and
   `docs/development/changelog.md`.
9. Run-local command intents, receipts, projections, manifests, deep-audit fragments, review
   receipts, scope/finalize records, and negative-test evidence under
   `.omo/evidence/audit-26026fb3862e-20260715T004000Z/`. These operational records are ignored
   working evidence until portable evidence is explicitly bundled.

No `.rs`, `.ts`, `.tsx`, `.proto`, Cargo/package manifest, runtime asset, persistence schema, or UI
asset is an artifact of this plan.

## Primitive vs emergent review

Every capability records which lower-level physical or computational primitive justifies its state
and which labels remain subjective, social, or observer-side. Maturity evidence is rejected when it
promotes species, disease, profession, class, skill, institution, settlement, ritual, technology,
historical period, or other human interpretation into authoritative meaning. Observer analytics may
classify evidence but cannot mutate or become simulation state.

## Non-goals

- Implementing any product-code capability or changing runtime behavior.
- Implementing durable physical causality, production bootstrap, analytics, observer queries,
  performance infrastructure, biological mana, UI features, or optional LLM integration.
- Assigning an automatic Phase 27, fixed final phase count, or all-domains mega-phase.
- Awarding maturity from crate existence, type definitions, test names, green CI, trace volume,
  fixture/demo execution, timer events, or digest distance.
- Editing unrelated plans or starting the selected downstream ExecPlan.

## Implementation stages

1. Freeze the clean source baseline, worktree identity, graph index, source-plan hash, and target
   preimages.
2. Create and activate this native ExecPlan before audit collection.
3. Add the zero-dependency audit checker, closed schemas, contracts, examples, and negative fixtures.
4. Inventory and reconcile capability candidates, baseline blobs, symbols, tests, bindings, and
   planned evidence for all 30 domains.
5. Run the four fixed deep blocker audits against baseline production roots.
6. Materialize and validate the canonical capability maturity audit and portable evidence bundle.
7. Derive the acyclic sequencing graph and deterministically select one first tranche or bounded
   prerequisite-remediation set.
8. Write exactly one downstream Draft ExecPlan without implementing it.
9. Synchronize the matrix, backlog, roadmap, documentation index, plan registry, and changelog.
10. Run regression, mutation, scope, semantic, closure, and attestation gates before marking this
    plan complete; closure remains separately user-authorized.

## Verification

- Validate all 24 native headings in this plan and their order; prove the checker rejects a copy
  missing `Persistence impact`.
- Validate closed schemas, adapter contracts, examples, fixtures, and exact diagnostics.
- Reconcile all 30 domains with zero unmapped source candidates or unresolved exact-test mappings;
  lower maturity rather than infer missing evidence.
- Validate each deep-audit fragment against its fixed roots, candidate/test manifests, and receipts.
- Materialize maturity from admitted evidence and reject non-contiguous or unsupported M2–M5 claims.
- Validate the portable bundle, sequencing graph, total selection, selected plan, links, governance,
  exact staged sets, and baseline-to-HEAD scope.
- Run `cargo run -p xtask -- ci`, `pnpm install --frozen-lockfile`, `pnpm lint`,
  `pnpm typecheck`, and `pnpm build` without modifying product code.
- Run `git diff --check` for baseline-to-HEAD, staged, and unstaged changes.

## Benchmark plan

This audit makes no runtime performance claim. It records whether each capability targeting M5 has
a representative bounded workload with warm-up, sample count or duration, time metrics, memory
metrics, provenance-growth metrics, observer-off/on overhead, and an acceptance envelope. Existing
diagnostics cannot satisfy M5. Missing representative evidence lowers the capability level or is
recorded as a future target.

## Determinism impact

The audit does not mutate authoritative state. Evidence collection fixes locale, timezone, color,
working directory, command arguments, tool versions, baseline blobs, canonical ordering, and
adapter-specific deterministic projections. Future capability claims must identify RNG streams,
proposal ordering, replay anchors, and save/resume equivalence; audit receipt byte hashes preserve
integrity but do not themselves prove simulation determinism.

## Memory impact

No simulation memory layout changes. The audit records missing hot/cold-state budgets, persistence
size, provenance growth, bounded observer projection size, and representative peak/steady-state
memory measurements. Run-local raw evidence remains outside tracked source; the portable bundle
contains canonical projections and hashes rather than host-specific raw command streams.

## Observer impact

No observer protocol or UI code changes. The audit determines whether capabilities have bounded,
versioned, read-only projections for causal slices, domain series, state deltas, resolution
transitions, and objective/subjective separation. Missing inspection required for validation is a
recorded maturity gap, not a reason to add decorative UI in this plan.

## Explanation impact

No Explanation implementation changes. M4 and M5 claims require typed domain values or validated
state vectors, units and scales, comparison windows and baselines, uncertainty and insufficiency
behavior, alternatives or counterfactuals, and supporting traces. Digest equality may anchor a
comparison; digest-byte distance cannot support recovery, stability, or physical similarity.

## Persistence impact

No persistence format changes. The audit inventories whether authoritative capability state is
included in canonical snapshots and whether uninterrupted and save/resume runs are equivalent.
Missing persistence prevents M3 and therefore every higher maturity claim. Audit evidence itself is
preserved through immutable run receipts and a separately validated portable bundle.

## Cross-domain effects

The shallow inventory exposes gaps across all domains without claiming that every dependency is
ready. The deep audits test the four cross-cutting blockers where false maturity would most distort
sequencing. The resulting graph may express `requires`, `blocks`, `can_parallelize`, and
`evidence_only`; only `requires` participates in cycle rejection. Selection preserves the program
priority of simulation, concurrent Explanation support, validation-driven observer contracts, and
batched UI work.

## Risks

- A documentation-heavy inventory could be mistaken for maturity; only typed admitted evidence
  grants a level.
- Incomplete source or test discovery could silently inflate results; incomplete graph/LSP/test
  reconciliation instead caps affected capabilities.
- The audit could expand indefinitely; shallow coverage is fixed at 30 domains and deep coverage at
  four blocker families.
- Generated audit tooling could validate itself; run-owned tooling may check evidence but cannot
  supply maturity evidence.
- Governance edits could drift from the baseline; source claims remain baseline-blob-bound and live
  document blobs are recorded separately.
- A selected downstream plan could be started prematurely; this plan produces it as Draft only.

## Documentation changes

Create the canonical maturity audit, sequencing document, and downstream Draft ExecPlan. Update the
domain matrix, backlog, roadmap, documentation index, plan registry, and changelog only to reflect
validated audit results and lifecycle status. Keep completed Foundation plans historically intact.

## TODO changes

`TODO-DEPTH-001` remains Pending while this plan is Active. `TODO-SIM-001` and
`TODO-RUNTIME-001` remain Pending, and `TODO-BIO-003` remains Proposed. Audit evidence may be added
without rewriting pinned goals, acceptance criteria, or dependencies. Only separately authorized
closure may mark `TODO-DEPTH-001` complete.

## Decision log

- 2026-07-13: Use source baseline `26026fb3862e8d178a2e59df7a68a2901e80b123` for all product
  source and test evidence.
- 2026-07-13: Audit all 30 matrix domains shallowly and only the four accepted program blockers
  deeply.
- 2026-07-13: Derive maturity as the highest contiguous cumulative M0–M5 level supported by typed
  evidence.
- 2026-07-13: Treat fixtures, timers, digest distance, observer behavior, and diagnostic benchmarks
  as explicit negative controls or gaps, never substitutes for domain causality.
- 2026-07-13: Produce one downstream Draft plan by deterministic readiness/remediation selection;
  do not implement or start it in this audit.
- 2026-07-13: Keep product-code files outside this plan's allowed changes.
- 2026-07-15: V11 evidence passed: clean bootstrap/preflight; 132 baseline sources; 19 graph
  queries plus an absence receipt; LSP 105 complete/27 incomplete; 61 semantic rows; 4 endpoints;
  inventory 30/0/0/0; 19 hardening checks; reproducibility 334; and two independent audit passes.
- 2026-07-16: Canonical evidence and the 20/20 audit suite passed with 132 sources, 105 complete
  and 27 explicitly incomplete LSP captures, 61 semantic rows, and zero residual rust-analyzer
  processes after the direct stdio session shutdown.

## Progress

- [x] Todo 1: freeze run identity, baseline SHA/tree, graph status, and clean audit worktree.
- [x] Todo 2: create and activate this native ExecPlan.
- [x] Todo 3: commit checker, schemas, examples, and negative fixtures. (Canonical evidence and the 20/20 audit suite passed; the direct Rust-LSP stdio session shut down with zero residual rust-analyzer processes.)
- [x] Todo 4: reconcile the shallow 30-domain capability and source inventory. (Canonical evidence records 132 baseline sources, 105 complete and 27 explicitly incomplete LSP captures, and 61 semantic rows.)
- [ ] Todos 5–8: complete the four deep blocker audits.
- [ ] Todo 9: materialize the canonical audit and portable evidence bundle.
- [ ] Todo 10: validate the acyclic sequencing and total first-tranche selection.
- [ ] Todo 11: write the single downstream Draft ExecPlan.
- [ ] Todos 12–14: synchronize governance and prepare deterministic closure.
- [ ] Final verification: pass semantic, regression, mutation, and scope reviews.
- [ ] User-authorized closure: apply validated replacements, commit, and attest completion.
