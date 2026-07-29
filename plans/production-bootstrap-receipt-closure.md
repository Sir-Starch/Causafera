# Canonical Production Bootstrap Receipt Closure

**Status:** Implemented — waves 1-5 green, plus seven audit-hardening rounds; verification below is
re-run and re-recorded each round

**Accepted:** 2026-07-28

**Execution mode:** One Claude Code session, sequential waves, no OMO, no delegated workers.

## Goal

Close the contract gap between the six-stage production bootstrap in
`causafera-runtime` and the canonical historical DAG/receipt model in
`causafera-world`.

The result is a bounded, deterministic, causally inspectable production bootstrap record that:

- represents the current six runtime stages through one canonical plan;
- emits exactly one terminal receipt per stage, including stages with no domain effects;
- persists and fail-closed validates the plan, receipts, result fingerprints, and trace ancestry;
- is identical across runtime, observer session, reset, experiment, replay, and save/resume entry
  points for the same seed and recipe;
- exposes a bounded observer/Explanation read model for bootstrap receipts and initial conditions;
- proves that fixture/demo actor constructors are not production initialization paths.

This plan advances only the existing bounded bootstrap contract. It does not claim mature historical
synthesis, emergence, or world-scale simulation.

## Context

The accepted Detailed Development priority includes production bootstrap without fixtures, alongside
durable physical causality, valid analytics, and causal inspection. `TODO-RUNTIME-001` remains
Critical/In Progress. The repository has a real six-stage runtime bootstrap, but it is represented by
a runtime-local `HistoricalBootstrapPlan` while `causafera-world` has a separate canonical
`HistoricalBootstrapPlan` with deterministic stage seeds and receipt validation.

The runtime currently exports `BootstrapReceiptSnapshot { receipts: Vec<BootstrapReceiptRecord> }`
but writes an empty receipt list. The population/bootstrap persistence section currently stores only
`stage` and `trace` pairs at section major 1. The runtime still contains an unused public
`fixture_actors` constructor that creates stationary actors and fixture sensors; the absence of
production callers must become a mechanically checked contract, and the helper must be test-only or
removed if no tests require it.

## Relevant invariants

- **INV-014:** significant state changes retain provenance.
- **INV-016:** authoritative mutation stays inside the scheduler's phase boundary.
- **INV-019:** accepted behaviour remains causally inspectable.
- **INV-012–INV-013:** Explanation and observer are non-authoritative and cannot feed back.
- **INV-027–INV-031:** subjective observers never receive Ground Truth identity or omniscient state.
- **INV-037:** chunk addressing is not physical geometry, adjacency, or ownership.
- **INV-038:** digests are equality/divergence identities, never physical or semantic distances.
- **INV-039:** production state requires causal initialization.
- **INV-042:** runtime/bootstrap/persistence/observer responsibilities remain modular.
- **INV-043:** the world is one coherent spatial system, addressed rather than partitioned.

An earlier revision of this list cited INV-001–004 for determinism, INV-012–016 for the mutation
boundary, INV-021 for provenance and INV-040 for persistence. Checked against
`docs/architecture/invariants.md`, three of the four were wrong and the fourth was over-broad:

- INV-001–002 are omniscience and observation-is-not-Ground-Truth, INV-003–004 are mana invariants —
  none of the four is about determinism.
- INV-021 is the UI-as-observer rule, not provenance; provenance is INV-014.
- INV-040 is biological mana interaction, not persistence.
- INV-012–016 did contain the right invariant: **INV-016 is** the mutation boundary. The range was
  wrong only in reaching four further numbers that are not — INV-012–013 (Explanation and observer
  non-authority), INV-014 (provenance) and INV-015 (no random high-level history).

A previous revision of this very paragraph called all four citations wrong and glossed INV-012–016
as Explanation non-authority, which is accurate for two of the five numbers and not for INV-016.
Corrected against the source rather than left as a plausible-looking correction.

## Ontology domains affected

- Historical bootstrap and simulation runtime: current narrow M1/M2 path becomes a durable,
  inspectable contract without claiming M5.
- Population and actor promotion: bootstrap ancestry and aggregate/promoted conservation become
  explicit evidence.
- Provenance and persistence: bootstrap plan/record data becomes authoritative snapshot state.
- Observer and Explanation: bounded read models expose receipts, result fingerprints, and traces.

No new physical domain is introduced.

## Causal carriers affected

- Existing carriers reused: terrain generation, material-surface creation, population aggregate
  creation, actor promotion, material activity, and thermal field/reservoir initialization.
- New bounded information carrier: canonical bootstrap plan and one terminal receipt per stage.
- Existing causal trace store remains the provenance authority. The receipt record must reference
  committed traces; it must not become an independent causal history.

## Relevant documents and implementation surfaces

- `PLANS.md`
- `docs/architecture/invariants.md`
- `docs/architecture/detailed-development-rebaseline.md`
- `docs/architecture/determinism.md`
- `docs/architecture/provenance.md`
- `docs/architecture/observer.md`
- `docs/rfc/RFC-PERSIST-001.md`
- `docs/world/historical-bootstrap.md`
- `docs/ontology/domain-coverage-matrix.md`
- `docs/development/todo-backlog.md` (`TODO-RUNTIME-001`, `TODO-OBSERVER-003`, `TODO-EXPLAIN-003`)
- `crates/causafera-world/src/historical.rs`
- `crates/causafera-runtime/src/bootstrap.rs`
- `crates/causafera-runtime/src/runtime.rs`
- `crates/causafera-runtime/src/snapshots.rs`
- `crates/causafera-runtime/src/snapshot_sections.rs`
- `crates/causafera-runtime/src/actors/state.rs`
- `crates/causafera-runtime/src/population.rs`
- `crates/causafera-runtime/tests/thermal_bootstrap.rs`
- `crates/causafera-runtime/tests/material_surface_loop.rs`
- `crates/causafera-runtime/tests/material_surface_observer.rs`
- `crates/causafera-observer-api/src/query.rs`
- `crates/causafera-observer-wire/src/protocol.rs`
- `packages/observer-protocol/src/index.ts`
- `crates/causafera-explanation/src/ir.rs`
- `apps/observer/src-tauri/src/session.rs`
- `crates/causafera-lab/src/experiment.rs`

## Current state

1. `causafera-world::HistoricalBootstrapPlan` owns private canonical plan fields, deterministic
   `stage_seed`, and `validate_receipts`; it is currently reached by tests rather than the runtime
   production path.
2. `causafera-runtime::HistoricalBootstrapPlan` owns six executable stage structs and
   `for_runtime_config`; it runs terrain, material surface, population, actor promotion, material
   activity, and thermal reservoir stages in a fixed order.
3. Existing stage effects are committed through `CausalTraceStore` in `Phase::Lifecycle`, usually at
   simulation time zero, and are chained through `latest_physical_trace`.
4. `RuntimeState::export_snapshot` currently sets `bootstrap.receipts` to `Vec::new()`.
5. `encode_population_section`/`decode_population_section` currently encode only stage and trace
   for bootstrap receipts and do not encode a canonical plan or receipt result/causes.
6. `RuntimeState::import_snapshot` imports population and other state but does not validate a
   canonical bootstrap record.
7. `fixture_actors` has no current graph callers, but remains a production-source constructor and
   creates authored stationary bodies and fixture sensors.
8. `ObserverSnapshot` and the versioned observer protocol expose runtime summary, chunks, fields,
   and Explanation IR, but no bootstrap receipt summary.
9. Existing actor/material/mana and thermal plans prove the current production path and persistence
   foundations; this plan must reuse those seams and must not reopen their completed scopes.

## Proposed architecture

### 1. One canonical plan with an explicit runtime adapter

Rename the runtime-local `HistoricalBootstrapPlan` to `RuntimeBootstrapRecipe`. It remains the
executable configuration for the six current stage adapters. Add a canonical
`causafera_world::HistoricalBootstrapPlan` field to the recipe rather than maintaining two types
with the same name.

`RuntimeBootstrapRecipe::from_runtime_config` must construct the canonical plan from the validated
runtime configuration and the sorted active chunk set. The six stage IDs remain stable numeric
identities. Process schema IDs remain opaque numeric IDs; they are not human-language event names.
The stage dependency graph is the current ordered chain:

```text
stage 1 → stage 2 → stage 3 → stage 4 → stage 5 → stage 6
```

The adapter must:

- derive the canonical world seed from `RuntimeConfig::deterministic.world_seed`;
- derive sorted typed `ChunkId` targets from the active `ChartChunkCoord` set using a
  domain-separated deterministic addressing function; this is identity/addressing only and never
  a metric or geometry value;
- derive each stage parameter fingerprint from the complete canonical encoding of that stage's
  executable configuration, including all current fixed values and configured values;
- use the canonical `HistoricalBootstrapPlan::stage_seed` for any stage-local deterministic
  variation;
- preserve the current runtime stage order and all existing stage effects;
- reject duplicate, unsorted, inactive, or out-of-bound targets before any authoritative mutation.

The canonical historical stage timeline is a deterministic bootstrap ordering timeline, not a
request to advance `RuntimeState::advanced_through`. Use six non-overlapping one-unit intervals
`[0,1]`, `[1,2]`, …, `[5,6]` for canonical metadata, while preserving the existing runtime
Lifecycle trace timestamp convention until a separate accepted time contract changes it. Tests must
assert that constructing the recipe does not advance scheduler time.

### 2. Exactly one terminal receipt per stage

