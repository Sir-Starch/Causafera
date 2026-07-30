# Conserved Multi-Resolution Hydrology ExecPlan

**Status:** Accepted

## Goal

Replace the inert hydrology placeholder with deterministic, conserved, causally inspectable
hydrologic state and processes. The completed system must represent surface, unsaturated-soil,
groundwater, and unlabeled conveyance storage; accept explicit precipitation and
evapotranspiration forcing; perform infiltration, percolation, runoff, groundwater flow, baseflow,
and storage-discharge routing; cross same-chart chunk boundaries; remain conservative under
hydrology-specific resolution changes; persist and replay exactly; and expose bounded Explanation
and observer read models.

This is the full repository-evidenced hydrology tranche. It is not an MVP and does not use authored
river/lake labels, a hidden climate generator, or chunk boundaries as physical walls.

## Context

`crates/causafera-geography/src/hydrology.rs` currently contains only:

```rust
pub struct HydrologyCell {
    pub water_table: f32,
}
```

It has no constructors, validation, callers, tests, scheduler integration, persistence, provenance,
or observer surface. `crates/causafera-geography/src/climate.rs` is a similar inert `f32`
placeholder. The domain coverage matrix therefore records Hydrology and Climate as
documentation-only M0.

`docs/world/hydrology.md` describes the intended cycle and cross-domain relevance, but
`docs/rfc/RFC-HYDRO-001.md` remains Proposed and leaves resolution coupling and seasonal variation
unresolved. This plan resolves those two questions:

1. retained fine hydrology state is canonical; lower-detail computation uses exact aggregates and
   deterministically allocates accepted deltas back to retained state, so demotion loses no water
   and promotion invents none; and
2. hydrology consumes explicit tick-indexed forcing records. It does not implement semantic seasons
   or climate generation.

The closest implementation precedent is the thermal carrier:

- `causafera-domains/src/thermal/` owns proposal construction and conservation records;
- `causafera-runtime/src/thermal.rs` schedules evolution, commits sorted causal events, and installs
  state and traces only after preflight succeeds;
- `causafera-runtime/src/snapshot_sections.rs` persists the field, forcing reservoirs, receipts, and
  conservation records with strict decoding; and
- the thermal runtime tests cover frozen pre-state, cross-chunk transfer, exact conservation,
  malformed import, replay, save/resume, observer projection, and Explanation.

Terrain is a separate precedent for deterministic generation, chart-qualified cell addressing,
provenance, and real cross-chunk neighbors. Terrain is not itself a mutation-loop precedent.

## Relevant invariants

- **INV-009 — Geography is causal state.** Hydrology must modify future physical causality rather
  than exist as decorative metadata.
- **INV-010 — Causal relevance determines resolution.** Hydrology detail changes independently from
  other domain detail.
- **INV-012 / INV-013 — Observer analysis and attention are non-authoritative.** Hydrology evolution
  cannot depend on observer requests, locale, viewport, or classifications.
- **INV-014 — Provenance is first-class.** Every accepted source, sink, transfer, and storage change
  has reconstructable causal ancestry.
- **INV-016 — Authoritative mutation is phase controlled.** All hydrology changes pass through
  proposal, deterministic reduction, causal commit, and state installation.
- **INV-017 / INV-018 — Performance is architectural and measured.** Data layout and aggregation
  costs are planned; scale claims require reproducible benchmarks.
- **INV-019 — Emergence is inspectable.** Persistent flow structures and downstream hazard
  classifications must be reconstructable from typed state and traces.
- **INV-020 / INV-021 / INV-022 — Narrative, observer, and rendering are downstream.** They never
  mutate hydrology.
- **INV-023 — World generation has provenance.** Grid metrics, hydraulic substrate, initial storage,
  and forcing schedules are causally initialized.
- **INV-026 — Explanation carries confidence and traces.** Unsupported hydrology claims degrade to
  insufficiency rather than guessing.
- **INV-036 / INV-037 — Coordinate scope and geometry are explicit.** Hydrology grid metrics are not
  derived from chunk extent; resolution changes neither topology nor metric geometry.
- **INV-038 — Digests are identities.** Hydrologic similarity uses typed values, never digest-byte
  distance.
- **INV-039 — Production state requires causal initialization.** No demo basin or fixture water is
  allowed in production bootstrap.
- **INV-042 — Architecture remains modular.** State, evolution, runtime orchestration, persistence,
  and observation stay in named modules.
- **INV-043 — The world is one coherent spatial system.** Same-chart chunk seams cannot block or
  duplicate water transfer.

## Ontology domains affected

- **Hydrology** — M0 documentation placeholder becomes an executable, conserved physical domain.
  Final maturity wording is evidence-driven; this plan does not pre-claim an M-level.
- **Geography** — terrain elevation, roughness, chart identity, and real neighbor topology become
  hydrology inputs.
- **Space / Resolution** — adds an explicit hydrology grid metric and a domain-owned conservative
  aggregation contract.
- **Time** — forcing and routing use scheduler ticks plus an explicit timestep in the hydrology grid
  schema.
- **Matter** — surface material identity may select a typed hydraulic-substrate record, but this
  tranche does not add general material permeability or geological formation.
- **Climate** — remains M0. It gains only a future-compatible output boundary: persisted hydrology
  forcing records.
- **Explanation / Observer** — gain bounded, read-only hydrology evidence and projections.

## Causal carriers affected

New authoritative carriers:

- `WaterVolume`, measured in integer cubic millimetres (`1 mm³ = 1 μL`);
- `HydrologyGridMetric`, keyed by chart and carrying explicit cell area, orthogonal edge length, and
  timestep;
- `HydraulicSubstrateCell`, carrying soil capacity, infiltration limit, percolation fraction,
  specific yield, aquifer base, and surface/groundwater conductance;
- `HydrologyCellState`, carrying surface, soil, and groundwater storage plus trace anchors;
- `HydrologyConveyanceEdge`, an unlabeled physical edge with storage, capacity, release/inlet
  limits, outlet, and trace anchors;
- `HydrologyForcingRecord`, an explicit tick-indexed precipitation source and potential-ET sink
  demand;
- `HydrologyTransferReceipt`, recording every vertical, lateral, conveyance, forcing, and boundary
  transfer; and
- `HydrologyConservationReceipt`, recording all pre-state storage, accepted sources, final storage,
  sinks/exports, and exact residual.

Existing carriers reused:

- `ChartChunkCoord`, cell indices, and `same_chart_neighbor`;
- terrain elevation, roughness, and surface material identity;
- scheduler `Phase::Physics`;
- `CausalTraceStore`;
- production bootstrap stage records;
- physical/history digests; and
- observer query and Explanation evidence envelopes.

No authoritative carrier is a semantic `River`, `Lake`, `Wetland`, `Flood`, `Season`, or `Watershed`
enum. Those are downstream classifications over measurable storage, flow, geometry, and history.

## Relevant documents

- `AGENTS.md`
- `CLAUDE.md`
- `PLANS.md`
- `docs/vision/project-thesis.md`
- `docs/vision/uniqueness.md`
- `docs/architecture/invariants.md`
- `docs/architecture/detailed-development-rebaseline.md`
- `docs/architecture/determinism.md`
- `docs/architecture/performance.md`
- `docs/architecture/provenance.md`
- `docs/ontology/domain-coverage-matrix.md`
- `docs/ontology/causal-carriers.md`
- `docs/world/geography-philosophy.md`
- `docs/world/terrain.md`
- `docs/world/hydrology.md`
- `docs/world/climate.md`
- `docs/world/geology.md`
- `docs/world/spatial-hierarchy.md`
- `docs/rfc/RFC-HYDRO-001.md`
- `docs/rfc/RFC-GEO-001.md`
- `docs/rfc/RFC-RES-001.md`
- `plans/conserved-thermal-energy-carrier.md`
- `plans/thermal-conservation-aggregate-validation.md`
- `plans/terrain-chunk-boundary-continuity.md`
- `plans/terrain-structure-cross-chunk-neighbours.md`
- `plans/production-bootstrap-receipt-closure.md`

External physical-model references:

- USGS, *Water-budget methods*: inflow, outflow, and storage-change accounting across spatial and
  temporal scales — <https://www.usgs.gov/publications/water-budget-methods>
- NOAA National Water Model: separate surface, saturated-subsurface, and channel routing with
  external forcing — <https://water.noaa.gov/about/nwm>
- EPA SWMM: precipitation, evaporation, infiltration, groundwater percolation/interflow, overland
  storage/routing, and kinematic/dynamic routing boundaries —
  <https://www.epa.gov/water-research/storm-water-management-model-swmm>
- USACE HEC-HMS Muskingum-Cunge reference: continuity plus simplified momentum for channel routing —
  <https://www.hec.usace.army.mil/confluence/hmsdocs/hmstrm/channel-flow/muskingum-cunge-model>

## Current state

- `HydrologyCell` and `ClimateCell` are unvalidated `f32` placeholders with no behavior or users.
- `causafera-geography` depends only on `causafera-types`, `causafera-core`, and
  `causafera-world`.
- `causafera-domains` already depends on `causafera-geography`; reversing that edge would create a
  cycle.
- Terrain has deterministic, provenance-bearing cells and real same-chart cross-chunk neighbor
  access, but no registered metric cell area or edge length.
- `Runtime::new` registers systems sequentially. Scheduler registration order assigns global system
  IDs, and those IDs feed deterministic RNG stream keys.
- Physics registration currently includes `PhysicalPatternSystem`, `ThermalReservoirSystem`, and
  `ThermalEvolutionSystem`. Adding hydrology anywhere except after all existing registrations risks
  changing existing system IDs.
- Runtime snapshot section IDs currently end at thermal section `0x000E`; hydrology receives
  `0x000F`.
- `CURRENT_DIGEST_SCHEMA_VERSION` is 7. Hydrology authoritative state advances it to 8.
- `RUNTIME_RECIPE_SECTION_MAJOR` is 6. Adding hydrology bootstrap configuration advances it to 7.
- `OBSERVER_PROTOCOL_V1` is 1. The hydrology observer additions are additive optional fields and
  remain protocol V1; old decoders must ignore them and new decoders must reject malformed values.
- `cargo test -p causafera-geography --all-features` currently passes seven terrain tests and no
  hydrology tests.
- No active canonical hydrology ExecPlan or dedicated `TODO-HYDRO-*` entry exists.

## Proposed architecture

### 1. Ownership and module boundaries

`causafera-types` owns generic numeric primitives:

- `crates/causafera-types/src/physics.rs`
  - `WaterVolume(u64)`
  - `WaterDepthMm(u64)`
  - checked conversions and `i128` accumulation helpers

The canonical hydrology lattice is the terrain-aligned two-dimensional surface lattice:

```text
HydrologyCellKey {
    chunk: ChartChunkCoord,
    cell_ordinal: u16,
}
```

- Every chunk has exactly `SURFACE_CELL_COUNT = CHUNK_SIZE * CHUNK_SIZE` hydrology cells.
- Local coordinates are `(x, y)` with `0 <= x,y < CHUNK_SIZE`.
- Row-major ordinal is `y * CHUNK_SIZE + x`.
- Adjacency is four-face `-X, +X, -Y, +Y`; there are no vertical cell faces.
- Seam mapping preserves the orthogonal coordinate and uses
  `ChartChunkCoord::same_chart_neighbor`.
- Runtime `chunk_extent` never changes hydrology cell count, ordinal mapping, or metric scale.

`causafera-geography` owns canonical hydrology state and physical input schemas. Replace the flat
`src/hydrology.rs` with:

```text
crates/causafera-geography/src/hydrology/
  mod.rs
  metric.rs
  substrate.rs
  forcing.rs
  state.rs
```

`causafera-domains` owns hydrology evolution without owning canonical state:

```text
crates/causafera-domains/src/hydrology/
  mod.rs
  parameters.rs
  evolution.rs
  proposal.rs
  records.rs
  receipts.rs
  resolution.rs
```

`HydrologyEvolutionModel::propose(&HydrologyFieldSet, HydrologyEvolutionRequest)` returns a complete
`HydrologyEvolutionProposal`; it does not add inherent implementations to external geography types.

`causafera-runtime` owns orchestration and authoritative commit:

```text
crates/causafera-runtime/src/
  hydrology.rs
  hydrology_events.rs
  hydrology_validation.rs
```

Runtime state, bootstrap, snapshot, digest, Explanation, and observer adapters remain in their
existing cohesive modules.

### 2. Numeric and metric contract

`WaterVolume` is non-negative `u64` cubic millimetres. All multiplication, summation, proportional
allocation, and conservation arithmetic uses checked `i128`; conversion back to `u64` occurs only
after range validation.

No operation saturates. Overflow, underflow, invalid denominator, or out-of-range result rejects the
whole proposal before causal commit.

`HydrologyGridMetric` is keyed by chart and contains:

```text
schema_version = 1
cell_area_mm2: NonZeroU64
orthogonal_edge_length_mm: NonZeroU64
timestep_millis: NonZeroU64
```

This is a registered domain-grid metric. It is not inferred from `chunk_extent`, `CHUNK_SIZE`,
containment, observer zoom, or UI scale. Cross-chart transport is outside this tranche.

Depth conversion is:

```text
depth_mm = volume_mm3 / cell_area_mm2
remainder_mm3 stays in the same storage bucket
```

Fixed-point arithmetic provides replay and ledger exactness, not physical exactness. Quantization
remainders remain in donor storage and are never converted into an untracked sink.

### 3. Canonical state

Each `HydrologyCellState` stores:

```text
surface_water: WaterVolume
soil_water: WaterVolume
groundwater: WaterVolume
surface_last_change: TraceId
soil_last_change: TraceId
groundwater_last_change: TraceId
forcing_input_fingerprint: StateFingerprint
forcing_last_change: TraceId
last_change_before: HydrologyCellStorage
```

Each aligned `HydraulicSubstrateCell` stores:

```text
surface_capacity: WaterVolume
soil_capacity: WaterVolume
groundwater_capacity: WaterVolume
infiltration_limit_per_tick: WaterVolume
percolation_fraction_num: u32
percolation_fraction_den: NonZeroU32
specific_yield_num: u32
specific_yield_den: NonZeroU32
aquifer_base_elevation_mm: i64
baseflow_threshold: WaterVolume
baseflow_fraction_num: u32
baseflow_fraction_den: NonZeroU32
surface_conductance_mm2_per_tick: u64
groundwater_conductance_mm2_per_tick: u64
```

All fractions validate `0 <= numerator <= denominator`. Surface, soil, and groundwater storage have
explicit capacities. Capacity excess is never clamped: it is routed to the next bucket or causes proposal
rejection if no accounted path exists.

`HydrologyConveyanceEdge` uses a canonical edge key:

```text
min(chart, chunk, cell), max(chart, chunk, cell)
```

It stores physical capacity, storage, `release_fraction_num`,
`release_fraction_den`, `inlet_capacity_per_tick`, one outlet endpoint, and provenance. It has no
semantic water-body type. Edges may cross same-chart chunk seams. There is at most one conveyance
edge for a canonical cell face and at most one outgoing conveyance edge per cell. Missing same-chart
resident neighbors are explicit boundary records, not implicit walls.

`HydrologyBoundaryCondition` is keyed by an exterior cell face and contains independent surface and
groundwater channels:

```text
HydrologyBoundaryCondition {
    surface: FluxBoundary
    groundwater: FluxBoundary
}
FluxBoundary =
    NoFlux
  | Open {
        external_head_mm: i64,
        conductance_mm2_per_tick: u64,
    }
}
```

State constructors enforce these hard bounds before allocation:

```text
MAX_HYDROLOGY_CHARTS = 64
MAX_HYDROLOGY_CHUNKS = 128
MAX_HYDROLOGY_CELLS = 131_072
MAX_HYDROLOGY_EDGES = 262_144
MAX_HYDROLOGY_FORCING_RECORDS = 8_192
MAX_HYDROLOGY_TARGETS_PER_FORCING = 4_096
MAX_HYDROLOGY_TOTAL_FORCING_MEMBERS = 262_144
MAX_HYDROLOGY_FORCING_ORIGINS_PER_TICK = 8
MAX_HYDROLOGY_FORCING_ORIGINS_PER_CELL_PER_TICK = 6
MAX_HYDROLOGY_FORCING_HORIZON_TICKS = 1_000_000
MAX_HYDROLOGY_TRANSFERS_PER_TICK = 262_144
MAX_HYDROLOGY_STORED_RECEIPT_BATCHES = 8
MAX_HYDROLOGY_PERSISTED_TRANSFER_RECEIPTS = 262_144
MAX_HYDROLOGY_CHART_OVERRIDES = 64
MAX_HYDROLOGY_CELL_OVERRIDES = 131_072
MAX_HYDROLOGY_BOUNDARY_RECORDS = 524_288
MAX_HYDROLOGY_CAUSES_PER_EVENT = 16
MAX_HYDROLOGY_EFFECTS_PER_EVENT = 8
MAX_HYDROLOGY_SECTION_BYTES = 201_326_592
```

Each cell override contains at most four unique face directions. Boundary records are counted across
the complete resolved field. Chart overrides, cell overrides, face maps, boundary records, and total
forcing members all validate their counts before allocating.

The runtime retains the latest eight typed transfer/conservation batches in canonical tick order,
subject also to 262,144 transfer receipts across all retained batches. Before installing a new batch,
whole oldest batches are evicted until both bounds hold; a single batch above the transfer bound
rejects before causal commit. Older typed batches are evicted only after their causal events are
committed; the trace store remains the authoritative ancestry. Forcing schedule records, including
applied records, remain persisted so origin identity and allocation inputs survive typed receipt
eviction. Explanation over an evicted typed batch returns insufficiency for transfer detail but can
still report the persisted forcing record and origin.

### 4. Forcing and causal initialization

`HydrologyForcingRecord` contains:

```text
forcing_id: u64
scheduled_tick
targets: canonical non-empty HydrologyForcingMember vector
precipitation_volume
potential_et_volume
external_inflow_volume
origin_trace
producer_policy_schema
applied_at: Option<u64>
```

Each source/demand value is a total for the record, not a per-cell amount. A member contains
`{ cell: HydrologyCellKey, weight: NonZeroU64 }`. Members are sorted by cell key and unique.
Allocation uses the proportional largest-remainder rule in Section 6: descending remainder, then
ascending cell key. Overlap across records is legal and reduces in `(scheduled_tick, forcing_id)`
order. Before execution, the batch validates that no tick has more than
`MAX_HYDROLOGY_FORCING_ORIGINS_PER_TICK` distinct origins and no target cell has more than
`MAX_HYDROLOGY_FORCING_ORIGINS_PER_CELL_PER_TICK`; `limit + 1` rejects atomically. Every target must
be resident at its scheduled tick or the whole proposal rejects; there is no partial application.

At bootstrap, `scheduled_tick` must be strictly greater than the completed bootstrap tick and no
more than `MAX_HYDROLOGY_FORCING_HORIZON_TICKS` later; checked subtraction prevents wraparound.
Canonical records start with `applied_at = None`. Successful application changes it exactly once to
`Some(scheduled_tick)` in the same proposal as the water changes. Import rejects an applied record
whose timestamp differs, a pending record scheduled before the imported runtime time, or any attempt
to reapply a record.

Forcing origin is producer-neutral. `origin_trace` must reference a pre-existing committed root or
physical-source event whose opaque producer kind and `producer_policy_schema` authorize hydrology
forcing. The only accepted producer in this tranche is the seventh bootstrap stage. A future Climate
system may become another accepted producer by committing the same origin contract in an earlier
tick/phase; it does not need bootstrap ancestry.

`BOOTSTRAP_HYDROLOGY_FORCING_POLICY_V1 = 1`; unknown policy schema IDs reject. This plan reserves no
production Climate policy ID.

Forcing records are canonical state:

- initialized through an appended production bootstrap stage;
- persisted in hydrology section `0x000F`, major 1;
- included in physical digest schema 8;
- validated against the registered producer policy and origin-event ancestry on import; and
- marked applied exactly once at the scheduled tick, while the canonical record remains persisted.

Potential ET is demand, not guaranteed removal. Accepted ET is bounded by available surface and soil
water. Unmet demand is recorded, not treated as water loss.

Climate remains M0. A future climate implementation may produce the same forcing records without
changing hydrology semantics.

Append a seventh bootstrap stage after all existing stages. Preserve all existing stage IDs and
system identities. The hydrology stage initializes:

- grid metrics;
- substrate parameters;
- initial cell and conveyance storage;
- active/resident hydrology resolution state; and
- the bounded forcing schedule.

No production constructor may create a named basin, river, city supply, or demo rainfall event.

`RuntimeConfig` gains:

```text
HydrologyConfig {
    enabled: bool
    grid_metrics: bounded ordered map
    bootstrap_parameters: Option<HydrologyBootstrapParameters>
    forcing_schedule: bounded ordered HydrologyForcingSpec values
    resolution_policy: HydrologyResolutionPolicy
    limits_schema: 1
}
```

`HydrologyForcingSpec` has the forcing record's `forcing_id`, `scheduled_tick`, target members,
precipitation, potential ET, and external inflow, but no `origin_trace` and no producer-selected
policy. Configuration therefore never contains a trace that bootstrap has not created yet. Specs
are sorted and unique by `(scheduled_tick, forcing_id)` and validate against the same schedule,
target, overlap, and allocation bounds as canonical records.

`HydrologyBootstrapParameters` is a versioned, purely numeric schema:

