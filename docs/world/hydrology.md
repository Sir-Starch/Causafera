# Hydrology

Water as conserved causal state. Every quantity below is a fixed-point integer
that a ledger closes over exactly, every movement carries a causal trace, and
nothing in authoritative state is named.

This document describes what is implemented. The contract is
[`plans/hydrology.md`](../../plans/hydrology.md) and
[`RFC-HYDRO-001`](../rfc/RFC-HYDRO-001.md); the code decides what exists today.

## What water is

Water is a `WaterVolume`: an unsigned integer count of cubic millimetres. Not a
float, and not a depth — a depth is a volume divided by a cell area, and the two
are different quantities with different rounding.

Each hydrology cell holds three storages and nothing else:

```text
HydrologyCellStorage:
    surface:     WaterVolume    # ponded above the ground
    soil:        WaterVolume    # unsaturated zone
    groundwater: WaterVolume    # saturated zone
```

A fourth storage lives on conveyance edges, which are directed channels between
two adjacent cells with a capacity, an inlet limit, and a release fraction.

A cell is addressed by `HydrologyCellKey { chunk, cell_ordinal }` over the same
32 x 32 lattice terrain uses. A chunk is addressing, not geometry: containment
defines neither adjacency nor distance, and the metric that turns volumes into
depths is `HydrologyGridMetric { cell_area_mm2, cell_edge_mm, timestep_millis }`,
declared per chart. Runtime `chunk_extent` sizes the mana volume and is never a
hydrology length.

### There are no water bodies

There is no `WaterBody`, no `River`, `Lake`, `Wetland`, or `Reservoir` type, no
polygon boundary, no drainage network, and no catchment. Those are readings a
downstream observer may compute from storage and flow; they are not simulation
state, and nothing may feed such a classification back into the simulation. A
source audit —
[`tools/audit/test-hydrology-production-boundaries.mjs`](../../tools/audit/test-hydrology-production-boundaries.mjs)
— fails the build if one appears in a production path.

Surface material identity has no hydraulic meaning: two worlds differing only in
the material under the water behave identically, and no hydrology path reads a
`MaterialId` at all.

## Substrate and derived coefficients

Each cell has a `HydraulicSubstrateCell`: capacities, an infiltration limit, a
percolation fraction, a specific yield, an aquifer base elevation, a baseflow
threshold and fraction, and two conductances. Every coefficient is *derived* at
bootstrap from declared physical parameters and the grid metric, by exact integer
arithmetic:

```text
infiltration_limit_per_tick = floor(rate_mm_per_s * cell_area_mm2 * millis / 1000)
adjusted_transmissivity     = floor(base * roughness_reference
                                    / (roughness_reference + cell_roughness))
surface_conductance         = floor(adjusted * millis / (1000 * cell_edge_mm))
```

Groundwater conductance uses the same timestep and edge rule but is not adjusted
for surface roughness — roughness is a property of the surface.

## Forcing

Precipitation, potential evapotranspiration, and external inflow arrive only as
explicit persisted `HydrologyForcingRecord`s, each with a scheduled tick, an
opaque producer identity, weighted target cells, and an origin trace. There is no
climate model behind them and no wall clock: a record is state, applied at its
scheduled tick and then marked applied, so its identity and allocation inputs
survive receipt eviction.

Allocation across a record's targets uses canonical largest-remainder rounding,
so the parts sum to the whole exactly and the result does not depend on
iteration order.

## The tick

One hydrology tick runs a fixed substage order, each substage reading the state
the previous one produced. No cell observes another cell's same-substage write,
which is what makes a chunk seam behave like an interior face rather than like a
direction the solver happens to sweep.

```text
1 forcing            5 surface routing
2 infiltration       6 groundwater routing
3 percolation        7 conveyance routing
4 evapotranspiration 8 boundary export
                     9 conservation
```

Lateral flow is driven by head differences against one absolute reference:
surface head is the ponded water's own top surface, and groundwater head is the
water table implied by the stored volume and the specific yield
(`causafera_domains::groundwater_head_mm`). A face has one conductance whichever
side asks, computed as the harmonic mean of its two endpoints, and a face with a
zero endpoint conducts nothing.

## Conservation

Every tick produces a `HydrologyConservationReceipt` whose residual is computed
in `i128` and must be **exactly zero** before any event commits:

```text
storage_before + sources == storage_after + sinks
```

Not "close to zero" and not "within a tolerance" — the whole point of a
fixed-point carrier is that the ledger closes, and a tolerance would be a place
for a silent sink to live. Sources are accepted precipitation and external
inflow; sinks are accepted evapotranspiration and boundary export. A rounding
remainder stays in donor storage.

Every accepted, partly accepted, or wholly rejected movement is a
`HydrologyTransferReceipt` carrying requested, accepted, and unaccepted volumes,
both endpoints' before and after states, and its causal parents. All three
volumes travel because a limiter that engaged is evidence: recording only what
moved would make a bound that engaged indistinguishable from a process that had
nothing to do.

