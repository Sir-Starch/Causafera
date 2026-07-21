# Experiment-Recipe Mana Source ExecPlan

**Status:** Draft

## Goal

Add one production-path, immutable experiment-recipe mana source that introduces a bounded
fixed-point amount at one scheduled existing mana cell through the normal Mana-phase
proposal/commit boundary. The resulting in-world change must reuse the existing mana → material
surface → physical signal → perception → subjective scene → later action loop and remain
persisted, replayable, inspectable, and causally reconstructable.

This is the first externally originated but physically processed cause. It is not a general
operator API or intervention language.

## Context

The completed actor/material/mana slice provides one production causal loop. Runtime registration
in `crates/causafera-runtime/src/runtime.rs` schedules Physics, Mana evolution, Mana effects,
Resolution, Perception, Cognition, Action, and Lifecycle. `ManaRuntimeSystem::execute` already
proposes field evolution and commits deterministic Mana-phase provenance; `ManaEffectsSystem`
then commits a later material-surface condition transition. Material signals are range-limited
before perception, and tests prove a later subjective/action difference.

Current laboratory “intervention” controls suppress configured physical patterns. They are not an
external source ingress. The architecture boundary in
`docs/architecture/operator-physical-interventions.md` is Proposed and deliberately forbids
direct RuntimeState mutation, arbitrary code, and semantic outcome selection.

## Relevant invariants

INV-001 through INV-004, INV-012 through INV-016, INV-019, INV-021, INV-027 through INV-031,
INV-038, INV-039, and INV-040.

## Ontology domains affected

Mana, material surface, runtime recipe/configuration, provenance, persistence, perception,
cognition, action, observer, and Explanation.

## Causal carriers affected

Immutable recipe data, scheduled fixed-point mana source, mana-cell state, existing
material-surface condition, range-limited physical signal, generic features, subjective scene
cues, action proposals, and causal traces.

## Relevant documents

- `README.md`
- `docs/vision/project-thesis.md`
- `docs/vision/core-loop.md`
- `docs/architecture/invariants.md`
- `docs/architecture/provenance.md`
- `docs/architecture/determinism.md`
- `docs/architecture/observer.md`
- `docs/ontology/causal-carriers.md`
- `docs/rfc/RFC-MANA-001.md`
- `docs/rfc/RFC-PERSIST-001.md`
- `docs/world/mana-topology.md`
- `docs/architecture/operator-physical-interventions.md`
- `plans/actor-material-mana-loop.md`

## Current state

`Runtime::new` registers the production scheduler systems at
`crates/causafera-runtime/src/runtime.rs:474-520`. `PhysicalPatternSystem::execute`
emits canonical samples from changed material surfaces at `runtime.rs:2035-2064`.
`ManaRuntimeSystem::execute` calls `ManaField::propose_evolution`, commits the resulting
cell transitions in `Phase::Mana`, and installs the traced replacement field at
`runtime.rs:2096-2162`. `ManaEffectsSystem::execute` turns the existing field gate into a
traced material-surface transition at `runtime.rs:2191-2258`.

`Runtime::export_snapshot` only exports at a completed scheduler tick, and
`Runtime::from_snapshot` restores the recipe, system times, and validated state
(`runtime.rs:582-606`). The completed loop is covered by production-runtime replay,
save/resume, no-mana, no-repetition, causal-order, bounded observer/Explanation, and
signal-to-later-action tests in
`crates/causafera-runtime/tests/material_surface_loop.rs` and
`crates/causafera-runtime/tests/material_surface_observer.rs`.

No runtime source request, source receipt, operator-level provenance partition, or source-specific
observer/Explanation evidence exists.

## Proposed architecture

### Immutable recipe representation

Add `RuntimeConfig::experiment_recipe_mana_sources` and carry its canonical immutable
`ExperimentRecipeManaSource` collection in the runtime recipe. Add
`RuntimeState::executed_experiment_recipe_mana_sources` as a bounded receipt collection. The
recipe is immutable scenario input consumed by `Runtime::new`, historical bootstrap, experiment,
save/resume, and observer session paths; it is not a fixture constructor, network command, or
mutable Runtime control.

Each v1 record contains:

- an opaque stable numeric source record ID;
- enabled/disabled state;
- scheduled execution tick;
- one existing chart-qualified mana-cell address: `ChartChunkCoord` plus local cell index;
- one non-negative fixed-point `i64` amount;
- an explicit per-record maximum and a recipe-wide source budget;
- an opaque experiment-policy schema identity and canonical recipe identity/hash.

The collection has a hard cardinality cap and is sorted by scheduled tick then source record ID.
Duplicate IDs, duplicate canonical keys, negative amounts, over-budget amounts, invalid ticks,
inactive/out-of-extent cells, and malformed policy/schema values reject during configuration /
recipe validation before a scheduler commit. V1 exercises one record but must preserve canonical
ordering for multiple valid records.

