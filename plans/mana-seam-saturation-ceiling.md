# Mana Seam Saturation Ceiling ExecPlan

**Status:** Accepted and implemented.

## Goal

Make `maximum_intensity` bound a cell fed across a chunk seam exactly as it bounds a cell fed from
inside its own chunk (`TODO-MANA-006`).

## Context

`ManaField::propose_evolution` computes each field's own diffusion locally via `diffuse_cell`, which
clamps `current - decay - outgoing + incoming` to `0..=maximum_intensity` in a single step at the end
(mana.rs:999-1045). `ManaFieldSet::propose_evolution` then runs `apply_boundary_exchange`, a
post-pass that delivers the share each seam-touching cell already subtracted as part of its own
`outgoing` into the neighbouring chunk's proposal. That delivery, in `apply_exchange_delta`
(mana.rs:899-953), adds the delta straight into `proposal.proposed[index]` with no ceiling of its
own: a cell fed from inside its chunk is bounded once, at the end of its own computation; a cell fed
across a seam is not bounded at all. Measured on two adjacent extent-3 chunks with every cell seeded
at a `maximum_intensity` of 1 000, 10 of 54 cells finish above it, worst 1 034 (3.4% over).

The TODO's own Evidence entry argued a plain clamp is not the fix, reasoning that discarding the
excess would destroy mana the giving cell already parted with and reopen the conservation defect
`TODO-MANA-002` closed, and that returning it to whichever giver fed a saturated cell needs an
ordering-independent rule when several givers share one receiver — a corner or edge cell of a chunk
can receive from as many as six distinct neighbouring chunks at once, since the 18-neighbour stencil's
edge offsets touch two axes at a time. That framing does not hold up:

- **The interior path does not refund givers when its own clamp engages.** `diffuse_cell` subtracts
  `outgoing` unconditionally and adds `incoming` unconditionally, then clamps the sum once. If that
  sum would exceed `maximum_intensity`, the excess is discarded and the neighbour that contributed
  `incoming` is not compensated. A seam fix that refunds givers would make a seam cell behave
  *differently* from an interior cell under saturation — the same seam-vs-interior asymmetry INV-037
  forbids, with the sign flipped.
- **The seam conservation test does not exercise the clamp.** `diffusion_alone_conserves_mana_across_a_seam`
  sets `decay = 0` and `maximum_intensity = i64::MAX / 4`, so the ceiling never engages in it; its own
  comment already names "the only sanctioned losses" as "decay and the clamp." `TODO-MANA-002`'s
  conservation defect was truncation loss (an undivided outgoing budget subtracted against truncated
  incoming shares, every cell, every tick); ceiling loss is a different, already-sanctioned mechanism
  that only engages when a field is actually saturated. A clamp does not reopen it.

So the Goal line — bound a seam-fed cell *exactly as* an interior-fed cell — is satisfied by giving
the seam delivery the same single end-of-computation clamp the interior path already has, not by a
refund/ledger scheme the interior path has no equivalent of. Addition is commutative, so summing every
seam delta into a cell (unclamped) before clamping once is order-independent by construction, which is
what dissolves the "several givers" problem: no individual giver is singled out for refusal.

## Relevant invariants

- INV-037 — geometry/containment is not physical; a chunk face carries no physical meaning, so a cell
  must be bounded the same way regardless of which side of a seam it sits on. Refunding seam givers
  but not interior givers would make the boundary observable through the ceiling, which this plan
  avoids by using one clamp rule for both.
- INV-038 — digests are equality/divergence anchors only; the Verification section below states
  digest changes as measured inequalities/equalities, never a distance claim.
- INV-039 — not applicable; no bootstrap change.

## Ontology domains affected

Mana only. `maximum_intensity` remains a stated property of the field (RFC-MANA-001); this plan makes
its enforcement point uniform, not new.

## Causal carriers affected

None. The stencil, response channels, diffusion, decay, the gate model, and which carriers
participate are all untouched, per the TODO's Out of Scope.

