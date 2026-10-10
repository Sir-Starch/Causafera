# RFC-PERSIST-002: Durable Causal-History Storage

**Status:** Accepted
**Date:** 2026-07-31
**Depends on:** [RFC-TRACE-001](RFC-TRACE-001.md), [RFC-PERSIST-001](RFC-PERSIST-001.md), [causal-history-storage plan](../../plans/causal-history-storage.md), [hard invariants](../architecture/invariants.md)
**Supersedes:** Only the deferred full-history persistence/query portions of [RFC-TRACE-001](RFC-TRACE-001.md) and [RFC-PERSIST-001](RFC-PERSIST-001.md). Their event identity, proposal/reduce/commit, snapshot envelope, section identity, v1 header, and schema-8 snapshot contracts remain in force except where this RFC explicitly defines the accepted migration boundary.

## Summary

This RFC accepts the causal-history storage contract required by Todo 1 of [plans/causal-history-storage.md](../../plans/causal-history-storage.md). It defines schema 9 history storage as append-only canonical causal/evidence frames plus a transactional catalog, immutable checkpoints, bounded snapshots carrying a `HistoryAnchor`, generation-pinned readers, deterministic legacy schema-8 migration, and fail-closed recovery.

This document is a normative contract for future implementation work. It does not claim that durable causal-history bundles, schema 9 digests, `HistoryAnchor`, or `HistoryReader` are implemented in the current source tree.

## Authority and Scope

Normative sources for this RFC are, in order:

1. [plans/causal-history-storage.md](../../plans/causal-history-storage.md), especially Todo 1 and the proposed architecture.
2. [RFC-TRACE-001](RFC-TRACE-001.md), for deterministic event identity, causal ordering, opaque schema IDs, and non-semantic event records.
3. [RFC-PERSIST-001](RFC-PERSIST-001.md), for the v1 envelope, section registry, codec conventions, decode caps, migration policy, and current schema-8 snapshot behavior.
4. [docs/architecture/invariants.md](../architecture/invariants.md), especially INV-012, INV-014, INV-016, INV-017, INV-018, INV-019, INV-026, INV-038, INV-039, INV-041, and INV-042.

Derived objects are non-authoritative unless this RFC names them as primitive. Derived objects include SQLite indexes, hot caches, observer read models, Explanation projections, benchmark reports, and rebuilt child indexes. Rebuilding or evicting them must not alter canonical bytes, logical digests, replay validation, or authoritative state.

## Compatibility With Earlier RFCs

`RFC-TRACE-001` remains authoritative for:

- monotonic `EventId` and `TraceId` allocation;
- proposal-key ordering independent of producer execution order;
- parent-before-child ancestry;
- ordered causes and effects;
- opaque event kinds, target kinds, object IDs, property IDs, and fingerprints;
- the exclusion of named historical events or semantic labels from core provenance.

`RFC-PERSIST-001` remains authoritative for:

- snapshot `format_major=1` envelope identity and little-endian primitive encoding;
- section directory ordering, section uniqueness, section integrity, and checked allocation;
- current accepted schema-8 snapshot imports before this RFC's migration boundary;
- atomic file-save behavior for legacy one-file snapshots;
- fail-closed unsupported major versions and unknown required sections.

This RFC supersedes only these deferred or insufficient portions:

- `RFC-TRACE-001` "Persistence impact" deferral of snapshot representation.
- `RFC-TRACE-001` non-goals for full graph query language, transitive indexing, pruning, and compaction, but only to the extent replaced by bounded `HistoryReader` operations and rebuildable indexes.
- `RFC-PERSIST-001` persistence of the complete `SECTION_CAUSAL_TRACES` payload as the normal current snapshot representation.
- `RFC-PERSIST-001` absence of a durable full-history query/storage contract.
- `RFC-PERSIST-001` digest audit for physical/history/experiment schema 8, once schema 9 is emitted.

## Schema 9 Digest Contract

The implementation that satisfies this RFC must set `CURRENT_DIGEST_SCHEMA_VERSION=9` and update physical, history, and experiment digest call sites together. A partial bump is invalid.

### Physical State Digest V9

`physical_state_digest_v9` covers only current bounded physical/materialized fields that influence future physical evolution or identify the current physical state:

| Field group | Included inventory |
|---|---|
| Time and scheduler boundary | completed simulation time at the quiescent boundary |
| Material surfaces | current surface records, condition/contact/gate state, pending physics changes, current retained thermal state, current local gate state, and bounded transition windows that still influence current material identity |
| Pattern state | current bounded pattern samples and insertion order required by future pattern evolution |
| Mana fields | chart-qualified current field identities, observed-through time, intensity arrays, and current last-change anchors |
| Resolution and active chunks | current relevance/level entries, evaluated-through time, resolution policy identity, active chunk coordinates, chunk relevance, total mana, event count, and last transition trace |
| Actors and objects | objective actor body position, energy, objective feature/proposal counts, actor ancestry required for current identity, and objective actor/object membership |
| Population/bootstrap | current population aggregates, births/deaths/material flow totals, aggregate actor pool membership, canonical bootstrap plan identity, per-stage bounded result state, terminal bootstrap receipts, and causal ancestry required to validate current initialized state |
| Experiment recipe source receipts | bounded executed source receipts that guard against source re-execution |
| Thermal carrier | current thermal parameters, cell energies, reservoirs, active/resident chunk sets, current boundary records, current material exchange terms, and conservation state required for future thermal evolution |
| Hydrology | current hydrology configuration-linked state, resident/active sets, storage, conveyance, boundaries, applied/pending forcing required for future evolution, and current conservation anchors |

It excludes the complete causal store, complete thermal receipt maps, complete conservation receipt maps, complete hydrology retained-batch history, retention-policy windows used only by observer/Explanation projections, and every bounded historical transition collection whose only consumers are Explanation or observer read models. A field may remain in the physical digest only when an implementation inventory proves it affects future physical evolution or current physical identity.

### History Digest V9

`history_digest_v9` is a segment-boundary-independent logical history/evidence digest over the canonical record stream. It folds:

- all causal-event records using the field order preserved from current trace event encoding: event ID, trace ID, time, phase, kind ID, ordered causes, and ordered effects with target kind/object/property and before/after fingerprints;
- all domain evidence records for hydrology, thermal, material-surface, bootstrap/population, experiment forcing/input records, and future registered domains;
- all recorded external inputs needed for deterministic replay;
- all historical receipts removed from the physical digest exactly once.

The fold is independent of segment boundaries, storage file names, SQLite row order, compression choices, group size, durability mode, filesystem order, and index rebuild order. A persisted prefix/frontier may make ordinary ticks incremental, but a cold full-rescan oracle remains required for validation.

### Experiment Digest V9

`experiment_digest_v9` is a canonical fold of:

- world/recipe identity, including seed and runtime recipe fingerprint;
- `physical_state_digest_v9`;
- `history_digest_v9`.

`storage_root` is a separate integrity identity for durable bytes and catalog reachability. It is not a physical, history, or experiment digest and must not be compared as a simulation-state metric.

## Logical Digest Versus Storage Root

The logical history/evidence digest is the canonical identity of the causal/evidence record stream. It must not change when identical records are repartitioned across frames or segments.

The `storage_root` is an authenticated storage identity that folds sealed segment identities, active committed ranges, checkpoint identities, and catalog generation references. It may change when storage is repacked or sealed differently, provided the logical record stream and logical digest are unchanged.

Digest byte distance is never a physical, semantic, recovery, or stability metric.

## Bundle Layout and Vocabulary

Every production runtime must have a durable bundle before its first tick:

```text
<bundle>/
  catalog.sqlite
  segments/active-<sequence>.cfs
  segments/<content-address>.cfs
  checkpoints/<content-address>.ckp
  snapshots/<generation>.snap
  staging/
  migration/<source-digest>.proof
```

Vocabulary is normative:

| Term | Meaning |
|---|---|
| `logical generation` | The running-process generation after a complete logical tick has published authoritative state plus bounded overlay records. It may be ahead of durable storage in group mode. |
| `durable generation` | The generation visible through `catalog.sqlite` after all referenced bytes, ranges, indexes, checkpoints, and catalog rows validate and the durability boundary is forced. |
| `active overlay` | The bounded in-memory/active-segment view containing logical frames not yet published as a durable generation. It is limited by the configured group policy. |
| `checkpoint` | Immutable bounded runtime materialization at a durable frontier, not the full history and not every tick. |
| `HistoryAnchor` | The bounded snapshot reference to a bundle namespace, logical and durable generations, checkpoint digest, event/trace frontier, logical digest, storage root, digest schemas, evidence-coverage manifest digest, and durability mode. |
| `catalog` | The transactional visibility authority that maps durable generations to verified frame ranges, sealed segment identities, indexes, checkpoints, inputs, and snapshots. |

A portable snapshot is resumable only after the requested generation has been forced durable. New schema 9 snapshots carry `HistoryAnchor` and must not embed the complete event/evidence history.

## Frames, Segments, and Ordering

The history store is append-only.

- Each successful logical tick encodes one complete length-delimited frame.
- A frame is never split.
- Sealing cuts only between complete frames.
- A frame larger than the target segment size is stored as its own frame/segment unit.
- A sealed segment is immutable and content-addressed.
- Catalog rows publish exact `(file, start_offset, end_offset, frame_digest)` ranges.
- Uncommitted active-segment tails and orphan files are not visible durable history.

Canonical record order is:

```text
(commit_sequence, causal-event-before-evidence, event_id/record_key, schema_id, canonical key, payload digest)
```

No hash-map iteration, wall clock, locale, pointer identity, filesystem order, SQLite row order, or thread scheduling may participate in authoritative ordering.

Frame and segment digest rules:

- `frame_digest = BLAKE3(history-frame-domain || complete-header-without-digest || payload)`;
- `sealed_segment_identity = BLAKE3(history-segment-domain || immutable-header || complete-frame-range)`;
- logical history/evidence digest folds records, not storage boundaries;
- storage root folds sealed segment identities and active committed ranges separately.

## Canonical Records

### Causal Event Records

Causal records preserve the current `write_trace_event` field ordering and identity from `RFC-TRACE-001`:

| Field | Requirement |
|---|---|
| event ID | monotonic, unique, assigned at commit |
| trace ID | monotonic, unique, parent-before-child ancestry |
| time and phase | completed scheduler state for the event |
| kind ID | opaque schema/event identity, not semantic prose |
| causes | ordered direct cause trace IDs |
| effects | ordered targets and before/after 32-byte fingerprints |

### Domain Evidence Records

Every evidence record contains:

| Field | Requirement |
|---|---|
| schema ID | opaque deterministic evidence schema identity |
| schema major/minor | versioned domain payload contract |
| criticality | exactly `ReplayRequired` or `ExplanationOnly` |
| tick/time | canonical simulation time |
| causal anchors | sorted and deduplicated trace/event anchors |
| canonical key | domain-owned stable key for lookup and ordering |
| payload length | checked before allocation |
| canonical payload bytes | domain-owned canonical byte representation |
| payload digest | record-level integrity and ordering input |

Unknown evidence schemas remain raw-verifiable bytes for archival and Explanation inspection. Unknown `ReplayRequired` schemas make resume, migration, strict replay, and authoritative validation fail closed until a decoder/upcaster is available. Unknown `ExplanationOnly` schemas may be reported as semantically unavailable evidence; they must not be silently treated as support for a claim.

## Criticality and Decoder Retention

| Criticality | Decoder/upcaster requirement | Replay/resume behavior | Explanation behavior |
|---|---|---|---|
| `ReplayRequired` | A supported decoder or pure upcaster must be retained for every accepted major/minor in reachable history. | Missing, unsupported, corrupt, or non-upcastable payloads fail closed before installing runtime state or accepting replay. | May be queried only from a generation that passed authoritative validation; missing support remains an integrity/availability failure. |
| `ExplanationOnly` | A decoder is retained when available; raw byte verification is sufficient for archival integrity. | Missing decoder does not block replay if the record is not referenced by replay validation and the raw bytes/digests verify. | Returns explicit `Unknown`, `Partial`, `Missing`, or `IntegrityError`; never fabricates confidence or support. |

Decoder/upcaster retention is part of the storage contract. A migration, resume, or replay path may not drop a decoder because the hot projection has evicted the corresponding typed state.

## Evidence-Coverage Manifests

