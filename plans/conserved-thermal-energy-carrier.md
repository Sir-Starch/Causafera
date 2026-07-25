# Bounded Conserved Thermal Storage and Same-Chart Transfer ExecPlan

**Status:** Accepted — revision 6, final Oracle/Momus review passed

**Replaces:** the earlier exploratory draft of `plans/conserved-thermal-energy-carrier.md` (Gemini 3.1 Pro, 2026-07-22).

## Goal

Introduce a bounded, conserved, deterministic thermal-energy carrier that is authoritative simulation state. The carrier supports finite production-initialized reservoirs, fixed-point intra-chunk conduction, and same-chart cross-chunk face transfer with exact conservation accounting. It advances the **Energy** capability from M1 (numeric physical primitives) to M2 (executable conserved transfer/storage), while leaving thermal-to-material coupling, cross-chart transport, climate, and biology to later tranches.

## Context

The exploratory draft produced by Gemini 3.1 Pro proposed a fixed-point intra-chunk thermal field, explicit reservoirs, 6-neighbor diffusion, uniform heat capacity, and a separate `MaterialSurfaceThermalGate`. That draft was a useful starting point but assumed several things the repository does not yet support:

- It assumed the existing `Temperature` primitive was already fixed-point; it is currently `f64`.
- It proposed adding the thermal system to the same phase as mana evolution without analyzing scheduler registration-order and RNG-stream effects.
- It treated chunk boundaries as adiabatic, which conflicts with the invariant that chunk boundaries are not physical barriers.
- It proposed a material-condition effect (`condition +1`) without specifying a physically meaningful material response.

This ExecPlan rewrites the tranche from first principles. It keeps the thermal direction, replaces the premature abstractions, and narrows the scope to a complete, reviewable, causally inspectable carrier slice.

## Relevant invariants

- **INV-016** — Authoritative mutation is phase controlled.
- **INV-017** — Performance is architectural (data layout, cache locality, deterministic batch execution).
- **INV-019** — Emergence must be inspectable.
- **INV-038** — State digests are identities, not physical metrics.
- **INV-039** — Production state requires causal initialization.
- **INV-042** — Architecture remains modular and cohesive.
- **INV-043** — The world is one coherent spatial system. Missing transport is unimplemented, not intentionally impossible.

## Ontology domains affected

- **Energy** — M1 → M2 (conserved storage and same-chart transfer).
- **Physics** — adds fixed-point thermal-energy primitive; leaves existing `Temperature`/`Material` `f64` fields untouched.
- **Matter** — no runtime change; thermal-to-material coupling deferred.
- **Space** — reuses chart-qualified chunk addresses and same-chart neighbor adjacency.
- **Observer / Explanation** — adds bounded thermal read model and claim schema.

## Causal carriers affected

- New physical carrier: fixed-point thermal energy stored per cell and transferred across cell faces.
- New production carrier: finite `ThermalReservoir` initialized only through historical bootstrap, with traced injection into a specific cell.
- Existing carrier reused: same-chart chunk adjacency (pattern already established by mana boundary exchange).
- Deferred carriers: material-surface thermal gate, terrain-climate coupling, biological thermoregulation, mana-thermal interaction, experiment-recipe thermal source.

## Relevant documents

- `docs/vision/project-thesis.md`
- `docs/vision/uniqueness.md`
- `docs/architecture/invariants.md`
- `docs/architecture/detailed-development-rebaseline.md`
- `docs/architecture/determinism.md`
- `docs/architecture/performance.md`
- `docs/architecture/provenance.md`
- `docs/ontology/domain-coverage-matrix.md`
- `docs/ontology/causal-carriers.md`
- `docs/performance/benchmarks.md`
- `docs/world/terrain.md`
- `docs/world/climate.md`
- `docs/biology/physiology.md`
- `docs/explanation/architecture.md`
- `docs/explanation/explanation-ir.md`
- `docs/rfc/RFC-MANA-001.md`
- `docs/rfc/RFC-RES-001.md`
- `docs/rfc/RFC-GEO-002.md`
- `docs/rfc/RFC-TRACE-001.md`
- `plans/local-mana-material-surface-coupling.md`
- `plans/biological-mana-coupling.md`
- `plans/candidate-ledger.md`

## Current state

- `causafera-types/src/physics.rs` defines `Temperature` as `f64` and `Material::thermal_conductivity` / `specific_heat` as `f64`. These are not used by the runtime scheduler.
- `causafera-biology/src/physiology.rs` and `causafera-geography/src/climate.rs` contain `f32` temperature placeholders with no runtime loop.
- `causafera-domains/src/mana.rs` implements a fixed-point `ManaField` with `i64` intensities, `MANA_SCALE=1024`, 6-connected intra-chunk stencil, and same-chart cross-chunk boundary exchange (`apply_boundary_exchange`).
- `causafera-runtime/src/material_surface.rs` embeds a typed `MaterialSurfaceManaGate` with threshold/hysteresis transitions, separate gate-transition persistence, and strict causal validation.
- `causafera-runtime/src/runtime.rs` registers systems sequentially in `Runtime::new`; system IDs are assigned monotonically and feed RNG stream keys.
- `causafera-core/src/phases.rs` defines phases with fixed discriminants and a separate `Phase::ALL` execution order.
- `causafera-core/src/scheduler.rs` runs systems within a phase in registration order and assigns global system IDs in registration order.
- `causafera-core/src/provenance.rs` implements `CausalTraceStore::commit_batch`, which requires all event causes to pre-exist the batch.
- Snapshot sections are allocated through `crates/causafera-runtime/src/snapshot_sections.rs`. Section `0x000D` is already used for experiment-recipe mana-source receipts.

## Proposed architecture

### 1. Fixed-point state model

Authoritative conserved state is `ThermalEnergy`, a private `i64` newtype measured in a fixed-point unit (e.g., millijoules). The exact scale, maximum value, and minimum non-negative value are validated by `ThermalParameters`.

```rust
pub struct ThermalEnergy(i64);
```

- `Temperature` (`f64`) and `Material::thermal_conductivity` / `specific_heat` are **not migrated** in this tranche. They remain available for future domain work.
- `ThermalTemperature` is **not introduced** as an authoritative type. Temperature is derived as `Energy / HeatCapacity` only at observer/Explanation boundaries, not as simulation state.
- Each cell carries a fixed-point `heat_capacity` parameter in `ThermalParameters` (homogeneous for the bounded slice). This parameter is used only for observer/Explanation temperature derivation; the transfer algorithm operates directly on energy.

### 2. `ThermalField` and `ThermalFieldSet`

Modeled on `ManaField`/`ManaFieldSet` but simpler:

```rust
pub struct ThermalField {
    pub chunk: ChartChunkCoord,
    pub extent: u8,
    pub energy: Vec<ThermalEnergy>,
    pub last_change: Vec<TraceId>,
    pub last_change_before: Vec<ThermalEnergy>,
}

pub struct ThermalFieldSet {
    pub fields: BTreeMap<ChartChunkCoord, ThermalField>,
    pub batch_sequence: u64,
    pub conservation_last_change: TraceId,
}
```

- Dense row-major 3D grid up to `CHUNK_SIZE³`.
- Energy is always non-negative; `ThermalEnergy` rejects negative values.
- `last_change` is **never `None`** after bootstrap; every cell is initialized with a `Phase::Lifecycle` bootstrap/promotion trace.
- `last_change_before` mirrors the mana pattern for provenance.

### 3. Conservative six-face transfer algorithm

All transfer is computed from a single frozen thermal-step pre-state. The pre-state is the committed cell energy plus any pending reservoir injection accepted for this tick.

#### 3.1 Coefficient bound

`ThermalParameters` validates:

```text
0 < transfer_fraction <= floor(THERMAL_SCALE / 6)
```

This bound guarantees that a cell cannot be drained below zero by simultaneous outflow across all six faces in a single tick. If `THERMAL_SCALE` is not divisible by 6, the floor ensures the bound is integral.

#### 3.2 Face enumeration and endpoint ownership

Each undirected adjacent pair is processed exactly once.

1. For every cell, enumerate the six physical faces: `-X`, `+X`, `-Y`, `+Y`, `-Z`, `+Z`.
2. For each face, compute the neighbor's chart-qualified cell key.
3. Normalize the pair as `(min(endpoint_key), max(endpoint_key))` where the key is `(chart_id, chunk_x, chunk_y, chunk_z, cell_index)`.
4. Process the pair iff the current cell owns it (i.e., its key is the minimum). This guarantees each internal face and each cross-chunk face is visited exactly once.
5. Neighboring chunks must have compatible extents; mismatched extents reject the tick deterministically.

