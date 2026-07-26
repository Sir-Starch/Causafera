# Mana Topology

Mana is a local information-sensitive physical field. Phase 17 establishes its minimum deterministic topology without claiming final mana physics.

## Authoritative state

`causafera-domains::mana::ManaField` stores a chunk coordinate, a cubic extent of at most `CHUNK_SIZE`, row-major fixed-point intensity, the last committed causal trace and pre-change intensity per cell, and the latest incorporated simulation tick. Human names and classifications are absent.

## Physical inputs

The field accepts bounded `PhysicalPatternSample` batches. Each sample supplies an opaque fingerprint of canonical carrier structure, local coordinate, observation tick, magnitude, stable source ordinal, and causal trace. Integrations may derive fingerprints from acoustic wave structure, repeated geometry, motion, physical glyph arrangements, material state, or other objective carriers. They may not hash words, beliefs, concepts, practice meanings, document genres, or observer labels into a fingerprint.

## Minimal response

Canonical same-fingerprint samples increase response through repeated occurrence, regular temporal intervals, simultaneous occurrence, and repeated placement at distinct coordinates. Magnitude scales local injection. A fixed eighteen-neighbour stencil then applies diffusion and decay, with non-negative saturation. The six face neighbours carry weight 2 and the twelve edge neighbours weight 1, the smallest exact-integer weighting that is isotropic to fourth order, so a source spreads as a sphere rather than along the lattice axes and no floating point enters the authoritative path. A cell gives each neighbour an equal share and loses exactly the sum of the shares delivered, so diffusion moves mana without destroying it. A chunk face bounds the stencil only where no neighbouring chunk is active: across an active same-chart face the neighbour is counted like any other and receives the same share, so the chunk grid carries no physical meaning (INV-037). The observer projects the volume unreduced through the `FieldRaster` query, with the trace that last changed each cell, and the map assembles the received lattices into one field across the whole surveyed extent — so the absence of physical meaning in the chunk grid is visible in the drawing rather than only asserted here.

This permits two physically similar structures to couple even when societies interpret them differently, and physically different structures to couple differently even when agents believe they mean the same thing.

## Provenance and commits

Evolution produces replacement-state proposals and changed-cell records. It does not mutate the source field. Each changed cell carries traces supporting direct pattern injection and neighbouring prior field state, including when the neighbour lies in an adjacent chunk: a share crossing a seam is attributed to what produced the value that crossed, which is this tick's injection as well as the source cell's previous change. A caller must commit one new provenance trace per changed cell before constructing the next field.

No cell change is proposed without ancestry. A proposal whose changed cell has an empty cause list is refused rather than emitted, because a mana cell that changed for no recorded reason is authoritative state without provenance. That is total only because a cell holds mana only where some commit put it: fields start at zero and untraced, and every commit records the change on every cell it moves. Only the experiment-recipe source commits mana as a root event, and it does so through its own path.

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

The terrain coupling is implemented. The terrain carrier presents its standing structure at the
field's own lattice — one sample per plan-view column, magnitude equal to the column's mean relief
contrast, material discontinuity and roughness, fingerprint derived from the column's dominant
surface material and roughness class. Those samples reach the field but never `PhysicalPatternHistory`:
history retains change, and a structure that is merely still there has not happened again. Retaining
it would let the recurrence and periodicity channels accumulate over the window and so score the rate
at which the carrier is read rather than anything about the world, which was measured to run total
mana to twenty-one times the contact-driven baseline and hold every local gate permanently open.

What a standing structure earns is the response its within-tick structure supports: recurrence,
synchronisation and spatial repetition, and never periodicity, which needs occupied ticks the carrier
does not supply. Recurrence is included because RFC-MANA-001 defines it as additional occurrences of
the same fingerprint without requiring distinct ticks, and defines synchronisation as its same-tick
specialisation; same-tick co-occurrence therefore scores on both, for every carrier. The property the
exclusion buys is not that fewer channels fire but that none of them accumulate: the same emission
injects the same amount whether it is read on one tick or on fifty.

Whether the carrier reaches the loop is the persisted `RuntimeConfig::terrain_participation`
contract. See `plans/terrain-carrier-participation.md`.

RFC-GEO-002 classifies the current cubic field as bounded local Euclidean 3D inside one surface chart. Bare `ChunkCoord` is not a global planetary position. Cross-chart diffusion requires curvature-aware registered transforms. Future density, phase, spectral, or persistence components would add field-state dimensions, not extra spatial dimensions.

## Determinism and performance

All hot arithmetic is fixed-point integer arithmetic; sample and cell traversal is canonical. Field volume and input batches have public hard bounds. The dense CPU implementation makes no scale claim. Sparse and accelerated alternatives require benchmarks and bit-identical validation.

## Deferred phenomena

Additional field-to-matter mechanisms beyond the bounded material surface, interference phase
state, long-lived attractors, artifacts, gods/spirits, semantic observer classifications,
cross-chart transport, and visualization remain future work.

Same-chart cross-chunk exchange is not among them: it is implemented and conducts at the interior
rate, as the Geography section above records. This list said otherwise until now, which contradicted
that section of the same document. Its provenance gap is closed: a share crossing a seam carries the
ancestry of the value that crossed, and the proposal boundary refuses any cell change it cannot
attribute. What remains is that seam delivery does not apply the field's saturation ceiling, so a
cell fed across a seam in a saturated field can exceed `maximum_intensity` — recorded as
`TODO-MANA-006`.

## Related documents

- `docs/rfc/RFC-MANA-001.md`
- `docs/vision/project-thesis.md`
- `docs/architecture/provenance.md`
- `docs/ontology/causal-carriers.md`
- `docs/rfc/RFC-GEO-002.md`