Add a runtime-owned `BootstrapRuntimeState` containing the canonical plan, sorted stage receipts,
and bounded stage-result fingerprints. Store it in `RuntimeState` and include it in the physical
state digest as authoritative equality input. Bump `CURRENT_DIGEST_SCHEMA_VERSION` because the
digest definition changes; never compare digest bytes as a distance.

Each executable stage returns its committed effect traces as it does today. The stage coordinator
then performs one deterministic terminal completion commit:

- sort and deduplicate the stage effect traces;
- compute a `StateFingerprint` from the canonical, sorted stage output projection, not from digest
  byte distance and not from a semantic label;
- commit a real bounded bootstrap-stage-result state effect from the stage's absent/sentinel value
  to that result fingerprint;
- make the completion event's causes include the previous stage receipt traces and all effect traces
  produced by this stage;
- create one `HistoricalStageReceipt` whose `trace` is the completion trace, `result` is the stage
  result fingerprint, `completed_at` is the canonical stage end, and `causes` is exactly the sorted
  dependency receipt list required by `HistoricalBootstrapPlan::validate_receipts`;
- make an empty stage produce a completion receipt through the same real stage-result state effect;
- validate the complete record before exposing the constructed runtime.

The completion event is not decorative metadata: its effect is the authoritative transition of the
bounded bootstrap-stage-result state, and its trace must exist in the causal trace store. Existing
per-object stage effects remain and are included as causes, so the receipt cannot hide the detailed
ancestry.

### 3. Versioned persistence

Expand `BootstrapReceiptSnapshot` into a versioned representation of the canonical plan, stage
results, and receipts. It must preserve, in canonical order:

- plan ID and world seed;
- every stage ID, process schema ID, canonical start/end, detail ordinal, target list, dependency
  list, external causes, and parameter fingerprint;
- every receipt stage, completion time, result fingerprint, completion trace, and sorted causes;
- bounded stage-result entries and the maximum six-stage current envelope.

Bump the `SECTION_POPULATION_BOOTSTRAP` section major from 1 to 2. Keep the section ID stable and
reject every unsupported major, truncated payload, trailing bytes, duplicate ID, unsorted list,
missing stage, invalid target, forged trace, forged result, forged cause, and incomplete receipt
set. Do not silently default old empty bootstrap receipts into an authoritative production state.

`RuntimeState::import_snapshot` must reconstruct the canonical plan, call canonical receipt
validation, verify all receipt/completion traces exist in the imported trace store, verify completion
effects match stored stage results, verify target chunks are active, and verify population aggregate
counts plus promoted actor ancestry before returning authoritative state.

### 4. Bounded observer and Explanation projection

Extend the existing runtime summary projection with a bounded bootstrap summary rather than adding a
UI panel or exposing runtime storage. The summary contains at most six records and exposes only:

- opaque plan ID and world-seed identity needed for equality/replay inspection;
- stage ID, completion time, result fingerprint, completion trace, and dependency trace anchors;
- completeness/validation status and bounded promotion/population counts.

Use the existing versioned path:

`RuntimeState → RuntimeSnapshot → ObserverSnapshot → observer wire protocol → TypeScript observer protocol`.

Add optional/repeated fields after the existing runtime-summary fields so old fields retain their
meaning. Update the Rust encoder/decoder, protocol fixtures, TypeScript encoder/decoder types, and
protocol version/compatibility documentation deliberately. If the current protocol version cannot
represent the additive fields without ambiguity, introduce the next protocol version and update all
in-repository clients in the same wave; never overload an existing field with a new meaning.

Add one typed Explanation IR claim for a validated bootstrap record. The claim reports the bounded
observation window, stage count/completeness, result identity, and supporting trace anchors. Missing
or insufficient evidence must produce the existing unsupported/insufficient state with zero
confidence; the claim must not render historical process names, intention, purpose, or semantic
causes.

### 5. Fixture-free production entry points

Audit every production construction path that can create `Runtime` or resume `RuntimeState`:

- `Runtime::new` and runtime bootstrap construction;
- observer `session_config`/session reset and reconnect;
- `causafera-lab` experiment runners and replay setup;
- snapshot import/save-resume paths;
- production binaries and non-test examples.

All paths must use the same `RuntimeBootstrapRecipe` and produce byte-identical canonical plan and
receipt data for the same seed/config. Move `fixture_actors`/`fixture_sensors` into test-only support
or remove them if no tests require them. Tests may retain fixtures only when the test explicitly
needs an isolated primitive; no fixture helper may be reachable from production bootstrap.

## Primitive vs emergent review

Authoritative primitives are typed stage IDs, process schema IDs, stage parameters, seed
contributions, target addresses, causal traces, result fingerprints, population counts, actor
ancestry, and versioned snapshot fields.

Emergent or observer-only interpretations include historical narratives, named events, intentions,
roles, settlements, language, social meaning, and claims of mature world history. None may be added
to the bootstrap record or used as a causal input.

## Non-goals and Must-NOT-Have

- No new geology, hydrology, climate, ecology, biology, language, economy, settlements,
  institutions, authored lore, or deep-history synthesis.
- No fixture/demo residents, authored history tables, timer-driven bootstrap, or random high-level
  history generation.
- No semantic event enums or human-language strings as authoritative process identity.
- No replacement of the canonical `causafera-world` contract with a third parallel bootstrap model.
- No digest-byte similarity, recovery tolerance, or physical-distance metric.
- No direct Ground Truth exposure to cognition or observer-controlled mutation.
- No broad observer graph dump, UI redesign, LLM surface, or new operator API.
- No opportunistic cleanup of unrelated runtime, thermal, mana, or terrain code.

## Implementation stages

Each numbered item is one sequential Claude Code wave. Start every wave by inspecting `git status`
and the current diff. Before changing behaviour, add or enable the named RED test/scenario. Do not
start the next wave until the current wave's focused tests and diagnostics pass and its checkpoint
commit is created with explicit paths only.

- [x] 1. Establish the canonical bootstrap recipe and terminal receipt RED/green contract

  **Files:** `crates/causafera-world/src/historical.rs`,
  `crates/causafera-runtime/src/bootstrap.rs`, `crates/causafera-runtime/src/runtime.rs`,
  `crates/causafera-runtime/tests/historical_bootstrap.rs` (new), and the nearest runtime module
  exports.

  **Implementation:**

  - Add failing tests for six canonical stages, stable dependencies/targets/parameter fingerprints,
    exactly one receipt per stage, empty-stage completion, canonical validation, and unchanged
    scheduler time.
  - Add the smallest public accessors/snapshot constructors needed in `causafera-world` to convert
    the canonical plan and record without exposing mutable internals.
  - Rename the runtime-local plan to `RuntimeBootstrapRecipe`, add the explicit canonical plan
    adapter, and preserve the six existing executable stage structs and their effect owners.
  - Add `BootstrapRuntimeState` to `RuntimeState` and a stage coordinator that commits one real
    completion transition per stage, including no-op stages.
  - Ensure stage receipt causes and event causes are sorted, deduplicated, bounded, and deterministic.
  - Bump `CURRENT_DIGEST_SCHEMA_VERSION` and include the canonical bootstrap state in
    `physical_state_digest`; add the full-rescan/equality assertions required by the existing digest
    tests.

  **Acceptance:** `Runtime::new` produces a validated six-receipt record for the default observer
  recipe; same seed/config produces byte-identical plan/receipts, ancestry, physical digest, and
  history digest; different stage output changes the result fingerprint; zero-population/empty-stage
  configurations still produce a valid completion receipt; scheduler time remains unchanged.

  **Focused QA:**

  `cargo test -p causafera-runtime --test historical_bootstrap -- --nocapture`

  `cargo test -p causafera-runtime bootstrap --lib -- --nocapture`

  `cargo test -p causafera-world historical --lib -- --nocapture`

  **Checkpoint:** `feat(runtime): canonicalize production bootstrap receipts`.

- [x] 2. Persist and fail-closed validate the complete bootstrap record

  **Files:** `crates/causafera-runtime/src/snapshots.rs`,
  `crates/causafera-runtime/src/snapshot_sections.rs`,
  `crates/causafera-runtime/src/runtime.rs`,
  `crates/causafera-runtime/tests/historical_bootstrap.rs`,
  `crates/causafera-runtime/tests/thermal_persistence.rs` where shared section-version assertions
  belong.

  **Implementation:**

  - Replace the stage/trace-only `BootstrapReceiptSnapshot` with the bounded canonical plan,
    stage-result, and receipt snapshot structures.
  - Encode/decode the plan and record in canonical sorted order under
    `SECTION_POPULATION_BOOTSTRAP`; bump its section major from 1 to 2 and keep the section ID.
  - Wire `RuntimeState::export_snapshot` to export the actual record and
    `RuntimeState::import_snapshot` to reconstruct and validate it before returning state.
  - Verify imported receipt traces, completion effects, causes, result fingerprints, targets,
    stage count, active chunks, actor ancestry, and population aggregate conservation.
  - Add RED tests before implementation for roundtrip, same-seed save/resume equivalence, missing /
    duplicate / reordered / forged / truncated / trailing / unsupported-major inputs, stale trace
    references, wrong result fingerprints, wrong causes, inactive targets, and incomplete records.
  - Update envelope disassembly and section tests so unsupported authoritative versions reject rather
    than default or migrate silently.

  **Acceptance:** export/import preserves an equal canonical bootstrap record and equal state/history
  digests; uninterrupted and save/resume runs agree; every listed corruption case fails closed; old
  section major 1 is rejected; no empty bootstrap receipt list can be imported as a valid production
  record.

  **Focused QA:**

  `cargo test -p causafera-runtime --test historical_bootstrap -- --nocapture`

  `cargo test -p causafera-runtime --test thermal_persistence -- --nocapture`

  `cargo test -p causafera-runtime snapshot_sections --lib -- --nocapture`

  **Checkpoint:** `feat(persistence): persist canonical bootstrap records`.