#### 3.3 Signed flux per face

For the owned pair between cells `A` and `B`:

```text
raw_diff = A.pre_state - B.pre_state
if raw_diff > 0:
    flux = floor(raw_diff * transfer_fraction / THERMAL_SCALE)  // A -> B, positive
elif raw_diff < 0:
    flux = -floor(-raw_diff * transfer_fraction / THERMAL_SCALE)  // B -> A, negative
else:
    flux = 0

A.post_state += -flux
B.post_state +=  flux
```

- `flux` is always an integer in fixed-point units.
- `floor` division is the only rounding operation; the undivided remainder stays in the source cell.
- The same signed `flux` is subtracted from the donor and added to the recipient, so energy is conserved exactly on every face.

#### 3.4 Accumulation algorithm and preflight

```text
Preflight:
  for each cell in active region:
    pre_state(cell) = committed_energy(cell) + pending_injection(cell)
    check pre_state(cell) in [0, ThermalEnergy::MAX]

  for each owned adjacent pair (A, B) in canonical order:
    compute flux as above using i128 intermediates
    check accumulator[A] and accumulator[B] remain in [0, ThermalEnergy::MAX]

Commit (only after preflight succeeds):
  for each owned adjacent pair (A, B) in canonical order:
    accumulator[A] -= flux
    accumulator[B] += flux

  for each cell in active region:
    new_energy(cell) = range_check(accumulator[cell])
```

- All subtraction, multiplication, accumulation, range checking, and conservation-residual computation use `i128` intermediates.
- The coefficient bound ensures the accumulator never goes negative and never exceeds `ThermalEnergy::MAX` when the pre-state is within bounds.
- Reservoir injection is capped so that the logical pre-state of any cell never exceeds `ThermalEnergy::MAX`.
- If any preflight check fails, the tick fails deterministically with `RuntimeError::ThermalArithmeticError` and no authoritative state changes.

### 4. Atomic causal commit model for multi-face transfers

For each cell whose net energy changes during a tick, `ThermalEvolutionSystem` commits exactly one `THERMAL_CELL_CHANGE_EVENT_KIND` event in `Phase::Physics`.

#### 4.1 Event granularity

- **One event per changed cell per tick.** A cell participating in multiple face transfers is represented by a single net before/after property effect.
- **No per-face events.** Per-face bookkeeping is internal to the proposal; only the net cell transition is persisted.
- **Reservoir budget reductions** are committed in the same batch as the cell-change events that consume the injections.

#### 4.2 Property effect

```text
CausalTarget:
  object_kind: THERMAL_CELL_OBJECT_KIND
  object_id:   cell_object_id(chunk, cell_index)
  property:    THERMAL_ENERGY_PROPERTY

before: fingerprint(energy_before)
after:  fingerprint(energy_after)
```

#### 4.3 Parent traces

The event's `causes` vector contains only pre-existing `TraceId`s, in ascending order:

1. The cell's own `last_change` trace from the prior tick.
2. The `last_change` trace of every neighbor cell that participated in a non-zero flux with this cell during this tick.
3. The most recent pre-existing trace (bootstrap or prior transfer) of every reservoir that contributed to the cell's logical pre-state this tick.

Duplicate trace IDs are removed before commit. A cell-change event **never** cites an event from the same batch as a parent, per `RFC-TRACE-001`. The prior tick's `thermal_field_set.conservation_last_change` is also not a parent of a cell-change event, because the conservation event audits the previous batch rather than contributing energy to the current cell pre-state.

#### 4.4 Bounded per-cell transfer receipt

Because `CausalTraceStore` effects carry fingerprints rather than domain values, a bounded `ThermalCellTransferReceipt` is persisted for every cell that participates in the tick (changed cell or reservoir target). It contains:

- the cell key (chart, chunk, index);
- the pre-state energy;
- the post-state energy;
- the committed cell-change trace ID, if a cell-change event was emitted;
- for each of up to six incident faces: neighbor cell key, signed flux, and neighbor pre-state;
- for each reservoir that targeted the cell this tick (whether accepted, partially accepted, or fully rejected):
  - `reservoir_id`;
  - `scheduled_injection`: the amount requested before headroom capping;
  - `accepted_injection`: the amount actually added to the cell's logical pre-state;
  - `rejected_injection`: `scheduled_injection - accepted_injection`;
  - `transfer_trace_id`: the same-batch reservoir transfer event trace ID, if `accepted_injection > 0`; `None` if the injection was fully rejected.

The receipt is part of the snapshot and is included in the digest. It is read-only with respect to authoritative state and is used by Explanation to reconstruct signed face contributions, link same-batch reservoir transfers, and account for rejected headroom.

A cell with zero net energy change and no reservoir targeting receives no receipt. A cell targeted by a reservoir but with zero net energy change receives a receipt and has its `last_change` updated to the reservoir transfer event (or left unchanged if the injection was fully rejected), but does not emit a cell-change event.

#### 4.5 `last_change` semantics

After the event commits:

- `cell.last_change` becomes the new event's `TraceId`.
- `cell.last_change_before` stores `energy_before`.
- A cell with zero net change **and no reservoir injection** emits no event; its `last_change` and `last_change_before` remain unchanged.
- A cell targeted by a reservoir but with zero net energy change emits no cell-change event (because `CausalEffect` rejects unchanged fingerprints). Its `last_change` is updated to the reservoir transfer event's `TraceId` if any injection was accepted; if the reservoir was fully rejected, `last_change` remains unchanged. Its `last_change_before` stores the cell's pre-state energy. The `ThermalCellTransferReceipt` records scheduled/accepted/rejected amounts and the transfer trace ID (if any). This preserves the reservoir-to-cell accounting for the next tick and for Explanation.

#### 4.9 Reservoir budget-reduction event

Every reservoir with a non-zero `accepted_injection` in a tick commits exactly one `THERMAL_RESERVOIR_TRANSFER_EVENT_KIND` event in the same batch as the cell-change and conservation events.

```text
CausalTarget:
  object_kind: THERMAL_RESERVOIR_OBJECT_KIND
  object_id:   reservoir_id
  property:    THERMAL_RESERVOIR_BUDGET_PROPERTY

before: fingerprint(budget_before)
after:  fingerprint(budget_after)
```

**Parent traces.** The reservoir transfer event's `causes` vector cites only pre-existing traces:

- For the first transfer from a reservoir, the `Phase::Lifecycle` `THERMAL_RESERVOIR_BOOTSTRAP_EVENT` trace of that reservoir.
- For subsequent transfers, the previous `THERMAL_RESERVOIR_TRANSFER_EVENT` trace of that reservoir.

This creates a single-parent chain per reservoir, which is sufficient because the event's `after` fingerprint is the new budget and the target cell records the injection in its transfer receipt.

#### 4.6 Canonical ordering

Events are proposed in canonical order:

1. Reservoir transfer events sorted by `ThermalReservoirId`.
2. Cell-change events sorted by `ChartChunkCoord` then row-major index.
3. Conservation event (one per thermal batch).

The batch is committed via `CausalTraceStore::commit_batch` with sorted `EventProposalKey`s.

#### 4.7 Bounded provenance growth

- Maximum cell-change events per tick = number of cells whose energy changes.
- Maximum reservoir transfer events per tick = number of reservoirs that inject.
- Each cell-change event references at most 1 + 6 + R parent traces (1 self + up to 6 neighbors + R reservoir traces, where R is the number of reservoirs contributing to the cell this tick).
- Transfer receipts are bounded to six faces plus R reservoir contributions per cell.
- Multiple reservoirs may target the same cell; combined injection is capped deterministically to the target cell's headroom.

#### 4.8 Conservation event and receipt

Every thermal batch commits exactly one `THERMAL_CONSERVATION_EVENT_KIND` event in `Phase::Physics`. The event is authoritative provenance, not just an Explanation claim.

`CausalEffect` requires `before != after`, so the conservation event cannot use unchanged total-cell-energy or total-reservoir-budget fingerprints. Instead, it performs one state-changing effect on the authoritative `batch_sequence` field in `ThermalFieldSet`:

```text
CausalEffect:
  target:
    object_kind: THERMAL_CARRIER_OBJECT_KIND
    object_id:   thermal_carrier_id(active_region_fingerprint)
    property:    THERMAL_BATCH_SEQUENCE_PROPERTY
  before: fingerprint(thermal_field_set.batch_sequence)
  after:  fingerprint(thermal_field_set.batch_sequence + 1)
```