## Relevant documents

- `docs/development/todo-backlog.md` — `TODO-MANA-006`, `TODO-MANA-002` (conservation fix this
  entry's Evidence worried about reopening), `TODO-MANA-005` (provenance boundary the seam delivery's
  cause-tracking still routes through, untouched here).
- `docs/world/mana-topology.md` — the Deferred phenomena paragraph (lines 116-118) states this exact
  gap.
- `docs/rfc/RFC-MANA-001.md` — "All values saturate at a configured maximum," the field-model line
  this plan makes uniformly true.
- `docs/ontology/unresolved-assumptions.md` — line 49, "Two narrower questions remain."

## Current state

`apply_boundary_exchange` iterates every seam-touching source cell once per crossing stencil offset
and calls `apply_exchange_delta` immediately per delivery, mutating `proposal.proposed[index]` with no
bound. When a target cell already sits near `maximum_intensity` (from its own interior diffusion) and
receives one or more seam deltas, the sum can exceed the ceiling with nothing to stop it.

## Proposed architecture

Change `apply_exchange_delta`'s target-cell write from an unbounded add to a bounded add: read the
cell's current proposed value, add the delta, clamp the sum to `parameters.maximum_intensity`, and
write the clamped result back. Deltas are always non-negative (`diffusion_share` only ever returns a
non-negative share of a non-negative value), so this never engages the lower bound and never needs to
touch `0` clamping — only the ceiling matters here. Because addition is commutative and the clamp is
applied to the running total after every delivery, the final clamped value for a cell fed by multiple
givers is `min(v + d1 + d2 + ... + dn, max)` regardless of the order the givers are visited in —
exactly the closed form the interior path already computes in one step. `apply_boundary_exchange`'s
existing per-source, per-offset traversal order does not need to change; only the write it performs
per delivery does.

`apply_exchange_delta` needs `parameters.maximum_intensity` threaded in (it currently only receives
`proposals`, `chunk`, `index`, `delta`, `source_causes`, `target_last_change`); `apply_boundary_exchange`
already has `parameters` in scope and passes it through.

The existing `after == base` removal branch (mana.rs:920) and the causes-accumulation logic are
unchanged; they operate on the post-clamp `after` value exactly as before, so a cell whose clamped
result returns to its base value still drops its spurious change record, and `ManaEvolutionProposal::commit`'s
`committed_traces.len() != changes.len()` invariant still holds.

## Primitive vs emergent review

`maximum_intensity` is an existing stated bounded constant of the field model (RFC-MANA-001); this
plan makes an existing mechanism (the ceiling) apply uniformly to an existing pathway (seam delivery),
not a new primitive, not a new domain enum.

## Non-goals

- Refunding a giver whose contribution is truncated by the receiver's ceiling. Rejected in Context:
  the interior path has no equivalent mechanism, and adding one only for seams would make the seam
  behave differently from the interior, which is the opposite of INV-037.
- A persisted ledger of refused/truncated mana. Ceiling loss is a sanctioned loss the same way decay
  is; it needs no new authoritative state, provenance, snapshot encoding, or digest inclusion. Out of
  Scope already excludes "changing the ... gate model," and a ledger would be new state, not a gate.
- Changing the stencil, the response channels, decay, or the gate model (per the TODO's own Out of
  Scope).
- Changing the interior `diffuse_cell` clamp's own truncation behaviour when it engages under
  saturation — this plan makes the seam path match it, not the reverse.

## Implementation stages

**Wave 1 — failing repro test.**
Add a test in `crates/causafera-domains/src/mana.rs` reproducing the TODO's measured numbers: two
adjacent extent-3 `ManaField`s, every cell seeded at `maximum_intensity = 1_000`, decay/diffusion at
the module's standard test `parameters()`. Assert the pre-fix defect count/magnitude first (confirms
the repro matches the Evidence), then invert the assertion to "no cell exceeds 1 000" once the fix
lands.

