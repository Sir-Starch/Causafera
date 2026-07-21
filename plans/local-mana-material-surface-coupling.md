# Local Mana–Material-Surface Coupling ExecPlan

**Status:** Draft

## Goal

Replace the current global mana-total gate with one bounded local field-to-material interaction:
only the mana cell addressed by a contacted MaterialSurfaceId may activate that surface's
hysteresis gate and change its condition. The ordinary material signal → bounded perception →
subjective scene → later action path remains the downstream consequence.

This is a physical-locality slice, not a new mana mechanic, material model, or operator feature.

## Context

The completed actor/material/mana slice and the merged experiment-recipe source both use the
production scheduler, provenance, snapshots, observer, and Explanation paths. The source recipe
can target a chart-qualified mana cell, but the current ManaEffectsSystem::execute instead uses
ManaFieldSet::total_intensity() and selects the first contacted material surface. Thus a field-cell
location is not yet a physically meaningful constraint on the material effect.

MaterialSurfaceId already uses the same chart-qualified cell address shape validated by ManaField.
ManaField holds fixed-point intensity and the last committed trace for every cell. This slice
connects those existing local states through the current Phase::Mana proposal/commit boundary.

## Relevant invariants

INV-001 through INV-004, INV-012 through INV-016, INV-019, INV-021, INV-027 through INV-031,
INV-038, INV-039, and INV-040.

## Ontology domains affected

Mana, material surface, physical access/perception, cognition/action, provenance, persistence,
observer, and Explanation. It does not add a terrain, biological, social, or language domain.

## Causal carriers affected

Existing local mana-cell intensity and last-change trace; chart-qualified material-surface cell;
contact history; per-surface active/inactive gate state; material condition; range-limited
physical signal; generic features; subjective-scene cues; later action; causal traces.

## Relevant documents

- README.md
- docs/vision/project-thesis.md
- docs/vision/core-loop.md
- docs/architecture/invariants.md
- docs/architecture/provenance.md
- docs/architecture/determinism.md
- docs/architecture/observer.md
- docs/ontology/causal-carriers.md
- docs/ontology/domain-coverage-matrix.md
- docs/world/mana-topology.md
- docs/rfc/RFC-MANA-001.md
- docs/rfc/RFC-PERSIST-001.md
- plans/actor-material-mana-loop.md
- plans/experiment-recipe-mana-source.md

## Current state

Runtime::new registers ExperimentRecipeManaSourceSystem, ManaRuntimeSystem, and
ManaEffectsSystem in the existing Mana phase at
crates/causafera-runtime/src/runtime.rs:664-681. The source and ordinary pattern evolution
commit fixed-point cell changes with causal traces. MaterialSurfaceId is a chart-qualified chunk
plus a field-valid cell ordinal in crates/causafera-runtime/src/material_surface.rs.

The present effect code at runtime.rs:2778-2844 uses the global ManaFieldSet::total_intensity,
a single RuntimeState::mana_effect_active flag, and the first material surface with contact.
It emits a traced condition change, but it does not prove that the changed material site is
co-located with the mana cell that caused it. The global gate flag is stored in
PhysicalCountersSnapshot; material state and bounded transition history are stored in
MaterialSurfaceSnapshot and section 0x000C.

The completed production-path tests in
crates/causafera-runtime/tests/material_surface_loop.rs and
crates/causafera-runtime/tests/material_surface_observer.rs already prove replay, save/resume,
bounded signal-to-scene-to-action effects, provenance order, observer limits, and typed
Explanation support for the broader loop. They do not prove cell-local field/matter coupling.

## Proposed architecture

### Authoritative local gate state

Add a bounded MaterialSurfaceManaGate record keyed by the existing MaterialSurfaceId and owned by
RuntimeState. Each record contains only:

- active: bool;
- last_transition: Option<TraceId>.

There is exactly one record for every authoritative material-surface record, created by the
existing material-surface bootstrap as inactive and kept in canonical MaterialSurfaceId order.
The map cannot contain an ID without a corresponding surface, duplicate IDs, out-of-extent cell
ordinals, or unknown transition traces. Remove the global mana_effect_active state and its
unscoped activity event; no fallback global gate remains.

