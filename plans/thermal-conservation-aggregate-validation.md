# Thermal Conservation Aggregate Cross-Validation ExecPlan

**Status:** Accepted

## Goal

Make every `ThermalConservationReceipt`'s six aggregate literals — `total_cell_energy_before/after`,
`total_reservoir_budget_before/after`, `total_material_retained_before/after` — determined functions
of materialized state and per-receipt data, instead of trusted snapshot literals. On
`RuntimeState::import_snapshot`, cross-validate every batch's aggregates against checked `i128` sums
of the actual imported field energies, reservoir budgets, and material retained energies, so that a
cell appearing in no transfer receipt of any batch can no longer be tampered with undetected. This
closes `TODO-THERMAL-006`.

No encoded byte changes, no section-major bump, no digest-schema bump, no domain contract change.
This is import-path integrity hardening: reader strictness, not format evolution.

## Context

`plans/conserved-thermal-energy-carrier.md` established the conserved thermal carrier and its
per-tick `ThermalConservationReceipt`; `plans/thermal-material-surface-coupling.md` added the third
conserved bucket (material retained energy) and its two aggregate literals. Both are complete.

The independent review of `3e46bc2` ("fix(runtime): reconcile thermal receipts and reuse domain
geometry") identified the gap this plan closes. Today's import path enforces three things:

1. Per receipt, the signed-flux transition equation `pre_state - Σ(face.signed_flux) -
   material.signed_flux == post_state` (`crates/causafera-runtime/src/runtime.rs:2675-2694`,
   verified 2026-07-28). This applies to every batch, not only the latest.
2. Per batch, the reservoir identity `total_reservoir_budget_before - total_reservoir_budget_after ==
   Σ accepted_injection` (`crates/causafera-runtime/src/runtime.rs:2748-2772`).
3. For the latest batch only, each receipt's `post_state` binds to the current field cell energy
   (`crates/causafera-runtime/src/runtime.rs:2773-2799`), and each receipt's material term binds to
   the surface's `retained_energy` (`crates/causafera-runtime/src/runtime.rs:1150-1171`, inside
   `RuntimeState::validate_snapshot_references` at `crates/causafera-runtime/src/runtime.rs:1090`).

Nothing binds the six aggregate literals themselves. The residual check
(`total_after - total_before == 0`) is scale-invariant: it is satisfied by any set of literals whose
three-bucket sums balance, including fabricated ones. A cell that never appears in a transfer
receipt of any batch — the common case, since receipts are only emitted for cells with non-zero
participation — is bound to nothing at all. Its energy can be edited freely and the snapshot still
imports.

This is import-path integrity hardening. It changes no simulation behaviour, no domain contract, no
observer read model, and no Explanation output. It does not attempt to close the pre-alpha
untrusted-snapshot threat-model carve-out documented in `SECURITY.md:20-24`; it removes one
specific, mechanically checkable class of undetected divergence within that carve-out.

Note on numbering: the acceptance criterion's phrase "existing V1-V23 thermal contracts" refers to
the verification numbering in `plans/conserved-thermal-energy-carrier.md:599-766`. This plan's own
V-numbers below are local to this document and do not shadow that list, exactly as
`plans/thermal-material-surface-coupling.md` restarts at V1.

## Relevant invariants

- **INV-014 — Provenance is first-class** (`docs/architecture/invariants.md:59`). The receipt chain
  is the stored provenance for the thermal carrier. Anchoring the aggregates against materialized
  state is what makes that chain load-bearing on import rather than decorative: after this change,
  historical batch totals are reconstructable from final state plus per-receipt data, not merely
  asserted by the snapshot.
- **INV-016 — Authoritative mutation is phase controlled** (`docs/architecture/invariants.md:67`).
  The chain identity I5 below is only sound because nothing outside the `Phase::Physics` thermal
  batch mutates the three buckets. This plan proves that by assertion rather than assuming it
  (RISK-4).
- **INV-017 — Performance is architectural** (`docs/architecture/invariants.md:71`). The validation
  is designed to ride loops the import path already runs, plus exactly one full-field pass over data
  the decoder already materialized.
- **INV-018 — Scale claims require reproducible benchmarks** (`docs/architecture/invariants.md:75`).
  The TODO's Performance Requirements demand a measured import-time delta. Stage 5 records real
  before/after measurements; estimates are inadmissible and no number appears in this plan until
  Stage 5 lands.
- **INV-038 — State digests are identities, not physical metrics**
  (`docs/architecture/invariants.md:157`). Digest bytes must be identical before and after this
  change for any snapshot that was already valid. Stage 4 makes that an executable gate, not an
  assertion in prose.
- **INV-039 — Production state requires causal initialization**
  (`docs/architecture/invariants.md:161`). This validation enforces that imported thermal state is
  reachable only through committed batches, rather than being an arbitrary vector of numbers with a
  self-consistent summary attached.
- **INV-042 — Architecture remains modular and cohesive**
  (`docs/architecture/invariants.md:175`). `crates/causafera-runtime/src/runtime.rs` is 4250 lines
  (verified 2026-07-28). The validator goes in a new named sibling module, not into `runtime.rs`.

## Ontology domains affected

None. No domain contract, primitive, parameter, or causal process changes. Energy, Matter, and
Physics are untouched at the domain layer; `crates/causafera-domains` is not modified by this plan.
The change is confined to the runtime's snapshot-import validation surface.

## Causal carriers affected

None added, removed, or altered. The existing conserved carriers — `ThermalFieldSet` cell energy,
`ThermalReservoir` budgets, and material-surface retained thermal energy — are read for validation
only. No carrier gains state, changes representation, or changes conservation semantics.

## Relevant documents

- `docs/architecture/invariants.md`
- `docs/architecture/detailed-development-rebaseline.md`
- `docs/development/todo-backlog.md` (`TODO-THERMAL-006`)
- `plans/conserved-thermal-energy-carrier.md` (V1-V23 thermal contracts; receipt architecture)
- `plans/thermal-material-surface-coupling.md` (third conserved bucket; the material literals)
- `docs/rfc/RFC-PERSIST-001.md` (section-major and digest-schema history)
- `docs/simulation/long-run-experiments.md:66-89` (measured unbounded thermal-receipt growth)
- `SECURITY.md:20-24` (pre-alpha untrusted-snapshot threat-model carve-out)
- `docs/ontology/causal-carriers.md`, `docs/ontology/domain-coverage-matrix.md`
- `plans/candidate-ledger.md:30-39` (`ledger-2026-07-22-reject-cross-chart-propagation`)

## Current state

All line numbers below were read from source on 2026-07-28 and are recorded in the Decision log.

- `ThermalConservationReceipt` is defined at
  `crates/causafera-domains/src/thermal/records.rs:191-200` with fields `tick`,
  `total_cell_energy_before`, `total_cell_energy_after`, `total_reservoir_budget_before`,
  `total_reservoir_budget_after`, `total_material_retained_before`, `total_material_retained_after`,
  `residual`.
- `conservation_receipt` computes them at
  `crates/causafera-domains/src/thermal/receipts.rs:98-155`. Critically,
  `total_cell_energy_before = sum_i128(committed.values())` at
  `crates/causafera-domains/src/thermal/receipts.rs:107` sums the **pre-injection** `committed` map,
  while `total_cell_energy_after` at `receipts.rs:108` sums the post-diffusion `after` map. The call
  site passing `&committed` is `crates/causafera-domains/src/thermal/evolution.rs:53`, inside
  `ThermalFieldSet::propose_evolution` (`evolution.rs:17-69`).
- `accept_injections` (`crates/causafera-domains/src/thermal/injection.rs:27-99`) writes the
  **post-injection** pre-state at `injection.rs:82` (`pre_state.insert(injection.target, next)`
  where `next = current + accepted`). That post-injection value becomes the receipt's `pre_state`
  (bound as `logical_pre_state` at `crates/causafera-domains/src/thermal/receipts.rs:34-38`).
  This asymmetry is the origin of RISK-1 below.
- `preflight_faces` (`crates/causafera-domains/src/thermal/diffusion.rs:18-167`) creates face records
  strictly pairwise at `diffusion.rs:82-93` under the ownership skip `if key >= neighbor { continue; }`
  at `diffusion.rs:64-66`. Out-of-region neighbours produce `ThermalBoundaryRecord` (no
  `signed_flux`) at `diffusion.rs:56-62`, not a one-sided face. `ThermalBoundaryBehavior` has the
  single variant `NoFluxOutsideActiveRegion` (`records.rs:124-126`).
- `materials_after` is a **full-population** map: `diffusion.rs:117` inserts the unchanged value when
  `accepted == 0`, and `diffusion.rs:127` inserts the exchanged value otherwise. `material_records`
  is inserted only in the exchanged branch at `diffusion.rs:128`. So
  `total_material_retained_before/after` sum every material site, changed or not — the same
  untouched-member hole that cells have.
- `import_thermal_snapshot` is a free function at `crates/causafera-runtime/src/runtime.rs:2427`,
  returning a 6-tuple, called from `RuntimeState::import_snapshot` at `runtime.rs:980-987`. Its
  per-receipt loop `for receipt in snapshot.transfer_receipts` begins at `runtime.rs:2601`.
- `RuntimeState::validate_snapshot_references(&self)` is at `crates/causafera-runtime/src/runtime.rs:1090`
  and already hosts the latest-batch material bind at `runtime.rs:1150-1171`, which needs
  `self.material_surfaces` — imported from a **different** snapshot section
  (`import_material_surfaces` at `runtime.rs:2946`, called at `runtime.rs:1031`, decoded from
  `MATERIAL_SURFACE_SECTION_ID` while thermal is `THERMAL_SECTION_ID 0x000E`). `material_surfaces`
  is therefore not available inside `import_thermal_snapshot`.
- `physical_state_digest` already hashes all six aggregate literals at
  `crates/causafera-runtime/src/runtime.rs:2091-2096`.