```text
HydrologyBootstrapParameters {
    schema_version: 1
    default_surface_capacity: WaterVolume
    default_soil_capacity: WaterVolume
    default_groundwater_capacity: WaterVolume
    initial_surface: WaterVolume
    initial_soil: WaterVolume
    initial_groundwater: WaterVolume
    infiltration_rate_mm_per_second: u64
    percolation_fraction_num: u32
    percolation_fraction_den: NonZeroU32
    specific_yield_num: u32
    specific_yield_den: NonZeroU32
    aquifer_base_offset_mm: i64
    baseflow_threshold: WaterVolume
    baseflow_fraction_num: u32
    baseflow_fraction_den: NonZeroU32
    base_surface_transmissivity_mm3_per_second: u64
    base_groundwater_transmissivity_mm3_per_second: u64
    roughness_reference_mm: NonZeroU64
    conveyance_capacity: WaterVolume
    conveyance_initial_storage: WaterVolume
    conveyance_inlet_capacity_per_tick: WaterVolume
    conveyance_release_fraction_num: u32
    conveyance_release_fraction_den: NonZeroU32
    default_boundary: HydrologyBoundaryCondition
    chart_overrides: bounded ordered map<ChartId, HydrologyBootstrapOverride>
    cell_overrides: bounded ordered map<HydrologyCellKey, HydrologyBootstrapOverride>
}
```

`HydrologyBootstrapOverride` contains exactly these optional fields:

```text
surface_capacity
soil_capacity
groundwater_capacity
initial_surface
initial_soil
initial_groundwater
infiltration_rate_mm_per_second
percolation_fraction_num
percolation_fraction_den
specific_yield_num
specific_yield_den
aquifer_base_offset_mm
baseflow_threshold
baseflow_fraction_num
baseflow_fraction_den
base_surface_transmissivity_mm3_per_second
base_groundwater_transmissivity_mm3_per_second
roughness_reference_mm
conveyance_capacity
conveyance_initial_storage
conveyance_inlet_capacity_per_tick
conveyance_release_fraction_num
conveyance_release_fraction_den
face_boundaries: ordered map<FaceDirection, HydrologyBoundaryCondition>
```

It cannot override schema versions, defaults, or either override map recursively, and cannot carry
a material identity, soil class, biome, named water body, or language string. Precedence is cell
override, then chart override, then default. Face-specific conditions override the resolved default
only for that exterior face. A required metric, substrate value, boundary record, or initial value
still missing after precedence resolution rejects bootstrap. Every initial storage must fit its
resolved capacity; every fraction must be in `[0, 1]`; all derived coefficients must fit their
destination type without saturation.

`HydrologyResolutionPolicy` is:

```text
HydrologyResolutionPolicy {
    schema_version: 1
    enabled: bool
    max_level: u8  // must be <= 4
}
```

When false, all hydrology chunks evaluate at level zero while retaining the same canonical fine
state. `ResolutionField` is already keyed directly by `ChartChunkCoord`; no ID adapter exists or is
added. Hydrology's resident chunk set must equal the resident terrain chunk set. Every hydrology
resident chunk must have exactly one resolution entry; extra resolution entries are allowed only
for a runtime-resident non-hydrology chunk and are ignored by hydrology. Active hydrology chunks
must be a subset of resident hydrology chunks. When the policy is enabled, a missing, duplicate, or
above-`max_level` entry rejects the evolution request rather than being clamped. Resolution never
changes tick cadence.

`RuntimeConfig::new` uses `HydrologyConfig::disabled()`: no field, edge, forcing, or rainfall is
defaulted into existing sessions. Its canonical recipe encoding contains `enabled = false`,
`bootstrap_parameters = None`, the version fields, and empty ordered metric/forcing collections.
Any disabled config containing bootstrap parameters or non-empty collections rejects as
noncanonical. The required hydrology persistence section still records an explicit disabled state
under digest schema 8. Enabling requires `bootstrap_parameters = Some(...)`, explicit validated
metrics, uniform numeric substrate defaults or complete per-cell substrate records, initial
storage, boundary conditions, and forcing.

Within the seventh bootstrap stage, the complete field, edge, boundary, resolution, and spec proposal
is validated before any event commits. Specs are ordered by `(scheduled_tick, forcing_id)`. The
stage commits exactly one `HYDROLOGY_BOOTSTRAP_EVENT_KIND` schedule-origin event whose cause is the
sixth bootstrap stage's already committed terminal completion trace, whose opaque policy is
`BOOTSTRAP_HYDROLOGY_FORCING_POLICY_V1`, and whose forcing effect fingerprint covers the canonical
bytes of the entire bounded spec schedule. After that event commits, the infallible installation step converts
each spec to a canonical `HydrologyForcingRecord` by copying the prevalidated numeric fields,
setting every record's `origin_trace` to that schedule-origin event, and setting
`producer_policy_schema` to the bootstrap policy. The seventh stage's terminal receipt depends on
that single origin trace. A failed preflight or origin commit installs no hydrology state. Future
non-bootstrap producers construct canonical records directly only after their own origin event has
committed.

The bootstrap event has exactly seven aggregate effects on fixed
`CausalTarget(HYDROLOGY_BOOTSTRAP_OBJECT_KIND, object_id = 0, property)` identities, each
transitioning an absent fingerprint to the BLAKE3 digest of one canonical payload: metrics,
substrate, boundary conditions, initial storage, conveyance edges, resident/active resolution state,
and forcing specs. Their property IDs are allocated below. Its result digest covers the ordered
seven fingerprints.
Every initial cell bucket, edge, resolution anchor, and installed forcing record uses this event as
its initial change trace; the seventh stage terminal receipt cites the same trace. This keeps
bootstrap within the eight-effect and sixteen-cause limits while preserving a verifiable path from
every initialized carrier to its exact canonical bootstrap bytes. Import requires the event's sole
cause to equal the persisted sixth-stage completion trace and recomputes every aggregate target and
fingerprint.

`HydrologyObjectRegistry` avoids lossy hashing into `CausalTarget`'s `u64` object slot. Bootstrap
sorts all chart-qualified cell keys, edge keys, forcing identities, and resolution chunk keys
independently and assigns dense `u64` ordinals starting at zero within their distinct object kinds.
The four key-to-ordinal tables are persisted, digested, and validated as bijections on import.
Carrier proposal subjects and causal targets use these ordinals; the seven bootstrap aggregates use
the fixed object above. Unknown, duplicate, skipped, or out-of-order assignments reject.

Bootstrap constructs every cell from the terrain lattice. For each cell it creates at most one
outgoing conveyance edge toward the lowest strictly lower-elevation four-face neighbor. Equal lower
candidates break ties by canonical cell key; a local minimum has no outlet. Because every edge
strictly lowers elevation, the generated graph is acyclic. Terrain roughness adjusts
the configured surface transmissivity:

```text
adjusted_cell_transmissivity =
  floor(base_transmissivity * roughness_reference_mm
        / (roughness_reference_mm + cell_roughness_mm))
```

Surface material identity has no hydraulic meaning in this tranche and is not mapped to
permeability or a soil class.

Per-second bootstrap parameters are converted to per-tick solver coefficients:

```text
infiltration_limit_per_tick =
  floor(infiltration_rate_mm_per_second * cell_area_mm2 * timestep_millis / 1000)

surface_conductance_mm2_per_tick =
  floor(adjusted_cell_transmissivity_mm3_per_second * timestep_millis
        / (1000 * orthogonal_edge_length_mm))

groundwater_conductance_mm2_per_tick =
  floor(base_groundwater_transmissivity_mm3_per_second * timestep_millis
        / (1000 * orthogonal_edge_length_mm))
```

Groundwater transmissivity is not roughness-adjusted. Cell area, timestep, edge length, and surface
roughness are therefore causal inputs rather than unused metadata.

### 5. Deterministic process order

One hydrology Physics execution consists of frozen substages. Every substage reads one immutable
state and produces a complete next-state delta; no cell observes an earlier cell's same-substage
write.

Hydrology requires an atomic intra-batch DAG extension to `CausalTraceStore`.
`CausalEventDagProposal` causes are either `Existing(TraceId)` or
`Local(CausalEventProposalKey)`. `commit_dag_batch` first validates unique keys, all external traces,
all local references, acyclicity, the per-event cause/effect caps, aggregate store capacity, and a
canonical topological order. That order is produced by Kahn's algorithm: the ready set is ordered by
the proposal key's complete canonical bytes, the lexicographically least ready key is removed first,
and every local cause must have a lower `substage_ordinal` or the same ordinal with an acyclic
dependency. The encoded ordinal is therefore a validation boundary, while ready-key order is the
sole tie-breaker. It reserves final trace/event IDs in that exact order, resolves local causes, and
appends the complete batch only after every check succeeds; failure leaves the store byte-identical.
Existing `commit_batch` behavior and all existing subsystem event IDs stay unchanged. Hydrology uses
local causes so later substages cite the committed-within-the-same-atomic-batch event that produced
their actual input state.

1. **Forcing acceptance.** Add scheduled precipitation/external inflow to surface storage using
   checked arithmetic. Record accepted and rejected amounts.
2. **Infiltration.** Move
   `min(surface, infiltration_limit_per_tick, soil_capacity - soil_water)` from surface to soil.
3. **Percolation/recharge.** Move
   `floor(soil_water * percolation_fraction_num / percolation_fraction_den)`, bounded by groundwater
   capacity, from soil to groundwater. Any capacity-limited remainder stays in soil.
4. **Evapotranspiration.** Meet potential ET first from surface storage, then from soil storage.
   Groundwater is not directly withdrawn in this tranche. Record accepted and unmet demand.
5. **Surface routing.** Compute internal lateral and open-surface-boundary demand concurrently from
   the same frozen water-surface state, then apply one donor reduction.
6. **Groundwater routing and baseflow.** Compute groundwater lateral demand from water-table head;
   open-groundwater-boundary demand and baseflow participate in the same frozen-state donor
   reduction.
7. **Conveyance routing.** Release stored conveyance water toward its outlet using the linear
   storage-discharge rule below.
8. **Boundary export finalization.** Materialize sink receipts for boundary transfers already
   accepted in substages 5 and 6. This substage computes no new demand and removes no additional
   water. No-flux boundary channels retain the water.
9. **Conservation preflight.** Recompute every storage bucket and source/sink term in `i128`.
10. **Causal commit and installation.** Sort event proposals, commit them in `Phase::Physics`,
    install trace IDs, then atomically replace state and receipts.

### 6. Surface and groundwater transfer

Surface head:

```text
surface_head_mm = terrain_elevation_mm + floor(surface_volume / cell_area_mm2)
```

Groundwater head:

```text
saturated_depth_mm =
    floor(groundwater_volume * specific_yield_den
          / (cell_area_mm2 * specific_yield_num))
groundwater_head_mm = aquifer_base_elevation_mm + saturated_depth_mm
```

`specific_yield_num == 0` is invalid when groundwater storage is enabled.

For each same-chart orthogonal face, process the canonical endpoint pair exactly once. If
`head_a > head_b`:

```text
face_conductance =
    if conductance_a == 0 or conductance_b == 0:
        0
    else:
        floor(2 * conductance_a * conductance_b
              / (conductance_a + conductance_b))
raw_demand =
    floor(face_conductance * (head_a - head_b))
```

Use surface endpoint conductances for surface flow and groundwater endpoint conductances for
groundwater flow. The harmonic rule is symmetric; all products and sums use checked `i128`. The
reverse direction applies when `head_b > head_a`. Equal head produces zero demand.

Several outgoing faces may demand more water than the donor owns. First scale all donor demands
proportionally:

```text
base_i = floor(raw_i * available / sum_raw)
remainder_i = (raw_i * available) mod sum_raw
```

Distribute the remaining units by descending remainder, then ascending canonical edge key. The sum
of accepted outgoing transfers is therefore exactly `available`, independent of insertion order.

Then apply receiver constraints globally. For each receiver, sum donor-limited incoming demands. If
the sum exceeds remaining receiver capacity, scale those incoming demands with the same
largest-remainder rule, breaking ties by `(receiver_key, donor_key, edge_key)`. Rejected incoming
water remains in its donor. Because receiver scaling only decreases already donor-bounded transfers,
one donor pass followed by one receiver pass satisfies both constraints without iteration.

Surface flow enters the neighbor's surface storage by default. It enters a face's conveyance edge
instead only when that edge is directed from the donor toward its outlet and the computed head
direction matches. Edge inflow is bounded by `inlet_capacity_per_tick` and remaining edge capacity.
A reverse-head transfer never enters or reverses the directional edge; it follows the ordinary
surface-to-neighbor path.
Groundwater flow enters neighbor groundwater. Baseflow uses:

```text
baseflow_excess = max(0, groundwater - baseflow_threshold)
baseflow_requested =
    floor(baseflow_excess * baseflow_fraction_num / baseflow_fraction_den)
```

Accepted baseflow is the minimum allowed by groundwater availability, the outgoing edge's remaining
storage capacity, and its remaining `inlet_capacity_per_tick` after surface inflow. A cell with no
outgoing edge retains groundwater. Baseflow competes in the donor reducer with groundwater lateral
outflow, using the same canonical largest-remainder rule. Every accepted transfer subtracts and
adds the same integer amount.

For each exterior face, the surface channel reads `surface_head_mm` during substage 5 and the
groundwater channel reads `groundwater_head_mm` during substage 6. Each open channel's raw export is:

```text
max(0, selected_head_mm - external_head_mm) * conductance_mm2_per_tick
```

Surface export participates in the surface donor constraint with internal surface outflows.
Groundwater export participates in the groundwater donor constraint with internal groundwater
outflows and baseflow. Equal/lower head exports zero. Substage 8 only records the accepted amounts
as sinks.

### 7. Conveyance storage-discharge routing

Conveyance is physical edge storage, not a semantic river classification.

For frozen pre-release edge storage `S`:

```text
release = min(S, floor(S * release_fraction_num / release_fraction_den))
```

The fraction validates `0 <= num <= den`. If the outlet cell has its own outgoing conveyance edge,
release enters that downstream edge, subject to its remaining storage and per-tick inlet capacity;
otherwise release enters the outlet cell's surface storage, subject to remaining surface capacity.
All raw releases are computed before any receiver is updated. When multiple source edges target the
same downstream edge or surface cell and exceed its remaining capacity or inlet limit, reduce them
proportionally with the Section 6 largest-remainder rule, breaking equal remainders by ascending
source edge key. Capacity-limited release remains in its source edge. Every edge computes release
from the complete frozen pre-release edge state, so water received during this substage cannot
release again until the next tick and no same-tick cascade is possible. Flat or closed depressions
retain storage. Full dynamic-wave behavior, backwater, flow reversal, pressurization, and
infrastructure control are outside this plan.

### 8. Exact conservation and provenance

Each tick emits one terminal `HydrologyConservationReceipt`:

```text
storage_before =
    surface_before + soil_before + groundwater_before + conveyance_before
sources = accepted_precipitation + accepted_external_inflow
storage_after =
    surface_after + soil_after + groundwater_after + conveyance_after
sinks = accepted_et + boundary_exports
residual = storage_before + sources - storage_after - sinks
```

All terms are `i128`. `residual` must equal zero before any event is committed.

Transfer receipts include:

- batch sequence and tick;
- process kind as an opaque numeric ID, not a human string;
- canonical source and target;
- requested, accepted, and unaccepted volume;
- before/after storage;
- causal parents; and
- committed transfer and storage-change trace IDs.

Limiters are explicit receipt data. They never disappear as clamping.

Causal fan-in is fixed before DAG construction:

- each applied forcing record emits one application event citing its one origin; for each targeted
  cell, one forcing-settlement event folds all records in canonical order, cites that cell's prior
  surface/soil traces plus at most six distinct origins, and fingerprints the ordered per-record
  allocations so receipts remain attributable without becoming causes. Its effect transitions the
  cell object's `HYDROLOGY_FORCING_PROPERTY`; the resulting fingerprint/trace are persisted on the
  cell even when accepted water and ET are zero;
- infiltration cites the cell's forcing-settlement event when present (otherwise its existing
  surface trace) plus its existing soil trace;
- percolation cites the infiltration event when present (otherwise the current soil reference) plus
  its existing groundwater trace;
- ET cites the current surface reference (infiltration, forcing settlement, or existing surface),
  the current soil reference (percolation, infiltration, or existing soil), and the forcing
  settlement carrying ET demand; a zero-demand cell emits no ET event;
- surface-routing events cite the cell's current surface reference, at most four neighboring current
  surface references, and the one outgoing edge's existing trace when applicable;
- groundwater-routing events cite the cell's current groundwater reference, at most four neighboring
  current groundwater references, and the outgoing edge's existing trace;
- edge-inflow events cite the edge's existing trace plus the local source surface/groundwater routing
  results;
- each edge-release allocation cites its source edge's local post-inflow event, the receiver's local
  post-inflow/routing event, and at most three competing source-edge local events;
- each receiver settlement cites its prior local receiver event and at most four accepted release
  allocation events;
- fine bucket/edge `last_change` anchors point to their terminal local settlement event, so receipt
  eviction does not remove ancestry; and
- coarse execution still emits terminal events per affected fine bucket/edge.

For every bucket, “current reference” means the latest local event in the preceding list that
actually changed or settled that bucket, otherwise its pre-tick `Existing(TraceId)`. A process with
zero accepted transfer emits no bucket-change event, except forcing settlement, which exists whenever
a scheduled record targets the cell so accepted/rejected source and ET demand share one durable
input anchor. This alias rule is part of canonical proposal construction and prevents optional
zero-work events from changing the DAG.

To bind terminal conservation without unbounded fan-in, hydrology builds a canonical 16-ary
aggregation tree over all terminal fine bucket, edge, resolution, and forcing-application local
event keys. Leaf carrier bytes use `HydrologyCarrierKey` version 1, including resolution variant
`0x05`. Bucket tags are `0x01=surface`, `0x02=soil`, `0x03=groundwater`,
`0x04=forcing-input`, `0x05=conveyance`, `0x06=resolution`, and
`0x07=forcing-record`, and `0x08=coarse-process`.

`CausalEventProposalKey` version 1 bytes are:

```text
0x01 version
substage_ordinal: u8
process_kind: u32 big-endian
carrier_length: u16 big-endian
carrier_key_bytes
local_ordinal: u32 big-endian
```

The local ordinal starts at zero among otherwise equal key prefixes and must be contiguous. Synthetic
coarse-input leaves, coarse-input nodes, coarse-process events, and terminal aggregate nodes all
draw object IDs from the one persisted `next_hydrology_batch_node_id` counter. Before proposal-key
construction, allocate them in this order: coarse groups by
`(tick, block_key, constitutive_group_key, process_kind)`; within each group, member leaves by cell
key, then input nodes bottom-up and left-to-right, then its process event; only after all coarse
groups have allocated their complete trees and process events, allocate the one terminal tree's
nodes bottom-up and left-to-right. Each allocated synthetic event uses batch object kind,
its allocated ID, and `HYDROLOGY_COARSE_INPUT_PROPERTY` for coarse leaves/nodes/processes or
`HYDROLOGY_BATCH_ROOT_PROPERTY` for terminal nodes. Its carrier key is variant `0x06` containing the
same ID and its proposal-key `local_ordinal` is zero. No other proposal may consume this counter.

Terminal leaf *records* sort by
`(carrier_key_bytes, bucket_kind_byte, proposal_key_bytes)`. A record fingerprint is BLAKE3 over the
length-prefixed domain bytes `causafera.hydrology.batch-leaf.v1`, bucket tag, length-prefixed carrier
key, length-prefixed local proposal key, effect count, and each ordered effect target/before/after
fingerprint with fixed-width target fields and length-prefixed fingerprints. One settlement event
may be terminal for multiple bucket/carrier records; each record remains in this ordered list and
fingerprint input. Within an aggregate node's cause list, repeated proposal keys are stable-deduplicated
at first occurrence, while every record fingerprint remains in the node payload. Thus duplicate
terminal memberships neither violate cause uniqueness nor erase a conserved carrier.

At level zero, split leaf records into consecutive groups of at most 16. Each aggregate node cites
the stable-distinct local keys in that group and fingerprints the length-prefixed domain bytes
`causafera.hydrology.batch-node.v1`, level `u32` big-endian, child count `u8`, and each
length-prefixed child fingerprint in order.
At every later level, group the prior level's nodes consecutively by 16 and apply the same formula.
One leaf still produces one level-zero aggregate root with one cause. Zero leaves produce one
level-zero root with zero causes and the node formula's `child_count = 0`. Nodes receive consecutive
IDs in the shared allocation order defined above; the terminal portion remains bottom-up: all
level-zero groups left-to-right, then level one left-to-right, continuing through the root last.
Every node effect transitions its fixed target from absent to the node fingerprint, and its result
digest is that fingerprint. Import
reconstructs every group, fingerprint, ID, and the next counter. The conservation event cites this
always-present local root, the previous conservation trace, and at most eight forcing origins. Thus
every event stays within `MAX_HYDROLOGY_CAUSES_PER_EVENT = 16`, while the durable trace DAG retains
every actual same-tick/cross-bucket/routing/capacity dependency after typed receipts are evicted.

Allocate new runtime identifiers without renumbering existing values:

