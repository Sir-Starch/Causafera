# Local Mana–Material-Surface Coupling ExecPlan

**Status:** Accepted

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
- docs/explanation/explanation-ir.md
- docs/rfc/RFC-MANA-001.md
- docs/rfc/RFC-PERSIST-001.md
- plans/actor-material-mana-loop.md
- plans/experiment-recipe-mana-source.md

## Current state

Runtime::new registers ExperimentRecipeManaSourceSystem, ManaRuntimeSystem, and
ManaEffectsSystem in the existing Mana phase at
crates/causafera-runtime/src/runtime.rs:664-681 (conceptually; exact line numbers may shift
with implementation). The source and ordinary pattern evolution commit fixed-point cell changes
with causal traces. MaterialSurfaceId is a chart-qualified chunk plus a field-valid cell ordinal in
crates/causafera-runtime/src/material_surface.rs.

The present effect code uses the global ManaFieldSet::total_intensity, a single
RuntimeState::mana_effect_active flag, and the first material surface with contact. It emits a
traced condition change, but it does not prove that the changed material site is co-located with
the mana cell that caused it. The global gate flag is stored in PhysicalCountersSnapshot; material
state and bounded transition history are stored in MaterialSurfaceSnapshot and section 0x000C.

The completed production-path tests in
crates/causafera-runtime/tests/material_surface_loop.rs and
crates/causafera-runtime/tests/material_surface_observer.rs already prove replay, save/resume,
bounded signal-to-scene-to-action effects, provenance order, observer limits, and typed
Explanation support for the broader loop. They do not prove cell-local field/matter coupling.

## Accepted architecture

### Authoritative local gate state

Embed a typed gate inside each `MaterialSurface`:

```rust
struct MaterialSurfaceManaGate {
    active: bool,
    last_transition: Option<TraceId>,
}
```

Add to `MaterialSurface`:

```rust
struct MaterialSurface {
    condition: i64,
    contact_count: u64,
    last_transition: TraceId,            // mandatory: bootstrap always creates it
    last_contact_trace: Option<TraceId>, // None until first committed contact
    gate: MaterialSurfaceManaGate,
}
```

There is exactly one gate per authoritative surface record, created by the existing material-surface
bootstrap as inactive with `gate.last_transition: None` and kept in canonical MaterialSurfaceId order.
The map cannot contain an ID without a corresponding surface, duplicate IDs, out-of-extent cell
ordinals, or unknown transition traces. Remove the global `mana_effect_active` state and its
unscoped activity event; no fallback global gate remains.

A never-contacted surface and an inactive gate have no valid contact or gate transition trace, so
both `last_contact_trace` and `gate.last_transition` are `Option<TraceId>`. These options are
semantically meaningful absence, not trace-id zero.

### Historical contact eligibility (bounded policy of this slice)

For this slice, a surface is eligible to react to its matching local mana cell if it has ever been
contacted (`contact_count > 0`). A committed Action-phase `MATERIAL_SURFACE_CONTACT_EVENT_KIND`
arms the surface for the next Mana-phase evaluation. This is an explicit bounded policy for this
slice, not a universal future material-physics rule. A later design that needs contemporaneous or
ongoing contact must add a persisted, phase-controlled contact relation with begin/end events.

### Boundary semantics

For each historically contacted surface, read only the matching mana cell (`surface.id.chunk`,
`surface.id.cell_index`) and its fixed-point intensity `local_mana`.

Let `threshold = ManaParameters.effect_threshold` and `hysteresis = ManaParameters.effect_hysteresis`.
If `threshold <= 0` the effect is disabled entirely.

- **inactive gate + local_mana <= threshold** → no proposal, no event, no transition.
- **inactive gate + local_mana > threshold** → rising transition: a single
  `MATERIAL_SURFACE_MANA_EVENT_KIND` event with a gate-property effect (`active` false → true) and a
  condition-property effect (`condition` +1) in the same event.
- **active gate + local_mana >= threshold - hysteresis** → no proposal, gate remains active.
- **active gate + local_mana < threshold - hysteresis** → gate-only falling transition: a
  single `MATERIAL_SURFACE_MANA_EVENT_KIND` event with only a gate-property effect
  (`active` true → false); the material condition does not change.