- `RuntimeError::InvalidSnapshot(&'static str)` is at `crates/causafera-runtime/src/runtime.rs:455`.
- `ThermalFieldSet::batch_sequence() -> u64` is at
  `crates/causafera-domains/src/thermal/field.rs:154-156`. `snapshot.conservation_receipts.len()` is
  already required to equal it at `crates/causafera-runtime/src/runtime.rs:2550`, and the latest
  conservation trace is anchored at `runtime.rs:2583-2589`.
- `decode_thermal_energy_vec` (`crates/causafera-runtime/src/snapshot_sections.rs:2383-2399`, called
  at `snapshot_sections.rs:561`) already materializes the full per-cell energy vector during decode.
- Version constants pinned by `thermal_persistence_literal_version_contract`
  (`crates/causafera-runtime/tests/thermal_persistence.rs:412-426`): `THERMAL_SECTION_ID = 0x000E`,
  `THERMAL_SECTION_MAJOR = 2`, `CURRENT_DIGEST_SCHEMA_VERSION = 6`.
  `MATERIAL_SURFACE_SECTION_MAJOR` (currently 3) is not pinned by that test.
- `RuntimeState` fields: `thermal_fields: ThermalFieldSet` (`runtime.rs:487`),
  `thermal_reservoirs: BTreeMap<ThermalReservoirId, ThermalReservoir>` (`runtime.rs:490`),
  `material_surfaces: BTreeMap<MaterialSurfaceId, MaterialSurface>` (`runtime.rs:507`),
  plus `thermal_receipts` and `thermal_conservation_receipts`. Per-cell energies are read via
  `field.energy()` returning a slice.

## Proposed architecture

### Notation

Index batches `b = 1..N` in ascending conservation `TraceId` order, which is the iteration order of
the `BTreeMap<TraceId, ThermalConservationReceipt>` and equals commit order.
`N == thermal_fields.batch_sequence()`.

- `C` = total cell energy, `B` = total reservoir budget, `M` = total material retained energy.
- Superscript `-` = the batch's reported "before" literal, `+` = its reported "after" literal.
- `R_b` = the set of `ThermalCellTransferReceipt`s whose `conservation_trace` is batch `b`'s trace.

All sums are computed in `i128` with `checked_add`. Any overflow returns
`RuntimeError::InvalidSnapshot`, never a wrap.

### The six identities

**I1 — Reservoirs (ALREADY ENFORCED, `crates/causafera-runtime/src/runtime.rs:2748-2772`)**

```text
B_b^- - B_b^+ == Σ_{r∈R_b} Σ_{res∈r.reservoirs} res.accepted_injection
```

**I2 — Cells (NEW). Note the injection term.**

```text
C_b^+ - C_b^- == Σ_{r∈R_b} (r.post_state - r.pre_state)
               + Σ_{r∈R_b} Σ_{res∈r.reservoirs} res.accepted_injection
```

The injection term is mandatory and is the single highest-risk line in this change. `r.pre_state` is
the **post-injection** value (`injection.rs:82` → `receipts.rs:34-38`), while `C_b^-` is summed from
the **pre-injection** `committed` map (`receipts.rs:107`). For each cell,
`post_state - committed = (post_state - pre_state) + accepted_injection`. Omitting the term makes I2
fail on every snapshot with a live reservoir. See RISK-1.

**I3 — Material (NEW)**

```text
M_b^+ - M_b^- == Σ_{r∈R_b, r.material = Some(m)} m.signed_flux
```

Sign convention matches `ThermalFaceRecord::signed_flux`: positive flux leaves the cell and enters
the material, so a positive sum raises `M`.

**I3a — Per receipt (NEW, free)**

```text
m.retained_after - m.retained_before == m.signed_flux    for every r with r.material = Some(m)
```

**I4 — Residual (ALREADY ENFORCED, `crates/causafera-domains/src/thermal/receipts.rs:139-144` on
production, and re-derived on import)**

```text
(C_b^+ + B_b^+ + M_b^+) - (C_b^- + B_b^- + M_b^-) == 0
```

**I5 — Chain (NEW)**, for `b` in `2..=N`:

```text
C_{b-1}^+ == C_b^-    and    B_{b-1}^+ == B_b^-    and    M_{b-1}^+ == M_b^-
```

**I6 — Terminal anchor (NEW)**, against materialized final state:

```text
C_N^+ == Σ_chunk Σ_cell field.energy()[cell]
B_N^+ == Σ_{res ∈ thermal_reservoirs.values()} res.budget
M_N^+ == Σ_{s ∈ material_surfaces.values()} s.thermal.retained_energy
```

**I7 —** If `N == 0`, skip the whole aggregate validation. A bootstrapped, never-evolved snapshot has
no conservation receipts (`runtime.rs:2550` already requires
`conservation_receipts.len() == batch_sequence()`), and the existing bootstrap-anchor checks at
`runtime.rs:1203-1212` and `runtime.rs:2822` continue to govern that case unchanged.

### Soundness argument

I6 pins batch `N`'s three "after" literals to real materialized state. Given those, I1, I2, and I3
each pin one of batch `N`'s "before" literals from `R_N` alone — each identity has exactly one
remaining unknown once the "after" value is fixed. I5 then propagates batch `N`'s "before" literals
onto batch `N-1`'s "after" literals, and the argument repeats by downward induction to `b = 1`.

Therefore all `6N` aggregate literals become determined functions of (final materialized state,
per-receipt data). Per-receipt data is itself already bound: the signed-flux transition equation at
`crates/causafera-runtime/src/runtime.rs:2675-2694` ties every receipt's `pre_state`, `post_state`,
face fluxes, and material flux together for every batch, and I3a additionally ties the material
record's own before/after pair to its flux.

**No historical field state is stored or reconstructed.** Backward reconstruction of per-cell
historical energies is explicitly unnecessary and is not attempted: the induction runs entirely on
*scalar totals* plus already-resident receipt data, and totals are all the aggregate literals claim
to be. Reconstructing per-cell history would require storing `N` full field snapshots — unbounded
memory for a strictly weaker guarantee than what the totals chain already delivers.

### Known residual limitation (documented, not chased)

A uniform offset applied only to every batch's `C_b^-` and `C_b^+` is rejected by I6 because the
terminal `C_N^+` no longer equals the materialized final field. The genuinely unclosed case requires
a coordinated forgery: shift the materialized final field by `+Δ` and apply the same `+Δ` to
`C_b^-` and `C_b^+` for every `b`. I6 then matches the shifted field, every difference identity is
preserved, I4 cancels the offset, and I5 preserves chain continuity. `C_1^-`, the bootstrap total,
has no independent persisted anchor, so nothing records the original total before the first batch.
A snapshot that consistently shifts both final state and the full aggregate chain is not detectable
by this scheme.

This falls squarely inside the pre-alpha untrusted-snapshot carve-out in `SECURITY.md:20-24` and is
**not** in scope. It must be named in the `TODO-THERMAL-006` Resolution paragraph.

**Escalation trigger, evaluated in Stage 1 only:** if the thermal bootstrap event's committed effects
already encode per-cell initial energies (or a bootstrap total), then anchoring `C_1^-` against them
closes the induction at the bottom as well. Take that only if it is a few lines against existing
committed state. If it requires new persisted state, a new event, or a format change, document the
finding in the Decision log and move on. Do not expand scope.

### Placement

New sibling module `crates/causafera-runtime/src/thermal_conservation_validation.rs`, declared
`mod thermal_conservation_validation;` in `crates/causafera-runtime/src/lib.rs` alongside the
existing private `mod thermal;` / `mod thermal_events;` (INV-042; `runtime.rs` is already 4250
lines).

The module exposes:

```rust
/// Per-batch receipt-side totals, accumulated once during import.
pub(crate) struct ThermalBatchReceiptTotals {
    pub(crate) cell_transition: i128,     // Σ (post_state - pre_state)
    pub(crate) accepted_injection: i128,  // Σ Σ accepted_injection
    pub(crate) material_flux: i128,       // Σ material.signed_flux
}
```

Wiring:

1. **Receipt-side accumulation** folds into the existing per-receipt loop in the free function
   `import_thermal_snapshot` (`crates/causafera-runtime/src/runtime.rs:2601`), keyed by
   `receipt.conservation_trace` into a `BTreeMap<TraceId, ThermalBatchReceiptTotals>`. I3a is checked
   inline in the same loop where the material record is already reconstructed
   (`runtime.rs:2645-2674`) — it costs one comparison and needs no extra data.
   `import_thermal_snapshot` returns this map as a seventh tuple element; `RuntimeState::import_snapshot`
   binds it at `runtime.rs:980-987` and holds it as a local.

2. **The aggregate validator** is the post-assembly `&self` step, because I6's material anchor needs
   `self.material_surfaces`, which is imported from a different section
   (`import_material_surfaces`, `runtime.rs:2946`, called at `runtime.rs:1031`) and is therefore
   unavailable inside `import_thermal_snapshot`. `RuntimeState::validate_snapshot_references`
   (`runtime.rs:1090`) gains one parameter and forwards to the new module:

   ```rust
   fn validate_snapshot_references(
       &self,
       thermal_receipt_totals: &BTreeMap<TraceId, ThermalBatchReceiptTotals>,
   ) -> Result<(), RuntimeError>
   ```

   Its existing call site in `RuntimeState::import_snapshot` passes the local from step 1. The
   validator call sits immediately after the existing latest-batch material bind at
   `runtime.rs:1150-1171`, so all thermal cross-section validation is contiguous.

3. The validator performs, in order: the I6 terminal anchor (one full-field pass, one reservoir pass,
   one material pass), then a single descending walk over `b = N..1` applying I5, I1, I2, I3, and I4.

### Cost

