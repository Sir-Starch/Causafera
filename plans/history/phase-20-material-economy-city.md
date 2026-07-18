# Phase 20 Material Economy and City Infrastructure

> **Historical record.** This completed ExecPlan describes a Foundation Era project stage. Its implementation status and terminology may be outdated; use [the documentation index](../../docs/index.md), [roadmap](../../docs/roadmap/roadmap.md), and [active plans](../../PLANS.md) for current guidance.

## Goal

Implement the adjacent Phase 20 foundations as one bounded package: trace-backed physical inventories, transfers, transformations and labour records, plus parcel/building/infrastructure topology that consumes those material carriers.

## Context

Phase 19 established contestable property claims and distributed organizations. The existing economy module is an untyped floating-point placeholder and no city code exists. City construction depends directly on material lots and transformations, so the two TODOs share one causal boundary and are convenient to complete together.

## Relevant invariants

INV-006, INV-009, INV-014 through INV-019, INV-023, and INV-025.

## Ontology domains affected

Matter, geography, economy, society, practice, and city infrastructure.

## Causal carriers affected

Physical material lots, transfers, transformation ancestry, performed labour, spatial containment, structural components, and directed infrastructure connections.

## Relevant documents

The project thesis, hard invariants, ontology coverage documents, settlement and city documents, maintenance guidance, ADR-002, RFC-SOCIAL-001, and TODO-ECON-001/TODO-CITY-001.

## Current state

`causafera-domains::economy` contains only raw integer endpoints, a raw material number, and `f64` quantity. City infrastructure has documentation but no authoritative contracts.

## Proposed architecture

Replace the placeholder with a canonically ordered `EconomicState`. Inventory lots retain material identity, physical holder/location, integer quantity, time, and trace. Transfers consume one source lot and name a destination lot. Transformations list bounded input and output lots and an optional practice; labour contributions remain agent-attributed performed records. Property is never inferred from possession: optional ownership support references Phase 19 `PropertyClaimId` records.

Add a canonically ordered `CityState`. Parcel records reference validated spatial `PlaceId`s; buildings are physical entities on parcels with material-lot components; infrastructure uses opaque network schemas and directed nodes/links with capacity, length, condition, material provenance, and traces. Water, sewage, roads, and other network meanings remain historical schemas rather than a semantic enum.

## Primitive vs emergent review

Typed identity, integer amount/capacity/condition, spatial reference, physical connectivity, performed work, time, and trace are primitive bookkeeping. Commodity, price, job, ownership validity, road, sewer, utility, building use, district, settlement, shortage, and city are social/agent/observer interpretations.

## Non-goals

Markets, prices, currency, allocation decisions, jobs, production scheduling, automatic ownership, growth generation, hydraulic simulation, traffic, fire, degradation, repairs, semantic network types, fake settlements, lifecycle mutation, observer protocol, persistence, and scale claims.

## Implementation stages

1. Add opaque carrier IDs and replace the economy placeholder.
2. Add parcel, building, and generic infrastructure contracts tied to material lots.
3. Validate capacities, uniqueness, references, conservation declarations, canonical ordering, and topology.
4. Test the semantic boundaries and update RFCs, TODO, roadmap, ontology, subsystem docs, changelog, and plan registry.

## Verification

Workspace tests, strict clippy, formatting, diff checks, architectural searches for floats/strings/semantic enums, and knowledge-graph refresh.

## Benchmark plan

No performance claim is made. Dedicated batch/traversal benchmarks remain required before scale claims.

## Determinism impact

No RNG, floating point, unordered collection, locale, or system time. Inputs are bounded and canonicalized by typed numeric identity.

## Memory impact

All top-level and nested vectors have hard capacities. Sorted vectors permit binary-search reference checks and deterministic traversal.

## Observer impact

None in this phase; future read-only projections may expose supply-chain and network provenance.

## Explanation impact

Trace-backed ancestry can later support explanations of material availability and structural dependencies without becoming authoritative narrative.

## Persistence impact

None; no persistence format is introduced.

## Cross-domain effects

Geography constrains locations, practices may support transformations, social claims may support asserted ownership, and physical urban recurrence may later couple to mana.

## Risks

Opaque schemas could be misused as hidden enums; possession could be confused with ownership; transformation records could be mistaken for a live production engine; infrastructure topology could be mistaken for generated urban history.

## Documentation changes

Accept economy/city RFCs and update the index, roadmap, ontology records, city documents, changelog, and rebaseline report.

## TODO changes

Complete TODO-ECON-001 and TODO-CITY-001 with explicit foundation scope and deferrals.

## Decision log

- 2026-07-12: Batch both Phase 20 TODOs because infrastructure construction consumes the same material-lot provenance foundation.
- 2026-07-12: Keep possession physical and ownership contestable through references to social property claims.
- 2026-07-12: Use opaque infrastructure schemas instead of water/road/sewage enums.

## Progress

- [x] Economic contracts implemented.
- [x] City contracts implemented.
- [x] Documentation and phase tracking updated.
- [x] Full verification passes.