After the batch commits, `thermal_field_set.batch_sequence` is incremented by one. This field is part of authoritative state, serialized in the snapshot, and included in digests.

The actual conservation accounting is stored in a `ThermalConservationReceipt` in the snapshot, keyed by the conservation event's `TraceId`:

- `tick`: the tick index;
- `total_cell_energy_before` / `total_cell_energy_after`;
- `total_reservoir_budget_before` / `total_reservoir_budget_after`;
- `residual`: the signed difference `(cells_after + reservoirs_after) - (cells_before + reservoirs_before)` in fixed-point units.

The residual must equal exactly zero. If it does not, the batch aborts with `RuntimeError::ThermalConservationViolation` and no state changes.

**Parent traces.** The conservation event's `causes` vector cites only pre-existing traces:

- For the first physics tick, the `Phase::Lifecycle` bootstrap events of all participating reservoirs and the bootstrap/promotion traces of all active thermal cells.
- For subsequent ticks, the previous tick's conservation event trace, plus any reservoir bootstrap traces for reservoirs that inject for the first time.

**Correlation to same-batch events.** The conservation event does not contain same-batch `TraceId`s in its `CausalEventProposal`; the `CausalEventProposal` type has no payload field and `TraceId`s are assigned only after the batch is sorted. Instead, correlation is established through the snapshot:

- The `ThermalCellTransferReceipt` for every participating cell in the tick is stored in the snapshot.
- The `ThermalConservationReceipt` for the tick is stored in the snapshot.
- The conservation event's `TraceId` is stored as `thermal_field_set.conservation_last_change`.
- Explanation reconstructs the tick by reading the conservation event, the conservation receipt, the cell-change events, the reservoir transfer events, and the transfer receipts from the same snapshot; the tick index links them.

**`last_change` semantics.** The conservation event's `TraceId` is stored as `thermal_field_set.conservation_last_change`. It becomes the parent for the next tick's conservation event and is included in the snapshot.

### 5. Finite reservoirs and atomic injection

Energy enters the system only through `ThermalReservoir` records initialized during historical bootstrap. Each reservoir has:

- a unique `ThermalReservoirId`;
- a chart-qualified target cell;
- an initial budget (fixed-point energy);
- a per-tick injection schedule or one-shot flag;
- a causal trace anchor (the bootstrap event).

#### 5.1 Production initialization path

The authoritative production path is **historical bootstrap**. A `ThermalReservoir` record is created by the bootstrap stage DAG with a `THERMAL_RESERVOIR_BOOTSTRAP_EVENT` in `Phase::Lifecycle`. The event records the reservoir ID, target cell, initial budget, and parent generation traces. No fixture or demo constructor may create a reservoir in production paths.

Experiment-recipe thermal sources are explicitly deferred to a later tranche.

#### 5.2 Reservoir system order and atomicity

- `ThermalReservoirSystem` runs first in `Phase::Physics`.
- It scans active reservoirs, validates that the target cell has headroom for the scheduled injection, and stores a `ThermalInjectionProposal` for each valid injection.
- It does **not** commit any events.
- `ThermalEvolutionSystem` runs second in `Phase::Physics`.
- It reads the pending proposals, applies them to the logical pre-state, computes diffusion, builds a complete after-state, runs preflight, and commits a single batch containing:
  - reservoir budget-reduction events (one per injecting reservoir), defined in Section 4.9;
  - cell-change events (one per cell with non-zero net energy change), defined in Section 4.1;
  - one conservation event, defined in Section 4.8.
- Cells with a non-zero reservoir injection but zero net energy change do not emit a cell-change event; their `last_change` is updated to the reservoir transfer event, and a `ThermalCellTransferReceipt` is stored.
- The batch is committed atomically via `CausalTraceStore::commit_batch`. After the batch succeeds, reservoir budgets, field energies, trace anchors, and transfer receipts are installed through infallible state updates.

This ordering means **newly injected energy propagates in the same tick**, and conservation is preserved transactionally.

#### 5.3 Conservation invariant across the batch

Before the batch commits, with all arithmetic in `i128`:

```text
sum(cells_after) + sum(reservoirs_after) == sum(cells_before) + sum(reservoirs_before)
```

The residual must equal exactly zero in fixed-point units. If it does not, the batch aborts with `RuntimeError::ThermalConservationViolation` and no state changes.

The residual and the before/after totals are recorded in the `ThermalConservationReceipt` described in Section 4.8, keyed by the `THERMAL_CONSERVATION_EVENT_KIND` event's `TraceId`. That event and receipt are committed in the same atomic batch as the reservoir transfers and cell changes, so authoritative provenance carries a per-tick conservation receipt.

#### 5.4 Reservoir target-cell chunk residency

A `ThermalReservoir` is bound to a single chart-qualified target cell at creation and never moves.

- **Bootstrap validation:** The target cell's chunk must be a member of the static active region. A reservoir whose target chunk is outside the active region is rejected during historical bootstrap with `BootstrapError::ThermalReservoirOutsideActiveRegion`.
- **Runtime residency check:** Before proposing an injection, `ThermalReservoirSystem` asserts that the target chunk is loaded. If the target chunk is active but not resident, the tick fails with `RuntimeError::ThermalRegionIncomplete` and no state changes.
- **No dynamic entry/exit:** Because the active region is static in this tranche, a valid reservoir never "exits" the active region or changes target chunk. Promotion/demotion of chunks and migration of reservoirs are deferred to a later tranche that defines conserved aggregate/escrow carriers.
- **Cross-chunk diffusion from target cell:** Energy injected into a target cell may diffuse across same-chart chunk boundaries according to Section 7. The reservoir itself remains associated with the target cell; only the energy carrier moves.

#### 5.5 Multiple reservoirs targeting the same cell

Multiple reservoirs may target the same cell. `ThermalReservoirSystem` resolves combined headroom deterministically:

1. Collect all pending injection proposals for the cell.
2. Sort proposals by `ThermalReservoirId` (ascending).
3. For each proposal in order:
   - `accepted = min(scheduled_injection, remaining_headroom)`;
   - `rejected = scheduled_injection - accepted`;
   - reduce the remaining headroom by `accepted`.
4. The logical pre-state of the cell includes the sum of all `accepted` injections.
5. Each reservoir's budget is reduced only by its accepted amount; the `ThermalCellTransferReceipt` for the target cell records `scheduled_injection`, `accepted_injection`, and `rejected_injection` for every reservoir that targeted the cell.

This rule is deterministic, preserves exact conservation, and avoids the ambiguity of a tick failure versus silent capping. Because `CausalEffect` rejects unchanged fingerprints, a reservoir whose `accepted_injection` is zero emits no `THERMAL_RESERVOIR_TRANSFER_EVENT` for this tick; its budget is unchanged, and the full rejection is recorded in the target cell's transfer receipt (`rejected_injection == scheduled_injection`, `transfer_trace_id == None`).

### 6. Scheduler integration and system order

#### 6.1 Phase::Physics execution order

Within `Phase::Physics`, systems execute in registration order. The canonical order for this tranche is:

1. `PhysicalPatternSystem` (existing; pattern-based physical processes).
2. Any other existing `Phase::Physics` systems registered before this tranche (preserved in existing order).
3. `ThermalReservoirSystem` (new; proposes reservoir injections only).
4. `ThermalEvolutionSystem` (new; computes diffusion, runs preflight, commits thermal batch).

This ordering means:

- Thermal evolution observes the committed state of all preceding `Phase::Physics` systems for this tick.
- No preceding system observes thermal changes from the same tick.
- Reservoir injection is proposed immediately before thermal evolution consumes it.

#### 6.2 Non-participating systems

The following systems are **explicitly non-participating** in this tranche. They produce no thermal proposals, consume no thermal state, and must not mutate `ThermalFieldSet` or `ThermalReservoir` records:

- **Terrain systems** — `TerrainChunk` remains static after bootstrap; no runtime terrain mutation reads or writes thermal energy.
- **Climate/weather systems** — the existing `f32`/`f64` temperature placeholders in `causafera-geography/src/climate.rs` are not runtime systems in this tranche and do not exchange energy with `ThermalFieldSet`.
- **Season systems** — no season driver runs in `Phase::Physics`; any future seasonal driver is deferred until it can exchange a conserved carrier with thermal state.
- **Biology systems** — biological thermoregulation and body-temperature coupling are deferred.
- **Mana systems** — no thermal influence on mana fields in this tranche.