MaterialSurfaceTransition gains local in-world evidence for Mana-phase condition changes: the
addressed mana-cell ordinal, its before/after fixed-point intensity, and the direct mana-cell
trace. Bootstrap and Action transitions retain an explicit absence of this evidence. This is
ordinary World Ground Truth, not experiment-recipe metadata.

### Local proposal and commit boundary

ManaEffectsSystem remains in the existing Phase::Mana; no phase is added. On each tick it
iterates contacted material surfaces in canonical ID order. For each surface it reads only the
matching ManaField cell (surface.id.chunk, surface.id.cell_index), its fixed-point intensity,
and its last-change trace.

For each site, it applies the existing threshold/hysteresis relation to that site's persisted
MaterialSurfaceManaGate.active value. It creates at most one canonical
LocalManaMaterialSurfaceProposal per site. The proposal contains before/after gate state and, only
for an inactive-to-active transition with a committed matching cell trace, optional before/after
material condition.

The proposals are sorted by EventProposalKey using the existing material-surface object ID and
are committed through CausalTraceStore::commit_batch(time, Phase::Mana, ...). Each changed gate
commits one MATERIAL_SURFACE_MANA_EVENT_KIND event with a new typed gate-property effect and,
on a rising transition, the existing condition-property effect in the same event. A rising event
has the matching local mana-cell trace and the same surface's latest contact trace as ordered
parents. A falling event has the relevant local mana trace where one exists and changes no
material condition. Gate-only events update the gate record and its transition trace but do not
enter the bounded material-condition transition window. A valid rising event also updates the
existing material-surface condition and pending physical-change set.

The existing experiment recipe becomes an ordinary input control: a source to a contacted
surface's matching cell may activate that surface; an equal source to another cell may not do so
solely by raising a chunk or world total. Recipe identity, policy, amount budget, and external
origin stay behind the existing operator redaction boundary.

### Persistence, digest, and replay

Persist gate records and local mana transition evidence with MaterialSurfaceSnapshot in a new
required major version of material-surface section 0x000C; reject unsupported versions and
malformed, duplicate, unsorted, missing, out-of-extent, or unknown-trace records before runtime
installation. Remove the obsolete global gate field from PhysicalCountersSnapshot in a new
required major version of section 0x0006. Advance the authoritative digest schema because the
local gate state and local evidence are state identity inputs.

Runtime::export_snapshot, Runtime::import_snapshot, canonical physical/history digests, and
snapshot reference validation must include every gate record and local evidence field. The same
seed/configuration must replay exactly; continuous and resumed runs must be equal across both a
pre-activation and a post-activation checkpoint. No external state or observer query may supply
or alter a local gate.

Import validation is semantic, not trace-existence-only. Each persisted gate transition trace
must be a Mana-phase MATERIAL_SURFACE_MANA_EVENT_KIND event with the new gate-property effect for
that exact MaterialSurfaceId and its declared before/after active state. Each local evidence trace
must be either an ordinary MANA_EVENT_KIND cell effect or an experiment-recipe source cell effect
for the exact chart-qualified cell, with matching fixed-point before/after values, and be a parent
of the recorded condition event. Bootstrap and Action transitions must carry no local Mana
evidence. Existing material/contact-anchor validation in runtime.rs is extended with these same
property, phase, value, address, and parentage checks.

Increase the MANA_EFFECTS_SYSTEM_ID registration revision from 1 to 2. Before installing a
snapshot, validate data.recipe.system_registrations for exact canonical equality with the
compiled runtime_system_registrations() vector, including phase, system schema, revision, and
registration order. Runtime::from_snapshot rejects an old, missing, reordered, or tampered
registration record before restoring scheduler times or authoritative RuntimeState.

### Observer and Explanation boundary

Publish MaterialSurfaceDelta schema V3 rather than reinterpret V2 fields. V3 retains V2's
mana_total and recipe-source-specific optional fields with their existing meanings, then adds
proto/wire/TypeScript fields 15 local_mana_before, 16 local_mana_after, and 17
local_mana_transition_trace_id. The existing cell ordinal remains the matching local cell address.
Rust observer API, protobuf, wire codec, observer session, and TypeScript protocol must round-trip
V3 exactly, reject malformed required V3 values, preserve response capacity, and not claim V3 data
through a V2 schema number.