```text
HYDROLOGY_SYSTEM_ID = 13
HYDROLOGY_FORCING_EVENT_KIND = 35
HYDROLOGY_CELL_CHANGE_EVENT_KIND = 36
HYDROLOGY_EDGE_TRANSFER_EVENT_KIND = 37
HYDROLOGY_CONSERVATION_EVENT_KIND = 38
HYDROLOGY_BOOTSTRAP_EVENT_KIND = 39
HYDROLOGY_REPRESENTATION_EVENT_KIND = 40
HYDROLOGY_BATCH_AGGREGATE_EVENT_KIND = 41
HYDROLOGY_COARSE_INPUT_LEAF_EVENT_KIND = 42
HYDROLOGY_COARSE_INPUT_AGGREGATE_EVENT_KIND = 43
HYDROLOGY_COARSE_PROCESS_EVENT_KIND = 44
HYDROLOGY_CELL_OBJECT_KIND = 14
HYDROLOGY_EDGE_OBJECT_KIND = 15
HYDROLOGY_FORCING_OBJECT_KIND = 16
HYDROLOGY_BOOTSTRAP_OBJECT_KIND = 17
HYDROLOGY_RESOLUTION_OBJECT_KIND = 18
HYDROLOGY_BATCH_OBJECT_KIND = 19
HYDROLOGY_SURFACE_PROPERTY = 25
HYDROLOGY_SOIL_PROPERTY = 26
HYDROLOGY_GROUNDWATER_PROPERTY = 27
HYDROLOGY_CONVEYANCE_PROPERTY = 28
HYDROLOGY_FORCING_PROPERTY = 29
HYDROLOGY_BATCH_SEQUENCE_PROPERTY = 30
HYDROLOGY_RESOLUTION_PROPERTY = 31
HYDROLOGY_BOOTSTRAP_METRICS_PROPERTY = 32
HYDROLOGY_BOOTSTRAP_SUBSTRATE_PROPERTY = 33
HYDROLOGY_BOOTSTRAP_STORAGE_PROPERTY = 34
HYDROLOGY_BOOTSTRAP_EDGES_PROPERTY = 35
HYDROLOGY_BOOTSTRAP_RESOLUTION_PROPERTY = 36
HYDROLOGY_BOOTSTRAP_FORCING_PROPERTY = 37
HYDROLOGY_BOOTSTRAP_BOUNDARIES_PROPERTY = 38
HYDROLOGY_BATCH_ROOT_PROPERTY = 39
HYDROLOGY_COARSE_INPUT_PROPERTY = 40
```

Before implementation, verify these values remain unused at the current target commit. If a prior
accepted change has claimed one, update this plan's Decision log before selecting the next unused
value; do not silently renumber.

### 9. Multi-resolution contract

Hydrology resolution is independent of terrain, mana, population, and observer resolution.

| Representation | Decision |
| --- | --- |
| Delete fine state on demotion and reconstruct on promotion | Rejected: invents detail/provenance and cannot guarantee conservation. |
| Keep independent coarse and fine authoritative states | Rejected: creates competing truths and a reconciliation problem. |
| Retain fine canonical state; compute coarse proposals and allocate back | Chosen: one truth, exact storage, reversible detail changes, measurable work reduction. |

- Fine cell/edge state remains canonical and resident.
- A coarse unit is an ordered chart-grid block. At level `L`,
  `block_edge = 2^min(L, 4)` and membership uses global terrain-cell coordinates, never chunk
  extent.
- Cells inside a block are partitioned into canonical constitutive groups by exact
  `(metric, substrate, boundary-kind)` tuple. Each group sums area, capacity, storage, forcing, and
  per-tick limits.
- Coarse forcing is the exact sum of member forcing.
- Vertical processes execute once per constitutive group. Every accepted group delta is returned to
  fine members by the capped largest-remainder reducer defined below.
- Internal lateral faces are not evaluated at coarse resolution.
- Every fine face on a block boundary remains authoritative and is evaluated once from its frozen
  fine endpoints. Its accepted transfer is installed directly on those fine endpoints.
- Coarse block boundary totals are receipt and validation aggregates of those accepted fine-face
  transfers only. They are never netted and redistributed.
- Heterogeneous boundary conductance is therefore neither averaged nor replaced by an invented
  constitutive class.
- Capacity-aware largest-remainder back-allocation applies only to aggregate vertical-process and
  forcing deltas within a constitutive group.
- `HydrologyResolutionState { level, last_change }` is persisted per chunk. Demotion and promotion
  commit `HYDROLOGY_REPRESENTATION_EVENT_KIND` against that chunk's dense resolution object ordinal
  and `HYDROLOGY_RESOLUTION_PROPERTY`; the prior `last_change` is the cause and the new trace becomes
  the anchor. No canonical storage is deleted.
- Promotion reactivates retained state and never synthesizes detail.
- If a coarse delta cannot be allocated without violating capacity, the proposal rejects atomically;
  it does not spill into an invented bucket.

The capped largest-remainder reducer takes a total `T`, a canonical cell-key-ordered vector of
nonnegative weights `w_i`, and per-member ceilings `c_i`, requiring `T <= sum(c_i)`. With remaining
total `R = T`, discard full/zero-weight members, compute
`base_i = floor(R * w_i / sum(w))`, grant `min(base_i, remaining_c_i)`, subtract all grants from
`R`, and repeat if any member capped. When no member caps in a round, distribute that round's
remaining units by descending `(R_before * w_i) mod sum(w)`, then ascending cell key, one unit per
member. Checked `i128` arithmetic is
used throughout. A positive total with no eligible positive-weight member rejects.

Process-specific weights and ceilings are fixed:

- precipitation/external inflow: process each
  `(scheduled_tick, forcing_id, precipitation|external_inflow)` separately in canonical order;
  weight is the member's already allocated fine forcing request and ceiling is remaining surface
  capacity;
- infiltration: both weight and ceiling are
  `min(surface, infiltration_limit_per_tick, remaining_soil_capacity)`;
- percolation: weight is the member's raw fraction result; ceiling is that result bounded by
  remaining groundwater capacity;
- ET surface and soil passes: process each `(scheduled_tick, forcing_id, et_bucket)` separately;
  surface weight is the member's already allocated fine ET demand and ceiling is available surface
  water; soil weight is that member's demand left after its allocated surface withdrawal and ceiling
  is available soil water.

For every invocation, first compute the process's raw group candidate with the Section 5 equation:
the per-record requested source/demand total, aggregate infiltration minimum, aggregate percolation
fraction result, or remaining ET demand. Then set
`T = min(raw_group_candidate, sum(member_ceilings))`. The reducer never receives an unallocatable
ordinary candidate; `T > sum(c_i)` remains an internal error. This explicitly handles quantization
such as two one-unit soil cells at fraction one-half, where aggregate percolation rounds to one but
all fine ceilings round to zero.

Each allocation applies paired subtraction/addition to the same fine member where the process is an
internal transfer. Source and sink allocations update the matching fine forcing receipt. The
accepted group total, sum of fine deltas, and paired bucket changes must agree exactly or the entire
proposal rejects.

Coarse process ancestry is itself bounded and canonical. For every evaluated
`(tick, block_key, constitutive_group_key, process_kind)`:

1. Build one input leaf per fine member, sorted by cell key. The leaf fingerprint is BLAKE3 over
   the length-prefixed domain `causafera.hydrology.coarse-input-leaf.v1`, tick, length-prefixed block
   and constitutive-group keys, process kind, length-prefixed cell carrier bytes, unsigned
   big-endian `u128` weight and ceiling, reference count, and the ordered current-reference trace IDs
   plus their fixed-width effect targets and length-prefixed effect fingerprints. Its event cites
   those current references (at most three), transitions its allocated synthetic target from absent
   to the leaf fingerprint, and uses that fingerprint as its result digest.
2. Group leaf events consecutively by 16 and build bottom-up input nodes with the same level/group ID
   order and length-delimited node formula as the terminal tree, using domain
   `causafera.hydrology.coarse-input-node.v1`. Each node transitions its allocated synthetic target
   from absent to its node fingerprint and uses that fingerprint as its result digest.
3. With one member, still create one one-child root. With zero members, create one zero-cause empty
   root with `child_count = 0`.
4. Emit one coarse-process event citing that root. Its fingerprint is BLAKE3 over the
   length-prefixed domain `causafera.hydrology.coarse-process.v1`, tick, length-prefixed block and
   constitutive-group keys, process kind, unsigned big-endian `u128` raw candidate, summed ceilings,
   accepted `T`, and the input-root fingerprint. It transitions its allocated synthetic target from
   absent to that fingerprint and uses the fingerprint as its result digest. Fine allocation events
   cite this local process event plus their own current bucket references.

Input leaves use `HYDROLOGY_COARSE_INPUT_LEAF_EVENT_KIND`; input nodes use
`HYDROLOGY_COARSE_INPUT_AGGREGATE_EVENT_KIND`; the process event uses
`HYDROLOGY_COARSE_PROCESS_EVENT_KIND`. All use the shared persisted aggregate-node ID counter and
`HYDROLOGY_COARSE_INPUT_PROPERTY`. The terminal tree includes each group's coarse-process event
under bucket tag `0x08=coarse-process`, including `T = 0`, so zero-change evaluation remains
durably reconstructable. Import rebuilds every input tree, current-reference set, weight, ceiling,
candidate, `T`, allocation, fingerprint, and ID.

Resolution may alter approximation detail and therefore future trajectory. It may not alter total
water, topology, metric geometry, boundary openness, or causal ancestry.

The runtime owns the resolution adapter. It validates the directly
`ChartChunkCoord`-keyed `ResolutionField` against the resident/active rules above and supplies the
per-chunk level to `HydrologyEvolutionRequest`. Hydrology evaluates every tick at every level; this
changes spatial work, not temporal cadence. `Phase::Resolution` changes apply on the next tick
because Physics already completed. No `causafera-domains -> causafera-resolution` dependency is
added.

### 10. Scheduler compatibility

Register `HydrologyRuntimeSystem` after every existing registration in `Runtime::new`. Although it
runs in `Phase::Physics`, appending it preserves the implicit IDs and RNG streams of existing
systems.

Stage 1 captures executable pre-hydrology evidence:

- current registration IDs/stream samples for fixed world seed, tick, and phase;
- physical and history digests for fixed configurations;
- snapshot export/import bytes and resumed digests.

After hydrology registration, existing registration IDs/stream samples and a canonical projection of
all pre-hydrology subsystem state/section payloads must remain byte-identical with hydrology
disabled. Full envelope bytes and full physical/history/experiment digests are expected to differ
because the recipe, required section set, and shared schema marker intentionally advance. Tests
assert that the difference is exactly the declared versioned footprint, not impossible whole-file
identity. Hard-coded constants or comments alone do not satisfy this gate.

Hydrology uses no RNG in this tranche. Forcing is explicit state.

When hydrology is enabled, `Runtime::tick` executes through `RuntimeTickTransaction`. At tick start,
it stages every mutable tick-owned value: `RuntimeState`, causal trace store/counters, simulation
time, scheduler/RNG stream counters, pending inputs and their consumption cursors, receipt-retention
counters, and the hydrology aggregate-node counter. Immutable system registrations/configuration are
shared. Every phase and system, including hydrology Physics and later actor/lifecycle/resolution
work, reads/writes only the staged values.

After the final phase, the transaction validates all domain invariants and canonically encodes the
complete would-be snapshot envelope. Only if it is at most persistence `MAX_TOTAL_SIZE` does one
infallible swap publish every staged value. Any system error, DAG error, invariant failure, or
oversized final envelope drops the staging transaction and leaves authoritative state, traces,
counters, queues, time, and RNG streams byte-identical. Hydrology-disabled ticks retain the frozen
legacy path and Stage 1 byte/stream evidence. This whole-tick boundary, not a Physics-local estimate,
guarantees that a later-phase event cannot make an accepted hydrology tick unexportable.

### 11. Persistence and digest

- Allocate `HYDROLOGY_SECTION_ID = 0x000F`.
- Start `HYDROLOGY_SECTION_MAJOR = 1`, minor 0.
- Advance `RUNTIME_RECIPE_SECTION_MAJOR` from 6 to 7.
- Advance `CURRENT_DIGEST_SCHEMA_VERSION` from 7 to 8.
- Encode all maps and sets in canonical key order.
- Read and validate section byte length and every declared collection count against the hard bounds
  before allocating.
- Persist metrics, substrate, field state, conveyance state, active/resident resolution state and
  anchors, forcing schedules and `applied_at` state, transfer batches, conservation receipts,
  boundary records, object-registry bijections, and per-bucket/edge trace anchors.
- Before accepting configuration/import and before committing any tick, deterministically compute
  the exact canonical encoded hydrology-section length after required whole-batch eviction, then the
  exact complete snapshot envelope length including every other section, causal-trace bytes, header,
  and section directory. Reject if hydrology exceeds `MAX_HYDROLOGY_SECTION_BYTES` or the complete
  envelope exceeds persistence `MAX_TOTAL_SIZE = 256 MiB`; no accepted state may become
  unexportable.
- Enforce `MAX_HYDROLOGY_TOTAL_FORCING_MEMBERS` across the complete schedule before allocating member
  vectors, in addition to the per-record cap.
- Include all authoritative hydrology state and forcing in the physical digest.
- Include hydrology causal events in the history digest through the existing trace store.
- `ExperimentDigest` also carries `CURRENT_DIGEST_SCHEMA_VERSION`; schema 8 deliberately changes its
  identity marker even though hydrology is not experiment state. Tests pin this version-only impact.
- Reject missing section `0x000F` under digest schema 8.
- Reject section major other than 1.
- Reject old digest schemas under the existing fail-closed policy; do not fabricate empty hydrology.
- Validate canonical ordering, uniqueness, bounds, capacities, cell counts, edge endpoints,
  chart/metric identity, receipt arithmetic, forcing ancestry, trace existence, batch continuity,
  active/resident containment, and terminal conservation before accepting imported state.

### 12. Explanation and observer

Explanation adds typed claims for:

- storage and water-table range;
- forcing ancestry and accepted/unmet forcing;
- transfer path and limiter evidence;
- exact conservation residual;
- boundary export; and
- insufficient/unsupported evidence.

Observer API adds bounded optional hydrology fields to protocol V1:

- per-chunk surface/soil/groundwater rasters;
- conveyance storage/flow summaries;
- latest forcing and conservation summary;
- bounded transfer receipts by chart-qualified scope; and
- trace anchors.

Existing Rust and TypeScript `RuntimeSummary` fields 28–35 carry bootstrap summaries, but
`query.proto` currently stops at field 27. Stage 7 first backfills the protobuf mirror without
changing wire bytes:

```text
RuntimeSummary bootstrap mirror:
  28 bootstrap_schema_version            uint32
  29 bootstrap_plan_id                   uint64
  30 bootstrap_world_seed                uint64
  31 bootstrap_stage_count               uint32
  32 bootstrap_complete                  bool
  33 bootstrap_configured_population     uint64
  34 bootstrap_configured_promotion_limit uint32
  35 bootstrap_receipts                  repeated BootstrapReceipt

BootstrapReceipt:
   1 stage                               uint64
   2 completed_at                        uint64
   3 result                              bytes (exactly 32 bytes)
   4 completion_trace                    uint64
   5 dependency_traces                   repeated uint64
```

These declarations mirror the already-shipped custom wire contract and do not allocate new
semantics. Hydrology begins after that occupied range. The exact additive V1 hydrology
representation is:

```text
RuntimeSummary:
  36 hydrology_summary_schema_version    uint32
  37 hydrology_total_surface             bytes (canonical unsigned u128 LEB128)
  38 hydrology_total_soil                bytes (canonical unsigned u128 LEB128)
  39 hydrology_total_groundwater         bytes (canonical unsigned u128 LEB128)
  40 hydrology_total_conveyance          bytes (canonical unsigned u128 LEB128)
  41 hydrology_latest_residual           bytes (canonical ZigZag i128 LEB128)
  42 hydrology_active_chunk_count        uint32
  43 hydrology_latest_forcing_tick        optional uint64
  44 hydrology_latest_forcing_id          optional uint64
  45 hydrology_latest_forcing_origin      optional uint64
  46 hydrology_latest_accepted_source     bytes (canonical unsigned u128 LEB128)
  47 hydrology_latest_accepted_et         bytes (canonical unsigned u64 LEB128)

WorldChunkSnapshot:
   9 hydrology_deltas                    repeated HydrologyCellDelta
  10 hydrology_delta_schema_version      uint32
  11 hydrology_transfer_summaries        repeated HydrologyTransferSummary
  12 hydrology_transfer_schema_version   uint32
  13 hydrology_conveyance_summaries      repeated HydrologyConveyanceSummary
  14 hydrology_conveyance_schema_version uint32

FieldRasterKind:
   4 surface water volume
   5 soil water volume
   6 groundwater volume

FieldRaster hydrology extension:
  13 unsigned_values                    bytes (packed canonical u64 LEB128)
  14 unsigned_values_schema_version     uint32
```

For hydrology raster kinds, legacy signed `values` and `auxiliary` bands are empty, schema field 14
equals `HYDROLOGY_RASTER_VALUES_SCHEMA_V1 = 1`, and field 13 contains exactly `edge * edge * depth`
shortest-form `u64` values. Rust exposes `Vec<u64>` and TypeScript exposes `BigUint64Array`; neither
converts through `i64` or `Float64Array`. Non-hydrology rasters reject fields 13–14.

`HydrologyCellDelta` has:

```text
 1 chart_id                  uint64
 2 chunk_x                   sint32
 3 chunk_y                   sint32
 4 chunk_z                   sint32
 5 cell_ordinal              uint32
 6 surface_before            bytes (canonical unsigned LEB128)
 7 surface_after             bytes (canonical unsigned LEB128)
 8 soil_before               bytes (canonical unsigned LEB128)
 9 soil_after                bytes (canonical unsigned LEB128)
10 groundwater_before        bytes (canonical unsigned LEB128)
11 groundwater_after         bytes (canonical unsigned LEB128)
12 net_forcing               bytes (canonical ZigZag i128 LEB128)
13 net_lateral_flow          bytes (canonical ZigZag i128 LEB128)
14 transition_trace_id       uint64
15 conservation_trace_id     uint64
16 transition_tick           uint64
```

`HydrologyTransferSummary` has:

```text
 1 process_kind              uint32
 2 source_key                bytes (canonical HydrologyCarrierKey encoding)
 3 target_key                bytes (canonical HydrologyCarrierKey encoding)
 4 requested_volume          bytes (canonical unsigned LEB128)
 5 accepted_volume           bytes (canonical unsigned LEB128)
 6 unaccepted_volume         bytes (canonical unsigned LEB128)
 7 transfer_trace_id         uint64
 8 conservation_trace_id     uint64
 9 tick                      uint64
10 forcing_origin_trace_id   optional uint64
```

`HydrologyConveyanceSummary` has:

```text
1 edge_key                   bytes (canonical HydrologyCarrierKey encoding)
2 storage                    bytes (canonical unsigned LEB128)
3 capacity                   bytes (canonical unsigned LEB128)
4 accepted_inflow            bytes (canonical unsigned LEB128)
5 accepted_release           bytes (canonical unsigned LEB128)
6 last_change_trace_id       uint64
7 tick                       uint64
```

`HydrologyCarrierKey` version 1 has exact fixed encodings:

- variant `0x01`, cell, length 23: variant; chart `u64`; chunk x/y/z as two's-complement `i32`;
  ordinal `u16`, all big-endian;
- variant `0x02`, edge, length 45: variant followed by two 22-byte cell bodies in ascending key
  order;
- variant `0x03`, exterior face, length 24: variant, one 22-byte cell body, direction byte
  `0=-X, 1=+X, 2=-Y, 3=+Y`; and
- variant `0x04`, forcing record, length 17: variant, scheduled tick `u64`, forcing ID `u64`,
  big-endian; and
- variant `0x05`, resolution chunk, length 21: variant, chart `u64`, chunk x/y/z as two's-complement
  `i32`, all big-endian; and
- variant `0x06`, batch aggregate node, length 9: variant and the persisted node `u64` ID,
  big-endian. This variant is used only for local DAG proposal keys and trace inspection, not as a
  physical transfer endpoint.

Decoders reject unknown variants/directions, wrong lengths, noncanonical endpoint order, trailing
bytes, and source/target duplicates where the process requires distinct carriers. A transfer
summary's unique canonical key is
`(tick, process_kind, source_key_bytes, target_key_bytes, transfer_trace_id)`.

`HYDROLOGY_SUMMARY_SCHEMA_V1 = 1`, `HYDROLOGY_DELTA_SCHEMA_V1 = 1`,
`HYDROLOGY_TRANSFER_SUMMARY_SCHEMA_V1 = 1`,
`HYDROLOGY_CONVEYANCE_SUMMARY_SCHEMA_V1 = 1`, `MAX_HYDROLOGY_DELTAS = 64`,
`MAX_HYDROLOGY_TRANSFER_SUMMARIES = 64`, and `MAX_HYDROLOGY_CONVEYANCE_SUMMARIES = 64`. Projection
keeps the latest tick first and canonical cell, transfer-key, or edge-key order within a tick.
On a new-engine payload, runtime fields 36–42 are one atomic required group even when hydrology is
disabled, in which case totals/count/residual are zero. Fields 43–47 describe the greatest applied
`(scheduled_tick, forcing_id)` and are either all present or all absent; the accepted-source value is
precipitation plus external inflow. A pre-hydrology payload may omit all fields 36–48. Partial
groups, duplicate scalar fields, wrong wire types, unknown summary schema, and noncanonical zero
encodings reject.

`MAX_QUERY_PAYLOAD_BYTES = 1 MiB` remains the request-payload cap. Add a distinct
`MAX_QUERY_RESPONSE_PAYLOAD_BYTES = 1 MiB` and enforce it before allocating during response decode
and before emitting `QueryResponse.payload`. Duplicate cell, transfer, or conveyance keys within a
tick reject decoding. Unsigned and signed byte integers must use their shortest canonical encoding;
values outside the declared `u64`/`u128`/`i128` field domain reject.

