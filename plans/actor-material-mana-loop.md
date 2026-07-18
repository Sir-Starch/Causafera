# Actor Material Mana Loop ExecPlan

**Status:** Draft

## Goal

Replace the runtime's counter-based mana feedback with one production actor-mediated, durable local
material-pattern loop whose later physical signal can be perceived, subjectively situated, acted
upon, saved, resumed, and causally reconstructed.

## Context

`PhysicalPatternSystem::execute` currently increments `physical_counter` and `ManaEffectsSystem::execute`
commits a numeric boost to the same stand-in property. The runtime already schedules physics, mana,
perception, cognition, action, and lifecycle; persists runtime state; and records provenance. The
first detailed slice should connect those real seams to a physical property rather than add another
contract or observer-only surface.

## Relevant invariants

INV-001 through INV-004, INV-012 through INV-016, INV-019, INV-021, INV-027 through INV-031,
INV-038, INV-039, and INV-040.

## Ontology domains affected

Matter/physical pattern, mana, geography-local physical space, perception, cognition, action,
runtime, persistence, provenance, observer, and Explanation.

## Causal carriers affected

Actor contact/motion, local material surface state, repeated spatial-temporal material pattern,
fixed-point mana field intensity, mana-to-material change, bounded physical signals, generic
features, subjective scene cues, action proposals, and causal traces.

## Relevant documents

`docs/vision/project-thesis.md`, `docs/vision/core-loop.md`,
`docs/architecture/detailed-development-rebaseline.md`, `docs/architecture/provenance.md`,
`docs/architecture/determinism.md`, `docs/architecture/observer.md`,
`docs/ontology/causal-carriers.md`, `docs/simulation/perceptual-features.md`,
`docs/world/mana-topology.md`, `docs/rfc/RFC-MANA-001.md`, `docs/rfc/RFC-PERCEPT-001.md`, and
`docs/rfc/RFC-TRACE-001.md`.

## Current state

`Runtime::new` schedules the required production phases. `ActorPerceptionSystem` and
`ActorCognitionSystem` construct a bounded subjective scene before `ActorActionSystem` commits
movement. `ManaRuntimeSystem` commits field-cell traces, but `ManaEffectsSystem` currently writes
only `physical_mana_effect_boost`; `PhysicalPatternSystem` turns it into counter and sample
changes. Runtime snapshots already include authoritative runtime state and resume through
`Runtime::from_snapshot`.

## Proposed architecture

Introduce one bounded, chart-qualified local material-surface state owned by the runtime's
production bootstrap and lifecycle/action paths. An actor's physically valid local contact changes
that state by canonical proposal/commit. Its repeated state transitions emit non-semantic pattern
samples to mana. Above a fixed threshold, mana changes the same material state through a separate,
traced physical proposal rather than a counter. The changed state produces range-limited physical
signals; the existing feature-to-scene path selects a subjective target and the existing action
path can later act in response. The read model exposes only a bounded before/after material delta,
numeric field context, and trace anchors.

## Primitive vs emergent review

Surface condition, contact, position, magnitude, field intensity, and causal ancestry are
authoritative. Ritual, tool use, usefulness, magic, practitioner, spell, and the observer's human
description of the pattern remain non-authoritative or emergent.

## Non-goals

No biological retention/coupling, spell or ritual taxonomy, generic material economy, terrain
erosion, multi-region world generation, language/social transmission, broad causal explorer, UI
milestone, or M5-scale claim.

## Implementation stages

1. Replace the counter stand-in with a bounded persistent local material-surface record and a
   causal bootstrap receipt; remove the counter from the accepted loop.
2. Commit actor contact/action changes and repeated material-pattern samples in canonical phase
   order, preserving existing subjective-scene boundaries.
3. Commit a thresholded mana-to-material response with typed before/after values and no semantic
   input; prove no-field and no-repetition controls leave material unchanged.
4. Emit physically accessible signals from the changed material state, route them through the
   existing perception/scene/action path, and persist all new authoritative state.
5. Add one bounded observer/Explanation projection and an end-to-end replay, save/resume, and
   causal-reconstruction scenario.

## Verification

Run the production runtime from the causal bootstrap, not a fixture: actor contact changes a local
material value; repeated transitions change mana; mana changes that value; a later bounded signal
reaches a subjective scene and a later action; every transition has parent-before-child traces.
Prove same-seed replay, save/resume equivalence, batch-order invariance, no-repetition, no-mana,
and observer-cadence/locale controls.

## Benchmark plan

Measure a bounded active chunk with one promoted actor and one material site: tick time, peak and
steady memory, provenance growth, snapshot size, and observer-off versus bounded-query overhead.
Report the envelope without a scale claim.

## Determinism impact

Use fixed-point state, chart-qualified addresses, stable actor/site ordering, explicit scheduler
phases, canonical proposal keys, and named RNG streams only if variation is introduced. Replay and
save/resume must compare canonical digests for equality only.

## Memory impact

Keep material records bounded to active local sites; aggregate or omit inactive detail only through
an explicit later resolution contract. Bound stored samples, trace references, observer pages, and
snapshot sections.

## Observer impact

Add a versioned bounded read-only material delta/query sufficient to inspect one site, its field
context, and trace anchors. Do not add a panel unless this read model cannot be inspected through
the existing observer workflow.

## Explanation impact

Produce typed material before/after values, field intensity/threshold context, observation window,
controls, insufficiency behavior, and provenance references. Do not infer purpose, ritual, or
semantic meaning from the loop.

## Persistence impact

Snapshot the material-surface state, bounded history needed by mana, causal ancestry, and any
actor-visible signal source. An uninterrupted run and save/load/resume run must have equal
canonical state and history digests.

## Cross-domain effects

Physical action and matter create the pattern carrier; mana responds and changes matter; perception
and cognition receive only accessible signals; subjective scene construction informs later action;
provenance, persistence, observer, and Explanation make the loop inspectable.

## Risks

Reintroducing a disguised counter, treating subjective IDs as Ground Truth, exposing exact object
identity to cognition, hidden semantic categories, unbounded material detail, or explaining a
digest difference as a physical metric.

## Documentation changes

Update the material/mana runtime documentation, relevant causal-carrier and observer/Explanation
contracts, the backlog, and this plan's progress only when implementation evidence lands.

## TODO changes

Advance the accepted slice of `TODO-SIM-001`; update `TODO-RUNTIME-001`, `TODO-OBSERVER-003`, and
`TODO-EXPLAIN-003` only for evidence actually delivered. Do not mark `TODO-DEPTH-001` complete.

## Decision log

- 2026-07-18: Select this slice before biological mana coupling because it replaces an existing
  counter-based feedback stand-in with the smallest production-path causal loop that visibly
  exercises Causafera's thesis.
- 2026-07-18: Reuse the current scheduler, causal bootstrap, snapshot, perception, subjective
  scene, action, and observer foundations; defer new biology, broad material economy, and UI work.

## Progress

Draft only. No product implementation has begun.
