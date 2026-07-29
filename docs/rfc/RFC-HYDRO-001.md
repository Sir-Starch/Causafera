# RFC-HYDRO-001: Multi-Resolution Hydrology

**Status:** Accepted

## Summary

Hydrology is deterministic, conserved, causally inspectable physical state. It represents surface,
unsaturated-soil, groundwater, and unlabeled conveyance storage on a terrain-aligned lattice; it
consumes explicit tick-indexed forcing records rather than generating weather; and it evaluates at
multiple spatial resolutions over one retained fine canonical state.

The canonical implementation contract is [`plans/hydrology.md`](../../plans/hydrology.md). This RFC
records the accepted architectural decisions; the ExecPlan records the staged implementation,
verification, and progress.

## Motivation

Hydrology touches settlement viability, agriculture, biology, disease, economy, and the mana field's
response to measurable physical structure. None of those couplings can be built on a decorative
water-table scalar. They need water that is actually conserved, actually moves between places for
physical reasons, and can be explained after the fact from typed state and causal traces.

The prior placeholder was a single unvalidated `f32` per cell with no constructors, callers,
scheduler integration, persistence, or provenance. The maturity matrix therefore recorded Hydrology
as documentation-only M0.

## Details

### Units and numeric contract

- Water is `WaterVolume`, a non-negative `u64` count of cubic millimetres (`1 mm³ = 1 μL`).
- Depth is `WaterDepthMm`, aligned with terrain's millimetre elevation.
- All multiplication, summation, proportional allocation, and conservation arithmetic uses checked
  `i128`; conversion back to `u64` happens only after range validation.
- No operation saturates. Overflow, underflow, an invalid denominator, or an out-of-range result
  rejects the whole proposal before causal commit.
- Fixed-point arithmetic provides replay and ledger exactness, not physical exactness. Quantization
  remainders remain in donor storage and are never converted into an untracked sink.

### Grid metric

`HydrologyGridMetric` is registered per chart and carries explicit `cell_area_mm2`,
`orthogonal_edge_length_mm`, and `timestep_millis`. It is never inferred from `chunk_extent`,
`CHUNK_SIZE`, containment, observer zoom, or UI scale. Chunks are addressing and computation units,
not metric geometry (INV-036, INV-037, INV-043).

### Lattice and topology

The canonical hydrology lattice is the terrain-aligned two-dimensional surface lattice. Every chunk
has exactly `CHUNK_SIZE * CHUNK_SIZE` cells addressed by row-major ordinal. Adjacency is four-face
(`-X, +X, -Y, +Y`); there are no vertical cell faces. Seam mapping preserves the orthogonal
coordinate and uses `ChartChunkCoord::same_chart_neighbor`, so a same-chart chunk seam behaves
exactly like an interior face. Cross-chart transport is out of scope for this tranche.

### State

Per cell: surface, soil, and groundwater storage, each with its own trace anchor, plus a persisted
forcing-input fingerprint and trace. Aligned per cell: a `HydraulicSubstrateCell` carrying explicit
capacities, infiltration limit, percolation fraction, specific yield, aquifer base elevation,
baseflow threshold and fraction, and surface/groundwater conductance.

Conveyance is `HydrologyConveyanceEdge`: physical edge storage with capacity, release fraction,
per-tick inlet capacity, and one outlet endpoint. It has no semantic water-body type. Edges may
cross same-chart chunk seams.

Exterior faces carry an explicit `HydrologyBoundaryCondition` with independent surface and
groundwater channels, each either `NoFlux` or `Open { external_head_mm, conductance_mm2_per_tick }`.
A missing same-chart resident neighbour is an explicit boundary record, never an implicit wall.

### Process order

One hydrology `Phase::Physics` execution runs frozen substages, each reading one immutable state and
producing a complete next-state delta: forcing acceptance, infiltration, percolation/recharge,
evapotranspiration, surface routing, groundwater routing and baseflow, conveyance routing, boundary
export finalization, conservation preflight, then causal commit and installation. No cell observes
another cell's same-substage write, so no same-tick cascade and no directional bias by iteration
order is possible.

Lateral transfer uses a symmetric harmonic endpoint conductance and head difference. Donor
availability is applied first and receiver capacity second, both with a canonical largest-remainder
rule; rejected water is retained by its donor. Capacity excess is never silently clamped — it is
routed to an accounted destination or the proposal rejects.

### Conservation and provenance

Every tick emits one terminal conservation receipt whose residual
(`storage_before + sources - storage_after - sinks`) must be exactly zero in `i128` before any event
commits. Every accepted source, sink, transfer, and storage change carries reconstructable causal
ancestry (INV-014). Limiters are explicit receipt data, never disappearing clamps.

Same-tick cross-substage ancestry requires an atomic intra-batch DAG commit whose causes may be
existing traces or local proposal keys, validated and canonically topologically ordered before any
identifier is reserved. Terminal ancestry is bound through a canonical 16-ary aggregation tree so
every event stays within the existing sixteen-cause bound while the durable trace DAG still reaches
every terminal bucket, edge, resolution, and forcing-application event.