Deferred systems are listed in Non-goals and tracked as TODOs.

#### 6.3 Registration mechanics

- Append both `ThermalReservoirSystem` and `ThermalEvolutionSystem` to the end of all existing registrations in `Runtime::new`, while assigning them to `Phase::Physics`.
- No new phase is introduced. Existing phase discriminants and `Phase::ALL` order remain unchanged.
- Because system IDs are assigned globally in registration order, appending at the end preserves existing IDs 0–8 and their RNG streams.
- The two new systems receive global IDs 9 and 10; they execute in `Phase::Physics` in that order.

```rust
// existing registrations 0..8 remain unchanged
scheduler.register_system(Phase::Physics, Box::new(PhysicalPatternSystem::new(...)));
scheduler.register_system(Phase::Mana, Box::new(ExperimentRecipeManaSourceSystem::new(...)));
// ...
scheduler.register_system(Phase::Lifecycle, Box::new(PopulationLifecycleSystem::new(...)));

// new thermal systems appended
scheduler.register_system(Phase::Physics, Box::new(ThermalReservoirSystem::new(...)));
scheduler.register_system(Phase::Physics, Box::new(ThermalEvolutionSystem::new(...)));
```

Registration metadata in `runtime_system_registrations` is updated to include the two new systems with global orders 9 and 10, phase `Physics`, and appropriate schema revisions.

### 7. Same-chart boundary behavior and active region

The active region is an authoritative, static set of chart-qualified chunks defined at bootstrap. It is not affected by runtime loading or materialization.

For each cross-chunk face in the active region:

| Neighbor state | Behavior |
|---|---|
| **Active and resident** | Compute flux using neighbor's committed cell energy. |
| **Active but not resident** | The entire tick fails with `RuntimeError::ThermalRegionIncomplete`; no authoritative state changes. |
| **Outside active region** | No flux; record as unrepresented transport in the boundary record. |

#### 7.1 Static active region

- The active region is set at historical bootstrap and does not change during this tranche.
- Demotion/promotion of chunks is deferred to a later tranche that defines conserved aggregate/escrow carriers.
- Snapshot validation rejects any thermal field whose active-region neighbors are missing from the snapshot.

#### 7.2 Runtime loading guarantee

- Before each tick, the runtime asserts that all active-region chunks are loaded.
- If any active chunk is not loaded, the tick fails deterministically with `RuntimeError::ThermalRegionIncomplete`.
- This prevents transient loading state from altering authoritative physics.

### 8. Persistence

- New snapshot section `THERMAL_SECTION_ID = 0x000E` (major version 1). `0x000D` is already allocated.
- Serialize `ThermalFieldSet` (including `batch_sequence` and `conservation_last_change`), `ThermalReservoir` set, active region set, boundary records, parameter fingerprint, transfer receipts, and conservation receipts.
- Transfer receipts are serialized for every participating cell in the tick; conservation receipts are serialized for every tick in which a thermal batch ran. Both are part of the snapshot and contribute to the digest.
- Import rejects unknown versions, malformed extents, duplicate chunks, negative energy, mismatched trace anchors, active-region gaps, reservoir budgets that do not balance against committed transfer events, conservation receipts with a non-zero residual, and mismatched `batch_sequence` values.

### 9. Observer and Explanation

- Add `ThermalEnergySummary` to `ObserverSnapshot` (total cell energy, total reservoir budget, active chunk count, active cell count).
- Add `ThermalFieldDelta` to `ObserverWorldSnapshot` for bounded transitions (reservoir injection, cell energy change above a threshold). Cap at 64 entries.
- Add Explanation schema `THERMAL_CARRIER_CONSERVATION_SCHEMA` reporting:
  - observation window;
  - total cell energy before/after;
  - total reservoir budget before/after;
  - conservation residual (exactly zero in fixed-point units), derived from the authoritative `ThermalConservationReceipt`;
  - reservoir transfer trace support;
  - neighbor transfer trace support;
  - fixed-point scale note.
- The observer/Explanation residual is not a separate computation; it reads the residual recorded in the most recent conservation receipt.
- Do not expose dense per-cell energy in routine queries; add a scoped cell-query only if required for validation.

## Primitive vs emergent review

**Primitive:**
- Fixed-point thermal energy per cell.
- Transfer fraction.
- Reservoir budget and injection schedule.
- Cell-level last-change trace.
- Active-region boundary record.
- Per-cell transfer receipt.

**Emergent / observer-only:**
- Temperature gradients (derived from energy and heat capacity).
- Equilibrium states.
- "Hot", "cold", "warming" concepts (agent/observer classifications).
- Any material damage, phase, or biological response.

## Non-goals

1. No thermal-to-material gate or material condition change in this tranche.
2. No cross-chart thermal transport.
3. No climate simulation or mutable terrain state.
4. No biological thermoregulation or body-temperature coupling.
5. No thermal influence on mana fields.
6. No convection, radiation, or fluid dynamics.
7. No phase changes (melting, boiling, ignition).
8. No migration of the existing `f64` `Temperature` type.
9. No new scheduler phase.
10. No experiment-recipe thermal source.
11. No dynamic active-region demotion/promotion.
12. No UI milestone; only bounded read-model support.

## Implementation stages

### Stage 1 — Domain types and parameters

Files:
- `crates/causafera-types/src/physics.rs` — add `ThermalEnergy` newtype.
- `crates/causafera-domains/src/thermal.rs` — new module with `ThermalParameters`, `ThermalField`, `ThermalFieldSet`, `ThermalReservoir`, `ThermalReservoirId`, `ThermalCellChange`, `ThermalCellTransferReceipt`, `ThermalInjectionProposal`, and conservative transfer proposals.
- `crates/causafera-domains/src/lib.rs` — expose thermal module.

Acceptance:
- `cargo check -p causafera-domains` passes.
- Unit tests prove `ThermalEnergy` rejects negative values and validates parameter bounds.

### Stage 2 — Runtime system and scheduler integration

Files:
- `crates/causafera-runtime/src/thermal.rs` — new module with `ThermalReservoirSystem` and `ThermalEvolutionSystem`.
- `crates/causafera-runtime/src/runtime.rs` — append both systems in `Phase::Physics` after all existing registrations; update `runtime_system_registrations`.

Acceptance:
- `cargo check -p causafera-runtime` passes.
- A zero-source zero-gradient run leaves existing subsystem digests unchanged except for the deliberate digest schema bump.
- Existing system IDs remain 0–8; new thermal systems receive IDs 9 and 10.

### Stage 3 — Persistence and snapshot compatibility

Files:
- `crates/causafera-runtime/src/snapshot_sections.rs` — add `THERMAL_SECTION_ID = 0x000E` encoder/decoder.
- `crates/causafera-runtime/src/snapshots.rs` — include thermal state, active region, boundary records, and transfer receipts in `RuntimeSnapshotData`.
- `crates/causafera-runtime/src/runtime.rs` — bump digest schema version, include thermal state in digests.

Acceptance:
- Snapshot round-trip tests pass.
- Malformed thermal sections reject deterministically.

### Stage 4 — Historical bootstrap reservoir

Files:
- Historical bootstrap infrastructure — finite reservoir initialization with provenance.
- Tests use `Runtime::new` and accepted historical bootstrap; no fixture/demo constructors in production paths.

Acceptance:
- A reservoir created through bootstrap commits a `Phase::Lifecycle` reservoir event.
- A `Phase::Physics` batch commits reservoir budget reduction and cell energy change together.
- Total system energy equals sum of reservoir budgets at initialization.

### Stage 5 — Observer and Explanation surface

Files:
- `crates/causafera-observer-api/src/query.rs` — add thermal summary fields and `ThermalFieldDelta`.
- `crates/causafera-explanation/src/ir.rs` — add `THERMAL_CARRIER_CONSERVATION_SCHEMA` claim.
- `crates/causafera-runtime/src/runtime.rs` — map authoritative thermal state and transfer receipts to observer/explanation structs.
- `proto/causafera/observer/v1/query.proto` and `explanation.proto` — add messages.
- `crates/causafera-observer-wire/src/protocol.rs` — encode/decode new fields.
- `packages/observer-protocol/src/index.ts` — TypeScript decoder.

Acceptance:
- Observer summary round-trips through Rust → protobuf → TypeScript.
- Explanation claim reports conservation residual exactly zero and trace support.

### Stage 6 — Verification, benchmarks, and documentation