- **One** full-field summation, for the I6 terminal anchor only. Import already materializes every
  cell via `decode_thermal_energy_vec` (`snapshot_sections.rs:2383-2399`, called at
  `snapshot_sections.rs:561`), so this is `O(V)` `i128` adds over hot, already-resident data.
- Everything else is `O(Σ|receipts|)` folded into the loop at `runtime.rs:2601` that already runs,
  plus `O(N)` for the batch walk.
- Total: `O(V + Σ|receipts| + N)`, no new traversals of anything.
- **Never** a per-batch full-field summation. That would be `O(N·V)` and is the one design mistake
  this section exists to forbid. The TODO's "consider incremental or batch-scoped summation"
  suggestion is answered by the induction: incremental schemes buy nothing over a single terminal
  anchor plus an `O(N)` chain walk, because the chain already reduces `N` full sums to one.

### Versioning: no bump

`THERMAL_SECTION_MAJOR` stays 2, `MATERIAL_SURFACE_SECTION_MAJOR` stays 3, and
`CURRENT_DIGEST_SCHEMA_VERSION` stays 6. Reasons:

1. **Zero encoded byte changes.** No field is added, removed, reordered, or reinterpreted. Encode and
   decode are untouched.
2. **Digests are already identical.** `physical_state_digest` already hashes all six aggregate
   literals (`crates/causafera-runtime/src/runtime.rs:2091-2096`), so for any snapshot that was
   already valid, the digest before and after this change is bit-identical. Stage 4 makes this an
   executable gate, not a claim.
3. **A section major encodes byte layout for readers.** Reader *strictness* is not layout. Bumping
   would make current readers reject snapshots that are, in fact, valid — turning a correctness
   improvement into a spurious compatibility break.

`thermal_persistence_literal_version_contract`
(`crates/causafera-runtime/tests/thermal_persistence.rs:412-426`) anchors these constants and must
remain untouched. If it needs editing, the change has exceeded this plan's scope and must stop.

## Primitive vs emergent review

Neither. No new primitive is introduced and no emergent behaviour is claimed. The validator derives
nothing that is retained: it computes checked `i128` sums, compares them to snapshot literals,
returns `Ok(())` or `Err(RuntimeError::InvalidSnapshot(_))`, and drops every intermediate. It writes
no state, emits no event, produces no trace, and adds no field to `RuntimeState`. The
`ThermalBatchReceiptTotals` accumulator is a transient import-time local, not authoritative state.

## Non-goals

1. **`TODO-THERMAL-001` cross-chart thermal transport.** Blocked, not deferred by preference:
   `docs/rfc/RFC-GEO-002.md:104` explicitly defers cross-chart transforms and atlas generation; no
   `ChartTransform` or seam-registry type exists; `plans/candidate-ledger.md:30-39`
   (`ledger-2026-07-22-reject-cross-chart-propagation`) names exactly these blockers
   (`WorldGeometrySchemaId` registry, chart seam transforms, atlas generation or hand-off,
   persistence of cross-chart state); and production bootstrap creates a single chart
   (`crates/causafera-runtime/src/config.rs:87`, `chart_id: SpatialChartId::new(1)`).
2. **`TODO-THERMAL-007`** (material expansion / damage accumulation / phase change). Requires a user
   decision between three materially different response models, each of which would be a different
   plan. Not bundled.
3. **`TODO-THERMAL-008`** (heterogeneous per-material thermal properties). Opposite persistence
   posture: it forces `THERMAL_SECTION_MAJOR` 2→3 and `CURRENT_DIGEST_SCHEMA_VERSION` 6→7, and pulls
   `f64` `Material::thermal_conductivity`/`specific_heat` into an integer fixed-point domain,
   requiring its own float-to-fixed conversion-and-determinism policy and a per-material re-proof of
   the `6 * transfer_fraction + material_exchange_fraction <= scale` bound. Bundling it would make
   this plan's "no version bump" gate impossible to state. Not bundled.
4. **`TODO-THERMAL-003` / `004` / `005`.** Independently deferred, untouched.
5. **Receipt retention or compaction.** The separately-documented unbounded thermal-receipt growth in
   `physical_state_digest` (`docs/simulation/long-run-experiments.md:66-89`) is adjacent and real —
   this plan's validation cost scales with the same `Σ|receipts|` — but closing it requires reordering
   the digest write sequence and a deliberate schema migration. Out of scope; noted so the Stage 5
   benchmark is read in that context.
6. **Non-latest-batch per-cell bounds checking.** Already closed
   (`runtime_import_rejects_non_latest_receipt_cell_index_out_of_bounds`,
   `crates/causafera-runtime/tests/thermal_persistence.rs:252-273`).
7. **Closing the `SECURITY.md:20-24` untrusted-snapshot carve-out**, including the coordinated
   final-state-plus-chain limitation described above.
8. **Any change that reorders or alters digest input**, adds a snapshot field, or changes any section
   major or digest schema version.
9. **Any semantic or derived-temperature concept.** Energy remains the authoritative unit.
10. No observer, Explanation, protocol, UI, or domain-crate change.

## Implementation stages

Every task below names its exact files and its binary verification command. Stages are ordered and
must not be reordered. Stage 1 and Stage 2 are diagnostic gates: they must produce their stated
findings before any production line is written.

### Stage 1 — Observational harness, NO enforcement

Files: `crates/causafera-runtime/tests/thermal_conservation_aggregates.rs` (new, test-only).

No production source is modified in this stage.

1. **Build the multi-batch fixture** in `crates/causafera-runtime/tests/support/thermal.rs` as a
   helper returning a `RuntimeSnapshotData` produced by a real production `Runtime`
   (`Runtime::new(runtime_config(seed))` + `run_ticks(k)` + `export_snapshot()`), following the
   pattern of `evolved_snapshot` at `crates/causafera-runtime/tests/thermal_persistence.rs:16`.
   Choose `k` and the seed so the fixture satisfies the RISK-2 criteria below.
2. **Fixture assertion gate (RISK-2).** Write
   `thermal_conservation_aggregates.rs::fixture_satisfies_aggregate_validation_preconditions`
   asserting, on the exported snapshot, all four of:
   - `N >= 3` (`snapshot.thermal.field_set.batch_sequence >= 3`);
   - at least one reservoir with strictly positive residual `budget`;
   - at least one material surface with strictly positive `retained_energy`;
   - at least one thermal cell whose `ThermalCellKey` appears in **no** `transfer_receipts` entry of
     **any** batch.
   If any assertion fails, fix the fixture (different seed, more ticks, adjusted reservoir schedule)
   before proceeding. Do not proceed on a fixture that cannot satisfy all four.
3. **RISK-3 assertion.** Write
   `thermal_conservation_aggregates.rs::face_signed_flux_sums_to_zero_per_batch`: for each batch,
   assert `Σ_{r∈R_b} Σ_{f∈r.faces} f.signed_flux == 0`. This must be shown empirically, not argued
   from `diffusion.rs:82-93`'s pairwise construction.
4. **RISK-4 assertion.** Write
   `thermal_conservation_aggregates.rs::non_physics_phases_preserve_thermal_buckets`: export a
   `RuntimeSnapshotData` after enough ticks to exercise Action and Mana phases, then inspect the
   committed `traces.events` of the snapshot to prove that every post-bootstrap mutation anchor for
   the three buckets is a `Phase::Physics` event. Specifically assert:
   - For every `thermal_conservation_receipts[*].trace`, the resolved event has `phase == Phase::Physics`
     and `kind == THERMAL_CONSERVATION_EVENT_KIND`.
   - For every `thermal_receipts[*][*].cell_change_trace_id`, the resolved event has `phase == Phase::Physics`
     and `kind == THERMAL_CELL_CHANGE_EVENT_KIND`.
   - For every `thermal_receipts[*][*].reservoirs[*].transfer_trace_id`, the resolved event has
     `phase == Phase::Physics` and `kind == THERMAL_RESERVOIR_TRANSFER_EVENT_KIND`.
   - For every `material_surface_thermal_transitions[*].trace`, the resolved event has `phase == Phase::Physics`
     and `kind == MATERIAL_SURFACE_THERMAL_EXCHANGE_EVENT_KIND`.
   - For every `thermal_reservoirs[*].last_change` trace, the resolved event has `phase == Phase::Physics`
     or `phase == Phase::Lifecycle` (the latter only for the bootstrap trace); for every
     `thermal_reservoirs[*].bootstrap_trace`, the resolved event has `phase == Phase::Lifecycle`.
   - For every `thermal_fields[*].last_change` trace, the resolved event has `phase == Phase::Physics`
     or `phase == Phase::Lifecycle`; the `Phase::Lifecycle` case is permitted because untouched cells
     retain their bootstrap trace (`causafera-domains/src/thermal/field.rs:46`). A cell whose
     `last_change` trace differs from the bootstrap trace must have `phase == Phase::Physics`.
   This is the executable proxy available from the public `Runtime` API (`run_ticks` / `export_snapshot`),
   because `Runtime` does not expose per-phase execution. It mirrors the trace-phase checks already in
   `RuntimeState::validate_snapshot_references` (`runtime.rs:1179-1257`); making it an explicit test
   proves the I5 chain assumption before I5 is enforced. If any non-Physics event mutates one of the
   three buckets after bootstrap, this assertion fails and the chain identity is unsound.
5. **Observational identity recomputation.** Write
   `thermal_conservation_aggregates.rs::engine_snapshots_satisfy_all_six_identities`: a test-only
   recomputation of I1, I2, I3, I3a, I4, I5, I6 over the engine-produced fixture, asserting each
   holds. This is the specification the Stage 3 implementation must match, written before it.
