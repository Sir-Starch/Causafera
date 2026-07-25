# Mana Topology

Mana is a local information-sensitive physical field. Phase 17 establishes its minimum deterministic topology without claiming final mana physics.

## Authoritative state

`causafera-domains::mana::ManaField` stores a chunk coordinate, a cubic extent of at most `CHUNK_SIZE`, row-major fixed-point intensity, the last committed causal trace and pre-change intensity per cell, and the latest incorporated simulation tick. Human names and classifications are absent.

## Physical inputs

The field accepts bounded `PhysicalPatternSample` batches. Each sample supplies an opaque fingerprint of canonical carrier structure, local coordinate, observation tick, magnitude, stable source ordinal, and causal trace. Integrations may derive fingerprints from acoustic wave structure, repeated geometry, motion, physical glyph arrangements, material state, or other objective carriers. They may not hash words, beliefs, concepts, practice meanings, document genres, or observer labels into a fingerprint.

## Minimal response

Canonical same-fingerprint samples increase response through repeated occurrence, regular temporal intervals, simultaneous occurrence, and repeated placement at distinct coordinates. Magnitude scales local injection. A fixed eighteen-neighbour stencil then applies diffusion and decay, with non-negative saturation. The six face neighbours carry weight 2 and the twelve edge neighbours weight 1, the smallest exact-integer weighting that is isotropic to fourth order, so a source spreads as a sphere rather than along the lattice axes and no floating point enters the authoritative path. A cell gives each neighbour an equal share and loses exactly the sum of the shares delivered, so diffusion moves mana without destroying it. A chunk face bounds the stencil only where no neighbouring chunk is active: across an active same-chart face the neighbour is counted like any other and receives the same share, so the chunk grid carries no physical meaning (INV-037).

This permits two physically similar structures to couple even when societies interpret them differently, and physically different structures to couple differently even when agents believe they mean the same thing.

## Provenance and commits

Evolution produces replacement-state proposals and changed-cell records. It does not mutate the source field. Each changed cell carries traces supporting direct pattern injection and neighbouring prior field state. A caller must commit one new provenance trace per changed cell before constructing the next field.

## Bounded material-surface coupling

The active actor/material/mana slice uses one concrete field-to-matter path. Runtime-owned
`MaterialSurface` records are addressed by a chart-qualified chunk and a local cell ordinal, are
created by `HistoricalBootstrapPlan`, and retain condition, contact count, and last transition
trace. A valid actor action commits its body and surface-contact effects in the Action phase.
Physics emits canonical samples only from changed surfaces; mana therefore receives measurable
structure, not an action label or a semantic interpretation. Each historically contacted surface
reads only its matching mana cell. A rising local threshold/hysteresis transition commits one
Mana-phase event containing the gate and condition changes with local-cell and prior-surface
ancestry; a falling transition commits only the gate change. Crossing either direction is a
persisted provenance commit, not an untraced control flag.

This is a bounded local material response, not a general material economy, terrain process, or
biological coupling. The changed surface supplies a range-limited physical signal, but agents
still receive only generic extracted features and subjective-scene cues.

## Bounded immutable experiment-recipe source

One additional Mana-phase path commits an external experiment-policy creation
through the same proposal/commit boundary. `RuntimeConfig` carries an immutable
recipe of at most 16 `ExperimentRecipeManaSource` records, each with an opaque
source record ID, enabled flag, scheduled tick, chart-qualified cell address,
fixed-point `i64` amount, per-record maximum, recipe-wide budget, and policy
schema V1. The recipe rejects duplicates, invalid cells, invalid ticks, negative
or over-budget amounts, and malformed policy before any scheduler commit.

`ExperimentRecipeManaSourceSystem` (system schema id 19, registration order 1,
`Phase::Mana`) reads only the immutable recipe and its own bounded executed-receipt
state. At a record's scheduled tick it commits exactly one root external-cause
event (event kind 17, object kind 9, property 13, empty causes) with two effects:
a mana-cell before/after transition and an executed-receipt record. It then
installs the cell intensity and a `last_change` trace through
`ManaFieldSet::propose/commit_experiment_recipe_mana_source`. Disabled or zero-
amount records never commit and never receipt.

Executed receipts in `RuntimeState` are bounded to 16, sorted by
`(executed_tick, source_record_id)`, and contain the source record ID,
scheduled/executed tick, source trace, before/after intensity, recipe hash, and
policy schema. They prevent re-execution across save/resume. The source trace
becomes a parent-before-child ancestor of every derived mana evolution, gate
activity, and material-surface transition that uses the changed cell.

Same-seed replay and pre/post-source save/resume are exactly equal. A disabled
or zero-amount source produces no source commit and remains equal to its no-
record control in physical, history, and canonical digests.

This is an experiment-specific accounting rule, not a general source API,
conservation redesign, reservoir, or redistribution. A production conservation
model, operator reservoir, and broader external-creation policies remain deferred.

## Geography

Fields are chunk-local causal state, so terrain, geology, hydrology, climate, ecology, and construction can later alter sample production or field parameters. Phase 17 does not invent those couplings. Same-chart cross-chunk exchange is implemented and conducts at the interior rate; cross-chart transport is deferred.

RFC-GEO-002 classifies the current cubic field as bounded local Euclidean 3D inside one surface chart. Bare `ChunkCoord` is not a global planetary position. Cross-chart diffusion requires curvature-aware registered transforms. Future density, phase, spectral, or persistence components would add field-state dimensions, not extra spatial dimensions.

## Determinism and performance

All hot arithmetic is fixed-point integer arithmetic; sample and cell traversal is canonical. Field volume and input batches have public hard bounds. The dense CPU implementation makes no scale claim. Sparse and accelerated alternatives require benchmarks and bit-identical validation.

## Deferred phenomena

Additional field-to-matter mechanisms beyond the bounded material surface, interference phase
state, long-lived attractors, artifacts, gods/spirits, semantic observer classifications,
cross-chunk exchange, and visualization remain future work.

## Related documents

- `docs/rfc/RFC-MANA-001.md`
- `docs/vision/project-thesis.md`
- `docs/architecture/provenance.md`
- `docs/ontology/causal-carriers.md`
- `docs/rfc/RFC-GEO-002.md`