Add MATERIAL_SURFACE_LOOP_LOCAL_MANA_TRANSITION_SCHEMA with ExplanationClaimSchemaId 15 rather
than changing the existing schema 14 claim. For a
Mana-mediated transition, world-facing output may expose chart/chunk, local cell ordinal,
condition before/after, local mana before/after, and typed trace anchors. It must not expose an
experiment source record ID, recipe hash, policy schema, external-origin flag, operator intent,
or labels such as reward, punishment, worship, or divine intervention.

Explanation may reconstruct that a local in-world mana-cell transition and an earlier contact
preceded the surface condition change, or report insufficient evidence when no local condition
effect occurred. It must not infer an operator's purpose or treat a nonmatching cell as a cause.

## Primitive vs emergent review

Field-cell intensity, contact, gate activity, material condition, trace ancestry, and bounded
signals are authoritative physical mechanisms. Magic, blessing, curse, ritual meaning, target
intent, operator purpose, and an agent's explanation remain subjective, Meta Truth, or
observer-only wording. No semantic label participates in the local gate key or transition.

## Non-goals

- no second carrier ingress, including terrain-to-mana activation;
- no terrain mutation, terrain generation redesign, geology, hydrology, climate, thermal/energy,
  electrical, combustion, or general physical solver;
- no new material-property model, material economy, material transformation catalogue, or
  conservation redesign;
- no new mana field dimension, retention model, biological coupling, ecology, practice,
  cognition, belief, language, social, or institutional work;
- no experiment-recipe API redesign, operator API, general intervention DSL, runtime mutation
  endpoint, UI control, or direct RuntimeState mutation;
- no scheduler phase, observer UI milestone, scale claim, maturity audit, or reopening of the
  completed actor/material/mana or experiment-recipe source plans.

## Implementation stages

1. Define the bounded MaterialSurfaceManaGate state and canonical snapshot records; replace the
   global gate state in RuntimeState, runtime snapshot validation, and digest inputs without
   changing the existing scheduler phase order.
2. Replace the global-total/first-contact logic in ManaEffectsSystem with pure, per-surface local
   gate and condition proposals keyed by MaterialSurfaceId; commit canonically in the Mana phase
   with exact local mana/contact parents.
3. Version the material-surface and physical-counter snapshot sections, codecs, semantic import
   validation, runtime-system registration validation, and save/resume support for the new
   authoritative state and evidence.
4. Publish MaterialSurfaceDelta V3 and its matching typed Explanation claim across the Rust API,
   proto, wire codec, observer session, and TypeScript protocol with allowed local in-world
   evidence; preserve experiment-policy redaction.
5. Add production-path integration, negative-control, replay, save/resume, provenance,
   observer/Explanation, codec, and bounded-envelope benchmark tests; update documentation only
   where implementation evidence establishes the new behavior.

## Verification

All integration tests begin with Runtime::new and its historical bootstrap, never direct
authoritative mutation or a fixture/demo constructor. Extend the completed material-loop suites
with these acceptance tests:

1. a recipe source at a contacted surface's matching cell commits through the Mana phase and
   changes only that surface through a local gate;
2. the same valid source amount at another cell does not change the contacted surface merely
   because chunk/world total mana crosses the threshold;
3. ordinary repeated material-surface samples at the matching cell still activate the same local
   effect path, proving the slice does not depend on experiment recipes;
4. a local value below threshold changes mana but produces neither a material condition change
   nor a supported condition-effect Explanation claim;
5. hysteresis deactivation commits a local gate transition without another condition increment;
6. no-contact, disabled mana effect threshold, and unavailable/nonmatching local cell controls
   produce no material effect;
7. same-seed runs replay to equal canonical physical/history digests and equal snapshots;
8. continuous and save/resume trajectories are equal before and after local activation;
9. every local Mana-phase material effect has parent-before-child matching mana-cell and contact
   traces, while a nonmatching mana-cell trace is absent from the causal parent set;