Files:
- Add tests across `causafera-domains`, `causafera-runtime`, `causafera-observer-wire`, and `causafera-explanation`.
- Update `docs/ontology/domain-coverage-matrix.md`, `docs/ontology/causal-carriers.md`, `CHANGELOG.md`, and this plan.

## Verification

Each scenario below names the exact test surface, command, and expected observable.

### V1 — Hot cell with six cold neighbors
- **What:** A 3×3×3 chunk with energy `E` in the center cell and `0` elsewhere; run one tick.
- **Verify:** Center cell energy remains non-negative; total chunk energy equals `E` exactly; each neighbor receives a positive, equal flux (by symmetry); flux magnitude equals `floor(E * transfer_fraction / THERMAL_SCALE)`.
- **Test:** `crates/causafera-domains/tests/thermal_diffusion.rs`.
- **Command:** `cargo test -p causafera-domains hot_cell_six_cold_neighbors`
- **Expected:** PASS; center energy >= 0; sum == E; neighbor energy == flux for each of 6 neighbors.

### V2 — Multiple simultaneous face transfers touching the same cell
- **What:** A cell with different-energy neighbors on all six faces; run one tick.
- **Verify:** The cell's net change equals the signed sum of all six face fluxes; the cell never goes negative; energy is conserved across the whole chunk.
- **Test:** `crates/causafera-domains/tests/thermal_diffusion.rs`.
- **Command:** `cargo test -p causafera-domains simultaneous_six_face_transfers`
- **Expected:** PASS; net change == sum of face fluxes; residual == 0.

### V3 — Missing neighbor outside active region
- **What:** A thermal chunk with a same-chart neighbor that is outside the active region; run one tick.
- **Verify:** No flux across the boundary face; boundary record marks unrepresented transport; the chunk still evolves correctly across its loaded faces.
- **Test:** `crates/causafera-runtime/tests/thermal_boundaries.rs`.
- **Command:** `cargo test -p causafera-runtime --test thermal_boundaries outside_active_region`
- **Expected:** PASS; boundary record present; energy conserved among active cells.

### V4 — Missing active neighbor rejects tick
- **What:** A thermal chunk with a same-chart neighbor inside the active region but not loaded.
- **Verify:** The tick fails with `RuntimeError::ThermalRegionIncomplete`; no authoritative state changes; trace store, reservoir budgets, cells, and receipts are unchanged.
- **Test:** `crates/causafera-runtime/tests/thermal_boundaries.rs`.
- **Command:** `cargo test -p causafera-runtime --test thermal_boundaries active_neighbor_missing`
- **Expected:** PASS; error variant matches; no state changes.

### V5 — Reservoir exhaustion
- **What:** A reservoir with budget `B` and per-tick injection rate `r`; run until exhausted.
- **Verify:** The reservoir injects exactly `r` per tick while budget remains; the final injection is the remaining budget; after exhaustion, no further injection events occur; total cell energy equals the initial budget.
- **Test:** `crates/causafera-runtime/tests/thermal_reservoir.rs`.
- **Command:** `cargo test -p causafera-runtime --test thermal_reservoir exhaustion`
- **Expected:** PASS; cumulative injected == B; no injection after exhaustion.

### V6 — Exact cell-plus-reservoir conservation
- **What:** Multiple reservoirs and chunks with cross-chunk flux; run 1000 ticks.
- **Verify:** `sum(all cell energies) + sum(all reservoir budgets)` equals the initial total energy exactly.
- **Test:** `crates/causafera-runtime/tests/thermal_reservoir.rs`.
- **Command:** `cargo test -p causafera-runtime --test thermal_reservoir exact_global_conservation`
- **Expected:** PASS; residual == 0.

### V7 — Conservation residual checked every tick
- **What:** Same setup as V6; run 1000 ticks.
- **Verify:** The conservation residual is asserted after every tick, not only at the end.
- **Test:** `crates/causafera-runtime/tests/thermal_reservoir.rs`.
- **Command:** `cargo test -p causafera-runtime --test thermal_reservoir per_tick_residual_zero`
- **Expected:** PASS; no tick has a non-zero residual.

### V8 — Invalid parameter rejection
- **What:** `ThermalParameters` with `transfer_fraction > floor(THERMAL_SCALE / 6)`, negative heat capacity, or zero scale.
- **Verify:** Construction returns `ThermalError::InvalidParameters`; runtime never installs invalid parameters.
- **Test:** `crates/causafera-domains/tests/thermal_parameters.rs`.
- **Command:** `cargo test -p causafera-domains invalid_parameters_reject`
- **Expected:** PASS with expected error.

### V9 — Real arithmetic limits and accumulated cell deltas
- **What:** A cell near `ThermalEnergy::MAX` with small neighbors; a reservoir injecting near `ThermalEnergy::MAX`; run many ticks.
- **Verify:** Preflight checks reject any operation that would overflow `i128` or exceed `ThermalEnergy::MAX`; final energy is within bounds; conservation residual is zero.
- **Test:** `crates/causafera-domains/tests/thermal_diffusion.rs`.
- **Command:** `cargo test -p causafera-domains accumulated_delta_bounds`
- **Expected:** PASS; no overflow; residual == 0.

### V10 — Upper-bound protection: cell receiving from six hot neighbors
- **What:** A cell near `ThermalEnergy::MAX` receives inflow from six neighbors also near `ThermalEnergy::MAX`.
- **Verify:** The recipient cell never exceeds `ThermalEnergy::MAX`; preflight rejects any configuration that would overflow.
- **Test:** `crates/causafera-domains/tests/thermal_diffusion.rs`.
- **Command:** `cargo test -p causafera-domains upper_bound_six_hot_neighbors`
- **Expected:** PASS; recipient <= MAX.

### V11 — Face processed exactly once regardless of construction order
- **What:** Build a thermal field set by inserting chunks in different orders; run one tick.
- **Verify:** Each cross-chunk face is processed exactly once; final energies are identical across orderings.
- **Test:** `crates/causafera-domains/tests/thermal_diffusion.rs`.
- **Command:** `cargo test -p causafera-domains canonical_face_order_invariant`
- **Expected:** PASS; energies identical.

### V12 — Frozen-state test: no cascade during same tick
- **What:** A three-cell chain A-B-C where A is hot and C is cold; run one tick.
- **Verify:** Flux from A to B and from B to C are both computed from the pre-tick committed state; B's received energy does not increase its outflow to C within the same tick.
- **Test:** `crates/causafera-domains/tests/thermal_diffusion.rs`.
- **Command:** `cargo test -p causafera-domains frozen_state_no_cascade`
- **Expected:** PASS; B's outflow to C equals floor((B_old - C_old) * k / S), not based on A→B flux.

### V13 — Same-tick reservoir propagation via receipt
- **What:** A reservoir injects into cell A; A has a neighbor B that is cooler than A after injection.
- **Verify:** On the injection tick, the flux from A to B is larger than it would have been without injection; the `ThermalCellTransferReceipt` for A records `scheduled_injection`, `accepted_injection`, and `rejected_injection` and the same-batch reservoir transfer trace ID; A's cell-change event (or `last_change` update, if net-zero) links the reservoir transfer; B's cell-change event cites A's pre-existing `last_change` trace only (no same-batch parent edge), because B is not directly injected by the reservoir.
- **Test:** `crates/causafera-runtime/tests/thermal_reservoir.rs`.
- **Command:** `cargo test -p causafera-runtime --test thermal_reservoir same_tick_propagation`
- **Expected:** PASS; flux increases; receipt links reservoir transfer; no same-batch parent edge from B to reservoir.

### V14 — Net-zero target with reservoir injection
- **What:** A reservoir injects into cell A, and A simultaneously loses the same amount to neighbors; A's net energy is unchanged.
- **Verify:** A emits no cell-change event because `CausalEffect` rejects unchanged fingerprints; A's `last_change` is updated to the reservoir transfer event's `TraceId`; a `ThermalCellTransferReceipt` is stored for A, recording `scheduled_injection`, `accepted_injection`, and `rejected_injection`; recipients change and cite A's pre-existing `last_change`.
- **Test:** `crates/causafera-runtime/tests/thermal_reservoir.rs`.
- **Command:** `cargo test -p causafera-runtime --test thermal_reservoir net_zero_target`
- **Expected:** PASS; A event absent; A.last_change == reservoir transfer trace; receipt present; recipients change.

### V15 — Atomic batch failure rolls back all state
- **What:** Force a causal-capacity rejection or conservation mismatch during the thermal batch.
- **Verify:** Trace count, reservoir budgets, cell energies, anchors, and receipts are all unchanged after the failure.
- **Test:** `crates/causafera-runtime/tests/thermal_reservoir.rs`.
- **Command:** `cargo test -p causafera-runtime --test thermal_reservoir atomic_failure_rollback`
- **Expected:** PASS; state unchanged.