Hydrology Explanation does not widen `NumericClaimValue` or its wire schema. Exact per-carrier
`u64` water volumes are encoded as existing `Ratio { numerator, denominator: 1 }` values; minimum
and maximum volume are separate claims rather than an unsigned range. Signed head/depth values use
existing `Scalar`/`Range`. A committed conservation residual is exactly zero and uses
`Scalar { value: 0 }`. Whole-scope totals outside `u64` and any nonzero/uncommitted residual return
typed insufficiency; they are never narrowed. This keeps `explanation.proto`, Rust, and TypeScript
V1 numerically lossless without a new claim-value variant.

The canonical runtime has seven bootstrap receipts, but the already-shipped V1 observer field 35
retains its frozen six-entry cap and continues to contain stages one through six byte-for-byte. The
seventh hydrology receipt is projected separately in optional field 48:

```text
RuntimeSummary:
  48 hydrology_bootstrap_receipt         optional BootstrapReceipt
```

It is absent when hydrology is disabled or bootstrap has not completed, and otherwise must contain
stage seven. A new decoder rejects any other stage, a duplicate stage-seven receipt, or a field-35
count above six. This split lets a pre-hydrology V1 decoder accept the new payload by ignoring field
48 without weakening its existing six-receipt bound. Fields 28–35 remain the exact legacy six-stage
projection even when canonical runtime bootstrap has seven stages: field 31 is `6`, field 32 means
that stages one through six are complete, and field 35 contains exactly their six receipts in stage
order. New clients define complete hydrology bootstrap as that legacy completion predicate plus a
valid field-48 stage-seven receipt; they never reinterpret field 31 or field 32 as seven-stage
metadata.

The wire change is additive. A frozen pre-hydrology TypeScript V1 decoder oracle captured in Stage 1
must ignore the new fields when decoding a new payload. New clients must decode real hydrology
payloads and reject malformed/overflowing/duplicate/unsupported data. Update and compile:

- Rust observer API and wire crates;
- `proto/causafera/observer/v1/query.proto`;
- `packages/observer-protocol/src/index.ts`; and
- `tools/audit/test-observer-bootstrap-decoder.mjs`;
- `tools/audit/test-observer-hydrology-decoder.mjs`; and
- `tools/audit/test-observer-hydrology-legacy-decoder.mjs`.

No observer UI is included. No authoritative `EntityId`, semantic water-body classification, or
unbounded history is exposed.

## Primitive vs emergent review

Primitive physical state:

- volume, depth, elevation, area, edge length, timestep;
- storage capacity and conductance;
- chart-qualified cells and physical edges;
- explicit forcing and boundary conditions;
- accepted transfers and causal traces.

Emergent or downstream:

- river, stream, lake, pond, wetland, flood, drought, watershed, aquifer class, season, reliable
  water source, agricultural suitability, disease risk, settlement viability, and mana pattern.

The plan must not promote any emergent term into an authoritative enum, bootstrap label, event kind,
or routing shortcut.

## Non-goals

- Full climate or atmospheric generation.
- Geological formation, strata, deformation, or aquifer classification.
- Snow/ice accumulation or phase change.
- Sediment transport, erosion, solutes, salinity, pollutants, or water quality.
- Full Saint-Venant hydraulics, backwater, flow reversal, pressurization, coastal tides, or
  cross-chart ocean routing.
- Dams, pumps, weirs, canals, irrigation operations, municipal networks, or other infrastructure
  control.
- Ecology, agriculture, health, settlement, economy, history, biology, or mana coupling
  implementation.
- Semantic water-body or hazard labels in authoritative state.
- UI work or observer-driven activation.
- CUDA/GPU work or unmeasured scale claims.
- Migration shims that default absent hydrology into an old production snapshot.

These are exclusions of adjacent capabilities, not reductions of the requested hydrology cycle.

## Implementation stages

Each stage is one green wave and one local checkpoint commit. Before every checkpoint:

1. inspect `git status`;
2. inspect staged and unstaged diffs;
3. stage only the stage allowlist;
4. rerun the focused commands;
5. commit only when green; and
6. record the commit hash and exact evidence in Progress.

RED tests remain uncommitted until paired implementation makes the wave green. If checkpoint commits
are prohibited, stop after the first green wave and request authorization.

Implementation must not begin while this plan is Draft. Owner acceptance first changes the status
line to `Accepted`, moves this entry from Draft Plans to Active Plans in `PLANS.md`, and records the
acceptance date in Decision log. Stage 1 begins only after that governance transition.

### Stage 1 — Freeze contracts and capture legacy behavior

Files:

- `plans/hydrology.md`
- `PLANS.md`
- `docs/rfc/RFC-HYDRO-001.md`
- `docs/development/todo-backlog.md`
- `crates/causafera-runtime/tests/hydrology_legacy_compatibility.rs`
- `tools/audit/fixtures/observer-protocol-v1-pre-hydrology.mjs`
- `tools/audit/test-observer-hydrology-legacy-decoder.mjs`

Work:

- change RFC-HYDRO-001 from Proposed to Accepted only after its state, forcing, resolution,
  seasonality, units, ownership, and non-goal language matches this plan;
- create `TODO-HYDRO-001`;
- capture engine-executed legacy scheduler stream samples, registration order, pre-hydrology
  subsystem section payloads, state projections, and resume evidence before adding hydrology;
- freeze the current TypeScript V1 decoder as a test-only legacy oracle and record its source digest;
- make fixtures non-vacuous by asserting at least one existing system consumes an RNG stream and at
  least one tick changes physical/history state.

Focused gate:

```bash
cargo test -p causafera-runtime --test hydrology_legacy_compatibility -- --nocapture
node tools/audit/test-observer-hydrology-legacy-decoder.mjs
node tools/audit/check-entry-points.mjs
git diff --check
```

### Stage 2 — Fixed-point primitives and geography-owned state

Files:

- `crates/causafera-types/src/physics.rs`
- `crates/causafera-geography/src/lib.rs`
- delete `crates/causafera-geography/src/hydrology.rs`
- `crates/causafera-geography/src/hydrology/mod.rs`
- `crates/causafera-geography/src/hydrology/metric.rs`
- `crates/causafera-geography/src/hydrology/substrate.rs`
- `crates/causafera-geography/src/hydrology/forcing.rs`
- `crates/causafera-geography/src/hydrology/state.rs`

Work:

- add the numeric, metric, substrate, forcing, cell, field, edge, and active/resident state contracts;
- validate every bound and canonical ordering;
- add unit/property-style tests for conversion remainders, overflow, zero denominators, duplicate
  keys, invalid endpoints, incompatible extents, input-order independence, fixed 2D ordinal/seam
  mapping, and every hard allocation bound.

Focused gate:

```bash
cargo test -p causafera-types physics -- --nocapture
cargo test -p causafera-geography hydrology -- --nocapture
cargo clippy -p causafera-types -p causafera-geography --all-targets --all-features -- -D warnings
cargo fmt --all -- --check
git diff --check
```

### Stage 3 — Local vertical cycle and exact receipts

Files:

- `crates/causafera-core/src/lib.rs`
- `crates/causafera-core/src/provenance.rs`
- `crates/causafera-domains/src/lib.rs`
- `crates/causafera-domains/src/hydrology/mod.rs`
- `crates/causafera-domains/src/hydrology/parameters.rs`
- `crates/causafera-domains/src/hydrology/evolution.rs`
- `crates/causafera-domains/src/hydrology/proposal.rs`
- `crates/causafera-domains/src/hydrology/records.rs`
- `crates/causafera-domains/src/hydrology/receipts.rs`
- `crates/causafera-domains/tests/hydrology_vertical_cycle.rs`
- `crates/causafera-domains/tests/hydrology_conservation.rs`

Work:

- add atomic `commit_dag_batch` with local cause references, canonical topological ordering,
  cycle/missing-reference/capacity rejection, exact rollback, and unchanged legacy `commit_batch`
  tests;
- implement frozen forcing, infiltration, percolation, ET, checked storage updates, transfer
  receipts, and terminal conservation;
- independently recompute the receipt ledger in tests rather than trusting proposal totals;
- add negative controls for zero precipitation, saturated soil, zero infiltration, full groundwater,
  ET disabled, unmet ET, overflow, and atomic rollback.

Focused gate:

```bash
cargo test -p causafera-core provenance -- --nocapture
cargo test -p causafera-domains --test hydrology_vertical_cycle -- --nocapture
cargo test -p causafera-domains --test hydrology_conservation -- --nocapture
cargo clippy -p causafera-core -p causafera-domains --all-targets --all-features -- -D warnings
cargo fmt --all -- --check
git diff --check
```

### Stage 4 — Terrain routing, groundwater, conveyance, and seams

Files:

- `crates/causafera-domains/src/hydrology/evolution.rs`
- `crates/causafera-domains/src/hydrology/proposal.rs`
- `crates/causafera-domains/src/hydrology/records.rs`
- `crates/causafera-domains/src/hydrology/receipts.rs`
- `crates/causafera-domains/tests/hydrology_routing.rs`
- `crates/causafera-domains/tests/hydrology_boundaries.rs`
- `crates/causafera-domains/tests/hydrology_groundwater.rs`

Work:

- implement surface head, groundwater head, proportional donor allocation, baseflow, conveyance
  storage-discharge, and explicit boundary export;
- reuse chart-qualified same-chart neighbor topology;
- prove a seam face behaves like an interior face, each edge is processed once, the receiver changes,
  flat/closed storage accumulates, and construction order is irrelevant;
- prove no same-substage cascade through a chain of three or more cells/edges.

Focused gate:

```bash
cargo test -p causafera-domains --test hydrology_routing -- --nocapture
cargo test -p causafera-domains --test hydrology_boundaries -- --nocapture
cargo test -p causafera-domains --test hydrology_groundwater -- --nocapture
cargo clippy -p causafera-domains --all-targets --all-features -- -D warnings
cargo fmt --all -- --check
git diff --check
```

### Stage 5 — Conservative hydrology resolution

Files:

- `crates/causafera-domains/src/hydrology/resolution.rs`
- `crates/causafera-domains/src/hydrology/proposal.rs`
- `crates/causafera-domains/src/hydrology/evolution.rs`
- `crates/causafera-domains/src/hydrology/records.rs`
- `crates/causafera-domains/src/hydrology/receipts.rs`
- `crates/causafera-domains/tests/hydrology_resolution.rs`

Work:

- implement exact vertical-process aggregation and deterministic fine allocation, authoritative
  fine block-boundary transfer with aggregate validation receipts, promotion/demotion receipts, and
  capacity-failure rollback;
- implement constitutive grouping, global block addressing, and explicit fine boundary-face
  evaluation;
- prove demotion/promotion preserves every water bucket and topology;
- prove coarse mode evaluates fewer internal faces/groups on a non-vacuous heterogeneous fixture.

Focused gate:

```bash
cargo test -p causafera-domains --test hydrology_resolution -- --nocapture
cargo clippy -p causafera-domains --all-targets --all-features -- -D warnings
cargo fmt --all -- --check
git diff --check
```

### Stage 6 — Runtime commit, bootstrap, persistence, digest, and import validation

Files:

- `crates/causafera-runtime/src/lib.rs`
- `crates/causafera-runtime/src/config.rs`
- `crates/causafera-runtime/src/runtime.rs`
- `crates/causafera-runtime/src/bootstrap.rs`
- `crates/causafera-runtime/src/snapshots.rs`
- `crates/causafera-runtime/src/snapshot_sections.rs`
- `crates/causafera-runtime/src/digests.rs`
- `crates/causafera-runtime/src/hydrology.rs`
- `crates/causafera-runtime/src/hydrology_events.rs`
- `crates/causafera-runtime/src/hydrology_validation.rs`
- `crates/causafera-runtime/src/tick_transaction.rs`
- `crates/causafera-runtime/tests/support/hydrology.rs`
- `crates/causafera-runtime/tests/hydrology_runtime.rs`
- `crates/causafera-runtime/tests/hydrology_persistence.rs`
- `crates/causafera-runtime/tests/hydrology_determinism.rs`
- `crates/causafera-runtime/tests/hydrology_import_validation.rs`
- `crates/causafera-runtime/tests/hydrology_legacy_compatibility.rs`
- `crates/causafera-runtime/tests/historical_bootstrap.rs`
- `crates/causafera-runtime/tests/material_surface_loop.rs`
- `crates/causafera-runtime/tests/thermal_persistence.rs`
- `crates/causafera-runtime/tests/thermal_determinism.rs`
- `crates/causafera-runtime/tests/thermal_conservation_aggregates/neutrality.rs`
- `crates/causafera-lab/src/experiment.rs`
- `crates/causafera-observer-api/src/query.rs`
- `crates/causafera-observer-wire/src/protocol.rs`
- `crates/causafera-observer-wire/tests/protocol.rs`
- `apps/observer/src-tauri/src/session.rs`
- `packages/observer-protocol/src/index.ts`
- `tools/audit/test-observer-bootstrap-decoder.mjs`
- `tools/audit/test-hydrology-production-boundaries.mjs`
- `tools/audit/check-entry-points.mjs`
- `tools/audit/run-source-tests.mjs`

Work:

- append runtime registration without shifting legacy IDs/streams;
- add the hydrology-enabled whole-tick staging transaction and near-cap later-phase rollback;
- add and validate `HydrologyConfig`, with disabled default and bounded explicit enablement;
- validate directly chart-keyed `ResolutionField` resident/active coverage in the runtime adapter
  and prove a production-path transition affects the next tick, never observer attention;
- add the seventh production bootstrap stage and causal initialization receipts;
- preserve the Rust, TypeScript, observer-wire, and Tauri session field-35 bootstrap-summary cap at
  six and the field-31 projected stage count at six, preserve field 32 as legacy six-stage
  completion, and add the separately bounded optional field-48 stage-seven receipt in the same
  checkpoint, so frozen V1 consumers accept the additive payload;
- commit sorted forcing, transfer, storage, representation, and conservation events;
- allocate section `0x000F` major 1, runtime recipe major 7, and digest schema 8;
- persist and validate the complete hydrology state and ancestry;
- preflight exact section size and composed collection/receipt/forcing-member bounds so every
  accepted state remains exportable;
- prove malformed, forged, duplicate, unsorted, overflowed, trace-inconsistent, and
  conservation-inconsistent snapshots fail closed;
- prove replay, input order, export/import/export, save/resume, and disabled-hydrology legacy
  compatibility using legacy state/section projections while asserting the intentional full-envelope
  and shared-digest-schema changes;
- run a source/entry-point audit that rejects production fixture/demo hydrology and any use of
  `chunk_extent` as hydrology metric scale.

Focused gate:

```bash
cargo test -p causafera-runtime --test hydrology_runtime -- --nocapture
cargo test -p causafera-runtime --test hydrology_persistence -- --nocapture
cargo test -p causafera-runtime --test hydrology_determinism -- --nocapture
cargo test -p causafera-runtime --test hydrology_import_validation -- --nocapture
cargo test -p causafera-runtime --test hydrology_legacy_compatibility -- --nocapture
cargo test -p causafera-observer-api --all-features
cargo test -p causafera-observer-wire --all-features
cargo test -p causafera-observer --all-features
pnpm typecheck
node tools/audit/test-observer-bootstrap-decoder.mjs
node tools/audit/test-hydrology-production-boundaries.mjs
cargo clippy -p causafera-runtime --all-targets --all-features -- -D warnings
cargo fmt --all -- --check
git diff --check
```

### Stage 7 — Explanation and observer protocol

Files:

- `crates/causafera-explanation/src/ir.rs`
- `crates/causafera-explanation/src/classification.rs`
- `crates/causafera-explanation/src/render.rs`
- `crates/causafera-observer-api/src/lib.rs`
- `crates/causafera-observer-api/src/query.rs`
- `crates/causafera-observer-api/src/stream.rs`
- `crates/causafera-observer-wire/src/protocol.rs`
- `crates/causafera-observer-wire/tests/protocol.rs`
- `proto/causafera/observer/v1/query.proto`
- `packages/observer-protocol/src/index.ts`
- `crates/causafera-runtime/src/runtime.rs`
- `crates/causafera-runtime/src/snapshots.rs`
- `crates/causafera-runtime/src/field_raster.rs`
- `crates/causafera-runtime/tests/hydrology_observer.rs`
- `crates/causafera-runtime/tests/hydrology_explanation.rs`
- `apps/observer/src-tauri/src/session.rs`
- `tools/audit/test-observer-hydrology-decoder.mjs`
- `tools/audit/test-observer-hydrology-legacy-decoder.mjs`
- `tools/audit/test-observer-proto-schema.mjs`
- `tools/audit/check-entry-points.mjs`
- `tools/audit/run-source-tests.mjs`

Work:

- backfill the existing bootstrap-summary fields 28–35 and `BootstrapReceipt` in
  `query.proto`, byte-for-byte matching the Rust and TypeScript codecs, before adding hydrology
  fields;
- add bounded typed state, flux, forcing, conservation, and insufficiency evidence;
- add lossless unsigned hydrology raster bands, atomic summary presence rules, and the distinct
  response-payload cap;
- encode exact hydrology Explanation values through existing lossless numeric variants and return
  insufficiency rather than narrowing unsupported whole-scope totals;
- keep protocol V1 additive and verify old-client compatibility;
- round-trip Rust and TypeScript codecs with real engine payloads;
- add exact `.proto` schema-conformance audit coverage for all pre-existing bootstrap and new
  hydrology field/message numbers; the repository has no generated protobuf binding pipeline, so no
  generated-binding claim is made;
- adversarially decode malformed and unsupported hydrology payloads;
- prove locale, query cadence, viewport, and observer absence do not alter physical/history digests.

Focused gate:

```bash
cargo test -p causafera-explanation --all-features
cargo test -p causafera-observer-api --all-features
cargo test -p causafera-observer-wire --all-features
cargo test -p causafera-runtime --test hydrology_observer -- --nocapture
cargo test -p causafera-runtime --test hydrology_explanation -- --nocapture
cargo test -p causafera-observer --all-features
pnpm typecheck
node tools/audit/test-observer-hydrology-decoder.mjs
node tools/audit/test-observer-hydrology-legacy-decoder.mjs
node tools/audit/test-observer-proto-schema.mjs
node tools/audit/run-source-tests.mjs
git diff --check
```

### Stage 8 — Benchmarks, status documents, and full validation

Files:

- `apps/observer/src-tauri/examples/hydrology_bench.rs`
- `docs/performance/benchmarks.md`
- `docs/observer/protocol.md`
- `docs/architecture/observer.md`
- `docs/architecture/protocol.md`
- `docs/explanation/explanation-ir.md`
- `docs/world/hydrology.md`
- `docs/world/historical-bootstrap.md`
- `docs/world/spatial-hierarchy.md`
- `docs/ontology/causal-carriers.md`
- `docs/ontology/domain-coverage-matrix.md`
- `docs/development/todo-backlog.md`
- `CHANGELOG.md`
- `PLANS.md`
- `plans/hydrology.md`

Work:

- measure the representative workloads below;
- reconcile aspirational hydrology documentation with implemented behavior;
- reconcile observer/protocol/Explanation documents with the seventh bootstrap receipt, hydrology
  wire fields, unsigned rasters, numeric-value policy, and response cap;
- update maturity only to the evidence-supported range;
- close or advance `TODO-HYDRO-001`;
- record every checkpoint and command in Progress;
- run the complete validation suite.

## Verification

Every integration scenario uses engine-produced state. Pure constructor/reducer bound tests may use
direct validated value objects and a bounded stub trace resolver, but may not support replay,
bootstrap, persistence, provenance, or maturity claims. Multi-batch checks require at least three
committed hydrology batches and assert that receipts are non-empty before evaluating them.

### V1 — Numeric and metric rejection

Reject zero metric area/edge/timestep, invalid fractions, negative signed inputs, overflow,
incompatible field lengths, scheduled ticks outside the bootstrap-relative horizon, per-record and
aggregate forcing-member overflow, and every configured hard bound. The
`tools/audit/test-hydrology-production-boundaries.mjs` source audit rejects any production
hydrology use of runtime `chunk_extent` as a metric.

### V2 — Precipitation source ancestry

Accepted precipitation changes surface storage, cites the persisted forcing origin trace, and appears
exactly once in the conservation source term. Weighted multi-cell allocation sums to the record
total; duplicate members reject; overlapping records reduce canonically; a non-resident member
rolls back the whole proposal. The production-bootstrap integration case proves every record shares
the one schedule-origin trace and that its result digest covers the ordered specs. Separately, a
proposal-level bound test uses direct valid records plus a stub resolver for six distinct existing
origins and proves cell-event fan-in remains at or below 16; a seventh cell origin rejects before proposal
construction. That lower-level case is only forward-compatibility coverage for a future producer
and is not evidence that production currently has a second forcing producer. Applied forcing
records remain available after typed receipt eviction.

### V3 — Zero-forcing negative control

With no forcing and closed boundaries, total water remains constant and no forcing event appears.
When no bucket/edge/resolution/forcing record changes, the canonical empty level-zero aggregation
root still commits and the conservation event cites it.

### V4 — Infiltration bounds

Infiltration never exceeds surface availability, per-tick limit, or remaining soil capacity.

### V5 — Saturated-soil counterfactual

With identical forcing and terrain, saturated soil produces less infiltration and more retained
surface/runoff water than unsaturated soil.

### V6 — Percolation and groundwater capacity