10. malformed/duplicate/missing/out-of-extent gate records, bad evidence references, and old
    required snapshot majors reject before authoritative installation; tampering with a valid
    trace's surface, cell, property, value, kind, phase, gate state, or parentage also rejects;
11. an old, missing, reordered, or tampered system-registration vector rejects before scheduler
    or RuntimeState installation;
12. MaterialSurfaceDelta V3 and its Explanation claim round-trip through Rust, proto, wire,
    observer session, and TypeScript with bounded capacity; V2 fields retain their existing
    semantics and serialized V3 redaction tests reject all experiment-policy/operator fields;
13. with material signals suppressed, local field and material state remain traced while the
    subjective scene and later action do not diverge.

Extend crates/causafera-runtime/tests/material_surface_benchmark.rs only for the existing bounded
release envelope: one active chunk, one contacted surface, one local mana source or ordinary
material carrier, and a bounded world-chunks query. Record tick time, snapshot bytes, provenance
growth, and query bytes; make no scale or throughput claim.

## Determinism impact

Use existing fixed-point field values, chart-qualified IDs, ordered maps, canonical proposal keys,
and CausalTraceStore::commit_batch. The slice introduces no RNG, floats, system time, locale,
hash-map iteration, pointer identity, or implicit ordering. Digest comparison establishes equality
only, not physical distance or recovery magnitude.

## Memory impact

Gate records are one bounded entry per active material surface. Transition history remains capped
by MAX_MATERIAL_SURFACE_TRANSITIONS; observer and Explanation values remain within their existing
bounded windows. No per-agent, global-grid, or unbounded trace cache is introduced.

## Observer impact

Use the existing versioned world-chunk query and material delta. Do not add an interactive view or
expose operator Meta Truth. The new values exist only because they are necessary to inspect the
local causal claim and must retain existing response-capacity limits.

## Explanation impact

Use the existing typed material-loop claim. It reports local before/after numeric evidence,
trace support, controls, and insufficiency; it does not add a semantic interpretation layer.

## Persistence impact

Material-surface state and physical counters receive required major-version changes. Import fails
closed on incompatible versions; snapshots, replay bundles, and save/resume contain all local-gate
authoritative state. No mutable external source is consulted during replay.

## Cross-domain effects

Existing physical patterns or immutable recipe sources change a local mana cell. The local field
can change the matching contacted material surface; that surface supplies an ordinary bounded
signal; existing perception, subjective-scene construction, and action react without Ground Truth
cell identity. The following candidate may add a terrain carrier into this now-local path.

## Risks

Risks are preserving a disguised global gate, selecting a nonmatching surface, retaining stale
gate state after import, losing local parentage in a batch commit, leaking recipe Meta Truth,
breaking snapshot compatibility without failing closed, or mistaking digest divergence for a
physical measure. The required controls and codec/provenance tests are mandatory mitigations.

## Documentation changes

After implementation evidence lands, update docs/world/mana-topology.md,
docs/ontology/causal-carriers.md, docs/rfc/RFC-PERSIST-001.md,
docs/observer/protocol.md, and Explanation documentation only where the implemented local
contract requires it. Keep proposed operator architecture documents Proposed.

## TODO changes

Do not modify the roadmap or backlog for this Draft. Update only evidence-backed portions of
TODO-SIM-001, TODO-OBSERVER-003, and TODO-EXPLAIN-003 after implementation; do not advance
biology, terrain, or the paused maturity audit.

## Decision log

- 2026-07-21: Selected after the experiment-recipe mana source implementation because a cell
  address is not yet respected by the current global field-to-material gate.
- 2026-07-21: Terrain formation remains the likely following physical-carrier slice, but it is
  deferred until its local mana consequences cannot select arbitrary material surfaces.
- 2026-07-21: Rejected upper-cognition work, biological coupling, thermal/energy physics,
  hydrology, broad material transformations, and intervention infrastructure as unsupported or
  out of scope for this bounded physical-world tranche.

## Progress

Draft only. No product implementation, runtime behavior change, active plan, or maturity claim
is authorized by this document.