These four cases are mutually exclusive and cover every state/value pair. Gate-only transitions are
inspectable through a separate bounded gate-only projection in the observer read model.

### Gate transition records

Gate transitions are persisted separately from condition transitions so that gate-only falling
events survive save/resume without entering the bounded material-condition transition window:

```rust
struct MaterialSurfaceGateTransition {
    id: MaterialSurfaceId,
    occurred_at: SimulationTime,
    before_active: bool,
    after_active: bool,
    local_mana_before: i64,
    local_mana_after: i64,
    local_mana_trace: TraceId,
    contact_trace: Option<TraceId>, // Some for rising, None for falling
    transition_trace: TraceId,
}
```

`MaterialSurfaceSnapshot` gains a bounded `gate_transitions: Vec<MaterialSurfaceGateTransition>`
field with the same cap as condition transitions (`MAX_MATERIAL_SURFACE_TRANSITIONS`). Eviction
prefers older gate-only records so the newest rising/falling causal observation remains available
to observer and Explanation paths.

### Exact parent sets and snapshot consistency

A rising gate event's semantic cause set is:

1. the matching local mana-cell `last_change` trace;
2. the surface's `last_contact_trace` (the latest committed contact, which must exist because the
   surface is historically contacted);
3. the surface's prior condition `last_transition` when it differs from the contact trace;
4. the prior `gate.last_transition` when present.

A falling gate-only event's semantic cause set is:

1. the matching local mana-cell `last_change` trace;
2. the prior `gate.last_transition`.

After collecting the semantic causes, sort them ascending by `TraceId` and remove duplicates before
passing them to `CausalEventProposal`, satisfying the store's strict-order requirement.

Snapshot import enforces the following consistency rules:

- `contact_count == 0` requires `last_contact_trace == None` and `gate.last_transition == None`.
- `contact_count > 0` requires `last_contact_trace` to be a valid `MATERIAL_SURFACE_CONTACT_EVENT_KIND`
  event for this surface whose condition effect advances `contact_count` by exactly one and whose
  resulting `contact_count` equals the persisted `contact_count`.
- `gate.active == true` requires `gate.last_transition` to be a valid rising
  `MATERIAL_SURFACE_MANA_EVENT_KIND` event for this surface.
- `gate.active == false` with `gate.last_transition == Some(t)` requires `t` to be a valid falling
  gate-only event.
- Every persisted gate transition's `before_active`/`after_active` must match the gate-property
  effect fingerprint in the referenced trace.
- Every rising event's condition-property effect must match the surface's `condition` and
  `contact_count` at that time.

### Local proposal and commit boundary

ManaEffectsSystem remains in the existing Phase::Mana; no phase is added. On each tick it iterates
contacted material surfaces in canonical MaterialSurfaceId order. For each surface it reads only the
matching ManaField cell, its fixed-point intensity, and its `last_change` trace.

For each site it applies the boundary semantics to that site's persisted `gate.active` state. It
creates at most one canonical `LocalManaMaterialSurfaceProposal` per surface. The proposal contains
before/after gate state, the local mana before/after values, the local mana trace, and, only for a
rising transition, the before/after material condition.

Proposals are sorted by `EventProposalKey` using `MANA_EFFECTS_SYSTEM_ID`, the surface object id, and
a deterministic operation ordinal, then committed through `CausalTraceStore::commit_batch(time,
Phase::Mana, ...)`. Each changed gate commits one `MATERIAL_SURFACE_MANA_EVENT_KIND` event with:

- a gate-property effect (`active` before/after);
- on a rising transition, the condition-property effect (`condition` +1) in the same event;
- on a falling transition, no condition effect.

After the batch succeeds, update the surface record's `gate.active`, `gate.last_transition`, and
`last_transition` (rising only), and append the appropriate gate transition record. Gate-only
events do not update `condition` or enter the condition transition window.

The existing experiment recipe becomes an ordinary input control: a source to a contacted surface's
matching cell may activate that surface; an equal source to another cell may not do so solely by
raising a chunk or world total. Recipe identity, policy, amount budget, and external origin stay
behind the existing operator redaction boundary.