V1 accounting policy: a valid source creates mana from outside the simulated universe under an
immutable experiment policy. This is an experiment-specific accounting rule, not the final global
conservation model. An operator reservoir, redistribution of existing mana, and other
simulation-specific accounting policies remain deferred and are not designed here.

### Proposal, commit, and causal order

Register an `ExperimentRecipeManaSourceSystem` as the first existing `Phase::Mana` system. Do not
add a scheduler phase. At each tick it reads only validated immutable recipe records and its
authoritative executed-record state, emits canonical `ExperimentRecipeManaSourceProposal` values,
and commits a bounded mana-cell before/after transition through
`CausalTraceStore::commit_batch`.

The source commit uses new opaque constants
`EXPERIMENT_RECIPE_MANA_SOURCE_EVENT_KIND`,
`EXPERIMENT_RECIPE_MANA_SOURCE_OBJECT_KIND`, and
`EXPERIMENT_RECIPE_MANA_SOURCE_PROPERTY`, plus source-record object/property fingerprints,
canonical proposal key, execution tick, target-cell identity, amount, policy schema, recipe hash,
and budget information in engine/operator-level provenance. It is a root external cause and
therefore has no prior World Ground Truth trace parent. The resulting source trace must become a
parent of every derived ordinary mana evolution, gate, material-surface, and later physical-signal
transition that causally uses the changed cell.

The system records an executed receipt containing source record ID, scheduled/executed tick,
committed source trace, before/after fixed-point intensity, recipe hash, and policy schema.
Receipts prevent re-execution after save/resume. Neither source record IDs, policy schemas,
recipe hashes, nor the external-origin interpretation enter agent cognition.

After source commit, existing `ManaRuntimeSystem`, `ManaEffectsSystem`, `ActorPerceptionSystem`,
`ActorCognitionSystem`, and `ActorActionSystem` remain the only owners of their current
transitions. The source never directly assigns material condition, physical signal, percept,
scene, belief, or action outcome.

### Persistence, replay, observer, and Explanation

Persist canonical immutable recipe records/identity in the runtime recipe/configuration snapshot
representation and persist source execution receipts in a dedicated bounded authoritative snapshot
representation or a versioned extension of the existing runtime-owned section. Import validates
canonical order, budget, source-cell bounds, recipe hash, and receipt/source correspondence before
authoritative installation. Unsupported required versions fail closed.

Same seed and recipe must replay to equal canonical physical/history digests. Continuous execution
and save/resume runs must be equal both across a pre-source checkpoint and after a committed
source. A disabled or zero amount source produces no source commit and remains equal to its
control recipe.

World-level observer output may expose bounded mana/material before/after values, execution tick,
and causal trace anchors needed to reconstruct an in-world chain. It must not expose external
origin, experiment policy, recipe identity/hash, source record ID, operator intent, or labels such
as divine intervention, reward, punishment, worship target, or operator intention.

Explanation may reconstruct the typed in-world mana/material transition and its supporting traces,
including insufficiency or absent downstream evidence. It must not render engine/operator policy
or semantic intent. Operator-facing provenance remains outside ordinary world observer and
Explanation interfaces.

## Primitive vs emergent review

Recipe identity, enabled flag, scheduled tick, chart-qualified cell address, fixed-point amount,
budget, executed receipt, source trace, and in-world field/material values are authoritative
mechanisms. Experiment purpose, operator intent, divine intervention, reward, punishment,
worship target, magic meaning, and agent interpretations remain Engine Meta Truth, subjective
models, or observer-only wording; none is an authoritative mana semantic.

## Non-goals

- no general intervention DSL, parser, compiler, formula language, arbitrary formulas, units
  language, arbitrary regions, arbitrary temporal functions, or unrestricted scripting;
- no runtime mutation endpoint, network or UI control, general operator API, direct RuntimeState
  mutation, or new scheduler phase;
- no physical solver, electrical model, general conservation redesign, operator reservoir, or
  redistribution policy;
- no mana retention redesign, biological mana coupling, ecology, geology, economy, society,
  language, institutions, consciousness archive, death, resurrection, identity lineage, or
  cross-simulation transfer;
- no broad observer UI work, scale claim, maturity audit machinery, or reopening of the completed
  actor/material/mana plan.

## Implementation stages

1. Define bounded immutable recipe/source/receipt state, canonical validation, external
   experiment-policy schema, exact v1 budget rules, and recipe hash inputs in production runtime
   configuration without adding a mutable control surface.
2. Add the first `Phase::Mana` source system, canonical source proposals, root external-source
   commit event, executed receipts, and strict parent-before-child integration with the existing
   mana evolution/gate path.
3. Extend snapshots/import validation and digest inputs for recipe source records and receipts;
   preserve exact replay and pre-source/post-source save-resume equivalence.