6. **Escalation evaluation.** Inspect the thermal bootstrap event's committed effects (the
   `THERMAL_FIELD_BOOTSTRAP_EVENT_KIND` path in `crates/causafera-runtime/src/bootstrap.rs` and
   `crates/causafera-runtime/src/thermal_events.rs`) and determine whether they encode per-cell
   initial energies or a bootstrap total that could anchor `C_1^-`. Record the finding in the
   Decision log either way. Implement the anchor **only** if it is a few lines against already-committed
   state; otherwise document and move on.

   **Finding (recorded 2026-07-28):** the bootstrap event commits a generic stage effect via
   `commit_bootstrap_stage_event` (`bootstrap.rs:323-333`) with fingerprinted before/after values,
   not the actual per-cell energy total. The initial energies live only in `ThermalField::energy`
   and `ThermalField::last_change_before`; no persisted scalar records the bootstrap total cell
   energy. Anchoring `C_1^-` would require a new event, a new persisted field, or a new effect
   encoding — more than "a few lines against already-committed state". Therefore the coordinated
   final-state-plus-chain limitation remains scoped to `SECURITY.md:20-24` and is not closed by this
   plan.

**GATE (Stage 1):** all six identities hold on the fixture; the RISK-2 fixture assertion passes; the
RISK-3 and RISK-4 assertions pass; the `C_1^-` escalation is decided and recorded.

```bash
cargo test -p causafera-runtime --test thermal_conservation_aggregates
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

### Stage 2 — RED negative controls against the CURRENT import path

Files: `crates/causafera-runtime/tests/thermal_conservation_aggregates.rs` (extend).

Author negative controls 1-4 from the Verification section against today's unmodified import path
and record their actual outcome. These are expected to **fail to reject** — that is the point: it
proves the gap is real and that the controls are not vacuously passing for some unrelated reason.

1. Author `runtime_import_rejects_forged_cell_and_material_aggregate_totals` (control 1).
2. Author `runtime_import_rejects_untouched_cell_energy_tampering` (control 2).
3. Author `runtime_import_rejects_coordinated_untouched_cell_and_terminal_total_forgery` (control 3).
4. Author `runtime_import_rejects_historical_batch_material_total_forgery` (control 4).
5. Confirm observationally that `total_reservoir_budget_before/after` forgery is already rejected by
   the existing reservoir identity (`runtime.rs:2748-2771`); record this in Progress.
6. Run the four new controls and capture the transcript verbatim into the Progress section, showing
   that `RuntimeState::import_snapshot` currently returns `Ok(_)` for each tampered cell/material
   snapshot.

**GATE (Stage 2):** controls 1-4 each demonstrably fail to reject on the current code; the
reservoir-total forgery is already rejected by the existing identity (recorded as confirmation, not
a failure); and the workspace compiles clean. If any cell/material control rejects *today*, stop:
either the control is testing something already covered (rewrite it to hit the real hole) or the
gap analysis is wrong (revisit this plan before writing any implementation).

```bash
cargo test -p causafera-runtime --test thermal_conservation_aggregates -- --nocapture
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

**No checkpoint commit is created for Stage 2.** Per `PLANS.md`, a RED state is never a checkpoint.
The Stage 2 control tests are carried in the working tree and committed together with the Stage 3
implementation that turns them green, as one atomic RED→GREEN checkpoint. Stage 2's evidence is the
recorded transcript, not a commit.

### Stage 3 — Implement

Files: `crates/causafera-runtime/src/thermal_conservation_validation.rs` (new),
`crates/causafera-runtime/src/lib.rs` (one `mod` line),
`crates/causafera-runtime/src/runtime.rs` (accumulator fold + one signature + one call).

1. Create `crates/causafera-runtime/src/thermal_conservation_validation.rs` with
   `ThermalBatchReceiptTotals` and the validator entry point. All sums use `i128` + `checked_add`;
   every overflow path returns `RuntimeError::InvalidSnapshot` with a distinct `&'static str`
   message. Every comparison failure returns a distinct message so a failing test names the identity
   it violated.
2. Add `mod thermal_conservation_validation;` to `crates/causafera-runtime/src/lib.rs`, private,
   alphabetically adjacent to `mod thermal;` / `mod thermal_events;`.
3. In `import_thermal_snapshot` (`crates/causafera-runtime/src/runtime.rs:2427`), fold the per-batch
   accumulation into the existing loop at `runtime.rs:2601` and check I3a inline where the material
   record is reconstructed (`runtime.rs:2645-2674`). Return the accumulator as a seventh tuple
   element.
4. Bind the accumulator at the call site (`crates/causafera-runtime/src/runtime.rs:980-987`) and pass
   it to `validate_snapshot_references`, whose signature gains the one parameter.
5. Call the validator from `validate_snapshot_references` (`crates/causafera-runtime/src/runtime.rs:1090`)
   immediately after the existing latest-batch material bind at `runtime.rs:1150-1171`, guarded by
   the I7 early return when `self.thermal_fields.batch_sequence() == 0`.
6. Verify by inspection that `crates/causafera-domains` is unmodified and that no encode/decode
   function in `crates/causafera-runtime/src/snapshot_sections.rs` is touched.

**GATE (Stage 3):** Stage 2's controls 1-4 flip GREEN; Stage 1's observational tests still pass; the
full workspace is clean under both feature configurations.

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo test --workspace --no-default-features
cargo run -p xtask -- ci
git diff --check
```

### Stage 4 — Format and determinism neutrality

Files: `crates/causafera-runtime/tests/thermal_conservation_aggregates.rs` (extend).

1. **Digest neutrality gate.** Record `physical_state_digest` and `history_digest` for the Stage 1
   fixture at the pre-change commit, then assert byte-identical values post-change. This is an
   executable gate producing a recorded pair of values in Progress, not an assertion in prose.
2. **Version-constant gate.** Confirm by `git diff` that `THERMAL_SECTION_MAJOR`,
   `MATERIAL_SURFACE_SECTION_MAJOR`, and `CURRENT_DIGEST_SCHEMA_VERSION` are untouched, and that
   `crates/causafera-runtime/tests/thermal_persistence.rs:412-426`
   (`thermal_persistence_literal_version_contract`) is unmodified and passing.
3. **Round-trip gate.** `export → import → export` produces byte-identical snapshot bytes and an
   identical `physical_state_digest` (control 6).
4. **Continuation gate.** Continuing `k` ticks from a resumed state yields digests identical to an
   uninterrupted `2k`-tick run (control 6).
5. **Order-independence.** Control 5, asserted at the accumulator level (see Verification).

**GATE (Stage 4):** all four gates green; the recorded digest pair is identical.

```bash
cargo test -p causafera-runtime --test thermal_conservation_aggregates
cargo test -p causafera-runtime --test thermal_persistence
git diff -- crates/causafera-runtime/src/snapshot_sections.rs
git diff -- crates/causafera-runtime/tests/thermal_persistence.rs
cargo test --workspace --all-features
```

### Stage 5 — Benchmark and record

Files: `crates/causafera-runtime/src/benchmark.rs`,
`crates/causafera-runtime/examples/thermal_import_benchmark.rs` (new),
`docs/development/todo-backlog.md`, `CHANGELOG.md`,
`docs/ontology/causal-carriers.md`, `docs/ontology/domain-coverage-matrix.md`, `PLANS.md`,
this plan.

1. **Measure import wall time** with the harness described in the Benchmark plan section. Add
   `measure_import_wall_time` to `crates/causafera-runtime/src/benchmark.rs` and create
   `crates/causafera-runtime/examples/thermal_import_benchmark.rs`. Run
   `cargo run -p causafera-runtime --example thermal_import_benchmark --release` on the pre-change
   code and the post-change code, using the same harness, seed, tick count, 10 repetitions and 100
   imports per repetition. Record the workload description, the snapshot's `N`, `V`, and
   `Σ|receipts|`, hardware, toolchain, exact command, and mean/median/stddev for both measurements
   **in the Benchmark plan section of this file**. INV-018 forbids estimates: if the measurement
   cannot be taken, say so and do not report a number.
2. **Close `TODO-THERMAL-006`** in `docs/development/todo-backlog.md:468-483` with a Resolution
   paragraph that names: the six identities, the terminal-anchor-plus-induction structure, the
    measured import-time delta, and the coordinated final-state-plus-chain limitation with its
    explicit `SECURITY.md:20-24` scoping.
3. **`CHANGELOG.md`** — aggregate conservation cross-validation on import; state explicitly that no
   section major and no digest schema version changed.
4. **`docs/ontology/causal-carriers.md`** — note in the implemented-carrier-boundaries discussion
   that the thermal carrier's persisted aggregate totals are now bound to materialized state on
   import.
5. **`docs/ontology/domain-coverage-matrix.md`** — update the Energy row and the Simulation-runtime
   row for the strengthened import-integrity evidence. Do not raise any maturity level; this is
   integrity hardening, not depth.
6. **`PLANS.md`** — move this plan from Draft Plans to Completed Detailed Development Plans, with a
   `TODO-THERMAL-006` reference.
7. Complete this plan's Decision log and Progress sections.

**GATE (Stage 5):** benchmark measured and recorded verbatim; the TODO carries a Resolution
paragraph naming the residual limitation; the full CI gate is green.

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
git diff --check
node tools/audit/check-entry-points.mjs
node tools/audit/run-source-tests.mjs
```

## Verification

Every negative control runs against an **engine-produced** snapshot with `N >= 3` batches, satisfying
the Stage 1 RISK-2 fixture criteria. Tests live in
`crates/causafera-runtime/tests/thermal_conservation_aggregates.rs` and mirror the style of
`crates/causafera-runtime/tests/thermal_persistence.rs`: `// Given:` / `// When:` / `// Then:`
comments and `assert!(matches!(imported, Err(RuntimeError::InvalidSnapshot(_))))`.

