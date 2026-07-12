# RFC-CITY-001: Physical Urban Infrastructure Foundation

**Status:** Accepted

## Summary

Represent the Phase 20 city minimum as bounded parcel references, physical building records, and generic directed infrastructure topology tied to traceable material lots.

## Spatial and material boundary

Parcels reference objective `PlaceId` containment nodes instead of duplicating geometry. Buildings are physical `EntityId` records located on parcels and cite their material lots. Infrastructure networks contain spatial nodes and directed links with integer capacity, length, condition, construction-material references, and causal traces.

## No semantic network enum

An opaque `InfrastructureSchemaId` may historically distinguish networks used for surface movement, water conveyance, waste removal, or communication. The core does not contain `Road`, `Water`, or `Sewage` variants and does not infer social purpose from topology. Meanings may differ between agents and institutions.

## Primitive versus emergent

Spatial reference, physical entity, directed connectivity, length, capacity, condition, material ancestry, time, and trace are primitive bookkeeping. Road, sewer, utility, building use, district, settlement, city, ownership, accessibility, and urban value are emergent social or observer concepts.

## Determinism and performance

Collections have hard bounds and canonical typed-ID ordering. Topology validation rejects self-links, missing/cross-network endpoints, invalid material references, and zero capacity/length. Outgoing traversal is deterministic. No generated layout or scale claim is introduced.

## Deferred work

Spatial geometry validation, flow physics, traffic, hydrology, building interiors, construction lifecycle, degradation, maintenance, fire, failures, growth, governance, rights, observer projection, persistence, causal resolution, and benchmarks remain future work.

## Decision log

- 2026-07-12: Build city contracts on the same material-lot foundation as the economy.
- 2026-07-12: Store topology and physical properties without a semantic infrastructure taxonomy.
- 2026-07-12: Do not generate a settlement or urban history during the foundation phase.
