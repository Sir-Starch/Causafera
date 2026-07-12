# RFC-HIST-001: Causal Historical Bootstrap Orchestration

**Status:** Accepted

## Summary

Phase 21 defines a bounded deterministic plan/receipt boundary for historical synthesis. It coordinates domain adapters and verifies causal continuity; it does not generate semantic historical outcomes or authorize state mutation itself.

## Authoritative plan

`HistoricalBootstrapPlan` contains an explicit world seed and a bounded canonical list of `HistoricalStage`s. A stage contains an opaque process schema, non-empty simulation-time interval, numeric detail ordinal, sorted target chunks, dependencies on earlier non-overlapping stages, external causal traces, and a parameter fingerprint.

Deep and recent history are policies over numeric fields, not semantic variants. A process schema identifies a registered binary adapter contract, never a primitive war, plague, migration, discovery, or settlement kind.

## Deterministic synthesis boundary

Each stage receives a domain-separated seed contribution derived only from world seed, bootstrap identity, stage identity, process schema identity, and start time. Concrete adapters remain responsible for READ → PROPOSE → REDUCE → COMMIT. The plan cannot mutate domain state.

After commit, an adapter supplies a `HistoricalStageReceipt` containing stage identity, completion time, result-state fingerprint, committed trace, and direct causes.

## Causal continuity

A complete record requires exactly one receipt per stage. Receipt causes must exactly equal the union of external causes and committed traces of declared dependencies. Receipt traces must be unique. Missing, extra, reordered, duplicated, or narratively invented causes are rejected.

## Resolution and ontology boundary

Detail ordinal and target chunks allow low-resolution deep history and higher-resolution recent/focal history. They do not implement aggregation, conservation, promotion, demotion, or individual synthesis.

Primitive bookkeeping includes typed IDs, time spans, numeric detail, spatial targets, fingerprints, dependency edges, seed inputs, and traces. Named eras, peoples, settlements, institutions, technologies, conflicts, disasters, families, and narratives are emergent state or downstream classifications.

## Determinism and bounds

Plans and receipts use canonical sorted vectors and integer-only seed mixing. Counts are capped. Locale, strings, floating point, system time, entropy, pointer identity, hash iteration, and producer scheduling cannot influence validation or stage seeds.

## Decisions

- **Accepted:** bounded canonical stage DAG with explicit time, target, detail, parameters, and causes.
- **Accepted:** stable stage seed contributions and exact committed receipt ancestry.
- **Accepted:** opaque process schemas rather than high-level historical event enums.
- **Deferred:** concrete domain synthesis adapters, aggregation, scheduling, caching, persistence, observer projection, acceleration, and benchmarks.