### Identity collision handling

Both `material_surface_object_id` and the mana cell object id use `cell_object_id(chunk, cell_index)`,
a `u64` hash that is not guaranteed injective. During bootstrap and snapshot import, validate that:

- every material surface maps to a unique `(MATERIAL_SURFACE_OBJECT_KIND, object_id)` pair;
- every mana cell used by `MANA_OBJECT_KIND` maps to a unique `(MANA_OBJECT_KIND, object_id)` pair.

Cross-kind collisions (e.g., a material surface and a mana cell sharing the same object id) are
allowed because the object kind disambiguates them. Detected same-kind collisions reject
deterministically before the state becomes authoritative. This preserves canonical `EventProposalKey`
ordering and exact-cell validation. If future designs add more surfaces or field dimensions per cell,
introduce persisted unique object IDs instead.

### Local evidence design

`ManaField` gains a per-cell `last_change_before: Vec<i64>` vector that records the fixed-point
intensity immediately before the most recent commit to that cell. This is updated by both ordinary
`ManaEvolutionProposal::commit` and `commit_experiment_recipe_mana_source`. It lets the local gate
evidence be captured without decoding opaque fingerprints.

Local mana evidence is stored explicitly in `MaterialSurfaceGateTransition`:

- `local_mana_before`: the matching cell's `last_change_before` value at evaluation time.
- `local_mana_after`: the matching cell's current `intensity` at evaluation time.
- `local_mana_trace`: the `TraceId` of the matching cell's `last_change`.

The referenced trace is still required to be a Mana-phase `MANA_EVENT_KIND` or
`EXPERIMENT_RECIPE_MANA_SOURCE_EVENT_KIND` cell effect for the exact chart-qualified cell. This keeps
the authoritative evidence typed and avoids treating `StateFingerprint` private bytes as a storage
contract.

### Persistence, digest, and replay

Persist gate records, gate transition records, and local mana evidence with
`MaterialSurfaceSnapshot` in a new required major version of material-surface section 0x000C (V2);
reject unsupported versions and malformed, duplicate, unsorted, missing, out-of-extent, or
unknown-trace records before runtime installation. Add `last_change_before` to `ManaField` and bump
mana-field section 0x0003 to V2. Remove the obsolete global gate field from PhysicalCountersSnapshot
in a new required major version of section 0x0006 (V3). Advance the runtime recipe section 0x0001 to
V4 because the system registration vector changes. Advance the authoritative digest schema to V4
because the local gate state and local evidence are state identity inputs.

Runtime::export_snapshot, Runtime::import_snapshot, canonical physical/history digests, and
snapshot reference validation must include every gate record, gate transition, and local evidence
field. The same seed/configuration must replay exactly; continuous and resumed runs must be equal
across both a pre-activation and a post-activation checkpoint. No external state or observer query
may supply or alter a local gate.

Import validation is semantic, not trace-existence-only. Each persisted gate transition trace
must be a Mana-phase `MATERIAL_SURFACE_MANA_EVENT_KIND` event with the gate-property effect for
that exact MaterialSurfaceId and its declared before/after active state. Each local evidence trace
must be either an ordinary `MANA_EVENT_KIND` cell effect or an experiment-recipe source cell effect
for the exact chart-qualified cell, with matching fixed-point before/after values. For rising
transitions the local evidence trace must be a parent of the recorded condition event; for falling
gate-only transitions it must be a parent of the gate-only event. Bootstrap and Action transitions
must carry no local Mana evidence. Existing material/contact-anchor validation in runtime.rs is
extended with these same property, phase, value, address, and parentage checks.

Increase the `MANA_EFFECTS_SYSTEM_ID` registration revision from 1 to 2. Before installing a
snapshot, validate `data.recipe.system_registrations` for exact canonical equality with the
compiled `runtime_system_registrations()` vector, including phase, system schema, revision, and
registration order. Runtime::from_snapshot rejects an old, missing, reordered, or tampered
registration record before restoring scheduler times or authoritative RuntimeState.

### Observer and Explanation boundary