### V16 — Combined headroom for same-target reservoirs
- **What:** Two reservoirs target the same cell; their combined injection would exceed `ThermalEnergy::MAX`.
- **Verify:** The cell's headroom is filled deterministically. Proposals are sorted by `ThermalReservoirId`; each reservoir's `accepted_injection` is the minimum of its `scheduled_injection` and the remaining headroom. `rejected_injection` is recorded in the target cell's `ThermalCellTransferReceipt` for each reservoir. The reservoir with the lower ID consumes headroom first; the higher ID receives the remainder or zero. Budgets are reduced only by `accepted_injection`. The tick succeeds with a zero conservation residual.
- **Test:** `crates/causafera-runtime/tests/thermal_reservoir.rs`.
- **Command:** `cargo test -p causafera-runtime --test thermal_reservoir same_target_headroom`
- **Expected:** PASS; sum of accepted injections == headroom; `rejected_injection` fields account for the excess; residual == 0.

### V17 — Transfer receipt reconstructs face fluxes
- **What:** A multi-face transfer with asymmetric neighbors.
- **Verify:** The `ThermalCellTransferReceipt` for the changed cell contains the correct signed flux and neighbor key for each face.
- **Test:** `crates/causafera-runtime/tests/thermal_receipts.rs`.
- **Command:** `cargo test -p causafera-runtime --test thermal_receipts face_reconstruct`
- **Expected:** PASS; receipt matches computed fluxes.

### V18 — Malformed snapshot rejection
- **What:** Tamper with thermal section bytes (negative energy, bad extent, duplicate chunk, active-region gap, mismatched reservoir budget).
- **Verify:** `import_snapshot` rejects before installing state.
- **Test:** `crates/causafera-runtime/tests/thermal_persistence.rs`.
- **Command:** `cargo test -p causafera-runtime --test thermal_persistence malformed_rejects`
- **Expected:** PASS with `RuntimeError::InvalidSnapshot`.

### V19 — Save/resume equivalence
- **What:** Run 50 ticks, save snapshot, resume, run 50 more; compare to a continuous 100-tick run.
- **Verify:** Canonical physical and history digests are equal; thermal state equal.
- **Test:** `crates/causafera-runtime/tests/thermal_persistence.rs`.
- **Command:** `cargo test -p causafera-runtime --test thermal_persistence save_resume_equivalence`
- **Expected:** PASS; digests match.

### V20 — Production bootstrap provenance
- **What:** Initialize a runtime with a historical bootstrap reservoir.
- **Verify:** Reservoir bootstrap event is `Phase::Lifecycle`; first physics batch includes reservoir budget reduction and cell energy change; total energy budget balances.
- **Test:** `crates/causafera-runtime/tests/thermal_bootstrap.rs`.
- **Command:** `cargo test -p causafera-runtime --test thermal_bootstrap`
- **Expected:** PASS; trace ancestry valid.

### V21 — Legacy system IDs and RNG stability
- **What:** Register thermal systems after all existing systems.
- **Verify:** Existing system IDs remain 0–8; new systems are 9 and 10; zero-thermal runs produce identical legacy subsystem traces as before.
- **Test:** `crates/causafera-runtime/tests/thermal_determinism.rs`.
- **Command:** `cargo test -p causafera-runtime --test thermal_determinism legacy_ids_stable`
- **Expected:** PASS; IDs and traces match.

### V22 — Observer/Explanation round-trip
- **What:** Run a reservoir injection; query observer summary and explanation.
- **Verify:** TypeScript decoder parses thermal summary; Explanation claim reports residual `0` and cites reservoir/neighbor trace support.
- **Test:** `crates/causafera-observer-wire` round-trip test + `causafera-explanation` claim test.
- **Command:**
  ```
  cargo test -p causafera-observer-wire --test protocol thermal_roundtrip
  cargo test -p causafera-explanation thermal_conservation_claim
  ```
- **Expected:** PASS.

### V23 — Full CI gate
- **Command:**
  ```
  cargo fmt --all -- --check
  cargo clippy --workspace --all-targets --all-features -- -D warnings
  cargo test --workspace --all-features
  cargo test --workspace --no-default-features
  cargo run -p xtask -- ci
  ```
- **Expected:** All green.

## Benchmark plan

- **Workload:** One fully populated `CHUNK_SIZE³` thermal field plus one same-chart neighbor chunk, with a center-cell reservoir injection.
- **Metrics:**
  - Wall time per tick for `ThermalEvolutionSystem` (microseconds).
  - Cells updated per tick.
  - Peak memory for thermal working buffers.
  - Snapshot bytes per chunk.
  - Provenance event count growth per tick.
  - Observer query payload bytes for thermal summary.
- **Method:** Warm-up plus measured run as specified in `docs/performance/benchmarks.md`.
- **Target:** No absolute latency claim without hardware baseline. Reported numbers become the baseline for future thermal work.

## Determinism impact

- No new phase; existing phase discriminants and execution order unchanged.
- New systems appended after all existing registrations; existing system IDs 0–8 and RNG streams stable.
- New thermal systems receive global IDs 9 and 10; registration metadata is updated.
- `CURRENT_DIGEST_SCHEMA_VERSION` is incremented because authoritative state changes.
- Same inputs produce identical new digests.
- Zero-thermal configurations produce unchanged legacy subsystem outputs.

## Memory impact

- Adds one `i64` per thermal cell plus per-cell `TraceId` and `i64` last-change-before.
- For a dense `CHUNK_SIZE³` field, this is comparable to one mana field dimension.
- Working buffers for cross-chunk face flux are transient per tick.
- No unbounded caches; transfer receipts are bounded per cell and retained with cell-change events.

## Observer impact

- Adds bounded thermal summary to `ObserverSnapshot`.
- Adds bounded `ThermalFieldDelta` list to `ObserverWorldSnapshot` (capped at 64 entries).
- No new UI panel; data is available for validation and future UI milestones.

## Explanation impact

- Adds one claim schema for conservation accounting.
- Claim reports residual exactly zero in fixed-point units.
- Uses bounded per-cell transfer receipts to reconstruct signed face contributions.
- No human prose or semantic interpretation added.

## Persistence impact

- New required section `0x000E` major version 1.
- Digest schema version bump.
- Old snapshots fail closed on unknown section or incompatible digest schema.
- Save/resume includes full thermal state, active region, boundary records, reservoir budgets, and transfer receipts.

## Cross-domain effects

- **Mana:** None in this tranche. Future tranche may allow thermal patterns to feed mana samples.
- **Matter:** None in this tranche. Future tranche may add thermal exposure to material surfaces.
- **Space:** Reuses chart-qualified chunk addressing and same-chart adjacency.
- **Climate / Biology:** No runtime coupling. Their `f32`/`f64` placeholders remain untouched.

## Risks

| Risk | Mitigation |
|------|------------|
| Cell energy driven negative | Coefficient bound `transfer_fraction <= floor(THERMAL_SCALE / 6)`; `i128` bounds checks; reservoir injection capped by target headroom. |
| Conservation violation | Pairwise subtraction/addition of identical quantized flux; per-tick residual must equal exactly zero; atomic batch commit for reservoir and cell changes. |
| Cross-chunk ordering non-determinism | Lexicographic endpoint ownership; canonical cell/chunk ordering; boundary records. |
| Runtime loading state alters physics | Static active region; whole-tick failure if any active chunk not loaded. |
| Memory blow-up from dense 3D fields | Cap extent at `CHUNK_SIZE`; sparse representation deferred. |
| Observer/Explanation surface underestimated | Stage 5 is gated until domain logic is stable; no UI work. |
| Scope creep into material/climate coupling | Strict non-goals list and review gate. |
| Snapshot format collision | Use `0x000E`, not `0x000D`. |

## Documentation changes

- Update `docs/ontology/domain-coverage-matrix.md`: Energy M2 row; add thermal carrier row notes.
- Update `docs/ontology/causal-carriers.md`: list thermal energy as a physical carrier.
- Update `docs/performance/benchmarks.md`: add thermal benchmark baseline.
- Update `CHANGELOG.md`: new thermal carrier, digest schema bump, new snapshot section.
- Update `plans/candidate-ledger.md` with this tranche outcome and re-evaluation triggers.

## TODO changes

