# Thermal-to-Material Coupling ExecPlan

**Status:** Accepted

## Goal

Give material surfaces a physically meaningful, authoritative response to local thermal exposure:
a bounded, non-negative **retained-heat** accumulator that exchanges energy with its co-located
`ThermalField` cell inside the same atomic, conservation-checked Physics-phase batch that already
diffuses cell-to-cell energy and injects reservoirs. This closes `TODO-THERMAL-002` and resolves the
previously unassigned `TODO-MATER-???` "material response model" dependency by defining that model
here, rather than treating it as a separate blocker.

This is a physical-locality slice in the same family as the completed local mana-material coupling,
but the response is a conserved energy exchange, not a semantic `condition +1` toggle. It does not
add cross-chart transport, climate, biology, expansion, damage accumulation, or phase change — those
remain later, explicitly named follow-ups.

## Context

`plans/conserved-thermal-energy-carrier.md` (completed) gave the world a fixed-point `ThermalEnergy`
carrier with exact same-chart conservation, but explicitly removed the exploratory draft's
`MaterialSurfaceThermalGate`/`condition +1` idea as "no physically meaningful material response
defined yet; deferred to future tranche." Its own Non-goals list named that tranche's boundary:
"No thermal-to-material gate or material condition change in this tranche." `TODO-THERMAL-002` is
that named follow-up, and its acceptance criteria explicitly forbid the same shortcut again:
"Material surfaces track bounded thermal exposure state; transitions commit trace-backed events; no
'condition +1' semantic toggle."

Separately, `plans/local-mana-material-surface-coupling.md` (completed) embedded a typed
`MaterialSurfaceManaGate` into `MaterialSurface`, giving every historically-contacted surface a
one-directional, hysteresis-gated reaction to its co-located `ManaField` cell. That slice is the
right shape for a *derived, one-directional* reaction to an external field. It is the wrong shape
here: the backlog's own ontology line for `TODO-THERMAL-002` reads "Energy and matter exchange
through **conserved carriers**," and the goal itself names "retained heat" first among its examples.
An accumulator denominated in `ThermalEnergy` that only ever reads the field and never debits it
would invent energy, which the entire carrier tranche (`i128` preflight, `ThermalConservationReceipt`,
abort-on-nonzero-residual) exists to make impossible. So this tranche defines a **bidirectional
conserved** exchange: material retained energy is a third conserved bucket alongside cell energy and
reservoir budgets, participating in the same per-tick residual-must-be-zero accounting.

Every bootstrapped `MaterialSurface` today sits at `(chunk, cell_index = 0)` for every chunk in
`state.active_chunks`, which is exactly `thermal_active_region.active_chunks()` (both are built from
the same `active_chunk_keys` in `RuntimeState::new`). `MaterialSurfaceId` and `ThermalCellKey` share
the same `(ChartChunkCoord, u16 cell_index)` shape, so every material surface is guaranteed a
co-located, resident thermal cell today, by construction, not by runtime luck. This tranche does not
add a silent-exclusion path for a hypothetical surface without a co-located field: per this
repository's own guidance not to add error handling for scenarios that cannot happen, a material
surface with no matching thermal field is treated as an internal invariant violation
(`ThermalError::PositionOutsideField`, the same error the domain layer already returns for any other
out-of-field cell key), not a runtime condition to tolerate.

## Relevant invariants

- **INV-016** — Authoritative mutation is phase controlled.
- **INV-017** — Performance is architectural.
- **INV-019** — Emergence must be inspectable.
- **INV-038** — State digests are identities, not physical metrics.
- **INV-039** — Production state requires causal initialization.
- **INV-042** — Architecture remains modular and cohesive.
- **INV-043** — The world is one coherent spatial system; missing coupling is unimplemented, not
  intentionally impossible.

## Ontology domains affected

- **Energy** — extends M2 (conserved storage/transfer) with a third conserved bucket (material
  retained energy) inside the same atomic batch.
- **Matter** — advances from "chart-qualified material surfaces with durable condition state" to
  also carrying a bounded, conserved, trace-backed thermal-exposure state. This resolves the
  `TODO-MATER-???` dependency named in the backlog entry.
- **Physics** — no new fixed-point primitive; reuses `ThermalEnergy`.
- **Observer / Explanation** — adds a bounded per-surface thermal-exchange delta and a new
  Explanation claim schema.

## Causal carriers affected

- New conserved bucket: material-surface retained thermal energy, denominated in the existing
  `ThermalEnergy` fixed-point type, exchanged with the co-located `ThermalField` cell.
- Existing carrier reused: `ThermalFieldSet` cell energy and its frozen per-tick pre-state; the
  existing reservoir accepted/rejected headroom vocabulary, generalized to a second, symmetric case.
- Existing carrier reused: `MaterialSurfaceId` chart-qualified cell addressing (unchanged).
- Deferred carriers: heterogeneous per-material thermal properties (from `causafera_types::Material`),
  expansion, damage accumulation, phase change, cross-chart material thermal transport, biological or
  mana coupling to the material's retained heat.

## Relevant documents

- `docs/vision/project-thesis.md`
- `docs/architecture/invariants.md`
- `docs/architecture/detailed-development-rebaseline.md`
- `docs/ontology/domain-coverage-matrix.md`
- `docs/ontology/causal-carriers.md`
- `docs/rfc/RFC-MANA-001.md` (precedent for a local, per-cell gate; explicitly not reused here)
- `docs/rfc/RFC-TRACE-001.md`
- `plans/conserved-thermal-energy-carrier.md`
- `plans/local-mana-material-surface-coupling.md`

## Current state

- `crates/causafera-domains/src/thermal/{field,evolution,diffusion,injection,receipts,records,proposal,neighbor,arithmetic}.rs`
  implement the completed carrier: `ThermalFieldSet::propose_evolution` calls `accept_injections`
  (reservoir headroom, adjusts the frozen per-tick pre-state) then `preflight_faces` (pairwise signed
  flux across six faces, canonical `key >= neighbor` ownership, `i128` accumulator with per-step bound
  checks via `update_delta`/`check_energy_bounds`), then `records_for_cells` and `conservation_receipt`.
- `ThermalParameters` validates `0 < transfer_fraction <= scale / 6` (equivalently
  `6 * transfer_fraction <= scale`) so that six simultaneous face outflows cannot drive a cell
  negative.
- `crates/causafera-runtime/src/thermal.rs` (`ThermalReservoirSystem`, `ThermalEvolutionSystem`) and
  `thermal_events.rs` build and commit one atomic `Phase::Physics` batch per tick: reservoir-transfer
  events, cell-change events, one conservation event, in that canonical order.
- `crates/causafera-runtime/src/material_surface.rs` defines `MaterialSurface { condition,
  contact_count, last_transition, last_contact_trace, gate: MaterialSurfaceManaGate }`. `gate` is the
  completed one-directional local mana coupling; it is the pattern this tranche explicitly does not
  copy for thermal, per the acceptance criteria's "no condition +1" instruction and the conserved-carrier
  requirement above.
- `MaterialSurfaceBootstrapStage::bootstrap` (`crates/causafera-runtime/src/bootstrap.rs`) creates
  exactly one `MaterialSurface` at `(chunk, 0)` per chunk in `state.active_chunks`, which is the same
  chunk set used to build `thermal_active_region` and `thermal_fields`.