Every migration from schema 8 to schema 9 must produce an evidence-coverage manifest for every migrated evidence schema. The manifest is authoritative migration evidence and is referenced by checkpoints and `HistoryAnchor`.

Each coverage entry contains:

| Field | Requirement |
|---|---|
| evidence schema ID and major/minor | schema being migrated |
| criticality | `ReplayRequired` or `ExplanationOnly` |
| copied ranges/frontiers | exact source event/trace/time ranges copied into canonical frames |
| missing ranges/frontiers | ranges the source cannot prove because old bounded sections had already evicted typed receipts |
| first guaranteed typed-evidence frontier | earliest frontier after which the migrated typed evidence is complete |
| source completeness | `Complete`, `Partial`, or `Unknown` for the legacy source |
| count and payload digest summary | copied record count and canonical digest contribution |
| migration algorithm version | pure deterministic algorithm identifier |

A complete causal graph does not imply complete typed evidence. Missing legacy receipts are `Partial` or `Unknown`, never silently complete.

## Transactions, Publication, and Recovery

The default durability policy is bounded group durability:

- `GroupCommit { max_ticks: 32, max_wall_time: 250 ms }`;
- `Save`, checkpoint creation, explicit `flush_durable`, bundle handoff, clean shutdown, and migration completion force the pending group immediately;
- `EveryTick` is the explicit high-durability mode and forces publication for every completed tick.

For one successful logical tick:

1. Begin a `HistoryTxn` at the current logical generation and head.
2. Allocate deterministic IDs and validate the proposal/DAG batch against durable records, transaction-local records, and the bounded overlay.
3. Stage bounded runtime deltas, causal records, domain evidence, and recorded external inputs.
4. Encode one complete frame and append it to the active segment.
5. Publish logical state and active overlay atomically in the running process.

For one forced durable group:

1. Complete and flush the active-segment byte range.
2. `sync_all` the segment file and newly created parent directory entries.
3. Begin a catalog `IMMEDIATE` transaction with `synchronous=FULL`.
4. Recheck namespace, predecessor durable generation, frame digests/ranges, head IDs, replay-critical schema availability, checkpoint references, and logical digest continuity.
5. Insert frame/index/evidence/input/checkpoint rows and one durable generation.
6. Commit and force the catalog durability boundary.
7. Set `durable generation = logical generation` and clear the active overlay.

A logical tick failure restores scheduler/system time, bounded state pre-image, pending IDs, hot cache, staged evidence, and overlay. A process or power failure may discard only unacknowledged logical ticks after the last durable generation, bounded by 32 ticks or 250 ms in default group mode. Restart loads exactly the last complete durable generation or a later complete one; it never exposes a mixed generation.

Corrupt, missing, foreign, duplicated, truncated, trailing, offset-invalid, count-mismatched, or digest-mismatched referenced objects fail closed. Derived indexes may be rebuilt deterministically; referenced canonical bytes may not be repaired by guesswork.

## Checkpoints, Anchors, and Snapshots

A checkpoint is not a snapshot. A snapshot is not a checkpoint. A `HistoryAnchor` is neither the full history nor a checkpoint.

| Artifact | Bound | Contents | Authority |
|---|---|---|---|
| checkpoint | At or before 64 MiB of canonical committed frame bytes, 4096 durably published ticks, or explicit Save | bounded `RuntimeSnapshotData`, recipe/input fingerprints, event/trace frontier, logical history/evidence digest, storage root, physical digest v9, evidence-coverage manifest digest | immutable materialization at a durable frontier |
| `HistoryAnchor` | small snapshot section/reference | namespace, logical generation, durable generation, checkpoint digest, frontier, roots, digest schemas, durability mode, evidence-coverage manifest digest | resumable pointer to verified bundle state |
| snapshot envelope | bounded transport copy under the existing 256 MiB cap | current bounded state plus `HistoryAnchor`; no complete `0x000A` history in schema 9 | portable transport after durable force |

`SECTION_CAUSAL_TRACES=0x000A` is retained only as a legacy schema-8 migration input. Schema 9 snapshots use `HistoryAnchor`; they do not carry the full causal trace store.

## HistoryReader and Explanation Semantics

`HistoryReader` operations pin exactly one logical generation and its durable base/overlay view for the lifetime of the query page/result. A query must not silently cross generations.