- Add `TODO-THERMAL-001`: Cross-chart thermal transport.
- Add `TODO-THERMAL-002`: Thermal-to-material coupling.
- Add `TODO-THERMAL-003`: Thermal influence on mana field.
- Add `TODO-THERMAL-004`: Climate/biology thermal integration.
- Add `TODO-THERMAL-005`: Experiment-recipe thermal source.
- Do not modify roadmap phase numbers; follow Detailed Development rebaseline.

## Decision log

- **2026-07-22:** Exploratory draft by Gemini 3.1 Pro reviewed and found to assume fixed-point types, new phase, and adiabatic boundaries that the repository does not support.
- **2026-07-22:** Decision to retain thermal direction but substantially redesign implementation.
- **2026-07-22:** Oracle review: keep separate `ThermalEnergy` carrier, run in `Phase::Physics`, include same-chart cross-chunk exchange, defer material gate, use section `0x000E`.
- **2026-07-22:** Momus review: rejected for lack of executable QA scenarios; added per-stage verification with exact commands and expected results.
- **2026-07-22:** Final tranche scope: bounded conserved thermal storage and same-chart transfer only; no material gate, no climate/biology coupling, no new phase.
- **2026-07-22:** User approved thermal direction and boundaries; requested resolution of remaining contracts before acceptance.
- **2026-07-22:** Contract resolutions:
  - Coefficient bound `transfer_fraction <= floor(THERMAL_SCALE / 6)` ensures non-negative energy under six-face outflow.
  - Lexicographic endpoint ownership processes each face exactly once.
  - Frozen post-injection thermal-step pre-state; exhaustive `i128` preflight before commit.
  - One event per changed cell per tick with bounded per-cell transfer receipt.
  - `ThermalReservoirSystem` prepares proposals; `ThermalEvolutionSystem` commits reservoir transfers and cell changes in one atomic batch; same-tick propagation.
  - Cell-change events cite only pre-existing traces; receipts correlate same-batch reservoir transfers.
  - Thermal systems appended after all existing registrations, preserving IDs 0–8.
  - Active region is static, authoritative, and loading-independent; missing active neighbor fails the whole tick.
  - Conservation residual must equal exactly zero in fixed-point units and is recorded in an authoritative `THERMAL_CONSERVATION_EVENT_KIND` event committed in the same batch.
  - `ThermalTemperature` not introduced; temperature derived only at observer/Explanation boundaries.
  - Historical bootstrap is the authoritative reservoir path; experiment-recipe source deferred.
  - Added atomic-failure, frozen-state, upper-bound, net-zero-target, combined-headroom, canonical-order, and legacy-ID tests.
- **2026-07-22:** Final review revisions (revision 3):
  - Added explicit `Phase::Physics` execution order and non-participating system list (terrain, climate/weather, season, biology, mana).
  - Added reservoir target-cell chunk residency rules: bootstrap validation, runtime residency check, no dynamic entry/exit.
  - Added authoritative conservation event with residual, before/after totals, and correlation references to same-batch cell-change and reservoir-transfer trace IDs.
  - Updated persistence and observer/Explanation sections to use the authoritative conservation event.
- **2026-07-23:** Oracle architecture review (revision 4) found three blockers and fixed them:
  - Conservation event no longer claims to store same-batch `TraceId`s in its `CausalEventProposal`; it uses a single `CausalEffect` on a batch sequence property and stores the residual/totals in a `ThermalConservationReceipt` in the snapshot.
  - Net-zero target cells with a non-zero reservoir injection no longer emit a cell-change event (because `CausalEffect` rejects unchanged fingerprints); instead, their `last_change` is updated to the reservoir transfer event and a transfer receipt is stored.
  - Reservoir budget-reduction event is fully defined with `CausalTarget`, `before`/`after` fingerprints, and parent-trace rules.
- **2026-07-23:** Oracle architecture review (revision 5) found four additional blockers and fixed them:
  - Defined `ThermalConservationReceipt` as the authoritative residual record and used a `THERMAL_BATCH_SEQUENCE_PROPERTY` effect to satisfy the `CausalEffect` invariant.
  - Allowed multiple reservoirs per cell with deterministic headroom capping by reservoir ID.
  - Clarified that neighbor cells cite only the target cell's pre-existing `last_change`, not the reservoir's trace directly.
  - Specified that rejected reservoir injections emit no transfer event and are recorded only in the target cell's transfer receipt.
- **2026-07-23:** Oracle architecture review (revision 6) found two representability gaps and fixed them:
  - Added authoritative `batch_sequence` and `conservation_last_change` fields to `ThermalFieldSet`, with persistence and digest inclusion.
  - Extended `ThermalCellTransferReceipt` to record `scheduled_injection`, `accepted_injection`, `rejected_injection`, and optional `transfer_trace_id` for every reservoir targeting a cell.
- **2026-07-23:** Implementation pre-flight: resolved a drafting inconsistency between Section 4.3 and Section 4.7. The explicit Section 4.3 parent-trace list is authoritative; cell-change events do not cite the prior conservation event as a parent. Section 4.7 corrected from `2 + 6 + R` to `1 + 6 + R`, and Section 4.3 now explicitly excludes `conservation_last_change` as a causal parent.
- **2026-07-25:** Independent review found two completion blockers: V3 did not retain outside-active-region boundary records, and V15 covered domain preflight failure rather than a real runtime causal-batch rejection. Both blockers were reopened for test-first correction.
- **2026-07-25:** A writing subagent discarded uncommitted runtime integration with worktree-destructive Git commands. Exact recovery used OpenCode snapshot tree `948399d8695ea58ce2263ab23e7b82d87bdfdbfa` and an explicit ten-file blob allowlist; no unrelated dirty path was replaced. `AGENTS.md` and `PLANS.md` now forbid worktree-discarding Git during implementation and require verified green-wave checkpoint commits.
- **2026-07-25:** Boundary records are current-committed-batch authoritative state. They use the frozen post-injection/pre-diffusion cell value, are strictly ordered by `(cell, neighbor)`, replace the prior batch atomically after successful causal commit, persist in required section `0x000E` major V1, and contribute once to physical digest V5.
- **2026-07-26:** Independent review found four completion blockers in `import_thermal_snapshot` and boundary reconstruction, and all four were reopened for test-first correction:
  - Imported transfer receipts were not checked against their signed face-flux equation, so coordinated receipt/boundary pre-state mutation was accepted. Fixed by validating `pre_state - sum(signed_flux) == post_state` with checked `i128` arithmetic before acceptance.
  - The latest batch's receipt post-state was not bound to current field energy, so a balanced but shifted receipt with matching boundary records was accepted. Fixed by requiring every receipt in `thermal_fields.conservation_last_change()`'s batch to match the indexed current-field cell energy.
  - Reservoir budget-difference subtraction used raw arithmetic, risking a debug-build panic or release wraparound on crafted extremes. Fixed with checked subtraction returning `InvalidSnapshot` on overflow.
  - `import_thermal_boundary_records` duplicated the domain's face/direction/wraparound geometry inline instead of reusing the authoritative implementation. Fixed by adding `ThermalFieldSet::boundary_neighbor_keys` in `causafera-domains/src/thermal/neighbor.rs` and calling it from the runtime, removing ~50 lines of duplicated logic.
  - Added regressions: `runtime_import_rejects_receipt_flux_transition_forgery`, `runtime_import_rejects_latest_receipt_post_state_mismatch`, `runtime_import_rejects_reservoir_budget_subtraction_overflow`, `thermal_persistence_literal_version_contract` (pins section `0x000E`, major V1, digest schema V5 as literal values), and domain test `boundary_neighbor_keys_returns_only_inactive_faces`.
