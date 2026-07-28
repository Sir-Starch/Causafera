# RFC-PERSIST-001: Deterministic Snapshot Format v1

**Status:** Accepted
**Date:** 2026-07-13
**Depends on:** RFC-TRACE-001, RFC-GEO-002, RFC-MANA-001, RFC-RES-001, RFC-HIST-001, RFC-EXPLAIN-001
**Replaces:** `TODO-PERSIST-001` (pending → accepted)

## Summary

Define the first bounded, versioned, canonical binary snapshot format that can export a completed-tick runtime state, survive atomic file write, and reconstruct an identical simulation whose physical-state and causal-history digests match the original at every matched checkpoint after restoration.

## Motivation

The post-Phase-24 implementation now includes:

- versioned `PhysicalStateDigest`, `HistoryDigest`, and `ExperimentDigest`;
- complete `CausalTraceStore` with event/effect/cause ancestry;
- bounded cross-tick `PhysicalPatternHistory`;
- chart-qualified multi-chunk mana fields and causal resolution;
- mana-effect hysteresis and bounded material-surface feedback state;
- active chunk bookkeeping and promotion/demotion state;
- objective biological actors, subjective scenes/active cognition, and action state;
- population aggregates and concrete historical bootstrap receipts;
- experiment manifests and read-only Explanation IR/analytics.

`causafera-runtime` now assembles the registered sections into a canonical logical-state snapshot
and reconstructs them through validated import before exposing a resumed runtime. Serializing Rust
object memory or derived observer data remains incorrect: the persisted boundary is the explicit
section inventory, with bounded allocation, validated reconstruction, and exact post-load digest
verification.

## Design

### Quiescent snapshot boundary

Version 1 snapshots are allowed only after a full scheduler tick has committed all phases. Pending proposal batches, temporary carrier buffers, locks, observer queues, and in-progress Explanation queries are excluded. Runtime exposes an explicit read-only `export_snapshot()` at this quiescent boundary.

Mid-phase snapshots are rejected and remain out of scope.

### Dependency direction

Avoid a persistence/runtime dependency cycle:

- `causafera-persistence` owns the generic envelope, section directory, canonical primitive codec, validation errors, integrity checks, and file I/O;
- runtime and domain crates expose stable logical export/reconstruction data or validated constructors;
- `causafera-runtime`, which already depends on persistence, assembles registered sections and reconstructs the scheduler/runtime recipe;
- persistence never imports or understands semantic runtime systems, actors, mana, or populations.

### Sectioned binary format

Use an explicit little-endian binary container. Rust struct layout and JSON are not authoritative encodings.

#### Header fields

| Field | Type | Description |
|-------|------|-------------|
| magic | [u8; 4] | Fixed bytes `0x4F 0x54 0x50 0x53` (`"OTPS"`) |
| format_major | u16 | Format major version (1 for v1) |
| format_minor | u16 | Format minor version |
| codec_revision | u32 | Canonical primitive codec revision |
| world_seed | u64 | Deterministic world seed |
| completed_time | u64 | Completed simulation time (tick count) |
| runtime_recipe_fingerprint | [u8; 32] | Runtime recipe/schema-registry fingerprint |
| physical_digest_schema | u16 | Physical digest schema version |
| physical_digest | [u8; 32] | Physical state digest at completed time |
| history_digest_schema | u16 | History digest schema version |
| history_digest | [u8; 32] | History digest at completed time |
| section_count | u16 | Number of sections |
| section_directory_offset | u64 | Offset to section directory |
| payload_integrity | [u8; 32] | BLAKE3 of entire payload (header + sections) |

#### Section directory entry

| Field | Type | Description |
|-------|------|-------------|
| section_schema_id | u64 | Opaque section schema identifier |
| section_major | u16 | Section schema major version |
| section_minor | u16 | Section schema minor version |
| flags | u32 | Reserved compatible feature flags |
| payload_offset | u64 | Offset to section payload |
| payload_length | u64 | Length of section payload in bytes |
| decoded_size_limit | u64 | Maximum decoded size for validation |
| section_integrity | [u8; 32] | BLAKE3 of section payload |