Required reader operations include:

- `event(trace_id)`;
- `parents(trace_id)`;
- `children(trace_id, page)`;
- `records(range, page)`;
- `latest_matching_event_before(target, property, trace)`;
- `evidence(schema, key/page)`;
- `walk_ancestors(trace, max_depth, max_nodes)`;
- `checkpoint_at(time)`.

Explanation remains downstream and non-authoritative. It keeps the existing 64-trace evidence cap and adds explicit availability states: `Complete`, `Partial`, `Unknown`, `Missing`, and `IntegrityError`. Runtime import, replay, and validation use unbounded correctness APIs with hard errors, never Explanation page budgets.

## Limits

All limits are checked before allocation and before indexing:

| Limit | Value |
|---|---:|
| `MAX_HISTORY_RECORDS_PER_FRAME` | 524,288 |
| `MAX_CAUSES_PER_EVENT` | 4,096 |
| `MAX_EFFECTS_PER_EVENT` | 4,096 |
| `MAX_FRAME_DECODED_BYTES` | 128 MiB |
| `MAX_CHECKPOINT_DECODED_BYTES` | 128 MiB |
| `MAX_QUERY_PAGE_RECORDS` | 256 |
| `MAX_QUERY_PAGE_BYTES` | 4 MiB |
| snapshot envelope cap inherited from RFC-PERSIST-001 | 256 MiB |
| default group overlay | 32 ticks or 250 ms |

One tick is one frame, so the two frame bounds cap how much a single tick may commit. A default nine-chunk hydrology tick commits roughly 68,000 records, which exceeded the first draft of these bounds (65,536 records, 16 MiB) on the plan's own acceptance workload; the values above are eight times those and equal the checkpoint bound for bytes. The widest configurable active region (radius 4, area shape, 81 chunks) is about nine times a nine-chunk tick, so it still approaches the record bound; a tick that exceeds a bound fails closed with a typed error. Publishing one tick as several frames under one logical generation would remove the ceiling without raising it and is recorded as an open item in the plan.

Duplicate IDs, unknown parents, invalid offsets, unsupported versions, trailing bytes, overflow, mismatched counts, noncanonical ordering, unsupported `ReplayRequired` schemas, and failed digest/root checks are hard errors.

## Exact Schema-8 Compatibility and Migration Matrix

Only the following legacy schema-8 snapshot combination is accepted as a migration source. Every other combination is deterministically rejected before installing runtime state or publishing a bundle.

In exact phrase form, the accepted combination is: `format_major=1`, supported `codec_revision=1`, `digest schema 8`, `runtime recipe major 7`, `population/bootstrap major 2`, causal trace section `0x000A` major 1, hydrology section `0x000F` major 1 when hydrology is present, and all other current accepted section majors listed below.

### Required Header Matrix

| Header field | Accepted value |
|---|---|
| `format_major` | 1 |
| `format_minor` | 0 or another minor accepted by the existing v1 minor rule |
| `codec_revision` | 1, the supported codec revision emitted by the current writer |
| `physical_digest_schema` | 8 (`digest schema 8`) |
| `history_digest_schema` | 8 (`digest schema 8`) |
| digest schema identity | both physical and history schemas must be 8; schema 9 is produced only as the migration output |

### Required Section Matrix

| Section | Accepted major | Presence |
|---|---:|---|
| `0x0001` runtime recipe/configuration | 7 | required |
| `0x0002` spatial/chart and active chunks | 1 | required |
| `0x0003` mana field set | 2 | required |
| `0x0004` causal resolution field | 1 | required |
| `0x0005` physical pattern history | 1 | required |
| `0x0006` runtime counters and material activity | 3 | required |
| `0x0007` biological actor objective state | 1 | required |
| `0x0008` actor subjective/active cognition | 1 | required |
| `0x0009` population aggregates and bootstrap | 2 | required |
| `0x000A` causal trace store | 1 | required migration input |
| `0x000B` experiment manifest | 1 | optional, when present |
| `0x000C` material surfaces | 3 | required |
| `0x000D` experiment recipe mana source receipts | 1 | required |
| `0x000E` conserved thermal carrier | 2 | required |
| `0x000F` hydrology | 1 | required for current schema-8 baseline; when present in any accepted legacy fixture it must be major 1 |