- **2026-07-26:** Final 5-lane review of commit `3e46bc2` (goal/constraint, QA execution, code quality, security, context mining):
  - QA independently reproduced red-then-green evidence in an isolated worktree at the parent commit: all three forgery/mismatch/overflow regressions failed or panicked pre-fix and passed post-fix; confirmed non-vacuous.
  - Code quality: PASS, minor nits only (broad `Err(InvalidSnapshot(_))` matching instead of message-specific assertions; the geometry test's `extent = 1` case does not exercise cross-chunk flat-index wrapping).
  - Context mining: PASS; confirmed no other duplicated copy of the removed inline neighbor-geometry pattern remains, confirmed `boundary_neighbor_keys` is genuinely reachable as public API from `causafera-runtime`, and flagged that `CHANGELOG.md`/`todo-backlog.md`/`domain-coverage-matrix.md` were not touched by `3e46bc2` (a documentation-process gap, not a factual error) — resolved by this same day's documentation update.
  - Security: FAIL, HIGH, four findings. Independent verification (reading the surrounding `import_thermal_snapshot` body, `SECURITY.md`, and `causafera-types::ChartChunkCoord::same_chart_neighbor`) found two findings out of scope of this remediation (a claimed `batch_sequence` "downgrade" bypass that does not correspond to any real function in the codebase; and an unchecked-`i32`-arithmetic panic risk in `same_chart_neighbor` that is pre-existing and was called identically by the removed inline code, not introduced or worsened here) and two real, narrower gaps that exceed the four originally-scoped blockers: (a) non-latest-batch transfer receipts did not bounds-check `cell.cell_index` against the field's cell count, and (b) `ThermalConservationReceipt`'s aggregate `total_cell_energy_after`/`total_reservoir_budget_after` fields are never cross-validated against a recomputed sum of actual field/reservoir state (only per-cell state of latest-batch *touched* cells is bound).
  - User decision: fix (a) now as a minimal, same-pattern addition; defer (b) as `TODO-THERMAL-006` in `docs/development/todo-backlog.md`, since it is a larger design question (full-field summation cost on every import) requiring its own scoping rather than a bugfix-minimal change.
  - Fixed (a): `import_thermal_snapshot`'s transfer-receipt loop now looks up the receipt's field once and rejects any `cell_index` at or beyond `field.energy().len()`, for every batch, not only the latest. Added regression `runtime_import_rejects_non_latest_receipt_cell_index_out_of_bounds`, which runs two ticks to produce a non-latest conservation batch, corrupts that batch's transfer-receipt cell index to the field volume, and asserts rejection.
  - Re-verified after this follow-up fix: `cargo fmt --check`, `cargo clippy --workspace --all-targets --all-features -D warnings`, `cargo test -p causafera-runtime --test thermal_persistence` (11/11), `cargo test --workspace` (both feature sets), and `cargo run -p xtask -- ci` all pass.

## Progress

- Planning and contract resolution complete.
- Implementation complete on branch `feat/thermal-energy-carrier` from baseline `e0bf3c9`.
- Files added: `crates/causafera-types/src/physics.rs` (ThermalEnergy), `crates/causafera-domains/src/thermal/` (domain module), `crates/causafera-runtime/src/thermal.rs`, `crates/causafera-runtime/src/thermal_events.rs`, and thermal integration tests.
- Files modified: `crates/causafera-runtime/src/runtime.rs`, `crates/causafera-runtime/src/bootstrap.rs`, `crates/causafera-runtime/src/snapshot_sections.rs`, `crates/causafera-runtime/src/snapshots.rs`, `crates/causafera-observer-api/src/query.rs`, `crates/causafera-observer-wire/src/protocol.rs`, `crates/causafera-explanation/src/ir.rs`, `packages/observer-protocol/src/index.ts`, `proto/causafera/observer/v1/query.proto`, `proto/causafera/observer/v1/explanation.proto`, and documentation.
- Verification results:
  - `cargo fmt --all -- --check` passed.
  - `cargo clippy --workspace --all-targets --all-features -- -D warnings` passed.
  - `cargo test --workspace --all-features` passed.
  - `cargo test --workspace --no-default-features` passed.
  - `cargo run -p xtask -- ci` passed.
  - `pnpm install --frozen-lockfile`, `pnpm lint`, `pnpm typecheck`, and `pnpm build` passed.
  - `pnpm --dir packages/observer-protocol typecheck` passed.
  - Audit entry-point and link validators passed. `node tools/audit/run-source-tests.mjs` could not start because the locally installed `omo` 4.19.1 executable does not include the repository-audited `packages/lsp-daemon/dist/cli.js` provider layout; this is an environment/tool-distribution prerequisite, not a thermal test failure.
- Causality clarification: Section 4.3 parent list is authoritative; Section 4.7 corrected from `2 + 6 + R` to `1 + 6 + R`; cell-change events do not cite the prior conservation event as a parent.
- INV-037 compliance: thermal energy flows across same-chart chunk boundaries; boundaries are not physical barriers. The existing mana chunk-boundary seam is tracked as `TODO-MANA-002` and is not modified in this tranche.
- Post-review V3 evidence: `outside_active_region_boundary_record` passes with no inactive-face flux, an exact same-chart boundary record, loaded-face evolution, and zero residual.
- Post-review V15 evidence: `thermal::tests::atomic_failure_rollback` forces the real `CausalCommitError::UnknownCause` path and verifies unchanged trace store, fields and anchors, reservoirs and budgets, active region, parameters, transfer and conservation receipts, pending injections, boundary records, failure slot, and thermal-system time.
- Post-review persistence evidence: six thermal persistence tests pass, including non-empty canonical boundary round-trip, malformed scalar/order/topology/pre-state rejection, and continuous/save-resume equality.
- Manual external-driver evidence: a two-chunk run produced 54 domain boundary records and 126 runtime records, exact zero conservation residual, canonical thermal envelope bytes, and equal continuous/save-resume physical digests.
- Local checkpoint history:
  - Process safety: `acb9a50`, `1775f31`.
  - Domain and types: `f219fcf`, `9c0cb1e`, `4c7275c`, `0900766`, `070ae7d`, `895ac63`.
  - Explanation, observer, runtime, persistence, and protocol: `6537730`, `64fa55c`, `3a531fc`, `b15cfc5`, `b1d62e9`.
  - Documentation currency: `410e197`, `4a554bb`, `2a92950`, `590c21b`, `681abb3`, `9beee01`, `4a9b9df`, `cd87121`.
  - Post-review recovery and boundary-state corrections: `f742ba0`, `009b21d`, `b076385`, `4502b04`, `2b5a343`.
  - Final checkpoint documentation: `4ad2316`.
- 2026-07-26 remediation verification (independent re-run after receipt-reconciliation, checked-arithmetic, domain-geometry-reuse, and literal-version fixes):
  - `cargo fmt --all -- --check` passed.
  - LSP diagnostics clean on all four changed files.
  - Focused tests passed: `thermal_persistence` (10/10, including the three new forgery/overflow regressions and the literal-version contract), `thermal_neighbor_geometry`, `thermal_diffusion` (6/6), `thermal_boundaries`, `thermal_receipts`, `thermal_determinism`, `thermal_bootstrap`, and `thermal::tests::atomic_failure_rollback` (V15).
  - `cargo clippy -p causafera-domains -p causafera-runtime --all-targets --all-features -- -D warnings` passed, then repeated workspace-wide with the same result.
  - `cargo test --workspace --all-features` and `cargo test --workspace --no-default-features` both passed with zero failures.
  - `cargo run -p xtask -- ci` passed.
  - `git diff --check` reported no whitespace errors; diff scope confirmed limited to `crates/causafera-domains/src/thermal/neighbor.rs`, `crates/causafera-domains/tests/thermal_neighbor_geometry.rs`, `crates/causafera-runtime/src/runtime.rs`, and `crates/causafera-runtime/tests/thermal_persistence.rs`.
  - `.debug-journal.md` (temporary investigation ledger) removed after verification, per its own note.

---

## Appendix: Retained / Replaced / Expanded / Removed from exploratory draft

| Exploratory draft element | Disposition | Rationale |
|---------------------------|-------------|-----------|
| Thermal direction | **Retained** | Repository evidence supports advancing the Energy domain with a physical carrier. |
| Fixed-point thermal energy carrier | **Retained but redesigned** | State is `ThermalEnergy`, not a renamed `Temperature`; existing `f64` types are not migrated. |
| Explicit finite reservoirs | **Expanded** | Reservoirs initialized through historical bootstrap with provenance; experiment-recipe source deferred. |
| Intra-chunk 6-neighbor diffusion | **Replaced** | Conservative pairwise flux across undirected faces, including same-chart cross-chunk faces. |
| Adiabatic chunk boundaries | **Removed** | Violates INV-043. Same-chart exchange included; cross-chart deferred as unimplemented. |
| New `Phase::Energy` / shared Mana phase | **Removed** | System runs in `Phase::Physics` after existing registrations; no phase/RNG disruption. |
| `MaterialSurfaceThermalGate` and `condition +1` | **Removed from this tranche** | No physically meaningful material response defined yet; deferred to future tranche. |
| Uniform homogeneous heat capacity | **Retained** | Parameter in `ThermalParameters`; heterogeneous capacities deferred. |
| Observer/Explanation thermal schemas | **Retained** | Added bounded summary, delta, and conservation claim. |
