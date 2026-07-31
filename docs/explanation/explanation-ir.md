# Explanation IR

Explanation IR is a structured intermediate representation for simulation explanations. It contains typed claims, evidence references, and causal traces. No human prose belongs in Explanation IR.

## Structure

Conceptual example:

```text
PhenomenonExplanation

subject:
    ConceptId 8172

classification:
    EMERGENT_SOCIAL_CATEGORY

display_label:
    local_lexeme 4412

origin:
    repeated perception of similar individuals

key_associations:
    rhythmic distal-hand movement
    South Canal residence
    bakery work

historical_transitions:
    physiological perception
    → occupational association
    → geographic identity
    → inherited social category

confidence:
    ...
```

## Required Capabilities

Explanation IR must support:

- typed claims;
- evidence references;
- causal trace references;
- confidence levels;
- alternative interpretations;
- temporal ranges;
- objective / local / agent perspectives.

## Perspectives

The same phenomenon may have different Explanation IR depending on perspective:

- **Objective:** Ground Truth and observer analytics
- **Local:** Community belief distribution
- **Agent-specific:** One subjective model
- **Historical:** Development over time

## Bounded material-surface loop claim

The active actor/material/mana slice supplies `MaterialSurfaceLoopClaim` as a live, read-only
Explanation input. It records typed condition bounds, field-total context, an observation window,
the actor-contact trace, an optional mana-effect trace, and optional in-world mana transition
values. Its deterministic claims distinguish supported evidence from explicit insufficiency and
include control/window schemas plus `MATERIAL_SURFACE_LOOP_MANA_TRANSITION_SCHEMA` (opaque schema
ID 14). That claim is `Supported` only when a mana-transition trace and its before/after values are
available; otherwise it is `Unknown`. Explanation may cite source-derived in-world ancestry, but
it never renders why an operator selected a source, its recipe identity, or its policy.

Verified cell-local coupling is represented separately by
`MATERIAL_SURFACE_LOOP_LOCAL_MANA_TRANSITION_SCHEMA` (opaque schema ID 15). It is supported only
by persisted local mana and gate-transition traces scoped to the same material surface. Its numeric
range is ordered local fixed-point evidence and never exposes recipe or operator metadata.

The live input reads only the bounded retained material-transition window. Retention is
deterministic and protects the newest mana-mediated transition when ordinary contacts would
otherwise fill the window; when contact ancestry is absent, the claim remains explicitly
insufficient rather than inventing a causal anchor.

## Bounded hydrology claims

Hydrology contributes ten opaque claim schemas, 20 through 29, over one chunk or
over the whole resident scope. Every one of them is a number with trace anchors;
none of them is a name.

- **20, 21** — the smallest and largest per-carrier storage in scope, as two
  claims rather than one range. `NumericClaimValue::Range` is `i64` on both ends
  and a water volume is a `u64`, so a range would silently lose the upper half of
  what a carrier can hold.
- **22** — the whole-scope storage total.
- **23** — the water-table elevation range, which *is* signed and therefore a
  genuine `Range`. It comes from `causafera_domains::groundwater_head_mm`, the
  same function the routing solver uses, so the claim describes the number that
  actually drove flow rather than a second implementation of the formula.
- **24, 25** — the latest applied forcing record's accepted and unmet volumes.
  Its evidence list is led by the producer's own origin trace: a forcing claim
  that stopped at the settlement would say what arrived without saying where from.
- **26, 27** — one transfer's accepted volume and what its limiter refused. The
  limiter claim is emitted at zero as well, because a bound that did not engage
  is evidence and its absence would make "nothing was refused" and "nothing was
  asked" the same answer.
- **28** — the exact conservation residual of the latest retained batch.
- **29** — boundary export.

Exact `u64` volumes travel as `Ratio { volume, 1 }`. `NumericClaimValue` is not
widened to carry them: widening it would change `explanation.proto`, the Rust
codec, and the TypeScript codec at once, for a quantity the existing variant
already carries losslessly. A denominator of one is not a fraction standing in
for a number — it is the number, in the one variant whose numerator is a `u64`.

Insufficiency is the answer wherever evidence is missing, never a narrowed one:

- a chunk that is not resident, and a session that never enabled hydrology,
  answer `Unknown` with no traces and zero confidence rather than erroring;
- a whole-scope total above `u64::MAX` answers `Unknown` while the per-carrier
  bounds beside it stay supported — the measurement exists and this schema cannot
  carry it, and saying so is the only honest answer available;
- a forcing record whose batch retention has evicted answers `Unknown`, because
  eviction removes typed detail and a number nothing supports is worse than none;
  and
- a nonzero conservation residual answers `Unknown`. A committed batch closes
  exactly, so a nonzero residual does not mean "a slightly different measurement",
  it means this batch is not committed evidence. Thermal's equivalent claim errors
  out instead; hydrology's contract is insufficiency.

A transfer whose requested, accepted, and unaccepted volumes do not close is a
different matter and *is* an error. Those three come from one receipt, so a caller
that can present them not closing has built the claim from parts of different
transfers.

## Related Documents

- `docs/explanation/architecture.md` - Explanation pipeline
- `docs/explanation/deterministic-rendering.md` - Rendering IR to text
- `docs/explanation/causal-summaries.md` - Causal trace summaries
- `docs/explanation/confidence.md` - Confidence representation