4. Add bounded world-facing observer and typed Explanation inputs for source-caused in-world
   transitions, with explicit redaction tests for all operator Meta Truth.
5. Add production-path integration/negative-control tests and one bounded benchmark envelope;
   update direct documentation only after the evidence lands.

## Verification

Production-path tests start from `Runtime::new` / historical bootstrap, never a fixture or direct
state mutation. Required acceptance tests:

1. an enabled valid source commits at its scheduled tick, changes the target mana cell through
   the Mana phase, and can change the existing later material/signal/scene/action path;
2. zero amount produces no source provenance commit and matches its control;
3. disabled recipe produces no source provenance commit and matches its control;
4. a source below the existing meaningful material-effect threshold may change mana but creates no
   material consequence or overconfident Explanation claim;
5. a source scheduled for a different tick has no effect before its tick and commits exactly once
   at its own tick;
6. malformed, duplicate, invalid-cell, invalid-tick, and over-budget recipes reject before
   authoritative commit;
7. equivalent valid source-record input order yields equal state/history digests and receipt order;
8. enabled-source same-seed replay is exactly equal;
9. continuous and save/resume execution are exactly equal at pre-source and post-source
   checkpoints;
10. external source trace is parent-before-child of all observed derived mana/material transitions;
11. observer responses remain capacity-bounded and carry only permitted in-world evidence;
12. agent-facing perception/cognition/action state and world Explanation contain no recipe hash,
   source ID, policy schema, operator intent, or prohibited semantic labels;
13. with the existing physical-signal boundary suppressed, source → mana → material remains
   traced while subjective scene/later action does not diverge.

Reuse and extend the completed loop controls in
`crates/causafera-runtime/tests/material_surface_loop.rs` and
`crates/causafera-runtime/tests/material_surface_observer.rs`.

## Benchmark plan

Measure only the bounded v1 envelope: one active source record and the configured hard-cap
collection. Report tick time, snapshot bytes, provenance growth, and bounded observer-query size
with source disabled versus enabled. This validates bounded response size and adds no throughput,
scale, or physical-performance claim.

## Determinism impact

All source state is fixed-point or typed numeric identity. Recipe records, proposals, receipts,
and source commits use canonical ordering. No source uses system time, locale, pointer identity,
hash iteration, unseeded randomness, or scheduling-order dependence. Digest comparison establishes
equality only, never physical distance or recovery magnitude.

## Memory impact

Recipe records and receipts have explicit caps. One receipt is retained per committed source
record, with validated correspondence to immutable recipe data. Observer and Explanation output
remain bounded and do not serialize operator intent.

## Observer impact

Extend only the existing bounded world-chunk/material-loop inspection path as needed to expose
typed in-world consequences and trace anchors. Do not add an interactive operator panel or reveal
Meta Truth. Query capacity and redaction are acceptance conditions.

## Explanation impact

Extend typed material-loop evidence only when required to show source-caused in-world transitions,
observation windows, controls, and insufficiency. Explanation may cite source-derived in-world
trace ancestry but cannot assert why an operator selected the source.

## Persistence impact

The recipe configuration and executed receipts are authoritative digest inputs. Snapshot import
must reject malformed source state before it reaches RuntimeState and preserve exact equality on
continuous versus resumed trajectories. No external mutable file may supply a source during replay.

## Cross-domain effects

The slice adds one external recipe cause to existing mana and material state. Existing physical
access, cognition, action, provenance, persistence, observer, and Explanation boundaries process
the consequences. It does not add a new semantic cross-domain dispatcher.

## Risks

Risks include disguising a direct outcome assignment as a source, leaking operator intent through
observer/Explanation data, creating an unbounded or mutable control API, re-executing a source
after resume, missing source ancestry in derived effects, and treating digest-byte differences as
physical metrics. The controls above are mandatory mitigations.

## Documentation changes

Update `docs/world/mana-topology.md`, `docs/rfc/RFC-PERSIST-001.md`,
`docs/ontology/causal-carriers.md`, `docs/observer/protocol.md`, and Explanation documentation
only if corresponding implementation evidence lands. Keep
`docs/architecture/operator-physical-interventions.md` Proposed; do not describe a general
operator system as implemented.

## TODO changes

Do not alter roadmap or backlog for this Draft. Update TODO entries only after accepted
implementation evidence lands; do not create maturity-audit artifacts.

## Decision log

- 2026-07-21: Selected after the completed actor/material/mana slice because the runtime now has
  a validated in-world downstream causal path but no bounded externally originated cause.
- 2026-07-21: V1 uses immutable production recipe input and explicit experiment-policy external
  creation, rather than a mutable operator endpoint, conservation redesign, or general DSL.
- 2026-07-21: Full biological mana coupling remains deferred because current runtime has no
  integrated detailed biological carrier/physiology state required by RFC-BIO-003.

## Progress

Draft only. No implementation is authorized by this plan.