**Wave 2 — fix.**
Thread `parameters.maximum_intensity` into `apply_exchange_delta`; clamp the post-add value there
before writing `proposal.proposed[index]`. Re-run the domain crate's mana test module; the new test
goes green and all existing mana tests (`a_seam_conducts_exactly_as_the_interior_does`,
`diffusion_alone_conserves_mana_across_a_seam`, `boundary_exchange_that_returns_to_start_emits_no_mana_change`,
provenance tests) stay green since none of them saturate the ceiling.

**Wave 3 — digest verification at production config.**
The TODO's own Evidence states the ceiling is unreached at the production `maximum_intensity`
(1 000 000): seed 7's field-wide total, 82 679, is 8.3% of the cap even concentrated in one cell. Run
the six-seed production suite and assert the physical digest is unchanged from pre-fix — this change
should be behaviourally inert at production scale and only visible in the constructed near-cap
scenario the TODO measured. If a digest moves, stop and re-examine before continuing (a moved digest
at production scale means the ceiling engages somewhere unmeasured, which is new information, not an
expected outcome of this fix).

**Wave 4 — documentation.**
Update `docs/world/mana-topology.md` (close the Deferred-phenomena gap at lines 116-118),
`docs/ontology/unresolved-assumptions.md` (line 49, narrow to one remaining question),
`docs/development/todo-backlog.md` (`TODO-MANA-006` → Completed, with a Resolution explaining the
Evidence's refund/ledger framing was a deferral rationale, not a design mandate, and stating why a
uniform clamp satisfies the Goal without reopening `TODO-MANA-002`), `CHANGELOG.md`,
`docs/roadmap/roadmap.md`, `docs/ontology/domain-coverage-matrix.md` (Mana row).

## Verification

- `cargo test --release -p causafera-domains` — mana module, including the new repro/regression test.
- `cargo test --release --workspace` — full workspace, zero failures.
- `cargo fmt --all -- --check` — clean.
- `cargo clippy --release --workspace --all-targets -- -D warnings` — clean.
- Replay determinism: existing snapshot round-trip and replay-identity tests continue to pass
  unmodified; this change does not touch RNG, ordering, or trace structure, only where a bound is
  applied to an already-computed sum.

## Benchmark plan / measured evidence

No performance claim; this changes a single scalar comparison per seam delivery, not asymptotic cost.
Behavioural evidence:

- Repro (Wave 1): two extent-3 chunks, every cell seeded at `maximum_intensity = 1 000` — pre-fix,
  reproduces the TODO's "10 of 54 cells over, worst 1 034" or documents the actual measured numbers if
  they differ from the TODO text, before the fix is written.
- Post-fix: same scenario, zero cells exceed 1 000.
- Production-scale digest check (Wave 3): six seeds, physical digest before/after, expected unchanged
  given the TODO's own headroom measurement (82 679 / 1 000 000 = 8.3% of cap concentrated).

## Determinism impact

Behaviourally inert at every configuration where the ceiling was never reached (all measured
production seeds, per the TODO's own Evidence and Wave 3's re-check). Only a world whose mana field
actually saturates near a seam changes: such a world's physical digest changes, because a proposed
intensity value changes, which is exactly the Determinism Requirements' own prediction ("any fix
changes intensities and therefore the physical digest of a saturated world"). Same-seed replay is
unaffected in either case — the fix is a pure function of already-committed inputs, with no RNG,
ordering, or trace-structure change.

## Memory impact

None; no new fields, no new persisted state, no snapshot format change.

## Observer impact

The TODO's Observer Implications noted the map's mana lens had no reliable upper bound because a seam
cell could exceed the field's own stated maximum. After this fix, no cell exceeds
`maximum_intensity`, so a colour ramp normalised against it is reliable across the whole drawn field,
including seams. No renderer code changes in this plan; the bound it can now rely on is a property of
the field, not a new feature to draw.

## Explanation impact

The TODO's Explanation Implications noted a saturation claim could not be made about a seam cell. That
claim is now available: a seam cell's proposed intensity is guaranteed `<= maximum_intensity`, the
same guarantee already held for an interior cell.