- [x] 3. Expose bounded bootstrap evidence through observer and Explanation

  **Files:** `crates/causafera-runtime/src/snapshots.rs`,
  `crates/causafera-runtime/src/runtime.rs`, `crates/causafera-observer-api/src/query.rs`,
  `crates/causafera-observer-wire/src/protocol.rs`,
  `crates/causafera-observer-wire/tests/protocol.rs`,
  `packages/observer-protocol/src/index.ts`,
  `crates/causafera-explanation/src/ir.rs`,
  `crates/causafera-runtime/tests/material_surface_observer.rs` or a new focused observer test.

  **Implementation:**

  - Add a bounded bootstrap summary to the existing runtime-summary read model with a hard maximum
    of six records; do not expose `RuntimeState` or agent Ground Truth identity.
  - Preserve existing wire field meanings. Use additive fields only, or explicitly bump the observer
    protocol version and update every in-repository client if additive encoding is not safe.
  - Extend Rust and TypeScript encode/decode paths with canonical ordering, duplicate rejection,
    required-field handling, unknown-field policy, and payload-size bounds.
  - Add a typed Explanation claim backed by the receipt result and completion/dependency traces;
    return the existing unsupported/insufficient evidence state when the record is incomplete.
  - Add protocol tests for canonical bytes, roundtrip, missing/duplicate fields, invalid counts,
    unknown fields, trailing bytes, and forged trace anchors.

  **Acceptance:** a real `Runtime::new` observer session returns at most six receipt summaries;
  protocol roundtrip is byte-stable; locale or observer polling does not alter authoritative digests;
  Explanation exposes typed evidence and trace anchors without semantic process labels; invalid
  records never reach observer output.

  **Focused QA:**

  `cargo test -p causafera-observer-wire --test protocol -- --nocapture`

  `cargo test -p causafera-runtime --test material_surface_observer -- --nocapture`

  `pnpm --dir packages/observer-protocol build`

  `pnpm --dir packages/observer-protocol typecheck`

  **Checkpoint:** `feat(observer): expose bounded bootstrap evidence`.

- [x] 4. Converge production entry points and remove fixture reachability

  **Files:** `crates/causafera-runtime/src/actors/state.rs`,
  `crates/causafera-runtime/src/runtime.rs`, `crates/causafera-runtime/src/bootstrap.rs`,
  `apps/observer/src-tauri/src/session.rs`, `crates/causafera-lab/src/experiment.rs`,
  production binaries/examples found by the preflight call-site audit,
  `crates/causafera-runtime/tests/historical_bootstrap.rs`, and any test-only support module.

  **Implementation:**

  - Add failing equivalence tests for observer session construction, headless `Runtime::new`, lab
    experiment setup, reset/reconnect, replay, and save/resume using the same seed/config.
  - Route all production construction paths through `RuntimeBootstrapRecipe`; keep snapshot import
    as the only resume path and never re-bootstrap imported authoritative state.
  - Prove actor promotion ancestry points to the canonical actor-promotion receipt and that aggregate
    population plus promoted actor state conserves the configured bootstrap population.
  - Move `fixture_actors` and `fixture_sensors` under `#[cfg(test)]` or a test-only support module;
    remove them only if the preflight inventory proves no test requires them. Do not replace them
    with a different production fixture.
  - Add a static/source audit test or checked command that fails when production source calls a
    fixture constructor. The allowlist must name test-only paths explicitly and must not rely on
    grep output alone for a positive production claim.

  **Acceptance:** no production call path reaches fixture/demo constructors; all production-shaped
  entry points produce byte-identical canonical bootstrap plan/receipt data for the same seed/config;
  reset and replay do not duplicate bootstrap traces; imported snapshots resume without rebuilding
  bootstrap; population and actor ancestry checks pass.

  **Focused QA:**

  `cargo test -p causafera-runtime --test historical_bootstrap -- --nocapture`

  `cargo test -p causafera-runtime --test thermal_bootstrap -- --nocapture`

  `cargo test -p causafera-lab --lib -- --nocapture`

  `rg -n 'fixture_(actors|sensors)' crates apps packages --glob '*.rs' --glob '*.ts' --glob '*.tsx'`

  **Checkpoint:** `test(runtime): prove fixture-free bootstrap entry points`.

- [x] 5. Record bounded performance evidence and synchronize documentation

  **Files:** `crates/causafera-runtime/src/benchmark.rs` or the existing benchmark test surface,
  `crates/causafera-runtime/tests/historical_bootstrap.rs`,
  `docs/architecture/detailed-development-rebaseline.md` only if contract wording changes,
  `docs/ontology/domain-coverage-matrix.md`, `docs/development/todo-backlog.md`,
  `docs/architecture/observer.md`, `docs/rfc/RFC-PERSIST-001.md`,
  `docs/world/historical-bootstrap.md`, `CHANGELOG.md`, and `PLANS.md`.

  **Implementation:**

  - Measure the accepted bounded envelope already used by the observer: nine active chunks,
    bootstrap population 512, eight promoted actors, and the current sensor configuration.
  - Capture bootstrap wall time, snapshot bytes, receipt bytes, provenance growth, import time, and
    bounded observer query overhead with observer polling disabled as the control.
  - Store reproducible command output under the existing ignored benchmark artifact convention; do
    not commit machine-specific generated output and do not claim a general scale result.
  - Update maturity language only to state the exact bounded evidence delivered. Advance
    `TODO-RUNTIME-001`, `TODO-OBSERVER-003`, and `TODO-EXPLAIN-003` only for criteria proven by this
    plan. Do not mark `TODO-DEPTH-001`, `TODO-HIST-001`, or broad historical synthesis complete.
  - Correct only documentation facts changed by this plan. Do not opportunistically rewrite the
    stale candidate ledger or unrelated roadmap prose in the same wave.

  **Acceptance:** the benchmark reports nonzero reproducible measurements for the stated envelope;
  repeated same-seed runs produce identical authoritative record/digest outputs; documentation
  records the bounded envelope and explicit unknowns; no unverified maturity or scale claim is added.

  **Focused QA:**

  `cargo test -p causafera-runtime --test historical_bootstrap -- --nocapture`

  `cargo run -p xtask -- ci`

  `git diff --check`

  **Checkpoint:** `docs(runtime): record bootstrap receipt closure evidence`.

## Dependency matrix

| Task | Depends on | Reason |
| --- | --- | --- |
| 1 | Existing runtime/world/provenance contracts | The adapter and receipt semantics must be defined before persistence or protocol work. |
| 2 | 1 | Snapshot bytes must represent the canonical record produced by task 1. |
| 3 | 2 | Observer and Explanation must read validated persisted-shaped data, not a second model. |
| 4 | 1, 2 | Entry-point equivalence and fixture checks require the canonical recipe and import contract. |
| 5 | 1–4 | Measurements and documentation must describe the complete delivered surface. |

The paused maturity audit and broad `TODO-DEPTH-001` are not reopened. If implementation discovers a
missing prerequisite that changes the topology or requires a new authoritative domain, stop and
record it in the Decision Log rather than expanding this plan silently.

## Verification and final verification wave

The following final checks run only after tasks 1–5 are green and all checkpoint commits exist.

- [x] F1. Run focused Rust unit/integration tests for canonical plan construction, receipt validation,
  snapshot roundtrip/corruption, observer protocol, Explanation, fixture-free entry points, and
  save/resume equivalence. Record exact commands and results in `Progress`.
- [x] F2. Run `cargo fmt --all -- --check` and
  `cargo clippy --workspace --all-targets --all-features -- -D warnings`; resolve only regressions
  introduced by this plan.
- [x] F3. Run `cargo test --workspace --all-features` and
  `cargo test --workspace --no-default-features`; report any pre-existing failure separately.
- [x] F4. Run `cargo run -p xtask -- ci`, `pnpm lint`, `pnpm typecheck`, `pnpm build`,
  `node tools/audit/check-entry-points.mjs`, `node tools/audit/run-source-tests.mjs`, and
  `git diff --check`.
- [x] F5. Manual QA Gate: run the real observer-session test
  `cargo test -p causafera-observer --bin causafera-observer session_negotiates_and_streams_real_runtime_snapshots -- --exact --nocapture`
  and the new receipt-specific session test
  `cargo test -p causafera-observer --bin causafera-observer session_exposes_six_bootstrap_receipts -- --exact --nocapture`.
  The receipt-specific test must assert six strictly ordered receipts, canonical completion traces
  resolving in the session's trace store, and the existing population/actor conservation assertion.
  Then run the direct persistence scenario
  `cargo test -p causafera-runtime --test thermal_persistence save_resume_equivalence -- --exact --nocapture`
  and the new bootstrap equivalence scenario
  `cargo test -p causafera-runtime --test historical_bootstrap production_bootstrap_save_resume_preserves_record -- --exact --nocapture`.
  These tests must assert uninterrupted versus resumed plan/receipt equality and equal canonical
  state/history digests. Capture the exact command output as evidence; a passing compile alone is
  not sufficient.

## Determinism impact

- No new nondeterministic source is permitted.
- Active chunks, targets, stage effects, receipt causes, and snapshot records use sorted canonical
  order.
- Stage parameters and result fingerprints use canonical encodings and explicit domain-separated
  seeds.