Publish `MaterialSurfaceDelta` schema V3 rather than reinterpret V2 fields. V3 retains V2's
condition/contact/recipe-source-specific optional fields with their existing meanings, then adds
fields 15 `local_mana_before`, 16 `local_mana_after`, and 17 `local_mana_transition_trace_id`.

Add a separate bounded `MaterialSurfaceGateDelta` for gate-only falling/sub-threshold transitions:

```protobuf
message MaterialSurfaceGateDelta {
  uint64 chart_id = 1;
  sint32 chunk_x = 2;
  sint32 chunk_y = 3;
  sint32 chunk_z = 4;
  uint32 cell_ordinal = 5;
  bool before_active = 6;
  bool after_active = 7;
  sint64 local_mana_before = 8;
  sint64 local_mana_after = 9;
  uint64 local_mana_transition_trace_id = 10;
  uint64 gate_transition_trace_id = 11;
  optional uint64 contact_trace_id = 12;
  uint64 transition_tick = 13;
}
```

Rust observer API, protobuf, wire codec, observer session, and TypeScript protocol must round-trip
V3 and gate-only deltas exactly, reject malformed required values, preserve response capacity, and
not claim V3 data through a V2 schema number.

Add `MATERIAL_SURFACE_LOOP_LOCAL_MANA_TRANSITION_SCHEMA` with ExplanationClaimSchemaId 15. Schema
14 (`MATERIAL_SURFACE_LOOP_MANA_TRANSITION_SCHEMA`) remains the generic in-world mana-transition
claim and is frozen for backward compatibility. Schema 15 is the verified-local-coupling claim:
its value is a `NumericClaimValue::Range(min(local_mana_before, local_mana_after),
max(local_mana_before, local_mana_after))`, satisfying the `NumericClaimValue` requirement that
`start <= end`. Direction is conveyed by the paired `MaterialSurfaceDelta` / `MaterialSurfaceGateDelta`
and the active before/after fields, not by the claim range. Evidence traces include the local mana
trace, the gate transition trace, and, for rising transitions, the contact trace. The matching
chart/chunk/cell ordinal is implicit in the scoped query/delta, not encoded inside the claim. Schema
15 must not expose an experiment source record ID, recipe hash, policy schema, external-origin flag,
operator intent, or labels such as reward, punishment, worship, or divine intervention.

Explanation queries are scoped by `MaterialSurfaceId` so that local evidence is attached to the
correct surface. For a rising transition, schema 15 is `Supported`; for a falling gate-only
transition, schema 15 is also `Supported` with the local-mana range evidence. When no local
evidence exists, schema 15 is `Unknown`. It must not infer an operator's purpose or treat a
nonmatching cell as a cause.

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

1. **State & snapshot schema.** Extend `MaterialSurface` with the embedded typed gate and
   `last_contact_trace`. Add `MaterialSurfaceGateTransition` and the `gate_transitions` field to
   `MaterialSurfaceSnapshot`. Remove `mana_effect_active` from `RuntimeState` and
   `PhysicalCountersSnapshot`. Bump material-surface section 0x000C to V2, physical-counters section
   0x0006 to V3, runtime-recipe section 0x0001 to V4, and advance the authoritative digest schema.

2. **Local ManaEffectsSystem.** Rewrite global-total/first-contact logic with pure per-surface local
   gate and condition proposals keyed by MaterialSurfaceId. Apply the four boundary-semantics rules.
   Commit canonically in the Mana phase with exact local mana/contact/gate parents. Remove
   `commit_mana_effect_activity_event`. Validate material-surface object-id uniqueness during
   bootstrap and snapshot import.

3. **Persistence, digest, and validation.** Update section encoders/decoders, import validation,
   reference validation, and physical/history digests for the new gate state, gate transitions, and
   local evidence. Enforce snapshot consistency rules for gate/contact/trace state. Validate system
   registration vector equality before scheduler installation.

4. **Observer/Explanation V3 and schema 15.** Add local-mana fields to `MaterialSurfaceDelta` V3 and
   add `MaterialSurfaceGateDelta`. Add schema 15 as verified-local-coupling while freezing schema 14.
   Scope Explanation queries by surface.