Percolation follows the fixed fraction, remains in soil when groundwater is full, and creates no
unaccounted loss.

### V7 — ET sink and unmet demand

ET draws surface then soil, never groundwater, and records accepted plus unmet demand. Zero ET emits
no ET transfer/event.

### V8 — Surface downhill response

Two equal-storage cells with different terrain elevation transfer toward lower head; equal heads
produce zero transfer.

### V9 — Donor and receiver oversubscription

Multiple outgoing face demands are proportionally limited; accepted amounts sum exactly to donor
availability and use canonical remainder ordering. Simultaneous incoming demands above receiver
capacity are reduced canonically, with rejected water retained by each donor.

### V10 — Frozen substage

A three-cell chain cannot move one unit through two faces in the same routing substage.

### V11 — Same-chart seam equivalence

The same physical two-cell setup produces identical flux when the cells are within one chunk or
across a same-chart chunk seam.

### V12 — Edge processed once

Construction and insertion permutations produce one receipt per canonical edge and identical final
state/digests.

### V13 — Explicit boundary behavior

No-flux retains water; open export removes the same amount recorded in the boundary sink ledger.
The open-boundary head/conductance equation is asserted exactly. Missing resident neighbors never
silently export or block without a boundary record.

### V14 — Groundwater head and flow

Groundwater moves toward lower computed head, respects explicit groundwater capacity and specific
yield, and conserves exact volume.

### V15 — Baseflow and conveyance delay

Groundwater above threshold follows the exact excess-times-fraction equation, enters conveyance
storage only within groundwater/edge/inlet bounds, and competes canonically with groundwater
lateral outflow. Conveyance releases no more than frozen pre-release storage, respects downstream
edge or outlet-surface capacity, retains any limited remainder, chooses the cell's one canonical
outgoing edge, and cannot cascade through multiple edges in one tick. A permutation-tested case with
at least three upstream edges oversubscribing one downstream edge asserts the exact
largest-remainder allocation, source-edge tie-break, and retained remainder.

### V16 — Closed basin conservation

Across at least 100 ticks with nonzero internal transfers and no external sources/sinks,
`storage_before == storage_after` exactly.

### V17 — Whole-budget conservation

Across forcing, ET, routing, baseflow, and export,
`before + sources == after + sinks` for every tick and for the aggregate run.
The terminal conservation event's 16-ary local-cause tree reaches every terminal fine bucket, edge,
resolution, and forcing-application event. Tests cover zero, one, 16, 17, and multi-level leaf counts and import
reconstructs the same consecutive grouping, root fingerprint, bottom-up level/group IDs, and counter.
With at least two coarse groups, the test also reconstructs the shared synthetic-ID counter: all
leaves, nodes, and process events for every canonically ordered coarse group precede every ID in the
single terminal tree, and insertion permutations produce the same IDs and trace DAG.

### V18 — Atomic rollback

Any overflow, invalid target, incompatible metric, allocation failure, unknown cause, or nonzero
residual leaves state, traces, forcing-consumption state, counters, and digests unchanged.
`commit_dag_batch` cycle, missing-local-reference, duplicate-key, cause-cap, and reserved-capacity
failures are byte-identical rollbacks.
A near-`MAX_TOTAL_SIZE` hydrology-enabled tick whose later lifecycle/actor event would cross the cap
rejects the whole tick: Physics changes, later-phase changes, traces, queues, time, counters, and RNG
stream probes all match the pre-tick snapshot.

### V19 — Resolution aggregation

Fine-to-coarse groups exact constitutive tuples and sums every bucket exactly. Vertical and forcing
deltas allocate back with exact largest remainder. Internal lateral faces are skipped; each fine
block-boundary face is evaluated once and installed on its actual endpoints; block totals exactly
aggregate those receipts and are never redistributed. The non-vacuous benchmark evaluates fewer
internal process groups/faces than fine mode. Zero-, one-, 16-, 17-, and multi-level-member coarse
input trees reproduce exact weights, ceilings, current references, process totals, fingerprints, and
bottom-up node IDs after import, including an evaluated `T = 0` group.

### V20 — Resolution demotion/promotion

Demotion and later promotion preserve storage, topology, metrics, and retained fine provenance; no
detail is synthesized.

### V21 — Coarse allocation failure

An unallocatable coarse delta rejects atomically rather than overflowing a fine cell or inventing a
sink.

### V22 — Legacy registration/RNG stability

With hydrology disabled, pre-hydrology registration IDs/stream probes and canonical projections of
legacy subsystem state and section payloads remain byte-identical. Full envelopes and
physical/history/experiment digests differ only through the declared recipe/required-section/schema
version footprint.

### V23 — Deterministic replay and input order

Same seed, metrics, substrate, forcing, and tick count produce identical state, receipts, traces, and
digests under every tested map/input insertion order.

### V24 — Persistence canonical round-trip

Export/import/export bytes are identical under hydrology section major 1 and digest schema 8.
Physical, history, and experiment digest schema markers all equal 8; the experiment digest's
version-only identity change is pinned. The maximum accepted configured state exports below
`MAX_HYDROLOGY_SECTION_BYTES` and the complete envelope exports below `MAX_TOTAL_SIZE`; adding one
cell, edge, boundary/override entry, forcing member, retained receipt, trace event, or encoded byte
beyond the applicable component or complete-envelope cap rejects before allocation/commit.

### V25 — Malformed and forged import

Reject missing/duplicate/unsorted records, bad version, overflow, bad capacities, invalid endpoints,
unknown traces, forged forcing ancestry, receipt arithmetic changes, batch gaps, and conservation
forgery with specific errors.

### V26 — Save/resume equivalence

An uninterrupted `2N`-tick run matches `N` ticks, save/import, then `N` ticks in state, receipts,
traces, forcing consumption, and digests.

### V27 — Observer neutrality

All supported locales, query cadences, viewports, and observer absence produce identical
authoritative digests.

### V28 — Observer/wire compatibility

The frozen pre-hydrology V1 decoder oracle accepts a new engine payload because field 35 remains at
its six-receipt bound and it ignores additive hydrology fields 36–48;
new Rust and TypeScript decoders round-trip the same payload and reject
malformed/overflowing/duplicate data. The test pins existing bootstrap fields 28–35 byte-for-byte,
the atomic hydrology summary fields 36–47, the stage-seven receipt in field 48, chunk fields 9–14,
hydrology raster fields 13–14, all
declared schema markers, all three 64-entry summary bounds, forcing carrier identity, current conveyance
storage, exact carrier lengths/directions, response-payload cap, canonical transfer keys, and
shortest-form integer encodings. The exact `.proto` schema-conformance audit pins the existing
bootstrap group/nested receipt and every hydrology declaration to the Rust/TypeScript wire contract;
the repository has no generated-binding pipeline. Rust `Vec<u64>` and TypeScript
`BigUint64Array` raster values round-trip values above `i64::MAX` exactly.

### V29 — Explanation evidence

Supported claims cite authoritative traces and exact typed values; unknown scope or missing history
returns insufficiency without fabricated classification. A per-carrier volume above `i64::MAX`
round-trips as ratio-over-one, while a whole-scope total above `u64::MAX` returns insufficiency
instead of narrowing.

### V30 — Production bootstrap provenance

Every metric, substrate, initial storage bucket, edge, resolution anchor, and forcing record traces
to the seventh canonical bootstrap event and its terminal receipt. Its seven aggregate fingerprints
recompute from canonical payload bytes, the result digest covers them in order, and the persisted
dense object registries are collision-free bijections. The named production-boundary source audit
proves no fixture/demo constructor appears in production paths.

### V31 — Metric and parameter counterfactuals

Holding other inputs fixed: doubling timestep doubles derived per-tick infiltration/conductance until
an explicit bound engages; doubling edge length halves derived conductance subject to integer
remainder; increasing roughness reduces surface conductance; changing cell area changes
volume-to-depth and infiltration volume in the stated units. Roughness alone does not change
groundwater conductance; doubling base groundwater transmissivity doubles its derived conductance
until a bound engages. Surface material identity alone has no effect.

### V32 — Production resolution transition

An engine-driven, directly `ChartChunkCoord`-keyed `ResolutionField` change passes exact
resident/active coverage validation, changes hydrology grouping on the next tick, preserves
water/topology/ancestry, and is identical with and without any observer. Missing resident entries,
extra nonresident entries, and levels above policy reject.

### V33 — Bounds and retention

Every decode-before-allocation cap rejects `limit + 1`; accepted receipt history retains at most the
latest eight whole batches and 262,144 transfer receipts in canonical tick order, evicts older typed
detail without deleting causal traces, and makes old Explanation requests return insufficiency.
Pre-commit exact-size accounting includes trace bytes and all non-hydrology sections and proves every
accepted state remains exportable below the existing 256 MiB total snapshot cap.