- Same seed/config must produce equal plan, receipts, ancestry, state digest, history digest, and
  snapshot envelope bytes.
- Observer query cadence, locale, polling, and Explanation rendering cannot mutate or reorder
  authoritative state.

## Memory and performance impact

- Bootstrap records are bounded to six stages plus bounded per-stage trace causes and targets.
- Stage effect traces remain in the existing causal store; the receipt record must not duplicate full
  event payloads.
- Snapshot and observer payloads enforce explicit count/byte limits before allocation.
- Benchmark evidence covers bootstrap time, import time, snapshot size, provenance growth, and
  observer overhead for the stated envelope only.

## Observer impact

The observer receives a read-only bounded summary derived from `RuntimeSnapshot`. It does not receive
mutable runtime handles, authoritative actor identity for cognition, arbitrary causal graph data, or
unbounded stage effects. Existing fields retain their meaning. New fields are versioned and tested for
canonical encoding, decoding, bounds, unknown fields, and locale independence.

## Explanation impact

The new claim identifies that the current initial state has a validated bootstrap record, reports
typed stage/result values and observation window, and anchors the claim to completion/dependency
traces. It does not translate opaque process schema IDs into a narrative or infer why a stage exists.
An incomplete or unsupported record is represented as unknown/insufficient using existing IR
semantics.

## Persistence impact

`SECTION_POPULATION_BOOTSTRAP` remains the section ID and advances from major 1 to major 2. The new
major is required because the authoritative payload changes from two fields per receipt to a complete
canonical plan/record. Old and unsupported majors fail closed. The digest schema advances from 6 to 7
because the authoritative physical-state digest includes the canonical bootstrap record.

No compatibility shim may interpret an old empty receipt list as a valid current production record.

## Cross-domain effects

This plan connects existing terrain, material, population, actor, activity, and thermal bootstrap
effects to one inspectable causal record. It does not add new cross-domain dynamics. Population
conservation and actor promotion ancestry become explicit acceptance evidence, while observer and
Explanation remain downstream read models.

## Risks and mitigations

- **Duplicate plan models:** rename the runtime recipe and keep `causafera-world` canonical.
- **Decorative receipts:** require a real completion state effect and verify its trace/effect on import.
- **No-op stages:** always commit a bounded completion transition so every stage has a receipt.
- **Fixture false positive:** combine graph/call-site inspection with a checked source/build audit and
  explicit test-only paths.
- **Schema drift:** bump section/digest/protocol versions deliberately and reject unsupported data.
- **Broad-history creep:** keep the six current stages as the complete implementation surface and
  reject new domain synthesis in review.
- **Observer leakage:** expose only typed bounded receipt fields and trace anchors; retain the
  existing observer/cognition separation.
- **Performance overclaim:** record measurements and envelope; do not convert them into scale claims.

## Documentation changes

The implementation worker updates only documents whose facts change:

- `PLANS.md` registers this accepted plan while it has unfinished stages and moves it to completed
  records only after all final verification passes.
- `docs/development/todo-backlog.md` records delivered evidence for the specific runtime/observer/
  Explanation criteria.
- `docs/ontology/domain-coverage-matrix.md` records the bounded historical-bootstrap and observer
  maturity change without broad maturity claims.
- `docs/architecture/observer.md`, `docs/rfc/RFC-PERSIST-001.md`, and
  `docs/world/historical-bootstrap.md` record the accepted receipt/read-model/version contracts.
- `CHANGELOG.md` records the user-visible persistence/observer contract change.

## TODO changes

- Advance only the proven portions of `TODO-RUNTIME-001`, `TODO-OBSERVER-003`, and `TODO-EXPLAIN-003`.
- Do not close `TODO-DEPTH-001`, `TODO-HIST-001`, `TODO-PERSIST-001`, `TODO-ANALYTICS-001`, or any
  broad domain-depth TODO based on this bounded slice.
- If a new follow-up is discovered for stage-history synthesis, receipt-growth management, or
  deeper observer causal queries, add a separate `Detailed Development — ...` TODO with evidence;
  do not hide it in this plan.
- Opened `TODO-PERSIST-004` (snapshot import re-derives what it can instead of trusting it), per the
  rule above: the defect class is persistence-wide, pre-dates this plan, and would otherwise be
  absorbed a few fields at a time into a plan about bootstrap receipts.
- Opened `TODO-OBS-003` (decoder parity outside the runtime summary), per the rule above: the
  divergences are pre-existing, outside this plan's surface, and recorded with their evidence rather
  than absorbed here.

## Decision log

- **2026-07-28:** User approved the bounded next-plan direction and required sequential Claude Code
  execution without OMO.
- **2026-07-28:** Selected canonical production bootstrap receipt closure because the runtime already
  executes six causal stages, the world crate already validates canonical receipts, and snapshots
  currently export an empty bootstrap record. This is a narrower and more evidence-backed seam than
  broad historical synthesis or another generic material coupling slice.
- **2026-07-28:** Kept the six existing runtime stages as the complete implementation surface. New
  geology, climate, ecology, language, settlement, institution, economy, and authored history remain
  explicitly out of scope.
- **2026-07-28:** Chose a runtime adapter around the canonical world plan, a bounded real completion
  state effect per stage, population/bootstrap section major 2, digest schema 7, and additive observer
  fields unless the current protocol cannot represent them safely.
- **2026-07-28 (wave 1):** Moved the snapshot plumbing for the record — the expanded
  `BootstrapReceiptSnapshot` and the `SECTION_POPULATION_BOOTSTRAP` major bump — into wave 1 rather
  than wave 2. `assemble_envelope` computes its header digests by importing the snapshot it is
  assembling, so leaving export writing an empty record through wave 1 would have left every
  save/resume test RED at the wave-1 checkpoint. Wave 2 kept the fail-closed import validation and
  the corruption matrix, which is where its acceptance criteria actually live.
- **2026-07-28 (wave 1):** Removed `ConcreteHistoricalBootstrapAdapter`. It wrapped the renamed plan
  type in an enum with no callers anywhere in the workspace, and the recipe can no longer implement
  `HistoricalBootstrapAdapter`: the coordinator returns a validated record, not a flat trace list.
  Keeping it would have meant retaining a second bootstrap entry point that produces no receipts,
  which this plan exists to eliminate.
- **2026-07-28 (wave 1):** The stage coordinator reads what a stage committed from the trace store
  rather than from the adapter's returned `Vec<TraceId>`. `ActorPromotionStage` commits two events
  per promotion — the actor transition and the aggregate transition — and reported only
  `latest_physical_trace`, so its receipt would have omitted half its own ancestry. The adapter's
  reported traces are still checked to be a subset of what it committed, so a stale trace from an
  earlier stage fails closed.
- **2026-07-28 (wave 2, discovered prerequisite):** `active_chunk_shape` was never written to the
  runtime recipe section. An `Area` chart therefore resumed as a `Line` chart: the state sections
  restored all nine chunks while the restored configuration described three, and nothing compared
  the two. This is a defect that predates the plan; it surfaced because import now re-derives the
  canonical plan from the persisted configuration, and the plan's targets come from the active chunk
  set. Fixed here rather than recorded and deferred, because "identical across runtime, observer
  session, reset, experiment, replay, and save/resume entry points for the same seed and recipe" is
  this plan's stated goal and is false without it. `RUNTIME_RECIPE_SECTION_MAJOR` moves 5 to 6.
- **2026-07-28 (wave 2):** Import compares the persisted plan against the plan the persisted
  configuration reproduces, rather than validating each field separately. One comparison covers plan
  identity, world seed, stage spans, the dependency chain, parameter fingerprints, and the sorted
  active-chunk targets, and it cannot drift from what production builds because it calls the same
  constructor.
- **2026-07-28 (wave 2, gate superseded by the fifth through seventh audits):** Population
  conservation and promoted-actor ancestry are asserted only while `advanced_through` is zero. The
  *scoping rationale* below still holds; the *gate* does not — it is now the trace store's shape,
  with the clock as a mutual pin, because `advanced_through` arrives from the snapshot. From the first tick, `lifecycle_births_and_deaths`,
  `lifecycle_movement`, and `lifecycle_actor_resolution` legitimately add, remove, move, promote and
  demote, so an equality against the configured bootstrap population afterwards would be a false
  assertion rather than a protective one.
- **2026-07-28 (wave 3):** Kept `OBSERVER_PROTOCOL_V1`. The bootstrap summary is representable as
  additive fields 28+ without ambiguity, unknown fields are already skipped by both decoders, and an
  absent field 28 decodes to an explicit "absent" schema version rather than to a record claiming
  zero stages. Making the new fields required would have been a breaking change wearing an additive
  disguise.
- **2026-07-28 (wave 3):** The Explanation claims do not report result fingerprints as numeric
  values. A fingerprint is an equality identity, not a magnitude (INV-038); the completion trace
  anchors are how a reader reaches the committed effect that carries the result. Claim schemas 18/19
  were left unregistered in the Explanation renderer, which already keeps an unregistered schema's
  identity in every locale, rather than inventing UI labels for them in this plan.
- **2026-07-28 (wave 3):** `RuntimeSnapshot` lost `Copy` because the summary carries a receipt list.
  Three call sites in `causafera-lab` moved from `copied()` to `cloned()`.
- **2026-07-28 (wave 4):** Removed `fixture_actors`/`fixture_sensors` outright rather than moving
  them behind `#[cfg(test)]`. The preflight inventory found no production caller and no test that
  needed them, and the plan permits removal in exactly that case. `ActorRuntimeConfig` is left in
  place: it is a configuration type, not a fixture constructor, and removing it would be
  opportunistic cleanup.