5. **Scenario contract tests.** Add multi-surface locality, historical contact eligibility,
   hysteresis, boundary equality, sub-threshold/falling gate, recipe-source locality, replay,
   save/resume, persistence validation, object-id collision rejection, and observer/Explanation
   redaction tests.

## Verification

All integration tests begin with Runtime::new and its historical bootstrap, never direct
authoritative mutation or a fixture/demo constructor. Extend the completed material-loop suites
with these acceptance tests. The required QA commands are:

```bash
# Run all material-surface loop and observer tests added for this slice.
cargo test -p causafera-runtime --test material_surface_loop
cargo test -p causafera-runtime --test material_surface_observer

# Validate wire/round-trip and Explanation codec coverage for V3 and gate-only deltas.
cargo test -p causafera-observer-wire --test protocol
cargo test -p causafera-explanation

# Run the full workspace suite and the repository CI gate.
cargo test --workspace --all-features
cargo test --workspace --no-default-features
cargo run -p xtask -- ci
```

Scenario groups map to the test files as follows:

- Locality, contact gating, hysteresis, boundary equality, pre/post-contact eligibility, replay,
  save/resume, and persistence validation → `material_surface_loop.rs`;
- Observer V3/gate-only deltas, schema 15, and redaction → `material_surface_observer.rs` and
  `causafera-observer-wire` round-trip tests;
- Explanation IR, schema 15 rendering, and insufficiency → `causafera-explanation` tests.

Each scenario must pass with both RED→GREEN test-runner evidence and a real-surface artifact
(snapshot digest, wire response, or Explanation frame) captured in the orchestrator notepad.

1. a recipe source at a contacted surface's matching cell commits through the Mana phase and
   changes only that surface through a local gate;
2. the same valid source amount at another cell does not change the contacted surface merely
   because chunk/world total mana crosses the threshold;
3. with two contacted surfaces in different cells, raising only cell A activates only surface A;
   raising only cell B activates only surface B; raising both activates both in one Mana-phase batch;
   all cite only their own local mana-cell trace and own contact trace;
4. ordinary repeated material-surface samples at the matching cell still activate the same local
   effect path, proving the slice does not depend on experiment recipes;
5. a local value below the activation threshold changes mana but produces neither a gate nor a
   material condition change, and yields no supported condition-effect or schema-15 claim;
6. boundary equality: inactive gate with `local_mana == threshold` does not transition; active gate
   with `local_mana == threshold - hysteresis` stays active;
7. hysteresis deactivation: active gate with `local_mana < threshold - hysteresis` commits a local
   gate-only falling transition without another condition increment, and the gate-only delta is
   observable;
8. high mana at a surface's matching cell before the first contact produces no material effect;
   after a committed contact, the next Mana-phase evaluation uses the real contact trace as a parent;
9. no-contact, disabled mana effect threshold (`threshold <= 0`), and unavailable/nonmatching local
   cell controls produce no material effect;
10. same-seed runs replay to equal canonical physical/history digests and equal snapshots;
11. continuous and save/resume trajectories are equal before and after local activation;
12. every local Mana-phase material effect has parent-before-child matching mana-cell, contact,
    prior condition, and prior gate traces as specified; a nonmatching mana-cell trace is absent
    from the causal parent set;
13. malformed/duplicate/missing/out-of-extent gate records, bad evidence references, inconsistent
    gate/contact/trace state, material-surface object-id collisions, and old required snapshot
    majors reject before authoritative installation; tampering with a valid trace's surface, cell,
    property, value, kind, phase, gate state, or parentage also rejects;
14. an old, missing, reordered, or tampered system-registration vector rejects before scheduler
    or RuntimeState installation;
15. MaterialSurfaceDelta V3, MaterialSurfaceGateDelta, and schema 15 round-trip through Rust, proto,
    wire, observer session, and TypeScript with bounded capacity; V2 fields retain their existing
    semantics and serialized V3/gate-only redaction tests reject all experiment-policy/operator
    fields;
16. with material signals suppressed, local field and material state remain traced while the
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
bounded windows. No per-agent, global-grid, or unbounded trace cache is introduced. Gate-only
transitions add a small bounded observer projection.

## Observer impact