Sections are strictly ordered by schema ID and unique. Unknown required sections fail load. Unknown explicitly optional sections may be skipped only when the envelope declares that compatibility rule.

### Initial section registry (v1)

The first complete runtime snapshot includes separate bounded sections for:

1. **Runtime recipe and configuration** (`0x0001`, current major V5)
   - deterministic configuration (seed, stream parameters);
   - registered system schema IDs and revisions;
   - phase and registration order;
   - domain adapter schema revisions;
   - authoritative system parameters (mana, pattern, resolution, actor bounds, and the
     material-surface physical-signal boundary);
   - immutable experiment-recipe mana source records (bounded, sorted, validated);
   - completed scheduler time.

2. **Spatial/chart and active-chunk state** (`0x0002`)
   - chart identity and active chunk coordinates;
   - per-chunk relevance, level, total mana, event count, last transition trace;
   - chunk extent and radius configuration.

3. **Mana field set** (`0x0003`)
   - per-field id, chunk, extent, observed-through time;
   - intensity arrays (little-endian i64 per cell);
    - last-change trace IDs and pre-change fixed-point values per cell;
    - field state only; thresholds/hysteresis are recipe parameters and each committed activation
      gate is held by its material-surface record.

4. **Causal resolution field** (`0x0004`)
   - field id, evaluated-through time;
   - ordered chunk coordinates;
   - relevance values, levels, last traces;
   - resolution policy identifier.

5. **Physical pattern history** (`0x0005`)
   - per-pattern cap, global cap;
   - per-pattern sample queues (pattern id, chunk, position, time, magnitude, ordinal, cause);
   - insertion order queue.

6. **Runtime counters and material activity** (`0x0006`, current major V3)
   - physical event counts;
   - mana cell changes, physical effects;
   - resolution changes, transitions;
   - perceived features, subjective objects;
   - action committed/rejected counts;
   - material activity events.
    This is bookkeeping only; durable material and gate state belongs only to section `0x000C`.

7. **Biological actor objective state** (`0x0007`)
   - actor id, body position, energy;
   - sensor aperture configurations;
   - objective feature counts;
   - action proposal counts;
   - actor ancestry trace lists.

8. **Actor subjective/active cognition** (`0x0008`)
   - subjective scene presence/structure;
   - attention state (bounds, weights, focus);
   - body schema parts (subjective ids, relative positions, extents);
   - self-model associations (strengths, supporting percepts).
   - **Constraint:** no authoritative entity/place/body/chart/frame/trace IDs beyond explicitly external inaccessible bookkeeping.

9. **Population aggregates and bootstrap** (`0x0009`)
   - per-chart population counts, births, deaths;
   - material inflow/outflow;
   - causal ancestry trace lists;
   - aggregate actor pool membership;
   - historical bootstrap stage receipts (opaque).

10. **Causal trace store** (`0x000A`)
    - next event ID, next trace ID counters;
    - event count;
    - forward SoA arrays: event ids, trace ids, times, phases, kinds;
    - cause offsets and flat cause trace IDs;
    - effect offsets and flat causal effects (target kind/object/property, before/after fingerprints);
    - child index is NOT persisted; rebuilt deterministically after decode.

11. **Experiment manifest** (`0x000B`) — optional
    - format version, seed set, parameters;
    - code/schema revision identifiers;
    - warm-up, duration, hardware metadata;
    - activity counts, memory record;
   - state/history digests, confidence, supporting traces, evidence flag.

12. **Material surfaces** (`0x000C`, current major V3)
   - chart-qualified surface IDs and bounded condition/contact/last-transition records;
    - sorted pending physics changes, per-surface contact/gate anchors, bounded condition history,
      and bounded local-mana gate transitions;
   - at most 128 transition records, with eviction preferring an older non-mana record so the
     newest mana-mediated causal observation remains available to bounded observer and
     Explanation paths;
   - optional trace fields encode presence, so missing contact/mana ancestry is distinct from a
     valid `TraceId(0)`.
   - V3 adds each surface's `MaterialSurfaceThermalState` (retained energy, optional last-exchange
     trace) and a separately bounded (at most 128, oldest evicted first) list of
     `MaterialSurfaceThermalTransition` records — the material-side half of the conserved
     retained-heat exchange with the co-located thermal cell (`TODO-THERMAL-002`); import checks
     surface existence, strict trace ordering, and a real state change per record, mirroring the
     existing condition/gate transition validation.