- **2026-07-28 (post-audit):** An independent audit found two real defects, both confirmed by
  proof-of-defect runs before being fixed.

  **Import accepted a coherently forged record.** The pairwise checks — receipt against completion
  effect, receipt against materialized stage result — all pass for a snapshot that rewrites the
  three together, and the completion's causes were only required to *contain* the receipt's causes,
  so every stage-effect trace could be stripped from a completion while the record still validated.
  That defeats the stated purpose of the design: the receipt could hide exactly the detailed
  ancestry it exists to carry. Import now re-derives each stage's committed window from the trace
  store — the store is append-only and bootstrap precedes the first tick, so the completions
  partition its prefix — recomputes every result fingerprint from what the store says the stage
  committed, and requires each completion's causes to equal exactly its stage effects plus its
  predecessor. Getting past this now requires forging every stage effect's payload, which is a
  different self-consistent history rather than a false account of this one, and that is the
  boundary `SECURITY.md` already draws.

  **The observer decoders accepted incomplete and self-contradictory summaries.** Fields 28..=35
  were read individually, so a payload with only the schema field decoded to a zero-filled record,
  an unknown schema version was accepted as if it were version 1, `complete` and `stage_count` and
  the receipt count were never cross-checked, repeated scalars silently took the last value, and —
  worst — receipts arriving without field 28 were parsed and then dropped. They are now one atomic
  optional group in both Rust and TypeScript: all of it, or none of it and the explicit absent
  schema. The two decoders are kept rule-identical deliberately; two decoders of one wire contract
  that disagree on validity are worse than one. That claim is bounded to the runtime summary and its
  bootstrap group, which a later parity audit verified across 900 mutation vectors with zero
  divergences. The six other shared decoders are **not** in parity, on pre-existing root causes
  outside this plan's surface; they are recorded with the audit's evidence as `TODO-OBS-003` rather
  than fixed here.

- **2026-07-28 (second audit):** A re-audit of `cf3f018` confirmed the first two fixes and found
  three more, all real.

  **Stage parameter fingerprints were incomplete.** `chunk_extent` decides the lattice terrain and
  thermal fields are built at, and `sensor_count` decides how promoted actors are equipped, but
  neither reached the corresponding stage's parameters. Probing it showed the consequence is worse
  than a colliding plan identity: because neither value appears in the per-object committed effects
  either, stage 1, stage 4 and stage 6 produced **identical result fingerprints** across configs
  that differ in them. That falsifies this plan's own acceptance criterion that different stage
  output changes the result fingerprint, which the original test only exercised along `terrain_seed`
  and `bootstrap_population`. Both values now live on their stage structs, feed the fingerprints, and
  — for the two stages that read them directly — are used from the struct rather than re-read from
  the configuration at execution.

  **The canonical stage seed was never handed to adapters.** RFC-HIST-001 states that each stage
  receives a domain-separated seed contribution; the trait took only `&mut RuntimeState` and the
  accessor sat unused. The trait now passes it. All six adapters ignore it, because no current stage
  has stage-local deterministic variation — but the signature is the contract, and a stage that later
  needs randomness has to take it from here rather than deriving a seed of its own. Implementing an
  accepted RFC is not speculative generality; leaving it as an unused accessor was the smell.

  **Nested receipt fields were laxer than the summary's.** The top-level group was made strict in the
  previous pass while fields 1..=4 inside a receipt still allowed duplicates with last-value-wins and
  silently skipped a known field arriving on the wrong wire type. Both decoders now reject each,
  which also removes the inconsistency of one message being strict and the message nested inside it
  not being.

  Digest schema stays 7 and population/bootstrap section major stays 2 even though the plan identity
  and every stage result moved. Both versions were introduced on this branch and `main` is still at
  schema 6 and major 1, so no snapshot outside this branch has ever carried either; bumping again
  would version a contract nothing has seen. The pinned neutrality digests are re-recorded a second
  time with that reasoning stated.

- **2026-07-28 (third audit):** A third audit of `0ea17cb` found five more, all real.

  **The two decoders disagreed on which payloads are valid.** Rust bounds `configured_promotion_limit`
  to 32 bits; TypeScript widened it with `Number()`, which neither rejects nor saturates. A payload at
  2³² was rejected by one decoder and accepted by the other. That is the one failure mode a single
  wire contract cannot tolerate, so the bound is now applied on the bigint before widening, and the
  schema version and stage count are compared as bigints too rather than after a lossy conversion.

  **`sensor_count` still did not drive execution.** The previous pass added it to the stage struct and
  its fingerprint but left `promote_actor_from_aggregate` reading the configuration, so the parameter
  described the promotion without parameterizing it. It is now passed by the caller that declares it,
  and the regression test asserts the promoted actors actually carry that many apertures rather than
  only that the fingerprint moved.

  **A fully mistyped summary fell through to the absent schema.** A partly mistyped group was already
  rejected as a missing field, but a payload with every scalar on the wrong wire type reported "this
  reader predates the summary" about a peer that had tried to send one. Known fields 28..=35 on the
  wrong wire type now fail closed in both decoders, matching what the nested receipt already did.

  **The benchmark's byte figure was mislabelled.** `encoded_snapshot_bytes` summed payload sections
  only, while the two benchmarks beside it measure the complete encoded envelope; the documented
  figure was therefore not comparable with theirs and understated a file on disk. It now measures the
  whole envelope, and the numbers are re-measured. The wall-time sampling is also stated precisely:
  the checked-in test runs the benchmark twice as an identity check, and the five-sample ranges were
  taken by hand.

  **Progress had no rows for the audit commits.** Added, with their focused verification.

- **2026-07-28 (fifth audit):** A fifth audit of `ab38ea1` found five defects, one per row of the
table below: a fail-open gate, a cluster of shared-`Cursor` decoder divergences, a target check that
was never made, and two documentation contradictions this branch had introduced and left standing.
(This entry sits after the fourth audit's below only because the table was written first and
relabelled later; the rounds ran in numeric order.)

| Defect | Proof | Regression coverage |
| --- | --- | --- |
| `advanced_through` arrives from the snapshot and gates both bootstrap-time checks, but was never cross-checked against `recipe.completed_time`; a snapshot presenting as bootstrap-time while claiming to have advanced skipped both | reproduced: 100 residents deleted and every promoted ancestry erased, import returned `Ok` | `a_snapshot_whose_clocks_disagree_is_rejected`, `a_snapshot_that_moves_only_its_completed_time_is_rejected` |
| TypeScript accepted protobuf wire type 5 and an unbounded field number; Rust rejects both. Rust defaulted fields 13..=22 that TypeScript requires | comparison of both cursors | `tools/audit/test-observer-bootstrap-decoder.mjs` |
| Nothing compared plan targets against the restored active chunk set; the property held through thermal and duplicate validation rather than through this plan's own §3 promise to verify target chunks are active. `RFC-PERSIST-001` describes only the plan-versus-configuration comparison, which is what the audit showed to be insufficient | probe | `a_target_that_is_not_an_active_chunk_is_rejected` |
| Four documents still said no observer-overhead figure is reported while two reported one | inspection | wave-5 Decision Log entry superseded in place; the other three corrected |
| Progress §F1 recorded 33 and 10 where the suites stood at 43 and 20 | inspection | counts refreshed and marked as re-run per round |


- **2026-07-28 (fourth audit):** A fourth audit of `5367351` found four more.

  **Field 35 on a varint wire diverged.** TypeScript's generic `wire === 0` branch ran before the
  bootstrap wire-type guard, so the field was stored as a scalar and then silently dropped while Rust
  rejected it. The guard now runs first and is table-driven: each bootstrap field declares the one
  wire type it may arrive on, which is harder to get partially right than a range check.

  **An over-wide varint diverged.** Neither decoder checked the tenth byte's payload bits. Rust
  shifted them into a `u64` and truncated — `0x02 << 63` becomes zero — while TypeScript accumulates
  into a bigint, cannot truncate, and rejected the result downstream. This is the third divergence of
  the same shape found in three consecutive audits, and unlike the first two it lives in the shared
  `Cursor`, so it affected every field of every message rather than the bootstrap group. Both now
  reject a tenth byte above one.

  **The benchmark did not follow this repository's own methodology.**
  `docs/performance/benchmarks.md` requires warm-up, mean/median/stddev and raw samples; the bootstrap
  benchmark timed a loop and divided. It now warms up four repetitions, measures twenty per
  distribution, and retains every raw sample. The consequence was not cosmetic: the observer encoding
  overhead had been reported as unresolvable through three passes, and that turned out to be a
  property of measuring the control and its counterpart as two sequential blocks. Interleaved at
  twenty samples the control's spread collapses to tens of nanoseconds, the distributions separate,
  and the overhead is now reported as a figure.

  **`docs/performance/benchmarks.md` described `TODO-PERF-001` as open** in three places after the
  backlog had closed it and carried the remainder to `TODO-PERF-003`. Corrected in the benchmarks
  document, which was the stale source.