- `RandomStream` keys are `(world_seed, time, phase, system_id)` where `system_id` is the scheduler's
  registration-order counter (`Scheduler::register_system`'s `next_system_id`), which is **distinct**
  from the `*_SYSTEM_ID` constants used only for `EventProposalKey` ordering
  (`THERMAL_EVOLUTION_SYSTEM_ID = 12`, etc.). This tranche adds no new `System`/`register_system` call,
  so no RNG stream is renumbered and `runtime_system_registrations()` does not change.
- `crates/causafera-runtime/src/snapshot_sections.rs` encodes thermal section `0x000E` (major 1) and
  material-surface section `0x000C` (major 2). `import_thermal_snapshot` in `runtime.rs` already
  reconstructs and validates each `ThermalCellTransferReceipt`'s signed-flux equation
  (`pre_state - sum(face.signed_flux) == post_state`) and the latest batch's receipt-vs-current-energy
  bind; both checks must be extended for the new material term.
- `crates/causafera-observer-api/src/query.rs` defines `MaterialSurfaceDelta`, `MaterialSurfaceGateDelta`,
  and `ThermalFieldDelta` as separate bounded delta shapes (a gate-only or thermal-only transition does
  not overload the condition-centric `MaterialSurfaceDelta`). `crates/causafera-explanation/src/ir.rs`
  defines schemas 14/15 (material/mana) and 16 (`THERMAL_CARRIER_CONSERVATION_SCHEMA`); 17 is unused.

## Proposed architecture

### 1. Authoritative material thermal state

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MaterialSurfaceThermalState {
    pub retained_energy: ThermalEnergy, // in [0, ThermalParameters.material_thermal_capacity]
    pub last_exchange: Option<TraceId>, // None until the first non-zero exchange event
}
```

Added to `MaterialSurface` as a new field `thermal: MaterialSurfaceThermalState`. Bootstrap sets
`retained_energy: ThermalEnergy::ZERO, last_exchange: None` — mirroring exactly how `gate` bootstraps
inactive with `last_transition: None`, because the existing `MATERIAL_SURFACE_BOOTSTRAP_EVENT`
targets only the `condition` property and `CausalEffect` rejects an unchanged (0→0) fingerprint, so no
event can anchor a zero-value thermal property at bootstrap. A first-ever exchange event therefore
cites no "prior exchange" parent, exactly as a surface's first gate activation cites no
`prior_gate_trace` — this is established precedent, not a new gap.

### 2. Homogeneous exchange parameters

Extend `ThermalParameters`:

```rust
pub struct ThermalParameters {
    pub transfer_fraction: i64,
    pub heat_capacity: i64,
    pub scale: i64,
    pub material_exchange_fraction: i64,   // new
    pub material_thermal_capacity: i64,    // new
}
```

`ThermalParameters::validate` changes its bound from `transfer_fraction <= scale / 6` to:

```text
0 < transfer_fraction
0 <= material_exchange_fraction
0 < material_thermal_capacity <= ThermalEnergy::MAX
6 * transfer_fraction + material_exchange_fraction <= scale
```

A cell now has up to seven simultaneous outflows in the worst case (six faces plus one co-located
material sink), so the coefficient bound must cover all seven, not just six. `material_exchange_fraction`
is allowed to be exactly `0` — not as a special-cased "disabled" branch, but because the flux formula
(Section 3) computes `floor(magnitude * 0 / scale) = 0` unconditionally in that case, so coupling
turns itself off for free wherever a world configuration (or an existing test fixture that predates
this tranche and asserts exact diffusion-only numbers) does not want it. This keeps every existing
`ThermalParameters` call site mechanically extendable with `material_exchange_fraction: 0` without
touching its other numeric assertions, while still requiring the coefficient bound to hold globally
(a single set of parameters cannot know per-tick which cells carry a material site, so the bound must
be safe for the worst case where every cell does). At the current runtime default
(`transfer_fraction = 128, scale = 1024`), `6 * 128 = 768`, leaving up to `256` of headroom for a
non-zero `material_exchange_fraction`; the chosen production default is recorded in the Decision log.
Both new fields are homogeneous across all surfaces, exactly as `heat_capacity` is homogeneous today —
`causafera_types::Material`'s per-material `thermal_conductivity`/`specific_heat` (`f64`, unused by the
runtime) are not pulled in this tranche.

### 3. Domain-level material exchange, integrated additively into `propose_evolution`

The domain layer stays ignorant of `MaterialSurfaceId`, `contact_count`, and the runtime's gate
concept. It only sees a bounded map of thermal-cell-keyed material sites:

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ThermalMaterialSite {
    pub retained_before: ThermalEnergy,
    pub last_exchange: Option<TraceId>, // the surface's prior exchange trace, for cell-change parentage
}
```

`ThermalEvolutionRequest` gains `pub materials: &'a BTreeMap<ThermalCellKey, ThermalMaterialSite>`.
The runtime builds this map each tick from `state.material_surfaces` by direct lookup — every
bootstrapped surface's `(chunk, cell_index)` is a valid `ThermalCellKey` today (see Context above), so
this is a plain key conversion, not a filter. If a future change ever created a surface without a
matching field, `propose_evolution` surfaces that as `ThermalError::PositionOutsideField`, the same
error already returned for any other out-of-field key — it is not handled as a tolerated runtime
condition.

`preflight_faces` (renamed in signature only enough to add material handling; its existing loop,
ownership rule, and per-step bound checks are unchanged) gains one additional step per cell, executed
after the six-face loop, using the same frozen `pre_state` values the face loop already uses (so
same-tick propagation and no-cascade semantics match the face algorithm exactly):

```text
for each key in pre_state.keys():
    ... existing six-face loop, unchanged ...
    if let Some(site) = materials.get(&key):
        raw_diff = pre_state[key] - site.retained_before          // i128
        magnitude = |raw_diff|
        candidate = floor(magnitude * material_exchange_fraction / scale)
        if raw_diff > 0:                                          // cell hotter: cell -> material
            headroom = material_thermal_capacity - site.retained_before
            accepted = min(candidate, headroom)
            rejected = candidate - accepted
            cell_delta -= accepted    // via update_delta, existing [0, ThermalEnergy::MAX] bound check
            material_after[key] = site.retained_before + accepted
        elif raw_diff < 0:                                        // material hotter: material -> cell
            accepted = candidate                                  // candidate <= retained_before, proven below; no headroom cap needed
            rejected = 0
            cell_delta += accepted
            material_after[key] = site.retained_before - accepted
        else:
            accepted = 0; rejected = 0
            material_after[key] = site.retained_before
```

**Why the falling direction needs no headroom cap:** `magnitude = retained_before - pre_state[key] <=
retained_before` (`pre_state[key] >= 0`), and `material_exchange_fraction <= scale`, so
`candidate = floor(magnitude * material_exchange_fraction / scale) <= magnitude <= retained_before`.
The material can never be asked to give away more than it holds. `check_material_bounds` (analogous to
`check_energy_bounds`, range `[0, material_thermal_capacity]`) still guards this defensively, consistent
with the codebase's checked-arithmetic style, but is not load-bearing for correctness here — only the
heating direction's headroom cap is.

The cell side reuses the existing `update_delta`/`check_energy_bounds` machinery unchanged (a
material sink is simply one more delta contribution against the same `[0, ThermalEnergy::MAX]` bound
that six-face outflows already respect, now covered by the widened coefficient bound in Section 2).
There is at most one material site per cell (current bootstrap creates exactly one surface per active
chunk at `cell_index = 0`; a future multi-surface-per-chunk design would need a distinct keying scheme
and is out of scope), so there is no pairwise-ownership question analogous to the face `key >=
neighbor` rule.

`preflight_faces` returns one additional bounded map alongside the existing three:
`AfterMaterials = BTreeMap<ThermalCellKey, ThermalEnergy>` (every material site's post-tick retained
energy, including unchanged ones, needed to compute the conservation total) and
`MaterialRecords = BTreeMap<ThermalCellKey, ThermalMaterialTransferRecord>` (populated **only** for
sites with `signed_flux != 0`, exactly like `FaceRecords` already only holds non-zero-flux faces —
this is what lets `records_for_cells` reuse its existing "does this cell have anything to report"
skip condition and keeps per-tick receipt growth bounded to actually-changed cells, not every
material-bearing cell every tick):

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ThermalMaterialTransferRecord {
    pub retained_before: ThermalEnergy,
    pub retained_after: ThermalEnergy,
    /// Positive: flowed cell -> material. Negative: flowed material -> cell.
    /// Same sign convention as `ThermalFaceRecord::signed_flux`.
    pub signed_flux: i64,
    pub rejected: ThermalEnergy, // non-zero only when heating hit `material_thermal_capacity`
}
```

`ThermalFieldSet::propose_evolution` threads `request.materials` through to `preflight_faces` and
passes the resulting `AfterMaterials`/`MaterialRecords` into `records_for_cells` (extended to accept
them) and `conservation_receipt` (extended to sum a third bucket).

### 4. Cell-change parent sets and receipts gain the material term

`ThermalCellChange.parent_traces` gains the co-located material's `last_exchange` trace (when `Some`
and the flux was non-zero) alongside the cell's own prior trace, its six neighbors' prior traces, and
any contributing reservoirs' `last_change`. This is why `ThermalMaterialSite` (Section 3) carries
`last_exchange`, not just `retained_before` — `records_for_cells` needs it exactly the way it already
pulls `.last_change` off `reservoirs: &BTreeMap<ThermalReservoirId, ThermalReservoir>`, and it has no
other source for it (the domain layer does not otherwise track per-surface trace history). The bound
in the completed carrier tranche's Section 4.7 (`1 + 6 + R` parent traces per cell-change event)
becomes **`1 + 6 + R + 1`** (at most one material parent, since there is at most one material site per
cell).

`ThermalCellTransferReceipt` gains `pub material: Option<ThermalMaterialTransferRecord>`. A cell whose
only participation this tick is a non-zero material exchange (no face flux, no reservoir) still
receives a receipt — `records_for_cells`'s existing skip condition
(`cell_faces.is_empty() && cell_reservoirs.is_empty()`) is extended to also check
`cell_material.is_none()`.

`ThermalConservationReceipt` gains:

```rust
pub total_material_retained_before: i128,
pub total_material_retained_after: i128,
```

and the residual equation becomes:

```text
residual = (cells_after + reservoirs_after + materials_after)
         - (cells_before + reservoirs_before + materials_before)
```

still asserted to equal exactly zero, aborting the whole batch with the existing
`RuntimeError::ThermalConservationViolation` path otherwise. `materials_before`/`materials_after` sum
**every** surface in `state.material_surfaces`, not just the ones whose flux happened to be non-zero
this tick — the exchange step can only move energy between a cell and its own co-located site, so an
untouched surface contributes the same value to both sides and nets to zero, but the sum itself must
stay total, not partial, or a future change that legitimately narrows the per-tick map would silently
corrupt the residual check without any test catching it.

### 5. One event per changed material surface, in the existing atomic batch

No new `System` is registered. `ThermalEvolutionSystem::execute` (unchanged control flow) builds the
material-sites map from `state.material_surfaces`, passes it through `propose_evolution`, and
`build_thermal_events` (in `thermal_events.rs`) gains a fourth event-building step alongside
reservoir-transfer, cell-change, and conservation events:

```text
CausalTarget:
  object_kind: MATERIAL_SURFACE_OBJECT_KIND
  object_id:   material_surface_object_id(id)     // == cell_object_id(chunk, cell_index)
  property:    MATERIAL_SURFACE_THERMAL_RETAINED_PROPERTY  (new)
before: fingerprint_i64(surface_thermal_fingerprint_domain, retained_before)
after:  fingerprint_i64(surface_thermal_fingerprint_domain, retained_after)
```

Emitted only when `retained_before != retained_after` (`CausalEffect` rejects an unchanged
fingerprint, so a zero-flux surface emits nothing this tick — the same net-zero precedent as thermal
cells and mana gate transitions). **Parents** (pre-existing traces only, ascending, deduplicated):

1. the surface's own prior `thermal.last_exchange` trace, when `Some`;
2. the co-located cell's prior `last_change` trace (the value before this tick's own cell-change
   event, i.e. the same `prior_traces` map `records_for_cells` already threads through for face and
   reservoir parents).

**Canonical ordering** extends the existing three-group order
(`plans/conserved-thermal-energy-carrier.md` §4.6) to four groups, each internally sorted:

1. reservoir-transfer events, by `ThermalReservoirId`;
2. material-exchange events, by `MaterialSurfaceId`'s `(ChartChunkCoord, cell_index)` — i.e. by
   `ThermalCellKey`, since the domain layer only knows cells;
3. cell-change events, by `ChartChunkCoord` then row-major index;
4. one conservation event.

All four groups commit together via the existing `CausalTraceStore::commit_batch(time, Phase::Physics,
...)` call already in `ThermalEvolutionSystem::execute`; `EventProposalKey` continues to use
`THERMAL_EVOLUTION_SYSTEM_ID` for all four groups (matching how reservoir-transfer and cell-change
events already share that same system ID today), disambiguated by ordinal.

After the batch commits, for every surface with a committed exchange trace:
`state.material_surfaces[id].thermal.retained_energy = retained_after` and
`.thermal.last_exchange = Some(trace)`. A bounded transition record is appended
(`MAX_MATERIAL_SURFACE_TRANSITIONS`-capped, eviction rule mirroring
`record_material_surface_gate_transition`'s "evict a non-terminal-looking entry first" pattern, applied
here as "evict the oldest thermal transition first" since every thermal transition is equally
inspectable, unlike gate transitions which distinguish rising/falling):

```rust
pub struct MaterialSurfaceThermalTransition {
    pub id: MaterialSurfaceId,
    pub occurred_at: SimulationTime,
    pub before_retained: i64,
    pub after_retained: i64,
    pub cell_pre_state: i64,
    pub signed_flux: i64,
    pub cell_trace: TraceId,       // the co-located cell's prior last_change cited as parent
    pub transition_trace: TraceId,
}
```

### 6. Persistence

- `MaterialSurface` snapshot record (section `0x000C`) gains `retained_energy: i64`,
  `last_exchange: Option<TraceId>`; bump `MATERIAL_SURFACE_SECTION_MAJOR` from 2 to 3.
- `MaterialSurfaceSnapshot` gains `thermal_transitions: Vec<MaterialSurfaceThermalTransitionSnapshot>`,
  capped at `MAX_MATERIAL_SURFACE_TRANSITIONS`, encoded/decoded and import-validated the same way
  `gate_transitions` already is (strict trace ordering, surface existence, effect/fingerprint match).
- Thermal section (`0x000E`) parameter encoding gains the two new `ThermalParameters` fields; bump
  `THERMAL_SECTION_MAJOR` from 1 to 2. `ThermalCellTransferReceipt` encoding gains an optional material
  record (presence flag + `retained_before`/`retained_after`/`signed_flux`/`rejected`).
  `ThermalConservationReceipt` encoding gains the two new `i128` totals.
- Bump `CURRENT_DIGEST_SCHEMA_VERSION` because authoritative state (material retained energy, its
  trace anchor) is now a digest input.
- `import_thermal_snapshot`'s existing receipt-flux equation
  (`pre_state - sum(face.signed_flux) == post_state`) becomes
  `pre_state - sum(face.signed_flux) - material.signed_flux.unwrap_or(0) == post_state`. The existing
  "latest batch's receipt post-state matches current field energy" check is unchanged (it already
  reads `post_state`, which already reflects the material term). A new, symmetric check validates each
  surface's persisted `retained_energy` against the latest batch's material record for its cell when
  one exists (mirroring the existing latest-receipt-vs-current-energy bind for cells), and validates
  `retained_energy` for every surface stays within `[0, material_thermal_capacity]`.
- No change to `runtime_system_registrations()` (section `0x0001` unaffected) — no new `System` is
  registered, per Section 3 above.

### 7. Observer and Explanation

Add a new bounded delta shape (not an overload of the condition-centric `MaterialSurfaceDelta`,
mirroring how `MaterialSurfaceGateDelta` was kept separate from it):

```rust
pub struct MaterialSurfaceThermalDelta {
    pub chart_id: u64, pub chunk_x: i32, pub chunk_y: i32, pub chunk_z: i32, pub cell_ordinal: u16,
    pub before_retained: i64,
    pub after_retained: i64,
    pub cell_pre_state: i64,
    pub signed_flux: i64,
    pub thermal_exchange_trace_id: u64,
    pub transition_tick: u64,
}
```

Added to `ObserverWorldSnapshot` as `material_surface_thermal_deltas: Vec<MaterialSurfaceThermalDelta>`
with the same bounded-capacity convention as the existing delta lists (cap at 64, oldest evicted
first). Add Explanation schema 17, `MATERIAL_SURFACE_THERMAL_EXCHANGE_SCHEMA`, reporting a
`NumericClaimValue::Range(min(before, after), max(before, after))` retained-energy range (satisfying
`start <= end`, direction conveyed by the paired delta, matching schema 15's precedent), with evidence
traces `[thermal_exchange_trace_id, cell_trace]`, scoped by `MaterialSurfaceId`, `Unknown` when no
exchange evidence exists for the queried surface. It must not expose `Material`'s per-type properties,
a temperature figure, or any damage/phase label — those remain out of scope.

Extend the Rust observer API, `proto/causafera/observer/v1/query.proto` and `explanation.proto`,
`causafera-observer-wire` encode/decode, the observer session, and
`packages/observer-protocol/src/index.ts` for round-trip coverage of the new delta and schema. No
Tauri/React UI change: per AGENTS.md, UI work batches after a stable read model enables a complete
inspection workflow, and this bounded delta does not by itself justify a new panel.

## Primitive vs emergent review

**Primitive:** material retained thermal energy (fixed-point, bounded, conserved); the
material-exchange fraction and capacity parameters; the per-tick signed flux between a cell and its
co-located material; the transition trace anchor.

**Emergent / observer-only:** any notion of "hot," "cold," "warming," "cooling," "saturated," or
"damaged" material; a derived temperature for the material (would require a heat-capacity-style
divisor, not introduced here); expansion, phase change, or structural damage (deferred, named below).

## Non-goals

1. No temperature derivation for material surfaces (energy stays the authoritative unit, exactly as
   for thermal cells).
2. No heterogeneous per-material thermal properties from `causafera_types::Material`
   (`thermal_conductivity`/`specific_heat`); parameters stay homogeneous, like `heat_capacity` today.
3. No expansion, damage accumulation, or phase-change modeling — named as follow-up TODOs.
4. No cross-chart material thermal transport.
5. No mana or biological coupling to the material's retained heat.
6. No new scheduler `System`, no new RNG stream, no change to `runtime_system_registrations()`.
7. No reuse of `condition` or the mana gate's rising/falling hysteresis shape for this response.
8. No UI panel.

## Implementation stages

### Stage 1 — Domain types, parameters, and the additive preflight change

Files: `crates/causafera-domains/src/thermal/{records.rs (types), field.rs, evolution.rs,
diffusion.rs, receipts.rs, proposal.rs}`.

- Add `ThermalMaterialSite`, `ThermalMaterialTransferRecord` to `records.rs` (alongside the existing
  record types); extend `ThermalParameters` and its `validate()` bound; extend
  `ThermalConservationReceipt` with the two material totals; extend `ThermalCellChange` and
  `ThermalCellTransferReceipt` with the material term.
- Extend `preflight_faces` additively (new `materials` parameter, new per-cell step after the
  six-face loop, two new return maps) without restructuring the existing loop or its ownership rule.
  Generalize `signed_flux` to take an explicit `(fraction, scale)` pair instead of a whole
  `ThermalParameters`, so both faces and the material step call the same helper.
- Extend `records_for_cells` (parent set, receipt inclusion, skip condition) and
  `conservation_receipt` (third bucket) in `receipts.rs`.
- Extend `ThermalEvolutionRequest`/`propose_evolution` in `evolution.rs`/`proposal.rs` to thread the
  material map through.
- Confirmed `ThermalEnergy::MAX` is a `const Self` and `ThermalEnergy::get()` is `const fn`, so
  `ThermalParameters::validate`/`new` stay `const fn` with the widened bound. Every existing
  `ThermalParameters::new(...)` call site (positional, 3 args today) and every struct-literal
  construction must be updated for the two new fields in this stage, not discovered incrementally via
  compiler errors: `causafera-domains/tests/{thermal_diffusion.rs, thermal_parameters.rs}`,
  `causafera-runtime/src/runtime.rs`, `causafera-runtime/src/snapshot_sections.rs`
  (`decode_thermal_section`'s struct-literal construction), `causafera-runtime/tests/{thermal_reservoir.rs,
  thermal_boundaries.rs, thermal_receipts.rs, support/thermal.rs}`.

Acceptance:
- `cargo check -p causafera-domains` passes.
- New tests in `crates/causafera-domains/tests/thermal_material_coupling.rs`: symmetric heating
  (material colder than cell), symmetric cooling (material hotter than cell), capacity-limited
  heating leaves the rejected remainder in the cell, seven-simultaneous-outflow non-negativity (six
  cold neighbors plus a cold material sink), exact three-bucket conservation over many ticks, invalid
  `ThermalParameters` (`6 * transfer_fraction + material_exchange_fraction > scale`,
  `material_thermal_capacity <= 0` or `> ThermalEnergy::MAX`) rejects.

### Stage 2 — Runtime state, event building, and bootstrap

Files: `crates/causafera-runtime/src/{material_surface.rs, thermal.rs, thermal_events.rs,
bootstrap.rs, runtime.rs (event kind/property constants, RuntimeState field, config default)}`.

- Add `MaterialSurfaceThermalState` and the `thermal` field on `MaterialSurface`; add
  `MATERIAL_SURFACE_THERMAL_RETAINED_PROPERTY` and `MATERIAL_SURFACE_THERMAL_EXCHANGE_EVENT_KIND`
  constants; add fingerprint helpers; add `MaterialSurfaceThermalTransition` and its bounded
  `state.material_surface_thermal_transitions` list with recorder/evictor.
- `MaterialSurfaceBootstrapStage::bootstrap` initializes `thermal: MaterialSurfaceThermalState {
  retained_energy: ThermalEnergy::ZERO, last_exchange: None }`.
- `ThermalEvolutionSystem::execute` builds the material-sites map from `state.material_surfaces`
  filtered to resident thermal chunks, passes it into `propose_evolution`, and after commit applies
  `retained_after`/exchange traces back onto `state.material_surfaces` and records bounded
  transitions.
- `thermal_events.rs`'s `build_thermal_events` gains the material-exchange event group with its
  parent-set rule and canonical ordering position.
- Add `ThermalParameters::new(...)` call-site defaults for `material_exchange_fraction`/
  `material_thermal_capacity` in `RuntimeState::new` (values recorded in the Decision log).
- Add validation functions mirroring `validate_material_surface_gate_state`/
  `validate_material_surface_gate_transition_history` for the new thermal transition record, wired
  into the existing `RuntimeState` snapshot-recomputation validation pass.

Acceptance:
- `cargo check -p causafera-runtime` passes.
- Existing `thermal_*` and `material_surface_*` integration test suites still pass unmodified except
  where they must be updated for the new struct fields (compile-level updates only, no semantic
  change to prior assertions).
- No new `System::run` registrations exist in `Runtime::new`; `runtime_system_registrations()` is
  unchanged; a legacy-ID/RNG-stability test confirms existing system IDs 0–12 and their RNG streams
  are byte-identical to before this tranche for an all-zero-thermal, no-contact configuration.

### Stage 3 — Persistence, digest, and validation

Files: `crates/causafera-runtime/src/{snapshot_sections.rs, runtime.rs (import/export, digests)}`.

- Bump `MATERIAL_SURFACE_SECTION_MAJOR` to 3 and `THERMAL_SECTION_MAJOR` to 2; extend encode/decode
  for the new fields on both sections.
- Bump `CURRENT_DIGEST_SCHEMA_VERSION`; include material retained energy and its trace anchor in the
  physical digest.
- Extend `import_thermal_snapshot`'s receipt-flux equation and the latest-batch-vs-current-energy
  check with the material term; add the new surface-retained-energy-vs-latest-material-record bind
  and the `[0, material_thermal_capacity]` range check.
- Extend `import_material_surfaces` for the new snapshot fields and the `thermal_transitions` list
  (bounded, strictly ordered, surface-existence-checked, effect-fingerprint-checked).

Acceptance:
- Snapshot round-trip tests pass, including a multi-tick run with both heating and capacity-limited
  and cooling exchanges.
- Malformed thermal sections (forged material signed flux, out-of-range retained energy, retained
  energy exceeding `material_thermal_capacity`, stale/duplicate/misordered `thermal_transitions`)
  reject deterministically before authoritative installation, following the exact pattern the
  2026-07-26 receipt-reconciliation remediation established for the face term.

### Stage 4 — Observer and Explanation

Files: `crates/causafera-observer-api/src/query.rs`, `crates/causafera-explanation/src/ir.rs`,
`crates/causafera-runtime/src/runtime.rs` (mapping), `proto/causafera/observer/v1/{query,explanation}.proto`,
`crates/causafera-observer-wire/src/protocol.rs`, `packages/observer-protocol/src/index.ts`.

- Add `MaterialSurfaceThermalDelta`, wire it into `ObserverWorldSnapshot` with the existing 64-entry
  bounded-capacity convention.
- Add Explanation schema 17 (`MATERIAL_SURFACE_THERMAL_EXCHANGE_SCHEMA`), scoped by `MaterialSurfaceId`.
- Round-trip through Rust → protobuf → wire → TypeScript.

Acceptance:
- `cargo test -p causafera-observer-wire --test protocol` passes with the new delta round-tripping.
- `cargo test -p causafera-explanation` passes with schema 17's `Supported`/`Unknown` cases.
- `pnpm --dir packages/observer-protocol typecheck` passes.

### Stage 5 — Verification, benchmarks, and documentation

- Full-suite tests across `causafera-domains`, `causafera-runtime`, `causafera-observer-wire`, and
  `causafera-explanation`.
- Update `docs/ontology/causal-carriers.md` (material retained energy as a physical carrier),
  `docs/ontology/domain-coverage-matrix.md` (Energy/Matter rows), `CHANGELOG.md`,
  `docs/development/todo-backlog.md` (close `TODO-THERMAL-002`; add named follow-ups for expansion/
  damage/phase-change and heterogeneous material properties), `docs/roadmap/roadmap.md` if it
  references this tranche, and `PLANS.md`'s Active Planning list.

## Verification

### V1 — Symmetric heating: cold material, hot cell
Material `retained_before = 0`, cell `pre_state = E > 0`, no other participants. One tick.
Verify: `signed_flux > 0` (cell → material), `retained_after = signed_flux`,
`cell_after = E - signed_flux`, `signed_flux = floor(E * material_exchange_fraction / scale)`
(assuming below capacity), residual over the tick is zero.
Test: `crates/causafera-domains/tests/thermal_material_coupling.rs::symmetric_heating`.

### V2 — Symmetric cooling: hot material, cold cell
Material `retained_before = R > 0`, cell `pre_state = 0`. One tick.
Verify: `signed_flux < 0` (material → cell), magnitude `<= R`, `retained_after = R - |signed_flux| >= 0`,
`cell_after = |signed_flux|`, residual zero.
Test: `..._material_coupling.rs::symmetric_cooling`.

### V3 — Capacity-limited heating
Material `retained_before` near `material_thermal_capacity`, cell very hot.
Verify: `accepted = capacity - retained_before`, `rejected = candidate - accepted > 0`, the rejected
amount stays in the cell (`cell_after = E - accepted`, not `E - candidate`), `retained_after =
capacity` exactly, residual zero.
Test: `..._material_coupling.rs::capacity_limited_heating_leaves_the_remainder_in_the_cell`.

### V4 — Seven-simultaneous-outflow non-negativity
A cell with energy `E`, six cold neighbors, and a cold co-located material sink; one tick.
Verify: cell energy stays `>= 0`; total of (cell + six neighbors + material) is exactly conserved;
this is the material-tranche analogue of the carrier tranche's V1/V10.
Test: `..._material_coupling.rs::seven_way_outflow_stays_non_negative`.

### V5 — Exact three-bucket conservation over many ticks
Multiple chunks, reservoirs injecting, cross-chunk face flux, and material exchange all active; run
many ticks against the real production runtime (not a dedicated hand-built fixture).
Verify: `sum(cells) + sum(reservoirs) + sum(materials)` equals the initial total exactly at every
tick, not only at the end.
Test: `crates/causafera-runtime/tests/support/thermal.rs::total_energy` (rewritten to sum all three
buckets from `RuntimeSnapshotData`, not just fields + reservoirs), exercised by
`thermal_reservoir.rs::exact_global_conservation` (64 ticks) and `::exhaustion` (10 ticks). No
separate dedicated fixture file was needed once the shared helper covered the real production
runtime, which has material coupling active by default.

### V6 — Invalid parameter rejection
`material_exchange_fraction < 0`; `material_thermal_capacity <= 0` or `> ThermalEnergy::MAX`;
`6 * transfer_fraction + material_exchange_fraction > scale` (including the case where
`material_exchange_fraction == 0` but `transfer_fraction` alone already exceeds `scale / 6`, to prove
the widened bound subsumes rather than loosens the original one).
Verify: `ThermalParameters::validate` rejects; the runtime never installs invalid parameters.
Test: `crates/causafera-domains/tests/thermal_parameters.rs` (extended).

### V7 — Net-zero exchange emits no event
Material and cell already at equilibrium (`raw_diff` rounds to zero flux).
Verify: no `MATERIAL_SURFACE_THERMAL_EXCHANGE_EVENT_KIND` event is committed; `thermal.last_exchange`
and `retained_energy` are unchanged; no transition record is appended.
Test: `causafera-domains/tests/thermal_material_coupling.rs::equilibrium_produces_no_material_record_or_change`
(both zero, no event or record at all). A second, subtler case — the material's outflow is *nonzero*
but exactly offset by a same-tick face inflow, so the *cell's net* change is zero even though a
material-exchange event and receipt do exist for it — is covered by
`::material_and_face_flux_cancel_leaving_no_cell_change_event` and
`::install_committed_traces_preserves_anchor_for_material_only_net_zero_cell` (the latter proves the
cell's field anchor is left untouched rather than dangling, since no `ThermalCellChange` and no
accepted reservoir transfer exist to re-anchor it that tick).

### V8 — First exchange has no prior-exchange parent
A surface's very first non-zero exchange.
Verify: the event's `causes` contains only the co-located cell's prior `last_change` (no
self-referencing "prior exchange" parent), matching the gate's first-activation precedent.
Test: implicitly proven by every persistence round-trip in `thermal_persistence.rs` (notably
`save_resume_equivalence`, 100 ticks across every bootstrapped surface) and
`material_surface_thermal_observer.rs`: `validate_material_surface_thermal_transition_history`
independently recomputes each transition's expected causal parent set by searching the trace store
for a prior same-kind/same-target event, rather than trusting the mutable `last_exchange` field, so
it would reject any surface's first exchange if production code wrongly cited (or wrongly omitted) a
prior-exchange parent. `thermal.rs::second_material_exchange_cites_prior_tick_cell_trace_not_current`
additionally proves the *second* exchange's `cell_trace` is read pre-tick, not post-tick, for the
same surface.

### V9 — (removed)
Dropped after Advisor review: every bootstrapped surface has a co-located resident thermal cell by
construction (see Context), so "surface without a matching field" is an internal-invariant violation
(`ThermalError::PositionOutsideField`), not a runtime condition this tranche tolerates or tests for.
Testing it would mean hand-constructing an inconsistent `RuntimeState` to exercise a branch that
cannot occur through any accepted bootstrap path — exactly the error handling this repository's own
guidance says not to add.

### V10 — Atomic batch failure rolls back all state
Force a causal-capacity rejection (unknown cause) during a tick with active material exchange.
Verify: material `retained_energy`, `last_exchange`, transition history, and all other thermal/
reservoir/cell state are unchanged after the failure (extends the carrier tranche's V15).
Test: `causafera-runtime/src/thermal/tests.rs::atomic_failure_rollback` — the existing test (which
already forces a real `CausalCommitError::UnknownCause`) was extended in place with
`material_surfaces`/`material_surface_thermal_transitions` before/after equality assertions, rather
than duplicated into a new test that would risk exercising a different (weaker) failure path.

### V11 — Malformed snapshot rejection
Forge a material signed flux inconsistent with the receipt equation; set `retained_energy` negative
or above `material_thermal_capacity`; misorder or duplicate `thermal_transitions`; reference an
unknown surface.
Verify: `import_snapshot` rejects before installing state.
Test: `thermal_persistence.rs::runtime_import_rejects_non_latest_receipt_material_flux_forgery` is
the new dedicated test — it proves the *new* mechanism this tranche introduces (the extended
signed-flux equation, `pre_state - sum(face.signed_flux) - material.signed_flux == post_state`)
rejects a coordinated forgery in a **non-latest** batch, not only the latest, mirroring
`runtime_import_rejects_receipt_flux_transition_forgery`'s coverage of the pre-existing face-only
equation. The remaining cases in this V11 (negative/over-capacity `retained_energy`, unknown-surface
reference, misordered/duplicate `thermal_transitions`) are enforced in `import_material_surfaces` and
`validate_material_surface_thermal_state` — structural checks that mirror the existing gate- and
condition-transition validators field-for-field (existence, no-op rejection, strict trace ordering,
bounded cap, capacity range) — but do not have dedicated adversarial unit tests, matching the exact
depth of testing those sibling validators already carry (also untested by dedicated adversarial
cases, only by round-trip happy paths). This is parity with existing precedent, not a new gap.

### V12 — Save/resume equivalence
Run N ticks with active material exchange, save, resume, run N more; compare to a continuous 2N-tick
run.
Verify: canonical physical and history digests are equal; material retained energy and transition
history are equal.
Test: `thermal_persistence.rs::save_resume_equivalence` (extended).

### V13 — Legacy system IDs and RNG stability
Verify the existing registration-order system IDs (0–10, one per `register_system` call in
`Runtime::new` — a distinct namespace from the `*_SYSTEM_ID` constants such as
`THERMAL_EVOLUTION_SYSTEM_ID = 12`, which key `EventProposalKey` ordering, not RNG streams) and their
RNG streams are unchanged, because this tranche registers no new `System`; an all-zero-thermal,
no-contact configuration produces identical legacy subsystem traces to before this tranche except for
the deliberate digest-schema bump.
Test: `crates/causafera-runtime/tests/thermal_determinism.rs::legacy_ids_stable`.

### V14 — Observer/Explanation round-trip
Run a heating and a cooling exchange; query the observer world snapshot and Explanation.
Verify: `MaterialSurfaceThermalDelta` round-trips through Rust → protobuf → wire → TypeScript with
bounded capacity; schema 17 reports the correct range and evidence traces, and `Unknown` for an
unqueried/untouched surface.
Test:
`causafera-observer-wire/src/protocol.rs::world_query_roundtrips_material_surface_thermal_deltas`
(Rust round-trip through the wire codec) and
`::decode_world_snapshot_rejects_thermal_material_deltas_under_v3_schema` (the version gate actually
gates); `causafera-explanation/src/ir.rs`'s three
`material_surface_thermal_exchange_*` tests (range direction, cooling-direction ordering, `Unknown`
carrying current retained energy); `causafera-runtime/tests/material_surface_thermal_observer.rs`'s
three tests against a live production `Runtime` (bounded delta projection,
unknown-surface rejection distinct from no-evidence, `Unknown` before the first exchange becoming
`Supported` after). TypeScript-side decoding was added to
`packages/observer-protocol/src/index.ts` and verified via
`pnpm --dir packages/observer-protocol typecheck` (no dedicated TS test file exists in this package;
none existed before this tranche either).

### V15 — Full CI gate
```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo test --workspace --no-default-features
cargo run -p xtask -- ci
pnpm install --frozen-lockfile
pnpm lint
pnpm typecheck
pnpm build
pnpm --dir packages/observer-protocol typecheck
```

## Benchmark plan

- **Workload:** the existing thermal benchmark's active region plus one material surface per chunk
  exchanging every tick.
- **Metrics:** wall time per tick for `ThermalEvolutionSystem` (microseconds, before/after this
  tranche), transfer-receipt bytes per tick (material term adds a small, bounded per-cell increment),
  snapshot bytes per chunk, provenance event count growth per tick.
- **Method:** warm-up plus measured run per `docs/performance/benchmarks.md`.
- **Honesty note:** `docs/ontology/domain-coverage-matrix.md` already names unbounded thermal-receipt
  growth in `physical_state_digest` as a measured, open gap (`TODO-PERF-002`/`TODO-PERF-003`
  follow-ups from `plans/performance-baseline-and-digest-cost.md`). Adding a bounded material record to
  every participating cell's receipt makes that named-but-unresolved growth somewhat larger per tick;
  this plan does not claim to fix it, only to report the measured delta honestly rather than claim "no
  impact."

## Determinism impact

No new `System`, no new RNG stream, no new scheduler phase. All new arithmetic uses the same `i128`
checked-intermediate style as the existing face/reservoir path. Same-seed runs replay to identical
digests within a schema version. Zero-thermal, zero-material-flux configurations are unaffected
beyond the deliberate digest-schema bump.

## Memory impact

One `ThermalEnergy` (`i64`) plus one `Option<TraceId>` per material surface — negligible next to
per-cell thermal/mana state. Bounded transition history capped at `MAX_MATERIAL_SURFACE_TRANSITIONS`,
same cap already used for condition and gate transitions. Per-tick material transfer records are
bounded to at most one entry per surface, same shape as the existing bounded reservoir/face records.

## Observer impact

Adds one new bounded delta list (`MaterialSurfaceThermalDelta`, capped at 64 entries, oldest evicted
first) to `ObserverWorldSnapshot`. No new UI panel.

## Explanation impact

Adds Explanation schema 17, scoped by `MaterialSurfaceId`, reporting a numeric retained-energy range
with trace evidence. No semantic interpretation, temperature figure, or damage/phase label.

## Persistence impact

Material-surface section bumps to major 3; thermal section bumps to major 2; digest schema bumps.
Old snapshots fail closed. Save/resume includes all new authoritative state.

## Cross-domain effects

Purely local: a material surface only exchanges with its own co-located thermal cell. No mana,
biology, climate, or cross-chart effect. A later tranche may let a material's retained heat feed a
mana sample or a biological thermoregulation input — both remain out of scope here and are tracked as
named follow-ups.

## Risks

| Risk | Mitigation |
|------|------------|
| Invented energy from a one-directional derived accumulator | Rejected during design (see Context); this plan is bidirectional and conserved. |
| Cell driven negative by seven simultaneous outflows | Widened coefficient bound `6 * transfer_fraction + material_exchange_fraction <= scale`, enforced by `ThermalParameters::validate`. |
| Material driven negative | Proven algebraically that the cooling-direction candidate flux never exceeds `retained_before`; defensive bound check retained. |
| Material exceeding its capacity | Explicit headroom cap on the heating direction only, with the rejected remainder staying in the cell (no energy loss). |
| Conservation violation | Third bucket folded into the existing `i128` residual-must-be-zero check and atomic batch abort. |
| Renumbering existing RNG streams or system IDs | No new `System`/`register_system` call; verified against `Scheduler::register_system`'s registration-order key before implementation. |
| Snapshot format collision or silent version skew | New major versions for both affected sections; `import_thermal_snapshot`'s existing flux-equation check extended rather than bypassed. |
| Reintroducing the forbidden `condition +1` shortcut | New dedicated property (`MATERIAL_SURFACE_THERMAL_RETAINED_PROPERTY`) and event kind; `condition` is untouched by this tranche. |
| Scope creep into expansion/damage/phase-change/mana coupling | Explicit Non-goals list and named follow-up TODOs. |

## Documentation changes

- `docs/ontology/causal-carriers.md`: add material retained thermal energy as a physical carrier.
- `docs/ontology/domain-coverage-matrix.md`: update the Energy and Matter rows.
- `CHANGELOG.md`: new conserved material-thermal exchange, digest schema bump, two new snapshot
  section majors.
- `docs/development/todo-backlog.md`: close `TODO-THERMAL-002`; add follow-up TODOs for expansion/
  damage/phase-change response and heterogeneous per-material thermal properties.
- `docs/roadmap/roadmap.md`: update if it references this tranche's status.
- `PLANS.md`: move this plan from Active Planning to its completed-and-implemented entry once done.

## TODO changes

- Close `TODO-THERMAL-002`.
- Add a new TODO for expansion/damage-accumulation/phase-change material response (explicitly
  deferred, Non-goal 3).
- Add a new TODO for heterogeneous per-material thermal properties sourced from
  `causafera_types::Material` (Non-goal 2).
- Do not modify `TODO-THERMAL-003`/`004`/`005` scope; they remain independently deferred.

## Decision log

- 2026-07-28: Advisor review selected bidirectional conserved exchange over (a) a one-directional
  derived accumulator and (b) a dimensionless "dose" gate reusing the mana-coupling shape. Both
  alternatives were rejected: (a) invents energy relative to the carrier tranche's own conservation
  guarantee and the backlog's "exchange through conserved carriers" language; (b) is structurally the
  `MaterialSurfaceThermalGate` the carrier tranche already named and explicitly deferred as "no
  physically meaningful material response defined yet."
- 2026-07-28: Confirmed via `causafera-core/src/scheduler.rs` that RNG stream keys derive from
  registration order (`Scheduler::register_system`'s counter), not the `*_SYSTEM_ID` constants used
  for `EventProposalKey`. This makes the no-new-`System`, additive-`propose_evolution` integration
  mandatory for RNG stability rather than merely preferable, and confirmed it requires no
  `runtime_system_registrations()` change.
- 2026-07-28: Confirmed via `bootstrap.rs` that material surfaces are created only at historical
  bootstrap (single caller of `commit_material_surface_bootstrap_event`), one per active chunk at
  `cell_index = 0`, and that `state.active_chunks` and `thermal_active_region.active_chunks()` are
  built from the same `active_chunk_keys`, so every surface has a co-located resident thermal cell
  today. Superseded by the next entry: a material site with no matching field cell is treated as an
  internal invariant violation, not tolerated defensively.
- 2026-07-28 (Advisor correction): rejected the earlier "silent exclusion" plan for a material site
  with no co-located thermal cell. Since bootstrap guarantees the pairing today, a mismatch can only
  mean an internal invariant broke; silently excluding it would hide that break instead of surfacing
  it, and the repository's own guidance against defensive handling for scenarios that can't happen
  applies directly. `preflight_faces` now rejects with `ThermalError::PositionOutsideField` before any
  flux is computed (`crates/causafera-domains/src/thermal/diffusion.rs`).
- 2026-07-28: Rejected reusing the mana-coupling's "historical contact eligibility" policy
  (`contact_count > 0`) for thermal exchange: heat exchange with the environment does not depend on
  whether an actor has touched the surface, and gating it on contact would make the conserved total
  jump discontinuously at first contact — a semantic shortcut the mana plan itself flagged as
  "of this slice, not a universal future material-physics rule."
- 2026-07-28: Kept the new response as a separate `MaterialSurfaceThermalDelta`/event/property rather
  than extending `condition` or `gate`, per the acceptance criteria's explicit "no condition +1"
  instruction and because the thermal exchange and the mana gate are independent physical couplings
  with independent trigger conditions.
- 2026-07-28: `record_material_surface_thermal_transition`'s bounded history evicts with
  `Vec::remove(0)` (oldest-first), unlike `MaterialSurfaceGateTransition`'s recorder, which
  preferentially evicts an inactive/non-terminal entry to keep a rising-edge/falling-edge pair
  together. Thermal transitions have no such asymmetry to preserve — every exchange is equally
  inspectable regardless of direction — so plain oldest-first eviction is the correct, simpler rule
  here rather than an oversight relative to the gate recorder.
- 2026-07-28 (Advisor correction): rejected folding the new schema-17 claim into the existing
  `material_surface_loop_explanation`'s frame. That function requires a `MaterialSurfaceTransition`
  (condition/mana) to exist before it produces any report at all
  (`ok_or("missing material surface transition history")`), but thermal exchange is not gated on
  contact — a surface can be thermally active while never having been touched. Folding in would make
  such a surface's thermal state unexplainable, violating the "every accepted capability remains
  causally inspectable" rule. Implemented instead as a standalone
  `RuntimeState::material_surface_thermal_explanation` / `Runtime::observer_material_surface_thermal_explanation_for_surface`
  pair, modeled on the single-claim `thermal_conservation_explanation`, not on the multi-claim loop
  function. A surface ID absent from `material_surfaces` is rejected as `InvalidSnapshot` (a bad
  query); a real surface with no entry in the bounded transition history yields an `Unknown` claim
  carrying its current `retained_energy` (readable even when history has been evicted past the
  128-entry cap) — these are deliberately different outcomes, not collapsed into one.
- 2026-07-28: `MaterialSurfaceThermalDelta` and schema 17 are framed as "recent exchange evidence,"
  not "all exchanges ever" — the same 128-entry bounded `material_surface_thermal_transitions`
  history (oldest evicted first) backs both the 64-entry observer delta list and the Explanation
  query, so a surface that exchanged heat long enough ago can legitimately show zero evidence in
  either read model despite having a nonzero `retained_energy`. This is the same framing already
  implicit in the mana-gate's delta list and loop claim; nothing new is promised here.
- 2026-07-28: The new `MaterialSurfaceThermalDelta`/field 8 shares `material_surface_delta_schema_version`
  (bumped to a new V4) rather than getting its own dedicated version field the way `thermal_deltas`
  did. The version-field boundary follows the *addressed object type*, not "which subsystem produced
  the receipt": `MaterialSurfaceThermalDelta` is keyed by the same `MaterialSurfaceId` as
  `MaterialSurfaceDelta`/`MaterialSurfaceGateDelta` (which already share that field across the V1→V2→V3
  history), while `ThermalFieldDelta` addresses a different object (`ThermalCellKey`) and correctly
  keeps its own `thermal_delta_schema_version`.
- 2026-07-28: Explanation schema 17 was not added to `causafera-explanation/src/render.rs`'s
  localized `schema_name` table. That table is not exhaustive — schema 16
  (`THERMAL_CARRIER_CONSERVATION_SCHEMA`), its immediate predecessor, is not registered there either,
  and unregistered schemas fall back to a generic per-locale renderer rather than panicking. Adding
  localized names for every new schema is not required by this tranche's acceptance criteria and
  matches the precedent set by schema 16, not a new gap.
- 2026-07-28: Production `material_exchange_fraction = 64` (`RuntimeState::new`, out of
  `scale = THERMAL_SCALE = 1024`, alongside `transfer_fraction = 128`). Chosen well below the
  validated ceiling (`6 * 128 + material_exchange_fraction <= 1024` allows up to 256) so that a
  material surface's exchange is visibly slower than face-to-face diffusion per tick rather than
  dominating it — this is a foundation-phase default for observability of the new coupling, not a
  tuned physical constant; heterogeneous per-material rates are explicitly out of scope (see
  Non-goals) and follow-up TODOs.

## Progress

Accepted. Implementation authorized by this document. See CHANGELOG.md and the Progress notes appended
here as implementation lands.

- `3e8d81b` — plan accepted (this document, `PLANS.md` entry).
- `662952c` — Stage 1 (domain layer: `causafera-domains/src/thermal/*`, new
  `tests/thermal_material_coupling.rs`). Verified: `cargo test -p causafera-domains` green.
- `42a7ba8` — Stage 2 + Stage 3 (runtime layer: `MaterialSurfaceThermalState`, event kind 33/property
  23, production parameter defaults, digest schema bump to 6, section major bumps, persistence
  encode/decode, fail-closed validation including the extended signed-flux equation) plus the
  additional Wave-2 test coverage (V5/V7/V8/V10/V11, see Verification above). Verified: `cargo fmt
  --all -- --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`,
  `cargo test --workspace` under default features, `--all-features`, and `--no-default-features`, all
  green.
- `1447c83` — Stage 4 (observer/Explanation surface: `MaterialSurfaceThermalDelta` in
  `causafera-observer-api`, schema 17 in `causafera-explanation`, the
  `material_surface_thermal_explanation` / `observer_material_surface_thermal_explanation_for_surface`
  pair in `causafera-runtime`, wire encode/decode and field 8 in `causafera-observer-wire`,
  `query.proto`/`explanation.proto`, `packages/observer-protocol/src/index.ts`) landed together with
  V14's test coverage above. Verified: full workspace test suite
  (default/`--all-features`/`--no-default-features`) green, `cargo clippy`/`cargo fmt` clean,
  `pnpm --dir packages/observer-protocol typecheck` clean.
- Stage 5 (documentation): closed `TODO-THERMAL-002` in `docs/development/todo-backlog.md` and opened
  its two named follow-ups, `TODO-THERMAL-007` (expansion/damage/phase-change response) and
  `TODO-THERMAL-008` (heterogeneous per-material thermal properties). Updated `CHANGELOG.md`,
  `docs/ontology/causal-carriers.md` (third conserved bucket), `docs/ontology/domain-coverage-matrix.md`
  (Energy/Matter rows and the `TODO-PERF-002`/`003` growth note), `docs/rfc/RFC-PERSIST-001.md`
  (section-major and digest-schema history, including a stale pre-existing summary paragraph that had
  not been updated for the thermal carrier's own prior introduction), `docs/observer/protocol.md`, and
  `docs/observer/frontend-redesign-handoff.md`. `PLANS.md`'s entry was corrected to "accepted and
  implemented" in place — the file's own convention keeps completed Detailed Development plans listed
  under "Active Planning" rather than moving them to a separate section; only Foundation Era plans move
  to `plans/history/`. Per-tick performance cost was **not benchmarked** in this tranche; `CHANGELOG.md`
  and this plan both say so explicitly rather than omitting the claim.
- Full CI gate (Stage 5's V15) run and green: `cargo fmt --all -- --check`, `cargo clippy --workspace
  --all-targets --all-features -- -D warnings`, `cargo test --workspace` under default features,
  `--all-features`, and `--no-default-features`, `cargo run -p xtask -- ci`, `pnpm install
  --frozen-lockfile`, `pnpm lint`, `pnpm typecheck`, `pnpm build`, and
  `pnpm --dir packages/observer-protocol typecheck`.
- Remaining: the documentation checkpoint commit and the PR.