Use the existing versioned world-chunk query and material delta. Add a gate-only delta for
falling/sub-threshold transitions. Do not add an interactive view or expose operator Meta Truth.
The new values exist only because they are necessary to inspect the local causal claim and must
retain existing response-capacity limits.

## Explanation impact

Use the existing typed material-loop claim and add schema 15 for verified local coupling. It reports
local before/after numeric evidence, trace support, controls, and insufficiency; it does not add a
semantic interpretation layer.

## Persistence impact

Material-surface state, physical counters, runtime recipe, and digests receive required major-version
changes. Import fails closed on incompatible versions; snapshots, replay bundles, and save/resume
contain all local-gate authoritative state. No mutable external source is consulted during replay.

## Cross-domain effects

Existing physical patterns or immutable recipe sources change a local mana cell. The local field
can change the matching contacted material surface; that surface supplies an ordinary bounded
signal; existing perception, subjective-scene construction, and action react without Ground Truth
cell identity. The following candidate may add a terrain carrier into this now-local path.

## Risks

Risks are preserving a disguised global gate, selecting a nonmatching surface, retaining stale
gate state after import, losing local parentage in a batch commit, leaking recipe Meta Truth,
breaking snapshot compatibility without failing closed, mistaking digest divergence for a physical
measure, conflating inactive/no-transition with active/falling semantics, treating optional trace
absence as a zero trace, or failing to scope Explanation/observer output per surface. The required
controls and codec/provenance tests are mandatory mitigations.

## Documentation changes

After implementation evidence lands, update docs/world/mana-topology.md,
docs/ontology/causal-carriers.md, docs/rfc/RFC-PERSIST-001.md,
docs/observer/protocol.md, docs/explanation/explanation-ir.md, and Explanation documentation only
where the implemented local contract requires it. Keep proposed operator architecture documents
Proposed.

## TODO changes

Do not modify the roadmap or backlog for this ExecPlan. Update only evidence-backed portions of
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
- 2026-07-22: Finalized accepted architecture. Embedded typed gate inside MaterialSurface with
  optional `gate.last_transition` and `last_contact_trace`; removed parallel gate map and bare-bool
  options. Historical contact eligibility is an explicit bounded policy of this slice, not a
  universal future material-physics rule.
- 2026-07-22: Resolved Explanation schema ambiguity by freezing schema 14 as the generic in-world
  mana-transition claim and introducing schema 15 as the verified-local-coupling claim.
- 2026-07-22: Specified exact boundary semantics: inactive with `local_mana <= threshold` → no
  transition; inactive with `local_mana > threshold` → rising gate+condition; active with
  `local_mana >= threshold - hysteresis` → no transition; active with
  `local_mana < threshold - hysteresis` → gate-only falling transition.
- 2026-07-22: Added a separate bounded `MaterialSurfaceGateTransition` record and
  `gate_transitions` vector so gate-only transitions survive save/resume without entering the
  condition-transition window.
- 2026-07-22: Specified exact parent sets: rising events cite local mana trace, contact trace,
  prior condition transition, and prior gate transition; falling events cite local mana trace and
  prior gate transition.
- 2026-07-22: Decided to store explicit `local_mana_before`, `local_mana_after`, and
  `local_mana_trace` in the gate transition record rather than decoding opaque fingerprint bytes.
- 2026-07-22: Added snapshot consistency rules linking `contact_count`, `last_contact_trace`,
  `gate.active`, and `gate.last_transition`.
- 2026-07-22: Added material-surface object-id collision validation because
  `material_surface_object_id` is not guaranteed injective; collisions reject deterministically.
- 2026-07-22: Specified `MaterialSurfaceGateDelta` fields including `gate_transition_trace_id`
  and `optional contact_trace_id`, and schema-15 representation as
  `Range(min(before, after), max(before, after))` with trace evidence.
- 2026-07-22: Accepted version bumps: digest schema V4, runtime recipe section 0x0001 V4,
  physical counters 0x0006 V3, material surfaces 0x000C V2, observer MaterialSurfaceDelta V3,
  MANA_EFFECTS_SYSTEM_ID revision 2.

## Progress

Accepted. Implementation authorized by this document.