- **2026-07-28 (fifth audit):** The recorded reason for having no TypeScript adversarial tests was
  checkably wrong, and the gap is now closed rather than re-justified. The claim had been that the
  workspace has no JavaScript test runner; it does — `node tools/audit/run-source-tests.mjs` drives
  `node --test` over `tools/audit/*.mjs` and is in `AGENTS.md`'s validation suite. The TypeScript
  obstacle was also softer than stated: `packages/observer-protocol` already carries `typescript` as
  a devDependency, so the module can be compiled and imported in one step.
  `tools/audit/test-observer-bootstrap-decoder.mjs` now runs adversarial vectors through the real
  decoder, built from hand-written bytes rather than from the decoder's own inverse, and is
  registered in the runner. Confirmed to have teeth: reverting three of the parity guards fails
  exactly three of them. It began at seventeen `node --test` cases, of which three are positive
  controls rather than rejections, and stands at twenty-one after round seven added the field-raster
  lattice section.

- **2026-07-29 (sixth audit):** Split across four agents by axis. **A bootstrap-time guard was
  fail-open:** `advanced_through` alone decided whether the domain checks ran, and it arrives from
  the snapshot, so a record could turn them off by claiming to have advanced. The gate is now the
  store's shape — `holds_only_the_bootstrap_prefix` — with the clock as a mutual pin, so neither a
  trailing event with the clock at zero nor a moved clock over a bootstrap-only store is accepted.
  Population conservation gained a promotion-count equality, because the single sum is satisfiable by
  deleting every actor and inflating an aggregate by the same number. Aggregates must sit in active
  chunks, `actor_ancestry` must cover exactly the actor set, material surfaces must cover the active
  chunks, reservoir budget and target are recomputed from the stage-six window, and the trace store's
  identifier counters may no longer be rolled back. **One of this plan's own tests was vacuous:**
  `a_target_that_is_not_an_active_chunk_is_rejected` tripped thermal validation before reaching the
  guard it named. The root cause was `rejects()` accepting any error, which is now an exact-message
  assertion at every call site.

- **2026-07-29 (seventh audit):** Split across four agents by axis; one was lost to a session limit
  and its configuration sweep was adopted directly instead. Findings verified against the code before
  acting, which mattered three times.

  **One appended event disabled every bootstrap-time domain check.** `validate_bootstrap_domain_state`
  returned `Ok` as soon as the store held anything past the last completion, so a snapshot could
  append one junk event, claim to have advanced, and gut the population, the actor set, the material
  surfaces and a reservoir budget together. The checks that do not depend on the store ending at
  bootstrap now run unconditionally in `validate_persistent_domain_state`: surfaces cover the active
  chunks, actor objects cover the actors, the aggregate actor pool names live actors, and the living
  population equals the bootstrap population plus births minus deaths.

  **Two of the audit's own suggestions were wrong and were not implemented as given.** Exact surface
  equality would have been a false rejection — thermal transfers create further surfaces at non-zero
  cell indices during a run — so the unconditional form requires coverage, not equality. And the
  proposed reservoir clause is unreachable: import already resolves every reservoir against its
  thermal field, rejecting both a chunk with no field and an out-of-range cell, so a clause here
  would read as protection that is not there. It was deliberately not added.

  **`next_actor_id` could be rolled back**, the same defect as the closed trace-counter rollback on
  the one counter that fix did not cover. `promote_actor` issues `ActorId::new(next_actor_id)` into a
  map, so the next promotion replaces a live actor: residents disappear with no death event and the
  corrupted state re-exports clean. **The trace store accepted two shapes the commit path forbids** —
  events with no effects, and times that are not non-decreasing — and both are what made the forged
  trailing event cheap.

  **A guard had no test, and a test was named for a guard it never reached.** The aggregate-ancestry
  check survived deletion of the entire suite; it is now reached directly through the runtime root
  trace and through a stage completion. The test named for it actually covers the prefix guard and
  has been renamed accordingly.

  **The false-rejection axis is now a permanent test.** Every round tightens what import accepts, and
  a check phrased too strongly does not fail loudly — it makes a legitimate save unloadable, which no
  adversarial test can detect. `every_state_the_runtime_produces_survives_import` sweeps the
  configuration space at bootstrap and after advancement through direct import, an envelope round
  trip, and an export-resume-export chain.

- **2026-07-29 (eighth audit):** Narrowed to what round seven actually changed, after the loop began
  re-covering the same ground. Three findings, and two of them are about round seven's own work.

  **Two fields silently overrode what they are derived from.** `actor_action_bounds` is the sole
  displacement bound `validate_action` enforces and is built from `config.action_bounds`;
  `ResolutionPolicy` governs detail promotion and demotion and is a compiled constant, not
  configuration at all. A snapshot could carry `i64::MAX` beside a persisted configuration saying
  eight, and choose its own resolution thresholds. Both are now one comparison against something the
  same snapshot already carries, and `ResolutionPolicy` has a single named definition so construction
  and import cannot drift.

  **Both varint-bound tests were weaker than they read, and one had no teeth at all.** The Node
  counterpart appended the over-wide varint to a summary carrying no bootstrap group, so the
  partial-group check rejected the payload whatever the bound did — all five mutants survived. The
  Rust test asserted only that a representable tenth byte was *not* rejected, which a bound at the
  wrong shift or one dropping bit 63 also satisfies. Both now pin what they claim: the Node vector
  sits inside a complete group with a decoding control, and the Rust test asserts the decoded value.

  **An attempted fix was reverted rather than shipped.** The audit showed the round-seven population
  identity is satisfiable by moving the counters, with the trace store left byte-identical — 100
  residents erased and laundered into a clean save. Anchoring each aggregate to the last effect that
  changed it looked like the fix and is not: `fingerprint_population_aggregate` mixes count, births,
  deaths and material flow into one fingerprint, while material flow is transitioned under a
  *different* property, so no single committed effect anchors the whole aggregate and the check
  falsely rejected honest state. It was caught by `every_state_the_runtime_produces_survives_import`
  and by the envelope assembly failing on an honest snapshot — the negative control added in round
  seven doing exactly the job it was added for. A correct fix needs a count-only fingerprint on the
  aggregate effect, which is a deliberate change to the effect payload and the digest, or a replay of
  the aggregate's effect chain. Recorded as `TODO-PERSIST-004` rather than guessed at again.

  **The loop was going in circles, and the reason is structural.** Every round since the fourth has
  found more instances of one defect — import trusting a persisted value it could re-derive — and
  there are dozens of such fields across geography, thermal, mana, resolution and cognition. Grinding
  them down a few per round inside a bootstrap-receipt plan is scope creep with no endpoint, so the
  whole surface is opened as `TODO-PERSIST-004` with the reproductions, and only the instances inside
  this plan's own contract were fixed here. The documentation half of the circling had the same
  shape: benchmark figures were duplicated into the backlog and the maturity matrix, went stale
  whenever a round re-measured, and were corrected in place twice. Those copies are now replaced by
  pointers to the one place the figures live.

- **2026-07-29 (post-audit, not done):** Two audit findings are recorded rather than closed. The
  payload-wide byte limit on the runtime-summary decoder is pre-existing behaviour of the whole
  payload, bounded at the query layer by `MAX_QUERY_PAYLOAD_BYTES`; this plan bounds its own group's
  counts and leaves the payload-wide limit to whoever owns that contract. Decoder parity **outside**
  the runtime summary is recorded as `TODO-OBS-003`: six pre-existing divergences across the other
  decoders, the sharpest being Explanation IR, where Rust validates and normalizes through its
  constructors and TypeScript does neither, so identical accepted bytes produce different objects.
  Closing that is a cross-decoder change well outside this plan's surface. The earlier companion
  finding — no TypeScript adversarial coverage — was closed in the fifth round and is no longer an
  exception. The source fixture audit remains textual and does not prove reachability, which is what
  that test's own doc comment says; the reachability claim rests on the entry-point equivalence and
  ancestry tests.

- **2026-07-28 (wave 5, superseded by the fourth audit):** No observer-overhead figure was reported.
  Control and measured poll means straddled each other across five runs at the bounded envelope, so
  the bootstrap-summary encoding cost was judged to be below this harness's noise. **This conclusion
  was wrong, and wrong in an instructive way:** it was a property of measuring the control and its
  counterpart as two sequential blocks of eight, not of the system. The fourth audit's methodology
  fix — four warm-up repetitions, twenty measured, control and counterpart interleaved — collapses
  the control's spread to tens of nanoseconds and separates the distributions. The overhead is
  roughly 300-350 ns per poll, about 3-4%, and is now reported. The original caution was right in
  kind (the repository had once recorded a physically-impossible negative overhead from single-run
  measurement) and wrong in remedy: the answer to a noisy measurement is a better measurement, not a
  refusal to report.

## Progress

**Implemented.** Waves 1-5 are green and each has a checkpoint commit. Branch
`agent/production-bootstrap-receipt-closure`, from clean `main` HEAD `730e306`.

