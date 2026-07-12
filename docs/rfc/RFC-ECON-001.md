# RFC-ECON-001: Traceable Material Economy Foundation

**Status:** Accepted

## Summary

Represent the Phase 20 economy minimum as bounded physical inventory lots, transfers, transformations, and performed labour records. These carriers describe material history, not markets or objective economic meaning.

## Objective carrier boundary

An inventory lot records a `MaterialId`, physical holder and location, positive integer quantity, time, and causal trace. A transfer relates an existing source lot to a destination lot of the same material and cannot exceed the recorded source quantity. A transformation relates bounded input and output lot sets and may cite a practice. Labour records identify an actual agent contribution and duration.

## Possession is not ownership

The lot holder records physical custody. Optional `PropertyClaimId` references preserve socially asserted ownership support from Phase 19, but the economy never decides which claim is valid. A lot may have no claims or multiple competing claims.

## Primitive versus emergent

Material identity, integer amount, holder/location, physical ancestry, performed duration, time, and trace are authoritative bookkeeping. Commodity categories, prices, value, wages, jobs, shortage, surplus, market, ownership validity, and production purpose remain emergent or analytical.

## Determinism and performance

All collections are capped and canonically sorted. References use typed IDs and binary search. There is no floating point, RNG, string, unordered traversal, or scale claim.

## Deferred work

Proposal/commit lifecycle mutation, conservation across committed batches, scheduling, recipes, tools, capability checks, allocation, markets, prices, currency, ownership transfer, observer projection, persistence, resolution aggregation, and benchmarks remain future work.

## Decision log

- 2026-07-12: Replace raw floating-point flows with typed integer material-lot ancestry.
- 2026-07-12: Keep custody objective and ownership contestable.
- 2026-07-12: Record labour as performed contributions, never semantic jobs.