### Resolution coupling — resolved

The first previously unresolved question is settled as follows.

| Representation | Decision |
| --- | --- |
| Delete fine state on demotion and reconstruct on promotion | Rejected: invents detail and provenance, and cannot guarantee conservation. |
| Keep independent coarse and fine authoritative states | Rejected: creates competing truths and a reconciliation problem. |
| Retain fine canonical state; compute coarse proposals and allocate back | **Chosen.** |

Retained fine cell and edge state stays canonical and resident. A coarse unit is an ordered
chart-grid block addressed by global terrain-cell coordinates, never by chunk extent. Cells inside a
block are partitioned into canonical constitutive groups by exact `(metric, substrate,
boundary-kind)` tuple. Vertical processes and forcing execute once per constitutive group over exact
aggregates, and every accepted group delta is returned to fine members by a capped
largest-remainder reducer. Internal lateral faces are not evaluated at coarse resolution; every fine
face on a block boundary remains authoritative and is installed on its actual fine endpoints, with
coarse block totals existing only as receipt and validation aggregates.

Demotion therefore loses no water and promotion invents none. Resolution may alter approximation
detail and therefore future trajectory; it may not alter total water, topology, metric geometry,
boundary openness, or causal ancestry. Hydrology resolution is independent of terrain, mana,
population, and observer resolution (INV-010), and cannot be selected by observer attention
(INV-012, INV-013).

### Seasonal variation — resolved

The second previously unresolved question is settled as follows: **hydrology consumes explicit
tick-indexed forcing records and does not implement semantic seasons or climate generation.**

`HydrologyForcingRecord` carries a scheduled tick, a canonical non-empty weighted target member set,
a precipitation total, a potential-ET demand total, an external inflow total, a committed
`origin_trace`, a producer policy schema, and an `applied_at` state that transitions exactly once.
Per-record totals are allocated across members by the proportional largest-remainder rule.

Forcing origin is producer-neutral. The only accepted producer in this tranche is the appended
seventh production bootstrap stage. A future Climate system may become another accepted producer by
committing the same origin contract in an earlier tick and phase, without changing hydrology
semantics. Climate remains M0 and gains only this output boundary.

Potential ET is a demand, not a guaranteed removal: accepted ET is bounded by available surface and
soil water, and unmet demand is recorded rather than treated as water loss.

### Ownership

- `causafera-types` owns generic numeric primitives (`WaterVolume`, `WaterDepthMm`).
- `causafera-geography` owns canonical hydrology state and physical input schemas.
- `causafera-domains` owns hydrology evolution and proposal construction without owning canonical
  state.
- `causafera-runtime` owns orchestration, authoritative commit, bootstrap, persistence, and the
  resolution adapter.

This preserves the existing crate dependency direction; `causafera-domains` already depends on
`causafera-geography`, and reversing that edge would create a cycle. No
`causafera-domains -> causafera-resolution` dependency is added.

### Downstream surfaces

Explanation and the observer read hydrology state and never mutate it (INV-020, INV-021, INV-022).
Explanation gains typed claims for storage and water-table range, forcing ancestry and accepted or
unmet forcing, transfer path and limiter evidence, exact conservation residual, boundary export, and
explicit insufficiency. Observer protocol V1 gains bounded optional additive fields; old decoders
ignore them and new decoders reject malformed values.

## Non-goals

- Full climate or atmospheric generation.
- Geological formation, strata, deformation, or aquifer classification.
- Snow or ice accumulation and phase change.
- Sediment transport, erosion, solutes, salinity, pollutants, or water quality.
- Full Saint-Venant hydraulics, backwater, flow reversal, pressurization, coastal tides, or
  cross-chart ocean routing.
- Dams, pumps, weirs, canals, irrigation operations, municipal networks, or other infrastructure
  control.
- Ecology, agriculture, health, settlement, economy, history, biology, or mana coupling
  implementation.
- Semantic water-body or hazard labels in authoritative state.
- Observer UI work or observer-driven activation.
- CUDA/GPU work or unmeasured scale claims.
- Migration shims that default absent hydrology into an old production snapshot.

## Primitive versus emergent

Primitive and authoritative: volume, depth, elevation, area, edge length, timestep, storage capacity,
conductance, chart-qualified cells, physical edges, explicit forcing and boundary conditions,
accepted transfers, and causal traces.

Emergent and downstream only: river, stream, lake, pond, wetland, flood, drought, watershed, aquifer
class, season, reliable water source, agricultural suitability, disease risk, settlement viability,
and mana pattern. No authoritative carrier is a semantic `River`, `Lake`, `Wetland`, `Flood`,
`Season`, or `Watershed` enum, and no such term may become a bootstrap label, event kind, or routing
shortcut.

## Unresolved Questions

None. Resolution coupling and seasonal variation, the two questions this RFC previously left open,
are resolved above.