### V34 — Full validation

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
node tools/audit/test-hydrology-production-boundaries.mjs
node tools/audit/test-observer-hydrology-decoder.mjs
node tools/audit/test-observer-hydrology-legacy-decoder.mjs
```

Skipped, unavailable, interrupted, or failing commands must be reported exactly and cannot be
recorded as passing.

## Benchmark plan

Use a checked-in headless benchmark harness with fixed seed, explicit metric/substrate/forcing, and
non-vacuous transfers.

Workloads:

1. one chunk, local vertical cycle only;
2. three production-generated line chunks, including both active seam faces;
3. nine chunks, surface plus groundwater plus conveyance;
4. the same nine chunks at fine and coarse hydrology resolution;
5. snapshot export/import with at least three receipt batches; and
6. 100-, 1,000-, and 10,000-tick run-length sweeps to expose receipt/digest growth.

For each workload:

- warm up before measurement;
- run at least 10 repetitions;
- record exact command, commit, build profile, toolchain, OS, CPU, memory, cell/edge count, tick
  count, evaluated vertical groups, evaluated internal/boundary faces, receipt count, mean, median,
  standard deviation, and min/max;
- assert exact conservation during the measured run; and
- report raw evidence in the plan Progress section or a linked checked-in benchmark document.

No absolute pass threshold or scale claim is declared before baseline measurement. Regressions
introduced during a later wave require a measured explanation or remediation before checkpoint.
The retained-fine/coarse design is accepted as a performance architecture only if the coarse
workload evaluates fewer vertical groups and internal faces than the fine workload while all
conservation, replay, topology, and ancestry checks remain green; timing alone is not the oracle.

## Determinism impact

- Authoritative hydrology uses checked integer arithmetic and ordered maps/edges.
- Rounding remainders remain in donor storage or use canonical largest-remainder allocation.
- Every substage reads a frozen state.
- Forcing is explicit persisted state; no wall clock or hidden RNG is used.
- Hydrology registration is appended; legacy streams and legacy subsystem projections are pinned,
  while full digest identities change deliberately with schema 8.
- Resolution aggregation and allocation are canonical.
- Observer and Explanation access cannot select active hydrology resolution or alter state.
- Digest schema 8 deliberately distinguishes hydrology-bearing state from prior schemas.

## Memory impact

Expected standing costs per hydrology cell:

- three `u64` storage values;
- substrate parameters and capacities;
- current and prior trace/state anchors;
- active/resident bookkeeping; and
- optional bounded recent receipt indexing.

Per-edge costs include storage, capacity, release/inlet limits, outlet, and traces. Exact byte counts and
growth are measured in Stage 8. Standing state is capped at 128 chunks/131,072 cells/262,144 edges;
typed history at eight batches and 262,144 transfers; the hydrology section at 192 MiB after
whole-batch eviction; and the complete snapshot, including trace history and all other sections, at
the existing 256 MiB persistence cap. Exact pre-commit encoding makes those bounds compositional
rather than aspirational. Authoritative trace retention still follows INV-014 within that exportable
envelope. Any unbounded per-tick secondary map discovered during measurement becomes a recorded
risk/TODO before maturity claims.

## Observer impact

Observer read models gain bounded hydrology rasters, storage/flow summaries, forcing/conservation
evidence, and trace anchors. Protocol V1 remains because additions are optional and wire-compatible.
The plan does not add UI.

## Explanation impact

Explanation gains typed hydrology claims and insufficiency states. Claims use physical volumes,
heads, transfers, storage ranges, source/sink terms, and traces. Human labels such as flood or river
remain observer analysis and never become causal input.

## Persistence impact

- new required hydrology section `0x000F`, major 1;
- runtime recipe section major 6 → 7;
- shared physical/history/experiment digest schema 7 → 8;
- complete forcing, state, resolution, receipt, boundary, and trace persistence;
- strict fail-closed import for absent/unsupported/forged state; and
- canonical round-trip and save/resume requirements.

No old snapshot is defaulted into an empty-water world.

## Cross-domain effects

- Terrain provides elevation, roughness, surface material identity, and neighbor topology.
- Future geology may generate `HydraulicSubstrateCell` values.
- Future climate may generate `HydrologyForcingRecord` values.
- Future ecology, biology, agriculture, settlement, economy, health, and mana may consume typed
  hydrology state or traces only through later accepted coupling plans.
- Hydrology does not mutate those adjacent domains in this tranche.

## Risks

### R1 — No existing metric cell size

Using chunk extent as metres would violate INV-043. The explicit chart-keyed hydrology grid metric is
a hard prerequisite, persisted and traced from bootstrap.

### R2 — Silent conservation loss at bounds

Saturating arithmetic or final clamping would create a hidden source/sink. Checked preflight,
explicit limiters, and independently recomputed receipts are mandatory.

### R3 — Directional bias and same-tick cascade

Sequential in-place routing would depend on cell order. Frozen substages, canonical edge ownership,
and order-permutation tests close this risk.

### R4 — Climate/geology smuggling

Hard-coded rainfall seasons or semantic soil/geology labels would implement adjacent M0 domains by
stealth. Typed forcing/substrate interfaces and explicit non-goals prevent this.

### R5 — Semantic water-body shortcuts

Named river/lake/flood enums would violate the no-semantic-label rule. Only measurable storage,
geometry, conductance, and history are authoritative.

### R6 — Resolution creates or deletes detail

Aggregate-only state would make promotion fabricate distribution. Retained fine canonical state and
exact allocation are mandatory.

### R7 — Legacy RNG drift

Inserting a system registration can shift every later system ID. Capture executable pre-change
evidence and append hydrology last.

### R8 — Persistence forgery

A self-consistent section can still contradict traces, forcing, or conservation. Import validation
must cross-check materialized state, receipts, registered forcing-origin ancestry, and trace events.

### R9 — Receipt/digest growth

Per-tick hydrology data may compound existing snapshot and digest costs. The run-length benchmark
must measure this before scale claims.

### R10 — Overclaiming physical accuracy

Fixed-point exactness is computational, not scientific exactness. Documentation must state routing
assumptions, quantization, and excluded hydraulic regimes.

## Documentation changes

Stage 1:

- accept and expand `docs/rfc/RFC-HYDRO-001.md`;
- register this plan in `PLANS.md`; and
- create `TODO-HYDRO-001`.

Stage 8, after evidence:

- align `docs/world/hydrology.md` with implemented carriers and exclusions;
- document hydrology resolution in `docs/world/spatial-hierarchy.md`;
- add carriers to `docs/ontology/causal-carriers.md`;
- update the maturity matrix conservatively;
- update CHANGELOG with observed behavior and version changes; and
- move this plan from Active to Completed only when every stage and verification gate is complete.

## TODO changes

Create `TODO-HYDRO-001: Conserved Multi-Resolution Hydrology` in Stage 1:

- **Status:** In Progress after the plan is accepted.
- **Dependencies:** accepted RFC-HYDRO-001; existing terrain continuity and cross-chunk neighbor
  contracts; provenance, persistence, and resolution foundations.
- **Acceptance:** all carriers/processes, conservation, provenance, resolution, persistence,
  observer/Explanation, and verification in this plan.
- **Out of scope:** the Non-goals above.

Close it only after Stage 8 evidence. Any measured but unsolved performance or cross-chart issue gets
its own evidence-backed follow-up TODO rather than being hidden in Progress.

## Decision log

- **2026-07-29 — Plan accepted.** The project owner accepted revision 19 as the authoritative
  implementation contract. Implementation is authorized to proceed through the staged green-wave
  checkpoints below.
- **2026-07-29 — Full hydrology scope, not an invented MVP.** The request was open-ended; repository
  intent and the maturity gap jointly require the complete conserved surface/subsurface cycle,
  routing, aggregation, persistence, and inspectability.
- **2026-07-29 — Code/matrix settle current status; world docs settle target intent.** Hydrology is
  M0 today despite aspirational documentation.
- **2026-07-29 — Split ownership.** Geography owns canonical state and inputs; domains owns
  evolution; runtime orchestrates. This preserves the existing dependency direction.
- **2026-07-29 — Terrain-aligned 2D lattice.** Hydrology uses the terrain surface's fixed
  `CHUNK_SIZE²` row-major cells and four-face adjacency; runtime `chunk_extent` is unrelated.
- **2026-07-29 — Cubic-millimetre volume and explicit grid metric.** This matches terrain's
  millimetre elevation while avoiding chunk-as-metric violations. `i128` intermediates and checked
  conversion prevent silent overflow.
- **2026-07-29 — No authoritative semantic water-body labels.** Persistent storage/flow structures
  remain physical; named classifications are downstream.
- **2026-07-29 — Explicit forcing instead of climate generation.** Tick-indexed persisted forcing
  with producer-neutral committed origins resolves seasonality without implementing Climate.
- **2026-07-29 — Disabled runtime default.** Existing runtime construction adds no water or forcing;
  hydrology requires explicit validated configuration and causal bootstrap.
- **2026-07-29 — Two-sided transfer reduction.** Donor availability is applied first, receiver
  capacity second, with canonical largest-remainder allocation and rejected water retained by donors.
- **2026-07-29 — Same-batch provenance requires an atomic local-cause DAG.** A validated
  `commit_dag_batch` extension resolves local proposal keys only after full-batch preflight, keeping
  legacy commits unchanged while preserving durable substage ancestry and atomic rollback.
- **2026-07-29 — Retained fine state under resolution.** Coarse computation is conservative and
  allocates back to retained state; demotion/promotion never destroys or invents water.
- **2026-07-29 — Continuity/storage-discharge routing.** Full dynamic hydraulics is an adjacent depth
  tranche, not required for the repository's current hydrology contract.
- **2026-07-29 — Append scheduler registration and pin legacy behavior.** Existing system IDs/RNG
  streams and legacy subsystem projections are protected by executable byte-identity evidence;
  full envelopes/digests change intentionally with versioned schemas.
- **2026-07-29 — Version allocations.** Hydrology section `0x000F` major 1, runtime recipe major 7,
  digest schema 8, additive observer protocol V1.
- **2026-07-29 — Claude Code compatibility.** This canonical file follows `PLANS.md` and assumes no
  external orchestration framework, agent roles, command aliases, or tool-specific control files.
- **2026-07-29 — Bounded causal composition.** A tick and target cell accept at most six forcing
  origins; coarse execution emits fine-grained events. This fits the existing 16-cause event bound
  without losing persisted origin detail.
- **2026-07-29 — Fine boundary faces remain authoritative in coarse mode.** Coarse lateral totals
  are validation/read-model aggregates only; no boundary delta is redistributed.
- **2026-07-29 — Observer tags start at 36.** Runtime summary fields 28–35 already carry bootstrap
  data. Stage 7 first backfills their missing protobuf declarations; hydrology then uses additive
  fields 36–48 and exact bounded chunk messages under protocol V1.
- **2026-07-29 — Boundary demands share their physical donor reductions.** Surface and groundwater
  open-boundary demand are frozen and reduced with the corresponding internal outflows; the later
  boundary substage only finalizes sink receipts.
- **2026-07-29 — Convergent conveyance is globally reduced.** Competing releases use proportional
  largest remainder and canonical source-edge ties against one receiver's capacity/inlet bound.
- **2026-07-29 — Bootstrap accepts trace-free forcing specs.** The seventh stage preflights the full
  proposal, commits one digest-bearing schedule-origin event, then infallibly installs canonical
  records sharing that committed trace. This keeps bootstrap receipt fan-in bounded independently of
  schedule size.
- **2026-07-29 — Dense causal object registries.** Chart-qualified keys cannot be losslessly packed
  into one `u64`; canonical per-kind dense bijections provide collision-free causal object IDs.
- **2026-07-29 — Bootstrap aggregates use fixed causal targets.** The seventh-stage event cites the
  sixth-stage completion trace and writes seven allocated fingerprint properties on bootstrap object
  zero; no nonexistent stage-start trace or unaddressed aggregate remains.
- **2026-07-29 — Boundary input is bootstrap evidence.** Canonical boundary bytes have their own
  seventh aggregate effect/property; groundwater transmissivity is explicitly not roughness-adjusted.
- **2026-07-29 — Durable bucket ancestry fits the cause cap.** Cell origin fan-in is six; surface,
  soil, groundwater, and edge events include all cross-bucket pre-state dependencies within 16.
- **2026-07-29 — Symmetric face conductance and capped coarse allocation.** Endpoint conductance uses
  a checked harmonic mean; every aggregate vertical delta uses the fully specified iterative capped
  largest-remainder reducer.
- **2026-07-29 — Bounds compose through exact encoding.** State/receipt maxima were reduced, a total
  forcing-member cap added, and exact pre-commit section plus complete-envelope sizing guarantees
  exportability under the existing 256 MiB snapshot cap.
- **2026-07-29 — Hydrology observer values remain lossless in V1.** Unsigned raster bands use packed
  `u64` plus `BigUint64Array`; summary totals use atomic `u128` byte fields; Explanation reuses exact
  existing variants or returns insufficiency rather than narrowing.
- **2026-07-29 — Exportability is a whole-tick contract.** Hydrology-enabled ticks stage every
  mutable phase and publish only after the final complete envelope passes the 256 MiB cap; a
  Physics-only size check cannot protect against later-phase growth.
- **2026-07-29 — V1 preserves the six-receipt bootstrap field.** The seventh canonical receipt is
  exposed in optional hydrology field 48, while fields 28–35 retain the exact six-stage projection,
  stage count, and legacy completion meaning. Frozen V1 decoders therefore retain both their
  field-31 and field-35 bounds and ignore the additive hydrology projection.
- **2026-07-29 — Synthetic DAG IDs and ordering are global.** Coarse leaves/nodes/processes and
  terminal nodes share one persisted counter with a fixed allocation order; Kahn ready-key order,
  exact targets, length-delimited fingerprints, and duplicate terminal membership handling make the
  committed DAG independent of proposal insertion order.
- **2026-07-29 — Stage 1: every reserved identifier verified unused at the target commit.** As
  Section 8 requires, all thirty allocations were checked against `29fdf4d`. Live values are system
  IDs 10–12, 19–21, 30, 42, 60–61; event kinds 1, 3, 4, 6–15, 17, 28–34; object kinds 1–13; and
  properties 1–4, 6–13, 20–24. Hydrology's system ID 13, event kinds 35–44, object kinds 14–19, and
  properties 25–40 are all free. Section IDs stop at `0x000E`, `RUNTIME_RECIPE_SECTION_MAJOR` is 6,
  `CURRENT_DIGEST_SCHEMA_VERSION` is 7, and `MAX_TOTAL_SIZE` is 256 MiB, exactly as the plan's
  Current state records. No renumbering was needed.
- **2026-07-29 — Stage 1 deviation: no existing system consumes an RNG stream, so the fixture pins
  stream *keys* instead.** Stage 1's work list asks the legacy fixture to assert that at least one
  existing system consumes an RNG stream. Measured at `29fdf4d`, none does: all eleven `System::run`
  implementations in `causafera-runtime` bind the parameter as `_stream`. Writing that assertion
  would have made it pass vacuously or not at all. What R7 actually threatens is the stream *key* —
  a registration inserted anywhere but last renumbers every later system, and
  `StreamKey { world_seed, time, phase, system_id }` seeds each stream — so
  `hydrology_legacy_compatibility.rs` pins the assigned IDs and the samples those IDs key, and
  proves the pinning is sensitive by showing a one-step ID shift changes every sample. The other
  half of the non-vacuity requirement, that at least one tick changes physical and history state, is
  asserted unchanged. The plan's Stage 6 legacy expectations are unaffected: appending hydrology
  last must still leave IDs 0–10 and their sample table identical.
- **2026-07-29 — Stage 1 deviation: registration IDs are observed, not declared.**
  `runtime_system_registrations()` declares the phase and order of each system, and snapshot import
  compares against it, but the declaration would move together with an inserted registration and so
  cannot detect one. `Runtime` now records `Scheduler::register_system`'s own return values in a
  read-only `scheduler_registrations()` manifest, and the Stage 1 fixture asserts that the live
  scheduler, the persisted recipe manifest, and the captured baseline all agree. This touches
  `crates/causafera-runtime/src/runtime.rs`, which is on Stage 6's allowlist rather than Stage 1's;
  it was moved forward because Section 10's rule that "hard-coded constants or comments alone do not
  satisfy this gate" cannot otherwise be met. Nothing consumes the manifest to execute.
- **2026-07-29 — Stage 1: the frozen decoder oracle is a compiled artefact with a pinned digest.**
  `tools/audit/fixtures/observer-protocol-v1-pre-hydrology.mjs` is the `tsc` emission of
  `packages/observer-protocol/src/index.ts` at source SHA-256
  `79c351d9…0322fa76`, commit `29fdf4d`. Transcribing it by hand was rejected: a paraphrased oracle
  proves compatibility with the paraphrase. Its own SHA-256 is pinned in the Stage 1 audit test, so
  an edit is a failure rather than a silent re-freeze, and the test additionally proves at freeze
  time that the frozen copy and the live decoder agree field-for-field on real payloads and reject
  the same malformed ones.

- **2026-07-29 — Stage 2 representation choices, all preserving the stated contract.** Four places
  where the implementation expresses a plan schema differently without changing what it holds or
  validates. (a) The four `*_fraction_num`/`*_fraction_den` pairs are carried as one
  `HydraulicFraction { numerator: u32, denominator: NonZeroU32 }`; `numerator()` and `denominator()`
  are still what gets encoded, and the plan's `0 <= num <= den` rule is enforced once instead of at
  four construction sites and every override merge. (b) `HydraulicSubstrateKey` — the
  `(metric, substrate, boundary-kind)` tuple's substrate half — is 88 canonical big-endian bytes
  rather than an eleven-element tuple, because Stage 5's coarse-process fingerprint needs exactly
  those bytes and a second encoding would be free to drift from the first. The one signed field's
  sign bit is flipped so byte order matches numeric order. (c) Constructors with eight or more
  same-typed arguments take a `*Parts` struct, so a transposition is a compile error rather than a
  silently different world. (d) `HydrologyCellKey::neighbor` returns `Option`: a chunk address that
  would leave `i32` has no neighbour, which callers already handle as an exterior face. Wrapping
  would join the two ends of a chart through arithmetic nobody modelled.
- **2026-07-29 — Stage 2 measurement: `i128` alone does not make a product safe.** The accumulation
  domain cannot hold the product of two whole-range `WaterVolume` values — `(2^64 - 1)²` needs 128
  unsigned bits and `i128` offers 127 — so "accumulate in `i128`" is a necessary and not a
  sufficient condition. Every multiplication in the solver goes through `checked_water_mul`, and the
  bound is pinned by test rather than assumed.

- **2026-07-30 — Stage 3 split: domains emits the logical DAG, the runtime commits it.**
  `HydrologyEvolutionModel::propose` returns `HydrologyEventPlan` values — a proposal key, a domain
  event-kind name, causes that may be `Local`, and effects naming a carrier, a property, and two
  already-computed fingerprints. It does **not** build `CausalEventDagProposal`: object kinds,
  property IDs, event-kind numbers, and the dense object registry are runtime schema, and a domain
  that knew them would make the runtime's causal numbering a domain concern. Fingerprints *are*
  computed in domains, because what is being hashed — a water volume, an ordered list of forcing
  allocations — is domain data; `blake3` is now a `causafera-domains` dependency for that. The
  §8 aggregation tree and the conservation event land in Stage 6's `hydrology_events.rs`, where the
  synthetic-node counter and committed effect targets it hashes actually live; Stage 3 emits the
  ordered terminal-leaf list the tree is built over. This mirrors the thermal seam, where domains
  produces state and receipts and `thermal_events.rs` produces proposals.
- **2026-07-30 — Stage 3 addition: the forcing settlement accounts for the water it delivered.**
  §8 specifies the settlement event's effect as the cell's `HYDROLOGY_FORCING_PROPERTY`. Implemented
  exactly that way, a cell that was only rained on — no infiltration, no ET — ends the tick holding
  more water while its `surface_last_change` still points at a previous tick, because substage 1 is
  where accepted precipitation and external inflow actually land and no other substage touches that
  cell. The transfer receipt records the change, but receipts are evicted after eight batches and
  the bucket anchor is what has to survive (INV-014). The settlement event therefore carries a
  second effect on the surface property whenever accepted source water is nonzero, and becomes that
  bucket's current reference and terminal anchor. The forcing-property effect the plan names is
  unchanged and still always present; this is an addition, not a substitution, and it stays inside
  the eight-effect cap at two.
- **2026-07-30 — Stage 3: forcing application and forcing settlement are separate processes.**
  §8 describes two different events — a record becoming spent, and a cell folding every record that
  reached it. Giving both the same opaque process ID made them indistinguishable in a receipt and in
  a proposal-key lookup, so `process::FORCING_APPLICATION` was allocated alongside
  `process::FORCING_SETTLEMENT`. For the same reason the evapotranspiration *event* uses a neutral
  `process::EVAPOTRANSPIRATION` rather than borrowing the surface receipt's identity: one event
  settles whichever of the two buckets it drew from, and naming it after either would misreport the
  half that did not move. The two ET *receipt* kinds stay distinct as the plan specifies.
- **2026-07-30 — Stage 3 measurement: water arithmetic cannot overflow the accumulation domain
  under the plan's bounds.** With at most 131 072 cells of three whole-range `u64` buckets, the
  world total is bounded by roughly `2^82.6` and every product the solver forms —
  `weight * total`, `value * numerator` — is bounded by `2^128` in `u128` or `2^96` in `i128`. So no
  reachable input overflows, and the plan's "negative control for overflow" has no reachable case at
  this stage. The checked arithmetic is retained as the *proof* of that property rather than as a
  mechanism that fires: `crates/causafera-types/src/physics.rs` pins the primitive behaviour
  directly, and the reachable boundary — capacity refusing water and recording it as unaccepted — is
  covered by `a_forcing_total_that_would_overflow_the_carrier_is_refused`. Recorded rather than
  papered over with an unreachable test.

- **2026-07-30 — Stage 4: terrain is borrowed, not copied into hydrology state.** §6 needs
  `terrain_elevation_mm` every tick and §3's `HydraulicSubstrateCell` does not carry it, so
  `HydrologyEvolutionRequest` gained `terrain: &BTreeMap<ChartChunkCoord, TerrainChunk>` — the same
  type and keying `crates/causafera-runtime/src/carrier.rs` already passes around. A per-cell
  elevation copy inside hydrology state was rejected: terrain elevation is an existing authoritative
  carrier the plan lists as reused, and a second persisted copy could drift from it while both
  claimed to be the ground water flows over. Every resident chunk must have a terrain entry filed
  under its own address, or the tick is refused rather than defaulting an elevation to zero and
  inventing a flat world. `SURFACE_CELL_COUNT == TERRAIN_CELLS_PER_CHUNK` is a compile-time
  assertion, because the two lattices sharing an ordinal is what makes the lookup meaningful.
- **2026-07-30 — Stage 4: an exterior face with no boundary record refuses the tick.** §3 says
  missing resident neighbours are explicit boundary records and V13 says they never silently export
  or block, so the solver rejects rather than defaulting to no-flux. The consequence is that every
  fixture — and, at Stage 6, bootstrap — must supply a record for each perimeter face; the shared
  test support gained `closed_perimeter` and `boundary_map` for that, and `Scenario::new` now
  defaults to a closed perimeter instead of an empty map. Stage 3's fixtures were relying on an
  implicit wall; they now say so. `tests/support/mod.rs` is therefore on this stage's allowlist even
  though the plan's Stage 4 file list names only the three new test binaries.
- **2026-07-30 — Stage 4: the donor tie-break needs one discriminator the plan does not name.**
  §6 breaks equal remainders by "ascending canonical edge key". Within a donor's interior faces that
  is literal, and exterior faces sort after all interior ones because `HydrologyCarrierKey`'s variants
  do. It leaves one case open: a cell's groundwater lateral outflow and its baseflow can cross the
  *same* face, so two competing demands would share a key. The reduction order is therefore
  `(donor, face, process_kind)`, which reduces to the plan's rule whenever the faces differ. Ordering
  uses the key types' value order (`HydrologyEdgeKey`'s canonicity is defined by `a < b`), while the
  terminal-leaf sort keeps the plan's explicit `carrier_key_bytes` byte order; the two differ only
  for negative chunk coordinates and each is used where its own contract names it.
- **2026-07-30 — Stage 4 addition: a cell settles when it took part in a transfer, not when its
  total moved.** Emitting the routing settlement only for cells whose bucket changed leaves a
  pass-through cell — ten units in, ten out — with no event, and both of its transfers with no
  committed event to name. That is the same provenance hole as the Stage 3 forcing settlement. The
  participation set is now `{donor} ∪ {cell receiver}` over accepted transfers, unioned with
  "value changed"; the effect's before and after are equal for a pass-through, which is an honest
  "settled at this value after these transfers" rather than a claim of change. A cell with no
  accepted transfer still emits nothing, so the plan's alias rule is unchanged.
- **2026-07-30 — Stage 4: three process identities added, for the reason `EVAPOTRANSPIRATION` was.**
  `process::SURFACE_ROUTING`, `GROUNDWATER_ROUTING`, and `CONVEYANCE_SETTLEMENT` name the per-cell
  and per-receiver *settlement events*, whose net change may come from lateral outflow, lateral
  inflow, conveyance inflow, and boundary export at once. Borrowing any one participating process's
  identity would misreport the others in a receipt and in a proposal-key lookup.
- **2026-07-30 — Stage 4: "competing source-edge local events" means their post-inflow events.**
  §8's release-allocation fan-in reads ambiguously: if a competitor's *release* event were the cause,
  two edges contending for one downstream capacity would cite each other and `commit_dag_batch` would
  reject the batch as cyclic. The causes are therefore each competitor's pre-release reference — its
  post-inflow local event or its pre-tick trace — which is both acyclic and the actual dependency:
  the allocation depended on what every competitor was holding before any of them released.
- **2026-07-30 — Stage 4: `GroundwaterWithoutSpecificYield` has no reachable case.** §6 calls a zero
  specific yield invalid when groundwater storage is enabled. Stage 2's substrate constructor already
  refuses a zero yield over real capacity, and its field constructor refuses storage above capacity,
  so stored groundwater always arrives with a yield. The solver keeps the guard as the divisor's
  stated precondition, and `stored_groundwater_can_never_reach_the_solver_without_a_specific_yield`
  pins the closure at the two constructors instead of asserting an unreachable error — the same
  treatment the Stage 3 overflow control got.
- **2026-07-30 — Stage 4: the solver validates the fan-in caps and the resident set itself.**
  `commit_dag_batch` enforces the cause and effect caps and would roll the batch back atomically, but
  returning a proposal that cannot possibly commit as if it were valid is a worse failure mode than
  refusing it, so `propose` checks both against the request's limits. It also requires the request's
  resident chunk set to equal the field set's, which is what stops `HydrologyActiveRegion` from being
  decoration: routing over a chunk the request does not consider resident would exchange water across
  an edge of the world it was told did not exist.
- **2026-07-30 — Stage 4 finding: fixtures that share one pre-tick trace cannot test a fan-in
  bound.** Causes are deduplicated, so a five-way fan-in over cells all anchored to one bootstrap
  trace collapses to a single cause and the cap assertion passes vacuously. Both fan-in tests build
  distinct per-bucket anchors and assert the widest event actually reaches five causes before
  asserting it stays under sixteen. Found by the cap negative control failing to fire, not by review
  of the passing test.

- **2026-07-30 — Stage 5: level zero is the fine path, not a one-cell coarse path.**
  `block_edge = 2^min(L, 4)` makes a level-zero block one cell, and running the coarse machinery
  over one-member groups would produce the same *state* through a completely different causal DAG —
  a coarse-process event, an input leaf, and an input node per cell per process. Level zero
  therefore dispatches to the fine substages, and a disabled policy is proven byte-identical to the
  fine path rather than an imitation of it. Lateral routing needs no such special case: a
  level-zero cell's block is itself, so every face touching it is a block boundary and is evaluated
  finely, which is exactly Stage 4's behaviour.
- **2026-07-30 — Stage 5: a cell's exterior-face signature participates in its constitutive
  identity.** §9's tuple is `(metric, substrate, boundary-kind)`, and boundary kind is per face, so
  the key carries all four faces: interior, or exterior with that face's
  `constitutive_kind`. The consequence is that a perimeter cell never groups with an interior one
  even when every boundary is closed — a chunk's edge cells fragment into their own groups. That
  reduces work reduction and is the conservative direction, so it is kept as written. Discovered by
  three tests failing on a 2x2 fixture placed at the chunk corner; the fixtures moved inside.
- **2026-07-30 — Stage 5 correction: the coarse total is clamped to the ceilings of members the
  process actually addressed.** §9 says `T = min(raw_group_candidate, sum(member_ceilings))` and
  also that "the reducer never receives an unallocatable ordinary candidate". Those two cannot both
  hold: weight zero means the process never addressed a member — a cell no record rained on, or none
  asked ET of — and the reducer refuses to hand it water, so counting its room produces a total the
  reducer must then reject. A single-target record over a coarse group would have failed the whole
  tick. `clamp_to_allocatable` therefore sums only positive-weight ceilings, which makes
  `UnallocatableTotal` unreachable for ordinary candidates exactly as §9 promises and keeps the
  guard for the genuine internal-error case. Capacity is still shared — but only among members the
  process addressed, which is what makes coarse forcing an approximation of distribution rather than
  a way to rain on cells nobody aimed at.
- **2026-07-30 — Stage 5: the coarse-process identity carries the forcing record.** §8 and §9 order
  synthetic ID allocation by `(tick, block_key, constitutive_group_key, process_kind)`, which cannot
  separate two source or ET invocations of one group that differ only by which record they came
  from — and §9 requires each `(scheduled_tick, forcing_id, kind)` to be processed separately. Two
  invocations would collide on the key and the ID assignment would be ambiguous.
  `HydrologyCoarseProcess::identity` therefore appends `(scheduled_tick, forcing_id)`, zero for the
  once-per-group processes. Stage 6 must include the same two fields in the coarse-process
  fingerprint, since the plan's fingerprint input does not currently distinguish them either.
- **2026-07-30 — Stage 5: the coarse-input trees are Stage 6, for the same reason the terminal tree
  is.** §9's input leaves, input nodes, and process events all draw object IDs from the runtime's
  persisted `next_hydrology_batch_node_id` counter, and the coarse-process event's proposal key
  contains that ID. The domain therefore emits `HydrologyCoarseProcess` — members in canonical cell
  order with exact weights, ceilings, grants, and current references, plus the raw candidate, summed
  ceilings, and accepted total — which is every input the plan's leaf and process fingerprints hash
  except the IDs. A fine allocation event names its process by index in `coarse_process`, and the
  runtime appends the resolved local cause; the domain reserves that one extra cause in its own cap
  check so an event cannot pass here and fail at commit.
- **2026-07-30 — Stage 5 fix to Stage 3: the forcing settlement fingerprint is computed after
  substage 4.** §4 and §8 have the settlement fingerprint cover the ordered per-record allocations,
  and those allocations include `accepted_et`, which substage 4 fills in. Stage 3 computed the
  fingerprint in substage 1 and then mutated the allocations, so the persisted value was one no
  recomputation from the same allocations could reproduce — which is precisely what Stage 6's import
  validation does. The settlement *event* is now pushed after substage 4 while its proposal key stays
  deterministic, so substages 2 through 4 still cite it as a local cause and its surface effect still
  carries the substage-1 delta. `a_settlement_fingerprint_is_recomputable_from_the_allocations_it_persists`
  pins it. `tests/hydrology_vertical_cycle.rs` is on this stage's allowlist for that regression.
- **2026-07-30 — Stage 5: coarse percolation can never exceed the fine total, by design.** §9 makes a
  member's percolation ceiling "that result bounded by remaining groundwater capacity" — its own raw
  fraction result — so the summed ceilings are at most the sum of the fine results while the aggregate
  candidate `floor(sum/den)` is at least that. The clamp therefore always picks the fine total. This
  is the plan's quantisation guard working as specified, not a missed opportunity: aggregate rounding
  is not permitted to create water. Recorded because the first version of the test asserted the
  larger aggregate and was wrong.

- **2026-07-30 — Stage 6 is executed in five waves.** The stage spans thirty files across the Rust
  runtime, the lab, the observer API and wire, the Tauri session, the TypeScript protocol, and the
  audit tooling, and its parts have genuinely different failure modes. The waves are: **A** the
  configuration contract, appended registration, and recipe major 7; **B** runtime hydrology state,
  the Physics system, the DAG mapping, the §8 terminal tree, the §9 coarse-input trees, the persisted
  node counter, and the whole-tick staging transaction; **C** the seventh production bootstrap stage
  and its object registry; **D** section `0x000F`, digest schema 8, and import validation; **E**
  the additive observer field 48 across all four language surfaces plus the audits. Each wave has its
  own green gate and checkpoint. The stage's contract is unchanged — this is execution order, not a
  rescope.
- **2026-07-30 — Stage 6 wave A: V22's "byte-identical" applies to legacy *subsystem* sections, not
  to the recipe.** Adding `HydrologyConfig` to `RuntimeConfig` changes the runtime recipe section by
  construction, and the recipe is what a session was configured to be, so a new domain belongs in it.
  Measured: the physical, history, and experiment digests are byte-identical after the change, and
  only the full-envelope digest moves. `pre_hydrology_section_payloads_are_pinned` therefore asserts
  every section *except* the recipe against its captured row, and asserts the recipe's change is
  exactly the declared one — major 6 to 7, and a payload longer by exactly the 24 bytes a disabled
  hydrology block encodes. `pre_hydrology_digests_are_pinned` keeps the three authoritative digests
  pinned unchanged and asserts the envelope digest moved. Re-pinning the whole table would have
  hidden precisely what V22 exists to expose.
- **2026-07-30 — Stage 6 wave A: a disabled hydrology configuration encodes itself explicitly.** The
  recipe writes the limits schema, the resolution policy's schema and flags, and both empty
  collection counts even when the domain is off. Encoding nothing would make "hydrology was disabled
  in this world" and "this snapshot predates hydrology" the same bytes, and only one of those is a
  statement about the world the snapshot describes. The 24-byte cost is asserted by name so future
  state cannot accumulate there unnoticed.

- **2026-07-30 — Stage 6 wave B correction to Stage 4: a pass-through cell anchors nothing.**
  `CausalEffect::new` refuses an effect whose before and after agree, and it is right to: an effect
  asserting no change is not a state change. Stage 4 had made a cell that passed water through — ten
  in, ten out — emit a routing settlement so its transfers would have an event to name; that event's
  effect would have been exactly such a claim. The rule is now what §8's alias rule says: no bucket
  change, no bucket-change event. Neither transfer is orphaned — the inbound one names the donor's
  event and the outbound one names the receiver's — and the cell's `last_change` correctly still
  points at whenever its stored volume last differed. Found by the core contract, not by review.
- **2026-07-30 — Stage 6 wave B: the coarse-input leaf fingerprint hashes reference *descriptors*,
  not trace IDs.** §9 step 1 hashes "the ordered current-reference trace IDs". A local reference has
  no trace ID until the batch commits, and the leaf fingerprint is an input to that batch — so
  hashing trace IDs is not computable where it must be computed. Each reference is hashed as a kind
  tag plus either the existing trace ID or the sibling's canonical proposal key, which is
  deterministic before commit and strictly more specific than an ID that depends on how much history
  the store already holds. The referenced event's own effect payload is also omitted: it is
  unreachable for a local sibling pre-commit, and the weight and ceiling the leaf already hashes are
  the quantities that state produced.
- **2026-07-30 — Stage 6 wave B: synthetic node IDs start at one.** Object ID zero is reserved for
  the batch-sequence object the conservation event settles. A tree node and the conservation event
  sharing an object ID would make two different claims about one target.
- **2026-07-30 — Stage 6 wave B: the conservation effect fingerprints the whole ledger.** §8 says the
  conservation event transitions the batch-sequence property but does not say what the fingerprint
  covers. It covers every ledger term, so the effect is a statement about the tick's water budget
  rather than about a counter that happened to advance.
- **2026-07-30 — Stage 6 wave B: the stream-keying ID and the system schema ID are different
  numbers.** `HYDROLOGY_SYSTEM_ID = 13` is the schema identity the persisted manifest declares;
  the scheduler's assigned ID — what `StreamKey` uses — is the registration order, which is 11.
  Risk R7 is about the second. The legacy fixture now asserts both: the first eleven registrations
  keep their phase and their stream ID, exactly one system was added, it runs in `Phase::Physics` at
  stream ID 11, and it declares schema 13.
- **2026-07-30 — Stage 6 wave B: `thermal_determinism`'s registration count became a lower bound.**
  Its contract is that legacy registrations are untouched and thermal occupies positions nine and
  ten, not that nothing may ever follow thermal. The consecutive-from-zero assertion is what would
  still catch an insertion, and it is kept.
- **2026-07-30 — Stage 6 wave B: overrides are validated against what they resolve to.** §4 requires
  every initial storage to fit its *resolved* capacity. An override that lowers only a capacity still
  inherits the default storage, and that pair has to be consistent. The check applies cell-then-chart-
  then-default precedence — the same order bootstrap applies — so what configuration validation
  rejects is what bootstrap would have failed to build, named rather than surfacing as a bare
  capacity error from whichever cell tripped over it first.

## Progress

- **2026-07-29 — Accepted.** Revision 19 is the authoritative hydrology implementation plan.
  Implementation is authorized but has not started; no implementation checkpoint exists.
- **2026-07-29 — Planning discovery complete.** Verified current hydrology/climate stubs, crate
  boundaries, scheduler behavior, persistence versions, maturity/RFC status, and thermal/terrain
  precedents. External process/routing recommendations were checked against USGS, NOAA, EPA, and
  USACE primary sources.
- **2026-07-29 — Adversarial planning review complete.** Ownership, forcing persistence, grid metric,
  resolution, legacy RNG, schema versioning, and generated protocol validation were challenged and
  incorporated before this draft.
- **2026-07-29 — Independent gap analysis folded into revision 1.** Corrected legacy-version expectations;
  completed hydraulic schemas, 2D lattice and forcing allocation; added two-sided capacity reduction,
  producer-neutral origins, disabled runtime config, same-batch provenance, hard bounds/retention,
  exact observer fields, runtime resolution seam, and exact stage allowlists.
- **2026-07-29 — Revision 1 drafted.** No implementation stage has started. No checkpoint commit
  exists. The worktree was clean before plan authoring.
- **2026-07-29 — Revision 2 drafted.** The canonical plan became decision-complete for high-accuracy
  review; implementation remained unstarted.
- **2026-07-29 — Second independent gap pass folded into revision 3.** Bounded forcing fan-in,
  exact baseflow and conveyance destinations, authoritative fine block-boundary transfers, complete
  bootstrap/resolution schemas, and noncolliding observer fields/messages are now explicit. No
  implementation stage has started and no checkpoint commit exists.
- **2026-07-29 — Approval review corrections folded into revision 4.** Clarified concurrent boundary
  donor accounting, convergent conveyance reduction, the exact nonrecursive override/disabled
  schemas, forcing/conveyance observer identities, and the missing protobuf mirror for existing
  bootstrap fields 28–35. Implementation remains unstarted and no checkpoint commit exists.
- **2026-07-29 — Final lifecycle correction folded into revision 5.** Configuration now carries
  trace-free forcing specs with an explicit atomic bootstrap conversion, and the observer's latest
  forcing identity includes its scheduled tick. Implementation remains unstarted and no checkpoint
  commit exists.
- **2026-07-29 — Bounded bootstrap ancestry folded into revision 6.** All bootstrap forcing specs
  share one canonical schedule-origin trace whose result digest covers the complete ordered
  schedule; the terminal stage receipt therefore stays below the causal fan-in cap. Implementation
  remains unstarted and no checkpoint commit exists.
- **2026-07-29 — Forcing-origin limit evidence corrected in revision 7.** Production integration
  verifies the single bootstrap schedule origin. Eight-/nine-origin composition is explicitly a
  lower-level forward-compatibility bound test with a stub resolver, not a claim that another
  production producer exists. Implementation remains unstarted and no checkpoint commit exists.
- **2026-07-29 — Isolated code-aware review folded into revision 8.** Corrected direct
  `ChartChunkCoord` resolution integration, endpoint conductance, coarse allocation, dense causal
  identities and bootstrap fingerprints, compositional persistence bounds, lossless
  observer/Explanation encodings, affected-file allowlists, and constructible benchmark shapes.
  Implementation remains unstarted and no checkpoint commit exists.
- **2026-07-29 — Bootstrap causal addressing correction folded into revision 9.** The actual
  pre-existing sixth-stage completion trace and fixed target identities for all seven aggregate
  effects are explicit. Implementation remains unstarted and no checkpoint commit exists.
- **2026-07-29 — Final causal/numeric closure folded into revision 10.** Added deterministic
  groundwater conductance, boundary bootstrap fingerprinting, complete cross-bucket trace parents,
  and `u128` latest-source encoding. Implementation remains unstarted and no checkpoint commit
  exists.
- **2026-07-29 — Edge ancestry correction folded into revision 11.** Edge events now include
  outlet-surface and applicable downstream-edge capacity ancestry, and the Decision Log matches the
  six-origin per-cell bound. Implementation remains unstarted and no checkpoint commit exists.
- **2026-07-29 — Atomic DAG/envelope closure folded into revision 12.** Added atomic local-cause DAG
  commit, bounded terminal aggregation ancestry, exact process-total caps and per-record attribution,
  complete snapshot sizing and override bounds, response-session coverage, and repository-accurate
  protobuf schema auditing. Implementation remains unstarted and no checkpoint commit exists.
- **2026-07-29 — Canonical DAG shape folded into revision 13.** Vertical local dependencies and
  fallback references are fully enumerated; the terminal 16-ary tree now has exact leaf/node
  encodings, consecutive grouping, bottom-up level/group IDs, and zero/singleton behavior. Implementation
  remains unstarted and no checkpoint commit exists.
- **2026-07-29 — Forcing-settlement anchor folded into revision 14.** Each targeted cell persists
  the exact forcing-input fingerprint and trace on its allocated forcing property, including
  zero-accepted source/ET cases. Implementation remains unstarted and no checkpoint commit exists.
- **2026-07-29 — Tree key encoding folded into revision 15.** Resolution carriers, bucket tags,
  proposal-key bytes/local ordinals, and bottom-up node ID order are versioned and exact.
  Implementation remains unstarted and no checkpoint commit exists.
- **2026-07-29 — Aggregate-node key folded into revision 16.** Batch aggregate nodes have an exact
  nonphysical carrier-key variant for local DAG proposal keys and trace inspection. Implementation
  remains unstarted and no checkpoint commit exists.
- **2026-07-29 — Coarse-input and whole-tick closure folded into revision 17.** Coarse dynamic inputs
  have canonical bounded ancestry trees, and a hydrology-enabled whole-tick staging transaction
  prevents later phases from crossing the complete snapshot cap. Stage 6 now updates and tests every
  seventh-bootstrap observer consumer atomically. Implementation remains unstarted and no checkpoint
  commit exists.
- **2026-07-29 — Exact DAG and frozen-decoder closure folded into revision 18.** Canonical
  topological tie-breaking, shared synthetic-node allocation, exact coarse/terminal targets and
  fingerprints, duplicate terminal memberships, and the V1-safe separate stage-seven receipt are
  explicit. Implementation remains unstarted and no checkpoint commit exists.
- **2026-07-29 — Legacy stage-count and global terminal-order closure folded into revision 19.**
  Fields 28–35 preserve the complete six-stage legacy projection, and the sole terminal tree is
  allocated only after every coarse group. Multi-group verification pins the shared counter order.
  Implementation remains unstarted and no checkpoint commit exists.

- **2026-07-29 — Stage 1 complete and checkpointed.** Branch `feat/conserved-hydrology`, from
  `29fdf4d` on a clean worktree. Checkpoint commit `1484ebe`; this hash is recorded in the
  immediately following documentation-only commit, since a commit cannot name itself.

  Changed:

  - `docs/rfc/RFC-HYDRO-001.md` — Proposed → **Accepted**, expanded from a four-line stub to the
    accepted architecture: units and the checked-`i128` numeric contract, the chart-keyed grid
    metric, the terrain-aligned 2D lattice and four-face topology, state and conveyance and boundary
    schemas, the frozen-substage process order, conservation and provenance, ownership by crate, and
    the downstream surfaces. Both previously unresolved questions are resolved in the RFC's own
    words: resolution coupling as retained-fine-state-plus-back-allocation, and seasonal variation as
    explicit tick-indexed forcing with producer-neutral committed origins rather than climate
    generation. Non-goals and the primitive/emergent split are stated.
  - `docs/development/todo-backlog.md` — `TODO-HYDRO-001: Conserved Multi-Resolution Hydrology`
    created, **In Progress**, placed in the geography cluster after `TODO-GEO-006`.
  - `PLANS.md` — Active Plans entry updated from "Implementation has not started" to the current
    branch and stage.
  - `crates/causafera-runtime/src/runtime.rs` — `Runtime` records the scheduler's own assigned
    system IDs and exposes them read-only as `scheduler_registrations()`. See the Stage 1 deviation
    entry in the Decision log.
  - `crates/causafera-runtime/Cargo.toml` — `blake3` added as a **dev**-dependency only, so the
    fixture can digest section payloads (`causafera-persistence` keeps `compute_integrity` private).
  - `crates/causafera-runtime/tests/hydrology_legacy_compatibility.rs` — new, seven tests.
  - `tools/audit/fixtures/observer-protocol-v1-pre-hydrology.mjs` — new, the frozen V1 decoder.
  - `tools/audit/test-observer-hydrology-legacy-decoder.mjs` — new, six tests.

  Captured pre-hydrology evidence, all measured from the engine at seed `20_260_729`, three
  `Line` chunks at extent 3, four actors on two sensors, bootstrap population 64, six ticks:

  - live scheduler registration IDs `0..=10` and their phases, cross-checked against the persisted
    recipe manifest;
  - the eleven `(world_seed, tick 0, phase, system_id)` stream samples those IDs key, with a
    sensitivity test proving a one-step ID shift changes every one of them;
  - all thirteen envelope sections — IDs 1–10, 12, 13, 14 — pinned by `(id, major, minor, flags,
    decoded size limit, payload length, BLAKE3)`; total payload 196 188 bytes, none empty;
  - `physical = 2a602728…eef822f3`, `history = fe5641b7…3a9e0f0e`,
    `experiment = dbdcdb84…0f461320`, all at digest schema 7, and complete envelope
    `218e6858…8a5012f3`; and
  - export/import/export byte identity, plus `2N`-tick equivalence against `N` ticks, save, resume,
    `N` ticks.

  Non-vacuity is asserted, not assumed: a tick moves both the physical and the history digest, the
  section table has thirteen non-empty rows, and the pinned stream samples are all distinct.

  Commands run and their actual results:

  - `cargo test -p causafera-runtime --test hydrology_legacy_compatibility -- --nocapture` — **7
    passed**, 0 failed.
  - `node tools/audit/test-observer-hydrology-legacy-decoder.mjs` — **6 passed**, 0 failed.
  - `node tools/audit/check-entry-points.mjs` — pass (`audit_entry_points=27 audit_tests=18`).
  - `git diff --check` — clean.
  - `cargo fmt --all -- --check` — clean.
  - `cargo clippy -p causafera-runtime --all-targets --all-features -- -D warnings` — clean.
  - `cargo test -p causafera-runtime --all-features` — **all suites passed** (111 unit + 19 suites,
    2 pre-existing ignored), run to confirm the `Runtime::new` registration refactor changed no
    existing behaviour.

  Not done in this stage, by design: the new audit test is deliberately **not** yet registered in
  `tools/audit/run-source-tests.mjs` or `check-entry-points.mjs`, because both files are on Stage 7's
  allowlist and Stage 1's gate invokes the test directly. The frozen oracle currently proves only
  faithfulness of the freeze; the hydrology-fields-ignored claim it exists for arrives in Stage 7.

- **2026-07-29 — Stage 2 complete and checkpointed.** Checkpoint commit `a3d67b7`. Fixed-point
  primitives and geography-owned state. No behaviour is scheduled or committed yet; this is the
  vocabulary the solver is written against.

  Changed:

  - `crates/causafera-types/src/physics.rs` — `WaterVolume` (non-negative `u64` mm³), `WaterDepthMm`,
    the `WaterAccumulator` `i128` domain, and `checked_water_mul` /
    `checked_water_div_floor` / `checked_water_rem_floor`. Flooring division and its paired
    Euclidean remainder replace Rust's truncating `/` and `%`, which disagree with the plan's
    `floor` specification for every negative numerator — and heads, deltas, and residuals are all
    signed. Nothing saturates.
  - `crates/causafera-geography/Cargo.toml` — `thiserror` added, matching the rest of the workspace.
  - `crates/causafera-geography/src/hydrology.rs` — **deleted** (the three-line `f32` placeholder).
  - `crates/causafera-geography/src/hydrology/mod.rs` — the twenty hard allocation bounds,
    `SURFACE_CELL_COUNT`, and one `HydrologyStateError` covering all four submodules.
  - `crates/causafera-geography/src/hydrology/metric.rs` — `HydrologyGridMetric` (schema 1,
    `NonZeroU64` area/edge/timestep) and the chart-keyed `HydrologyGridMetrics` registry. Depth
    conversion returns its sub-millimetre remainder rather than dropping it.
  - `crates/causafera-geography/src/hydrology/substrate.rs` — `HydraulicFraction`,
    `HydraulicSubstrateCell` with all eleven plan fields, and the canonical
    `HydraulicSubstrateKey`.
  - `crates/causafera-geography/src/hydrology/forcing.rs` — `HydrologyForcingMember`,
    `HydrologyForcingRecord`, `HydrologyForcingSchedule`, and
    `BOOTSTRAP_HYDROLOGY_FORCING_POLICY_V1`, including both origin fan-in bounds and the
    checked-subtraction bootstrap horizon.
  - `crates/causafera-geography/src/hydrology/state.rs` — the lattice (`HydrologyCellKey`,
    `FaceDirection`, `HydrologyEdgeKey`, `HydrologyExteriorFaceKey`), boundaries, cell state, the
    field and field set, conveyance, residency, resolution state, and the six-variant
    `HydrologyCarrierKey` encoding.
  - `crates/causafera-geography/src/lib.rs` — **no change needed**: `mod hydrology` resolves to the
    new directory unchanged. It is on the stage allowlist because the module moved, not because its
    text did.

  Evidence:

  - 78 new tests (15 in `causafera-types`, 63 in `causafera-geography`), all passing.
  - Every hard allocation bound in the stage's scope is tested at `limit` and at `limit + 1`:
    charts (64), chunks (128), edges (262 144), boundary records (524 288), forcing records (8 192),
    targets per record (4 096), total forcing members (262 144), origins per tick (8), and origins
    per cell per tick (6). `MAX_HYDROLOGY_CHUNKS * SURFACE_CELL_COUNT == MAX_HYDROLOGY_CELLS` is
    asserted, so the two independently stated constants cannot silently disagree.
  - Seam behaviour is proven, not assumed: every one of the 32 cells along a chunk seam resolves to
    its neighbour across the boundary, the relation is symmetric, and the orthogonal coordinate is
    preserved in both axes. A conveyance edge on a seam face constructs exactly like an interior one.
  - Input-order independence is proven for the metric registry, the field set, and the conveyance
    graph.
  - Every carrier-key variant round-trips at its exact declared length (23, 45, 24, 17, 21, 9), and
    decoding rejects short input, trailing bytes, unknown variants, unknown face directions,
    out-of-range ordinals, and reversed or degenerate edge endpoints.
  - Depth quantisation keeps its remainder: `depth * area + remainder == volume` exactly, including
    the case where a whole movement is below one millimetre of depth.

  Commands run and their actual results:

  - `cargo test -p causafera-types physics -- --nocapture` — **15 passed**, 0 failed.
  - `cargo test -p causafera-geography hydrology -- --nocapture` — **63 passed**, 0 failed.
  - `cargo clippy -p causafera-types -p causafera-geography --all-targets --all-features -D warnings`
    — clean.
  - `cargo fmt --all -- --check` — clean.
  - `git diff --check` — clean.
  - `cargo test --workspace --all-features` — all 71 suites passed, run to confirm deleting the
    `HydrologyCell` placeholder broke no consumer. It had none, as the plan's Context recorded.

- **2026-07-30 — Stage 3 complete and checkpointed.** Checkpoint commit `f167fde`. The local
  vertical cycle and exact receipts. Water now moves, and the ledger closes on it.

  Changed:

  - `crates/causafera-core/src/provenance.rs` — `CausalEventProposalKey` with the plan's exact
    version-1 byte encoding, `CausalEventDagCause`, `CausalEventDagProposal`,
    `CausalDagBatchLimits`, and `CausalTraceStore::commit_dag_batch`. Local causes are resolved
    only after unique keys, resolvable external traces, resolvable local references, the substage
    ordering rule, per-event caps, and store capacity have all passed, and the order is Kahn's
    algorithm with the ready set ordered by complete key bytes. Nothing touches `self` until every
    check succeeds. The per-event caps are caller-supplied rather than fixed here, so one domain's
    event shapes stay one domain's contract.
  - `crates/causafera-core/src/lib.rs` — **no change needed**; `pub use provenance::*` already
    exports the additions.
  - `crates/causafera-domains/Cargo.toml` — `blake3` added; see the Decision log entry on the split.
  - `crates/causafera-domains/src/lib.rs` — the `hydrology` module registered.
  - `crates/causafera-domains/src/hydrology/parameters.rs` — substage ordinals, twenty-one opaque
    process identities, and `HydrologyEvolutionLimits`.
  - `crates/causafera-domains/src/hydrology/records.rs` — `HydrologyBucket` with the plan's
    aggregation tag bytes, `HydrologyTransferReceipt`, the cell/edge change records, the
    forcing-settlement records, and `HydrologyError`.
  - `crates/causafera-domains/src/hydrology/receipts.rs` — `HydrologyConservationReceipt`,
    `HydrologyReceiptTotals` as an independent second derivation, and the paired-transfer and
    boundary-transfer validators.
  - `crates/causafera-domains/src/hydrology/proposal.rs` — the request, the logical event plan, the
    proposal, and the four canonical fingerprint encodings.
  - `crates/causafera-domains/src/hydrology/evolution.rs` — `allocate_largest_remainder` and
    substages 1–4 and 9.
  - `crates/causafera-domains/tests/support/mod.rs` — shared fixtures (**not** on the stage
    allowlist; added because two integration test files need one builder set and duplicating it
    would let the two drift).
  - `crates/causafera-domains/tests/hydrology_vertical_cycle.rs` — 23 tests.
  - `crates/causafera-domains/tests/hydrology_conservation.rs` — 13 tests.

  Evidence:

  - **`commit_dag_batch`** — 11 new tests. A three-substage chain resolves each local cause to a
    trace committed in the same batch; all six permutations of that batch produce a byte-identical
    store; independent proposals commit in lexicographic key order across every field of the
    encoding; a backwards substage edge, a two-node cycle within one substage, and a
    self-reference are each rejected; six distinct rejection classes each leave a store *with real
    history in it* byte-identical, and a valid batch afterwards still commits; legacy `commit_batch`
    issues exactly the identifiers it always did before, between, and after DAG commits; and a
    DAG-committed store re-imports from its own snapshot unchanged.
  - **Conservation** — every committed tick is checked five ways, none of them against the solver's
    own running totals: residual exactly zero; declared pre- and post-state totals equal to the two
    field sets' own sums; source and sink terms refolded from the per-transfer receipts and compared
    to the aggregate literals; every internal transfer proven to move the same amount out of one
    bucket as into the other; and `before + sources == after + sinks` written out longhand. A
    closed basin over **100 ticks** holds 11 987 mm³ exactly at every tick, with 100 of 100 ticks
    doing nonzero internal work.
  - **Process equations** asserted against the plan's formulas, not against output: infiltration
    bounded separately by availability, by its per-tick limit, and by remaining soil room (each on
    its own cell); percolation flooring `999/4` to 249 with the quarter-unit staying in soil;
    saturated soil infiltrating strictly less and retaining strictly more surface water than dry
    soil under identical forcing; ET drawing surface then soil and never groundwater; unmet demand
    of 470 against 30 of available water recorded rather than removed.
  - **Allocation** sums to the record total across five weight shapes and five totals; ties break by
    ascending key; a positive total with no positive-weight member is refused.
  - **Ancestry** — infiltration cites the forcing settlement that delivered the water it consumed
    rather than the surface bucket's pre-tick trace; percolation cites the infiltration that filled
    the soil; each bucket's terminal anchor is the last event that actually moved it, verified to be
    three different events in one tick; every event's causes are strictly ordered and deduplicated.
  - **Bounds and refusals** — a non-resident forcing target rolls back the whole proposal; a record
    reaching the wrong tick is refused; a batch past the transfer limit is refused, and the same
    world inside its limit is accepted; a chunk whose chart has no registered metric cannot be
    evolved.
  - **Order independence** — two chunks and two records built in both orders produce an identical
    proposal, with the fixture asserted to have actually moved water.

  Commands run and their actual results:

  - `cargo test -p causafera-core provenance -- --nocapture` — **19 passed**, 0 failed.
  - `cargo test -p causafera-domains --test hydrology_vertical_cycle -- --nocapture` — **23 passed**.
  - `cargo test -p causafera-domains --test hydrology_conservation -- --nocapture` — **13 passed**.
  - `cargo clippy -p causafera-core -p causafera-domains --all-targets --all-features -D warnings`
    — clean.
  - `cargo fmt --all -- --check` — clean.
  - `git diff --check` — clean.
  - `cargo test --workspace --all-features` — all 73 suites passed.

  Not done in this stage, by design: substages 5–8 (routing, baseflow, conveyance, boundary export)
  are Stage 4; the §8 aggregation tree and conservation event are Stage 6. `HydrologyEventKind`
  carries `EdgeTransfer` and `Representation` variants that nothing emits yet, and the proposal
  carries an always-empty `edge_changes`; both are the seams those stages fill.

- **2026-07-30 — Stage 4 complete.** Substages 5 through 8 are implemented and the whole tick closes
  over routing, baseflow, conveyance, and boundary export. Checkpoint commit `bdc2a83`.

  Files changed and why:

  - `crates/causafera-domains/src/hydrology/evolution.rs` — surface and groundwater head, harmonic
    face conductance, the frozen-state demand pass, the donor and receiver largest-remainder
    reductions, baseflow, conveyance storage-discharge release, and the substage-8 sink receipts,
    plus per-edge running state, the after-conveyance graph, and the two new preconditions.
  - `crates/causafera-domains/src/hydrology/proposal.rs` — the request carries terrain.
  - `crates/causafera-domains/src/hydrology/parameters.rs` — three settlement-event process
    identities.
  - `crates/causafera-domains/src/hydrology/records.rs` — six new refusal variants.
  - `crates/causafera-domains/tests/support/mod.rs` — terrain, perimeter, boundary, and conveyance
    fixtures; `Scenario` defaults to flat terrain and a closed perimeter.
  - `crates/causafera-domains/tests/hydrology_routing.rs`, `hydrology_boundaries.rs`,
    `hydrology_groundwater.rs` — 51 new tests.

  `crates/causafera-domains/src/hydrology/receipts.rs` is on the plan's Stage 4 file list but needed
  no change: `HydrologyReceiptTotals::from_receipts` already classified every routing, conveyance, and
  boundary process kind allocated in Stage 3, and `validate_paired_transfers` /
  `validate_boundary_transfers` already covered the new receipt shapes. The three new suites assert
  both against every proposal they build rather than leaving that untested.

  What the gates actually assert:

  - **Head and flux (V8)** — head is terrain plus ponded depth, so equal water on unequal ground
    moves and unequal water on level ground also moves; equal heads move nothing and emit no receipt;
    the face conductance is `floor(2·300·700/1000) = 420`, not the arithmetic mean; a zero endpoint
    stops the face from either side.
  - **Reduction (V9)** — four demands of 1, 2, 3, 4 against seven available accept exactly 1, 1, 2, 3
    and sum to the donor's holding; two donors of four into five units of receiver room accept 3 and
    2 by ascending donor key and each keeps the refused remainder; a demand reduced to zero still
    carries its requested amount.
  - **Frozen substage (V10)** — one unit cannot cross two faces of a three-cell staircase, and the
    middle cell's refused onward demand is recorded rather than absent.
  - **Seam (V11)** — the same physical pair produces byte-identical flux inside a chunk and across a
    same-chart chunk seam.
  - **Once per face (V12)** — one receipt per canonical face; cell, chunk, and edge insertion
    permutations produce identical receipts, state, events, terminal leaves, and ledger.
  - **Boundaries (V13)** — an empty *and* a partially populated boundary map both refuse the tick; a
    no-flux perimeter retains everything; an open face exports exactly `(100 − 30) × 7 = 490`; equal
    or higher external head exports zero; an underfunded export is reduced and recorded; the surface
    and groundwater channels of one face are independent.
  - **Groundwater (V14)** — saturated depth follows the exact specific-yield equation, the aquifer
    base shifts the head it is measured from, and receiver capacity limits inflow with the remainder
    left in the donor.
  - **Baseflow and conveyance (V15)** — baseflow is exactly `excess × fraction`; below threshold
    produces none; a cell with no outgoing edge retains its groundwater; edge storage capacity and
    per-tick inlet each bound it; baseflow and groundwater lateral outflow split a donor canonically;
    release is the exact fraction of frozen pre-release storage, capped by outlet capacity with the
    remainder retained; three upstream edges into 500 units of downstream room accept 167, 167, 166
    by ascending source-edge key; water entering an edge this tick cannot leave it; a directed edge
    takes surface flow only when the head agrees, and a reverse-head transfer never enters it;
    surface inflow spends the inlet budget before baseflow sees it.
  - **Conservation (V16, V17 in part)** — a sloped closed basin closes exactly for 100 consecutive
    ticks with nonzero internal transfers; a 25-tick run over forcing, ET, routing, and export closes
    per tick and in aggregate, with the ledger cross-checked against receipts recomputed from scratch.
  - **Preconditions** — a request whose resident set disagrees with the field set is refused; events
    past the cause or effect cap are refused before the proposal is returned; the widest event of a
    crowded tick reaches five distinct causes and stays under sixteen.

  Commands run and their actual results:

  - `cargo test -p causafera-domains --test hydrology_routing -- --nocapture` — **18 passed**.
  - `cargo test -p causafera-domains --test hydrology_boundaries -- --nocapture` — **9 passed**.
  - `cargo test -p causafera-domains --test hydrology_groundwater -- --nocapture` — **19 passed**.
  - `cargo test -p causafera-domains --all-features` — **145 passed** across 11 binaries, 0 failed.
  - `cargo clippy --workspace --all-targets --all-features -- -D warnings` — clean.
  - `cargo fmt --all -- --check` — clean.
  - `git diff --check` — clean.
  - `cargo test --workspace --all-features` — all 76 suites passed.
  - `cargo test --workspace --no-default-features` — all 76 suites passed.

  Not done in this stage, by design: resolution (Stage 5) still emits nothing, so
  `HydrologyEventKind::Representation` and `HydrologyProperty::Resolution` remain unused; the §8
  aggregation tree, the terminal conservation event, and the synthetic-node counter are Stage 6, which
  is where the ordered terminal-leaf list — now including conveyance edges — is consumed.

- **2026-07-30 — Stage 5 complete.** Conservative resolution is implemented: block addressing over
  global terrain-cell coordinates, exact constitutive grouping, the capped largest-remainder reducer,
  coarse vertical and forcing aggregation with capacity-aware back-allocation, authoritative fine
  block-boundary faces, and promotion/demotion records. Checkpoint commit `ecba3e7`.

  Files changed and why:

  - `crates/causafera-domains/src/hydrology/resolution.rs` (new) — `HydrologyBlockKey`,
    `HydrologyConstitutiveKey`, `HydrologyResolutionPlan`, `HydrologyResolutionPolicy`,
    `allocate_capped`, `clamp_to_allocatable`, and `representation_change`.
  - `crates/causafera-domains/src/hydrology/evolution.rs` — level-aware dispatch for substages 1
    through 4, the coarse source, infiltration, percolation, and two-pass ET group passes, the
    block-internal face skip in routing, the deferred settlement fingerprint, and the group-total
    versus fine-grants agreement check.
  - `crates/causafera-domains/src/hydrology/proposal.rs` — the request carries the per-chunk
    resolution state and the policy; the proposal carries `coarse_processes`; `HydrologyEventPlan`
    carries `coarse_process`; `HydrologyCoarseProcess`, `HydrologyCoarseMember`,
    `HydrologyRepresentationChange`, and `resolution_fingerprint` are new.
  - `crates/causafera-domains/src/hydrology/parameters.rs` — `process::REPRESENTATION` and
    `substage::REPRESENTATION`.
  - `crates/causafera-domains/src/hydrology/records.rs` — five new refusal variants.
  - `crates/causafera-domains/tests/support/mod.rs` — `Scenario::at_level` and the resolution fields.
  - `crates/causafera-domains/tests/hydrology_resolution.rs` (new) — 29 tests.
  - `crates/causafera-domains/tests/hydrology_vertical_cycle.rs` — the settlement-fingerprint
    regression.

  `crates/causafera-domains/src/hydrology/receipts.rs` is on the plan's Stage 5 file list but needed
  no change: every coarse transfer carries one of the process kinds `HydrologyReceiptTotals` already
  classifies, and the three new suites assert `validate_paired_transfers`,
  `validate_boundary_transfers`, and the totals cross-check against every proposal they build.
  Coarse block-boundary aggregate validation is therefore the existing receipt fold rather than a
  second parallel one, which is what "receipt and validation aggregates of those accepted fine-face
  transfers only" asks for.

  What the gates actually assert:

  - **Aggregation (V19)** — a uniform block gives bucket-for-bucket the same answer as the fine path;
    a heterogeneous block splits into one group per exact substrate rather than averaging; coarse
    percolation is capped at the sum of its members' own results; a group's recorded candidate,
    summed ceilings, accepted total, and per-member grants agree exactly and its members are in
    canonical cell order; an evaluated group that moves nothing is still recorded with `T = 0`.
  - **Block boundaries (V19)** — faces inside a block are not evaluated and faces leaving one are,
    with the exact skipped and evaluated pair named; an accepted boundary transfer is installed on
    its own two fine endpoints and a third cell receives nothing; a coarse chunk and a fine chunk
    exchange across their shared seam.
  - **Work reduction (V19)** — a non-vacuous uniform 8x8 interior region at level 2 evaluates four
    process groups where fine mode evaluates 64 cells, and every fine cell still receives its share.
  - **Forcing and ET (V19)** — a record's total splits by weight and then by group capacity; an
    untargeted member receives nothing; ET runs surface then soil in that order; two records over one
    group produce two distinct process identities and every identity in a tick is unique.
  - **Conservation** — a closed coarse basin closes exactly for 100 consecutive ticks.
  - **Demotion and promotion (V20)** — a level change preserves every bucket, the substrate, the cell
    count, and the conveyance topology; the representation event cites the prior anchor as its one
    cause and transitions only the level; a no-op change and a level above the policy are both
    refused.
  - **Allocation failure (V21)** — the reducer refuses a positive total with no eligible member and a
    total above its summed ceilings; the clamp is shown never to hand it either; a refused tick leaves
    the input state and its total byte-identical.
  - **Request validation** — a resident chunk with no resolution entry is refused; a level above the
    policy is refused rather than clamped; extra entries for chunks hydrology does not hold are
    ignored; a disabled policy is proven equal to the fine path.

  Commands run and their actual results:

  - `cargo test -p causafera-domains --test hydrology_resolution -- --nocapture` — **29 passed**.
  - `cargo test -p causafera-domains --all-features` — **191 passed** across 12 binaries, 0 failed.
  - `cargo clippy --workspace --all-targets --all-features -- -D warnings` — clean.
  - `cargo fmt --all -- --check` — clean.
  - `git diff --check` — clean.
  - `cargo test --workspace --all-features` — all 77 suites passed.
  - `cargo test --workspace --no-default-features` — all 77 suites passed.

  Not done in this stage, by design: the coarse-input leaf and node trees, the coarse-process event
  itself, and the synthetic-node ID counter are Stage 6, which is where the persisted counter lives.
  Nothing yet consumes `HydrologyRepresentationChange` — the runtime's `Phase::Resolution` adapter is
  Stage 6 as well.

- **2026-07-30 — Stage 6 wave A complete: the configuration contract.** `HydrologyConfig` exists, is
  disabled by default, validates its bounds when enabled, and round-trips through the canonical
  runtime recipe encoding at major 7. Checkpoint commit `9725d2d`.

  Files changed and why:

  - `crates/causafera-runtime/src/hydrology_config.rs` (new) — `HydrologyConfig`,
    `HydrologyForcingSpec`, `HydrologyBootstrapParameters`, `HydrologyBootstrapOverride`, and their
    validation.
  - `crates/causafera-runtime/src/config.rs` — `RuntimeConfig::hydrology`, defaulted to disabled and
    validated at construction.
  - `crates/causafera-runtime/src/runtime.rs` — thirteen refusal variants.
  - `crates/causafera-runtime/src/snapshot_sections.rs` — recipe major 6 to 7, and the canonical
    hydrology configuration encoder and decoder.
  - `crates/causafera-runtime/src/lib.rs` — module registration and re-export.
  - `crates/causafera-runtime/tests/hydrology_runtime.rs` (new) — 15 tests.
  - `crates/causafera-runtime/tests/hydrology_legacy_compatibility.rs` — the two V22 tests now
    separate the legacy subsystem payloads from the declared recipe change.

  What the gates actually assert:

  - **The disabled default** — a new `RuntimeConfig` carries no metric, no parameters, no forcing, and
    a disabled resolution policy, and constructs; a disabled configuration carrying any one of those
    four is refused rather than silently ignored.
  - **Bounded enablement** — enabling without bootstrap parameters or without an explicit grid metric
    is refused; an unknown limits or bootstrap schema is refused rather than assumed; each of the four
    fractions outside `[0, 1]` is refused; initial storage above its own capacity is refused for all
    four buckets rather than clamped; groundwater capacity without a specific yield is refused.
  - **The forcing schedule** — a record at tick zero is refused and tick one is admitted; the horizon
    itself is admitted and one tick past it is refused; unsorted, duplicated, empty-target, unsorted-
    target, and duplicated-target schedules are all refused; a resolution level above the
    representable maximum is refused rather than clamped.
  - **The canonical encoding** — a disabled configuration round-trips; an enabled one round-trips
    every field including a negative aquifer offset, per-chart and per-cell overrides, and a
    single-face open boundary; re-encoding a decoded value is byte-identical, so one configuration has
    one representation; two sessions differing only in whether hydrology is on do not share recipe
    bytes.
  - **V22** — every pre-hydrology subsystem section is byte-identical; the recipe section moved from
    major 6 to 7 and grew by exactly 24 bytes; the physical, history, and experiment digests are
    unchanged and the full-envelope digest moved.

  Commands run and their actual results:

  - `cargo test -p causafera-runtime --test hydrology_runtime` — **15 passed**.
  - `cargo test -p causafera-runtime --test hydrology_legacy_compatibility` — **7 passed**.
  - `cargo clippy --workspace --all-targets --all-features -- -D warnings` — clean.
  - `cargo fmt --all -- --check` — clean.
  - `git diff --check` — clean.
  - `cargo test --workspace --all-features` — all 78 suites passed.
  - `cargo test --workspace --no-default-features` — all 78 suites passed.
  - `node tools/audit/check-entry-points.mjs` — pass (27 entry points, 18 tests).
  - `node tools/audit/run-source-tests.mjs` — 0 failures.

  Not done in this wave, by design: nothing yet *executes* hydrology. The system is not registered,
  no runtime state is held, and no tick proposes anything — an enabled configuration currently
  validates, persists, and reloads without producing water. Waves B through E remain, in the order
  recorded in the Decision log; the digest schema is still 7 and section `0x000F` does not exist yet,
  so the plan's schema-8 and required-section changes are still ahead.

- **2026-07-30 — Stage 6 wave B complete: the runtime commits hydrology.** An enabled session builds
  a world from configured numbers and real terrain, advances it every tick in `Phase::Physics`, maps
  the domain's logical DAG onto the runtime's causal schema, builds the §8 terminal aggregation tree
  and the §9 coarse-input trees from one shared node counter, and installs the after-state only after
  the whole batch commits. Checkpoint commit `<pending>`.

  Files changed and why:

  - `crates/causafera-runtime/src/hydrology_events.rs` (new) — the allocated schema identifiers, the
    dense `HydrologyObjectRegistry`, carrier and property mapping, the batch-leaf, node, coarse-leaf,
    coarse-process, and batch-sequence fingerprints, the shared 16-ary tree builder, and the batch
    assembler.
  - `crates/causafera-runtime/src/hydrology.rs` (new) — `HydrologyRuntimeState` with whole-batch
    receipt retention, `HydrologyEvolutionSystem`, and causal initialization from configuration
    including the §4 per-second-to-per-tick conversions, downhill conveyance construction, and the
    explicit boundary perimeter.
  - `crates/causafera-runtime/src/runtime.rs` — the appended registration, the state field, the
    manifest declaration, `hydrology_state()`, and eight refusal variants.
  - `crates/causafera-runtime/src/hydrology_config.rs` — resolved-override validation.
  - `crates/causafera-runtime/Cargo.toml` — `blake3` moved to the main dependencies.
  - `crates/causafera-geography/src/hydrology/state.rs` — per-bucket anchor installers, the edge
    installer, and `Default` for the empty field set.
  - `crates/causafera-geography/src/hydrology/metric.rs` — `Default` for the empty registry.
  - `crates/causafera-geography/src/hydrology/forcing.rs` — `applied_at`.
  - `crates/causafera-geography/src/hydrology/mod.rs` — two error variants.
  - `crates/causafera-domains/src/hydrology/evolution.rs`,
    `crates/causafera-domains/tests/hydrology_routing.rs` — the pass-through correction above.
  - `crates/causafera-runtime/tests/hydrology_runtime.rs` — 9 further tests.
  - `crates/causafera-runtime/tests/hydrology_legacy_compatibility.rs`,
    `crates/causafera-runtime/tests/thermal_determinism.rs` — the append assertions above.

  What the gates actually assert:

  - **Causal initialization** — every resident chunk holds a full surface lattice; the per-tick
    infiltration limit is the exact product of the configured rate, the cell area, and the timestep;
    roughness-adjusted transmissivity survives the conversion as a positive conductance; every
    exterior face carries a record; every registry table is a bijection onto a dense range and covers
    every cell and every edge.
  - **Conveyance construction** — every edge joins two orthogonally adjacent resident cells, runs
    strictly downhill, and there is at most one outgoing edge per cell, so the graph is acyclic and a
    local minimum keeps its water.
  - **The committed tick** — an enabled session commits one retained batch per tick, each with a
    residual of exactly zero and a non-empty receipt set; a closed world's cells and edges together
    hold the same total after six ticks as before; every bucket that moved names a new trace and the
    conservation anchor advances; the terminal tree consumes synthetic node identifiers.
  - **Retention** — twelve ticks leave exactly eight retained batches, each with both its transfer
    and its conservation receipts, so eviction is whole-batch.
  - **Determinism** — two runs of the same configuration produce identical fields, conveyance, node
    counter, retained batch list, and conservation receipts.
  - **A disabled session** — holds no field, ticks, and produces no batch.
  - **Risk R7** — asserted as described in the Decision log.

  Commands run and their actual results:

  - `cargo test -p causafera-runtime --test hydrology_runtime` — **24 passed**.
  - `cargo test -p causafera-runtime --test hydrology_legacy_compatibility` — **7 passed**.
  - `cargo clippy --workspace --all-targets --all-features -- -D warnings` — clean.
  - `cargo fmt --all -- --check` — clean.
  - `git diff --check` — clean.
  - `cargo test --workspace --all-features` — all 78 suites passed.
  - `cargo test --workspace --no-default-features` — all 78 suites passed.
  - `node tools/audit/check-entry-points.mjs` — pass.
  - `node tools/audit/run-source-tests.mjs` — 35 passed, 0 failed.

  Not done in this wave, by design: causal initialization runs from `RuntimeState::new` rather than
  from a seventh bootstrap stage, so there is no origin event and therefore no forcing yet — the
  schedule is validated and persisted in the recipe but no record is installed, which is why nothing
  rains. Wave C adds the stage, its seven aggregate effects, and the forcing installation. The
  whole-tick staging transaction and near-cap later-phase rollback are also still ahead: a hydrology
  failure currently records itself in `state.failure` like every other system's, which stops the run
  but does not yet roll back a partially advanced tick.

Execution must begin by re-reading this Progress section and Decision log, then inspecting
`git status`. The implementing agent updates both sections whenever scope, contract, verification,
or checkpoint evidence changes.