All listed sections use minor 0 in the current accepted baseline. Unknown authoritative sections, missing required sections, duplicate sections, unsupported majors, unsupported required minors, invalid flags, trailing bytes, and payload-integrity mismatches are deterministic rejection cases. In particular, runtime recipe major 5 or 6, population/bootstrap major 1, causal trace major other than 1, hydrology major other than 1 when present, and digest schema other than 8 are rejected.

Schema-8 physical/history/experiment digest bytes are preserved in the migration proof. The migration output computes new schema 9 physical/history/experiment digests and a storage root; it must not claim byte equality between v8 and v9 digests.

## Migration Output

A valid schema-8 migration creates a new durable bundle and proof without rewriting the source snapshot. The proof records:

- source envelope digest;
- source header fields and section majors;
- v8 physical/history/experiment digest values;
- source causal event count and typed-receipt counts available in legacy sections;
- source-to-destination event/evidence mapping;
- schema 9 physical/history/experiment digest outputs;
- storage root;
- checkpoint and `HistoryAnchor` identity;
- migration algorithm version;
- one evidence-coverage manifest entry per migrated evidence schema.

The migration is idempotent. A duplicate destination, source mutation, unsupported section matrix, pre-hydrology source missing current required sections, truncated bytes, unsupported `ReplayRequired` schema, or corrupt referenced object fails closed.

## Ownership and Dependency Boundaries

| Crate/layer | Owns | Must not own |
|---|---|---|
| `causafera-core` | storage-independent owned causal/evidence records, canonical causal ordering rules, reader traits, transaction-local reader traits, bounded hot-cache semantics, causal batch validation | persistence backend, filesystem, SQLite, snapshots |
| `causafera-persistence` | canonical bytes, frame/segment codecs, `HistoryStore`, `HistoryCatalog`, SQLite backend, filesystem durability, checkpoints, indexes, recovery, typed storage errors | runtime/domain semantics, scheduler mutation, observer classifications |
| `causafera-runtime` | deterministic domain producers/reducers, staged runtime state, evidence payload codecs/decoders, digest field inventory, checkpoint payload assembly, scheduler-to-history transaction bridge | generic catalog storage ownership, Explanation authority |
| Explanation and observer layers | generation-pinned bounded reads, availability reporting, confidence/evidence presentation | mutation, replay validation, authoritative repair, classification feedback into simulation |

No dependency cycle is permitted. Persistence may depend on core; core must not depend on persistence. Downstream Explanation and observer code read through bounded pinned readers and never mutate storage.

## Non-Goals

- Raising the 256 MiB snapshot cap.
- Deleting, pruning, compacting away, or semantically rewriting authoritative history.
- Changing hydrology, thermal, material, population, mana, geography, RNG, or scheduler semantics.
- Introducing distributed storage, cloud synchronization, encryption, authentication, or snapshot signing.
- Redesigning observer UI or protocol surfaces.
- Treating semantic labels, narrative prose, LLM output, or localized strings as authoritative data.
- Claiming constant total catalog/index size; cold storage and derived indexes are expected to grow with history.
- Treating a bounded Explanation result as authoritative validation.
- Loading unsupported major versions by guesswork.
- Repairing corrupt referenced data silently.

## Decision Log

- 2026-07-31: Accepted schema 9 as the next digest schema. Physical state excludes historical receipt maps; history/evidence digest includes canonical causal, evidence, and input records.
- 2026-07-31: Accepted segment-boundary-independent logical history/evidence digest distinct from `storage_root`.
- 2026-07-31: Accepted append-only frames/segments plus a transactional catalog as durable visibility authority.
- 2026-07-31: Accepted bounded group durability by default: 32 ticks or 250 ms, with explicit `EveryTick` for per-tick power-loss durability.
- 2026-07-31: Accepted `ReplayRequired` versus `ExplanationOnly` criticality and decoder/upcaster retention as replay gates.
- 2026-07-31: Accepted evidence-coverage manifests for every migrated evidence schema.
- 2026-07-31: Accepted exact schema-8 migration only for the matrix in this RFC; all other combinations fail closed.
