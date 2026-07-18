# Actor Material Mana Loop ExecPlan

**Status:** Active

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

Introduce `RuntimeState::material_surfaces: BTreeMap<MaterialSurfaceId, MaterialSurface>` as the
sole authoritative store. `MaterialSurfaceId` is exactly `(ChartChunkCoord, u16 cell_index)` and
`MaterialSurface` contains `condition: i64`, `contact_count: u64`, and `last_transition: TraceId`.
The bootstrap owner is `MaterialSurfaceBootstrapStage::bootstrap`, added to
`HistoricalBootstrapPlan`; it creates one site for each causally bootstrapped active chunk and
commits `MATERIAL_SURFACE_BOOTSTRAP_EVENT_KIND` before any actor can access it.

`ActorActionSystem::execute` remains the only Action-phase owner of actor contact. After a valid
`ActionProposal`, it resolves an authoritative contact site from the actor's physical position and
the selected surface's chart/cell address, constructs `MaterialSurfaceContactProposal`, and calls
`commit_material_surface_contact_events`. That helper commits actor-body and surface-condition
effects together through `CausalTraceStore::commit_batch(Phase::Action)` under
`MATERIAL_SURFACE_CONTACT_EVENT_KIND`; its causes include the bootstrap/site transition and the
valid action trace. Cognition never receives `MaterialSurfaceId`, chart identity, cell index, or
trace identity.

`PhysicalPatternSystem::execute` becomes the Physics-phase pattern owner: it reads only changed
`material_surfaces`, calls `MaterialSurfaceCarrierAdapter::emit_samples`, and appends its bounded,
canonical `PhysicalPatternSample` values to `pending_samples` and `pattern_history`. The adapter
fingerprint and magnitude derive only from chart-local cell geometry and `condition` history.
`ManaRuntimeSystem::execute` remains the Mana-phase field proposal/commit owner.
`ManaEffectsSystem::execute` replaces `physical_mana_effect_boost` with
`ManaMaterialSurfaceEffectProposal` and `commit_mana_material_surface_effect_events`; each effect
changes a concrete `MaterialSurface.condition` through
`CausalTraceStore::commit_batch(Phase::Mana)` under `MATERIAL_SURFACE_MANA_EVENT_KIND` and cites
the committed mana-cell trace.

`material_surface_physical_signals` extends the existing physical-signal boundary with
range-limited signals sourced from changed surfaces. Existing generic feature extraction and
`ActorCognitionSystem::execute` map these into agent-local cues and a subjective scene; the next
`ActorActionSystem::execute` may act only from that scene. The bounded observer projection is
`MaterialSurfaceDelta`, added to the existing `ObserverQuery::world_chunks` /
`WorldChunkSnapshot` request-response path, and contains chart/chunk, cell ordinal, typed
before/after `condition`, mana context, and trace anchors. `MaterialSurfaceLoopClaim` is the
matching deterministic Explanation input; it reports values, window, controls, and insufficiency,
never purpose or ritual meaning.

## Primitive vs emergent review

Surface condition, contact, position, magnitude, field intensity, and causal ancestry are
authoritative. Ritual, tool use, usefulness, magic, practitioner, spell, and the observer's human
description of the pattern remain non-authoritative or emergent.

## Non-goals

No biological retention/coupling, spell or ritual taxonomy, generic material economy, terrain
erosion, multi-region world generation, language/social transmission, broad causal explorer, UI
milestone, or M5-scale claim.

## Implementation stages

1. Add `MaterialSurfaceId`, `MaterialSurface`, `RuntimeState::material_surfaces`, and
   `MaterialSurfaceBootstrapStage::bootstrap`; remove `physical_counter` and
   `physical_mana_effect_boost` from the accepted loop.
2. Implement `MaterialSurfaceContactProposal` and
   `commit_material_surface_contact_events` inside `ActorActionSystem::execute` with one
   Action-phase atomic batch for actor/body and material effects.
3. Implement `MaterialSurfaceCarrierAdapter::emit_samples` in
   `PhysicalPatternSystem::execute`, then replace the counter feedback in
   `ManaEffectsSystem::execute` with `ManaMaterialSurfaceEffectProposal` and
   `commit_mana_material_surface_effect_events` in Phase::Mana.
4. Implement `material_surface_physical_signals`, preserving the existing perception → subjective
   scene → action separation and inaccessible Ground Truth correspondence.
5. Add `MATERIAL_SURFACE_SECTION_ID`, `encode_material_surface_section`,
   `decode_material_surface_section`, and `RuntimeSnapshotData::material_surfaces`; add bounded
   `MaterialSurfaceDelta` / `MaterialSurfaceLoopClaim` projection support.
6. Add the required integration and control tests, benchmark the bounded envelope, and update
   subsystem documentation only with observed implementation evidence.

## Verification

The required production-path tests are
`actor_contact_material_surface_commits_causal_transition`,
`repeated_material_surface_transitions_drive_mana`,
`mana_material_surface_effect_changes_later_accessible_signal`,
`actor_material_surface_loop_replays_and_resumes_exactly`, and
`material_surface_loop_controls_reject_counter_fixture_and_observer_paths`.
They must run from `HistoricalBootstrapPlan`, not a fixture/demo constructor, and prove actor
contact changes a local material value; repeated transitions change mana; mana changes that value;
a later bounded signal reaches a subjective scene and a later action; and every transition has
parent-before-child traces. The final test contains no-repetition, no-mana, no-field-effect,
fixture-exclusion, observer-cadence, locale, and batch-order controls.

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

Extend the existing versioned `ObserverQuery::world_chunks` / `WorldChunkSnapshot` response with
bounded `MaterialSurfaceDelta` values. Each value exposes only chart/chunk, cell ordinal, typed
before/after condition, field context, and trace anchors. Do not add a panel unless the existing
observer workflow cannot inspect this read model.

## Explanation impact

Produce typed material before/after values, field intensity/threshold context, observation window,
controls, insufficiency behavior, and provenance references. Do not infer purpose, ritual, or
semantic meaning from the loop.

## Persistence impact

`MATERIAL_SURFACE_SECTION_ID` owns `RuntimeSnapshotData::material_surfaces`; its codecs are
`encode_material_surface_section` and `decode_material_surface_section`. It stores the bounded
surface records, last transition traces, and required bounded history. An uninterrupted run and
save/load/resume run must have equal canonical state and history digests.

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
- 2026-07-18: Decision-completeness review fixed the authoritative store, bootstrap and phase
  owners, proposal/commit operations, provenance event identities, snapshot section, observer and
  Explanation projections, and named integration/control tests. The plan is ready to implement.

## Progress

READY FOR IMPLEMENTATION. No product implementation has begun.