13. **Experiment recipe mana source receipts** (`0x000D`, current major V1)
    - bounded executed-receipt records (at most 16), sorted by `(executed_tick, source_record_id)`;
    - fields: source record ID, scheduled tick, executed tick, source trace, before/after
      fixed-point intensity, recipe hash, policy schema;
    - prevents re-execution of immutable recipe records after save/resume;
    - import validates receipt ordering, correspondence to enabled nonzero recipe records,
      tick equality, root source event kind/phase/empty causes, cell-effect fingerprints,
      recipe hash, and policy schema before authoritative installation;
    - unsupported major versions fail closed.

14. **Conserved thermal carrier** (`0x000E`, current major V2)
    - fixed-point cell energy, causal anchors, active/resident chunk sets, finite reservoirs,
      transfer receipts, and exact per-batch conservation receipts;
    - canonically ordered current-batch boundary records for same-chart faces outside the active
      region, including the frozen post-injection/pre-diffusion source energy;
    - import reconstructs the complete expected boundary face set and rejects missing, extra,
      duplicate, unsorted, cross-chart, nonadjacent, wrong-face, or pre-state-mismatched records;
    - incomplete payloads, unsupported major versions, and trailing bytes fail closed.
    - V2 extends `ThermalParameters` with `material_exchange_fraction`/`material_thermal_capacity`
      and each transfer receipt with an optional material exchange term (retained energy
      before/after, signed flux, rejected remainder) — the cell-side half of the conserved
      retained-heat exchange with a co-located material surface's `0x000C` state
      (`TODO-THERMAL-002`); the receipt-flux equation import already enforces
      (`pre_state - sum(face.signed_flux) == post_state`) is extended to
      `pre_state - sum(face.signed_flux) - material.signed_flux.unwrap_or(0) == post_state`, for
      every batch, not only the latest.

### Authoritative / non-authoritative boundary

**Authoritative (must persist):**

- format/schema versions and opaque section IDs;
- canonical numeric/typed state;
- physical/history digests;
- committed causal graph;
- explicit runtime recipe and adapter revisions;
- integrity digests and byte lengths;
- simulation time and deterministic seed/configuration.

**Not authoritative (must NOT persist):**

- English/Russian labels, narrative, UI layout, selected panels;
- observer classifications or localized Explanation rendering;
- wall-clock timestamps as simulation causes;
- semantic snapshot section names inside simulation state bytes;
- inferred missing history or best-effort repaired state;
- platform pointer/layout representation;
- derived child indexes, caches, performance telemetry;
- locks, mutexes, trait objects, thread state;
- pending proposal batches or temporary buffers.

### Provenance encoding

Persist the canonical forward representation of `CausalTraceStore`:

- IDs and next-ID counters;
- event times, phases, opaque kinds;
- ordered cause offsets/IDs;
- ordered effect offsets/targets/before/after fingerprints.

Derived child indexes are rebuilt deterministically and validated after decode instead of being duplicated. Load verifies:

- monotonic IDs;
- parent-before-child ancestry;
- valid offsets within declared bounds;
- strict effect ordering;
- referenced trace existence.

### Runtime reconstruction recipe

Trait objects and locks are never serialized. A versioned runtime recipe records:

- deterministic configuration;
- registered system schema IDs and revisions;
- phase and registration order;
- domain adapter schema revisions;
- authoritative system parameters;
- completed scheduler time.

Load resolves every required schema from a compiled registry, reconstructs systems in canonical order, restores authoritative sections through validated constructors, derives each system's next execution time from the completed tick, rebuilds caches/indexes, and verifies digests before exposing the runtime.

### Integrity and trust boundary

Use BLAKE3 for envelope/section corruption detection. It is not an authoritative simulation digest and makes no authenticity claim. Signed snapshots and adversarial remote storage remain out of scope.

Decode validates:

- all lengths/counts before allocation;
- checked arithmetic throughout;
- global and per-section caps;
- no duplicate/overlapping sections;
- no trailing authoritative bytes after declared payload;
- typed errors without panic.

### File durability

Manual save uses a same-directory temporary file:

```text
encode and verify in memory or bounded stream
→ create unique temporary sibling
→ write complete bytes
→ flush and fsync file
→ atomic rename over requested destination
→ fsync parent directory where supported
```

Failure leaves the prior completed snapshot intact. Temporary-file cleanup is best effort and never deletes the prior destination.

## Digest audit

### PhysicalStateDigest contributors

| Field | Section | Rationale |
|-------|---------|-----------|
| time | header | snapshot boundary time |
| material-surface records and transitions | 0x000C | bounded authoritative material state and history |
| material-surface gate state and local gate transitions | 0x000C | per-surface local activation state and evidence |
| pattern_history samples | 0x0005 | bounded temporal patterns |
| mana observed_through | 0x0003 | field time |
| mana field intensities | 0x0003 | per-cell i64 values |
| resolution evaluated_through | 0x0004 | field time |
| resolution relevance/level per chunk | 0x0004 | per-entry state |
| active_chunks total_mana/event_count/last_transition | 0x0002 | chunk bookkeeping |
| actors position/energy/features/proposals | 0x0007 | objective actor state |
| actor_ancestry | 0x0007 | causal links |
| actor_objects | 0x0007 | physical objects |
| population_aggregates | 0x0009 | conserved quantities |
| aggregate_actor_pool | 0x0009 | membership |
| executed experiment-recipe mana source receipts | 0x000D | bounded source execution state and replay guard |
| thermal fields, reservoirs, receipts, and current boundary records | 0x000E | conserved energy state, causal anchors, and active-region boundary evidence |

### HistoryDigest contributors

| Field | Section | Rationale |
|-------|---------|-----------|
| all trace events | 0x000A | complete causal graph |
| event IDs, trace IDs, times, phases, kinds | 0x000A | event identity |
| causes (ordered) | 0x000A | ancestry edges |
| effects (ordered, with fingerprints) | 0x000A | state change edges |

### Derived / rebuilt state (not persisted)

| Field | Rebuild method |
|-------|---------------|
| CausalTraceStore.children | deterministic BTreeMap from forward edges |
| ResolutionField entry lookups | sorted chunk arrays + binary search |
| Actor subjective scene | recomputed from perception on next tick |
| Observer read models | recomputed after load |
| Performance telemetry | fresh measurement |

## Migration policy

- Compatible minor section additions under explicit rules;
- Pure registered migrations for supported older minor schemas;
- New major version for incompatible container or authoritative semantic changes;
- Unsupported major versions fail closed; no guesswork loading.

For the active actor/material/mana slice, the runtime accepts authoritative digest schema V6,
runtime-recipe/configuration major V5, mana-field major V2, physical-counters major V3,
material-surface major V3, experiment-recipe mana source receipts major V1, and thermal-carrier
major V2. Any other required digest schema or section major, including recipe major V4 or an
unsupported receipts major, is rejected deterministically rather than being coerced into the
current causal state.

Recipe major rose from V4 to V5 when `RuntimeConfig` gained `terrain_participation`, which decides
whether the terrain carrier reaches the tick loop. A V4 snapshot carries every other field of V5 but
not that contract, so accepting it would mean resuming a world whose participation had been silently
defaulted — a different world from the one that was saved. It is therefore rejected rather than
migrated. See `plans/terrain-carrier-participation.md`.

## Security considerations

- Integrity digests detect corruption but do not authenticate;
- decode caps prevent memory exhaustion from malicious length declarations;
- no deserialization of trait objects or unsafe pointers;
- validated constructors prevent bypassing domain invariants;
- snapshot write does not add a Ground Truth causal event.

## Non-goals