| Wave | Commit | Focused verification |
| --- | --- | --- |
| registration | `1a7424f` | — |
| 1 | `d3f6d97` | `cargo test -p causafera-runtime --test historical_bootstrap` (12 passed), `cargo test -p causafera-world` (11 passed), `cargo test --workspace` green |
| 2 | `afc03f0` | `cargo test -p causafera-runtime` (all targets green, 27 bootstrap tests) |
| 3 | `7ec9daa` | `cargo test -p causafera-observer-wire --test protocol` (10 passed), `cargo test -p causafera-observer --bin causafera-observer` (11 passed), `pnpm lint`/`typecheck`/`build` green |
| 4 | `6791154` | `cargo test -p causafera-runtime --test historical_bootstrap` (31 passed), `cargo test -p causafera-lab --lib` (6 passed, 3 ignored benchmarks), `cargo test -p causafera-observer --bin causafera-observer` (12 passed) |
| 5 | `a676593` | `cargo test -p causafera-runtime --test historical_bootstrap` (33 passed), `cargo run -p xtask -- ci`, `git diff --check` |
| audit 1 | `cf3f018` | `cargo test -p causafera-runtime --test historical_bootstrap` (37 passed), `cargo test -p causafera-observer-wire --test protocol` (17 passed), full workspace green |
| audit 2 | `0ea17cb` | `cargo test -p causafera-runtime --test historical_bootstrap` (39 passed), `cargo test -p causafera-observer-wire --test protocol` (18 passed), full workspace green |
| audit 3 | `5367351` | `cargo test -p causafera-runtime --test historical_bootstrap` (39 passed), `cargo test -p causafera-observer-wire --test protocol` (20 passed), `cargo run -p xtask -- ci`, `git diff --check` |
| audit 4 | `5d47c0e` | `cargo test -p causafera-runtime --test historical_bootstrap` (40 passed), `cargo test -p causafera-observer-wire --test protocol` (20 passed), `cargo run -p xtask -- ci`, `git diff --check` |
| audit 5 | `5793f05` | `cargo test -p causafera-runtime --test historical_bootstrap` (43 passed), `cargo test -p causafera-observer-wire --test protocol` (20 passed), `node --test tools/audit/test-observer-bootstrap-decoder.mjs` (17 passed), `cargo run -p xtask -- ci` |
| audit 6 | `90ff6fe` | `cargo test -p causafera-runtime --test historical_bootstrap` (49 passed), `cargo test -p causafera-observer-wire --test protocol` (20 passed), `cargo run -p xtask -- ci`, `git diff --check` |
| audit 7 | `93bca21` | `cargo test -p causafera-runtime --test historical_bootstrap` (57 passed), `cargo test -p causafera-observer-wire` (21 lib + 20 protocol), `node --test tools/audit/test-observer-bootstrap-decoder.mjs` (21 passed), full workspace green in both feature modes, `cargo run -p xtask -- ci` |
| audit 8 | (this round) | `cargo test -p causafera-runtime --test historical_bootstrap` (58 passed), `cargo test -p causafera-observer-wire` (21 lib + 20 protocol), `node --test tools/audit/test-observer-bootstrap-decoder.mjs` (21 passed), `node tools/audit/run-source-tests.mjs` (35 passed), full workspace green in both feature modes, `cargo run -p xtask -- ci`, `pnpm lint`/`typecheck`/`build` |

Each row names the round's **implementation** commit; the documentation commit that records a row's
hash necessarily follows it and is not itself listed. Rows are in round order; audits 4 and 5 were
previously transposed here, and rounds 6 and 7 were missing entirely until this revision.

### Final verification

- **F1.** Re-run after every audit round; the counts below are current, not the wave-5 ones. They
  have now gone stale twice — recorded as 33/10 when the suites stood at 43/20, then left at 43 when
  round six took them to 49 — so each figure names the exact command that produces it.
  `cargo test -p causafera-runtime --test historical_bootstrap` — 58 passed, 0 failed.
  `cargo test -p causafera-observer-wire` — 21 passed (lib) and 20 passed (`tests/protocol.rs`).
  `cargo test -p causafera-explanation --lib` — 17 passed.
  `cargo test -p causafera-observer --bin causafera-observer` — 12 passed.
  `cargo test -p causafera-lab --lib` — 6 passed, 3 ignored (expensive benchmarks, ignored before this plan).
  `node --test tools/audit/test-observer-bootstrap-decoder.mjs` — 21 passed.
- **F2.** `cargo fmt --all -- --check` — clean. `cargo clippy --workspace --all-targets --all-features -- -D warnings`
  — clean.
- **F3.** `cargo test --workspace --all-features` — green. `cargo test --workspace --no-default-features` — green.
  No pre-existing failure was observed at any point.
- **F4.** `cargo run -p xtask -- ci`, `pnpm lint`, `pnpm typecheck`, `pnpm build`,
  `node tools/audit/check-entry-points.mjs`, `git diff --check` — all green.

  `node tools/audit/run-source-tests.mjs` is **not reliably green**, and both of its outcomes have
  now been observed here. It was recorded as green in error, then measured at 30 passed / 1 failed,
  and at round seven it reports **35 passed / 0 failed**. The unstable case is
  `tools/audit/test-capture-cargo-dispatch.mjs`, which creates a worktree at a frozen baseline commit
  and runs `cargo test -p ontopolis-core`, a crate that does not exist in this tree; run alone it
  takes about thirteen seconds and passes. The failure is branch-independent — it reproduced with
  this branch's changes reverted — so it is environment- or cache-dependent rather than
  deterministic, and the honest statement is that this script cannot be relied on either way. It is
  recorded rather than fixed: repairing a frozen-baseline audit fixture is unrelated to this plan.
  The 35 passing include the 21 decoder vectors added by this plan: the runner drives fifteen files,
  fourteen of which report as a single leaf each. An earlier revision of this line said 31, which was
  the count taken before round seven added four raster vectors — the same stale-number failure this
  section is about, in the sentence describing it.
- **F5. Manual QA gate.** The two observer commands as written in the checklist select nothing:
  with `--exact` the filter must be the module-qualified
  `session::tests::session_negotiates_and_streams_real_runtime_snapshots`, and the bare name reports
  `0 passed; 12 filtered out`. Run with the qualified names,
  `session_negotiates_and_streams_real_runtime_snapshots` and
  `session_exposes_six_bootstrap_receipts` both pass against a real `ObserverSession`; the receipt test
  asserts six strictly ordered receipts, each chained to the previous completion trace, every
  completion trace resolving in the session's own trace store, and the existing population/actor
  conservation. `thermal_persistence save_resume_equivalence` and
  `historical_bootstrap production_bootstrap_save_resume_preserves_record` both pass; the latter
  asserts an uninterrupted run and a resumed run agree on the canonical record and on both digests.

### Bounded measurement

Envelope only: nine active chunks (`Area`, radius 1), bootstrap population 512, eight promoted
actors, one sensor aperture each. AMD Ryzen 9 7950X3D, `rustc 1.97.1`, release profile.

Method: four unmeasured warm-up repetitions, then twenty measured repetitions per distribution, with
the observer control and its counterpart interleaved. Every raw sample is retained on the report.
The figures below are medians with population standard deviations, given as ranges across three
consecutive runs of the benchmark.

| Metric | Median | Stddev |
| --- | --- | --- |
| bootstrap wall time per `Runtime::new` | 3.11-3.15 ms | 41-131 us |
| import wall time per `RuntimeState::import_snapshot` | 0.191-0.193 ms | 2.5-8.3 us |
| observer poll, control (no encoding) | 9.33-9.37 us | 38-195 ns |
| observer poll, with summary encoding | 9.62-9.65 us | 248-2512 ns |

| Metric | Measured |
| --- | --- |
| encoded snapshot bytes (complete envelope) | 177 071 |
| canonical bootstrap record bytes | 1 676 |
| bootstrap provenance events | 53 |
| observer runtime-summary payload bytes | 436 |

**Observer encoding overhead: roughly 265-290 ns per poll, about 2.8-2.9%.** Earlier passes reported
this as unresolvable, and that was a property of the measurement rather than of the system: taken as
two sequential blocks of eight, the two means straddled each other. Interleaved at twenty samples the
control's spread collapses to tens of nanoseconds and the two distributions separate. An earlier
revision of this section gave 300-350 ns and 3-4% from a different set of three runs; the honest
statement across both sets is a few hundred nanoseconds and roughly 3%, and the narrower figure
above should not be read as more precise than the run-to-run spread allows.

Import cost did **not** measurably rise across two rounds of added validation, at this envelope. The
median sat at 0.190-0.197 ms when `validate_bootstrap_stage_replay` was added, and sits at
0.191-0.193 ms now that `validate_bootstrap_domain_state` and `validate_persistent_domain_state`
also run on every import. An audit reported this figure as stale on the strength of runs at
0.211-0.219 ms; those were taken while four subagents and a concurrent build were running, and the
figure reproduces inside its documented range on a quiet machine. That is a statement about
fifty-three bootstrap events and six recomputed fingerprints, not about a larger record.

A fourth run, taken first on a cold process, is excluded from the ranges above and recorded here
instead: bootstrap median 3.70 ms with a 551 us standard deviation, and one import sample of 2.83 ms
against a 0.36 ms median. Four warm-up repetitions do not absorb a cold machine, which is a limit of
this harness and is why the ranges name three runs rather than four.

No generated benchmark output is committed. None of these numbers is a scale result or a regression
threshold, and this machine is not reference hardware.

### Deviations from the accepted plan

1. Wave 1 carried the snapshot plumbing and the section-major bump that the plan had placed in wave 2,
   so no checkpoint was RED. Wave 2 kept the fail-closed validation and the corruption matrix.
2. `RUNTIME_RECIPE_SECTION_MAJOR` moved 5 to 6, which the plan did not anticipate. See the Decision Log
   entry: `active_chunk_shape` was never persisted, and the canonical plan is derived from the active
   chunk set, so save/resume equivalence was false without it.
3. `ConcreteHistoricalBootstrapAdapter` was removed, and `RuntimeSnapshot` lost `Copy`. Both are
   recorded in the Decision Log.
4. The observer-overhead figure was reported as unresolvable through three passes and is now
   reported as roughly 300-350 ns per poll; see the superseded wave-5 Decision Log entry for why the
   earlier conclusion was a property of the measurement rather than of the system.

### Post-audit hardening

An independent audit of `a676593` found two real defects — the coherent forgery and the stripped
stage effects, which the table below splits into three rows because they are fixed by three
separate checks. All were reproduced, fixed, and covered by
regression tests. See the Decision Log for what each one was and why the fix takes the shape it does.

