# Domain Coverage Matrix

Every fundamental domain must answer:

1. What objectively exists?
2. Where is authoritative state stored?
3. What processes modify it?
4. What moves or propagates?
5. How can the state be physically observed?
6. How may agents conceptualize it?
7. How can information about it spread?
8. How is it represented under causal resolution?
9. How is provenance preserved?
10. How is it exposed to the observer layer?
11. What are its performance risks?
12. What are its deterministic requirements?
13. Which other domains can it affect?
14. Which other domains can affect it?

A foundational domain cannot enter implementation without coverage analysis.

## Status interpretation after Phase 26

Foundation-era `Completed` and `Partial` labels describe the acceptance scope of Phases 0–26. They
do not mean that a domain is mature. Detailed Development uses the capability maturity levels in
`docs/architecture/detailed-development-rebaseline.md`:

```text
M0 documented → M1 contracted → M2 executable → M3 coupled → M4 observable → M5 validated
```

The maturity ranges below are provisional and intentionally conservative. A dedicated audit must
split broad domains into capabilities and assign evidence-backed levels before new sequencing is
accepted.

## Domains

| Domain | Foundation baseline | Provisional maturity | Detailed Development gap |
|--------|---------------------|----------------------|--------------------------|
| Space | Coordinate and hierarchy primitives | M1–M2 | Complete chart/frame migration, adjacency, topology, and physically owned local geometry |
| Time | Deterministic scheduler time | M2 | Domain timescales, calendars as emergent/observer structures, temporal aggregation, and benchmarked long horizons |
| Matter | Generic physical/material contracts; chart-qualified material surfaces with durable condition state | M1–M2 | Conserved transformations, contact/containment coupling, production mutation systems, and broader material provenance |
| Energy | Fixed-point thermal-energy carrier with conserved same-chart intra-chunk and cross-chunk transfer, finite historical-bootstrap reservoirs, exact conservation accounting, and persisted current-batch active-region boundary records | M2 | Thermal-to-material coupling, cross-chart transport, climate/biology integration, and economy |
| Pattern / Feature | Generic samples, bounded history, and local gate coupling participate in mana | M2–M3 | Broader physical carriers, validated detection metrics, resolution behavior, and domain effects |
| Spatial geometry | Finite charted 2.5D surface plus bounded local 3D contracts | M1–M2 | Concrete planet metric, atlas transforms, cross-chart geodesics, interiors/subsurface, and domain migration |
| Geography | Hierarchy, terrain cells, deterministic terrain carrier participating in the tick loop as a standing structure, per-cell elevation and roughness projected to the observer, terrain continuous across chunk boundaries with the carrier's structure magnitude reading real cross-chunk neighbours | M1–M2 | Durable geographic processes, materials, water, climate/ecology coupling, and multiscale synthesis |
| Geology | Documentation only | M0 | Formation, strata, deposits, deformation, material provenance, and long-timescale resolution |
| Hydrology | Documentation only | M0 | Conserved water state, terrain flow, groundwater, climate coupling, hazards, and aggregation |
| Climate | Documentation only | M0 | Atmosphere/energy state, transport, seasonality, terrain/water/ecology coupling, and resolution |
| Ecology | Documentation only | M0 | Populations, resources, trophic/material interactions, succession, disturbance, and observation |
| Biology | Body topology and pathogen contracts; minimal runtime body is separate | M1 with a narrow M2 path | Integrate morphology, physiology, development, heredity, reproduction, aging, death, pathogens, and population conservation into runtime |
| Physical access / perception | Range/threshold acquisition and generic extraction run for promoted actors | M2 | Real geometry, occlusion/media, multiple physical signal carriers, internal access, adaptation, and costs |
| Cognition | Bounded attention, scene, continuity, concepts, beliefs, trust, and hypotheses exist | M1 with a narrow M2 path | State-dependent goals, learning across real environments, memory lifecycle, prediction-error propagation, social inference, and performance |
| Language | Lineage, opaque phonology, communication boundary, lexical change, and documents | M1 | Physical acoustics, grammar/morphology, grounded interpretation, conversation, diffusion, writing/reading, and runtime coupling |
| Mana | Local fixed-point field, history response, diffusion/decay, traced boost feedback, per-surface local hysteresis gates with observer deltas and explanation claims, and a standing geometric producer for the spatial-repetition channel | M3–M4 | Validated pattern response, further real carriers, domain-specific effects, resolution, and deeper explanatory metrics |
| Causal resolution | Relevance field, hysteresis, traced transitions, and minimal actor/population promotion | M2–M3 | Domain aggregation contracts, conservation proofs, reconstruction, cross-chart operation, and quality/error metrics |
| Society | Distributed relation, role, authority, claim, rule, practice, and agreement contracts | M1 | Agent-local social inference, lifecycle, institutions, governance, enforcement, conflict, diffusion, and resolution |
| Economy | Inventory, transfer, transformation, labour, and contestable ownership contracts | M1 | Production runtime, allocation, scarcity response, capability constraints, exchange emergence, conservation, and aggregation |
| City infrastructure | Parcel/building/network contracts | M1 | Generated physical structures, flows, occupancy, degradation, maintenance, hazards, growth, and geographic coupling |
| Historical bootstrap | Stage DAG and receipt provenance; minimal aggregate bootstrap exists | M1 with a narrow M2 path | Concrete domain synthesis, causal population/actor creation, deep-time integration, aggregation, caching, and plausibility tests |
| Epistemics | Units, calibration ancestry, measurement, and physical documents | M1 | Instruments, experimental practice, uncertainty propagation, replication, institutions, credibility, and runtime learning |
| Practice | Bounded programs, proposal execution, mutation, and lineage | M1 | Embodied execution, resources/tools, learning, transmission, roles, failure, standardization, and mana coupling |
| Isekai | Transfer plans/receipts, imported priors, and capability separation | M1 | Transfer physics, body/cognition integration, translation, historical occurrence, contamination, persistence, and observation |
| Metaphysics | Neutral identity criteria and minimal trajectory probes | M1 | Domain-valid attractor/state metrics, causal hypotheses, repeated experiments, alternatives, and non-semantic agency evidence |
| Simulation runtime | Deterministic eight-phase harness, provenance, replay, persistence, bounded observer; modular architecture with extracted domain modules (INV-042) | M3 infrastructure; domain depth uneven | Remove fixtures/timers, integrate all claimed domains, durable coupling, maturity validation, provenance growth strategy, and representative performance |
| Explanation / analytics | Typed IR, evidence states, comparison context, localized rendering | M2–M3 infrastructure | Replace digest distances, add domain metrics/units/alternatives, causal queries, intervention design, phenomenon evaluation, and honest insufficiency |
| Observer | Versioned queries and bounded streams for summary, chunks, Explanation, and per-chunk field rasters with per-cell provenance | M3 infrastructure | Causal slices, domain series, objective/subjective views, entity/history inspection, schema metadata, and measured overhead |
| UI | Real dark Tauri/React observer with bounded aggregate views and five-locale presentation (`en-US`, `ru-RU`, `zh-Hans`, `de-DE`, `es-ES`) with persisted preference | M2 presentation | Batch stable inspection workflows after simulation/Explanation contracts mature; do not chase every internal field |
| Optional LLM surface | Policy boundary only | Not scheduled | Terminal gate only after target simulation and deterministic Explanation reach validated maturity |