## Persistence impact

None.

## Cross-domain effects

None beyond mana.

## Risks

- If a future change raises the production `maximum_intensity` default or the field's typical
  populated total closer to it, the ceiling could begin engaging in ordinary worlds; at that point the
  loss this plan sanctions (truncation at the ceiling, same as the interior path) becomes visible in
  production digests rather than only in constructed near-cap tests. That is an expected, not a new,
  consequence of an intentionally bounded field and is already covered by the "clamp is a sanctioned
  loss" reasoning in Context.

## Documentation changes

- `docs/world/mana-topology.md` — close the Deferred-phenomena gap.
- `docs/ontology/unresolved-assumptions.md` — narrow the "Two narrower questions remain" line to one.
- `docs/development/todo-backlog.md` — `TODO-MANA-006` marked Completed with Resolution.
- `docs/ontology/domain-coverage-matrix.md` — Mana row.
- `CHANGELOG.md`, `docs/roadmap/roadmap.md`, `PLANS.md`.

## TODO changes

`TODO-MANA-006`: Pending → Completed.

## Decision log

- Rejected the refund-to-giver design the TODO's own Evidence gestured at: it would make a seam cell's
  saturation behaviour differ from an interior cell's, which is the seam-vs-interior asymmetry INV-037
  forbids (see Context and Non-goals).
- Rejected a persisted refused-mana ledger: ceiling loss is a sanctioned loss with clear precedent
  (decay, and the interior clamp's own pre-existing truncation when it engages); a ledger would add
  new authoritative state, provenance, and digest surface for a loss mechanism that needs none of
  those to be "accounted for rather than silently created or destroyed" — it is accounted for by being
  a stated, tested rule rather than an unbounded write.
- Confirmed via direct code reading (not assumption) that the interior path's own clamp already
  discards excess without refunding contributing neighbours, and that the seam conservation test does
  not exercise the clamp at all (`maximum_intensity = i64::MAX / 4`) — both checked before committing
  to the uniform-clamp design, per Advisor consultation.

## Progress

- Wave 1+2 (repro test + fix), checkpoint `b2aa76d`: `crates/causafera-domains/src/mana.rs`
  (`apply_exchange_delta` threads and clamps to `maximum_intensity`; call sites in
  `apply_boundary_exchange` and the two direct unit tests updated; new test
  `seam_delivery_never_exceeds_the_saturation_ceiling` reproduces the TODO's scenario). Confirmed
  red before the fix (cell 5 proposed 1004 against a ceiling of 1000), green after.
  Verified: `cargo test --release -p causafera-domains` (21 mana tests, 0 failed),
  `cargo test --release --workspace` (0 failed), `cargo fmt --all -- --check` (clean),
  `cargo clippy --release --workspace --all-targets -- -D warnings` (clean).
- Wave 3 (digest verification), no separate commit (verification only, no source change): compared
  `cargo run --release -p causafera-observer --example seed_variation` output between a throwaway
  `git worktree` at pre-fix `HEAD` (`1f32e99`) and the post-fix working tree. Physical digest, history
  digest, total mana, cell counts and every behaviour metric were byte-identical across all six
  standard seeds and both gate configurations (default and open) at the production `chunk_extent` of
  3; only compiler-log lines and expected run-to-run timing noise differed. Worktree removed after
  comparison.
- Wave 4 (documentation), checkpoint pending: `docs/world/mana-topology.md`,
  `docs/ontology/unresolved-assumptions.md`, `docs/development/todo-backlog.md`,
  `docs/ontology/domain-coverage-matrix.md`, `CHANGELOG.md`, `PLANS.md`. `docs/roadmap/roadmap.md` was
  checked and deliberately left unchanged: it does not mention any individual `TODO-MANA-*` entry
  (002/004/005/007 included), tracking only larger vertical slices, so adding one here would be an
  inconsistent one-off rather than following existing practice.