## Multi-resolution

Fine state stays canonical at every level. A coarse level changes only how much
is *evaluated*: cells are partitioned into `HydrologyBlockKey` blocks of
`2^min(level, 4)` cells on a side, a block's constitutive inputs must match for
it to be grouped, and a coarse group's total is returned to its fine members by a
capped largest-remainder reducer that respects every member's own ceiling.

The level per chunk comes from the engine's own `ResolutionField`, not from a
hydrology-local policy. `HydrologyResolutionPolicy` declares a maximum the
session will accept, and a level above it **refuses the tick** rather than
clamping it — a clamp would silently evaluate a world at a detail the engine did
not choose.

Promotion and demotion conserve water, topology, and ancestry exactly. Detail
changes never change adjacency or distance.

## Persistence, digest, and bounds

Hydrology is section `0x000F` of the snapshot envelope, at section major 1. It
participates in the physical state digest at schema 8; the digest is an identity,
never a distance. Export and import are byte-identical round trips.

Import trusts none of it. The decoder rebuilds every collection through the same
constructor a live runtime uses, and enforces the composed bounds — the aggregate
forcing-member cap and the retention-wide receipt cap — as running totals, before
the vectors they bound are allocated rather than after. What no single
constructor can see is then checked across the whole state: registry bijection,
residency of every field, conveyance endpoint and outlet, resolution coverage,
forcing schedule order and origin fan-in, forcing timing against the clock, ledger
closure and receipt recomputation, and continuity of the retained batch window.

Two further classes are checked only at import, because a running tick cannot
have broken them. The state must agree with the configuration that bootstrapped
it — same grid metrics, same forcing records — and every trace anchor must name
an event the store actually holds: each bucket's, each conveyance edge's, each
resolution entry's, each retained receipt's parents, and each batch's own key,
which must be a hydrology conservation event rather than merely some trace.
Forcing ancestry is held to the same standard: a record's origin must be the
bootstrap stage's own origin event, under the one producer policy that stage
applies, because configuration cannot name a trace and a session may not declare
itself an authorized producer.

Retained typed detail is bounded: at most eight whole batches and 262,144
transfer receipts, evicted whole-batch-first in tick order. Eviction removes
typed detail without deleting causal traces, so an old Explanation request
answers with insufficiency rather than with a number nothing supports.

Every hydrology-enabled tick runs inside a whole-tick staging transaction. The
complete pre-image is staged before the tick starts; a tick that fails validation
— including one whose result would no longer fit the 256 MiB export cap — is
rolled back whole, leaving state, traces, counters, scheduler clock, and RNG
stream keys exactly as they were.

## Observation

Observer protocol V1 carries hydrology additively: a bounded whole-session
summary, per-tick cell deltas, transfer summaries, conveyance summaries, and
per-chunk water rasters in a lossless unsigned band. A decoder written before
hydrology existed reads a new payload and gets exactly what it always got. See
[`docs/observer/protocol.md`](../observer/protocol.md).

Explanation carries typed claims for storage bounds and water-table range,
forcing ancestry and accepted/unmet forcing, transfer path and limiter evidence,
the exact conservation residual, and boundary export — with explicit
insufficiency where evidence is missing rather than a narrowed number or a
fabricated classification. See
[`docs/explanation/explanation-ir.md`](../explanation/explanation-ir.md).

Neither may mutate state, and neither may select the active resolution.

## What is not implemented

Named here so this document is not read as a promise:

- climate or atmospheric generation — forcing records are the only input;
- geological formation, strata, or aquifer classification;
- snow, ice, and phase change;
- sediment, erosion, solutes, salinity, or water quality;
- full Saint-Venant hydraulics, backwater, flow reversal, or pressurization;
- coastal tides and cross-chart ocean routing;
- dams, pumps, weirs, canals, or irrigation networks;
- coupling to ecology, agriculture, health, settlement, economy, or mana; and
- GPU acceleration.

## Measured cost

Reproducible measurements, including the ceiling the export cap imposes on
session length, are in
[`docs/performance/benchmarks.md`](../performance/benchmarks.md).

## Related documents

- [`geography-philosophy.md`](geography-philosophy.md) — geographic causality
- [`terrain.md`](terrain.md) — the elevation and roughness hydrology reads
- [`spatial-hierarchy.md`](spatial-hierarchy.md) — what a chunk is and is not
- [`../ontology/causal-carriers.md`](../ontology/causal-carriers.md) — carrier identities
- [`../architecture/invariants.md`](../architecture/invariants.md) — the hard contract

## RFCs

- `RFC-HYDRO-001: Multi-Resolution Hydrology`

## TODO categories

- `HYDRO` — hydrology
- `WORLD` — general world systems