- Mid-phase or lock-free concurrent snapshots.
- Incremental/delta saves, journaling, branching timelines, or provenance compaction.
- Automatic periodic saves or crash recovery during a tick.
- Network/cloud storage, encryption, authentication, or snapshot signing.
- Observer Protocol Buffer reuse as persistence format.
- Persisting Explanation prose, LLM output, UI state, caches, threads, mutexes, or trait-object memory.
- Loading unsupported major versions through guesswork.
- Compression before the uncompressed reference format is benchmarked.
- Cross-architecture/endianness claims beyond the explicitly encoded little-endian logical format and tested Rust targets.

## Acceptance criteria

- [ ] every physical/history digest input has exactly one owning snapshot section or is explicitly deterministic derived state;
- [ ] no observer/Explanation rendering state appears in authoritative inventory;
- [ ] no crate dependency cycle is introduced;
- [ ] RFC and TODO scope are accepted before encoding begins;
- [ ] identical logical sections produce byte-identical snapshots regardless of insertion order;
- [ ] decoder performs no allocation before validating declared bounds;
- [ ] nontrivial multi-domain trace store roundtrips exactly;
- [ ] `HistoryDigest` matches before/after;
- [ ] `PhysicalStateDigest` matches exactly after all sections reconstruct;
- [ ] uninterrupted and resumed runs are bit-identical in canonical results;
- [ ] two encode/decode cycles produce the same bytes;
- [ ] load/save without ticking is idempotent;
- [ ] version failures are deterministic and diagnostic;
- [ ] golden v1 fixtures stored after format freeze.

## Cross-references

- `plans/history/persistence-roundtrip.md` — completed implementation plan
- `docs/architecture/invariants.md` — INV-006, INV-007, INV-011 through INV-023, INV-027 through INV-037
- `docs/architecture/determinism.md` — deterministic execution contracts
- `docs/architecture/provenance.md` — causal provenance requirements
- `docs/rfc/RFC-TRACE-001.md` — trace store design
- `docs/rfc/RFC-GEO-002.md` — chart-qualified geometry
- `docs/rfc/RFC-MANA-001.md` — mana field design
- `docs/rfc/RFC-RES-001.md` — causal resolution design
- `docs/rfc/RFC-HIST-001.md` — historical bootstrap design
- `docs/rfc/RFC-EXPLAIN-001.md` — explanation IR design

## Decision log

- 2026-07-13: Snapshot v1 is taken only at a completed scheduler tick.
- 2026-07-13: Persistence uses a sectioned canonical logical binary format, not Rust memory layout, JSON, or observer protobufs.
- 2026-07-13: Persistence owns envelope/codec/I/O; runtime owns domain section assembly and scheduler reconstruction.
- 2026-07-13: Physical and history digests are stored and recomputed independently after load.
- 2026-07-13: Forward provenance state is persisted; reverse indexes are deterministically rebuilt.
- 2026-07-13: Locks, trait objects, caches, observer state, Explanation prose, and LLM output are never authoritative snapshot data.
- 2026-07-13: Initial file writes use atomic replacement and preserve the prior valid snapshot on failure.
- 2026-07-13: Unsupported major versions fail closed; migration is explicit and pure.
- 2026-07-13: Section schema IDs are opaque u64 values without English meaning in authoritative bytes.
- 2026-07-21: Material-surface state and trace-anchor presence are digest inputs. The recipe and
  counters sections advanced to major V2 to encode the physical-signal boundary and
  scheduler-committed mana gate; the material-surface section begins at major V1. The loader
  rejects unsupported required versions.
- 2026-07-21: The immutable experiment-recipe source advances the runtime recipe section to V3,
  adds required receipt section `0x000D` V1, and advances the authoritative digest schema to V3.
  Receipt correspondence and source-event ancestry are validated before installation; unsupported
  required versions fail closed.
- 2026-07-28: A material surface's retained thermal energy and its co-located thermal cell's
  material exchange term become digest inputs (`TODO-THERMAL-002`). The material-surface section
  advances to major V3 (adds `MaterialSurfaceThermalState` and bounded `thermal_transitions`); the
  thermal-carrier section advances to major V2 (adds `material_exchange_fraction`/
  `material_thermal_capacity` and each receipt's optional material term); the authoritative digest
  schema advances to V6. The receipt-flux equation import already enforced is widened to include the
  material term, for every batch, not only the latest. Unsupported required versions fail closed.