| Defect | Proof | Regression coverage |
| --- | --- | --- |
| Import accepted a coherently forged result (completion effect, receipt result and stage-result entry rewritten together) | reproduced: import returned `Ok` | `a_coherently_forged_stage_result_is_rejected`, `a_stage_effect_rewritten_under_an_unchanged_receipt_is_rejected` |
| Import accepted a completion stripped of its stage-effect causes | reproduced: import returned `Ok` | `a_completion_stripped_of_its_stage_effects_is_rejected`, `a_stage_completion_the_record_does_not_name_is_rejected` |
| Observer decoders accepted partial, unknown-schema, contradictory and duplicate-field summaries, and dropped receipts arriving without field 28 | inspection of both decoders | six adversarial tests plus a control proving the byte-rewriting helpers are faithful, so none of them passes for the wrong reason |

A second audit of `cf3f018` confirmed those fixes and found three more.

| Defect | Proof | Regression coverage |
| --- | --- | --- |
| `chunk_extent` and `sensor_count` reached no stage parameter fingerprint, so configs differing only in them shared a plan identity **and** stage 1/4/6 result fingerprints | probed: all three compared equal across differing configs | `configuration_the_stages_execute_under_reaches_their_parameter_fingerprints` |
| The canonical stage seed RFC-HIST-001 requires each stage to receive was never passed to adapters | the accessor had no callers | `every_stage_receives_a_stable_domain_separated_seed` covers the seed's derivation, stability and separation. **It does not cover the wiring**: no adapter consumes the seed, so nothing observable changes if the parameter is dropped again, and the trait signature is a compile-time contract only. Narrowed here rather than claimed. |
| Receipt fields 1..=4 allowed duplicates with last-value-wins and skipped a known field on the wrong wire type, while the enclosing group was strict | inspection of both decoders | `a_contradictory_nested_receipt_is_rejected`, with its own faithfulness control |

A third audit of `0ea17cb` found five more.

| Defect | Regression coverage |
| --- | --- |
| Rust bounded `configured_promotion_limit` to 32 bits, TypeScript widened it unchecked: one decoder accepted what the other rejected | `a_promotion_limit_past_thirty_two_bits_is_rejected` (Rust side; the TypeScript side is covered by `tools/audit/test-observer-bootstrap-decoder.mjs`) |
| `sensor_count` was declared on the stage and fingerprinted but not passed to the promotion, so the parameter described a promotion it did not parameterize | `configuration_the_stages_execute_under_reaches_their_parameter_fingerprints` now asserts the promoted actors carry that many apertures |
| A summary with every scalar on the wrong wire type decoded as "absent" instead of malformed | `a_mistyped_summary_field_is_rejected_rather_than_read_as_absent` |
| `encoded_snapshot_bytes` summed payload sections while the neighbouring benchmarks measure the whole envelope | figures re-measured against the complete envelope |
| Progress carried no rows for the audit commits | rows added above |

A fourth audit of `5367351` found four more.

| Defect | Regression coverage |
| --- | --- |
| TypeScript accepted wire field 35 on a varint wire and dropped it silently; Rust rejected it | `a_mistyped_summary_field_is_rejected_rather_than_read_as_absent` covers the Rust side; the guard is table-driven in both |
| Neither decoder bounded the tenth byte of a varint, so Rust truncated where TypeScript rejected — in the shared cursor, affecting every message | both reject a tenth byte above one |
| The bootstrap benchmark ignored the repository's warm-up / mean-median-stddev / raw-sample requirement | `benchmark_summary_statistics_are_computed_over_the_retained_samples`, and the envelope test now asserts every measured repetition is retained |
| `docs/performance/benchmarks.md` described `TODO-PERF-001` as open after the backlog closed it | the benchmarks document corrected — in three places at the time, and in the two further places a later audit found |

A sixth audit of `5793f05` found the bootstrap-time gate itself fail-open, and one of this plan's own
tests vacuous.

| Defect | Regression coverage |
| --- | --- |
| `advanced_through` alone gated the domain checks and arrives from the snapshot | the gate is now the store's shape with the clock as a mutual pin; `a_bootstrap_time_snapshot_with_a_trailing_event_is_rejected` |
| Conservation as a single sum is satisfiable by deleting every actor and inflating an aggregate | `deleting_promoted_actors_and_inflating_an_aggregate_is_rejected` |
| Aggregates, actor ancestry, material surfaces, reservoir budget/target and the trace-store counters were all unchecked | `a_population_aggregate_outside_the_active_chunks_is_rejected`, `actor_ancestry_that_does_not_match_the_actor_set_is_rejected`, `deleting_a_material_surface_is_rejected`, `a_forged_thermal_reservoir_budget_or_target_is_rejected`, `rolled_back_trace_identifier_counters_are_rejected` |
| `a_target_that_is_not_an_active_chunk_is_rejected` tripped thermal validation before reaching the guard it named | `rejects()` asserts an exact message at every call site |

A seventh audit of `90ff6fe` found the bootstrap-time scope itself usable as an off switch.

| Defect | Regression coverage |
| --- | --- |
| One appended event disabled every bootstrap-time domain check; reproduced by gutting the population, actor set, surfaces and a reservoir budget behind a moved clock and re-exporting clean | `an_advanced_snapshot_cannot_disable_the_persistent_domain_checks` |
| `next_actor_id` could be rolled back, so the next promotion replaces a live actor | `a_rolled_back_actor_identifier_counter_is_rejected` |
| The trace store accepted effectless events and non-decreasing-time violations | `a_trace_store_holding_events_the_commit_path_forbids_is_rejected` |
| The aggregate-ancestry guard survived deletion of the whole suite | `an_aggregate_whose_ancestry_is_not_a_stage_effect_is_rejected` |
| `actor_objects` and the aggregate actor pool were unchecked | `actor_objects_that_do_not_cover_the_actors_are_rejected`, `an_aggregate_actor_pool_naming_a_phantom_actor_is_rejected` |
| No negative control existed for over-strictness | `every_state_the_runtime_produces_survives_import` |

An eighth audit of `93bca21` found two fields that silently override what they are derived from, and
one weakness in the seventh round's own work.

| Defect | Regression coverage |
| --- | --- |
| `actor_action_bounds` could contradict `config.action_bounds`; `ResolutionPolicy` could override the compiled constant. Both launder into a clean re-export | `state_that_overrides_its_own_configuration_is_rejected` |
| The Node counterpart to the varint-bound test had no teeth: it appended the varint to a summary carrying no bootstrap group, so the partial-group check rejected it whatever the bound did. All five mutants survived | the vector now sits inside a complete group, with a decoding control; removing the bound now fails exactly one test |
| The Rust varint test asserted only that a representable tenth byte was not rejected, so a bound at the wrong shift or one dropping bit 63 survived | the decoded value is asserted; both mutants are now killed |

Three audit findings are deliberately **not** acted on and are recorded in the Decision Log:
snapshot import trusting persisted values it could re-derive, across geography, thermal, mana,
resolution and cognition, is opened as `TODO-PERSIST-004` with reproductions rather than ground down
a few fields per round inside a bootstrap-receipt plan; the
payload-wide byte limit on the runtime summary is pre-existing and bounded at the query layer, and
decoder parity **outside** the runtime summary is opened as `TODO-OBS-003` rather than closed here.
Their former companion — no automated TypeScript adversarial coverage — was closed in the fifth round
by `tools/audit/test-observer-bootstrap-decoder.mjs`, after the reason recorded for skipping it
turned out to be checkably wrong.

### Not delivered

- Deep-history synthesis of any kind. Six stages is the complete implementation surface.
- `TODO-DEPTH-001`, `TODO-HIST-001`, `TODO-PERSIST-001`, `TODO-ANALYTICS-001` are untouched.
- `TODO-OBSERVER-003`'s causal event slices, domain time series, pagination, and four-level overhead
  measurement remain open; only its bootstrap-receipts clause is delivered.
- `TODO-EXPLAIN-003`'s comparison frames, counterfactual context, and alternatives remain open.
- Explanation claim schemas 18/19 have no registered locale names; they render by identity.
- Snapshot import re-deriving what it can instead of trusting it, outside this plan's own contract.
  Opened as `TODO-PERSIST-004` with reproductions for terrain, bootstrap-time thermal energy, mana
  intensity, actor object values, reservoir schedules, active-chunk and resolution state, the
  per-domain clocks, the material-surface transition histories, and subjective cognition. The two
  instances inside this plan's contract — `actor_action_bounds` and `ResolutionPolicy` — were fixed
  here. The living-population identity remains satisfiable by moving its own counters; closing it
  needs a deliberate change to the aggregate effect payload and the digest.
- Decoder parity outside the runtime summary and its bootstrap group. A systematic audit found those
  two in parity across 156 hand-built and 6,300 mutation-fuzz vectors with zero divergences, and six
  pre-existing root causes of divergence in the other shared decoders — world chunk snapshot,
  query/connect response, stream envelope, field raster, and Explanation IR, where Rust validates and
  normalizes through its constructors and TypeScript does neither. Opened as `TODO-OBS-003` with that
  evidence; closing it is a cross-decoder change outside this plan's surface.

### Planning evidence

Planning evidence was gathered from the clean `main` HEAD `730e306`, the canonical plan index,
roadmap, backlog, maturity matrix, code knowledge graph, runtime bootstrap, world historical contract,
snapshot section codecs, observer protocol, and existing production-path tests. No product code was
changed while preparing this plan.