### V1 — Fixture preconditions (RISK-2 gate)
`N >= 3`; at least one reservoir with residual budget; at least one surface with non-zero retained
energy; at least one cell that is in **no** transfer receipt of **any** batch **and** has **no**
thermal boundary record (i.e., it is genuinely unbound by today's validation surface).
Test: `::fixture_satisfies_aggregate_validation_preconditions`.

### V2 — Face flux sums to zero per batch (RISK-3)
For each batch, `Σ Σ face.signed_flux == 0` on engine output.
Test: `::face_signed_flux_sums_to_zero_per_batch`.

### V3 — Non-Physics phases preserve the three buckets (RISK-4)
From an exported `RuntimeSnapshotData`, every trace anchor that records a post-bootstrap mutation
to cell energy, reservoir budget, or material retained energy resolves to a `Phase::Physics` event.
Bootstrap anchors are `Phase::Lifecycle`. Specifically: all `THERMAL_CONSERVATION_EVENT_KIND`,
`THERMAL_CELL_CHANGE_EVENT_KIND`, `THERMAL_RESERVOIR_TRANSFER_EVENT_KIND`, and
`MATERIAL_SURFACE_THERMAL_EXCHANGE_EVENT_KIND` events are Physics-phase; untouched cells may still
carry their initial `Phase::Lifecycle` bootstrap trace.
Test: `::non_physics_phases_preserve_thermal_buckets`.

### V4 — All six identities hold on engine output
I1, I2, I3, I3a, I4, I5, I6 recomputed test-side over the fixture.
Test: `::engine_snapshots_satisfy_all_six_identities`.

### V5 — Forged aggregate cell / material totals (negative control 1)
Perturb each of the four currently unanchored aggregate literals independently, in both the latest
batch and a non-latest batch: `total_cell_energy_before`, `total_cell_energy_after`,
`total_material_retained_before`, `total_material_retained_after` — eight `(literal, batch)`
combinations. Each combination is exercised at `+1` and at `-1`, giving 16 rejection assertions. Every
one must reject on the post-change validator.
Test: `::runtime_import_rejects_forged_cell_and_material_aggregate_totals`.

Note on reservoir totals: `total_reservoir_budget_before` and `total_reservoir_budget_after` are
already cross-validated against `Σ accepted_injection` in today's import path
(`runtime.rs:2748-2771`). Forging them is already rejected, so they do not prove a new gap. Stage 2
records this as a confirming observation rather than as a RED failing control.

### V6 — Unbound-cell tampering (negative control 2)
Mutate the energy of a cell that is in **no** transfer receipt of **any** batch **and** has **no**
thermal boundary record, leaving every receipt and every aggregate literal alone. Must reject via
I6. The active-region boundary is already bound by `import_thermal_boundary_records`
(`runtime.rs:2816-2894`), so the reported gap concerns only genuinely interior, unbound cells.
**This is the test that proves the reported gap is closed. Without it the plan is unverified.**
Test: `::runtime_import_rejects_untouched_cell_energy_tampering`.

### V7 — Coordinated forgery (negative control 3)
Tamper an unbound cell (no receipt, no boundary record) **and** adjust `C_N^+` to match, so I6 is
satisfied. Must still reject, via I2 (batch `N`'s "before" no longer reconciles with `R_N`) or I5
(the chain to batch `N-1` breaks) — not via I6. Assert the rejection message identifies I2 or I5,
so the test cannot pass for the wrong reason.
Test: `::runtime_import_rejects_coordinated_untouched_cell_and_terminal_total_forgery`.

### V8 — Historical-batch tampering (negative control 4)
Perturb batch 1's `total_material_retained_after`. Must reject via I5 (batch 1's "after" no longer
equals batch 2's "before"). Assert the rejection message identifies I5.
Test: `::runtime_import_rejects_historical_batch_material_total_forgery`.

### V9 — Order independence (negative control 5)
The accept/reject verdict is invariant under input permutation. Exercise this at the **accumulator
level**, not by shuffling encoded bytes: decode already enforces strict receipt and trace ordering
(`crates/causafera-runtime/src/runtime.rs:2591-2600`, `2741-2747`), so byte shuffling tests the
decoder, not this validator. Feed the accumulator the same receipt multiset in several permutations
and assert identical `ThermalBatchReceiptTotals` and identical verdict. The `BTreeMap`-keyed fold is
order-independent by construction; the test pins that property against future refactors.
Test: `::aggregate_validation_verdict_is_input_order_independent`.

### V10 — Save/resume equivalence (negative control 6)
`export → import → export` produces identical bytes and an identical `physical_state_digest`;
continuing `k` ticks from the resumed state yields digests identical to the uninterrupted `2k`-tick
run.
Test: `::save_resume_equivalence_under_aggregate_validation`.

### V11 — Digest and version neutrality (Stage 4 gate)
`physical_state_digest` and `history_digest` byte-identical to the pre-change values for the fixture;
`THERMAL_SECTION_MAJOR`, `MATERIAL_SURFACE_SECTION_MAJOR`, and `CURRENT_DIGEST_SCHEMA_VERSION`
provably untouched by `git diff`;
`crates/causafera-runtime/tests/thermal_persistence.rs::thermal_persistence_literal_version_contract`
unmodified and passing.
Test: `::digest_and_section_versions_are_unchanged` plus the existing pinning test.

### V12 — Existing thermal contracts and full CI remain green
The V1-V23 contracts of `plans/conserved-thermal-energy-carrier.md:599-766` and the V1-V15 contracts
of `plans/thermal-material-surface-coupling.md`, plus the full CI gate:

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

## Benchmark plan

- **Metric:** wall time of `RuntimeState::import_snapshot`, before and after this change, on the
  same engine-produced snapshot.
- **Workload:** an engine-produced snapshot with the largest `N` (batch count), `V` (total cell
  count), and `Σ|receipts|` this repository can currently produce reproducibly. Record all three
  figures alongside the timings, because the added cost is `O(V + Σ|receipts| + N)` and a timing
  without those three numbers is uninterpretable. The workload is generated by
  `production_loop_config(seed)` (`crates/causafera-runtime/src/benchmark.rs:345`) plus
  `runtime.run_ticks(k)` for the largest `k` that keeps the benchmark acceptably fast.
- **Method:** a small, additive harness in the runtime crate. Add
  `pub fn measure_import_wall_time(data: &RuntimeSnapshotData, iterations: u32) -> Result<u128,
  RuntimeError>` to `crates/causafera-runtime/src/benchmark.rs`, which rejects zero iterations,
  calls `RuntimeState::import_snapshot` in a loop with `std::hint::black_box` on both input and
  result, and returns elapsed nanoseconds divided by `iterations`. Add a binary example
  `crates/causafera-runtime/examples/thermal_import_benchmark.rs` that:
  1. Builds a `Runtime` from `production_loop_config(seed)`.
  2. Warms up with `WARMUP_TICKS` ticks.
  3. Runs `MEASUREMENT_TICKS` ticks and exports a snapshot.
  4. Calls `measure_import_wall_time(&snapshot, INNER_ITERATIONS)` 10 times and prints one JSON line
     with mean, median, sample standard deviation, min, max, repetition and inner-iteration counts,
     workload dimensions, seed, toolchain, CPU model, and deterministic-mode flag.
  Warm-up, repetition count, inner iteration count, and seed follow
  `docs/performance/benchmarks.md`. Run the same example on the pre-change and post-change code
  against the same generated snapshot shape; the only implementation difference is the validator.
- **Command (exact):**
  ```bash
  cargo run -p causafera-runtime --example thermal_import_benchmark --release
  ```
- **Acceptance:** the TODO requires the added summation cost to be benchmarked against import time
  before acceptance. There is no pre-set budget; the requirement is a measured, reproducible,
  recorded delta.
- **Context for reading the result:** `docs/simulation/long-run-experiments.md:66-89` documents that
  `thermal_receipts` / `thermal_conservation_receipts` grow unboundedly with run length and that
  `physical_state_digest` already pays for it. This validator's `Σ|receipts|` term rides the same
  unbounded growth. That growth is a named, open, out-of-scope follow-up; this plan reports its
  measured contribution honestly and does not claim to fix or to be unaffected by it.
- **No number appears in this section until Stage 5 measures it.** INV-018 makes estimates
  inadmissible.

**Measurements:**

- **Machine:** local Linux development host, AMD Ryzen 9 7950X3D 16-Core Processor.
- **Toolchain:** `rustc 1.97.1 (8bab26f4f 2026-07-14)`.
- **Command:** `cargo run -p causafera-runtime --example thermal_import_benchmark --release`.
- **Workload:** `production_loop_config(seed=2026)` with `WARMUP_TICKS=4`,
  `MEASUREMENT_TICKS=32`, `REPETITIONS=10`, `INNER_ITERATIONS=100`; produced snapshot shape:
  `batch_count=36`, `total_cells=27`, `total_transfer_receipts=668`.
- **Pre-change (Wave 1 commit `9ddba30`, same harness):** `{"mean_import_ns":1052697.1,"median_import_ns":942363,"stddev_import_ns":177317.41585795922,"min_import_ns":925985,"max_import_ns":1405032,"repetitions":10,"inner_iterations":100,"batch_count":36,"total_cells":27,"total_transfer_receipts":668,"measurement_ticks":32,"seed":2026,"toolchain":"rustc 1.97.1 (8bab26f4f 2026-07-14)","hardware":"AMD Ryzen 9 7950X3D 16-Core Processor","deterministic_mode":true}`.
- **Post-change (current working tree, same harness):** `{"mean_import_ns":955733.4,"median_import_ns":949907,"stddev_import_ns":24938.951885665836,"min_import_ns":936455,"max_import_ns":1021484,"repetitions":10,"inner_iterations":100,"batch_count":36,"total_cells":27,"total_transfer_receipts":668,"measurement_ticks":32,"seed":2026,"toolchain":"rustc 1.97.1 (8bab26f4f 2026-07-14)","hardware":"AMD Ryzen 9 7950X3D 16-Core Processor","deterministic_mode":true}`.
- **Observed delta:** mean `-96963.7 ns` (`-9.21%`); median `+7544 ns` (`+0.80%`). No
  regression threshold was defined, so these measurements are recorded without a performance
  pass/fail claim.

## Determinism impact

None on simulation execution. No new `System`, no `register_system` call, no RNG stream, no scheduler
phase, no change to `runtime_system_registrations()`.

The validator is deterministic and side-effect free by construction:
- All accumulation is a fold into a `BTreeMap<TraceId, _>`, so it is independent of receipt
  encounter order.
- All sums are `i128` with `checked_add`; overflow returns `RuntimeError::InvalidSnapshot`, never
  wraps, so the verdict does not depend on platform integer behaviour.
- No `HashMap` iteration, no floating point, no time, no locale, no pointer identity.
- The batch walk iterates `BTreeMap` key order, which is `TraceId`-ascending and equals commit order.
- It writes nothing, so it cannot perturb any subsequent digest.

Digests are unchanged for every already-valid snapshot; V11 gates this.

## Memory impact

One transient `BTreeMap<TraceId, ThermalBatchReceiptTotals>` during import, `N` entries of three
`i128` each (48 bytes plus map overhead per batch), dropped when `import_snapshot` returns. The I6
anchor sums accumulate into three `i128` scalars over already-resident data and allocate nothing.
No field is added to `RuntimeState`, `RuntimeSnapshotData`, or any snapshot record. Steady-state
memory is unchanged.

## Observer impact

None. No delta shape, no field, no protocol message, no schema version, no UI change. The observer
read model is not touched.

## Explanation impact

None. No claim schema is added or modified. The Explanation Engine's existing
`THERMAL_CARRIER_CONSERVATION_SCHEMA` reads the same receipts it reads today, with the same values.

## Persistence impact

No format change. No section major bump (`THERMAL_SECTION_MAJOR` stays 2,
`MATERIAL_SURFACE_SECTION_MAJOR` stays 3), no digest schema bump
(`CURRENT_DIGEST_SCHEMA_VERSION` stays 6), no encode or decode change.

The only behavioural change is reader strictness: snapshots whose aggregate literals diverge from
their real summed state are now rejected with `RuntimeError::InvalidSnapshot` instead of being
installed. Any snapshot that was genuinely valid still imports, byte-for-byte, to an identical
digest. Rejection is fail-closed and occurs before authoritative installation, consistent with the
existing thermal import validation surface.

## Cross-domain effects

None. The validation reads three buckets that already exist and are already conserved together. It
introduces no coupling between domains, no new carrier, and no new dependency between crates —
`crates/causafera-domains` is not modified. The one cross-*section* dependency (thermal receipts
validated against `material_surfaces` from a different snapshot section) already exists at
`crates/causafera-runtime/src/runtime.rs:1150-1171` and is the reason the validator is placed in the
post-assembly `&self` step rather than inside `import_thermal_snapshot`.

## Risks

### RISK-1 — "The injection offset" (highest risk in this change)

`receipt.pre_state` is **post**-injection: `accept_injections` writes
`pre_state = committed + accepted` at `crates/causafera-domains/src/thermal/injection.rs:82`, and
that value is bound as `logical_pre_state` at
`crates/causafera-domains/src/thermal/receipts.rs:34-38`. But `conservation_receipt` computes
`total_cell_energy_before` from `committed`, the **pre**-injection map, at
`crates/causafera-domains/src/thermal/receipts.rs:107` (passed from
`crates/causafera-domains/src/thermal/evolution.rs:53`).

An implementer will naturally write `C^+ - C^- == Σ(post_state - pre_state)`, watch it fail on every
snapshot with a live reservoir, and "fix" it by gating the check on empty reservoirs, or by skipping
I2 when any reservoir participated, or by falling back to the residual. The result is green CI with
cells still unbound — exactly the failure this plan exists to prevent.

**Mitigation.** The `+ Σ accepted_injection` term in I2 is mandatory and load-bearing. Any
conditional weakening of I2 — an `if reservoirs.is_empty()` guard, an early `continue`, a
residual-only fallback, or a tolerance — is forbidden by this plan. The Stage 1 multi-batch fixture
must contain a live reservoir with residual budget (V1), and V4 must show I2 holding **with** the
injection term on that fixture before any implementation is written. A reviewer seeing I2 weakened
must reject the change.

### RISK-2 — "Vacuous fixtures"

Writing the negative controls against a one-batch snapshot with exhausted reservoir budgets and
`material_exchange_fraction = 0`. Then I5 has no links to test, the `B` and `M` deltas are identically
zero, and only I6 is ever exercised — so the actual reported hole (a non-latest batch, an untouched
cell) is never touched, and every control passes for the wrong reason.

**Mitigation.** Stage 1's fixture assertion (V1) is a hard gate that must pass **before a single line
of validation is written**: `N >= 3`; at least one reservoir with residual budget; at least one
surface with non-zero retained energy; at least one cell in no receipt of any batch. If the fixture
cannot satisfy all four, fix the fixture first. V7 and V8 additionally assert *which* identity
rejected, so a control cannot pass via I6 when it is supposed to prove I2 or I5.

### RISK-3 — Implicit zero-sum face-flux assumption

I2 combined with the already-enforced I4 implicitly asserts that face fluxes sum to zero across each
batch. This should hold under `ThermalBoundaryBehavior::NoFluxOutsideActiveRegion`
(`crates/causafera-domains/src/thermal/records.rs:124-126`), because face records are created
pairwise inside the active region at `crates/causafera-domains/src/thermal/diffusion.rs:82-93` under
the `key >= neighbor` skip at `diffusion.rs:64-66`, and out-of-region neighbours produce a
`ThermalBoundaryRecord` carrying no `signed_flux` (`diffusion.rs:56-62`). A one-sided boundary flux
record would make I2 fire falsely on legitimate snapshots and look like a data bug.

**Mitigation.** Verified empirically in Stage 1 by V2, not assumed from the construction argument
above.

### RISK-4 — I5 assumes only the Physics batch mutates the three buckets

The chain identity is unsound if anything outside `ThermalEvolutionSystem` mutates cell energy,
reservoir budgets, or material retained energy between Physics batches.
`crates/causafera-runtime/src/material_surface.rs` rewrites whole `MaterialSurface` values during the
Action phase (`material_surface.rs:1154`, `commit_material_surface_contact_events`) and during the
Mana phase (`material_surface.rs:1253`, `apply_local_mana_material_surface_transition`). Reading
those paths, both carry `thermal` through unchanged — the Action path via
`thermal: before_surface.thermal` in `crates/causafera-runtime/src/actors/action.rs:131`, the Mana
path via `let mut after = proposal.before` followed by mutations confined to `gate`, `condition`, and
`last_transition`. If either ever reset `retained_energy`, I5 would fire on legitimate snapshots.

**Mitigation.** Proven by assertion in Stage 1 (V3), covering all three buckets, before I5 is
enforced. Reading the code is not sufficient evidence for a load-bearing invariant.

### Other risks

| Risk | Mitigation |
|------|------------|
| Per-batch full-field summation sneaks in, making import `O(N·V)` | Architecture forbids it explicitly; the terminal anchor plus `O(N)` chain walk is the only permitted shape; Stage 5 benchmarks the real cost. |
| Silent `i128` wrap producing a false accept | Every sum uses `checked_add`; every overflow returns `RuntimeError::InvalidSnapshot` with a distinct message. |
| An accidental section-major or digest-schema bump | Stage 4 gate V11 checks `git diff` on the constants and requires `thermal_persistence_literal_version_contract` to be unmodified; digest bytes must be identical. |
| Growing `runtime.rs` further (INV-042) | The validator is a new named sibling module; `runtime.rs` gains one accumulator fold, one signature parameter, and one call. |
| Scope creep into `TODO-THERMAL-007`/`008` or receipt compaction | Explicit Non-goals with their concrete blockers named. |
| The coordinated final-state-plus-chain limitation being mistaken for full coverage | Named in Proposed architecture, and required to be named in the `TODO-THERMAL-006` Resolution paragraph and in `CHANGELOG.md`. |
| Controls passing because a *different* existing check rejected first | V7 and V8 assert the specific rejection message, so each control proves the identity it targets. |

## Documentation changes

- `docs/development/todo-backlog.md:468-483` — close `TODO-THERMAL-006` with a Resolution paragraph
  naming the six identities, the induction structure, the measured import-time delta, and the
  coordinated final-state-plus-chain limitation scoped to `SECURITY.md:20-24`.
- `CHANGELOG.md` — aggregate conservation cross-validation on snapshot import; explicitly state that
  no section major and no digest schema version changed.
- `docs/ontology/causal-carriers.md` — note that the thermal carrier's persisted aggregate totals are
  bound to materialized state on import.
- `docs/ontology/domain-coverage-matrix.md` — Energy row and Simulation-runtime row, import-integrity
  evidence only. No maturity level rises.
- `PLANS.md` — move from Draft Plans to Completed Detailed Development Plans on completion.
- No change to `docs/rfc/RFC-PERSIST-001.md`: no format or schema version changes, so its
  section-major and digest-schema history is untouched. Confirm this at Stage 5 rather than assuming.

## TODO changes

- Close `TODO-THERMAL-006` with the Resolution paragraph described above.
- Do not modify `TODO-THERMAL-001`, `003`, `004`, `005`, `007`, or `008`. They remain independently
  deferred with their existing scope.
- Open no new TODO. The coordinated final-state-plus-chain limitation is documented as scoped inside the
  existing `SECURITY.md` carve-out, not as a new work item; the receipt retention/compaction gap is
  already tracked via `TODO-PERF-002` / `TODO-PERF-003` and
  `docs/simulation/long-run-experiments.md:66-89`.

## Decision log

- **2026-07-28 — Source line numbers verified against working-tree source.** Every file:line
  reference in this plan was read from current source on this date, not carried from prior analysis.
  Corrections applied relative to the originating consultation:
  - Reservoir identity I1 is at `crates/causafera-runtime/src/runtime.rs:2748-2772`, not 2745-2770.
  - The `import_thermal_snapshot` receipt loop begins at
    `crates/causafera-runtime/src/runtime.rs:2601`, not 2596; the free function itself starts at
    `runtime.rs:2427`.
  - `total_cell_energy_before` is computed in `crates/causafera-domains/src/thermal/receipts.rs:107`
    inside `conservation_receipt` (`receipts.rs:98-155`), **not** in `evolution.rs:29-59`.
    `evolution.rs:29-59` is the `propose_evolution` body that calls `accept_injections` (lines 29-30)
    and `conservation_receipt` (lines 51-59), passing the pre-injection `&committed` at line 53. The
    original citation pointed at the call site, not the computation.
  - `preflight_faces` spans `crates/causafera-domains/src/thermal/diffusion.rs:18-167`; the
    `materials_after` full-population inserts are at `diffusion.rs:117` (unchanged) and
    `diffusion.rs:127` (exchanged), both as originally stated; `diffusion.rs:42-49` is the
    material-site guard loop, not the whole function.
  - `crates/causafera-runtime/src/material_surface.rs:1253` runs in `Phase::Mana`
    (`apply_local_mana_material_surface_transition`), not the Action phase; `material_surface.rs:1154`
    is the Action-phase path (`commit_material_surface_contact_events`). RISK-4 covers both.
  - The latest-batch material bind is at `crates/causafera-runtime/src/runtime.rs:1150-1171`, inside
    `validate_snapshot_references` at `runtime.rs:1090`.
  - INV-018's actual title is "Scale claims require reproducible benchmarks"
    (`docs/architecture/invariants.md:75`), not a wording about estimates being inadmissible; the
    substantive requirement is unchanged and INV-016 and INV-038 were added as directly relevant.
  Confirmed unchanged and correct: `runtime.rs:2685` (signed-flux equation),
  `runtime.rs:2091-2096` (digest hashes all six literals), `runtime.rs` total length 4250,
  `injection.rs:82`, `snapshot_sections.rs:561`, `config.rs:87`, `RFC-GEO-002.md:104`,
  `candidate-ledger.md:30-39`, `long-run-experiments.md:66-89`, `SECURITY.md:20-24`, and
  `thermal_persistence.rs:412-426`.
- **2026-07-28 — Material bucket is in scope, and is not a second TODO.** `TODO-THERMAL-006`'s text
  names only cells and reservoirs because it predates `TODO-THERMAL-002`, which added the third
  conserved bucket. `total_material_retained_before/after` sum **all** material sites, not only
  exchanged ones — `crates/causafera-domains/src/thermal/diffusion.rs:117` inserts into
  `materials_after` on the unchanged branch and `diffusion.rs:127` on the exchanged branch, so `M` is
  a full-population sum carrying the identical untouched-member hole that `C` has. The existing
  latest-batch material check at `crates/causafera-runtime/src/runtime.rs:1150-1171` only binds
  surfaces that *have* a receipt. Covering `M` via I3/I3a/I5/I6 is the same equation applied to a
  third bucket, not a scope expansion, and omitting it would leave the plan's own I4 residual check
  trivially satisfiable through the material term.
- **2026-07-28 — No version bump.** `THERMAL_SECTION_MAJOR` (2),
  `MATERIAL_SURFACE_SECTION_MAJOR` (3), and `CURRENT_DIGEST_SCHEMA_VERSION` (6) are unchanged.
  Three independent reasons: (a) zero encoded byte changes — no field added, removed, or reordered;
  (b) `physical_state_digest` already hashes all six aggregate literals at
  `crates/causafera-runtime/src/runtime.rs:2091-2096`, so digests over any valid snapshot are
  bit-identical before and after; (c) a section major encodes byte layout for readers, and reader
  strictness is not layout — bumping would falsely reject snapshots that are in fact valid. Stage 4
  makes "digest unchanged" an executable gate rather than an assertion.
- **2026-07-28 — Validator placement.** New sibling module
  `crates/causafera-runtime/src/thermal_conservation_validation.rs` per INV-042 (`runtime.rs` is
  4250 lines). The I6 material anchor cannot live in the free function `import_thermal_snapshot`
  (`runtime.rs:2427`), because `material_surfaces` is decoded from a different snapshot section and
  installed later (`import_material_surfaces`, `runtime.rs:2946`, called at `runtime.rs:1031`).
  The whole aggregate validator therefore runs from the post-assembly `&self` method
  `validate_snapshot_references` (`runtime.rs:1090`), which already hosts the latest-batch material
  bind. Receipt-side accumulation folds into the existing loop at `runtime.rs:2601` to keep the cost
  at `O(Σ|receipts|)` on a traversal that already happens.
- **2026-07-28 — The accumulator is threaded, not recomputed.** `validate_snapshot_references` takes
  `&self` only, so the receipt-side accumulator cannot reach it implicitly. Rather than adding a
  field to `RuntimeState` (which would make a transient import artifact into authoritative state) or
  recomputing the fold from `self.thermal_receipts` (a second pass), `import_thermal_snapshot` returns
  the accumulator as a seventh tuple element and `validate_snapshot_references` gains one parameter.
  This keeps the validator in the single method the placement decision requires while preserving the
  single-pass cost model.
- **2026-07-28 — Backward reconstruction rejected as unnecessary.** The terminal anchor (I6) plus
  downward induction through I1/I2/I3 and I5 determines all `6N` literals from final state and
  per-receipt data. Storing or reconstructing historical per-cell field state would cost `O(N·V)`
  memory for a strictly weaker guarantee, since the aggregate literals are scalar totals and totals
  are exactly what the chain already determines.
- **2026-07-28 — Coordinated final-state-plus-chain limitation accepted, not chased.** A uniform
  aggregate offset alone is rejected by I6. The undetected case requires shifting the materialized
  final field and every batch's `C_b^-`/`C_b^+` by the same `+Δ`; all difference and chain identities
  then remain true because `C_1^-`, the bootstrap total, has no independent persisted anchor. This
  sits inside the pre-alpha untrusted-snapshot carve-out at `SECURITY.md:20-24`. Stage 1 evaluates
  exactly one escalation: if the thermal bootstrap event's committed effects already encode per-cell
  initial energies, anchoring `C_1^-` against them closes the induction at the bottom — taken only if
  it is a few lines against existing committed state, otherwise documented and left open.
- **2026-07-28 — `TODO-THERMAL-007` and `008` deliberately not bundled.** `007` requires a user
  decision between expansion, damage accumulation, and phase change — three materially different
  response models, hence three different plans. `008` carries the opposite persistence posture: it
  forces `THERMAL_SECTION_MAJOR` 2→3 and `CURRENT_DIGEST_SCHEMA_VERSION` 6→7 and pulls `f64`
  `Material::thermal_conductivity`/`specific_heat` into an integer fixed-point domain, requiring its
  own conversion-and-determinism policy and a per-material re-proof of the
  `6 * transfer_fraction + material_exchange_fraction <= scale` bound. Bundling either would make
  this plan's no-version-bump gate unstateable.
- **2026-07-28 — `TODO-THERMAL-001` is blocked, not deprioritized.** `docs/rfc/RFC-GEO-002.md:104`
  defers cross-chart transforms and atlas generation; no `ChartTransform` or seam-registry type
  exists; `plans/candidate-ledger.md:30-39` names the four concrete blockers; production bootstrap
  creates a single chart (`crates/causafera-runtime/src/config.rs:87`).
- **2026-07-28 — Order-independence is tested at the accumulator, not the byte stream.** Decode
  already enforces strict receipt and trace ordering (`crates/causafera-runtime/src/runtime.rs:2591-2600`
  and `runtime.rs:2741-2747`), so permuting encoded bytes would exercise the decoder rather than this
  validator and would produce a misleading green. V9 permutes the receipt multiset fed to the fold.
- **2026-07-28 — Stage 2 gets no checkpoint commit.** `PLANS.md` forbids checkpointing a RED state.
  The negative controls are authored in Stage 2, their failure-to-reject is recorded as a transcript
  in Progress as the Stage 2 gate evidence, and the control tests are committed together with the
  Stage 3 implementation as one atomic RED→GREEN checkpoint.
- **2026-07-28 — Momus review: plan rejected on two executability blockers, corrected.**
  1. The original Stage 1 RISK-4 assertion demanded per-phase snapshots of `material_surfaces`, but
     the public `Runtime` API exposes only whole-tick execution/export. Investigation of
     `crates/causafera-runtime/src/runtime.rs` and the trace-store layout showed that the correct
     executable proxy is to inspect `snapshot.traces.events` after `export_snapshot`: every mutation
     anchor for the three buckets already resolves to a `Phase::Physics` event (or Lifecycle for
     reservoir bootstrap). Stage 1 item 4 and V3 were rewritten to use this trace-inspection test,
     mirroring the existing `validate_snapshot_references` checks at `runtime.rs:1179-1257`.
  2. The original Benchmark plan named the checked-in `performance_baseline` harness from
     `plans/performance-baseline-and-digest-cost.md`, but that harness has no import-snapshot
     measurement mode. Investigation of `crates/causafera-runtime/src/benchmark.rs` showed the
     existing `std::time::Instant` / `mean_tick_elapsed_ns` pattern can be extended with a small
     helper and a new example binary. The Benchmark plan, Stage 5 item 1, and the Wave 4 file
     allowlist were rewritten to add `measure_import_wall_time` in `benchmark.rs` and a new example
     `thermal_import_benchmark.rs`, with exact command
     `cargo run -p causafera-runtime --example thermal_import_benchmark --release`.
- **2026-07-28 — Momus re-review: plan rejected on two scope/contradiction blockers, corrected.**
  1. V5 originally claimed all 24 forged aggregate-total cases were currently accepted, but the
     reservoir identity at `runtime.rs:2748-2771` already rejects forged `total_reservoir_budget_*`
     literals. V5 was narrowed to the four actually unanchored literals (`total_cell_energy_*` and
     `total_material_retained_*`); the already-enforced reservoir totals are noted as a confirming
     observation, not a RED control.
  2. V3 originally required every `thermal_fields[*].last_change` trace to be `Phase::Physics`, but
     untouched cells retain their bootstrap trace, which is `Phase::Lifecycle`
     (`causafera-domains/src/thermal/field.rs:46` and `crates/causafera-runtime/src/bootstrap.rs:456-458`).
     V3 and Stage 1 item 4 were relaxed to permit Lifecycle traces that equal the bootstrap trace,
     while requiring every *post-bootstrap* mutation anchor to be `Phase::Physics`.
- **2026-07-28 — `C_1^-` escalation evaluated and declined.** The thermal field bootstrap event
  (`THERMAL_FIELD_BOOTSTRAP_EVENT_KIND`) commits a generic stage effect with fingerprinted
  before/after values (`bootstrap.rs:323-333`), not the actual per-cell energy total. The initial
  energies exist only in `ThermalField::energy` and `ThermalField::last_change_before`. Anchoring
  `C_1^-` would require a new persisted scalar, a new event, or a new effect encoding — more than
  the "few lines against already-committed state" threshold. The coordinated
  final-state-plus-chain limitation remains inside the `SECURITY.md:20-24` carve-out.
- **2026-07-28 — Independent verification tightened tests, benchmark evidence, and limitation
  wording.** V5 now exercises both `+1` and `-1` across all eight literal/batch combinations; V7
  asserts the I2 rejection and V8 asserts the I5 rejection. Exact integration controls isolate I3
  and I3a; a direct validator unit control isolates aggregate I4 after I2/I3 pass, while a separate
  import control covers the persisted non-zero residual field. Validator unit tests and aggregate
  integration tests were split into cohesive sibling
  modules so every changed Rust file remains below the 250-pure-LOC ceiling, and the missing-terminal
  receipt path now fails closed instead of relying on `expect`. The import benchmark runs the same
  10-by-100 statistical harness on baseline commit `9ddba30` and the post-change tree, records CPU,
  mean, median, sample standard deviation, min and max, and makes no performance pass/fail claim
  because no regression threshold was defined. Documentation now states precisely that I6 rejects a
  uniform aggregate-only offset; the remaining carve-out requires a coordinated shift of final field
  state and the entire aggregate chain.
- **2026-07-28 — This plan's V-numbering is local.** The acceptance criterion's "V1-V23 thermal
  contracts" refers to `plans/conserved-thermal-energy-carrier.md:599-766`. This plan restarts at V1
  following the convention of `plans/thermal-material-surface-coupling.md`; V12 requires the carrier
  plan's V1-V23 to remain green.

## Progress

Accepted and implemented. The implementation, Rust, frontend, entry-point audit, and documentation
gates are green. `node tools/audit/run-source-tests.mjs` is environment-blocked: 9 of 14 tests pass,
while five direct-Rust/LSP cases cannot start because the pinned Rust 1.97.1 toolchain has no
`rust-analyzer` component. This unavailable check is not reported as passing. Checkpoint commits were
not created because the active environment policy requires explicit authorization for commits;
uncommitted state and evidence are recorded below.

### Commit strategy

Four atomic checkpoint commits, one per green wave. Per `AGENTS.md` and `PLANS.md`: inspect
`git status` and both staged and unstaged diffs before each checkpoint, stage only the wave's
explicit file allowlist by path (never `git add .`), rerun the wave's focused verification, and
record the commit hash and the commands that passed below. Never checkpoint a RED, uncompilable, or
partially integrated state. Never begin a wave while a completed prior wave exists only as
uncommitted working-tree state.

**Wave 1 — Stage 1 (observational harness; tests only, no production change)**
- Commit hash: `9ddba30`
- File allowlist:
  - `crates/causafera-runtime/tests/thermal_conservation_aggregates.rs`
- Verifying commands: `cargo test -p causafera-runtime --test thermal_conservation_aggregates`;
  `cargo fmt --all -- --check`;
  `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- Gate evidence: V1, V2, V3, V4 pass; `C_1^-` escalation decision recorded in the Decision log.
- Result: **GREEN**

**Wave 2 — Stage 2 + Stage 3 (RED controls and the implementation that greens them, one atomic
RED→GREEN checkpoint)**
- Commit hash: `_not created; commits require explicit authorization in this environment_`
- File allowlist:
  - `crates/causafera-runtime/src/thermal_conservation_validation.rs` (new)
  - `crates/causafera-runtime/src/lib.rs`
  - `crates/causafera-runtime/src/runtime.rs`
  - `crates/causafera-runtime/tests/thermal_conservation_aggregates.rs`
- Verifying commands: `cargo fmt --all -- --check`;
  `cargo clippy --workspace --all-targets --all-features -- -D warnings`;
  `cargo test --workspace --all-features`; `cargo test --workspace --no-default-features`;
  `cargo run -p xtask -- ci`; `git diff --check`
- Stage 2 gate evidence (transcript showing controls 1-4 fail to reject on pre-change code):
  `cargo test -p causafera-runtime --test thermal_conservation_aggregates` reports 4 failures:
  - `runtime_import_rejects_forged_cell_and_material_aggregate_totals`: `forging cell_energy_before in batch 0 must be rejected`;
  - `runtime_import_rejects_untouched_cell_energy_tampering`: `tampering an unbound cell energy must be rejected`;
  - `runtime_import_rejects_coordinated_untouched_cell_and_terminal_total_forgery`: `coordinated untouched-cell + terminal total forgery must be rejected`;
  - `runtime_import_rejects_historical_batch_material_total_forgery`: `historical batch material total forgery must be rejected`.
  Reservoir-total forgery is already rejected (`reservoir_aggregate_totals_are_already_rejected` passes),
  confirming existing I1 enforcement.
- Stage 3 gate evidence (V5, V6, V7, V8 green): V5 exercises 16 `±1` perturbations across all eight
  literal/batch combinations; V6 rejects unbound-cell tampering; V7 asserts the exact I2 rejection;
  V8 asserts the exact I5 rejection. Additional exact integration controls isolate I3 and I3a; a
  direct validator unit test isolates aggregate I4. The integration target reports 16 passing tests
  after the modular split, and the validator unit module reports 5.
- Result: **GREEN**

**Wave 3 — Stage 4 (format and determinism neutrality)**
- Commit hash: `_not created; commits require explicit authorization in this environment_`
- File allowlist:
  - `crates/causafera-runtime/tests/thermal_conservation_aggregates.rs`
  - `crates/causafera-runtime/src/thermal_conservation_validation.rs` (accumulator unit tests)
- Verifying commands: `cargo test -p causafera-runtime --test thermal_conservation_aggregates`;
  `cargo test -p causafera-runtime thermal_conservation_validation::tests`;
  `cargo test -p causafera-runtime --test thermal_persistence`;
  `git diff -- crates/causafera-runtime/src/snapshot_sections.rs`;
  `git diff -- crates/causafera-runtime/tests/thermal_persistence.rs`;
  `cargo fmt --all -- --check`;
  `cargo clippy --workspace --all-targets --all-features -- -D warnings`;
  `cargo test --workspace --all-features`;
  `cargo test --workspace --no-default-features`;
  `cargo run -p xtask -- ci`;
  `git diff --check`
- Gate evidence: recorded pre/post `physical_state_digest` and `history_digest` pair identical:
  - `physical_state_digest`: `744fcbbdf76f0ce77a8126a8ece05f3b848dda2d9ba78f79c965f62921037869`
  - `history_digest`: `d3052aa5863d415cd1d0740c98597776002deaf9d497fd39026cf259fba70ff4`
  - Values recorded at Wave 1 commit `9ddba30` using a temporary worktree and reproduced identically after the Stage 3 changes.
  - V9 (`aggregate_validation_verdict_is_input_order_independent`) green; the accumulator-level order-independence is additionally unit-tested in `thermal_conservation_validation::tests::accumulator_is_order_independent`.
  - V10 (`save_resume_equivalence_under_aggregate_validation`) green.
  - V11 (`digest_and_section_versions_are_unchanged`) green; `THERMAL_SECTION_MAJOR`, `MATERIAL_SURFACE_SECTION_MAJOR`, and `CURRENT_DIGEST_SCHEMA_VERSION` are unchanged; `thermal_persistence_literal_version_contract` passes.
  - `git diff -- crates/causafera-runtime/src/snapshot_sections.rs` and `git diff -- crates/causafera-runtime/tests/thermal_persistence.rs` produced no output.
- Result: **GREEN**

**Wave 4 — Stage 5 (benchmark and documentation)**
- Commit hash: `_not created; commits require explicit authorization in this environment_`
- File allowlist:
  - `crates/causafera-runtime/src/benchmark.rs`
  - `crates/causafera-runtime/examples/thermal_import_benchmark.rs`
  - `docs/development/todo-backlog.md`
  - `CHANGELOG.md`
  - `docs/ontology/causal-carriers.md`
  - `docs/ontology/domain-coverage-matrix.md`
  - `PLANS.md`
  - `plans/thermal-conservation-aggregate-validation.md`
- Verifying commands: full CI gate as listed in Stage 5 and V12.
- Gate evidence: measured import wall time before/after recorded verbatim in the Benchmark plan
  section with workload, `N=36`, `V=27`, `Σ|receipts|=668`, CPU, toolchain, command, and
  mean/median/sample-standard-deviation over 10 repetitions of 100 imports;
  `TODO-THERMAL-006` Resolution paragraph names the coordinated final-state-plus-chain limitation.
  `node tools/audit/check-entry-points.mjs` passes; `node tools/audit/run-source-tests.mjs` reports
  9/14 pass and five environment-blocked tests because `rustup which rust-analyzer` fails for the
  pinned toolchain.
- Result: **GREEN except environment-blocked source-audit cases described above**
